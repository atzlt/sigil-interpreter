use std::ops::Range;

use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    functions::{FnEntry, FunctionRegistry},
    value::Value,
    vm::{
        Chunk, OpCode,
        frame::{CallFrame, StackWindow, StackWindowMut},
    },
};

const MAX_CALL_DEPTH: usize = 256;
const STACK_GROW: usize = 256;
const STACK_INIT: usize = 256;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("stack overflow")]
    StackOverflow { span: Range<usize> },
    #[error("invalid opcode: 0x{op_byte:02X}")]
    InvalidOpCode { op_byte: u8, span: Range<usize> },
    #[error("undefined function: {name}")]
    UndefinedFunction { name: String, span: Range<usize> },
    #[error("instruction pointer out of bounds: {ip}")]
    IpOutOfBounds { ip: usize, span: Range<usize> },
}

#[derive(Debug)]
pub struct VM<'c> {
    stack: Vec<Value>,
    frames: Vec<CallFrame<'c>>,
    globals: Vec<Value>,
}

impl Default for VM<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'c> VM<'c> {
    pub fn new() -> Self {
        Self {
            stack: vec![Value::Nil; STACK_INIT],
            frames: Vec::new(),
            globals: Vec::new(),
        }
    }

    fn ensure_global(&mut self, slot: usize) {
        if slot >= self.globals.len() {
            self.globals.resize(slot + 1, Value::Nil);
        }
    }

    pub fn run(
        &'c mut self,
        chunks: &'c [Chunk],
        registry: &FunctionRegistry,
    ) -> Result<Value, RuntimeError> {
        use OpCode::*;

        self.init_main(&chunks[0])?;
        loop {
            let op_byte = self.read();
            let op = OpCode::from_repr(op_byte).expect("Unrecognized opcode");
            match op {
                MOVE => {
                    let dst = self.read() as usize;
                    let src = self.read() as usize;
                    self.stack_mut()[dst] = self.stack()[src].clone();
                }
                LOADK => {
                    let dst = self.read() as usize;
                    let k = self.read_wide() as usize;
                    self.stack_mut()[dst] = self.chunk().constants.get(k as u16).clone();
                }
                LOADBOOL => {
                    let dst = self.read() as usize;
                    let val = self.read() != 0;
                    self.stack_mut()[dst] = Value::Bool(val);
                }
                LOADNIL => {
                    let dst = self.read() as usize;
                    self.stack_mut()[dst] = Value::Nil;
                }
                LOADFUN => {
                    let dst = self.read() as usize;
                    let fun = self.read_wide() as usize;
                    self.stack_mut()[dst] = Value::Fn {
                        fn_id: fun,
                        upvalues: smallvec::SmallVec::new(),
                    };
                }
                GETGLB => {
                    let dst = self.read() as usize;
                    let slot = self.read_wide() as usize;
                    self.ensure_global(slot);
                    self.stack_mut()[dst] = self.globals[slot].clone();
                }
                SETGLB => {
                    let slot = self.read_wide() as usize;
                    let src = self.read() as usize;
                    self.ensure_global(slot);
                    self.globals[slot] = self.stack()[src].clone();
                }
                CALL => {
                    let dst: usize = self.read() as usize;
                    let reg = self.read() as usize;
                    let offset = self.read() as usize;
                    let argc = self.read() as usize;

                    let reg = &self.stack()[reg];
                    let fn_id = match reg {
                        Value::Fn { fn_id, .. } => *fn_id,
                        _ => {
                            return Err(RuntimeError::UndefinedFunction {
                                name: "this variable is not callable".into(),
                                span: self.locus_span(),
                            });
                        }
                    };

                    self.handle_call(chunks, registry, fn_id, argc, dst, offset)?;
                }
                CALLK => {
                    let dst: usize = self.read() as usize;
                    let fn_id = self.read_wide() as usize;
                    let offset = self.read() as usize;
                    let argc = self.read() as usize;

                    self.handle_call(chunks, registry, fn_id, argc, dst, offset)?;
                }
                RETURN => {
                    let reg = self.read() as usize;
                    let from_top_level = self.exit_frame(reg);
                    if let Some(ret) = from_top_level {
                        return Ok(ret);
                    }
                }
                JMP => {
                    let ip = self.ip() as isize - 1;
                    let offset = self.read_i16();
                    let new_ip = ip + offset as isize;
                    if new_ip < 0 || new_ip as usize >= self.chunk().code.len() {
                        return Err(RuntimeError::IpOutOfBounds {
                            ip: self.ip(),
                            span: self.locus_span(),
                        });
                    }
                    self.set_ip(new_ip as usize);
                }
                TEST => {
                    let ip = self.ip() as isize - 1;
                    let reg = self.read() as usize;
                    let offset = self.read_i16();
                    if !self.stack()[reg].is_truthy() {
                        let new_ip = ip + offset as isize;
                        if new_ip < 0 || new_ip as usize >= self.chunk().code.len() {
                            return Err(RuntimeError::IpOutOfBounds {
                                ip: self.ip(),
                                span: self.locus_span(),
                            });
                        }
                        self.set_ip(new_ip as usize);
                    }
                }
                CLOSURE | NEWSTRUCT => {
                    let span = self.locus_span();
                    return Err(RuntimeError::InvalidOpCode { op_byte, span });
                }
            }
        }
    }

    fn handle_call(
        &mut self,
        chunks: &'c [Chunk],
        registry: &FunctionRegistry,
        fn_id: usize,
        argc: usize,
        dst: usize,
        offset: usize,
    ) -> Result<(), RuntimeError> {
        let func = registry
            .get(&fn_id)
            .ok_or_else(|| RuntimeError::UndefinedFunction {
                name: format!("{}", registry.resolve_id(fn_id)),
                span: self.locus_span(),
            })?;

        let regs = self.read_bytes(argc);
        match func {
            FnEntry::Intrinsic(func) => {
                let reg_offset = self.frames.last().unwrap().reg_offset;
                let mut arg_refs: SmallVec<[&Value; 4]> = SmallVec::with_capacity(argc);
                for &reg in regs {
                    arg_refs.push(&self.stack[reg_offset + reg as usize]);
                }
                let result = func(&arg_refs);
                drop(arg_refs);
                self.stack_mut()[dst] = result;
            }
            FnEntry::ChunkIdx(chunk_idx) => {
                let chunk = &chunks[*chunk_idx];
                let reg_offset = self.frames.last().unwrap().reg_offset;
                for (i, &reg) in regs.iter().enumerate() {
                    let src = reg_offset + reg as usize;
                    let dst_abs = reg_offset + offset + i + 1;
                    if src != dst_abs {
                        self.stack[dst_abs] = self.stack[src].clone();
                    }
                }
                self.enter_frame(chunk, dst, offset)?;
            }
        }
        Ok(())
    }

    // ── Bytecode reading ──

    fn read_bytes(&mut self, count: usize) -> &'c [u8] {
        let frame = self.frames.last_mut().unwrap();
        let start = frame.ip;
        frame.ip = start + count;
        &frame.chunk.code[start..frame.ip]
    }

    pub(super) fn read(&mut self) -> u8 {
        let frame = self.frames.last_mut().unwrap();
        let byte = frame.chunk.code[frame.ip];
        frame.ip += 1;
        byte
    }

    pub(super) fn read_i16(&mut self) -> i16 {
        let frame = self.frames.last_mut().unwrap();
        let bytes = [frame.chunk.code[frame.ip], frame.chunk.code[frame.ip + 1]];
        frame.ip += 2;
        i16::from_le_bytes(bytes)
    }

    pub(super) fn read_wide(&mut self) -> u16 {
        let frame = self.frames.last_mut().unwrap();
        let bytes = [frame.chunk.code[frame.ip], frame.chunk.code[frame.ip + 1]];
        frame.ip += 2;
        u16::from_le_bytes(bytes)
    }

    pub(super) fn ip(&self) -> usize {
        self.frames.last().unwrap().ip
    }

    pub(super) fn set_ip(&mut self, new: usize) {
        self.frames.last_mut().unwrap().ip = new
    }

    pub(super) fn chunk(&self) -> &Chunk {
        self.frames.last().unwrap().chunk
    }

    // ── Stack access ──

    pub(super) fn stack(&self) -> StackWindow<'_> {
        let offset = self.frames.last().unwrap().reg_offset;
        StackWindow::new(&self.stack, offset)
    }

    pub(super) fn stack_mut(&mut self) -> StackWindowMut<'_> {
        let offset = self.frames.last().unwrap().reg_offset;
        StackWindowMut::new(&mut self.stack, offset)
    }

    // ── Frame management ──

    fn ensure_stack(&mut self, abs_idx: usize) -> Result<(), RuntimeError> {
        if abs_idx >= self.stack.len() {
            let new_len = ((abs_idx + STACK_GROW) / STACK_GROW) * STACK_GROW;
            self.stack.resize(new_len, Value::Nil);
        }

        if self.stack.len() >= MAX_CALL_DEPTH * STACK_INIT {
            Err(RuntimeError::StackOverflow {
                span: self.locus_span(),
            })
        } else {
            Ok(())
        }
    }

    fn new_frame(
        &mut self,
        chunk: &'c Chunk,
        ret_dst: usize,
        reg_offset: usize,
    ) -> Result<(), RuntimeError> {
        let cur_offset = self.frames.last().unwrap().reg_offset;
        let ret_dst_abs = cur_offset + ret_dst;
        let reg_offset_abs = cur_offset + reg_offset + 1;
        self.ensure_stack(reg_offset_abs + 255)?;
        let frame = CallFrame::new(ret_dst_abs, reg_offset_abs, chunk);
        self.frames.push(frame);
        Ok(())
    }

    pub fn init_main(&mut self, chunk: &'c Chunk) -> Result<(), RuntimeError> {
        self.ensure_stack(STACK_INIT - 1)?;
        self.frames.push(CallFrame::new(0, 0, chunk));
        Ok(())
    }

    pub(super) fn enter_frame(
        &mut self,
        chunk: &'c Chunk,
        ret_dst: usize,
        reg_offset: usize,
    ) -> Result<(), RuntimeError> {
        self.new_frame(chunk, ret_dst, reg_offset)?;
        Ok(())
    }

    pub(super) fn exit_frame(&mut self, res_reg: usize) -> Option<Value> {
        let dst = self.frames.last().unwrap().ret_dst;
        let ret_val = self.stack()[res_reg].clone();
        self.frames.pop();
        if self.frames.is_empty() {
            Some(ret_val)
        } else {
            self.stack[dst] = ret_val;
            None
        }
    }

    fn locus_span(&self) -> Range<usize> {
        let frame = self.frames.last().unwrap();
        frame.chunk.locus_at(frame.ip).cloned().unwrap_or(0..0)
    }
}
