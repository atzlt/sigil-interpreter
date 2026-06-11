use std::fmt;

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

impl LangItem {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "add" => Some(Self::Add),
            "sub" => Some(Self::Sub),
            "mul" => Some(Self::Mul),
            "div" => Some(Self::Div),
            "rem" => Some(Self::Rem),
            "neg" => Some(Self::Neg),
            "not" => Some(Self::Not),
            "eq" => Some(Self::Eq),
            "neq" => Some(Self::Neq),
            "lt" => Some(Self::Lt),
            "gt" => Some(Self::Gt),
            "le" => Some(Self::Le),
            "ge" => Some(Self::Ge),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FnModifier {
    Intrinsic,
    LangItem(LangItem),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FnLookupKey {
    LangItem(LangItem),
    Name(Spur),
    External(String),
}

impl fmt::Display for FnLookupKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FnLookupKey::LangItem(item) => write!(f, "lang-item({item})"),
            FnLookupKey::Name(spur) => write!(f, "ƒ_{spur:?}"),
            FnLookupKey::External(name) => write!(f, "{name}"),
        }
    }
}

#[derive(Debug)]
pub enum FnEntry {
    Intrinsic(IntrinsicFn),
    ChunkIdx(usize),
}

#[derive(Debug, Default)]
pub struct FunctionRegistry {
    keys: AHashMap<FnLookupKey, usize>,
    backward: Vec<FnLookupKey>,
    entries: Vec<FnEntry>,
}

impl FunctionRegistry {
    pub fn register_intrinsic(&mut self, name: FnLookupKey, func: IntrinsicFn) {
        self.entries.push(FnEntry::Intrinsic(func));
        self.backward.push(name.clone());
        let id = self.entries.len() - 1;
        self.keys.insert(name, id);
    }

    pub fn register(&mut self, name: FnLookupKey, idx: usize) -> usize {
        self.entries.push(FnEntry::ChunkIdx(idx));
        self.backward.push(name.clone());
        let id = self.entries.len() - 1;
        self.keys.insert(name, id);
        id
    }

    pub fn get_id(&self, name: &FnLookupKey) -> Option<&usize> {
        self.keys.get(name)
    }

    pub fn get(&self, id: &usize) -> Option<&FnEntry> {
        self.entries.get(*id)
    }

    pub fn resolve_id(&self, id: usize) -> FnLookupKey {
        self.backward[id].clone()
    }

    pub fn with_std() -> Self {
        use self::FnLookupKey::LangItem;
        use self::LangItem::*;
        let mut reg = Self::default();

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
        fn print(args: &[&Value]) -> Value {
            let a = args[0];
            println!("{a}");
            Value::Nil
        }

        reg.register_intrinsic(LangItem(Add), add);
        reg.register_intrinsic(FnLookupKey::External("add".into()), add);
        reg.register_intrinsic(LangItem(Sub), sub);
        reg.register_intrinsic(FnLookupKey::External("sub".into()), sub);
        reg.register_intrinsic(LangItem(Mul), mul);
        reg.register_intrinsic(FnLookupKey::External("mul".into()), mul);
        reg.register_intrinsic(LangItem(Div), div);
        reg.register_intrinsic(FnLookupKey::External("div".into()), div);
        reg.register_intrinsic(LangItem(Rem), rem);
        reg.register_intrinsic(FnLookupKey::External("rem".into()), rem);
        reg.register_intrinsic(LangItem(Neg), neg);
        reg.register_intrinsic(FnLookupKey::External("neg".into()), neg);
        reg.register_intrinsic(LangItem(Not), not);
        reg.register_intrinsic(FnLookupKey::External("not".into()), not);
        reg.register_intrinsic(LangItem(Eq), eq);
        reg.register_intrinsic(FnLookupKey::External("eq".into()), eq);
        reg.register_intrinsic(LangItem(Neq), neq);
        reg.register_intrinsic(FnLookupKey::External("neq".into()), neq);
        reg.register_intrinsic(LangItem(Lt), lt);
        reg.register_intrinsic(FnLookupKey::External("lt".into()), lt);
        reg.register_intrinsic(LangItem(Le), le);
        reg.register_intrinsic(FnLookupKey::External("le".into()), le);
        reg.register_intrinsic(LangItem(Gt), gt);
        reg.register_intrinsic(FnLookupKey::External("gt".into()), gt);
        reg.register_intrinsic(LangItem(Ge), ge);
        reg.register_intrinsic(FnLookupKey::External("ge".into()), ge);
        reg.register_intrinsic(FnLookupKey::External("print".into()), print);
        reg
    }
}
