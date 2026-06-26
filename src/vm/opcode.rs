use strum_macros::FromRepr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromRepr)]
#[repr(u8)]
pub enum OpCode {
    MOVE,
    LOADK,
    LOADBOOL,
    LOADNIL,
    LOADFUN,
    GETGLB,
    SETGLB,
    CALL,
    CALLK,
    RETURN,
    JMP,
    TEST,
    CLOSURE,
    NEWSTRUCT,
    GETUPVAL,
    SETUPVAL,
}
