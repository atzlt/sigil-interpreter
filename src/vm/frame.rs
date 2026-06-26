use std::ops::{Index, IndexMut};

use smallvec::SmallVec;

use crate::value::Value;
use crate::vm::Chunk;

#[derive(Debug)]
pub(super) struct CallFrame<'c> {
    pub(super) ret_dst: usize,
    pub(super) reg_offset: usize,
    pub(super) ip: usize,
    pub(super) chunk: &'c Chunk,
    /// Slab keys of open upvalues pointing into this frame's stack.
    /// Sorted by absolute stack position for efficient batch-closing.
    pub(super) open_upvalues: Vec<u16>,
    /// Upvalue indices captured by the closure executing in this frame.
    /// `closure_upvalues[i]` is the slab key for the i-th upvalue.
    /// Empty for top-level functions and intrinsics.
    pub(super) closure_upvalues: SmallVec<[u16; 4]>,
}

impl<'c> CallFrame<'c> {
    pub(super) fn new(
        ret_dst: usize,
        reg_offset: usize,
        chunk: &'c Chunk,
        closure_upvalues: SmallVec<[u16; 4]>,
    ) -> Self {
        Self {
            ret_dst,
            reg_offset,
            ip: 0,
            chunk,
            open_upvalues: Vec::new(),
            closure_upvalues,
        }
    }
}

pub struct StackWindow<'a> {
    stack_ref: &'a [Value],
    offset: usize,
}

impl<'a> StackWindow<'a> {
    pub(super) fn new(stack_ref: &'a [Value], offset: usize) -> Self {
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
    pub(super) fn new(stack_ref: &'a mut [Value], offset: usize) -> Self {
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
