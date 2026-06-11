use ahash::AHashMap;
use lasso::Spur;
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
pub enum FnLookupKey {
    #[strum(to_string = "lang-item({0})")]
    LangItem(LangItem),
    #[strum(to_string = "ƒ_{0:?}")]
    Name(Spur),
}

#[derive(Debug)]
pub enum FnEntry {
    Intrinsic(IntrinsicFn),
    ChunkIdx(usize),
}

#[derive(Debug, Default)]
pub struct FunctionRegistry {
    keys: AHashMap<FnLookupKey, usize>,
    entries: Vec<FnEntry>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        FunctionRegistry {
            keys: AHashMap::new(),
            entries: Vec::new(),
        }
    }

    pub fn register_intrinsic(&mut self, name: FnLookupKey, func: IntrinsicFn) {
        self.entries.push(FnEntry::Intrinsic(func));
        let id = self.entries.len() - 1;
        self.keys.insert(name, id);
    }

    pub fn register(&mut self, name: FnLookupKey, idx: usize) {
        self.entries.push(FnEntry::ChunkIdx(idx));
        let id = self.entries.len() - 1;
        self.keys.insert(name, id);
    }

    pub fn get_id(&self, name: &FnLookupKey) -> Option<&usize> {
        self.keys.get(name)
    }

    pub fn get(&self, id: &usize) -> Option<&FnEntry> {
        self.entries.get(*id)
    }

    pub fn with_std() -> Self {
        use self::FnLookupKey::LangItem;
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

        reg.register_intrinsic(LangItem(Add), add);
        reg.register_intrinsic(LangItem(Sub), sub);
        reg.register_intrinsic(LangItem(Mul), mul);
        reg.register_intrinsic(LangItem(Div), div);
        reg.register_intrinsic(LangItem(Rem), rem);
        reg.register_intrinsic(LangItem(Neg), neg);
        reg.register_intrinsic(LangItem(Not), not);
        reg.register_intrinsic(LangItem(Eq), eq);
        reg.register_intrinsic(LangItem(Neq), neq);
        reg.register_intrinsic(LangItem(Lt), lt);
        reg.register_intrinsic(LangItem(Le), le);
        reg.register_intrinsic(LangItem(Gt), gt);
        reg.register_intrinsic(LangItem(Ge), ge);
        reg
    }
}
