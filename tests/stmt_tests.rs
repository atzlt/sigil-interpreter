use sigil_interpreter::{
    compiler::compile::{CompileError, compile_program},
    registry::FunctionRegistry,
    value::Value,
    vm::VM,
};

fn math_registry() -> FunctionRegistry {
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

    reg.register("add", add);
    reg.register("sub", sub);
    reg.register("mul", mul);
    reg
}

fn run_program(source: &str) -> Value {
    let mut chunk = compile_program(source).unwrap();
    println!("{chunk}");
    let registry = math_registry();
    let mut vm = VM::new();
    vm.run(&mut chunk, &registry).unwrap()
}

// ── full programs: let + expr stmt + return ──

#[test]
fn test_program_let_return() {
    assert_eq!(run_program("let x = 42; return x;"), Value::Number(42.0));
}

#[test]
fn test_program_let_expr_chain() {
    assert_eq!(
        run_program("let a = 1; let b = 2; let c = a + b; c;"),
        Value::Nil
    );
}

#[test]
fn test_program_let_expr_return() {
    assert_eq!(run_program("let x = 1 + 2 * 3; x + 4; return;"), Value::Nil);
}

#[test]
fn test_program_return_nil_after_lets() {
    assert_eq!(run_program("let a = 1; let b = 2; return;"), Value::Nil);
}

#[test]
fn test_program_return_expr_after_lets() {
    assert_eq!(
        run_program("let a = 10; let b = 20; return a + b; let b = 30; return a;"),
        Value::Number(30.0)
    );
}

// ── error kind matching ──

#[test]
fn test_let_undefined_var() {
    let err = compile_program("let y = x;").unwrap_err();
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_let_missing_identifier() {
    let err = compile_program("let = 42;").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_let_missing_equals() {
    let err = compile_program("let x 42;").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_let_missing_semicolon() {
    let err = compile_program("let x = 42").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_let_malformed_expr() {
    let err = compile_program("let x = 42; let y = x = 1;").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_expr_stmt_missing_semicolon() {
    let err = compile_program("42").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_expr_stmt_undefined_var() {
    let err = compile_program("x;").unwrap_err();
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_return_missing_semicolon() {
    let err = compile_program("return 42").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── blocks ──

#[test]
fn test_block_basic() {
    assert_eq!(
        run_program(r"{ let x = 42; return x; }"),
        Value::Number(42.0)
    );
}

#[test]
fn test_block_var_shadowing() {
    assert_eq!(
        run_program(r"let x = 1; { let x = 2; } return x;"),
        Value::Number(1.0)
    );
}

#[test]
fn test_block_nested() {
    assert_eq!(
        run_program(r"{ let a = 1; { let b = 2; return a + b; } }"),
        Value::Number(3.0)
    );
}

#[test]
fn test_block_register_reuse() {
    assert_eq!(
        run_program(r"let x = 1; { let y = x + 3; } let z = 3; return x + z;"),
        Value::Number(4.0)
    );
}

#[test]
fn test_block_out_of_scope() {
    let err =
        compile_program(r"let x = 1; { let y = 2; } let z = 3; return x + y + z;").unwrap_err();
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_block_unclosed() {
    let err = compile_program(r"{ let x = 42;").unwrap_err();
    assert!(matches!(err, CompileError::Unclosed { .. }));
}
