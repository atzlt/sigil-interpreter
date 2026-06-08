#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    MOVE = 0x00,
    LOADK = 0x01,
    LOADBOOL = 0x02,
    LOADNIL = 0x03,
    CALL = 0x04,
    CALLC = 0x05,
    RETURN = 0x06,
    JMP = 0x07,
    TEST = 0x08,
    CLOSURE = 0x09,
    NEWSTRUCT = 0x0A,
}

impl From<u8> for OpCode {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => OpCode::MOVE,
            0x01 => OpCode::LOADK,
            0x02 => OpCode::LOADBOOL,
            0x03 => OpCode::LOADNIL,
            0x04 => OpCode::CALL,
            0x05 => OpCode::CALLC,
            0x06 => OpCode::RETURN,
            0x07 => OpCode::JMP,
            0x08 => OpCode::TEST,
            0x09 => OpCode::CLOSURE,
            0x0A => OpCode::NEWSTRUCT,
            _ => panic!("invalid opcode: 0x{byte:02X}"),
        }
    }
}
