use std::ops::Range;

use lasso::Spur;
use logos::Logos;
use thiserror::Error;

use crate::{
    compiler::{lexer::Token, locals::LocalsTracker, register::RegisterTracker},
    vm::Chunk,
};

#[macro_export]
macro_rules! emit {
    ($chunk:expr, $op:ident) => {
        $chunk.emit_opcode($crate::vm::OpCode::$op);
    };
    ($chunk:expr, $op:ident, $($args:tt)*) => {
        $chunk.emit_opcode($crate::vm::OpCode::$op);
        emit_args!($chunk, $($args)*);
    };
}

#[macro_export]
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

// ── Compiler ──

type Span = Range<usize>;
type SpanInfo = (Span, String);

#[derive(Error, Debug, Clone)]
pub enum CompileError {
    #[error("unexpected token: {token}")]
    Unexpected { token: Token, diag: SpanInfo },
    #[error("unclosed delimiter")]
    Unclosed { open: SpanInfo, close: SpanInfo },
    #[error("unrecognized token")]
    Unrecognized(SpanInfo),
    #[error("register overflow")]
    RegisterOverflow(SpanInfo),
    #[error("undefined variable: {name}")]
    UndefinedVariable { name: String, diag: SpanInfo },
}

pub type Result<T> = std::result::Result<T, CompileError>;

pub struct Compiler<'a> {
    lexer: logos::SpannedIter<'a, Token>,
    pub(super) chunk: Chunk,
    pub(super) current: (Token, Span),
    pub(super) regs: RegisterTracker,
    pub(super) locals: LocalsTracker,
}

pub fn compile(source: &str) -> Result<Chunk> {
    let mut c = Compiler {
        lexer: Token::lexer(source).spanned(),
        chunk: Chunk::new(),
        current: (Token::default(), Span::default()),
        regs: RegisterTracker::new(256),
        locals: LocalsTracker::new(),
    };
    c.advance()?;
    let result_reg = c.expression()?;
    emit!(c.chunk, RETURN, result_reg, 1_u8);
    Ok(c.chunk)
}

impl Compiler<'_> {
    // ── Token handling ──

    pub(super) fn advance(&mut self) -> Result<()> {
        let spanned = self
            .lexer
            .next()
            .or_else(|| Some((Ok(Token::Eof), self.lexer.span())))
            .unwrap();
        let token = spanned.0.map_err(|_| {
            CompileError::Unrecognized((spanned.1.clone(), "unexpected token".to_string()))
        })?;
        self.current = (token, spanned.1);
        Ok(())
    }

    pub(super) fn check(&self, tok: &Token) -> bool {
        std::mem::discriminant(&self.current.0) == std::mem::discriminant(tok)
    }

    pub(super) fn matches(&mut self, tok: &Token) -> Result<bool> {
        if self.check(tok) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(super) fn consume(&mut self, expected: &Token) -> Result<()> {
        if self.check(expected) {
            self.advance()?;
            Ok(())
        } else {
            Err(CompileError::Unexpected {
                token: self.current.0,
                diag: (
                    self.current.1.clone(),
                    format!("expected {expected}, found {}", self.current.0),
                ),
            })
        }
    }

    pub(super) fn consume_close(&mut self, expected: &Token, open_span: Span) -> Result<()> {
        if self.check(expected) {
            self.advance()?;
            Ok(())
        } else {
            Err(CompileError::Unclosed {
                open: (open_span, "opened here".to_string()),
                close: (
                    self.current.1.clone(),
                    format!("expected {expected}, found {}", self.current.0),
                ),
            })
        }
    }

    pub(super) fn intern_resolve(&self, spur: &Spur) -> &str {
        self.lexer.extras.interner.resolve(spur)
    }

    // Helper functions

    pub(super) fn emit_test(&mut self, lhs: u8) -> usize {
        emit!(self.chunk, TEST, lhs, wide 0);
        self.chunk.last_wide()
    }

    pub(super) fn patch_conditional(
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
}
