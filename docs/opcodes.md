# Sigil VM Opcodes

## Instruction Format

Variable-length instruction encoding stored as a `Vec<u8>`.

| Operand    | Width | Meaning                   |
| ---------- | ----- | ------------------------- |
| `op`       | `u8`  | Opcode                    |
| `reg`      | `u8`  | Register index (0–255)    |
| `imm`      | `u8`  | Immediate byte (bool, count) |
| `const`    | `u16` | Constant pool index (0–65535) |
| `offset`   | `i16` | Signed jump offset in bytes |

All multi-byte values are little-endian.

---

## Data Movement

### MOVE `0x00`
| op | dst | src |
|----|--------|--------|

`stack[dst] = stack[src]`

---

### LOADK `0x01`
| op | dst | wide const |
|----|--------|-----------|

`stack[dst] = constants[const]`

---

### LOADBOOL `0x02`
| op | dst | val |
|----|--------|--------|

`stack[dst] = val != 0`

---

### LOADNIL `0x03`
| op | dst |
|----|--------|

`stack[dst] = nil`

---

## Calls

### CALL `0x04`
| op | dst | wide name | argc | arg0 | ... | arg{N} |
|----|--------|----------|---------|---------|-----|-------------|

Named function call. Looks up `constants[name]` in the function registry, dispatches to the best-matching overload based on runtime argument types. Arguments are specified as individual register indices (non-contiguous). Result written to `stack[dst]`.

```
stack[dst] = dispatch(constants[name], [stack[arg0], stack[arg1], ...])
```

### CALLC `0x05`
| op | dst | func | argc | arg0 | ... | arg{N} |
|----|--------|---------|---------|---------|-----|-------------|

Register-based call. Calls the function value stored in `stack[func]` with individually-specified register arguments. Result written to `stack[dst]`.

```
stack[dst] = stack[func]([stack[arg0], stack[arg1], ...])
```

---

## Control Flow

### RETURN `0x06`
| op | first | count |
|----|-----------|----------|

Returns `count` values starting from `stack[first]`. In top-level code, halts the VM and returns the first value. `count == 0` returns `nil`.

```
return stack[first .. first + count]
```

---

### JMP `0x07`
| op | wide offset |
|----|------------|

Unconditional jump. Adds `offset` to the instruction pointer (at the `JMP` opcode).

```
ip += offset
```

---

### TEST `0x08`
| op | reg | wide offset |
|----|--------|------------|

Conditional jump. If `stack[reg]` is falsey, adds `offset` to IP (the ip of the `TEST` opcode); otherwise falls through.

```
if !stack[reg].is_truthy() { ip += offset }
```

---

## Structures (unimplemented)

### CLOSURE `0x09`
| op | dst | wide proto |
|----|--------|-----------|

Creates a closure from the function prototype at `constants[proto]`, captures upvalues, stores in `stack[dst]`.

### NEWSTRUCT `0x0A`
| op | dst |
|----|--------|

Allocates a new empty struct, stores in `stack[dst]`.

---

## Truthiness

| Value     | Truthy? |
| --------- | ------- |
| `nil`     | false   |
| `Bool(b)` | `b`     |
| `Number(n)` | `n != 0.0` |
| `String(s)` | `!s.is_empty()` |
