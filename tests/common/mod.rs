#![allow(dead_code)]

use sigil_interpreter::{
    compiler::compile::{CompileError, compile, compile_program},
    registry::FunctionRegistry,
    value::Value,
    vm::VM,
};

pub fn math_registry() -> FunctionRegistry {
    let mut reg = FunctionRegistry::new();

    fn add(args: &[Value]) -> Value {
        let a = match &args[0] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        let b = match &args[1] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        Value::Number(a + b)
    }
    fn sub(args: &[Value]) -> Value {
        let a = match &args[0] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        let b = match &args[1] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        Value::Number(a - b)
    }
    fn mul(args: &[Value]) -> Value {
        let a = match &args[0] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        let b = match &args[1] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        Value::Number(a * b)
    }
    fn div(args: &[Value]) -> Value {
        let a = match &args[0] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        let b = match &args[1] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        Value::Number(a / b)
    }
    fn neg(args: &[Value]) -> Value {
        match &args[0] {
            Value::Number(n) => Value::Number(-n),
            _ => Value::Nil,
        }
    }
    fn lt(args: &[Value]) -> Value {
        let a = match &args[0] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        let b = match &args[1] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        Value::Bool(a < b)
    }
    fn ge(args: &[Value]) -> Value {
        let a = match &args[0] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        let b = match &args[1] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        Value::Bool(a >= b)
    }
    fn eq(args: &[Value]) -> Value {
        let a = match &args[0] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        let b = match &args[1] {
            Value::Number(n) => *n,
            _ => 0.0,
        };
        Value::Bool((a - b).abs() < f64::EPSILON)
    }

    reg.register("add", add);
    reg.register("sub", sub);
    reg.register("mul", mul);
    reg.register("div", div);
    reg.register("neg", neg);
    reg.register("lt", lt);
    reg.register("ge", ge);
    reg.register("eq", eq);
    reg
}

pub fn run_program(source: &str) -> Value {
    let mut chunk = compile_program(source).unwrap();
    println!("{chunk}");
    let registry = math_registry();
    let mut vm = VM::new();
    vm.run(&mut chunk, &registry).unwrap()
}

pub fn run_expr(source: &str) -> Value {
    let mut chunk = compile(source).unwrap();
    println!("{chunk}");
    let registry = math_registry();
    let mut vm = VM::new();
    vm.run(&mut chunk, &registry).unwrap()
}

pub fn compile_err(source: &str) -> CompileError {
    compile_program(source).unwrap_err()
}

pub fn compile_expr_err(source: &str) -> CompileError {
    compile(source).unwrap_err()
}
