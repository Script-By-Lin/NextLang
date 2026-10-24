# Systems Programming, Low-Level Control & LLVM Native Runtime Engineering

NextLang Phase 8 introduces systems programming capabilities, high-performance low-level memory primitives, native C FFI, and an **LLVM native compilation backend** producing native ELF executables.

---

## 1. LLVM Native Compilation Architecture

NextLang uses **LLVM** as its primary native backend. High-level NextLang code traverses the compilation pipeline:

```
Source (.nx)
    │
    ▼ [Lexer & Parser]
AST (Abstract Syntax Tree)
    │
    ▼ [Semantic Analyzer (Gate 1 - Gate 8)]
Type-Checked AST
    │
    ▼ [NIR Lowering]
Typed NIR (SSA Control-Flow Graph)
    │
    ▼ [NIR Optimizer: -O0 .. -O3]
Optimized NIR
    │
    ▼ [LLVM IR Emitter]
LLVM IR (.ll)
    │
    ▼ [LLVM Verification (`opt -passes=verify`)]
Verified LLVM IR
    │
    ▼ [Clang / LLVM Native Generator & Linker]
Native Executable (ELF x86_64) + Embedded C Runtime (`nx_runtime.o`)
```

### CLI Commands for LLVM

- **Build Native Executable**:
  ```bash
  nx build file.nx -o my_app --opt-level 2
  ```
- **Dump LLVM IR**:
  ```bash
  nx dump-llvm file.nx --opt-level 2
  ```
- **Build with LLVM IR Emission**:
  ```bash
  nx build file.nx -o my_app --emit-llvm
  ```
- **Run Natively**:
  ```bash
  nx run-native file.nx --time
  ```

---

## 2. Low-Level Type System

NextLang supports exact-width integer types and pointer abstractions:

| Type | Bitwidth | Representation |
| :--- | :--- | :--- |
| `INT8` | 8 bits | Signed two's complement integer |
| `INT16` | 16 bits | Signed two's complement integer |
| `INT32` | 32 bits | Signed two's complement integer |
| `INT64` | 64 bits | Signed two's complement integer |
| `INT128` | 128 bits | Signed two's complement integer |
| `UINT8` | 8 bits | Unsigned integer |
| `UINT16` | 16 bits | Unsigned integer |
| `UINT32` | 32 bits | Unsigned integer |
| `UINT64` | 64 bits | Unsigned integer |
| `UINT128` | 128 bits | Unsigned integer |
| `ISIZE` | 64 bits | Target pointer-width signed integer |
| `USIZE` | 64 bits | Target pointer-width unsigned integer |
| `PTR` | 64 bits | Raw untyped memory pointer (`ptr` in LLVM) |

---

## 3. Bitwise Operators

NextLang provides bitwise operations with C-compatible precedence and semantics:

```nextlang
var a:<INT32> = 12; // 0b1100
var b:<INT32> = 5;  // 0b0101

var and_val:<INT32> = a & b;  // Bitwise AND: 4
var or_val:<INT32>  = a | b;  // Bitwise OR:  13
var xor_val:<INT32> = a ^ b;  // Bitwise XOR: 9
var shl_val:<INT32> = a << 2; // Shift Left:  48
var shr_val:<INT32> = a >> 1; // Shift Right: 6
var not_val:<INT32> = ~a;     // Bitwise NOT: -13
```

Hexadecimal (`0x...`) and binary (`0b...`) integer literals are supported directly in the lexer.

---

## 4. Native Structs

Native structs represent contiguous C-compatible memory layouts without virtual method tables:

```nextlang
struct Vector3 {
    x:<INT32>,
    y:<INT32>,
    z:<INT32>
}

var v:<Vector3> = Vector3 { x: 10, y: 20, z: 30 };
vout("Vector:", v.x, v.y, v.z);
```

In the LLVM backend, `struct Vector3` compiles to `%struct.Vector3 = type { i32, i32, i32 }` and fields are accessed via `getelementptr inbounds`.

---

## 5. Native FFI & `unsafe` Blocks

NextLang features native Foreign Function Interface (FFI) bindings for C library integration:

```nextlang
extern "C" {
    fn abs(n:<INT32>):<INT32>;
    fn puts(str:<STRING>):<INT32>;
}

fn main() {
    unsafe {
        var res:<INT32> = abs(-42);
        vout("Result:", res);
        puts("Hello from native C!");
    }
}
```

### Safety Gate 8 (`NX-SYS-001`)

Calling `extern "C"` functions outside an `unsafe { ... }` block triggers safety diagnostic **`NX-SYS-001`**:
```
error[NX-SYS-001]: call to foreign C function 'abs' is unsafe and requires an 'unsafe' block
 --> src/main.nx:8:15
  |
8 | var x:<INT32> = abs(-5);
  |                 ^^^^^^^
  = help: wrap the call in an 'unsafe { ... }' block
```

---

## 6. Standard Systems Runtime Primitives

### File I/O
- `file_open(path:<STRING>, mode:<STRING>):<INT64>`: Opens a file and returns an opaque handle.
- `file_close(handle:<INT64>):<BOOL>`: Closes an open file handle.
- `file_read_all(path:<STRING>):<STRING>`: Reads an entire file into memory as a string.
- `file_write(path:<STRING>, content:<STRING>):<BOOL>`: Writes text content to a file.
- `file_exists(path:<STRING>):<BOOL>`: Checks whether a file exists on disk.
- `file_remove(path:<STRING>):<BOOL>`: Deletes a file.

### OS Environment & Process
- `get_pid():<INT32>`: Returns the current process ID.
- `get_cwd():<STRING>`: Returns the current working directory path.
- `env_get(key:<STRING>):<STRING>`: Queries an environment variable.
- `env_set(key:<STRING>, value:<STRING>):<BOOL>`: Sets or updates an environment variable.

### Byte Buffers & Pointer Operations
- `buffer_new(size:<UINT64>):<PTR>`: Allocates a zero-initialized contiguous byte buffer.
- `buffer_free(buf:<PTR>):<VOID>`: Deallocates a byte buffer.
- `buffer_size(buf:<PTR>):<UINT64>`: Returns the size of the buffer in bytes.
- `buffer_get_u8(buf:<PTR>, offset:<UINT64>):<UINT8>`: Reads a byte at byte offset.
- `buffer_set_u8(buf:<PTR>, offset:<UINT64>, val:<UINT8>):<VOID>`: Writes a byte at offset.
- `buffer_get_u32_le(buf:<PTR>, offset:<UINT64>):<UINT32>`: Reads a 32-bit unsigned little-endian integer.
- `buffer_set_u32_le(buf:<PTR>, offset:<UINT64>, val:<UINT32>):<VOID>`: Writes a 32-bit unsigned integer in little-endian order.
- `ptr_offset(ptr:<PTR>, bytes:<INT64>):<PTR>`: Advances a raw pointer by byte offset.
- `ptr_read_u8(ptr:<PTR>):<UINT8>`: Dereferences a raw pointer to read a byte.
- `ptr_write_u8(ptr:<PTR>, val:<UINT8>):<VOID>`: Writes a byte to raw pointer destination.
