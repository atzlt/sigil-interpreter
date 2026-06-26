use slab::Slab;

use super::upvalue::Upvalue;

#[derive(Debug)]
pub(super) struct Heap {
    upvalues: Slab<Upvalue>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            upvalues: Slab::new(),
        }
    }

    // ── Upvalue pool ──

    /// Allocate a new upvalue, returning its `u16` key.
    pub fn push_upvalue(&mut self, uv: Upvalue) -> u16 {
        self.upvalues.insert(uv) as u16
    }

    /// Immutable access to an upvalue by key.
    pub fn upvalue(&self, key: u16) -> &Upvalue {
        &self.upvalues[key as usize]
    }

    /// Mutable access to an upvalue by key.
    pub fn upvalue_mut(&mut self, key: u16) -> &mut Upvalue {
        &mut self.upvalues[key as usize]
    }
}
