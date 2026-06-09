use sigil_interpreter::{
    compiler::compile::{compile},
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

    reg.register("add", add);
    reg.register("sub", sub);
    reg.register("mul", mul);
    reg.register("div", div);
    reg.register("neg", neg);
    reg.register("ge", ge);
    reg
}

fn run(source: &str) -> Value {
    let mut chunk = compile(source).unwrap();
    println!("{chunk}");
    let registry = math_registry();
    let mut vm = VM::new();
    vm.run(&mut chunk, &registry).unwrap()
}

#[test]
fn test_simple_add() {
    assert_eq!(run("1 + 2"), Value::Number(3.0));
}

#[test]
fn test_precedence() {
    assert_eq!(run("2 * 3 + 1"), Value::Number(7.0));
    assert_eq!(run("1 + 2 * 3"), Value::Number(7.0));
}

#[test]
fn test_grouping() {
    assert_eq!(run("(1 + 2) * 3"), Value::Number(9.0));
}

#[test]
fn test_unary_neg() {
    assert_eq!(run("-5"), Value::Number(-5.0));
    assert_eq!(run("-(1 + 2)"), Value::Number(-3.0));
    assert_eq!(run("-1 + 2"), Value::Number(1.0));
}

#[test]
fn test_nested_ops() {
    assert_eq!(run("1 + 2 + 3"), Value::Number(6.0));
    assert_eq!(run("1 + 2 * 3 - 4 / 2"), Value::Number(5.0));
}

#[test]
fn test_deep_nested_ops() {
    let mut source = (0..=50)
        .map(|_| "0 + 0 * (")
        .collect::<Vec<_>>()
        .join("");
    let source2 = (0..=50)
        .map(|_| ")")
        .collect::<Vec<_>>()
        .join("");
    source.push_str("0");
    source.push_str(&source2);
    assert_eq!(run(&source), Value::Number(0.0));
}

#[test]
fn test_number_literal() {
    assert_eq!(run("42"), Value::Number(42.0));
    assert_eq!(run("3.14"), Value::Number(3.14));
}

#[test]
fn test_literals() {
    assert_eq!(run("true"), Value::Bool(true));
    assert_eq!(run("false"), Value::Bool(false));
    assert_eq!(run("nil"), Value::Nil);
}

#[test]
fn test_string_literal() {
    assert_eq!(run("\"hello\""), Value::String("hello".into()));
}

#[test]
fn test_register_reuse_long_chain() {
    // 100-term addition chain. Without a free list this would burn
    // >200 registers and overflow the 255 limit.
    let source = (1..=500)
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    assert_eq!(run(&source), Value::Number(125250.0));
}

#[test]
fn test_ternary_true() {
    assert_eq!(run("4 >= 3 ? 1 : 2"), Value::Number(1.0));
}

#[test]
fn test_ternary_false() {
    assert_eq!(run("3 >= 4 ? 1 : 2"), Value::Number(2.0));
}

#[test]
fn test_ternary_right_assoc() {
    assert_eq!(run("4 >= 3 ? 2 >= 1 ? 5 : 6 : 7"), Value::Number(5.0));
    assert_eq!(run("3 >= 4 ? 2 >= 1 ? 5 : 6 : 7"), Value::Number(7.0));
    assert_eq!(run("4 >= 3 ? 5 : 2 >= 1 ? 6 : 7"), Value::Number(5.0));
    assert_eq!(run("3 >= 4 ? 5 : 2 >= 1 ? 6 : 7"), Value::Number(6.0));
}

#[test]
fn test_nested_ternary() {
    let mut source = (1..=100)
        .map(|_| "1 >= 2 ? 1 : ")
        .collect::<Vec<_>>()
        .join("");
    source.push_str("2");
    assert_eq!(run(&source), Value::Number(2.0));
}
