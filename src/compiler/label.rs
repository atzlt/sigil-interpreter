use crate::vm::Chunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Label(usize);

#[derive(Debug, Clone, Copy)]
pub(super) enum RefKind {
    Jmp,
    Test,
}

#[derive(Debug)]
enum LabelState {
    Pending(Vec<(usize, RefKind)>),
    Placed(usize),
}

#[derive(Debug, Default)]
pub(super) struct LabelTracker {
    labels: Vec<LabelState>,
}

impl LabelTracker {
    pub fn alloc(&mut self) -> Label {
        let id = self.labels.len();
        self.labels.push(LabelState::Pending(Vec::new()));
        Label(id)
    }

    pub fn add_ref(&mut self, label: Label, ip: usize, kind: RefKind) {
        match &mut self.labels[label.0] {
            LabelState::Pending(refs) => refs.push((ip, kind)),
            _ => panic!("LabelTracker::add_ref on placed label"),
        }
    }

    pub fn resolve(&mut self, label: Label, target_ip: usize, chunk: &mut Chunk) {
        let state = std::mem::replace(&mut self.labels[label.0], LabelState::Placed(target_ip));
        if let LabelState::Pending(refs) = state {
            for (ip, kind) in refs {
                match kind {
                    RefKind::Jmp => {
                        let offset = (target_ip as isize - ip as isize) as i16;
                        let bytes = offset.to_le_bytes();
                        chunk.code[ip + 1] = bytes[0];
                        chunk.code[ip + 2] = bytes[1];
                    }
                    RefKind::Test => {
                        let offset = (target_ip - ip) as u16;
                        let bytes = offset.to_le_bytes();
                        chunk.code[ip + 2] = bytes[0];
                        chunk.code[ip + 3] = bytes[1];
                    }
                }
            }
        }
    }

    pub fn ip_of(&self, label: Label) -> usize {
        match &self.labels[label.0] {
            LabelState::Placed(ip) => *ip,
            _ => panic!("LabelTracker::ip_of on unplaced label"),
        }
    }
}
