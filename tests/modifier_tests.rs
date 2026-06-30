mod common;

use common::{compile_err, run_program};
use sigil_interpreter::{
    compiler::{CompileError, compile_program_with},
    functions::{FnEntry, FnLookupKey, FnTypeSig, FunctionRegistry, IntrinsicContext, IntrinsicFn},
    types::TypeId,
    value::Value,
    vm::VM,
};

/// Build a custom `FunctionRegistry` with the given intrinsics.
fn custom_registry(entries: &[(&str, IntrinsicFn, Vec<TypeId>)]) -> FunctionRegistry {
    let mut reg = FunctionRegistry::default();
    for &(name, func, ref types) in entries {
        reg.register(
            FnLookupKey::External(name.into()),
            FnEntry::Intrinsic(func),
            FnTypeSig { param_types: types.clone() },
        );
    }
    reg
}

fn run_with(source: &str, reg: FunctionRegistry) -> Value {
    let (chunks, funcs) = compile_program_with(source, reg).unwrap();
    let mut vm = VM::default();
    vm.run(&chunks, &funcs).unwrap()
}

// ── @intrinsic tests (powered by built-in std intrinsics) ──

#[test]
fn test_intrinsic_call() {
    assert_eq!(
        run_program("@intrinsic fn add(a, b); return add(1, 2);"),
        Value::Number(3.0)
    );
}

#[test]
fn test_intrinsic_unary() {
    assert_eq!(
        run_program("@intrinsic fn neg(x); return neg(42);"),
        Value::Number(-42.0)
    );
}

#[test]
fn test_intrinsic_expression() {
    assert_eq!(
        run_program(
            "@intrinsic fn add(a, b); @intrinsic fn mul(a, b); return mul(add(2, 3), 4);"
        ),
        Value::Number(20.0)
    );
}

#[test]
fn test_intrinsic_chained() {
    assert_eq!(
        run_program(
            "@intrinsic fn sub(a, b); @intrinsic fn neg(x); return sub(neg(5), 3);"
        ),
        Value::Number(-8.0)
    );
}

#[test]
fn test_intrinsic_undefined() {
    let err = compile_err("@intrinsic fn nonexistent(x);");
    assert!(matches!(err, CompileError::UndefinedFunction { .. }));
}

#[test]
fn test_intrinsic_missing_semicolon() {
    let err = compile_err("@intrinsic fn add(a, b) { }");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── @lang_item tests ──

#[test]
fn test_lang_item_call_by_name() {
    assert_eq!(
        run_program(r"@lang_item(neg) fn negate(x) { return 0 - x; } return negate(5);"),
        Value::Number(-5.0)
    );
}

#[test]
fn test_lang_item_overrides_operator() {
    // -10 calls LangItem::Neg, which now calls negate (returns 0 - x)
    assert_eq!(
        run_program(r"@lang_item(neg) fn negate(x) { return 0 - x; } return -10;"),
        Value::Number(-10.0)
    );
}

#[test]
fn test_lang_item_add_override() {
    // + now calls plus (same Number signature overwrites built-in)
    assert_eq!(
        run_program(r"@lang_item(add) fn plus(a: Number, b: Number) { return a - b; } return 1 + 2;"),
        Value::Number(-1.0)
    );
}

#[test]
fn test_lang_item_and_call_by_name() {
    assert_eq!(
        run_program(
            r"@lang_item(add) fn plus(a, b) { return a - b; } return plus(10, 3);"
        ),
        Value::Number(7.0)
    );
}

// ── combined modifiers ──

#[test]
fn test_lang_item_with_intrinsic() {
    // @intrinsic means lookup by name, @lang_item registers alias
    assert_eq!(
        run_program("@lang_item(neg) @intrinsic fn neg(x); return -42;"),
        Value::Number(-42.0)
    );
}

#[test]
fn test_intrinsic_only_does_not_register_lang_item() {
    // @intrinsic without @lang_item should NOT affect operators
    fn my_add(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
        Value::Number(args[0].as_num() + args[1].as_num())
    }
    let reg = custom_registry(&[(
        "my_add", my_add as IntrinsicFn,
        vec![TypeId::Any, TypeId::Any],
    )]);
    assert_eq!(
        run_with("@intrinsic fn my_add(a, b); return my_add(5, 7);", reg),
        Value::Number(12.0)
    );
}

// ── error cases ──

#[test]
fn test_lang_item_unknown_name() {
    let err = compile_err("@lang_item(blah) fn foo() { }");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_unknown_modifier() {
    let err = compile_err("@nonsense fn foo() { }");
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_intrinsic_custom_name() {
    fn make_big(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
        Value::Number(args[0].as_num() * 100.0)
    }
    let reg = custom_registry(&[(
        "make_big", make_big as IntrinsicFn,
        vec![TypeId::Any],
    )]);
    assert_eq!(
        run_with("@intrinsic fn make_big(x); return make_big(5);", reg),
        Value::Number(500.0)
    );
}
