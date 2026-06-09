use lasso::Spur;

use crate::compiler::compile::{CompileError, Compiler, Result};

#[derive(Debug, Clone)]
struct Local {
    id: Spur,
    reg: u8,
    depth: usize,
}

#[derive(Debug, Default)]
pub struct LocalsTracker {
    locals: Vec<Local>,
    current_depth: usize,
}

struct UndefinedVariable;

type LocResult<T> = std::result::Result<T, UndefinedVariable>;

impl LocalsTracker {
    pub fn new() -> Self {
        Self {
            locals: Vec::new(),
            current_depth: 0,
        }
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

    fn resolve_local(&mut self, id: Spur) -> LocResult<u8> {
        if self.locals.is_empty() {
            return Err(UndefinedVariable);
        }
        let mut scanner = self.locals.len() - 1;
        while self.locals[scanner].id != id {
            if scanner == 0 {
                return Err(UndefinedVariable);
            }
            scanner -= 1;
        }
        Ok(self.locals[scanner].reg)
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
    pub(super) fn add_local(&mut self, id: Spur, reg: u8) {
        self.locals.add_local(id, reg);
    }

    pub(super) fn resolve_local(&mut self, id: Spur) -> Result<u8> {
        self.locals
            .resolve_local(id)
            .map_err(|_| CompileError::UndefinedVariable {
                name: self.intern_resolve(&id).to_string(),
                diag: (self.prev_span.clone(), "undefined variable".to_string()),
            })
    }

    pub(super) fn enter_scope(&mut self) {
        self.locals.enter_scope();
    }

    pub(super) fn exit_scope(&mut self) {
        let freed = self.locals.exit_scope();
        self.regs.free_held(&freed);
    }
}
