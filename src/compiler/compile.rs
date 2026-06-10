use std::ops::Range;

use lasso::Spur;
use logos::Logos;
use thiserror::Error;

use crate::{
    compiler::{
        lexer::Token, loop_tracker::LoopTracker, register::RegisterTracker, variables::Variables,
    },
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

// ── Token cursor ──

type Span = Range<usize>;
type SpanInfo = (Span, String);

pub(super) struct TokenCursor {
    current: (Token, Span),
    next: Option<(Token, Span)>,
    prev_span: Span,
}

impl TokenCursor {
    fn new() -> Self {
        Self {
            current: (Token::default(), Span::default()),
            next: None,
            prev_span: Span::default(),
        }
    }

    fn token(&self) -> &Token {
        &self.current.0
    }

    fn span(&self) -> &Span {
        &self.current.1
    }

    fn prev_span(&self) -> &Span {
        &self.prev_span
    }

    fn advance(&mut self, lexer: &mut logos::SpannedIter<Token>) -> Result<()> {
        self.prev_span = self.current.1.clone();
        if let Some(peeked) = self.next.take() {
            self.current = peeked;
            return Ok(());
        }
        let spanned = lexer
            .next()
            .or_else(|| Some((Ok(Token::Eof), lexer.span())))
            .unwrap();
        let token = spanned.0.map_err(|_| {
            CompileError::Unrecognized((spanned.1.clone(), "unexpected token".to_string()))
        })?;
        self.current = (token, spanned.1);
        Ok(())
    }

    fn peek(&mut self, lexer: &mut logos::SpannedIter<Token>) -> Result<&Token> {
        if self.next.is_none() {
            let spanned = lexer
                .next()
                .or_else(|| Some((Ok(Token::Eof), lexer.span())))
                .unwrap();
            let token = spanned.0.map_err(|_| {
                CompileError::Unrecognized((spanned.1.clone(), "unexpected token".to_string()))
            })?;
            self.next = Some((token, spanned.1));
        }
        Ok(&self.next.as_ref().unwrap().0)
    }

    fn check(&self, tok: &Token) -> bool {
        std::mem::discriminant(&self.current.0) == std::mem::discriminant(tok)
    }
}

// ── Compiler ──

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
    pub(super) chunks: Vec<Chunk>,
    pub(super) tokens: TokenCursor,
    frames: Vec<CompilerFrame>,
    frame_ptr: usize,
}

fn new_compiler(source: &str) -> Compiler<'_> {
    Compiler {
        lexer: Token::lexer(source).spanned(),
        chunks: vec![Chunk::new()],
        tokens: TokenCursor::new(),
        frames: vec![CompilerFrame::new(0)],
        frame_ptr: 0,
    }
}

pub fn compile_expr(source: &str) -> Result<Chunk> {
    let mut c = new_compiler(source);
    c.advance()?;
    let result_reg = c.expression(None)?;
    emit!(c.chunk_mut(), RETURN, result_reg);

    // Temporary code
    let mut chunk = c.chunks;
    Ok(chunk.pop().unwrap())
}

pub fn compile_program(source: &str) -> Result<Chunk> {
    let mut c = new_compiler(source);
    c.advance()?;
    while !c.check(&Token::Eof) {
        c.statement()?;
    }
    let nil_reg = c.alloc_temp()?;
    emit!(c.chunk_mut(), LOADNIL, nil_reg);
    emit!(c.chunk_mut(), RETURN, nil_reg);
    
    // Temporary code
    let mut chunk = c.chunks;
    Ok(chunk.pop().unwrap())
}

// ── Token handling ──

impl Compiler<'_> {
    pub(super) fn advance(&mut self) -> Result<()> {
        self.tokens.advance(&mut self.lexer)
    }

    pub(super) fn peek(&mut self) -> Result<&Token> {
        self.tokens.peek(&mut self.lexer)
    }

    pub(super) fn check(&mut self, token: &Token) -> bool {
        self.tokens.check(token)
    }

    pub(super) fn current(&mut self) -> Token {
        *self.tokens.token()
    }

    pub(super) fn current_span(&mut self) -> &Span {
        self.tokens.span()
    }

    pub(super) fn prev_span(&mut self) -> &Span {
        self.tokens.prev_span()
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
                token: self.current(),
                diag: (
                    self.current_span().clone(),
                    format!("expected {expected}, found {}", self.current()),
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
                    self.current_span().clone(),
                    format!("expected {expected}, found {}", self.current()),
                ),
            })
        }
    }
}

// Helper functions

impl Compiler<'_> {
    pub(super) fn intern_resolve(&self, spur: &Spur) -> &str {
        self.lexer.extras.interner.resolve(spur)
    }

    pub(super) fn record_locus(&mut self) {
        let span = self.current_span().clone();
        self.chunk_mut().record_locus(span);
    }

    pub(super) fn frame_mut(&mut self) -> &mut CompilerFrame {
        &mut self.frames[self.frame_ptr]
    }

    pub(super) fn frame(&self) -> &CompilerFrame {
        &self.frames[self.frame_ptr]
    }

    pub(super) fn chunk_mut(&mut self) -> &mut Chunk {
        let id = self.frame().chunk_id;
        &mut self.chunks[id]
    }

    pub(super) fn chunk(&self) -> &Chunk {
        &self.chunks[self.frame().chunk_id]
    }

    pub(super) fn emit_move(&mut self, dst: u8, src: u8) {
        if dst != src {
            emit!(self.chunk_mut(), MOVE, dst, src);
        }
    }

    pub(super) fn emit_test(&mut self, lhs: u8) -> usize {
        let ip = self.chunk_mut().end();
        emit!(self.chunk_mut(), TEST, lhs, wide 0);
        ip
    }

    pub(super) fn emit_jmp(&mut self) -> usize {
        let ip = self.chunk_mut().end();
        emit!(self.chunk_mut(), JMP, wide 0);
        ip
    }

    pub(super) fn emit_jump_offset(&mut self, offset: isize) {
        let offset = (offset as i16).to_le_bytes();
        emit!(self.chunk_mut(), JMP, offset[0], offset[1]);
    }

    pub(super) fn patch_if(&mut self, test_ip: usize, if_end: usize) {
        self.chunk_mut()
            .patch_wide(test_ip + 2, (if_end - test_ip) as u16);
    }

    pub(super) fn patch_if_else(
        &mut self,
        test_ip: usize,
        if_end: usize,
        else_start: usize,
        else_end: usize,
    ) {
        self.chunk_mut()
            .patch_wide(if_end + 1, (else_end - if_end) as u16);
        self.chunk_mut()
            .patch_wide(test_ip + 2, (else_start - test_ip) as u16);
    }
}

// Compiler Frames

#[derive(Debug, Default)]
pub(super) struct CompilerFrame {
    pub(super) regs: RegisterTracker,
    pub(super) vars: Variables,
    pub(super) loops: LoopTracker,
    chunk_id: usize,
}

impl CompilerFrame {
    fn new(chunk_id: usize) -> Self {
        Self {
            regs: RegisterTracker::new(256),
            vars: Variables::default(),
            loops: LoopTracker::new(),
            chunk_id,
        }
    }
}
