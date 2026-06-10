mod common;

use common::run_expr;
use sigil_interpreter::value::Value;

#[test]
fn test_simple_add() {
    assert_eq!(run_expr("1 + 2"), Value::Number(3.0));
}

#[test]
fn test_precedence() {
    assert_eq!(run_expr("2 * 3 + 1"), Value::Number(7.0));
    assert_eq!(run_expr("1 + 2 * 3"), Value::Number(7.0));
}

#[test]
fn test_grouping() {
    assert_eq!(run_expr("(1 + 2) * 3"), Value::Number(9.0));
}

#[test]
fn test_unary_neg() {
    assert_eq!(run_expr("-5"), Value::Number(-5.0));
    assert_eq!(run_expr("-(1 + 2)"), Value::Number(-3.0));
    assert_eq!(run_expr("-1 + 2"), Value::Number(1.0));
}

#[test]
fn test_nested_ops() {
    assert_eq!(run_expr("1 + 2 + 3"), Value::Number(6.0));
    assert_eq!(run_expr("1 + 2 * 3 - 4 / 2"), Value::Number(5.0));
}

#[test]
fn test_deep_nested_ops() {
    let mut source = (0..=50).map(|_| "0 + 0 * (").collect::<Vec<_>>().join("");
    let source2 = (0..=50).map(|_| ")").collect::<Vec<_>>().join("");
    source.push_str("0");
    source.push_str(&source2);
    assert_eq!(run_expr(&source), Value::Number(0.0));
}

#[test]
fn test_number_literal() {
    assert_eq!(run_expr("42"), Value::Number(42.0));
    assert_eq!(run_expr("3.14"), Value::Number(3.14));
}

#[test]
fn test_literals() {
    assert_eq!(run_expr("true"), Value::Bool(true));
    assert_eq!(run_expr("false"), Value::Bool(false));
    assert_eq!(run_expr("nil"), Value::Nil);
}

#[test]
fn test_string_literal() {
    assert_eq!(run_expr("\"hello\""), Value::String("hello".into()));
}

#[test]
fn test_register_reuse_long_chain() {
    let source = (1..=500)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    assert_eq!(run_expr(&source), Value::Number(125250.0));
}

#[test]
fn test_ternary_true() {
    assert_eq!(run_expr("4 >= 3 ? 1 : 2"), Value::Number(1.0));
}

#[test]
fn test_ternary_false() {
    assert_eq!(run_expr("3 >= 4 ? 1 : 2"), Value::Number(2.0));
}

#[test]
fn test_ternary_right_assoc() {
    assert_eq!(run_expr("4 >= 3 ? 2 >= 1 ? 5 : 6 : 7"), Value::Number(5.0));
    assert_eq!(run_expr("3 >= 4 ? 2 >= 1 ? 5 : 6 : 7"), Value::Number(7.0));
    assert_eq!(run_expr("4 >= 3 ? 5 : 2 >= 1 ? 6 : 7"), Value::Number(5.0));
    assert_eq!(run_expr("3 >= 4 ? 5 : 2 >= 1 ? 6 : 7"), Value::Number(6.0));
}

#[test]
fn test_nested_ternary() {
    let mut source = (1..=254)
        .map(|_| "1 >= 2 ? 1 : ")
        .collect::<Vec<_>>()
        .join("");
    source.push_str("2");
    assert_eq!(run_expr(&source), Value::Number(2.0));
}

// ── and (&) ──

#[test]
fn test_and_truthtable() {
    assert_eq!(run_expr("1 && 1"), Value::Number(1.0));
    assert_eq!(run_expr("1 && 0"), Value::Number(0.0));
    assert_eq!(run_expr("0 && 1"), Value::Number(0.0));
    assert_eq!(run_expr("0 && 0"), Value::Number(0.0));
}

#[test]
fn test_and_returns_rhs_when_lhs_truthy() {
    assert_eq!(run_expr("1 && 2"), Value::Number(2.0));
    assert_eq!(run_expr("2 && 1"), Value::Number(1.0));
}

#[test]
fn test_and_returns_lhs_when_lhs_falsey() {
    assert_eq!(run_expr("0 && 2"), Value::Number(0.0));
}

#[test]
fn test_and_left_assoc() {
    // (1 && 2) && 3 → 2 && 3 → 3
    assert_eq!(run_expr("1 && 2 && 3"), Value::Number(3.0));
    // (1 && 0) && 2 → 0 && 2 → 0
    assert_eq!(run_expr("1 && 0 && 2"), Value::Number(0.0));
}

#[test]
fn test_and_with_nil() {
    assert_eq!(run_expr("nil && 1"), Value::Nil);
    assert_eq!(run_expr("1 && nil"), Value::Nil);
}

#[test]
fn test_and_with_bools() {
    assert_eq!(run_expr("false && false"), Value::Bool(false));
    assert_eq!(run_expr("true && false"), Value::Bool(false));
    assert_eq!(run_expr("false && true"), Value::Bool(false));
    assert_eq!(run_expr("true && true"), Value::Bool(true));
}

// ── or (|) ──

#[test]
fn test_or_truthtable() {
    assert_eq!(run_expr("1 || 1"), Value::Number(1.0));
    assert_eq!(run_expr("1 || 0"), Value::Number(1.0));
    assert_eq!(run_expr("0 || 1"), Value::Number(1.0));
    assert_eq!(run_expr("0 || 0"), Value::Number(0.0));
}

#[test]
fn test_or_returns_lhs_when_lhs_truthy() {
    assert_eq!(run_expr("1 || 2"), Value::Number(1.0));
    assert_eq!(run_expr("2 || 1"), Value::Number(2.0));
}

#[test]
fn test_or_returns_rhs_when_lhs_falsey() {
    assert_eq!(run_expr("0 || 2"), Value::Number(2.0));
}

#[test]
fn test_or_left_assoc() {
    // (0 || 1) || 2 → 1 || 2 → 1
    assert_eq!(run_expr("0 || 1 || 2"), Value::Number(1.0));
    // (0 || 0) || 2 → 0 || 2 → 2
    assert_eq!(run_expr("0 || 0 || 2"), Value::Number(2.0));
}

#[test]
fn test_or_with_nil() {
    assert_eq!(run_expr("nil || 1"), Value::Number(1.0));
    assert_eq!(run_expr("1 || nil"), Value::Number(1.0));
    assert_eq!(run_expr("nil || nil"), Value::Nil);
}

#[test]
fn test_or_with_bools() {
    assert_eq!(run_expr("true || false"), Value::Bool(true));
    assert_eq!(run_expr("false || true"), Value::Bool(true));
    assert_eq!(run_expr("false || false"), Value::Bool(false));
}

// ── and / or precedence ──

#[test]
fn test_and_binds_tighter_than_or() {
    // 0 || 1 && 2  →  0 || (1 && 2)  →  0 || 2  →  2
    assert_eq!(run_expr("0 || 1 && 2"), Value::Number(2.0));
    // 1 || 0 && 2  →  1 || (0 && 2)  →  1 || 0  →  1
    assert_eq!(run_expr("1 || 0 && 2"), Value::Number(1.0));
}

#[test]
fn test_and_or_chain() {
    // (1 && 2) || (3 && 0) || (0 && 5) || 6  →  2 || 0 || 0 || 6  →  2
    assert_eq!(run_expr("1 && 2 || 3 && 0 || 0 && 5 || 6"), Value::Number(2.0));
}

#[test]
fn test_and_or_long_chain() {
    let source = (1..=100)
        .map(|i| format!("{} && {}", i, i))
        .collect::<Vec<_>>()
        .join(" || ");
    // all are non-zero && non-zero = last value of each pair
    // || of all last values = first non-zero (which is 1)
    assert_eq!(run_expr(&source), Value::Number(1.0));
}
