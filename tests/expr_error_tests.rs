mod common;

use common::compile_expr_err;
use sigil_interpreter::compiler::{CompileError, compile_expr};

#[test]
fn test_leading_operator() {
    let err = compile_expr_err("+");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_leading_star() {
    let err = compile_expr_err("* 5");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_trailing_operator() {
    let err = compile_expr_err("1 +");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_trailing_operator_with_ws() {
    let err = compile_expr_err("1 + 2 *");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_double_operator() {
    let err = compile_expr_err("1 * * 2");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_unclosed_paren() {
    let err = compile_expr_err("(1 + 2");
    assert!(matches!(err, CompileError::Unclosed { .. }));
}

#[test]
fn test_nested_unclosed_paren() {
    let err = compile_expr_err("(1 + (2 + 3)");
    assert!(matches!(err, CompileError::Unclosed { .. }));
}

#[test]
fn test_stray_close_paren() {
    let err = compile_expr_err(")");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_empty_paren() {
    let err = compile_expr_err("()");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_empty_input() {
    let err = compile_expr_err("");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_leading_unary_then_eof() {
    let err = compile_expr_err("!");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_bang_on_number_is_ok() {
    assert!(compile_expr("!5").is_ok());
}

#[test]
fn test_negate_happy() {
    assert!(compile_expr("-5").is_ok());
    assert!(compile_expr("- 5").is_ok());
}

#[test]
fn test_binary_without_rhs_paren() {
    let err = compile_expr_err("(1 + )");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_unrecognized() {
    let err = compile_expr_err("1 ` 2");
    assert!(matches!(err, CompileError::Unrecognized { .. }));
}

#[test]
fn test_nested_ternary() {
    let mut source = (1..=400)
        .map(|_| "1 >= 2 ? 1 : ")
        .collect::<Vec<_>>()
        .join("");
    source.push_str("2");
    assert!(matches!(
        compile_expr_err(&source),
        CompileError::RegisterOverflow(_)
    ));
}
