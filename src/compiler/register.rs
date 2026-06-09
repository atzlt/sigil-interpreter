use crate::compiler::compile::{CompileError, Compiler, Result};

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
enum RegState {
    Held,
    Temp,
    #[default]
    Free,
}

#[derive(Debug)]
pub struct RegisterTracker {
    state: Vec<RegState>,
    held_pt: usize,
    temp_pt: usize,
    temp_first_run: bool,
}

struct RegisterOverflow;

type RegResult<T> = std::result::Result<T, RegisterOverflow>;

impl RegisterTracker {
    pub fn new(size: usize) -> Self {
        assert!(size <= 256);
        Self {
            state: vec![RegState::Free; size],
            held_pt: 0,
            temp_pt: size - 1,
            temp_first_run: true,
        }
    }

    fn alloc_held(&mut self) -> RegResult<u8> {
        if self.held_pt >= self.state.len() {
            return Err(RegisterOverflow);
        }
        assert_ne!(self.state[self.held_pt], RegState::Held);
        let new_reg = self.held_pt;
        self.state[new_reg] = RegState::Held;
        self.held_pt += 1;
        Ok(new_reg as u8)
    }

    fn alloc_temp(&mut self) -> RegResult<u8> {
        if self.temp_first_run {
            if self.state[self.temp_pt] == RegState::Free {
                self.state[self.temp_pt] = RegState::Temp;
                return Ok(self.temp_pt as u8);
            }
            self.temp_pt -= 1;
            if self.temp_pt > self.held_pt {
                let new_reg = self.temp_pt;
                self.state[new_reg] = RegState::Temp;
                return Ok(new_reg as u8);
            } else {
                self.temp_pt = self.state.len() - 1;
                self.temp_first_run = false;
            }
        }
        let mut scanner = self.temp_pt;
        loop {
            if self.state[scanner] == RegState::Free {
                self.state[scanner] = RegState::Temp;
                self.temp_pt = scanner;
                return Ok(scanner as u8);
            } else {
                scanner -= 1;
                if scanner <= self.held_pt {
                    scanner = self.state.len() - 1;
                }
                if scanner == self.temp_pt {
                    return Err(RegisterOverflow);
                }
            }
        }
    }

    fn is_reusable(&mut self, reg: u8) -> bool {
        (reg as usize) < self.state.len() && self.state[reg as usize] != RegState::Held
    }

    /// This is a no-op on Held registers.
    fn free_reg(&mut self, reg: u8) {
        if (reg as usize) < self.state.len() && self.state[reg as usize] == RegState::Temp {
            self.state[reg as usize] = RegState::Free;
        }
    }

    fn clear_temp(&mut self) {
        self.temp_pt = self.state.len() - 1;
        self.temp_first_run = true;
    }

    fn clear_all(&mut self) {
        self.held_pt = 0;
        self.clear_temp();
    }
}

impl<'a> Compiler<'a> {
    pub(super) fn alloc_temp(&mut self) -> Result<u8> {
        self.regs.alloc_temp().map_err(|_| {
            CompileError::RegisterOverflow((
                self.current.1.clone(),
                "register overflown here".to_string(),
            ))
        })
    }

    pub(super) fn alloc_held(&mut self) -> Result<u8> {
        self.regs.alloc_held().map_err(|_| {
            CompileError::RegisterOverflow((
                self.current.1.clone(),
                "register overflown here".to_string(),
            ))
        })
    }

    fn clear_temp(&mut self) {
        self.regs.clear_temp();
    }

    fn clear_all(&mut self) {
        self.regs.clear_all();
    }

    pub(super) fn reuse_or_alloc(&mut self, ops: &[u8]) -> Result<u8> {
        for &op in ops {
            if self.regs.is_reusable(op) {
                return Ok(op);
            }
        }
        self.alloc_temp()
    }

    pub(super) fn free_others(&mut self, dst: u8, ops: &[u8]) {
        for &op in ops {
            if dst != op {
                self.regs.free_reg(op);
            }
        }
    }
}
