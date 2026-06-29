use crate::value::Value;

#[derive(Debug, Clone)]
pub enum Upvalue {
    /// Value is still live at `stack[absolute_index]`.
    Open(usize),
    /// Value has been moved off the stack.
    Closed(Value),
}
