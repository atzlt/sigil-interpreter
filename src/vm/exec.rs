use anyhow::Result;
use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    registry::FunctionRegistry,
    value::Value,
    vm::{Chunk, OpCode},
};

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("stack overflow")]
    StackOverflow,
    #[error("invalid opcode: 0x{0:02X}")]
    InvalidOpCode(u8),
    #[error("undefined function: {0}")]
    UndefinedFunction(String),
    #[error("instruction pointer out of bounds: {0}")]
    IpOutOfBounds(usize),
}

const STACK_SIZE: usize = 256;

#[derive(Debug)]
pub struct VM {
    pub stack: [Value; STACK_SIZE],
}

impl Default for VM {
    fn default() -> Self {
        VM {
            stack: std::array::from_fn(|_| Value::Nil),
        }
    }
}

impl VM {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self, chunk: &mut Chunk, registry: &FunctionRegistry) -> Result<Value> {
        use OpCode::*;

        chunk.reset_ip();
        loop {
            if chunk.ip >= chunk.code.len() {
                return Err(RuntimeError::IpOutOfBounds(chunk.ip).into());
            }
            let op_byte = chunk.read();
            let op = OpCode::from(op_byte);
            match op {
                MOVE => {
                    let dst = chunk.read() as usize;
                    let src = chunk.read() as usize;
                    self.stack[dst] = self.stack[src].clone();
                }
                LOADK => {
                    let dst = chunk.read() as usize;
                    let k = chunk.read_wide() as usize;
                    self.stack[dst] = chunk.constants.get(k as u16).clone();
                }
                LOADBOOL => {
                    let dst = chunk.read() as usize;
                    let val = chunk.read() != 0;
                    self.stack[dst] = Value::Bool(val);
                }
                LOADNIL => {
                    let dst = chunk.read() as usize;
                    self.stack[dst] = Value::Nil;
                }
                CALL => {
                    let dst = chunk.read() as usize;
                    let name_idx = chunk.read_wide() as usize;
                    let argc = chunk.read() as usize;

                    let name = chunk.constants.get(name_idx as u16);
                    let name_str = match name {
                        Value::String(s) => s.as_str(),
                        _ => {
                            return Err(
                                RuntimeError::UndefinedFunction("<not a string>".into()).into()
                            );
                        }
                    };
                    let func = registry
                        .get(name_str)
                        .ok_or_else(|| RuntimeError::UndefinedFunction(name_str.to_string()))?;
                    let mut args: SmallVec<[Value; 8]> = SmallVec::with_capacity(argc);
                    for _ in 0..argc {
                        args.push(self.stack[chunk.read() as usize].clone());
                    }
                    let result = func(&args);
                    self.stack[dst] = result;
                }
                RETURN => {
                    let reg = chunk.read() as usize;
                    return Ok(self.stack[reg].clone());
                }
                JMP => {
                    let ip = chunk.ip as isize - 1;
                    let offset = chunk.read_i16();
                    let new_ip = ip + offset as isize;
                    if new_ip < 0 || new_ip as usize >= chunk.code.len() {
                        return Err(RuntimeError::IpOutOfBounds(chunk.ip).into());
                    }
                    chunk.ip = new_ip as usize;
                }
                TEST => {
                    let ip = chunk.ip as isize - 1;
                    let reg = chunk.read() as usize;
                    let offset = chunk.read_i16();
                    if !self.stack[reg].is_truthy() {
                        let new_ip = ip + offset as isize;
                        if new_ip < 0 || new_ip as usize >= chunk.code.len() {
                            return Err(RuntimeError::IpOutOfBounds(chunk.ip).into());
                        }
                        chunk.ip = new_ip as usize;
                    }
                }
                CALLC | CLOSURE | NEWSTRUCT => {
                    return Err(RuntimeError::InvalidOpCode(op_byte).into());
                }
            }
        }
    }
}
