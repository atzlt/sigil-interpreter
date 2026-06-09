use std::ops::Range;

use lasso::Spur;
use logos::Logos;
use thiserror::Error;

use crate::{compiler::lexer::Token, value::Value, vm::Chunk};

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
        $chunk.emit_u16($val);
        emit_args!($chunk, $($rest)*);
    };
    ($chunk:expr, $val:expr, $($rest:tt)*) => {
        $chunk.emit_u8($val as u8);
        emit_args!($chunk, $($rest)*);
    };
    ($chunk:expr, wide $val:expr) => {
        $chunk.emit_u16($val);
    };
    ($chunk:expr, $val:expr) => {
        $chunk.emit_u8($val as u8);
    };
    ($chunk:expr $(,)?) => {};
}

use emit;
use emit_args;

// ── Precedence levels (low → high) ──

const PREC_OR: u8 = 2;
const PREC_AND: u8 = 3;
const PREC_EQUALITY: u8 = 4;
const PREC_COMPARISON: u8 = 5;
const PREC_TERM: u8 = 6;
const PREC_FACTOR: u8 = 7;
const PREC_UNARY: u8 = 8;

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
    reg_free: Vec<u8>,
}

pub fn compile(source: &str) -> Result<Chunk> {
    let mut free = Vec::with_capacity(256);
    for i in (0..=255).rev() {
        free.push(i);
    }
    let mut c = Compiler {
        lexer: Token::lexer(source).spanned(),
        chunk: Chunk::new(),
        current: (Token::default(), Span::default()),
        reg_free: free,
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
        } else if self.current.0 == Token::Eof {
            Err(CompileError::Unclosed {
                diag: vec![
                    (open_span, "opened here".to_string()),
                    (self.current.1.clone(), format!("expected {expected}")),
                ],
            })
        } else {
            Err(CompileError::Unexpected {
                diag: vec![(
                    self.current.1.clone(),
                    format!("expected {expected}, found {}", self.current.0),
                )],
            })
        }
    }

    // ── Register allocation ──

    fn alloc_reg(&mut self) -> Result<u8> {
        self.reg_free
            .pop()
            .ok_or_else(|| CompileError::RegisterOverflow)
    }

    fn free_reg(&mut self, reg: u8) {
        if !self.reg_free.contains(&reg) {
            self.reg_free.push(reg);
        }
    }

    // ── Entry points ──

    fn expression(&mut self) -> Result<u8> {
        self.parse_precedence(PREC_OR)
    }

    // ── Pratt parser core ──

    fn parse_precedence(&mut self, min_bp: u8) -> Result<u8> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let Some(bp) = infix_bp(&self.current.0) else {
                break;
            };
            if bp < min_bp {
                break;
            }
            let op = self.current.0;
            self.advance()?;
            let rhs = self.parse_precedence(bp + 1)?;
            lhs = self.emit_binary(&op, lhs, rhs)?;
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
        let reg = self.alloc_reg()?;
        emit!(self.chunk, LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_string(&mut self, spur: Spur) -> Result<u8> {
        let s = self.lexer.extras.interner.resolve(&spur).to_string();
        let k = self.chunk.add_constant(Value::String(s));
        let reg = self.alloc_reg()?;
        emit!(self.chunk, LOADK, reg, wide k);
        Ok(reg)
    }

    fn emit_bool(&mut self, b: bool) -> Result<u8> {
        let reg = self.alloc_reg()?;
        emit!(self.chunk, LOADBOOL, reg, b as u8);
        Ok(reg)
    }

    fn emit_nil(&mut self) -> Result<u8> {
        let reg = self.alloc_reg()?;
        emit!(self.chunk, LOADNIL, reg);
        Ok(reg)
    }

    fn emit_identifier(&mut self, _name: Spur) -> Result<u8> {
        // TODO: variable lookup → register. See Phase 3.
        self.emit_nil()
    }

    fn emit_binary(&mut self, op: &Token, lhs: u8, rhs: u8) -> Result<u8> {
        let method = binary_op_method(op);
        let name_idx = self.chunk.add_constant(Value::String(method.into()));
        let reg = self.alloc_reg()?;
        emit!(self.chunk, CALL, reg, wide name_idx, 2_u8, lhs, rhs);
        self.free_reg(lhs);
        self.free_reg(rhs);
        Ok(reg)
    }

    fn emit_unary(&mut self, method: &str, operand: u8) -> Result<u8> {
        let name_idx = self.chunk.add_constant(Value::String(method.into()));
        let reg = self.alloc_reg()?;
        emit!(self.chunk, CALL, reg, wide name_idx, 1_u8, operand);
        self.free_reg(operand);
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

fn infix_bp(tok: &Token) -> Option<u8> {
    match tok {
        Token::Or => Some(PREC_OR),
        Token::And => Some(PREC_AND),
        Token::EqEq | Token::Neq => Some(PREC_EQUALITY),
        Token::Lt | Token::Le | Token::Gt | Token::Ge => Some(PREC_COMPARISON),
        Token::Plus | Token::Minus => Some(PREC_TERM),
        Token::Star | Token::Slash | Token::Percent => Some(PREC_FACTOR),
        // TODO: add more infix operators as language design settles
        _ => None,
    }
}
