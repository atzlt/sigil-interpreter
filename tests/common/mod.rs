#![allow(dead_code)]

use sigil_interpreter::{
    compiler::compile::{CompileError, compile_expr, compile_program},
    value::Value,
    vm::VM,
};

pub fn run_program(source: &str) -> Value {
    let mut chunk = compile_program(source).unwrap();
    println!("{chunk}");
    let mut vm = VM::default();
    vm.run(&mut chunk).unwrap()
}

pub fn run_expr(source: &str) -> Value {
    let mut chunk = compile_expr(source).unwrap();
    println!("{chunk}");
    let mut vm = VM::default();
    vm.run(&mut chunk).unwrap()
}

pub fn compile_err(source: &str) -> CompileError {
    compile_program(source).unwrap_err()
}

pub fn compile_expr_err(source: &str) -> CompileError {
    compile_expr(source).unwrap_err()
}
