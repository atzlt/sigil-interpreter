use std::ops::Range;

use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    functions::{FnEntry, FunctionRegistry, IntrinsicContext},
    value::Value,
    vm::{
        Chunk, OpCode,
        frame::{CallFrame, StackWindow, StackWindowMut},
        heap::{Heap, StructObject},
        upvalue::Upvalue,
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
    #[error("no matching overload for '{name}'")]
    NoMatchingOverload { name: String, span: Range<usize> },
    #[error("instruction pointer out of bounds: {ip}")]
    IpOutOfBounds { ip: usize, span: Range<usize> },
}

#[derive(Debug)]
pub struct VM<'c> {
    stack: Vec<Value>,
    frames: Vec<CallFrame<'c>>,
    globals: Vec<Value>,
    heap: Heap,
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
            heap: Heap::new(),
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
                    self.stack_mut()[dst] = Value::Fn(fun);
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

                    let reg_val = &self.stack()[reg];
                    let (fn_id, upvalues) = match reg_val {
                        Value::Fn(f) => (*f, SmallVec::new()),
                        Value::Closure { fn_id, upvalues } => (*fn_id, upvalues.clone()),
                        _ => {
                            return Err(RuntimeError::UndefinedFunction {
                                name: "this variable is not callable".into(),
                                span: self.locus_span(),
                            });
                        }
                    };

                    self.handle_call(chunks, registry, fn_id, argc, dst, offset, upvalues)?;
                }
                CALLK => {
                    let dst: usize = self.read() as usize;
                    let fn_id = self.read_wide() as usize;
                    let offset = self.read() as usize;
                    let argc = self.read() as usize;

                    self.handle_call(chunks, registry, fn_id, argc, dst, offset, SmallVec::new())?;
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
                CLOSURE => {
                    let dst = self.read() as usize;
                    let proto_idx = self.read_wide() as usize;

                    let (fn_id, upvalue_count) = match self.chunk().constants.get(proto_idx as u16)
                    {
                        Value::FnProto {
                            fn_id,
                            upvalue_count,
                        } => (*fn_id, *upvalue_count as usize),
                        _ => {
                            let span = self.locus_span();
                            return Err(RuntimeError::UndefinedFunction {
                                name: "this variable is not a function prototype".into(),
                                span,
                            });
                        }
                    };

                    let cur_offset = self.frames.last().unwrap().reg_offset;
                    let mut upvalue_indices: SmallVec<[u16; 4]> = SmallVec::new();

                    for _ in 0..upvalue_count {
                        let is_local = self.read() != 0;
                        let index = self.read() as usize;

                        if is_local {
                            // Capture from the current frame's stack.
                            let abs_pos = cur_offset + index;
                            let key = self.heap.push_upvalue(Upvalue::Open(abs_pos));
                            upvalue_indices.push(key);
                            self.insert_open_upvalue_sorted(key, abs_pos);
                        } else {
                            // Transitive capture — copy the upvalue key from
                            // the enclosing closure.
                            let parent_up = self.frames.last().unwrap().closure_upvalues[index];
                            upvalue_indices.push(parent_up);
                        }
                    }

                    self.stack_mut()[dst] = Value::Closure {
                        fn_id,
                        upvalues: upvalue_indices,
                    };
                }
                NEWSTRUCT => {
                    let dst = self.read() as usize;
                    let def_id = self.read_wide();
                    let count = self.read() as usize;

                    let mut field_names: Vec<String> = Vec::with_capacity(count);
                    let mut field_values: Vec<Value> = Vec::with_capacity(count);

                    for _ in 0..count {
                        let name_k = self.read_wide() as usize;
                        let reg = self.read() as usize;
                        let name = match self.chunk().constants.get(name_k as u16) {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        field_names.push(name);
                        field_values.push(self.stack()[reg].clone());
                    }

                    self.heap.ensure_struct_def(def_id, field_names);
                    let key = self.heap.push_struct(StructObject {
                        def_id,
                        fields: field_values.into_boxed_slice(),
                    });
                    self.stack_mut()[dst] = Value::Struct(key);
                }
                GETFIELD => {
                    let dst = self.read() as usize;
                    let obj_reg = self.read() as usize;
                    let name_k = self.read_wide() as usize;

                    let field_name = match self.chunk().constants.get(name_k as u16) {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };

                    let key = match &self.stack()[obj_reg] {
                        Value::Struct(k) => *k,
                        _ => {
                            return Err(RuntimeError::InvalidOpCode {
                                op_byte,
                                span: self.locus_span(),
                            });
                        }
                    };

                    let sobj = self.heap.struct_ref(key);
                    let idx = self
                        .heap
                        .struct_field_index(sobj.def_id, &field_name)
                        .ok_or_else(|| RuntimeError::InvalidOpCode {
                            op_byte,
                            span: self.locus_span(),
                        })?;

                    self.stack_mut()[dst] = sobj.fields[idx].clone();
                }
                SETFIELD => {
                    let obj_reg = self.read() as usize;
                    let name_k = self.read_wide() as usize;
                    let src_reg = self.read() as usize;

                    let field_name = match self.chunk().constants.get(name_k as u16) {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };

                    let key = match &self.stack()[obj_reg] {
                        Value::Struct(k) => *k,
                        _ => {
                            return Err(RuntimeError::InvalidOpCode {
                                op_byte,
                                span: self.locus_span(),
                            });
                        }
                    };

                    let sobj = self.heap.struct_ref(key);
                    let idx = self
                        .heap
                        .struct_field_index(sobj.def_id, &field_name)
                        .ok_or_else(|| RuntimeError::InvalidOpCode {
                            op_byte,
                            span: self.locus_span(),
                        })?;

                    self.heap.struct_mut(key).fields[idx] = self.stack()[src_reg].clone();
                }
                GETUPVAL => {
                    let dst = self.read() as usize;
                    let idx = self.read_wide() as usize;
                    let abs_key = self.frames.last().unwrap().closure_upvalues[idx] as usize;
                    let val = match self.heap.upvalue(abs_key as u16) {
                        Upvalue::Open(pos) => self.stack[*pos].clone(),
                        Upvalue::Closed(v) => v.clone(),
                    };
                    self.stack_mut()[dst] = val;
                }
                SETUPVAL => {
                    let src = self.read() as usize;
                    let idx = self.read_wide() as usize;
                    let abs_key = self.frames.last().unwrap().closure_upvalues[idx] as usize;
                    let val = self.stack()[src].clone();
                    match self.heap.upvalue_mut(abs_key as u16) {
                        Upvalue::Open(pos) => self.stack[*pos] = val,
                        Upvalue::Closed(v) => *v = val,
                    }
                }
            }
        }
    }

    fn handle_call(
        &mut self,
        chunks: &'c [Chunk],
        registry: &FunctionRegistry,
        static_id: usize,
        argc: usize,
        dst: usize,
        offset: usize,
        closure_upvalues: SmallVec<[u16; 4]>,
    ) -> Result<(), RuntimeError> {
        let regs = self.read_bytes(argc);
        let reg_offset = self.frames.last().unwrap().reg_offset;

        let args: SmallVec<[&Value; 4]> = regs
            .iter()
            .map(|&r| &self.stack[reg_offset + r as usize])
            .collect();

        let key = registry.resolve_id(static_id);
        let resolved_id = registry
            .resolve_overload(&key, &args, &self.heap)
            .ok_or_else(|| RuntimeError::NoMatchingOverload {
                name: key.to_string(),
                span: self.locus_span(),
            })?;

        let func = registry
            .get(&resolved_id)
            .ok_or_else(|| RuntimeError::UndefinedFunction {
                name: format!("{}", key),
                span: self.locus_span(),
            })?;

        match func {
            FnEntry::Intrinsic(func) => {
                let ctx = IntrinsicContext { heap: &self.heap };
                let result = func(&args, &ctx);
                drop(args);
                self.stack_mut()[dst] = result;
            }
            FnEntry::ChunkIdx(chunk_idx) => {
                drop(args);
                let chunk = &chunks[*chunk_idx];
                for (i, &reg) in regs.iter().enumerate() {
                    let src = reg_offset + reg as usize;
                    let dst_abs = reg_offset + offset + i + 1;
                    if src != dst_abs {
                        self.stack[dst_abs] = self.stack[src].clone();
                    }
                }
                self.enter_frame(chunk, dst, offset, closure_upvalues)?;
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
        closure_upvalues: SmallVec<[u16; 4]>,
    ) -> Result<(), RuntimeError> {
        let cur_offset = self.frames.last().unwrap().reg_offset;
        let ret_dst_abs = cur_offset + ret_dst;
        let reg_offset_abs = cur_offset + reg_offset + 1;
        self.ensure_stack(reg_offset_abs + 255)?;
        let frame = CallFrame::new(ret_dst_abs, reg_offset_abs, chunk, closure_upvalues);
        self.frames.push(frame);
        Ok(())
    }

    pub fn init_main(&mut self, chunk: &'c Chunk) -> Result<(), RuntimeError> {
        self.ensure_stack(STACK_INIT - 1)?;
        self.frames
            .push(CallFrame::new(0, 0, chunk, SmallVec::new()));
        Ok(())
    }

    pub(super) fn enter_frame(
        &mut self,
        chunk: &'c Chunk,
        ret_dst: usize,
        reg_offset: usize,
        closure_upvalues: SmallVec<[u16; 4]>,
    ) -> Result<(), RuntimeError> {
        self.new_frame(chunk, ret_dst, reg_offset, closure_upvalues)?;
        Ok(())
    }

    pub(super) fn exit_frame(&mut self, res_reg: usize) -> Option<Value> {
        let dst = self.frames.last().unwrap().ret_dst;
        let ret_val = self.stack()[res_reg].clone();

        // Close every open upvalue that points into this frame's stack.
        let open_keys: SmallVec<[u16; 4]> = self
            .frames
            .last()
            .unwrap()
            .open_upvalues
            .iter()
            .copied()
            .collect();
        for key in open_keys {
            if let Upvalue::Open(pos) = self.heap.upvalue(key) {
                let val = self.stack[*pos].clone();
                *self.heap.upvalue_mut(key) = Upvalue::Closed(val);
            }
        }

        self.frames.pop();
        if self.frames.is_empty() {
            Some(ret_val)
        } else {
            self.stack[dst] = ret_val;
            None
        }
    }

    /// Insert an open-upvalue key into the current frame's list, keeping it
    /// sorted by the absolute stack position the upvalue references.
    fn insert_open_upvalue_sorted(&mut self, key: u16, abs_pos: usize) {
        let frame = self.frames.last_mut().unwrap();
        let pos = frame
            .open_upvalues
            .binary_search_by(|&k| {
                let up = self.heap.upvalue(k);
                let p = match up {
                    Upvalue::Open(p) => *p,
                    Upvalue::Closed(_) => usize::MAX,
                };
                p.cmp(&abs_pos)
            })
            .unwrap_or_else(|e| e);
        frame.open_upvalues.insert(pos, key);
    }

    fn locus_span(&self) -> Range<usize> {
        let frame = self.frames.last().unwrap();
        frame.chunk.locus_at(frame.ip).cloned().unwrap_or(0..0)
    }
}
