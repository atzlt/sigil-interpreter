#![allow(dead_code)]

use sigil_interpreter::{
    compiler::compile::{CompileError, compile_expr, compile_program},
    value::Value,
    vm::{Chunk, VM},
};

fn print_chunks(chunks: &[Chunk]) {
    chunks.iter().enumerate().for_each(|(i, c)| {
        println!("Chunk #{i} =====================================================");
        println!("{c}");
    });
    println!("Program end ==================================================");
}

pub fn run_program(source: &str) -> Value {
    let compiled = compile_program(source).unwrap();
    print_chunks(&compiled.0);
    let mut vm = VM::default();
    vm.run(&compiled.0, &compiled.1).unwrap()
}

pub fn run_expr(source: &str) -> Value {
    let compiled = compile_expr(source).unwrap();
    print_chunks(&compiled.0);
    let mut vm = VM::default();
    vm.run(&compiled.0, &compiled.1).unwrap()
}

pub fn compile_err(source: &str) -> CompileError {
    compile_program(source).unwrap_err()
}

pub fn compile_expr_err(source: &str) -> CompileError {
    compile_expr(source).unwrap_err()
}
