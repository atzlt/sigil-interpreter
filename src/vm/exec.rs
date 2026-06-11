use std::ops::Range;

use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    functions::{FnEntry, FunctionRegistry},
    value::Value,
    vm::{Chunk, OpCode, frame::Frames},
};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("stack overflow")]
    StackOverflow,
    #[error("invalid opcode: 0x{op_byte:02X} at {}..{}", span.start, span.end)]
    InvalidOpCode { op_byte: u8, span: Range<usize> },
    #[error("undefined function: {name} at {}..{}", span.start, span.end)]
    UndefinedFunction { name: String, span: Range<usize> },
    #[error("instruction pointer out of bounds: {ip} at {}..{}", span.start, span.end)]
    IpOutOfBounds { ip: usize, span: Range<usize> },
}

fn locus_span(chunk: &Chunk, ip: usize) -> Range<usize> {
    chunk.locus_at(ip).cloned().unwrap_or(0..0)
}

#[derive(Debug)]
pub struct VM<'c> {
    pub(super) frames: Frames<'c>,
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
            frames: Frames::new(),
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

        self.frames.init_main(&chunks[0]);
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

                    let reg = &self.stack()[reg];
                    let fn_id = match reg {
                        Value::Fn(f) => *f,
                        _ => {
                            return Err(RuntimeError::UndefinedFunction {
                                name: "this variable is not callable".into(),
                                span: locus_span(self.chunk(), self.ip()),
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
                            span: locus_span(self.chunk(), self.ip()),
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
                                span: locus_span(self.chunk(), self.ip()),
                            });
                        }
                        self.set_ip(new_ip as usize);
                    }
                }
                CLOSURE | NEWSTRUCT => {
                    let span = locus_span(self.chunk(), self.ip());
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
                span: locus_span(self.chunk(), self.ip()),
            })?;
        let mut regs: SmallVec<[_; 8]> = SmallVec::with_capacity(argc);
        for _ in 0..argc {
            let arg = self.read();
            regs.push(arg as usize);
        }

        match func {
            FnEntry::Intrinsic(func) => {
                let mut args: SmallVec<[_; 8]> = SmallVec::with_capacity(argc);

                for i in 0..argc {
                    args.push(self.stack_index(regs[i]));
                }

                let result = func(&args);
                drop(args);
                self.stack_mut()[dst] = result;
            }
            FnEntry::ChunkIdx(chunk_idx) => {
                let chunk = &chunks[*chunk_idx];
                for i in 0..argc {
                    self.stack_mut()[offset + i + 1] = self.stack_index(regs[i]).clone();
                }
                self.enter_frame(chunk, dst, offset);
            }
        }
        Ok(())
    }
}
