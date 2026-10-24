# NextLang

NextLang is an explicit, flexible, and predictable general-purpose programming language designed to be easy to learn, secure by design, and faster than Python where appropriate. NextLang is implemented primarily in Rust and avoids `eval()` / `exec()`-style dynamic code execution.

This repository contains the **Phase 8: Systems Programming, Low-Level Control & LLVM Native Runtime Engineering (`nx build`, `nx run-native`, `nx dump-llvm`, `nx dump-nir`)** implementation. NextLang employs an **LLVM compilation pipeline** (`nx-codegen` lowering NIR to LLVM IR, verified with `opt` and compiled with `clang -O0..-O3`), complete with native C FFI, `unsafe` blocks with Gate 8 diagnostic safety, low-level integer and pointer types (`USIZE`, `ISIZE`, `UINT8..128`, `INT8..128`, `PTR`), bitwise operators, native structs, and high-performance system primitives for byte buffers, file I/O, and OS environment.

---

## Workspace Architecture

NextLang is structured as a modular Cargo workspace:

```text
NextLang/
├── Cargo.toml
├── crates/
│   ├── nx-diagnostics/    # Span tracking, source files, and rustc-style diagnostics
│   ├── nx-ast/            # Abstract syntax tree (AST), struct decls, unsafe blocks, extern blocks, bitwise AST
│   ├── nx-lexer/          # Lexical analyzer with keywords: unsafe, struct, extern, and operators ^, ~, <<, >>
│   ├── nx-parser/         # Pratt expression parser, recursive descent for structs, unsafe, extern, and bitwise ops
│   ├── nx-analyzer/       # Semantic analysis, Gate 8 NX-SYS-001 unsafe validation, low-level typechecking
│   ├── nx-runtime/        # Tree-walking execution engine, generation-counted heap, OOP, standard I/O
│   ├── nx-js-backend/     # JavaScript compiler backend targeting Node.js and browser environments
│   ├── nx-nir/            # High-level typed NextLang Intermediate Representation (CFG, bitwise ops, struct layouts)
│   ├── nx-native-runtime/ # Embedded C ABI runtime library providing file I/O, byte buffers, environment, memory
│   ├── nx-codegen/        # LLVM native compilation backend producing optimized standalone ELF x86_64 binaries
│   └── nx-cli/            # The `nx` command-line interface with LLVM native build, dump-llvm, and timing
├── examples/
│   ├── hello.nx           # Canonical NextLang program
│   ├── variables.nx       # Variables (var, const, tem), primitive, generic & union types
│   ├── conditions.nx      # if / otherwise / else branching and logical operators
│   ├── loops.nx           # As (while), fr<...> (for), and Do ... As (do-while) loops
│   ├── oop.nx             # Classes, interfaces, inheritance, constructors, super, and static members
│   ├── match_case.nx      # Pattern matching with literals, alternatives, guards, and wildcards
│   ├── ownership.nx       # Move vs copy, borrowing (&, &mut), tem escape, alloc/free heap management
│   ├── systems_bitwise.nx # Bitwise operations: &, |, ^, ~, <<, >>
│   ├── systems_struct.nx  # Native structs and struct literal instantiation
│   ├── systems_ffi.nx     # Native C FFI (extern "C") and unsafe blocks
│   └── systems_io.nx      # Standard library systems primitives (files, buffers, env, process)
├── benchmarks/
│   ├── collatz.nx         # NextLang computational Collatz sequence benchmark
│   └── rust_collatz.rs    # Reference native Rust benchmark implementation
└── docs/
    ├── syntax.md          # Language syntax specification, error gates, and CLI reference
    ├── memory-model.md    # Ownership, lifetimes, borrowing, generation-counted heap & target safety
    ├── javascript-backend.md # JS compiler pipeline, construct mappings, runtime helpers, & targets
    ├── native-backend.md  # Native architecture, C ABI, benchmarks, and CLI guide
    └── systems-programming.md # Phase 8 systems programming, LLVM native compile pipeline, and FFI
```

---

## Installation & CLI Usage

### Build the CLI

```bash
cargo build --release
```

The compiled binary will be located at `target/release/nx`.

### CLI Commands

```bash
# Display CLI version
nx --version

# Display CLI help
nx --help

# Build a standalone native ELF x86_64 executable
nx build examples/hello.nx -o hello --opt-level 3 --time

# Compile and run a NextLang source file natively
nx run-native examples/hello.nx --time

# Dump NextLang Intermediate Representation (NIR) control flow graph
nx dump-nir examples/hello.nx --opt-level 2

# Tokenize a NextLang source file and display token stream with spans
nx tokenize examples/match_case.nx

# Parse a NextLang source file and display the formatted AST
nx parse examples/match_case.nx

# Perform static lexical, syntactic, semantic, OOP, and ownership validation
nx check examples/ownership.nx

# Run a NextLang program through the validation error gates and execution engine
nx run examples/match_case.nx

# Run a program and output high-precision execution timing benchmarks to stderr
nx run examples/oop.nx --time

# Compile a NextLang program to JavaScript
nx compile-js examples/hello.nx -o dist/hello.js
```

---

## Syntax Overview

### Statements
Statements must terminate with a semicolon `;`:
```nextlang
vout("Hello, World!");
var age:<INT32> = 20;
age = 21;
```

### Variables
NextLang strictly forbids implicit/undeclared variables. Variables are declared with `var`, `const`, or `tem`:
```nextlang
var age:<INT32> = 25;       # Mutable variable
const PI:<FL64> = 3.14159;  # Immutable constant
tem i:<INT32> = 0;          # Temporary local variable
```

### Type Annotations
Types follow the variable name, enclosed in `< >`:
- **Primitives**: `INT8`, `INT16`, `INT32`, `INT64`, `INT128`, `UINT8`, `UINT16`, `UINT32`, `UINT64`, `UINT128`, `FL32`, `FL64`, `FL128`, `BOOL`, `CHAR`, `STRING`, `ANY`
- **Collections**: `LIST<INT32>`, `SET<INT32>`, `MAP<STRING, INT32>`, `LIST<ANY>`
- **Unions**: `INT32 | FL64`, `LIST<INT32 | FL64>`
- **Nominal Class & Interface Types**: `Rectangle`, `Shape`, `Printable`

### Standard Output
Output to the terminal uses `vout()`:
```nextlang
vout("Hello", name);
```

### Conditionals: `if`, `otherwise`, `else`
```nextlang
if age >= 18 & active {
    vout("Adult and active");
} otherwise age < 18 | not active {
    vout("Minor or inactive");
} else {
    vout("Other");
}
```

### Loops
- **While Loop (`As`)**:
  ```nextlang
  As i < 5 {
      vout(i);
      i++;
  }
  ```
- **For Loop (`fr<init : cond : update>`)**:
  ```nextlang
  fr<tem j:<INT32> = 0: j < 5: j++> {
      vout(j);
  }
  ```
- **Do-While Loop (`Do ... As`)**:
  ```nextlang
  Do {
      i--;
  } As i > 0;
  ```

### Object-Oriented Programming (OOP)
NextLang features a complete, statically verified OOP model with classes, interfaces, single inheritance, access modifiers, and static members:

```nextlang
interface Describable {
    fn describe():<STRING>;
}

abstract class Shape implements Describable {
    protected var name:<STRING> = "Shape";

    public fn init(shape_name:<STRING>):<VOID> {
        self.name = shape_name;
        return;
    }

    public abstract fn area():<FL64>;

    public fn describe():<STRING> {
        return self.name;
    }
}

class Rectangle extends Shape {
    protected var width:<FL64> = 0.0;
    protected var height:<FL64> = 0.0;

    public fn init(w:<FL64>, h:<FL64>):<VOID> {
        self.name = "Rectangle";
        self.width = w;
        self.height = h;
        return;
    }

    public fn area():<FL64> {
        return self.width * self.height;
    }
}

class Square extends Rectangle {
    public fn init(side:<FL64>):<VOID> {
        self.name = "Square";
        self.width = side;
        self.height = side;
        return;
    }

    public fn describe():<STRING> {
        return super.describe() + " (square)";
    }
}
```

- **Constructors**: Declared using `public fn init(...):<VOID>`. Instantiation via `ClassName(...)`.
- **Access Modifiers**: `public` (default), `private` (enclosing class only), and `protected` (class and subclasses).
- **Subtyping**: Derived class instances can be assigned directly to parent class or implemented interface types.
- **Static Members**: Declared with `static` and accessed via `ClassName.member`.

### Pattern Matching (`match ... case`)
NextLang supports exhaustive pattern matching with literal values, alternatives (`|`), variable bindings, guards (`if`), and wildcards (`_`):

```nextlang
match status {
    case 200 {
        vout("OK");
    }
    case 201 | 202 {
        vout("Accepted");
    }
    case code if code >= 400 & code < 500 {
        vout("Client Error: ", code);
    }
    case _ {
        vout("Other status");
    }
}
```

### Ownership, Borrowing & Manual Memory Management
NextLang achieves memory safety without a tracing GC through move semantics, borrow checking, and generational heap handles:

```nextlang
# Explicit heap allocation
var buffer:<HEAP<STRING>> = alloc("Managed Memory");

# Borrowing with &T (immutable) and &mut T (mutable)
var r:<&HEAP<STRING>> = &buffer;
vout(r);

# Explicit deallocation
free(buffer);
```

- **Move Semantics**: Assigning non-copy values moves ownership. Accessing moved values produces `NX-OWN-001`.
- **Borrow Checking**: Prevents mutation while borrowed (`NX-BORROW-001`) and moves while borrowed (`NX-BORROW-002`).
- **Scoped Temporaries (`tem`)**: Variables guaranteed not to escape function boundaries (`NX-OWN-003`).
- **Generational Heap**: Tracks deallocations to prevent double-frees (`NX-OWN-002`) and dangling references at runtime.

---

## Testing & Quality Assurance

Run the test suite across all workspace crates:

```bash
RUSTFLAGS="-D warnings" cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

---

## Automated Error-Gate Architecture

NextLang incorporates a staged error-gate architecture that guarantees code validity before execution:
- **Gate 1 (Lexical)**: Byte-precise span tracking, UTF-8 validation, and unclosed literal error handling.
- **Gate 2 (Syntactic)**: Statement termination (`;`), matching delimiters, and canonical NextLang grammar verification.
- **Gate 3 (Scope & Declarations)**: Enforced declaration before use, constant reassignment prevention, and scope hierarchy.
- **Gate 4 (Semantic Type System)**: Hybrid type checking, function arity/types, return paths, and static error gates.
- **Gate 5 (OOP System Gates `NX-OOP-001` - `NX-OOP-017`)**: Inheritance cycle prevention, abstract/interface instantiation protection, visibility enforcement, signature override compatibility, and contract adherence.
- **Gate 6 (Ownership & Borrowing `NX-OWN-001..003`, `NX-BORROW-001..003`, `NX-MATCH-001..004`)**: Statically validates move semantics, double-frees, borrow exclusivity, temporary variable containment, and pattern consistency.
- **Gate 7 (Runtime Safety & Generational Heap)**: Runtime error handling, generational slot checking, and precision execution timing.

---

## Multi-Phase Roadmap

- [x] **Phase 1: Language Foundation**: Lexer, AST, parser, diagnostics, CLI (`tokenize`, `parse`, `check`).
- [x] **Phase 2: Semantic Analysis & Type System**: Name resolution, lexical scopes, hybrid type checking, type inference, and assignment validation.
- [x] **Phase 3: Complete Object-Oriented Programming (OOP) System**: Classes, interfaces, single inheritance (`extends`), interface implementation (`implements`), access modifiers (`public`, `private`, `protected`), constructors (`init`), `self`/`super`, static members, abstract and final modifiers, and 17 static OOP error gates.
- [x] **Phase 4: Execution Engine**: Interpreter / Runtime evaluating NextLang AST programs, control flow, functions, OOP, and built-in standard I/O (`nx run <file.nx> [--time]`).
- [x] **Phase 5: Memory Model & Ownership**: Manual memory management with clear allocation, deallocation, ownership, borrow checking, scoped temporaries, match-case pattern matching, and runtime timing statistics.
- [x] **Phase 6: JavaScript Target**: Frontend web development compilation target and Node.js/browser interoperability (`nx compile-js <file.nx> [-o <out.js>]`).
- [x] **Phase 7: Native Compilation (NIR)**: Typed NextLang Intermediate Representation (NIR), Cranelift native code generator (nx-codegen), C ABI runtime, ELF x86_64 linking, and 265x execution speedup.
- [ ] **Phase 8: Backend Standard Library**: I/O, server primitives, async runtime, networking.
- [ ] **Phase 9: System Programming**: Low-level abstractions, pointer operations, OS interfaces.
- [ ] **Phase 10: Tooling & Ecosystem**: Build caching, LSP, debugger, formatter, package manager.


