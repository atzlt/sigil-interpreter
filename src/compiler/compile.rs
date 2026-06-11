use std::ops::Range;

use lasso::Spur;
use logos::Logos;
use thiserror::Error;

use crate::{
    compiler::{
        label::{Label, LabelTracker, RefKind},
        lexer::Token,
        loop_tracker::LoopTracker,
        register::RegisterTracker,
        variables::{GlobalStore, LocalsTracker},
    },
    functions::{FnLookupKey, FunctionRegistry},
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
    #[error("exiting top-level call frame")]
    ExitTopFrame(SpanInfo),
    #[error("undefined variable: {name}")]
    UndefinedVariable { name: String, diag: SpanInfo },
    #[error("undefined function: {name}")]
    UndefinedFunction { name: String, diag: SpanInfo },
}

pub type Result<T> = std::result::Result<T, CompileError>;

pub struct Compiler<'a> {
    lexer: logos::SpannedIter<'a, Token>,
    pub(super) chunks: Vec<Chunk>,
    pub(super) tokens: TokenCursor,
    pub(super) globals: GlobalStore,
    pub(super) funcs: FunctionRegistry,
    frames: Vec<CompilerFrame>,
}

pub(super) fn new_compiler(source: &str, funcs: FunctionRegistry) -> Compiler<'_> {
    Compiler {
        lexer: Token::lexer(source).spanned(),
        chunks: vec![Chunk::new()],
        tokens: TokenCursor::new(),
        globals: GlobalStore::default(),
        funcs,
        frames: vec![CompilerFrame::new(0, &[])],
    }
}

pub(super) fn compile(
    source: &str,
    funcs: FunctionRegistry,
    is_expr: bool,
) -> Result<(Vec<Chunk>, FunctionRegistry)> {
    let mut c = new_compiler(source, funcs);
    c.advance()?;
    if is_expr {
        let result_reg = c.expression(None)?;
        emit!(c.chunk_mut(), RETURN, result_reg);
    } else {
        while !c.check(&Token::Eof) {
            c.statement()?;
        }
        c.emit_safety_net()?;
    }
    Ok(c.take_compiled())
}

// ── Token handling ──

impl Compiler<'_> {
    pub fn take_compiled(self) -> (Vec<Chunk>, FunctionRegistry) {
        (self.chunks, self.funcs)
    }

    pub(super) fn advance(&mut self) -> Result<()> {
        self.tokens.advance(&mut self.lexer)
    }

    pub(super) fn peek(&mut self) -> Result<&Token> {
        self.tokens.peek(&mut self.lexer)
    }

    pub(super) fn check(&mut self, token: &Token) -> bool {
        self.tokens.check(token)
    }

    pub(super) fn current(&self) -> Token {
        *self.tokens.token()
    }

    pub(super) fn current_span(&self) -> &Span {
        self.tokens.span()
    }

    pub(super) fn prev_span(&self) -> &Span {
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

    pub(super) fn spur_eq(&mut self, spur: Spur, text: &str) -> bool {
        spur == self.lexer.extras.interner.get_or_intern(text)
    }

    pub(super) fn record_locus(&mut self) {
        let span = self.current_span().clone();
        self.chunk_mut().record_locus(span);
    }

    pub(super) fn frame_mut(&mut self) -> &mut CompilerFrame {
        let len = self.frames.len() - 1;
        &mut self.frames[len]
    }

    pub(super) fn frame(&self) -> &CompilerFrame {
        let len = self.frames.len() - 1;
        &self.frames[len]
    }

    pub(super) fn chunk_mut(&mut self) -> &mut Chunk {
        let id = self.frame().chunk_idx;
        &mut self.chunks[id]
    }

    pub(super) fn chunk(&self) -> &Chunk {
        &self.chunks[self.frame().chunk_idx]
    }

    pub(super) fn resolve_fn(&self, name: &FnLookupKey) -> Option<&usize> {
        self.funcs.get_id(name)
    }

    pub(super) fn emit_safety_net(&mut self) -> Result<()> {
        let nil_reg = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADNIL, nil_reg);
        emit!(self.chunk_mut(), RETURN, nil_reg);
        Ok(())
    }

    pub(super) fn emit_move(&mut self, dst: u8, src: u8) {
        if dst != src {
            emit!(self.chunk_mut(), MOVE, dst, src);
        }
    }

    pub(super) fn target_or_reuse(&mut self, target: Option<u8>, sources: &[u8]) -> Result<u8> {
        if let Some(t) = target {
            Ok(t)
        } else {
            self.reuse_or_alloc(sources)
        }
    }

    pub(super) fn emit_callk(&mut self, dst: u8, fn_slot: usize, frame_offset: u8, args: &[u8]) {
        let argc = args.len();
        emit!(self.chunk_mut(), CALLK, dst, wide fn_slot as u16, frame_offset, argc);
        self.chunk_mut().append(args);
    }

    pub(super) fn emit_call(&mut self, dst: u8, reg: u8, frame_offset: u8, args: &[u8]) {
        let argc = args.len();
        emit!(self.chunk_mut(), CALL, dst, reg, frame_offset, argc);
        self.chunk_mut().append(args);
    }

    pub(super) fn new_label(&mut self) -> Label {
        self.frame_mut().labels.alloc()
    }

    pub(super) fn emit_forward_test(&mut self, reg: u8) -> Label {
        let label = self.frame_mut().labels.alloc();
        let ip = self.chunk().end();
        emit!(self.chunk_mut(), TEST, reg, wide 0);
        self.frame_mut().labels.add_ref(label, ip, RefKind::Test);
        label
    }

    pub(super) fn emit_forward_jmp(&mut self) -> Label {
        let label = self.frame_mut().labels.alloc();
        let ip = self.chunk().end();
        emit!(self.chunk_mut(), JMP, wide 0);
        self.frame_mut().labels.add_ref(label, ip, RefKind::Jmp);
        label
    }

    pub(super) fn emit_forward_jmp_to(&mut self, label: Label) {
        let ip = self.chunk().end();
        emit!(self.chunk_mut(), JMP, wide 0);
        self.frame_mut().labels.add_ref(label, ip, RefKind::Jmp);
    }

    pub(super) fn emit_here_label(&mut self) -> Label {
        let label = self.frame_mut().labels.alloc();
        let ip = self.chunk().end();
        let chunk_idx = self.frame().chunk_idx;
        let labels = &mut self.frames.last_mut().unwrap().labels;
        labels.resolve(label, ip, &mut self.chunks[chunk_idx]);
        label
    }

    pub(super) fn emit_jmp_to(&mut self, label: Label) {
        let target = self.frame().labels.ip_of(label);
        let ip = self.chunk().end();
        let offset = target as isize - ip as isize;
        let bytes = (offset as i16).to_le_bytes();
        emit!(self.chunk_mut(), JMP, bytes[0], bytes[1]);
    }

    pub(super) fn place_label(&mut self, label: Label) {
        let target_ip = self.chunk().end();
        let chunk_idx = self.frame().chunk_idx;
        let labels = &mut self.frames.last_mut().unwrap().labels;
        labels.resolve(label, target_ip, &mut self.chunks[chunk_idx]);
    }

    /// Returns the chunk index.
    pub(super) fn new_frame(&mut self, args: &[Spur]) -> usize {
        self.chunks.push(Chunk::new());
        let chunk_idx = self.chunks.len() - 1;
        let frame = CompilerFrame::new(chunk_idx, args);
        self.frames.push(frame);
        chunk_idx
    }

    pub(super) fn exit_frame(&mut self) -> Result<()> {
        if self.frames.len() == 1 {
            return Err(CompileError::ExitTopFrame((
                self.current_span().clone(),
                "exiting from top-level call frame here".to_string(),
            )));
        }
        self.frames.pop();
        Ok(())
    }
}

// Compiler Frames

#[derive(Debug, Default)]
pub(super) struct CompilerFrame {
    pub(super) regs: RegisterTracker,
    pub(super) locals: LocalsTracker,
    pub(super) loops: LoopTracker,
    pub(super) labels: LabelTracker,
    chunk_idx: usize,
}

impl CompilerFrame {
    fn new(chunk_idx: usize, args: &[Spur]) -> Self {
        Self {
            regs: RegisterTracker::new_with(256, args.len()),
            locals: LocalsTracker::new_with(args),
            loops: LoopTracker::new(),
            labels: LabelTracker::default(),
            chunk_idx,
        }
    }
}
