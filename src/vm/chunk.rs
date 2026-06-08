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

    pub fn read_u8(&mut self) -> u8 {
        let byte = self.code[self.ip];
        self.ip += 1;
        byte
    }

    pub fn read_u16(&mut self) -> u16 {
        let bytes = [self.code[self.ip], self.code[self.ip + 1]];
        self.ip += 2;
        u16::from_le_bytes(bytes)
    }

    pub fn emit_u8(&mut self, byte: u8) {
        self.code.push(byte);
    }

    pub fn emit_opcode(&mut self, op: OpCode) {
        self.emit_u8(op as u8);
    }

    pub fn emit_u16(&mut self, val: u16) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }

    pub fn add_constant(&mut self, value: Value) -> u16 {
        self.constants.intern(value)
    }

    pub fn reset_ip(&mut self) {
        self.ip = 0;
    }
}
