use std::fmt;

use ahash::AHashMap;
use lasso::Spur;
use strum_macros::{Display, FromRepr};

use crate::types::{TypeId};
use crate::value::Value;
use crate::vm::heap::Heap;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct FnTypeSig {
    pub param_types: Vec<TypeId>,
}

#[derive(Debug, Clone)]
struct FnOverloads {
    overloads: AHashMap<FnTypeSig, usize>,
}

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
    /// Anonymous closure
    Anon(u32),
}

impl fmt::Display for FnLookupKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FnLookupKey::LangItem(item) => write!(f, "lang-item({item})"),
            FnLookupKey::Name(spur) => write!(f, "<fn#{spur:?}>"),
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
    keys: AHashMap<FnLookupKey, FnOverloads>,
    backward: Vec<FnLookupKey>, // backward queries lookup key from entry id
    entries: Vec<FnEntry>,
}

impl FunctionRegistry {
    pub fn register(&mut self, name: FnLookupKey, entry: FnEntry, signature: FnTypeSig) -> usize {
        self.entries.push(entry);
        let id = self.entries.len() - 1;
        self.backward.push(name.clone());

        if let Some(overloads) = self.get_overloads_mut(&name) {
            overloads.overloads.insert(signature, id);
        } else {
            let mut overloads = FnOverloads {
                overloads: AHashMap::new(),
            };
            overloads.overloads.insert(signature, id);
            self.keys.insert(name.clone(), overloads);
        }
        id
    }

    fn get_overloads(&self, name: &FnLookupKey) -> Option<&FnOverloads> {
        self.keys.get(name)
    }

    fn get_overloads_mut(&mut self, name: &FnLookupKey) -> Option<&mut FnOverloads> {
        self.keys.get_mut(name)
    }

    pub fn get(&self, id: &usize) -> Option<&FnEntry> {
        self.entries.get(*id)
    }

    pub fn resolve_id(&self, id: usize) -> &FnLookupKey {
        &self.backward[id]
    }

    pub fn get_static_id(&self, name: &FnLookupKey) -> Option<usize> {
        self.get_overloads(name)
            .and_then(|ov| ov.overloads.values().next())
            .copied()
    }

    /// Runtime overload resolution by least-cost matching:
    ///   Exact match:   cost 0
    ///   Subtype match: cost 1
    ///   Type mismatch: disqualifies
    /// The overload with the lowest total cost wins. This algorithm short-circuits on the first exact match.
    pub fn resolve_overload(
        &self,
        name: &FnLookupKey,
        args: &[&Value],
        heap: &Heap,
    ) -> Option<usize> {
        let overloads = self.get_overloads(name)?;
        let arg_types: Vec<TypeId> = args.iter().map(|v| v.type_id(heap)).collect();

        let mut best_cost: u32 = u32::MAX;
        let mut best_id: usize = 0;

        for (sig, &fn_id) in &overloads.overloads {
            if sig.param_types.len() != arg_types.len() {
                continue;
            }
            let mut cost: u32 = 0;
            let mut ok = true;
            for (p, a) in sig.param_types.iter().zip(arg_types.iter()) {
                if p == a {
                    // exact match: cost 0
                } else if *p == TypeId::Any {
                    cost += 1;
                } else {
                    ok = false;
                    break;
                }
            }
            if ok && cost < best_cost {
                best_cost = cost;
                best_id = fn_id;
                if cost == 0 {
                    break; // exact match found, no need to continue
                }
            }
        }
        if best_cost < u32::MAX { Some(best_id) } else { None }
    }

    pub fn with_std() -> Self {
        use self::FnLookupKey::*;
        use self::LangItem::*;
        use self::FnEntry::Intrinsic;
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

        let num_bin_sig = FnTypeSig {
            param_types: vec![TypeId::Number, TypeId::Number],
        };
        let num_un_sig = FnTypeSig {
            param_types: vec![TypeId::Number],
        };
        let any_un_sig = FnTypeSig {
            param_types: vec![TypeId::Any],
        };
        let any_bin_sig = FnTypeSig {
            param_types: vec![TypeId::Any, TypeId::Any],
        };

        reg.register(LangItem(Add), Intrinsic(add), num_bin_sig.clone());
        reg.register(External("add".into()), Intrinsic(add), num_bin_sig.clone());
        reg.register(LangItem(Sub), Intrinsic(sub), num_bin_sig.clone());
        reg.register(External("sub".into()), Intrinsic(sub), num_bin_sig.clone());
        reg.register(LangItem(Mul), Intrinsic(mul), num_bin_sig.clone());
        reg.register(External("mul".into()), Intrinsic(mul), num_bin_sig.clone());
        reg.register(LangItem(Div), Intrinsic(div), num_bin_sig.clone());
        reg.register(External("div".into()), Intrinsic(div), num_bin_sig.clone());
        reg.register(LangItem(Rem), Intrinsic(rem), num_bin_sig.clone());
        reg.register(External("rem".into()), Intrinsic(rem), num_bin_sig.clone());
        reg.register(LangItem(Neg), Intrinsic(neg), num_un_sig.clone());
        reg.register(External("neg".into()), Intrinsic(neg), num_un_sig.clone());
        reg.register(LangItem(Not), Intrinsic(not), any_un_sig.clone());
        reg.register(External("not".into()), Intrinsic(not), any_un_sig.clone());
        reg.register(LangItem(Eq), Intrinsic(eq), any_bin_sig.clone());
        reg.register(External("eq".into()), Intrinsic(eq), any_bin_sig.clone());
        reg.register(LangItem(Neq), Intrinsic(neq), any_bin_sig.clone());
        reg.register(External("neq".into()), Intrinsic(neq), any_bin_sig.clone());
        reg.register(LangItem(Lt), Intrinsic(lt), num_bin_sig.clone());
        reg.register(External("lt".into()), Intrinsic(lt), num_bin_sig.clone());
        reg.register(LangItem(Le), Intrinsic(le), num_bin_sig.clone());
        reg.register(External("le".into()), Intrinsic(le), num_bin_sig.clone());
        reg.register(LangItem(Gt), Intrinsic(gt), num_bin_sig.clone());
        reg.register(External("gt".into()), Intrinsic(gt), num_bin_sig.clone());
        reg.register(LangItem(Ge), Intrinsic(ge), num_bin_sig.clone());
        reg.register(External("ge".into()), Intrinsic(ge), num_bin_sig.clone());
        reg.register(External("print".into()), Intrinsic(print), any_un_sig.clone());
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
