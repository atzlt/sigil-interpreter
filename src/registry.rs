use ahash::AHashMap;

use crate::value::Value;

pub type HostFn = fn(&[Value]) -> Value;

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
}
