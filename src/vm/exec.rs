use std::ops::Range;

use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    functions::{FnType, FunctionRegistry},
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

fn locus_span(chunk: &Chunk) -> Range<usize> {
    chunk.locus_at(chunk.ip).cloned().unwrap_or(0..0)
}

#[derive(Debug)]
pub struct VM {
    pub(super) frames: Frames,
    globals: Vec<Value>,
    registry: FunctionRegistry,
}

impl Default for VM {
    fn default() -> Self {
        Self::new(FunctionRegistry::with_std())
    }
}

impl VM {
    pub fn new(registry: FunctionRegistry) -> Self {
        Self {
            frames: Frames::new(),
            globals: Vec::new(),
            registry,
        }
    }

    fn ensure_global(&mut self, slot: usize) {
        if slot >= self.globals.len() {
            self.globals.resize(slot + 1, Value::Nil);
        }
    }

    pub fn run(&mut self, chunk: &mut Chunk) -> Result<Value, RuntimeError> {
        use OpCode::*;

        chunk.reset_ip();
        loop {
            if chunk.ip >= chunk.code.len() {
                return Err(RuntimeError::IpOutOfBounds {
                    ip: chunk.ip,
                    span: locus_span(chunk),
                });
            }
            let op_byte = chunk.read();
            let op = OpCode::from_repr(op_byte).expect("Unrecognized opcode");
            match op {
                MOVE => {
                    let dst = chunk.read() as usize;
                    let src = chunk.read() as usize;
                    self.stack_mut()[dst] = self.stack()[src].clone();
                }
                LOADK => {
                    let dst = chunk.read() as usize;
                    let k = chunk.read_wide() as usize;
                    self.stack_mut()[dst] = chunk.constants.get(k as u16).clone();
                }
                LOADBOOL => {
                    let dst = chunk.read() as usize;
                    let val = chunk.read() != 0;
                    self.stack_mut()[dst] = Value::Bool(val);
                }
                LOADNIL => {
                    let dst = chunk.read() as usize;
                    self.stack_mut()[dst] = Value::Nil;
                }
                GETGLB => {
                    let dst = chunk.read() as usize;
                    let slot = chunk.read_wide() as usize;
                    self.ensure_global(slot);
                    self.stack_mut()[dst] = self.globals[slot].clone();
                }
                SETGLB => {
                    let slot = chunk.read_wide() as usize;
                    let src = chunk.read() as usize;
                    self.ensure_global(slot);
                    self.globals[slot] = self.stack()[src].clone();
                }
                CALL => {
                    let dst = chunk.read() as usize;
                    let name_idx = chunk.read_wide() as usize;
                    let offset = chunk.read() as usize;
                    let argc = chunk.read() as usize;

                    let name = chunk.constants.get(name_idx as u16);
                    let fn_id = match name {
                        Value::Fn(f) => f,
                        _ => {
                            return Err(RuntimeError::UndefinedFunction {
                                name: "this variable is not callable".into(),
                                span: locus_span(chunk),
                            });
                        }
                    };
                    let func = self.registry.get(fn_id).ok_or_else(|| {
                        RuntimeError::UndefinedFunction {
                            name: format!("{fn_id}"),
                            span: locus_span(chunk),
                        }
                    })?;

                    match func {
                        FnType::Intrinsic(func) => {
                            let window = &self.stack();
                            let mut args: SmallVec<[_; 8]> = SmallVec::with_capacity(argc);
                            for _ in 0..argc {
                                let val_ref = &window[chunk.read() as usize];
                                args.push(val_ref);
                            }

                            let result = func(&args);
                            drop(args);
                            self.stack_mut()[dst] = result;
                        }
                        FnType::Chunk(chunk_idx) => {
                            let chunk_idx = *chunk_idx;
                            for i in 1..=argc {
                                let window = &self.stack();
                                self.stack_mut()[offset + i] = window[chunk.read() as usize].clone();
                            }
                            self.enter_frame(chunk_idx, dst, offset);
                            unimplemented!("Bytecode function compilation not supported")
                        }
                    }
                }
                RETURN => {
                    let reg = chunk.read() as usize;
                    return Ok(self.stack()[reg].clone());
                }
                JMP => {
                    let ip = chunk.ip as isize - 1;
                    let offset = chunk.read_i16();
                    let new_ip = ip + offset as isize;
                    if new_ip < 0 || new_ip as usize >= chunk.code.len() {
                        return Err(RuntimeError::IpOutOfBounds {
                            ip: chunk.ip,
                            span: locus_span(chunk),
                        });
                    }
                    chunk.ip = new_ip as usize;
                }
                TEST => {
                    let ip = chunk.ip as isize - 1;
                    let reg = chunk.read() as usize;
                    let offset = chunk.read_i16();
                    if !self.stack()[reg].is_truthy() {
                        let new_ip = ip + offset as isize;
                        if new_ip < 0 || new_ip as usize >= chunk.code.len() {
                            return Err(RuntimeError::IpOutOfBounds {
                                ip: chunk.ip,
                                span: locus_span(chunk),
                            });
                        }
                        chunk.ip = new_ip as usize;
                    }
                }
                CLOSURE | NEWSTRUCT => {
                    return Err(RuntimeError::InvalidOpCode {
                        op_byte,
                        span: locus_span(chunk),
                    });
                }
            }
        }
    }
}
