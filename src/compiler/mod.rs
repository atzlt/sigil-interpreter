pub mod compile;
mod emit;
mod expr;
mod label;
mod lexer;
mod loop_tracker;
mod register;
mod stmt;
mod type_registry;
mod variables;

pub use compile::CompileError;
pub use type_registry::{StructDef, TypeId, TypeRegistry};

use crate::{functions::FunctionRegistry, vm::Chunk};

pub type Result<T> = std::result::Result<T, CompileError>;

pub fn compile_expr(source: &str) -> Result<(Vec<Chunk>, FunctionRegistry)> {
    compile::compile(source, FunctionRegistry::with_std(), true)
}

pub fn compile_expr_with(
    source: &str,
    funcs: FunctionRegistry,
) -> Result<(Vec<Chunk>, FunctionRegistry)> {
    compile::compile(source, funcs, true)
}

pub fn compile_program(source: &str) -> Result<(Vec<Chunk>, FunctionRegistry)> {
    compile::compile(source, FunctionRegistry::with_std(), false)
}

pub fn compile_program_with(
    source: &str,
    funcs: FunctionRegistry,
) -> Result<(Vec<Chunk>, FunctionRegistry)> {
    compile::compile(source, funcs, false)
}
