use smallvec::SmallVec;
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Default)]
pub enum Value {
    #[default]
    Nil,
    Bool(bool),
    Number(f64),
    String(String),
    /// A plain function without captured upvalues.
    Fn(usize),
    /// A closure — a function with captured upvalues (indices into `VM.upvalues`).
    Closure {
        fn_id: usize,
        upvalues: SmallVec<[u16; 4]>,
    },
    /// Function prototype stored in the constant pool.
    /// The `CLOSURE` opcode reads this to create a runtime `Fn` value
    /// with captured upvalues.
    FnProto {
        fn_id: usize,
        upvalue_count: u16,
    },
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a.to_bits() == b.to_bits(),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Fn(a), Value::Fn(b)) => a == b,
            (Value::Closure { fn_id: a, .. }, Value::Closure { fn_id: b, .. }) => a == b,
            (
                Value::FnProto {
                    fn_id: a,
                    upvalue_count: ac,
                },
                Value::FnProto {
                    fn_id: b,
                    upvalue_count: bc,
                },
            ) => a == b && ac == bc,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Nil => {}
            Value::Bool(b) => b.hash(state),
            Value::Number(n) => n.to_bits().hash(state),
            Value::String(s) => s.hash(state),
            Value::Fn(f) => f.hash(state),
            Value::Closure { fn_id, .. } => fn_id.hash(state),
            Value::FnProto {
                fn_id,
                upvalue_count,
            } => {
                fn_id.hash(state);
                upvalue_count.hash(state);
            }
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Fn(_) => true,
            Value::Closure { .. } => true,
            Value::FnProto { .. } => true,
        }
    }

    pub fn as_num(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            _ => 0.0,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::String(s) => write!(f, "{s}"),
            Value::Fn(fun) => write!(f, "ƒ_{fun:?}"),
            Value::Closure { fn_id, upvalues } => {
                write!(f, "ƒ_{fn_id:?}[{} up]", upvalues.len())
            }
            Value::FnProto {
                fn_id,
                upvalue_count,
            } => {
                write!(f, "<proto ƒ_{fn_id:?} {upvalue_count} up>")
            }
        }
    }
}
