use strum_macros::FromRepr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u8)]
pub enum OpCode {
    MOVE,
    LOADK,
    LOADBOOL,
    LOADNIL,
    GETGLB,
    SETGLB,
    CALL,
    RETURN,
    JMP,
    TEST,
    CLOSURE,
    NEWSTRUCT,
}
