#![allow(dead_code)]

use sigil_interpreter::{
    compiler::compile::{CompileError, compile_expr, compile_program},
    functions::FunctionRegistry,
    value::Value,
    vm::VM,
};

pub fn run_program(source: &str) -> Value {
    let mut chunk = compile_program(source).unwrap();
    println!("{chunk}");
    let registry = FunctionRegistry::with_std();
    let mut vm = VM::new();
    vm.run(&mut chunk, &registry).unwrap()
}

pub fn run_expr(source: &str) -> Value {
    let mut chunk = compile_expr(source).unwrap();
    println!("{chunk}");
    let registry = FunctionRegistry::with_std();
    let mut vm = VM::new();
    vm.run(&mut chunk, &registry).unwrap()
}

pub fn compile_err(source: &str) -> CompileError {
    compile_program(source).unwrap_err()
}

pub fn compile_expr_err(source: &str) -> CompileError {
    compile_expr(source).unwrap_err()
}
