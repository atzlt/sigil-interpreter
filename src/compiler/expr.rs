use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
    },
    emit, emit_args,
    functions::{FnLookupKey, LangItem},
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
const PREC_CALL: u8 = 90;

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
                Token::LParen => {
                    lhs = self.parse_call_expression(lhs, target)?;
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
                self.emit_unary(FnLookupKey::LangItem(LangItem::Neg), inner, target)
            }
            Token::Bang => {
                self.advance()?;
                let inner = self.parse_precedence(PREC_UNARY, target)?;
                self.emit_unary(FnLookupKey::LangItem(LangItem::Not), inner, target)
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

    // ── Intrinsic call helper ──

    fn emit_lang_item_call(
        &mut self,
        fun: FnLookupKey,
        args: &[u8],
        target: Option<u8>,
    ) -> Result<u8> {
        let fn_id = *self
            .resolve_fn(&fun)
            .ok_or_else(|| CompileError::UndefinedFunction {
                name: fun.to_string(),
                diag: (
                    self.current_span().clone(),
                    "this language item is not defined".to_string(),
                ),
            })?;
        let reg = self.target_or_reuse(target, args)?;
        self.emit_callk(reg, fn_id, self.reg_watermark(), args);
        self.free_other_temps(reg, args);
        Ok(reg)
    }

    // ── Bytecode emission ──

    fn emit_identifier(&mut self, name: Spur) -> Result<u8> {
        if let Some(reg) = self.try_resolve_local(name) {
            return Ok(reg);
        }
        if let Some(up_idx) = self.resolve_upvalue(name) {
            let reg = self.alloc_temp()?;
            emit!(self.chunk_mut(), GETUPVAL, reg, wide up_idx as u16);
            return Ok(reg);
        }
        let slot = self
            .resolve_global(name)
            .ok_or_else(|| CompileError::UndefinedVariable {
                name: self.intern_resolve(&name).to_string(),
                diag: (self.prev_span().clone(), "undefined variable".to_string()),
            })?;
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), GETGLB, reg, wide slot);
        Ok(reg)
    }

    fn emit_number(&mut self, n: f64) -> Result<u8> {
        let k = self.chunk_mut().add_constant(Value::Number(n));
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_string(&mut self, spur: Spur) -> Result<u8> {
        let s = self.intern_resolve(&spur).to_string();
        let k = self.chunk_mut().add_constant(Value::String(s));
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_bool(&mut self, b: bool) -> Result<u8> {
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADBOOL, reg, b as u8);
        Ok(reg)
    }

    fn emit_nil(&mut self) -> Result<u8> {
        let reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADNIL, reg);
        Ok(reg)
    }

    fn emit_binary(&mut self, op: &Token, lhs: u8, rhs: u8, target: Option<u8>) -> Result<u8> {
        self.emit_lang_item_call(binary_op_lang_item(op), &[lhs, rhs], target)
    }

    fn emit_unary(&mut self, fun: FnLookupKey, operand: u8, target: Option<u8>) -> Result<u8> {
        self.emit_lang_item_call(fun, &[operand], target)
    }

    fn emit_ternary(&mut self, test: u8, target: Option<u8>) -> Result<u8> {
        let skip_then = self.emit_forward_test(test);
        let reg = self.target_or_reuse(target, &[test])?;
        self.free_other_temps(reg, &[test]);

        let mhs = self.parse_precedence(0, target)?;
        self.emit_move(reg, mhs);
        let skip_else = self.emit_forward_jmp();
        self.free_other_temps(reg, &[mhs]);

        self.place_label(skip_then);
        self.consume(&Token::Colon)?;

        let rhs = self.parse_precedence(PREC_TERNARY, target)?;
        self.emit_move(reg, rhs);
        self.free_other_temps(reg, &[rhs]);

        self.place_label(skip_else);
        Ok(reg)
    }

    fn emit_short_circuit_or(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let skip_rhs = self.emit_forward_test(lhs);
        let reg = self.target_or_reuse(target, &[lhs])?;
        self.free_other_temps(reg, &[lhs]);

        self.emit_move(reg, lhs);
        let skip_end = self.emit_forward_jmp();

        self.place_label(skip_rhs);
        let rhs = self.parse_precedence(PREC_OR + 1, target)?;
        self.emit_move(reg, rhs);
        self.free_other_temps(reg, &[rhs]);

        self.place_label(skip_end);
        Ok(reg)
    }

    fn emit_short_circuit_and(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let skip_rhs = self.emit_forward_test(lhs);
        let reg = self.target_or_reuse(target, &[lhs])?;
        self.free_other_temps(reg, &[lhs]);

        let rhs = self.parse_precedence(PREC_AND + 1, target)?;
        self.emit_move(reg, rhs);
        let skip_end = self.emit_forward_jmp();

        self.place_label(skip_rhs);
        self.emit_move(reg, lhs);
        self.free_other_temps(reg, &[rhs]);

        self.place_label(skip_end);
        Ok(reg)
    }

    fn parse_call_expression(&mut self, lhs: u8, target: Option<u8>) -> Result<u8> {
        let mut args = vec![];
        loop {
            if self.current() == Token::RParen {
                break;
            }
            let arg = self.expression(target)?;
            args.push(arg);
            if !self.matches(&Token::Comma)? {
                break;
            }
        }
        self.consume(&Token::RParen)
            .map_err(|_| CompileError::Unexpected {
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    "expect argument list to close here".to_string(),
                ),
            })?;
        let dst = self.target_or_reuse(target, &[lhs])?;
        self.emit_call(dst, lhs, self.reg_watermark(), &args);
        Ok(dst)
    }
}

fn binary_op_lang_item(op: &Token) -> FnLookupKey {
    match op {
        Token::Plus => FnLookupKey::LangItem(LangItem::Add),
        Token::Minus => FnLookupKey::LangItem(LangItem::Sub),
        Token::Star => FnLookupKey::LangItem(LangItem::Mul),
        Token::Slash => FnLookupKey::LangItem(LangItem::Div),
        Token::Percent => FnLookupKey::LangItem(LangItem::Rem),
        Token::Equal => FnLookupKey::LangItem(LangItem::Eq),
        Token::Neq => FnLookupKey::LangItem(LangItem::Neq),
        Token::Lt => FnLookupKey::LangItem(LangItem::Lt),
        Token::Le => FnLookupKey::LangItem(LangItem::Le),
        Token::Gt => FnLookupKey::LangItem(LangItem::Gt),
        Token::Ge => FnLookupKey::LangItem(LangItem::Ge),
        // TODO: add more operators as language design settles
        _ => unreachable!("binary_op_lang_item called with non-binary token {op:?}"),
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
        Token::LParen => Some((PREC_CALL, Assoc::Left)),
        // TODO: add more infix operators as language design settles
        _ => None,
    }
}
