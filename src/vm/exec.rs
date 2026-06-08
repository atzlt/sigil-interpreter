use anyhow::Result;
use smallvec::SmallVec;

use crate::{
    error::RuntimeError,
    registry::FunctionRegistry,
    value::Value,
    vm::{Chunk, OpCode},
};

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
            let op_byte = chunk.read_u8();
            let op = OpCode::from(op_byte);
            match op {
                MOVE => {
                    let dst = chunk.read_u8() as usize;
                    let src = chunk.read_u8() as usize;
                    self.stack[dst] = self.stack[src].clone();
                }
                LOADK => {
                    let dst = chunk.read_u8() as usize;
                    let k = chunk.read_u16() as usize;
                    self.stack[dst] = chunk.constants.get(k as u16).clone();
                }
                LOADBOOL => {
                    let dst = chunk.read_u8() as usize;
                    let val = chunk.read_u8() != 0;
                    self.stack[dst] = Value::Bool(val);
                }
                LOADNIL => {
                    let dst = chunk.read_u8() as usize;
                    self.stack[dst] = Value::Nil;
                }
                CALL => {
                    let dst = chunk.read_u8() as usize;
                    let name_idx = chunk.read_u16() as usize;
                    let argc = chunk.read_u8() as usize;

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
                        args.push(self.stack[chunk.read_u8() as usize].clone());
                    }
                    let result = func(&args);
                    self.stack[dst] = result;
                }
                RETURN => {
                    let first = chunk.read_u8() as usize;
                    let count = chunk.read_u8() as usize;
                    if count == 0 {
                        return Ok(Value::Nil);
                    }
                    return Ok(self.stack[first].clone());
                }
                JMP => {
                    let raw = chunk.read_u16();
                    let offset = i16::from_le_bytes(raw.to_le_bytes());
                    let new_ip = chunk.ip as isize + offset as isize;
                    if new_ip < 0 {
                        return Err(RuntimeError::IpOutOfBounds(chunk.ip).into());
                    }
                    chunk.ip = new_ip as usize;
                }
                TEST => {
                    let reg = chunk.read_u8() as usize;
                    let raw = chunk.read_u16();
                    let offset = i16::from_le_bytes(raw.to_le_bytes());
                    if !self.stack[reg].is_truthy() {
                        let new_ip = chunk.ip as isize + offset as isize;
                        if new_ip < 0 {
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
