use ahash::AHashMap;
use strum_macros::{Display, FromRepr};

use crate::value::Value;

pub type IntrinsicFn = fn(&[&Value]) -> Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, Display)]
#[repr(u8)]
pub enum LangItem {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    Not,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display)]
pub enum FnId {
    #[strum(to_string = "lang-item({0})")]
    LangItem(LangItem),
}

#[derive(Debug)]
pub enum FnType {
    Intrinsic(IntrinsicFn),
    Chunk(usize),
}

#[derive(Debug, Default)]
pub struct FunctionRegistry {
    entries: AHashMap<FnId, FnType>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        FunctionRegistry {
            entries: AHashMap::new(),
        }
    }

    pub fn register(&mut self, name: FnId, func: IntrinsicFn) {
        self.entries.insert(name, FnType::Intrinsic(func));
    }

    pub fn get(&self, name: &FnId) -> Option<&FnType> {
        self.entries.get(name)
    }

    pub fn with_std() -> Self {
        use self::FnId::LangItem;
        use self::LangItem::*;
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

        reg.register(LangItem(Add), add);
        reg.register(LangItem(Sub), sub);
        reg.register(LangItem(Mul), mul);
        reg.register(LangItem(Div), div);
        reg.register(LangItem(Rem), rem);
        reg.register(LangItem(Neg), neg);
        reg.register(LangItem(Not), not);
        reg.register(LangItem(Eq), eq);
        reg.register(LangItem(Neq), neq);
        reg.register(LangItem(Lt), lt);
        reg.register(LangItem(Le), le);
        reg.register(LangItem(Gt), gt);
        reg.register(LangItem(Ge), ge);
        reg
    }
}
