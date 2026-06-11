use super::label::Label;

#[derive(Debug)]
struct LoopLabels {
    break_label: Label,
    continue_label: Label,
}

#[derive(Debug, Default)]
pub struct LoopTracker {
    contexts: Vec<LoopLabels>,
}

impl LoopTracker {
    pub fn new() -> Self {
        Self {
            contexts: Vec::new(),
        }
    }

    pub fn push_loop(&mut self, break_label: Label, continue_label: Label) {
        self.contexts.push(LoopLabels {
            break_label,
            continue_label,
        });
    }

    pub fn in_loop(&self) -> bool {
        !self.contexts.is_empty()
    }

    pub fn pop_loop(&mut self) {
        self.contexts.pop().expect("pop_loop with empty stack");
    }

    pub fn break_label(&self) -> Label {
        self.contexts.last().expect("not in loop").break_label
    }

    pub fn continue_label(&self) -> Label {
        self.contexts.last().expect("not in loop").continue_label
    }
}
