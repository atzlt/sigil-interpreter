use std::fmt;

use crate::{constant::ConstantPool, value::Value, vm::OpCode};

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ConstantPool,
    pub ip: usize,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: ConstantPool::new(),
            ip: 0,
        }
    }

    pub fn read(&mut self) -> u8 {
        let byte = self.code[self.ip];
        self.ip += 1;
        byte
    }

    pub fn read_wide(&mut self) -> u16 {
        let bytes = [self.code[self.ip], self.code[self.ip + 1]];
        self.ip += 2;
        u16::from_le_bytes(bytes)
    }

    pub fn read_i16(&mut self) -> i16 {
        let bytes = [self.code[self.ip], self.code[self.ip + 1]];
        self.ip += 2;
        i16::from_le_bytes(bytes)
    }

    pub fn emit(&mut self, byte: u8) {
        self.code.push(byte);
    }

    pub fn emit_opcode(&mut self, op: OpCode) {
        self.emit(op as u8);
    }

    pub fn emit_wide(&mut self, val: u16) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }

    pub fn patch_wide(&mut self, ip: usize, val: u16) {
        let val = val.to_le_bytes();
        self.code[ip] = val[0];
        self.code[ip + 1] = val[1];
    }

    pub fn add_constant(&mut self, value: Value) -> u16 {
        self.constants.intern(value)
    }

    pub fn reset_ip(&mut self) {
        self.ip = 0;
    }

    pub fn last(&self) -> usize {
        self.code.len() - 1
    }

    pub fn last_wide(&self) -> usize {
        self.code.len() - 2
    }

    pub fn end(&self) -> usize {
        self.code.len()
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pos = 0;
        let code = &self.code;
        while pos < code.len() {
            let op_byte = code[pos];
            pos += 1;
            let op = OpCode::from(op_byte);
            write!(f, "{:04}  ", pos - 1)?;
            match op {
                OpCode::MOVE => {
                    let dst = code[pos];
                    pos += 1;
                    let src = code[pos];
                    pos += 1;
                    writeln!(f, "MOVE    R{dst} R{src}")?;
                }
                OpCode::LOADK => {
                    let dst = code[pos];
                    pos += 1;
                    let k = u16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    let val = self.constants.get(k);
                    writeln!(f, "LOADK   R{dst} K{k} ; {val}")?;
                }
                OpCode::LOADBOOL => {
                    let dst = code[pos];
                    pos += 1;
                    let val = code[pos];
                    pos += 1;
                    let b = val != 0;
                    writeln!(f, "LOADBOOL R{dst} {b}")?;
                }
                OpCode::LOADNIL => {
                    let dst = code[pos];
                    pos += 1;
                    writeln!(f, "LOADNIL R{dst}")?;
                }
                OpCode::CALL => {
                    let dst = code[pos];
                    pos += 1;
                    let name_idx = u16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    let argc = code[pos] as usize;
                    pos += 1;
                    let name = self.constants.get(name_idx);
                    let args: Vec<String> = (0..argc)
                        .map(|_| {
                            let r = code[pos];
                            pos += 1;
                            format!("R{r}")
                        })
                        .collect();
                    writeln!(f, "CALL    R{dst} {name} [{}]", args.join(", "))?;
                }
                OpCode::CALLC => {
                    let dst = code[pos];
                    pos += 1;
                    let func = code[pos];
                    pos += 1;
                    let argc = code[pos] as usize;
                    pos += 1;
                    let args: Vec<String> = (0..argc)
                        .map(|_| {
                            let r = code[pos];
                            pos += 1;
                            format!("R{r}")
                        })
                        .collect();
                    writeln!(f, "CALLC   R{dst} R{func} [{}]", args.join(", "))?;
                }
                OpCode::RETURN => {
                    let first = code[pos];
                    pos += 1;
                    let count = code[pos];
                    pos += 1;
                    writeln!(f, "RETURN  R{first} {count}")?;
                }
                OpCode::JMP => {
                    let offset = i16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    let target = pos as isize - 3 + offset as isize;
                    writeln!(f, "JMP     {offset:+} -> {target}")?;
                }
                OpCode::TEST => {
                    let reg = code[pos];
                    pos += 1;
                    let offset = i16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    let target = pos as isize - 4 + offset as isize;
                    writeln!(f, "TEST    R{reg} {offset:+} -> {target}")?;
                }
                OpCode::CLOSURE => {
                    let dst = code[pos];
                    pos += 1;
                    let proto = u16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    writeln!(f, "CLOSURE R{dst} K{proto}")?;
                }
                OpCode::NEWSTRUCT => {
                    let dst = code[pos];
                    pos += 1;
                    writeln!(f, "NEWSTRUCT R{dst}")?;
                }
            }
        }
        Ok(())
    }
}
