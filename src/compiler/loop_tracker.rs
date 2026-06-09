#[derive(Debug, Default)]
pub struct LoopTracker {
    contexts: Vec<LoopCtx>,
}

#[derive(Debug)]
struct LoopCtx {
    breaks: Vec<usize>,
    continues: Vec<usize>,
    cond_start: usize,
}

#[derive(Debug)]
pub struct LoopPatch {
    pub breaks: Vec<usize>,
    pub continues: Vec<usize>,
    pub cond_start: usize,
}

impl LoopTracker {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
        }
    }

    pub fn push_loop(&mut self, cond_start: usize) {
        self.contexts.push(LoopCtx {
            breaks: Vec::new(),
            continues: Vec::new(),
            cond_start,
        });
    }

    pub fn in_loop(&self) -> bool {
        !self.contexts.is_empty()
    }

    pub fn add_break(&mut self, jmp_ip: usize) {
        self.contexts
            .last_mut()
            .expect("break outside loop")
            .breaks
            .push(jmp_ip);
    }

    pub fn add_continue(&mut self, jmp_ip: usize) {
        self.contexts
            .last_mut()
            .expect("continue outside loop")
            .continues
            .push(jmp_ip);
    }

    pub fn pop_loop(&mut self) -> LoopPatch {
        let ctx = self.contexts.pop().expect("pop_loop with empty stack");
        LoopPatch {
            breaks: ctx.breaks,
            continues: ctx.continues,
            cond_start: ctx.cond_start,
        }
    }
}
