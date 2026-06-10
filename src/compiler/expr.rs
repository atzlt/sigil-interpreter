use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
    },
    emit, emit_args,
    functions::{FnId, LangItem},
    value::Value,
};

// ── Precedence levels (low → high) ──

const PREC_TERNARY: u8 = 10;
const PREC_OR: u8 = 20;
const PREC_AND: u8 = 30;
const PREC_EQUALITY: u8 = 40;
const PREC_COMPARISON: u8 = 50;
const PREC_TERM: u8 = 60;
const PREC_FACTOR: u8 = 70;
const PREC_UNARY: u8 = 80;

#[derive(Clone, Copy)]
enum Assoc {
    Left,
    Right,
}

impl<'a> Compiler<'a> {
    pub(super) fn expression(&mut self, target: Option<u8>) -> Result<u8> {
        self.record_locus();
        self.parse_precedence(0, target)
    }

    fn parse_precedence(&mut self, min_bp: u8, target: Option<u8>) -> Result<u8> {
        let mut lhs = self.parse_prefix(target)?;

        loop {
            let Some((bp, assoc)) = infix_bp_assoc(&self.current()) else {
                break;
            };
            if bp < min_bp {
                break;
            }
            let op = self.current();
            self.advance()?;

            match op {
                Token::Quest => {
                    lhs = self.emit_ternary(lhs, target)?;
                }
                Token::And => {
                    lhs = self.emit_short_circuit_and(lhs, target)?;
                }
                Token::Or => {
                    lhs = self.emit_short_circuit_or(lhs, target)?;
                }
                _ => {
                    let next_bp = match assoc {
                        Assoc::Left => bp + 1,
                        Assoc::Right => bp,
                    };
                    let rhs = self.parse_precedence(next_bp, target)?;
                    lhs = self.emit_binary(&op, lhs, rhs, target)?;
                }
            }
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self, target: Option<u8>) -> Result<u8> {
        match &self.current() {
            Token::Number(n) => {
                let val = *n;
                self.advance()?;
                self.emit_number(val)
            }
            Token::String(spur) => {
                let spur = *spur;
                self.advance()?;
                self.emit_string(spur)
            }
            Token::BooleanLit(b) => {
                let b = *b;
                self.advance()?;
                self.emit_bool(b)
            }
            Token::Nil => {
                self.advance()?;
                self.emit_nil()
            }
            Token::Identifier(name) => {
                let name = *name;
                self.advance()?;
                self.emit_identifier(name)
            }
            Token::Minus => {
                self.advance()?;
                let inner = self.parse_precedence(PREC_UNARY, target)?;
                self.emit_unary(FnId::LangItem(LangItem::Neg), inner, target)
            }
            Token::Bang => {
                self.advance()?;
                let inner = self.parse_precedence(PREC_UNARY, target)?;
                self.emit_unary(FnId::LangItem(LangItem::Not), inner, target)
            }
            Token::LParen => {
                let open_span = self.current_span().clone();
                self.advance()?;
                let inner = self.expression(target)?;
                self.consume_close(&Token::RParen, open_span)?;
                Ok(inner)
            }
            _ => Err(CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    format!("expected expression, found {}", &self.current()),
                ),
            }),
        }
    }

    // ── Bytecode emission ──

    fn emit_identifier(&mut self, name: Spur) -> Result<u8> {
        if let Some(reg) = self.try_resolve_local(name) {
            return Ok(reg);
        }
        let slot = self
            .resolve_global(name)
            .ok_or_else(|| CompileError::UndefinedVariable {
                name: self.intern_resolve(&name).to_string(),
                diag: (self.prev_span().clone(), "undefined variable".to_string()),
            })?;
        let reg = self.alloc_temp()?;
        emit!(self.chunk, GETGLB, reg, wide slot);
        Ok(reg)
    }

    fn emit_number(&mut self, n: f64) -> Result<u8> {
        let k = self.chunk.add_constant(Value::Number(n));
        let reg = self.alloc_temp()?;
        emit!(self.chunk, LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_string(&mut self, spur: Spur) -> Result<u8> {
        let s = self.intern_resolve(&spur).to_string();
        let k = self.chunk.add_constant(Value::String(s));
        let reg = self.alloc_temp()?;
        emit!(self.chunk, LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_bool(&mut self, b: bool) -> Result<u8> {
        let reg = self.alloc_temp()?;
        emit!(self.chunk, LOADBOOL, reg, b as u8);
        Ok(reg)
    }

    fn emit_nil(&mut self) -> Result<u8> {
        let reg = self.alloc_temp()?;
        emit!(self.chunk, LOADNIL, reg);
        Ok(reg)
    }

    fn emit_binary(&mut self, op: &Token, lhs: u8, rhs: u8, target: Option<u8>) -> Result<u8> {
        let fun = binary_op_lang_item(op);
        let name_idx = self.chunk.add_constant(Value::Fn(fun));
        let reg = if let Some(target) = target {
            target
        } else {
            self.reuse_or_alloc(&[lhs, rhs])?
        };
        emit!(self.chunk, CALL, reg, wide name_idx, 2_u8, lhs, rhs);
        self.free_other_temps(reg, &[lhs, rhs]);
        Ok(reg)
    }

    fn emit_unary(&mut self, fun: FnId, operand: u8, target: Option<u8>) -> Result<u8> {
        let fun = self.chunk.add_constant(Value::Fn(fun));
        let reg = if let Some(target) = target {
            target
        } else {
            self.reuse_or_alloc(&[operand])?
        };
        emit!(self.chunk, CALL, reg, wide fun, 1_u8, operand);
        self.free_other_temps(reg, &[operand]);
        Ok(reg)
    }

    fn emit_ternary(&mut self, test: u8, target: Option<u8>) -> Result<u8> {
        let test_ip = self.emit_test(test);
        let reg = if let Some(target) = target {
            target
        } else {
            self.reuse_or_alloc(&[test])?
        };
        self.free_other_temps(reg, &[test]);

        let mhs = self.parse_precedence(0, target)?;
        self.emit_move(reg, mhs);
        let if_end = self.emit_jmp();
        self.free_other_temps(reg, &[mhs]);

        self.consume(&Token::Colon)?;

        let else_start = self.chunk.end();
        let rhs = self.parse_precedence(PREC_TERNARY, target)?;
        self.emit_move(reg, rhs);
        let else_end = self.chunk.end();
        self.free_other_temps(reg, &[rhs]);

        self.patch_if_else(test_ip, if_end, else_start, else_end);
        Ok(reg)
    }

    fn emit_short_circuit_or(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let test_ip = self.emit_test(lhs);
        let reg = if let Some(target) = target {
            target
        } else {
            self.reuse_or_alloc(&[lhs])?
        };
        self.free_other_temps(reg, &[lhs]);

        self.emit_move(reg, lhs);
        let if_end = self.emit_jmp();

        let else_start = self.chunk.end();
        let rhs = self.parse_precedence(PREC_OR + 1, target)?;
        self.emit_move(reg, rhs);
        let else_end = self.chunk.end();
        self.free_other_temps(reg, &[rhs]);

        self.patch_if_else(test_ip, if_end, else_start, else_end);
        Ok(reg)
    }

    fn emit_short_circuit_and(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let test_ip = self.emit_test(lhs);
        let reg = if let Some(target) = target {
            target
        } else {
            self.reuse_or_alloc(&[lhs])?
        };
        self.free_other_temps(reg, &[lhs]);

        let rhs = self.parse_precedence(PREC_AND + 1, target)?;
        self.emit_move(reg, rhs);
        let if_end = self.emit_jmp();

        let else_start = self.chunk.end();
        self.emit_move(reg, lhs);
        let else_end = self.chunk.end();
        self.free_other_temps(reg, &[rhs]);

        self.patch_if_else(test_ip, if_end, else_start, else_end);
        Ok(reg)
    }
}

fn binary_op_lang_item(op: &Token) -> FnId {
    match op {
        Token::Plus => FnId::LangItem(LangItem::Add),
        Token::Minus => FnId::LangItem(LangItem::Sub),
        Token::Star => FnId::LangItem(LangItem::Mul),
        Token::Slash => FnId::LangItem(LangItem::Div),
        Token::Percent => FnId::LangItem(LangItem::Rem),
        Token::Equal => FnId::LangItem(LangItem::Eq),
        Token::Neq => FnId::LangItem(LangItem::Neg),
        Token::Lt => FnId::LangItem(LangItem::Lt),
        Token::Le => FnId::LangItem(LangItem::Le),
        Token::Gt => FnId::LangItem(LangItem::Gt),
        Token::Ge => FnId::LangItem(LangItem::Ge),
        // TODO: add more operators as language design settles
        _ => unreachable!("binary_op_method called on non-binary token"),
    }
}

// ── Binding power tables ──

fn infix_bp_assoc(tok: &Token) -> Option<(u8, Assoc)> {
    match tok {
        Token::Quest => Some((PREC_TERNARY, Assoc::Right)),
        Token::Or => Some((PREC_OR, Assoc::Left)),
        Token::And => Some((PREC_AND, Assoc::Left)),
        Token::Equal | Token::Neq => Some((PREC_EQUALITY, Assoc::Left)),
        Token::Lt | Token::Le | Token::Gt | Token::Ge => Some((PREC_COMPARISON, Assoc::Left)),
        Token::Plus | Token::Minus => Some((PREC_TERM, Assoc::Left)),
        Token::Star | Token::Slash | Token::Percent => Some((PREC_FACTOR, Assoc::Left)),
        // TODO: add more infix operators as language design settles
        _ => None,
    }
}
