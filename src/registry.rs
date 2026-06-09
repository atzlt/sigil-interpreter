use ahash::AHashMap;

use crate::value::Value;

pub type HostFn = fn(&[&Value]) -> Value;

#[derive(Debug, Default)]
pub struct FunctionRegistry {
    entries: AHashMap<String, HostFn>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        FunctionRegistry {
            entries: AHashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, func: HostFn) {
        self.entries.insert(name.to_string(), func);
    }

    pub fn get(&self, name: &str) -> Option<&HostFn> {
        self.entries.get(name)
    }

    pub fn with_std() -> Self {
        let mut reg = Self::new();

        fn add(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a + b)
        }
        fn sub(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a - b)
        }
        fn mul(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a * b)
        }
        fn div(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a / b)
        }
        fn rem(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a % b)
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
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a < b)
        }
        fn le(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a <= b)
        }
        fn gt(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a > b)
        }
        fn ge(args: &[&Value]) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a >= b)
        }

        reg.register("add", add);
        reg.register("sub", sub);
        reg.register("mul", mul);
        reg.register("div", div);
        reg.register("mod", rem);
        reg.register("neg", neg);
        reg.register("not", not);
        reg.register("eq", eq);
        reg.register("neq", neq);
        reg.register("lt", lt);
        reg.register("le", le);
        reg.register("gt", gt);
        reg.register("ge", ge);
        reg
    }
}
