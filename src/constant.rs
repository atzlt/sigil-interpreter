use ahash::AHashMap;

use super::value::Value;

#[derive(Debug, Default)]
pub struct ConstantPool {
    map: AHashMap<Value, u16>,
    vec: Vec<Value>,
}

impl ConstantPool {
    pub fn new() -> Self {
        ConstantPool {
            map: AHashMap::new(),
            vec: Vec::new(),
        }
    }

    pub fn intern(&mut self, value: Value) -> u16 {
        if let Some(&idx) = self.map.get(&value) {
            return idx;
        }
        let idx = self.vec.len() as u16;
        self.vec.push(value.clone());
        self.map.insert(value, idx);
        idx
    }

    pub fn get(&self, idx: u16) -> &Value {
        &self.vec[idx as usize]
    }
}
