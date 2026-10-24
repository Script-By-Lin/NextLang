# NextLang Native Compiler Backend & Executable Generation (Phase 7)

## 1. Architectural Overview & Design Philosophy

The NextLang native compiler backend translates semantically-verified NextLang Abstract Syntax Trees (AST) into standalone, statically-linked ELF x86_64 executables on host Linux systems. 

The pipeline avoids interpreter overhead by transforming source code into native machine instructions via a multi-tiered architecture:

```
+-----------------------------------------------------------------------------------+
|                                  NextLang Source                                  |
|                                     (*.nx)                                        |
+-----------------------------------------------------------------------------------+
                                          |
                                          v  [nx-lexer & nx-parser]
+-----------------------------------------------------------------------------------+
|                         Abstract Syntax Tree (nx-ast)                             |
+-----------------------------------------------------------------------------------+
                                          |
                                          v  [nx-analyzer]
+-----------------------------------------------------------------------------------+
|                      Semantic Validation & Type Checking                          |
+-----------------------------------------------------------------------------------+
                                          |
                                          v  [nx-nir lowering]
+-----------------------------------------------------------------------------------+
|                   NextLang Intermediate Representation (NIR)                     |
|                   Control Flow Graph, Typed SSA-like Operands                     |
+-----------------------------------------------------------------------------------+
                                          |
                                          v  [nx-nir optimizations: -O0 .. -O3]
+-----------------------------------------------------------------------------------+
|               Optimized NIR (Constant Folding, Dead Code Elim)                    |
+-----------------------------------------------------------------------------------+
                                          |
                                          v  [nx-codegen via Cranelift 0.137.0]
+-----------------------------------------------------------------------------------+
|                        Relocatable ELF Object (*.o)                               |
+-----------------------------------------------------------------------------------+
                                          |
                                          v  [nx-native-runtime & System Linker gcc]
+-----------------------------------------------------------------------------------+
|                     Standalone ELF x86_64 Executable Binary                       |
+-----------------------------------------------------------------------------------+
```

---

## 2. Backend Selection Rationale: Cranelift vs LLVM vs C-Transpilation

Three primary strategies were evaluated for the NextLang native code generation backend:

| Criterion | Cranelift (Selected) | LLVM Direct | C-Transpilation |
| :--- | :--- | :--- | :--- |
| **Toolchain Dependencies** | **100% Pure Rust** (`cranelift-codegen` crate) | Requires `libllvm-dev`, C++ build tools, matching LLVM versions | Requires external C compiler (`gcc`/`clang`) for all codegen steps |
| **Compilation Speed** | **Ultra-fast** (<150 ms per module) | Heavy compilation overhead (multi-second JIT/codegen setup) | Moderately slow (I/O disk write + C compiler invocation) |
| **Binary Generation** | Direct ELF object file emission | Direct ELF object file emission | Intermediate `.c` file |
| **Portability** | Pure Cargo dependency; works on any Linux host | Fragile C++ linking and sysroot requirements | Sensitive to C compiler dialect and C standard compliance |
| **Code Quality** | Optimized native machine code (within 2-5x of LLVM) | State-of-the-art vectorization and loop optimizations | Dependent on external C compiler optimization flags |

**Conclusion**: Cranelift (`cranelift-codegen` v0.137.0) was chosen as the primary backend strategy. It guarantees that any user with a standard Rust and GCC toolchain can compile NextLang programs directly into native executables without managing gigabytes of external LLVM development headers or encountering LLVM version mismatches.

---

## 3. Type System & ABI Mapping

NextLang primitive and user-defined types map to NextLang Intermediate Representation (NIR), Cranelift IR types, and standard C ABI calling conventions:

| NextLang Type | NIR Type | Cranelift Type | C ABI Equivalent | Size / Representation |
| :--- | :--- | :--- | :--- | :--- |
| `<INT8>` | `NirType::I8` | `types::I8` | `int8_t` | 1 byte signed integer |
| `<INT16>` | `NirType::I16` | `types::I16` | `int16_t` | 2 bytes signed integer |
| `<INT32>` | `NirType::I32` | `types::I32` | `int32_t` | 4 bytes signed integer |
| `<INT64>` | `NirType::I64` | `types::I64` | `int64_t` | 8 bytes signed integer |
| `<INT128>` | `NirType::I128` | `types::I128` | `__int128_t` | 16 bytes signed integer |
| `<UINT8>` | `NirType::U8` | `types::I8` | `uint8_t` | 1 byte unsigned integer |
| `<UINT16>` | `NirType::U16` | `types::I16` | `uint16_t` | 2 bytes unsigned integer |
| `<UINT32>` | `NirType::U32` | `types::I32` | `uint32_t` | 4 bytes unsigned integer |
| `<UINT64>` | `NirType::U64` | `types::I64` | `uint64_t` | 8 bytes unsigned integer |
| `<UINT128>` | `NirType::U128` | `types::I128` | `unsigned __int128` | 16 bytes unsigned integer |
| `<FL32>` | `NirType::F32` | `types::F32` | `float` | 4 bytes IEEE 754 float |
| `<FL64>` | `NirType::F64` | `types::F64` | `double` | 8 bytes IEEE 754 float |
| `<BOOL>` | `NirType::Bool` | `types::I8` | `int8_t` / `bool` | 1 byte (`0` = false, `1` = true) |
| `<CHAR>` | `NirType::Char` | `types::I32` | `int32_t` | 4 bytes UTF-32 unicode scalar |
| `<STRING>` | `NirType::Ptr` | `types::I64` | `const char*` | 8 bytes null-terminated UTF-8 pointer |
| `<HEAP<T>>` | `NirType::Ptr` | `types::I64` | `void*` | 8 bytes heap allocation pointer |
| `Class / Object` | `NirType::Ptr` | `types::I64` | `void*` | 8 bytes pointer to allocated struct |
| `<VOID>` | `NirType::Void` | None (no return) | `void` | 0 bytes |

---

## 4. Memory Model & Stack / Heap Layout

### 4.1 Stack Frame Layout
- Local variables and temporaries allocated via `NirInst::Alloca` correspond to Cranelift explicit sized stack slots (`create_sized_stack_slot`).
- Pointer addresses to stack slots are computed using `builder.ins().stack_addr(types::I64, slot, offset)`.
- Stack loads and stores use `stack_load` and `stack_store` with 64-bit address width and explicit type sizes.

### 4.2 Heap Allocation Layout
- Class instances and explicit dynamic memory (`alloc(...)`) are allocated on the heap via `nx_rt_alloc(size)`.
- Heap memory is zero-initialized on allocation to ensure deterministic execution and prevent uninitialized memory reads.
- Deallocation is performed explicitly via `nx_rt_free(ptr)`.

### 4.3 Object Representation & Field Layout
- Class layouts (`NirClassLayout`) store instance fields in sequential declaration order, beginning with parent fields in single-inheritance hierarchies.
- Field access computes the base pointer plus the constant byte offset (`NirInst::GetFieldPtr`), then loads or stores through `MemFlagsData::new()` without trapping.
- Class constructors allocate `max(total_size_bytes, 8)` bytes, initialize `self`, execute constructor statements, and return the `self` pointer.

---

## 5. Native Runtime C ABI (`nx-native-runtime`)

The native runtime library is implemented in C with standard System V AMD64 ABI linkage (`runtime.c` and `runtime.h`):

| Function | C Signature | Description |
| :--- | :--- | :--- |
| `nx_rt_vout_str` | `void nx_rt_vout_str(const char *str)` | Prints null-terminated string to stdout |
| `nx_rt_vout_int` | `void nx_rt_vout_int(int64_t val)` | Formats and prints integer to stdout |
| `nx_rt_vout_float` | `void nx_rt_vout_float(double val)` | Formats and prints double to stdout |
| `nx_rt_vout_bool` | `void nx_rt_vout_bool(int8_t val)` | Prints `true` or `false` to stdout |
| `nx_rt_vout_char` | `void nx_rt_vout_char(int32_t ch)` | Encodes UTF-32 scalar to UTF-8 and prints |
| `nx_rt_vout_space` | `void nx_rt_vout_space(void)` | Emits separator space (`' '`) |
| `nx_rt_vout_newline` | `void nx_rt_vout_newline(void)` | Emits newline (`'\n'`) and flushes stdout |
| `nx_rt_input` | `const char* nx_rt_input(const char *prompt)` | Displays prompt, reads stdin line, strips newline |
| `nx_rt_to_int` | `int64_t nx_rt_to_int(const char *str)` | Parses string as 64-bit integer, panics on failure |
| `nx_rt_to_float` | `double nx_rt_to_float(const char *str)` | Parses string as 64-bit float, panics on failure |
| `nx_rt_len_str` | `int64_t nx_rt_len_str(const char *str)` | Computes string byte length via `strlen` |
| `nx_rt_alloc` | `void* nx_rt_alloc(int64_t size)` | Allocates zero-initialized heap memory |
| `nx_rt_free` | `void nx_rt_free(void *ptr)` | Releases heap memory |
| `nx_rt_str_concat` | `const char* nx_rt_str_concat(const char *a, const char *b)` | Concatenates two strings into newly allocated buffer |
| `nx_rt_str_eq` | `int8_t nx_rt_str_eq(const char *a, const char *b)` | Compares two strings for content equality (`strcmp`) |
| `nx_rt_panic` | `void nx_rt_panic(const char *msg)` | Prints runtime error message to stderr and exits with 1 |

---

## 6. CLI Command Reference

### `nx build`
Compiles a NextLang source file into a native ELF executable binary.
```bash
nx build <file.nx> [-o <output>] [--opt-level <0|1|2|3>] [--dump-nir] [--time]
```
- `-o, --output <file>`: Sets destination executable path (defaults to source filename without `.nx`).
- `--opt-level <0|1|2|3>`: Sets optimization level (default: 0).
- `--dump-nir`: Pretty-prints the lowered and optimized NIR to stdout.
- `--time`: Measures compilation duration and prints to stderr.

### `nx run-native`
Compiles (if source file provided) and executes native binary with optional argument passing.
```bash
nx run-native <file.nx|binary> [--time]
```
- Accepts either a NextLang `.nx` file (compiles to a temporary binary and runs) or an existing precompiled native binary.
- `--time`: Measures child process execution duration and prints to stderr.

### `nx dump-nir`
Lower NextLang AST into typed NIR and displays textual IR representation without compiling to native binary.
```bash
nx dump-nir <file.nx> [--opt-level <0|1|2|3>]
```

---

## 7. Performance Benchmarks

Workload: **Collatz Sequence (1..50,000)**  
Total arithmetic iterations: **5,025,114 steps**  
Host Environment: **Linux x86_64, Fedora release, Intel/AMD Host CPU**

| Execution Engine | Min Time | Median Time | Max Time | Speedup vs Interpreter |
| :--- | :--- | :--- | :--- | :--- |
| **NextLang AST Interpreter** | 10,274.93 ms | 10,372.82 ms | 10,383.01 ms | 1.0x (baseline) |
| **NextLang Native (`-O3`)** | 38.24 ms | 39.08 ms | 39.88 ms | **265.4x faster** |
| **Native Rust (`cargo --release`)** | 6.64 ms | 6.99 ms | 7.61 ms | **1,483.9x faster** |

### Benchmark Analysis
1. **Dramatic Speedup**: The native compiler backend delivers a **265x reduction in wall-clock execution time** compared to AST interpretation by eliminating AST tree walking, environment hashmap lookups, and heap-allocated `Value` enums.
2. **Low Overhead**: NextLang native binary performance is within ~5x of idiomatic LLVM-optimized Rust, despite Cranelift prioritizing rapid compilation speed over deep loop vectorization.
3. **Compilation Throughput**: `nx build` compiles the full benchmark in under 80 ms, making the edit-compile-test cycle virtually instantaneous.

---

## 8. Limitations & Scope Constraints

- **Platform Target**: Host Linux x86_64 (`x86_64-unknown-linux-gnu`). Cross-compilation to Windows or macOS is deferred.
- **Garbage Collection**: NextLang Phase 7 uses explicit manual heap management (`alloc`/`free`) and copy semantics for primitives. Automatic tracing garbage collection is planned for Phase 8.
- **Advanced Dynamic Reflection**: Type introspection and dynamic class loading are not implemented in the native code generator.
- **Phase Boundary**: Phase 7 is fully completed. Phase 8 will not begin without explicit user instruction.
