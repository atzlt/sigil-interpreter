pub mod chunk;
pub mod exec;
mod frame;
pub mod heap;
pub mod opcode;
pub mod upvalue;

pub use chunk::Chunk;
pub use exec::VM;
pub use heap::{Heap, StructObject};
pub use opcode::OpCode;
