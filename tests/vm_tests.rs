use sigil_interpreter::{
    registry::FunctionRegistry,
    value::Value,
    vm::{Chunk, OpCode::*, VM},
};

#[test]
fn test_loadk_and_move() {
    let mut chunk = Chunk::new();

    // LOADK R0, 42
    let k42 = chunk.add_constant(Value::Number(42.0));
    chunk.emit_opcode(LOADK);
    chunk.emit_u8(0); // dst R0
    chunk.emit_u16(k42);

    // LOADK R1, 10
    let k10 = chunk.add_constant(Value::Number(10.0));
    chunk.emit_opcode(LOADK);
    chunk.emit_u8(1); // dst R1
    chunk.emit_u16(k10);

    // MOVE R2, R0
    chunk.emit_opcode(MOVE);
    chunk.emit_u8(2); // dst R2
    chunk.emit_u8(0); // src R0

    // RETURN R2, 1
    chunk.emit_opcode(RETURN);
    chunk.emit_u8(2); // first_reg
    chunk.emit_u8(1); // count

    let registry = FunctionRegistry::new();
    let mut vm = VM::new();
    let result = vm.run(&mut chunk, &registry).unwrap();
    assert_eq!(result, Value::Number(42.0));
}

#[test]
fn test_bool_and_nil() {
    let mut chunk = Chunk::new();

    chunk.emit_opcode(LOADBOOL);
    chunk.emit_u8(0);
    chunk.emit_u8(1);

    chunk.emit_opcode(LOADNIL);
    chunk.emit_u8(1);

    chunk.emit_opcode(RETURN);
    chunk.emit_u8(0);
    chunk.emit_u8(1);

    let registry = FunctionRegistry::new();
    let mut vm = VM::new();
    let result = vm.run(&mut chunk, &registry).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_jmp() {
    let mut chunk = Chunk::new();

    chunk.emit_opcode(JMP);
    chunk.emit_u16(4u16);

    // Skipped:
    let k99 = chunk.add_constant(Value::Number(99.0));
    chunk.emit_opcode(LOADK);
    chunk.emit_u8(0);
    chunk.emit_u16(k99);

    // Runs:
    let k10 = chunk.add_constant(Value::Number(10.0));
    chunk.emit_opcode(LOADK);
    chunk.emit_u8(1);
    chunk.emit_u16(k10);

    chunk.emit_opcode(RETURN);
    chunk.emit_u8(1);
    chunk.emit_u8(1);

    let registry = FunctionRegistry::new();
    let mut vm = VM::new();
    let result = vm.run(&mut chunk, &registry).unwrap();
    assert_eq!(
        result,
        Value::Number(10.0),
        "JMP should skip over LOADK R0, 99"
    );
}

#[test]
fn test_test_true() {
    let mut chunk = Chunk::new();

    chunk.emit_opcode(LOADBOOL);
    chunk.emit_u8(0);
    chunk.emit_u8(1);

    chunk.emit_opcode(TEST);
    chunk.emit_u8(0);
    chunk.emit_u16(4u16);

    let k99 = chunk.add_constant(Value::Number(99.0));
    chunk.emit_opcode(LOADK);
    chunk.emit_u8(1);
    chunk.emit_u16(k99);

    chunk.emit_opcode(RETURN);
    chunk.emit_u8(1);
    chunk.emit_u8(1);

    let registry = FunctionRegistry::new();
    let mut vm = VM::new();
    let result = vm.run(&mut chunk, &registry).unwrap();
    assert_eq!(
        result,
        Value::Number(99.0),
        "TEST with true: should execute LOADK 99"
    );
}

#[test]
fn test_test_false() {
    let mut chunk = Chunk::new();

    chunk.emit_opcode(LOADBOOL);
    chunk.emit_u8(0);
    chunk.emit_u8(0);

    chunk.emit_opcode(TEST);
    chunk.emit_u8(0);
    chunk.emit_u16(4u16);

    // Skipped:
    let k99 = chunk.add_constant(Value::Number(99.0));
    chunk.emit_opcode(LOADK);
    chunk.emit_u8(1);
    chunk.emit_u16(k99);

    // Runs:
    let k10 = chunk.add_constant(Value::Number(10.0));
    chunk.emit_opcode(LOADK);
    chunk.emit_u8(2);
    chunk.emit_u16(k10);

    chunk.emit_opcode(RETURN);
    chunk.emit_u8(2);
    chunk.emit_u8(1);

    let registry = FunctionRegistry::new();
    let mut vm = VM::new();
    let result = vm.run(&mut chunk, &registry).unwrap();
    assert_eq!(
        result,
        Value::Number(10.0),
        "TEST with false: should skip LOADK 99, return 10"
    );
}
