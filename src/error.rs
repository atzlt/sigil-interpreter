use thiserror::Error;

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
