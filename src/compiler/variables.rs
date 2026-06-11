use crate::{compiler::compile::{Compiler, Result}, emit, emit_args};
use ahash::AHashMap;
use lasso::Spur;

#[derive(Debug, Default)]
pub(super) struct GlobalStore {
    slots: AHashMap<Spur, u16>,
    ptr: u16,
}

impl GlobalStore {
    pub fn declare(&mut self, id: Spur) -> u16 {
        if let Some(slot) = self.slots.get(&id) {
            *slot
        } else {
            let slot = self.ptr;
            self.slots.insert(id, self.ptr);
            self.ptr += 1;
            slot
        }
    }

    pub fn resolve(&self, id: Spur) -> Option<u16> {
        self.slots.get(&id).copied()
    }
}

#[derive(Debug, Clone)]
struct Local {
    id: Spur,
    reg: u8,
    depth: usize,
}

#[derive(Debug, Default)]
pub(super) struct LocalsTracker {
    locals: Vec<Local>,
    current_depth: usize,
}

impl LocalsTracker {
    pub fn new_with(args: &[Spur]) -> Self {
        Self {
            locals: args
                .iter()
                .enumerate()
                .map(|(i, a)| Local {
                    id: *a,
                    reg: i as u8,
                    depth: 0,
                })
                .collect(),
            current_depth: 0,
        }
    }

    pub fn is_top_level(&self) -> bool {
        self.current_depth == 0
    }

    fn add_local(&mut self, id: Spur, reg: u8) {
        let local = Local {
            id,
            reg,
            depth: self.current_depth,
        };
        if !self.locals.is_empty() {
            assert!(self.locals.last().unwrap().depth <= local.depth);
        }
        self.locals.push(local);
    }

    fn enter_scope(&mut self) {
        self.current_depth += 1;
    }

    fn resolve_local(&mut self, id: Spur) -> Option<u8> {
        if self.locals.is_empty() {
            return None;
        }
        let mut scanner = self.locals.len() - 1;
        while self.locals[scanner].id != id {
            if scanner == 0 {
                return None;
            }
            scanner -= 1;
        }
        Some(self.locals[scanner].reg)
    }

    fn exit_scope(&mut self) -> Vec<u8> {
        assert!(self.current_depth > 0);
        self.current_depth -= 1;
        let mut freed = Vec::new();
        while let Some(loc) = self.locals.last()
            && loc.depth > self.current_depth
        {
            freed.push(loc.reg);
            self.locals.pop();
        }
        freed.reverse();
        freed
    }
}

impl<'a> Compiler<'a> {
    pub(super) fn declare_global(&mut self, id: Spur) -> u16 {
        self.globals.declare(id)
    }

    pub(super) fn store_global_fn(&mut self, name: Spur, id: usize) -> Result<()> {
        let slot = self.globals.declare(name);
        let temp = self.alloc_temp()?;
        emit!(self.chunk_mut(), LOADFUN, temp, wide id as u16);
        emit!(self.chunk_mut(), SETGLB, wide slot, temp);
        self.frame_mut().regs.free_temp(temp);
        Ok(())
    }

    pub(super) fn resolve_global(&self, id: Spur) -> Option<u16> {
        self.globals.resolve(id)
    }

    pub(super) fn add_local(&mut self, id: Spur, reg: u8) {
        self.frame_mut().locals.add_local(id, reg);
    }

    pub(super) fn try_resolve_local(&mut self, id: Spur) -> Option<u8> {
        self.frame_mut().locals.resolve_local(id)
    }

    pub(super) fn is_top_level(&self) -> bool {
        self.frame().locals.is_top_level()
    }

    pub(super) fn enter_scope(&mut self) {
        self.frame_mut().locals.enter_scope();
    }

    pub(super) fn exit_scope(&mut self) {
        let freed = self.frame_mut().locals.exit_scope();
        self.frame_mut().regs.free_held(&freed);
    }
}
