use crate::compiler::compile::Compiler;
use ahash::AHashMap;
use lasso::Spur;

#[derive(Debug, Default)]
struct GlobalStore {
    slots: AHashMap<Spur, u16>,
    ptr: u16,
}

impl GlobalStore {
    pub fn declare(&mut self, id: Spur) -> u16 {
        if let Some(slot) = self.slots.get(&id) {
            *slot
        } else {
            self.ptr += 1;
            self.slots.insert(id, self.ptr);
            self.ptr
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
struct LocalsTracker {
    locals: Vec<Local>,
    current_depth: usize,
}

impl LocalsTracker {
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

#[derive(Debug, Default)]
pub struct Variables {
    globals: GlobalStore,
    locals: LocalsTracker,
}

impl<'a> Compiler<'a> {
    pub(super) fn declare_global(&mut self, id: Spur) -> u16 {
        self.frame_mut().vars.globals.declare(id)
    }

    pub(super) fn resolve_global(&self, id: Spur) -> Option<u16> {
        self.frame().vars.globals.resolve(id)
    }

    pub(super) fn add_local(&mut self, id: Spur, reg: u8) {
        self.frame_mut().vars.locals.add_local(id, reg);
    }

    pub(super) fn try_resolve_local(&mut self, id: Spur) -> Option<u8> {
        self.frame_mut().vars.locals.resolve_local(id)
    }

    pub(super) fn is_top_level(&self) -> bool {
        self.frame().vars.locals.is_top_level()
    }

    pub(super) fn enter_scope(&mut self) {
        self.frame_mut().vars.locals.enter_scope();
    }

    pub(super) fn exit_scope(&mut self) {
        let freed = self.frame_mut().vars.locals.exit_scope();
        self.frame_mut().regs.free_held(&freed);
    }
}
