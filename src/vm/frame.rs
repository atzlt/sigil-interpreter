use std::ops::{Index, IndexMut};

use crate::{
    value::Value,
    vm::{Chunk, ChunkReader, VM},
};

const MAX_CALL_DEPTH: usize = 256;

#[derive(Debug)]
pub(super) struct Frames<'c> {
    stack: Vec<Value>,
    frames: Vec<CallFrame<'c>>,
}

impl Default for Frames<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'c> Frames<'c> {
    pub fn new() -> Self {
        Self {
            stack: vec![const { Value::Nil }; 256 * MAX_CALL_DEPTH],
            frames: Vec::new(),
        }
    }

    fn frame(&self) -> &CallFrame<'_> {
        self.frames.last().unwrap()
    }

    pub(super) fn read(&mut self) -> u8 {
        self.frames.last_mut().unwrap().reader.read()
    }

    pub(super) fn read_i16(&mut self) -> i16 {
        self.frames.last_mut().unwrap().reader.read_i16()
    }

    pub(super) fn read_wide(&mut self) -> u16 {
        self.frames.last_mut().unwrap().reader.read_wide()
    }

    pub(super) fn ip(&self) -> usize {
        self.frames.last().unwrap().reader.ip
    }

    pub(super) fn set_ip(&mut self, new: usize) {
        self.frames.last_mut().unwrap().reader.ip = new
    }

    fn new_frame(&mut self, chunk: &'c Chunk, ret_dst: usize, reg_offset: usize) {
        let cur_offset = self.frame().reg_offset;
        let frame = CallFrame::new(
            cur_offset + ret_dst,
            cur_offset + reg_offset + 1,
            ChunkReader::new(chunk),
        );
        self.frames.push(frame);
    }

    pub fn init_main(&mut self, chunk: &'c Chunk) {
        self.frames
            .push(CallFrame::new(0, 0, ChunkReader::new(chunk)));
    }

    /// Returns `true` if we have exited the top-level frame, hence exiting the whole program.
    fn exit_frame(&mut self) -> bool {
        assert!(!self.frames.is_empty());
        self.frames.pop();
        self.frames.is_empty()
    }
}

impl<'c> VM<'c> {
    fn frame(&self) -> &CallFrame<'_> {
        self.frames.frame()
    }

    pub(super) fn enter_frame(&mut self, chunk: &'c Chunk, ret_dst: usize, reg_offset: usize) {
        self.frames.new_frame(chunk, ret_dst, reg_offset);
    }

    /// Returns `Some(return_value)` if we have exited the top-level frame, hence exiting the whole program.
    /// Remember that the ret dst in the call frame is *absolute*.
    pub(super) fn exit_frame(&mut self, res_reg: usize) -> Option<Value> {
        let dst = self.frame().ret_dst;
        let ret_val = self.stack()[res_reg].clone();
        if self.frames.exit_frame() {
            Some(ret_val)
        } else {
            self.frames.stack[dst] = ret_val;
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

    pub(super) fn stack_index(&self, idx: usize) -> &Value {
        let offset = self.frames.frames.last().unwrap().reg_offset;
        &self.frames.stack[offset + idx]
    }

    pub(super) fn read(&mut self) -> u8 {
        self.frames.read()
    }

    pub(super) fn read_i16(&mut self) -> i16 {
        self.frames.read_i16()
    }

    pub(super) fn read_wide(&mut self) -> u16 {
        self.frames.read_wide()
    }

    pub(super) fn ip(&self) -> usize {
        self.frames.ip()
    }

    pub(super) fn set_ip(&mut self, new: usize) {
        self.frames.set_ip(new);
    }

    pub(super) fn chunk(&self) -> &Chunk {
        self.frames.frames.last().unwrap().reader.chunk
    }
}

/// The `ret_dst` and `reg_offset` in this struct is _absolute_.
#[derive(Debug)]
struct CallFrame<'c> {
    ret_dst: usize,
    reg_offset: usize,
    reader: ChunkReader<'c>,
}

impl<'c> CallFrame<'c> {
    fn new(ret_dst: usize, reg_offset: usize, reader: ChunkReader<'c>) -> Self {
        Self {
            ret_dst,
            reg_offset,
            reader,
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
