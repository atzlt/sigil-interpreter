use slab::Slab;

use crate::value::Value;

use super::upvalue::Upvalue;

#[derive(Debug, Clone)]
pub struct StructDefHeader {
    pub field_names: Vec<String>,
}

/// A heap-allocated struct instance.
#[derive(Debug, Clone)]
pub struct StructObject {
    pub def_id: u16,
    pub fields: Box<[Value]>,
}

#[derive(Debug)]
pub struct Heap {
    upvalues: Slab<Upvalue>,
    pub struct_defs: Vec<StructDefHeader>,
    pub structs: Slab<StructObject>,
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

impl Heap {
    pub fn new() -> Self {
        Self {
            upvalues: Slab::new(),
            struct_defs: Vec::new(),
            structs: Slab::new(),
        }
    }

    // ── Upvalue pool ──

    pub fn push_upvalue(&mut self, uv: Upvalue) -> u16 {
        self.upvalues.insert(uv) as u16
    }

    pub fn upvalue(&self, key: u16) -> &Upvalue {
        &self.upvalues[key as usize]
    }

    pub fn upvalue_mut(&mut self, key: u16) -> &mut Upvalue {
        &mut self.upvalues[key as usize]
    }

    // ── Struct definitions ──

    pub fn ensure_struct_def(&mut self, def_id: u16, field_names: Vec<String>) {
        let idx = def_id as usize;
        if idx >= self.struct_defs.len() {
            self.struct_defs.resize(
                idx + 1,
                StructDefHeader {
                    field_names: Vec::new(),
                },
            );
        }
        if self.struct_defs[idx].field_names.is_empty() {
            self.struct_defs[idx].field_names = field_names;
        }
    }

    pub fn struct_field_index(&self, def_id: u16, name: &str) -> Option<usize> {
        self.struct_defs
            .get(def_id as usize)?
            .field_names
            .iter()
            .position(|n| n == name)
    }

    // ── Struct instances ──

    pub fn push_struct(&mut self, obj: StructObject) -> u16 {
        self.structs.insert(obj) as u16
    }

    pub fn struct_ref(&self, key: u16) -> &StructObject {
        &self.structs[key as usize]
    }

    pub fn struct_mut(&mut self, key: u16) -> &mut StructObject {
        &mut self.structs[key as usize]
    }
}
