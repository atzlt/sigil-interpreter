# Sigil VM Opcodes

## Instruction Format

Variable-length encoding. All multi-byte values are little-endian.

| Operand | Width | Meaning |
|---------|-------|---------|
| `op`    | u8    | Opcode |
| `reg`   | u8    | Register index (0–255) |
| `imm`   | u8    | Immediate byte (bool, argc) |
| `const` | u16   | Constant pool index |
| `slot`  | u16   | Global variable slot index |
| `offset`| i16   | Signed jump offset in bytes |

---

## Opcode Table

| Opcode     | Byte | Encoding | Status |
|------------|------|----------|--------|
| MOVE       | 0x00 | `[ op ][ dst ][ src ]` | ✅ |
| LOADK      | 0x01 | `[ op ][ dst ][  wide const  ]` | ✅ |
| LOADBOOL   | 0x02 | `[ op ][ dst ][ val ]` | ✅ |
| LOADNIL    | 0x03 | `[ op ][ dst ]` | ✅ |
| GETGLB     | 0x04 | `[ op ][ dst ][  wide slot  ]` | ✅ |
| SETGLB     | 0x05 | `[ op ][  wide slot  ][ src ]` | ✅ |
| CALL       | 0x06 | `[ op ][ dst ][  wide fn  ][ argc ][ arg0 ]...[ argN ]` | ✅ |
| CALLC      | 0x07 | `[ op ][ dst ][ func ][ argc ][ arg0 ]...[ argN ]` | ❌ |
| RETURN     | 0x08 | `[ op ][ reg ]` | ✅ |
| JMP        | 0x09 | `[ op ][  wide offset  ]` | ✅ |
| TEST       | 0x0A | `[ op ][ reg ][  wide offset  ]` | ✅ |
| CLOSURE    | 0x0B | `[ op ][ dst ][  wide proto  ]` | ❌ |
| NEWSTRUCT  | 0x0C | `[ op ][ dst ]` | ❌ |

---

## Data Movement

### MOVE `0x00`
```
[ op ][ dst ][ src ]
```
`stack[dst] = stack[src]`

### LOADK `0x01`
```
[ op ][ dst ][  wide const  ]
```
`stack[dst] = constants[const]`

### LOADBOOL `0x02`
```
[ op ][ dst ][ val ]
```
`stack[dst] = val != 0`

### LOADNIL `0x03`
```
[ op ][ dst ]
```
`stack[dst] = nil`

---

## Global Variables

### GETGLB `0x04`
```
[ op ][ dst ][  wide slot  ]
```
Loads `globals[slot]` into `stack[dst]`. Uninitialized → nil.

### SETGLB `0x05`
```
[ op ][  wide slot  ][ src ]
```
Stores `stack[src]` into `globals[slot]`. Auto-grows the global vector on first access.

---

## Calls

### CALL `0x06`
```
[ op ][ dst ][  wide fn  ][offset][ argc ][ arg0 ]...[ argN ]
```
Looks up `constants[fn]` as `Value::Fn(FnId)` in the function registry. Set a new call frame starting at `offset + 1` of the current frame. Result → `stack[dst]`.

---

## Control Flow

### RETURN `0x08`
```
[ op ][ reg ]
```
Halts and returns `stack[reg]`.

### JMP `0x09`
```
[ op ][  wide offset  ]
```
Unconditional jump: `ip += offset` (relative to the JMP opcode byte).

### TEST `0x0A`
```
[ op ][ reg ][  wide offset  ]
```
Conditional jump: if `!stack[reg].is_truthy()` then `ip += offset`.

---

## Structures *(unimplemented)*

### CLOSURE `0x0B`
```
[ op ][ dst ][  wide proto  ]
```
Creates closure from `constants[proto]` → `stack[dst]`.

### NEWSTRUCT `0x0C`
```
[ op ][ dst ]
```
Allocates empty struct → `stack[dst]`.

---

## Truthiness

| Value | Truthy? |
|-------|---------|
| `nil`   | `false` |
| `Bool(b)` | `b` |
| `Number(n)` | `n != 0.0` |
| `String(s)` | `!s.is_empty()` |
| `Fn(_)` | `true` |
