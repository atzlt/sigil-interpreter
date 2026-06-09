use std::ops::Range;

use lasso::Spur;
use logos::Logos;
use thiserror::Error;

use crate::{
    compiler::{lexer::Token, register::RegisterTracker},
    value::Value,
    vm::Chunk,
};

macro_rules! emit {
    ($chunk:expr, $op:ident) => {
        $chunk.emit_opcode($crate::vm::OpCode::$op);
    };
    ($chunk:expr, $op:ident, $($args:tt)*) => {
        $chunk.emit_opcode($crate::vm::OpCode::$op);
        emit_args!($chunk, $($args)*);
    };
}

macro_rules! emit_args {
    ($chunk:expr, wide $val:expr, $($rest:tt)*) => {
        $chunk.emit_wide($val);
        emit_args!($chunk, $($rest)*);
    };
    ($chunk:expr, $val:expr, $($rest:tt)*) => {
        $chunk.emit($val as u8);
        emit_args!($chunk, $($rest)*);
    };
    ($chunk:expr, wide $val:expr) => {
        $chunk.emit_wide($val);
    };
    ($chunk:expr, $val:expr) => {
        $chunk.emit($val as u8);
    };
    ($chunk:expr $(,)?) => {};
}

use emit;
use emit_args;

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

// ── Compiler ──

type Span = Range<usize>;
type SpanInfo = (Span, String);

#[derive(Error, Debug, Clone)]
pub enum CompileError {
    #[error("unexpected token")]
    Unexpected { diag: Vec<SpanInfo> },
    #[error("unclosed delimiter")]
    Unclosed { diag: Vec<SpanInfo> },
    #[error("unrecognized token")]
    Unrecognized { diag: Vec<SpanInfo> },
    #[error("register overflow")]
    RegisterOverflow,
}

type Result<T> = std::result::Result<T, CompileError>;

pub struct Compiler<'a> {
    lexer: logos::SpannedIter<'a, Token>,
    chunk: Chunk,
    current: (Token, Span),
    regs: RegisterTracker,
}

pub fn compile(source: &str) -> Result<Chunk> {
    let mut c = Compiler {
        lexer: Token::lexer(source).spanned(),
        chunk: Chunk::new(),
        current: (Token::default(), Span::default()),
        regs: RegisterTracker::new(256),
    };
    c.advance()?;
    let result_reg = c.expression()?;
    emit!(c.chunk, RETURN, result_reg, 1_u8);
    Ok(c.chunk)
}

impl Compiler<'_> {
    // ── Token handling ──

    fn advance(&mut self) -> Result<()> {
        let spanned = self
            .lexer
            .next()
            .or_else(|| Some((Ok(Token::Eof), self.lexer.span())))
            .unwrap();
        let token = spanned.0.map_err(|_| CompileError::Unrecognized {
            diag: vec![(spanned.1.clone(), "unexpected token".to_string())],
        })?;
        self.current = (token, spanned.1);
        Ok(())
    }

    fn check(&self, tok: &Token) -> bool {
        std::mem::discriminant(&self.current.0) == std::mem::discriminant(tok)
    }

    #[allow(dead_code)]
    fn matches(&mut self, tok: &Token) -> Result<bool> {
        if self.check(tok) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn consume(&mut self, expected: &Token) -> Result<()> {
        if self.check(expected) {
            self.advance()?;
            Ok(())
        } else {
            Err(CompileError::Unexpected {
                diag: vec![(
                    self.current.1.clone(),
                    format!("expected {expected}, found {}", self.current.0),
                )],
            })
        }
    }

    fn consume_close(&mut self, expected: &Token, open_span: Span) -> Result<()> {
        if self.check(expected) {
            self.advance()?;
            Ok(())
        } else {
            Err(CompileError::Unclosed {
                diag: vec![
                    (open_span, "opened here".to_string()),
                    (
                        self.current.1.clone(),
                        format!("expected {expected}, found {}", self.current.0),
                    ),
                ],
            })
        }
    }

    // ── Entry points ──

    fn expression(&mut self) -> Result<u8> {
        self.parse_precedence(0)
    }

    // ── Pratt parser core ──

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

            if op == Token::Quest {
                lhs = self.emit_ternary(lhs)?;
            } else {
                let next_bp = match assoc {
                    Assoc::Left => bp + 1,
                    Assoc::Right => bp,
                };
                let rhs = self.parse_precedence(next_bp)?;
                lhs = self.emit_binary(&op, lhs, rhs)?;
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
            Token::True => {
                self.advance()?;
                self.emit_bool(true)
            }
            Token::False => {
                self.advance()?;
                self.emit_bool(false)
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
                diag: vec![(
                    self.current.1.clone(),
                    format!("expected expression, found {}", &self.current.0),
                )],
            }),
        }
    }

    // ── Bytecode emission ──

    fn emit_number(&mut self, n: f64) -> Result<u8> {
        let k = self.chunk.add_constant(Value::Number(n));
        let reg = self.regs.alloc_temp()?;
        emit!(self.chunk, LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_string(&mut self, spur: Spur) -> Result<u8> {
        let s = self.lexer.extras.interner.resolve(&spur).to_string();
        let k = self.chunk.add_constant(Value::String(s));
        let reg = self.regs.alloc_temp()?;
        emit!(self.chunk, LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_bool(&mut self, b: bool) -> Result<u8> {
        let reg = self.regs.alloc_temp()?;
        emit!(self.chunk, LOADBOOL, reg, b as u8);
        Ok(reg)
    }

    fn emit_nil(&mut self) -> Result<u8> {
        let reg = self.regs.alloc_temp()?;
        emit!(self.chunk, LOADNIL, reg);
        Ok(reg)
    }

    fn emit_identifier(&mut self, _name: Spur) -> Result<u8> {
        // TODO: variable lookup → register. See Phase 3.
        self.emit_nil()
    }

    fn emit_test(&mut self, lhs: u8) -> usize {
        emit!(self.chunk, TEST, lhs, wide 0);
        self.chunk.last_wide()
    }

    fn patch_conditional(
        &mut self,
        test_ip: usize,
        if_end: usize,
        else_start: usize,
        else_end: usize,
    ) {
        self.chunk
            .patch_wide(if_end + 1, (else_end - if_end) as u16);
        self.chunk
            .patch_wide(test_ip + 2, (else_start - test_ip) as u16);
    }

    fn reuse_or_alloc(&mut self, ops: &[u8]) -> Result<u8> {
        for &op in ops {
            if self.regs.is_reusable(op) {
                return Ok(op);
            }
        }
        self.regs.alloc_temp()
    }

    fn free_others(&mut self, dst: u8, ops: &[u8]) {
        for &op in ops {
            if dst != op {
                self.regs.free_reg(op);
            }
        }
    }

    fn emit_binary(&mut self, op: &Token, lhs: u8, rhs: u8) -> Result<u8> {
        let method = binary_op_method(op);
        let name_idx = self.chunk.add_constant(Value::String(method.into()));
        let reg = self.reuse_or_alloc(&[lhs, rhs])?;
        emit!(self.chunk, CALL, reg, wide name_idx, 2_u8, lhs, rhs);
        self.free_others(reg, &[lhs, rhs]);
        Ok(reg)
    }

    fn emit_unary(&mut self, method: &str, operand: u8) -> Result<u8> {
        let name_idx = self.chunk.add_constant(Value::String(method.into()));
        let reg = self.reuse_or_alloc(&[operand])?;
        emit!(self.chunk, CALL, reg, wide name_idx, 1_u8, operand);
        self.free_others(reg, &[operand]);
        Ok(reg)
    }

    fn emit_ternary(&mut self, test: u8) -> Result<u8> {
        let test_ip = self.chunk.end();
        self.emit_test(test);
        let reg = self.reuse_or_alloc(&[test])?;

        let mhs = self.parse_precedence(0)?;
        emit!(self.chunk, MOVE, reg, mhs);
        let if_end = self.chunk.end();
        emit!(self.chunk, JMP, wide 0);

        self.consume(&Token::Colon)?;

        let else_start = self.chunk.end();
        let rhs = self.parse_precedence(PREC_TERNARY)?;
        emit!(self.chunk, MOVE, reg, rhs);
        let else_end = self.chunk.end();
        
        self.patch_conditional(test_ip, if_end, else_start, else_end);
        self.free_others(reg, &[test, mhs, rhs]);
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
        Token::And => "and",
        Token::Or => "or",
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
        Token::EqEq | Token::Neq => Some((PREC_EQUALITY, Assoc::Left)),
        Token::Lt | Token::Le | Token::Gt | Token::Ge => Some((PREC_COMPARISON, Assoc::Left)),
        Token::Plus | Token::Minus => Some((PREC_TERM, Assoc::Left)),
        Token::Star | Token::Slash | Token::Percent => Some((PREC_FACTOR, Assoc::Left)),
        // TODO: add more infix operators as language design settles
        _ => None,
    }
}
