use lasso::Spur;

use crate::{
    compiler::{
        compile::{CompileError, Compiler, Result},
        lexer::Token,
    },
    emit, emit_args,
    value::Value,
};

// ── Precedence levels (low → high) ──

const PREC_ASSIGN: u8 = 0;
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
    pub(super) fn expression(&mut self) -> Result<u8> {
        self.parse_precedence(0)
    }

    fn parse_precedence(&mut self, min_bp: u8) -> Result<u8> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let Some((bp, assoc)) = infix_bp_assoc(&self.current.0) else {
                break;
            };
            if bp < min_bp {
                break;
            }
            let op = self.current.0;
            self.advance()?;

            match op {
                Token::Quest => {
                    lhs = self.emit_ternary(lhs)?;
                }
                Token::And => {
                    lhs = self.emit_short_circuit_and(lhs)?;
                }
                Token::Or => {
                    lhs = self.emit_short_circuit_or(lhs)?;
                }
                _ => {
                    let next_bp = match assoc {
                        Assoc::Left => bp + 1,
                        Assoc::Right => bp,
                    };
                    let rhs = self.parse_precedence(next_bp)?;
                    lhs = self.emit_binary(&op, lhs, rhs)?;
                }
            }
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<u8> {
        match &self.current.0 {
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
                let inner = self.parse_precedence(PREC_UNARY)?;
                self.emit_unary("neg", inner)
            }
            Token::Bang => {
                self.advance()?;
                let inner = self.parse_precedence(PREC_UNARY)?;
                self.emit_unary("not", inner)
            }
            Token::LParen => {
                let open_span = self.current.1.clone();
                self.advance()?;
                let inner = self.expression()?;
                self.consume_close(&Token::RParen, open_span)?;
                Ok(inner)
            }
            _ => Err(CompileError::Unexpected {
                token: self.current.0,
                diag: (
                    self.current.1.clone(),
                    format!("expected expression, found {}", &self.current.0),
                ),
            }),
        }
    }

    // ── Bytecode emission ──

    fn emit_identifier(&mut self, name: Spur) -> Result<u8> {
        self.resolve_local(name)
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

    fn emit_binary(&mut self, op: &Token, lhs: u8, rhs: u8) -> Result<u8> {
        match *op {
            Token::Equals => {
                emit!(self.chunk, MOVE, lhs, rhs);
                self.regs.free_temp(lhs);
                self.regs.free_temp(rhs);
                Ok(lhs)
            }
            _ => {
                let method = binary_op_method(op);
                let name_idx = self.chunk.add_constant(Value::String(method.into()));
                let reg = self.reuse_or_alloc(&[lhs, rhs])?;
                emit!(self.chunk, CALL, reg, wide name_idx, 2_u8, lhs, rhs);
                self.free_other_temps(reg, &[lhs, rhs]);
                Ok(reg)
            }
        }
    }

    fn emit_unary(&mut self, method: &str, operand: u8) -> Result<u8> {
        let name_idx = self.chunk.add_constant(Value::String(method.into()));
        let reg = self.reuse_or_alloc(&[operand])?;
        emit!(self.chunk, CALL, reg, wide name_idx, 1_u8, operand);
        self.free_other_temps(reg, &[operand]);
        Ok(reg)
    }

    fn emit_ternary(&mut self, test: u8) -> Result<u8> {
        let test_ip = self.emit_test(test);
        let reg = self.reuse_or_alloc(&[test])?;
        self.free_other_temps(reg, &[test]);

        let mhs = self.parse_precedence(0)?;
        emit!(self.chunk, MOVE, reg, mhs);
        let if_end = self.emit_jmp();
        self.free_other_temps(reg, &[mhs]);

        self.consume(&Token::Colon)?;

        let else_start = self.chunk.end();
        let rhs = self.parse_precedence(PREC_TERNARY)?;
        emit!(self.chunk, MOVE, reg, rhs);
        let else_end = self.chunk.end();
        self.free_other_temps(reg, &[rhs]);

        self.patch_if_else(test_ip, if_end, else_start, else_end);
        Ok(reg)
    }

    fn emit_short_circuit_or(&mut self, lhs: u8) -> Result<u8> {
        let test_ip = self.emit_test(lhs);
        let reg = self.reuse_or_alloc(&[lhs])?;
        self.free_other_temps(reg, &[lhs]);

        if reg != lhs {
            emit!(self.chunk, MOVE, reg, lhs);
        }
        let if_end = self.emit_jmp();

        let else_start = self.chunk.end();
        let rhs = self.parse_precedence(PREC_OR + 1)?;
        emit!(self.chunk, MOVE, reg, rhs);
        let else_end = self.chunk.end();
        self.free_other_temps(reg, &[rhs]);

        self.patch_if_else(test_ip, if_end, else_start, else_end);
        Ok(reg)
    }

    fn emit_short_circuit_and(&mut self, lhs: u8) -> Result<u8> {
        let test_ip = self.emit_test(lhs);
        let reg = self.reuse_or_alloc(&[lhs])?;
        self.free_other_temps(reg, &[lhs]);

        let rhs = self.parse_precedence(PREC_AND + 1)?;
        emit!(self.chunk, MOVE, reg, rhs);
        let if_end = self.emit_jmp();

        let else_start = self.chunk.end();
        if reg != lhs {
            emit!(self.chunk, MOVE, reg, lhs);
        }
        let else_end = self.chunk.end();
        self.free_other_temps(reg, &[rhs]);

        self.patch_if_else(test_ip, if_end, else_start, else_end);
        Ok(reg)
    }
}

// ── Operator → lang-item method name ──

fn binary_op_method(op: &Token) -> &'static str {
    match op {
        Token::Plus => "add",
        Token::Minus => "sub",
        Token::Star => "mul",
        Token::Slash => "div",
        Token::Percent => "mod",
        Token::EqEq => "eq",
        Token::Neq => "neq",
        Token::Lt => "lt",
        Token::Le => "le",
        Token::Gt => "gt",
        Token::Ge => "ge",
        // TODO: add more operators as language design settles
        _ => unreachable!("binary_op_method called on non-binary token"),
    }
}

// ── Binding power tables ──

fn infix_bp_assoc(tok: &Token) -> Option<(u8, Assoc)> {
    match tok {
        Token::Equals => Some((PREC_ASSIGN, Assoc::Right)),
        Token::Quest => Some((PREC_TERNARY, Assoc::Right)),
        Token::Or => Some((PREC_OR, Assoc::Left)),
        Token::And => Some((PREC_AND, Assoc::Left)),
        Token::EqEq | Token::Neq => Some((PREC_EQUALITY, Assoc::Left)),
        Token::Lt | Token::Le | Token::Gt | Token::Ge => Some((PREC_COMPARISON, Assoc::Left)),
        Token::Plus | Token::Minus => Some((PREC_TERM, Assoc::Left)),
        Token::Star | Token::Slash | Token::Percent => Some((PREC_FACTOR, Assoc::Left)),
        // TODO: add more infix operators as language design settles
        _ => None,
    }
}
