mod common;

use common::{compile_err, run_program};
use sigil_interpreter::{compiler::CompileError, value::Value};

// ── struct construction (named) ──

#[test]
fn test_struct_named_construction() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2 { x: 1, y: 2 }; return v.x;"
        ),
        Value::Number(1.0)
    );
}

#[test]
fn test_struct_named_all_fields() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2 { x: 1, y: 2 }; return v.y;"
        ),
        Value::Number(2.0)
    );
}

#[test]
fn test_struct_named_fields_any_order() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2 { y: 3, x: 1 }; return v.x;"
        ),
        Value::Number(1.0)
    );
}

// ── struct construction (positional) ──

#[test]
fn test_struct_positional_construction() {
    assert_eq!(
        run_program("struct Vec2 { x: Number, y: Number } let v = Vec2(1, 2); return v.x;"),
        Value::Number(1.0)
    );
}

#[test]
fn test_struct_positional_all_fields() {
    assert_eq!(
        run_program("struct Vec2 { x: Number, y: Number } let v = Vec2(3, 4); return v.x + v.y;"),
        Value::Number(7.0)
    );
}

// ── field mutation ──

#[test]
fn test_struct_field_mutation() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2(1, 2); v.x = 5; return v.x;"
        ),
        Value::Number(5.0)
    );
}

#[test]
fn test_struct_field_mutation_other_field_unchanged() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2(1, 2); v.x = 5; return v.y;"
        ),
        Value::Number(2.0)
    );
}

#[test]
fn test_struct_field_mutation_both_fields() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2(1, 2); v.x = 5; v.y = 6; return v.x + v.y;"
        ),
        Value::Number(11.0)
    );
}

// ── nested struct read ──

#[test]
fn test_nested_struct_read_one_level() {
    assert_eq!(
        run_program(
            "struct Inner { a: Number } struct Outer { inner: Inner } \
             let v = Outer { inner: Inner(10) }; return v.inner.a;"
        ),
        Value::Number(10.0)
    );
}

#[test]
fn test_nested_struct_read_two_levels() {
    assert_eq!(
        run_program(
            "struct A { val: Number } struct B { a: A } struct C { b: B } \
             let v = C { b: B(A { val: 42 }) }; return v.b.a.val;"
        ),
        Value::Number(42.0)
    );
}

// ── nested struct mutation ──

#[test]
fn test_nested_struct_field_mutation() {
    assert_eq!(
        run_program(
            "struct Inner { a: Number } struct Outer { inner: Inner } \
             let v = Outer { inner: Inner(10) }; v.inner.a = 99; return v.inner.a;"
        ),
        Value::Number(99.0)
    );
}

#[test]
fn test_nested_struct_field_mutation_deep() {
    assert_eq!(
        run_program(
            "struct A { val: Number } struct B { a: A } struct C { b: B } \
             let v = C { b: B { a: A(1) } }; v.b.a.val = 100; return v.b.a.val;"
        ),
        Value::Number(100.0)
    );
}

// ── value equality ──

#[test]
fn test_struct_eq_same_values() {
    assert_eq!(
        run_program("struct Vec2 { x: Number, y: Number } return Vec2(1, 2) == Vec2(1, 2);"),
        Value::Bool(true)
    );
}

#[test]
fn test_struct_eq_different_values() {
    assert_eq!(
        run_program("struct Vec2 { x: Number, y: Number } return Vec2(1, 2) == Vec2(3, 4);"),
        Value::Bool(false)
    );
}

#[test]
fn test_struct_neq() {
    assert_eq!(
        run_program("struct Vec2 { x: Number, y: Number } return Vec2(1, 2) != Vec2(3, 4);"),
        Value::Bool(true)
    );
}

#[test]
fn test_struct_eq_nested() {
    assert_eq!(
        run_program(
            "struct Inner { a: Number } struct Outer { inner: Inner } \
             let x = Outer { inner: Inner(1) }; \
             let y = Outer { inner: Inner(1) }; \
             return x == y;"
        ),
        Value::Bool(true)
    );
}

#[test]
fn test_struct_eq_nested_different() {
    assert_eq!(
        run_program(
            "struct Inner { a: Number } struct Outer { inner: Inner } \
             let x = Outer { inner: Inner(1) }; \
             let y = Outer { inner: Inner(2) }; \
             return x == y;"
        ),
        Value::Bool(false)
    );
}

// ── structs in expressions ──

#[test]
fn test_struct_in_addition() {
    assert_eq!(
        run_program("struct Vec2 { x: Number, y: Number } let v = Vec2(3, 4); return v.x + v.y;"),
        Value::Number(7.0)
    );
}

#[test]
fn test_struct_in_comparison() {
    assert_eq!(
        run_program("struct Vec2 { x: Number, y: Number } let v = Vec2(3, 4); return v.x < v.y;"),
        Value::Bool(true)
    );
}

#[test]
fn test_struct_field_expression() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2(1, 2); return v.x * 10 + v.y;"
        ),
        Value::Number(12.0)
    );
}

// ── struct field as assignment RHS ──

#[test]
fn test_assign_struct_field_to_var() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } let v = Vec2(7, 8); let a = v.x; return a;"
        ),
        Value::Number(7.0)
    );
}

// ── multiple structs ──

#[test]
fn test_multiple_struct_instances() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } \
             let a = Vec2(1, 2); let b = Vec2(3, 4); \
             return a.x + b.y;"
        ),
        Value::Number(5.0)
    );
}

#[test]
fn test_multiple_struct_types() {
    assert_eq!(
        run_program(
            "struct Vec2 { x: Number, y: Number } struct Vec3 { x: Number, y: Number, z: Number } \
             let a = Vec2(1, 2); let b = Vec3(3, 4, 5); \
             return a.x + b.z;"
        ),
        Value::Number(6.0)
    );
}

// ── structs with different field types ──

#[test]
fn test_struct_with_bool_field() {
    assert_eq!(
        run_program("struct Flag { value: Bool } let f = Flag { value: true }; return f.value;"),
        Value::Bool(true)
    );
}

#[test]
fn test_struct_with_string_field() {
    assert_eq!(
        run_program(
            r#"struct Named { name: String } let n = Named { name: "hello" }; return n.name;"#
        ),
        Value::String("hello".into())
    );
}

// ── error: missing field ──

#[test]
fn test_struct_missing_field_error() {
    let err = compile_err("struct Vec2 { x: Number, y: Number } let v = Vec2 { x: 1 };");
    assert!(matches!(err, CompileError::MissingField { .. }));
}

// ── error: duplicate field ──

#[test]
fn test_struct_duplicate_field_error() {
    let err = compile_err("struct Vec2 { x: Number, y: Number } let v = Vec2 { x: 1, x: 2 };");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── error: unknown field ──

#[test]
fn test_struct_unknown_field_error() {
    let err = compile_err("struct Vec2 { x: Number, y: Number } let v = Vec2 { z: 1 };");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── error: positional count mismatch ──

#[test]
fn test_struct_positional_too_few_args() {
    let err = compile_err("struct Vec2 { x: Number, y: Number } let v = Vec2(1);");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_struct_positional_too_many_args() {
    let err = compile_err("struct Vec2 { x: Number, y: Number } let v = Vec2(1, 2, 3);");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── error: undefined struct type ──

#[test]
fn test_undefined_struct_type() {
    let err = compile_err("let v = Foo { x: 1 };");
    assert!(matches!(err, CompileError::UndefinedVariable { .. }));
}

// ── struct with nil field ──

#[test]
fn test_struct_with_nil_field() {
    assert_eq!(
        run_program("struct Opt { value: Nil } let o = Opt { value: nil }; return o.value;"),
        Value::Nil
    );
}
