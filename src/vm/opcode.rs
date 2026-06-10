use num_enum::TryFromPrimitive;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum OpCode {
    MOVE,
    LOADK,
    LOADBOOL,
    LOADNIL,
    GETGLB,
    SETGLB,
    CALL,
    CALLC,
    RETURN,
    JMP,
    TEST,
    CLOSURE,
    NEWSTRUCT,
}
