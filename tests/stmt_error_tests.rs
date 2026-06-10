mod common;

use common::compile_err;
use sigil_interpreter::compiler::compile::CompileError;

// ── let errors ──

#[test]
fn test_let_undefined_var() {
    let err = compile_err("let y = x;");
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_let_missing_identifier() {
    let err = compile_err("let = 42;");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_let_missing_equals() {
    let err = compile_err("let x 42;");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_let_missing_semicolon() {
    let err = compile_err("let x = 42");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── expression statement errors ──

#[test]
fn test_expr_stmt_missing_semicolon() {
    let err = compile_err("42");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_expr_stmt_undefined_var() {
    let err = compile_err("x;");
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

// ── return errors ──

#[test]
fn test_return_missing_semicolon() {
    let err = compile_err("return 42");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── block errors ──

#[test]
fn test_block_out_of_scope() {
    let err = compile_err(r"let x = 1; { let y = 2; } let z = 3; return x + y + z;");
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_block_unclosed() {
    let err = compile_err(r"{ let x = 42;");
    assert!(matches!(err, CompileError::Unclosed { .. }));
}

// ── if errors ──

#[test]
fn test_if_body_scope_does_not_leak() {
    let err = compile_err(r"if 1 { let x = 42; } return x;");
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_if_missing_condition() {
    let err = compile_err(r"if { return 1; }");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_if_missing_block() {
    let err = compile_err(r"if 1 return 1;");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── while errors ──

#[test]
fn test_while_body_scope_does_not_leak() {
    let err = compile_err(r"let i = 0; while i < 10 { let x = 42; } return x;");
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_while_missing_condition() {
    let err = compile_err(r"while { return 1; }");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_while_missing_block() {
    let err = compile_err(r"while 1 return 1;");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── assignment errors ──

#[test]
fn test_assignment_to_undefined_var() {
    let err = compile_err(r"x = 1;");
    dbg!(&err);
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

#[test]
fn test_assignment_to_undefined_var_in_block() {
    let err = compile_err(r"{ x = 1; }");
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

// ── break errors ──

#[test]
fn test_break_outside_loop() {
    let err = compile_err(r"break;");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_break_outside_loop_in_block() {
    let err = compile_err(r"{ break; }");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── continue errors ──

#[test]
fn test_continue_outside_loop() {
    let err = compile_err(r"continue;");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_continue_outside_loop_in_block() {
    let err = compile_err(r"{ continue; }");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}
