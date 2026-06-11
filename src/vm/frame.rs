use std::ops::{Index, IndexMut};

use crate::{value::Value, vm::VM};

const MAX_CALL_DEPTH: usize = 256;

#[derive(Debug)]
pub(super) struct Frames {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
}

impl Default for Frames {
    fn default() -> Self {
        Self::new()
    }
}

impl Frames {
    pub fn new() -> Self {
        Self {
            stack: vec![const { Value::Nil }; 256 * MAX_CALL_DEPTH],
            frames: vec![CallFrame::new(0, 0, 0)],
        }
    }

    fn frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }

    fn new_frame(&mut self, chunk_idx: usize, ret_dst: usize, reg_offset: usize) {
        let cur_offset = self.frame().reg_offset;
        let frame = CallFrame::new(chunk_idx, cur_offset + ret_dst, cur_offset + reg_offset + 1);
        self.frames.push(frame);
    }

    /// Returns `true` if we have exited the top-level frame, hence exiting the whole program.
    fn exit_frame(&mut self) -> bool {
        assert!(!self.frames.is_empty());
        self.frames.pop();
        self.frames.is_empty()
    }
}

impl VM {
    fn frame(&self) -> &CallFrame {
        self.frames.frame()
    }

    pub(super) fn chunk_idx(&self) -> usize {
        self.frame().chunk_idx
    }

    pub(super) fn enter_frame(&mut self, chunk_idx: usize, ret_dst: usize, reg_offset: usize) {
        self.frames.new_frame(chunk_idx, ret_dst, reg_offset);
    }

    /// Returns `Some(return_value)` if we have exited the top-level frame, hence exiting the whole program.
    pub(super) fn exit_frame(&mut self, res_reg: usize) -> Option<Value> {
        let dst = self.frame().ret_dst;
        let ret_val = self.stack()[res_reg].clone();
        if self.frames.exit_frame() {
            Some(ret_val)
        } else {
            self.stack_mut()[dst] = ret_val;
            None
        }
    }

    pub(super) fn stack(&self) -> StackWindow<'_> {
        let offset = self.frame().reg_offset;
        StackWindow::new(&self.frames.stack, offset)
    }

    pub(super) fn stack_mut(&mut self) -> StackWindowMut<'_> {
        let offset = self.frame().reg_offset;
        StackWindowMut::new(&mut self.frames.stack, offset)
    }
}

/// The `ret_dst` and `reg_offset` in this struct is _absolute_.
#[derive(Debug)]
struct CallFrame {
    chunk_idx: usize,
    ret_dst: usize,
    reg_offset: usize,
}

impl CallFrame {
    fn new(chunk_idx: usize, ret_dst: usize, reg_offset: usize) -> Self {
        Self {
            chunk_idx,
            ret_dst,
            reg_offset,
        }
    }
}

pub struct StackWindow<'a> {
    stack_ref: &'a [Value],
    offset: usize,
}

impl<'a> StackWindow<'a> {
    fn new(stack_ref: &'a [Value], offset: usize) -> Self {
        Self { stack_ref, offset }
    }

    fn get(&self, index: usize) -> &'a Value {
        &self.stack_ref[index + self.offset]
    }
}

impl<'a> Index<usize> for StackWindow<'a> {
    type Output = Value;

    fn index(&self, index: usize) -> &'a Self::Output {
        self.get(index)
    }
}

pub struct StackWindowMut<'a> {
    stack_ref: &'a mut [Value],
    offset: usize,
}

impl<'a> StackWindowMut<'a> {
    fn new(stack_ref: &'a mut [Value], offset: usize) -> Self {
        Self { stack_ref, offset }
    }
}

impl<'a> Index<usize> for StackWindowMut<'a> {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stack_ref[index + self.offset]
    }
}

impl<'a> IndexMut<usize> for StackWindowMut<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.stack_ref[index + self.offset]
    }
}
