use std::{fmt, ops::Range};

use num_enum::TryFromPrimitive;

use crate::{constant::ConstantPool, value::Value, vm::OpCode};

#[derive(Debug, Default)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ConstantPool,
    pub ip: usize,
    pub locus: Vec<(usize, Range<usize>)>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: ConstantPool::new(),
            ip: 0,
            locus: Vec::new(),
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

    pub fn record_locus(&mut self, span: Range<usize>) {
        self.locus.push((self.code.len(), span));
    }

    pub fn locus_at(&self, ip: usize) -> Option<&Range<usize>> {
        let idx = self.locus.partition_point(|(pos, _)| *pos <= ip);
        idx.checked_sub(1).map(|i| &self.locus[i].1)
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use OpCode::*;
        let mut pos = 0;
        let code = &self.code;
        while pos < code.len() {
            let op_byte = code[pos];
            pos += 1;
            let op = OpCode::try_from_primitive(op_byte).expect("Unrecognized opcode");
            write!(f, "{:04}  ", pos - 1)?;
            match op {
                MOVE => {
                    let dst = code[pos];
                    pos += 1;
                    let src = code[pos];
                    pos += 1;
                    writeln!(f, "MOVE    R{dst} R{src}")?;
                }
                LOADK => {
                    let dst = code[pos];
                    pos += 1;
                    let k = u16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    let val = self.constants.get(k);
                    writeln!(f, "LOADK   R{dst} K{k} ; {val}")?;
                }
                LOADBOOL => {
                    let dst = code[pos];
                    pos += 1;
                    let val = code[pos];
                    pos += 1;
                    let b = val != 0;
                    writeln!(f, "LOADBOOL R{dst} {b}")?;
                }
                LOADNIL => {
                    let dst = code[pos];
                    pos += 1;
                    writeln!(f, "LOADNIL R{dst}")?;
                }
                GETGLB => {
                    let dst = code[pos];
                    pos += 1;
                    let slot = u16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    writeln!(f, "GETGLB  R{dst} G{slot}")?;
                }
                SETGLB => {
                    let src = code[pos];
                    pos += 1;
                    let slot = u16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    writeln!(f, "SETGLB  G{slot} R{src}")?;
                }
                CALL => {
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
                CALLC => {
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
                RETURN => {
                    let reg = code[pos];
                    pos += 1;
                    writeln!(f, "RETURN  R{reg}")?;
                }
                JMP => {
                    let offset = i16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    let target = pos as isize - 3 + offset as isize;
                    writeln!(f, "JMP     {offset:+} -> {target}")?;
                }
                TEST => {
                    let reg = code[pos];
                    pos += 1;
                    let offset = i16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    let target = pos as isize - 4 + offset as isize;
                    writeln!(f, "TEST    R{reg} {offset:+} -> {target}")?;
                }
                CLOSURE => {
                    let dst = code[pos];
                    pos += 1;
                    let proto = u16::from_le_bytes([code[pos], code[pos + 1]]);
                    pos += 2;
                    writeln!(f, "CLOSURE R{dst} K{proto}")?;
                }
                NEWSTRUCT => {
                    let dst = code[pos];
                    pos += 1;
                    writeln!(f, "NEWSTRUCT R{dst}")?;
                }
            }
        }
        Ok(())
    }
}
