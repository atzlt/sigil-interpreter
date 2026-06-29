use std::fmt;

use ahash::AHashMap;
use lasso::Spur;
use strum_macros::{Display, FromRepr};

use crate::value::Value;
use crate::vm::heap::Heap;

/// Context passed to intrinsic functions at runtime.
/// Currently contains only the heap (for struct field access, value comparison, etc.).
/// Will be extended as more VM state needs to be exposed to intrinsics.
pub struct IntrinsicContext<'h> {
    pub heap: &'h Heap,
}

pub type IntrinsicFn = fn(&[&Value], &IntrinsicContext) -> Value;

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
    /// Anonymous closure — auto-generated numeric ID.
    Anon(u32),
}

impl fmt::Display for FnLookupKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FnLookupKey::LangItem(item) => write!(f, "lang-item({item})"),
            FnLookupKey::Name(spur) => write!(f, "ƒ_{spur:?}"),
            FnLookupKey::External(name) => write!(f, "{name}"),
            FnLookupKey::Anon(id) => write!(f, "<closure#{id}>"),
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
        use self::FnLookupKey::*;
        use self::LangItem::*;
        let mut reg = Self::default();

        fn add(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a + b)
        }
        fn sub(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a - b)
        }
        fn mul(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a * b)
        }
        fn div(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a / b)
        }
        fn rem(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Number(a % b)
        }
        fn neg(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            Value::Number(-args[0].as_num())
        }
        fn not(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            Value::Bool(!args[0].is_truthy())
        }
        fn eq(args: &[&Value], ctx: &IntrinsicContext) -> Value {
            Value::Bool(values_eq(args[0], args[1], ctx.heap))
        }
        fn neq(args: &[&Value], ctx: &IntrinsicContext) -> Value {
            Value::Bool(!values_eq(args[0], args[1], ctx.heap))
        }
        fn lt(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a < b)
        }
        fn le(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a <= b)
        }
        fn gt(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a > b)
        }
        fn ge(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0].as_num();
            let b = args[1].as_num();
            Value::Bool(a >= b)
        }
        fn print(args: &[&Value], _ctx: &IntrinsicContext) -> Value {
            let a = args[0];
            println!("{a}");
            Value::Nil
        }

        reg.register_intrinsic(LangItem(Add), add);
        reg.register_intrinsic(External("add".into()), add);
        reg.register_intrinsic(LangItem(Sub), sub);
        reg.register_intrinsic(External("sub".into()), sub);
        reg.register_intrinsic(LangItem(Mul), mul);
        reg.register_intrinsic(External("mul".into()), mul);
        reg.register_intrinsic(LangItem(Div), div);
        reg.register_intrinsic(External("div".into()), div);
        reg.register_intrinsic(LangItem(Rem), rem);
        reg.register_intrinsic(External("rem".into()), rem);
        reg.register_intrinsic(LangItem(Neg), neg);
        reg.register_intrinsic(External("neg".into()), neg);
        reg.register_intrinsic(LangItem(Not), not);
        reg.register_intrinsic(External("not".into()), not);
        reg.register_intrinsic(LangItem(Eq), eq);
        reg.register_intrinsic(External("eq".into()), eq);
        reg.register_intrinsic(LangItem(Neq), neq);
        reg.register_intrinsic(External("neq".into()), neq);
        reg.register_intrinsic(LangItem(Lt), lt);
        reg.register_intrinsic(External("lt".into()), lt);
        reg.register_intrinsic(LangItem(Le), le);
        reg.register_intrinsic(External("le".into()), le);
        reg.register_intrinsic(LangItem(Gt), gt);
        reg.register_intrinsic(External("gt".into()), gt);
        reg.register_intrinsic(LangItem(Ge), ge);
        reg.register_intrinsic(External("ge".into()), ge);
        reg.register_intrinsic(External("print".into()), print);
        reg
    }
}

/// Recursive value equality that traverses struct fields via the heap.
/// Used by the `==` and `!=` intrinsics.
fn values_eq(a: &Value, b: &Value, heap: &Heap) -> bool {
    match (a, b) {
        (Value::Struct(ka), Value::Struct(kb)) => {
            let sa = heap.struct_ref(*ka);
            let sb = heap.struct_ref(*kb);
            if sa.def_id != sb.def_id || sa.fields.len() != sb.fields.len() {
                return false;
            }
            sa.fields
                .iter()
                .zip(sb.fields.iter())
                .all(|(fa, fb)| values_eq(fa, fb, heap))
        }
        _ => a == b,
    }
}
