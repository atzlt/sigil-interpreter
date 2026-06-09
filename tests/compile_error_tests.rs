use sigil_interpreter::compiler::compile::{CompileError, compile};

#[test]
fn test_leading_operator() {
    let err = compile("+").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_leading_star() {
    let err = compile("* 5").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_trailing_operator() {
    let err = compile("1 +").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_trailing_operator_with_ws() {
    let err = compile("1 + 2 *").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_double_operator() {
    let err = compile("1 * * 2").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_unclosed_paren() {
    let err = compile("(1 + 2").unwrap_err();
    assert!(matches!(err, CompileError::Unclosed { .. }));
}

#[test]
fn test_nested_unclosed_paren() {
    let err = compile("(1 + (2 + 3)").unwrap_err();
    assert!(matches!(err, CompileError::Unclosed { .. }));
}

#[test]
fn test_stray_close_paren() {
    let err = compile(")").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_empty_paren() {
    let err = compile("()").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_empty_input() {
    // Bare EOF — advance() reads it, then expression() fails.
    let err = compile("").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_leading_unary_then_eof() {
    let err = compile("!").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_bang_on_number_is_ok() {
    // `!5` should work: negate 5 via `not` lang item
    assert!(compile("!5").is_ok());
}

#[test]
fn test_negate_happy() {
    assert!(compile("-5").is_ok());
    assert!(compile("- 5").is_ok());
}

#[test]
fn test_binary_without_rhs_paren() {
    // `(1 + )` — missing rhs inside paren
    let err = compile("(1 + )").unwrap_err();
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_unrecognized() {
    let err = compile("1 ` 2").unwrap_err();
    assert!(matches!(err, CompileError::Unrecognized { .. }));
}
