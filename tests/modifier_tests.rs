use sigil_interpreter::{
    compiler::{CompileError, compile_program_with},
    functions::{FnLookupKey, FunctionRegistry, LangItem},
    value::Value,
    vm::{Chunk, VM},
};

fn mk_intrinsic(name: &str, f: fn(&[&Value]) -> Value) -> (FnLookupKey, fn(&[&Value]) -> Value) {
    (FnLookupKey::External(name.into()), f)
}

fn mk_lang_item(
    item: LangItem,
    f: fn(&[&Value]) -> Value,
) -> (FnLookupKey, fn(&[&Value]) -> Value) {
    (FnLookupKey::LangItem(item), f)
}

fn registry(intrinsics: &[(FnLookupKey, fn(&[&Value]) -> Value)]) -> FunctionRegistry {
    let mut reg = FunctionRegistry::default();
    for (key, f) in intrinsics {
        reg.register_intrinsic(key.clone(), *f);
    }
    reg
}

fn run_with(source: &str, reg: FunctionRegistry) -> Value {
    let (chunks, funcs) = compile_program_with(source, reg).unwrap();
    print_chunks(&chunks);
    let mut vm = VM::default();
    vm.run(&chunks, &funcs).unwrap()
}

fn run_with_err(source: &str, reg: FunctionRegistry) -> CompileError {
    compile_program_with(source, reg).unwrap_err()
}

fn print_chunks(chunks: &[Chunk]) {
    chunks.iter().enumerate().for_each(|(i, c)| {
        println!("Chunk #{i} =====================================================");
        println!("{c}");
    });
    println!("Program end ==================================================");
}

// ── helpers: intrinsic functions ──

fn add(args: &[&Value]) -> Value {
    Value::Number(args[0].as_num() + args[1].as_num())
}

fn sub(args: &[&Value]) -> Value {
    Value::Number(args[0].as_num() - args[1].as_num())
}

fn mul(args: &[&Value]) -> Value {
    Value::Number(args[0].as_num() * args[1].as_num())
}

fn div(args: &[&Value]) -> Value {
    Value::Number(args[0].as_num() / args[1].as_num())
}

fn neg(args: &[&Value]) -> Value {
    Value::Number(-args[0].as_num())
}

fn not(args: &[&Value]) -> Value {
    Value::Bool(!args[0].is_truthy())
}

fn eq(args: &[&Value]) -> Value {
    Value::Bool(args[0] == args[1])
}

fn neq(args: &[&Value]) -> Value {
    Value::Bool(args[0] != args[1])
}

fn lt(args: &[&Value]) -> Value {
    Value::Bool(args[0].as_num() < args[1].as_num())
}

fn le(args: &[&Value]) -> Value {
    Value::Bool(args[0].as_num() <= args[1].as_num())
}

fn gt(args: &[&Value]) -> Value {
    Value::Bool(args[0].as_num() > args[1].as_num())
}

fn ge(args: &[&Value]) -> Value {
    Value::Bool(args[0].as_num() >= args[1].as_num())
}

fn rem(args: &[&Value]) -> Value {
    Value::Number(args[0].as_num() % args[1].as_num())
}

fn std_registry() -> FunctionRegistry {
    registry(&[
        mk_lang_item(LangItem::Add, add),
        mk_intrinsic("add", add),
        mk_lang_item(LangItem::Sub, sub),
        mk_intrinsic("sub", sub),
        mk_lang_item(LangItem::Mul, mul),
        mk_intrinsic("mul", mul),
        mk_lang_item(LangItem::Div, div),
        mk_intrinsic("div", div),
        mk_lang_item(LangItem::Rem, rem),
        mk_intrinsic("rem", rem),
        mk_lang_item(LangItem::Neg, neg),
        mk_intrinsic("neg", neg),
        mk_lang_item(LangItem::Not, not),
        mk_intrinsic("not", not),
        mk_lang_item(LangItem::Eq, eq),
        mk_intrinsic("eq", eq),
        mk_lang_item(LangItem::Neq, neq),
        mk_intrinsic("neq", neq),
        mk_lang_item(LangItem::Lt, lt),
        mk_intrinsic("lt", lt),
        mk_lang_item(LangItem::Le, le),
        mk_intrinsic("le", le),
        mk_lang_item(LangItem::Gt, gt),
        mk_intrinsic("gt", gt),
        mk_lang_item(LangItem::Ge, ge),
        mk_intrinsic("ge", ge),
    ])
}

// ── @intrinsic tests ──

#[test]
fn test_intrinsic_call() {
    assert_eq!(
        run_with("@intrinsic fn add(a, b); return add(1, 2);", std_registry()),
        Value::Number(3.0)
    );
}

#[test]
fn test_intrinsic_unary() {
    assert_eq!(
        run_with("@intrinsic fn neg(x); return neg(42);", std_registry()),
        Value::Number(-42.0)
    );
}

#[test]
fn test_intrinsic_expression() {
    assert_eq!(
        run_with(
            "@intrinsic fn add(a, b); @intrinsic fn mul(a, b); return mul(add(2, 3), 4);",
            std_registry()
        ),
        Value::Number(20.0)
    );
}

#[test]
fn test_intrinsic_chained() {
    assert_eq!(
        run_with(
            "@intrinsic fn sub(a, b); @intrinsic fn neg(x); return sub(neg(5), 3);",
            std_registry()
        ),
        Value::Number(-8.0)
    );
}

#[test]
fn test_intrinsic_undefined() {
    let reg = registry(&[mk_intrinsic("add", add)]);
    let err = run_with_err("@intrinsic fn nonexistent(x);", reg);
    assert!(matches!(err, CompileError::UndefinedFunction { .. }));
}

#[test]
fn test_intrinsic_missing_semicolon() {
    let reg = registry(&[mk_intrinsic("add", add)]);
    let err = run_with_err("@intrinsic fn add(a, b) { }", reg);
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

// ── @lang_item tests ──

#[test]
fn test_lang_item_call_by_name() {
    assert_eq!(
        run_with(
            r"@lang_item(neg) fn negate(x) { return 0 - x; } return negate(5);",
            std_registry()
        ),
        Value::Number(-5.0)
    );
}

#[test]
fn test_lang_item_overrides_operator() {
    // -10 calls LangItem::Neg, which now calls negate (returns 0 - x)
    assert_eq!(
        run_with(
            r"@lang_item(neg) fn negate(x) { return 0 - x; } return -10;",
            std_registry()
        ),
        Value::Number(-10.0)
    );
}

#[test]
fn test_lang_item_add_override() {
    // + now calls plus, which does a - b instead
    assert_eq!(
        run_with(
            r"@lang_item(add) fn plus(a, b) { return a - b; } return 1 + 2;",
            std_registry()
        ),
        Value::Number(-1.0)
    );
}

#[test]
fn test_lang_item_and_call_by_name() {
    assert_eq!(
        run_with(
            r"@lang_item(add) fn plus(a, b) { return a - b; } return plus(10, 3);",
            std_registry()
        ),
        Value::Number(7.0)
    );
}

// ── combined modifiers ──

#[test]
fn test_lang_item_with_intrinsic() {
    // @intrinsic means lookup by name, @lang_item registers alias
    assert_eq!(
        run_with(
            "@lang_item(neg) @intrinsic fn neg(x); return -42;",
            std_registry()
        ),
        Value::Number(-42.0)
    );
}

#[test]
fn test_intrinsic_only_does_not_register_lang_item() {
    // @intrinsic without @lang_item should NOT affect operators
    let mut reg = std_registry();
    // Add a custom intrinsic that tracks if it was called
    reg.register_intrinsic(FnLookupKey::External("my_add".into()), add);
    assert_eq!(
        run_with("@intrinsic fn my_add(a, b); return my_add(5, 7);", reg,),
        Value::Number(12.0)
    );
}

// ── error cases ──

#[test]
fn test_lang_item_unknown_name() {
    let err = run_with_err("@lang_item(blah) fn foo() { }", std_registry());
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_unknown_modifier() {
    let err = run_with_err("@nonsense fn foo() { }", std_registry());
    assert!(matches!(err, CompileError::Unexpected { .. }));
}

#[test]
fn test_intrinsic_custom_name() {
    // Register an intrinsic with a custom name, declare it in source
    let custom = |args: &[&Value]| -> Value { Value::Number(args[0].as_num() * 100.0) };
    let reg = registry(&[mk_intrinsic("make_big", custom)]);
    assert_eq!(
        run_with("@intrinsic fn make_big(x); return make_big(5);", reg,),
        Value::Number(500.0)
    );
}
