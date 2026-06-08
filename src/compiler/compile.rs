use anyhow::{Result, bail};
use lasso::Spur;
use logos::Logos;

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

pub struct Compiler<'a> {
    lexer: logos::Lexer<'a, Token>,
    chunk: Chunk,
    current: Token,
    reg_free: Vec<u8>,
}

pub fn compile(source: &str) -> Result<Chunk> {
    let mut free = Vec::with_capacity(256);
    for i in (0..=255).rev() {
        free.push(i);
    }
    let mut c = Compiler {
        lexer: Token::lexer(source),
        chunk: Chunk::new(),
        current: Token::default(),
        reg_free: free,
    };
    c.advance();
    let result_reg = c.expression()?;
    emit!(c.chunk, RETURN, result_reg, 1_u8);
    Ok(c.chunk)
}

impl Compiler<'_> {
    // ── Token handling ──

    fn advance(&mut self) {
        self.current = self.lexer.next().and_then(|r| r.ok()).unwrap_or(Token::Eof);
    }

    fn check(&self, tok: &Token) -> bool {
        std::mem::discriminant(&self.current) == std::mem::discriminant(tok)
    }

    #[allow(dead_code)]
    fn matches(&mut self, tok: &Token) -> bool {
        if self.check(tok) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, expected: &Token, msg: &str) -> Result<()> {
        if self.check(expected) {
            self.advance();
            Ok(())
        } else {
            bail!("{msg} (found {})", self.current_name())
        }
    }

    fn current_name(&self) -> String {
        match &self.current {
            Token::Identifier(spur) | Token::String(spur) => {
                self.lexer.extras.interner.resolve(spur).to_string()
            }
            other => format!("{:?}", other),
        }
    }

    // ── Register allocation ──

    fn alloc_reg(&mut self) -> u8 {
        self.reg_free
            .pop()
            .expect("register overflow: all 256 registers in use")
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
            let Some(bp) = infix_bp(&self.current) else {
                break;
            };
            if bp < min_bp {
                break;
            }
            let op = self.current;
            self.advance();
            let rhs = self.parse_precedence(bp + 1)?;
            lhs = self.emit_binary(&op, lhs, rhs);
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<u8> {
        match &self.current {
            Token::Number(n) => {
                let val = *n;
                self.advance();
                Ok(self.emit_number(val))
            }
            Token::String(spur) => {
                let spur = *spur;
                self.advance();
                Ok(self.emit_string(spur))
            }
            Token::True => {
                self.advance();
                Ok(self.emit_bool(true))
            }
            Token::False => {
                self.advance();
                Ok(self.emit_bool(false))
            }
            Token::Nil => {
                self.advance();
                Ok(self.emit_nil())
            }
            Token::Identifier(name) => {
                let name = *name;
                self.advance();
                Ok(self.emit_identifier(name))
            }
            Token::Minus => {
                self.advance();
                let inner = self.parse_precedence(PREC_UNARY)?;
                Ok(self.emit_unary("neg", inner))
            }
            Token::Bang => {
                self.advance();
                let inner = self.parse_precedence(PREC_UNARY)?;
                Ok(self.emit_unary("not", inner))
            }
            Token::LParen => {
                self.advance();
                let inner = self.expression()?;
                self.consume(&Token::RParen, "expected ')'")?;
                Ok(inner)
            }
            _ => {
                bail!("expected expression, found {}", self.current_name());
            }
        }
    }

    // ── Bytecode emission ──

    fn emit_number(&mut self, n: f64) -> u8 {
        let k = self.chunk.add_constant(Value::Number(n));
        let reg = self.alloc_reg();
        emit!(self.chunk, LOADK, reg, wide k);
        reg
    }

    fn emit_string(&mut self, spur: Spur) -> u8 {
        let s = self.lexer.extras.interner.resolve(&spur).to_string();
        let k = self.chunk.add_constant(Value::String(s));
        let reg = self.alloc_reg();
        emit!(self.chunk, LOADK, reg, wide k);
        reg
    }

    fn emit_bool(&mut self, b: bool) -> u8 {
        let reg = self.alloc_reg();
        emit!(self.chunk, LOADBOOL, reg, b as u8);
        reg
    }

    fn emit_nil(&mut self) -> u8 {
        let reg = self.alloc_reg();
        emit!(self.chunk, LOADNIL, reg);
        reg
    }

    fn emit_identifier(&mut self, _name: Spur) -> u8 {
        // TODO: variable lookup → register. See Phase 3.
        self.emit_nil()
    }

    fn emit_binary(&mut self, op: &Token, lhs: u8, rhs: u8) -> u8 {
        let method = binary_op_method(op);
        let name_idx = self.chunk.add_constant(Value::String(method.into()));
        let reg = self.alloc_reg();
        emit!(self.chunk, CALL, reg, wide name_idx, 2_u8, lhs, rhs);
        self.free_reg(lhs);
        self.free_reg(rhs);
        reg
    }

    fn emit_unary(&mut self, method: &str, operand: u8) -> u8 {
        let name_idx = self.chunk.add_constant(Value::String(method.into()));
        let reg = self.alloc_reg();
        emit!(self.chunk, CALL, reg, wide name_idx, 1_u8, operand);
        self.free_reg(operand);
        reg
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
