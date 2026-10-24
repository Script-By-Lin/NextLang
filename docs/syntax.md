# NextLang — Phase 1 Syntax & Language Specification

This document describes the finalized syntax for **Phase 1 of NextLang**, implemented in Rust.

> **Note on Scope**: Phase 1 focuses on lexical analysis, parsing, AST representation, diagnostics, and syntax validation. Later execution environments (interpreter, bytecode VM, and native compiler) are designated for future phases.

---

## 1. Source Files & Statements

NextLang source files use the `.nx` extension.

Every executable statement must end with a semicolon `;`:
```nextlang
vout("Hello, World!");
var age:<INT32> = 20;
age = 21;
i++;
return value;
```

Block statements (`if`, `As`, `fr`, `fn`, `{ ... }`) do not require a semicolon after the closing brace `}`:
```nextlang
if condition {
    vout("Yes");
}
```

However, `Do ... As` loops do end with a semicolon after the condition:
```nextlang
Do {
    countdown--;
} As countdown > 0;
```

---

## 2. Comments

Line comments begin with `#` (or `//`) and continue to the end of the line:
```nextlang
# This is a line comment
var x:<INT32> = 10; # Inline comment
```

---

## 3. Variable System

NextLang does not allow implicit or undeclared variables. Every variable must be declared with one of three keywords:

### `var` — Mutable Variable
Can be reassigned to compatible values:
```nextlang
var age:<INT32> = 25;
age = 26;
age += 1;
```

### `const` — Immutable Constant
Cannot be reassigned:
```nextlang
const PI:<FL64> = 3.1415926535;
```
Reassignment to a `const` produces a diagnostic error.

### `tem` — Temporary Variable
Represents a temporary/scoped local variable:
```nextlang
tem count:<INT32> = 0;
count++;
```

---

## 4. Types and Type Annotations

Type annotations follow the variable or parameter name, preceded by `:` and enclosed in `< >`:
```nextlang
var age:<INT32> = 30;
```

### Primitive Types
- **Signed Integers**: `INT8`, `INT16`, `INT32`, `INT64`, `INT128`
- **Unsigned Integers**: `UINT8`, `UINT16`, `UINT32`, `UINT64`, `UINT128`
- **Floating Point**: `FL32`, `FL64`, `FL128`
  - *Note: `FL128` is preserved as a distinct high-precision type in the AST.*
- **Other Primitives**: `BOOL`, `CHAR`, `STRING`, `ANY`

### Generic Collections
Generic type parameters are also specified inside angle brackets `< >`:
```nextlang
var numbers:<LIST<INT32>> = [1, 2, 3];
var ids:<SET<INT32>> = [1, 2, 3];
var users:<MAP<STRING, INT32>> = {
    "Alice": 100,
    "Bob": 200
};
var values:<LIST<ANY>> = [1, "hello", true];
```

Nested generic types:
```nextlang
var matrix:<LIST<LIST<INT32>>> = [
    [1, 2],
    [3, 4]
];
```

### Union Types
Union types are expressed with `|` inside type annotations:
```nextlang
var val:<INT32 | FL64> = 10;
var mixed:<LIST<INT32 | FL64>> = [1, 2.5, 3];
```

---

## 5. Standard I/O & Built-in Functions

### Terminal Output: `vout(...)`
NextLang's standard output function prints values separated by spaces and terminated by a newline:
```nextlang
vout("Hello, NextLang");
vout("Age:", age);
vout(10 + 20);
```

### Interactive Console Input: `input([prompt])`
Reads a single line of text from standard input (stdin). Optionally displays an interactive prompt without a trailing newline:
```nextlang
var name:<STRING> = input("Enter your name: ");
var raw_age:<STRING> = input();
```

### Type Conversions: `to_int(val)` and `to_float(val)`
Explicit numeric parsing functions:
```nextlang
var age:<INT64> = to_int(raw_age);
var pi:<FL64> = to_float("3.14159");
```

### Collection & String Length: `len(val)`
Returns the number of elements in a `LIST`, `SET`, or `MAP`, or character count of a `STRING`:
```nextlang
var count:<INT32> = len([1, 2, 3]); # Returns 3
```

---

## 6. Operators & Expressions

### Arithmetic
`+`, `-`, `*`, `/`, `%`

### Comparison
`==`, `!=`, `<`, `>`, `<=`, `>=`

### Logical Operators
- `|` — Logical OR
- `&` — Logical AND
- `not` — Logical NOT (unary prefix keyword)

Example:
```nextlang
if age >= 18 & active {
    vout("Adult and active");
}

if age < 18 | not active {
    vout("Minor or inactive");
}
```

### Increment and Decrement
`++` and `--` are supported as both statements and postfix/prefix expressions:
```nextlang
i++;
i--;
```

### Assignments
- Basic: `=`
- Compound: `+=`, `-=`, `*=`, `/=`, `%=`

### Range
- `..`

---

## 7. Control Flow

### Conditionals: `if`, `otherwise`, `else`
NextLang uses `if` for the initial condition, `otherwise` for subsequent conditions, and `else` for fallback:
```nextlang
if score >= 90 {
    vout("Grade: A");
} otherwise score >= 80 {
    vout("Grade: B");
} otherwise score >= 70 {
    vout("Grade: C");
} else {
    vout("Grade: F");
}
```

### While Loop: `As`
NextLang intentionally uses `As` for condition-controlled loops (no `while` keyword):
```nextlang
tem i:<INT32> = 0;
As i < 5 {
    vout(i);
    i++;
}
```

### For Loop: `fr<...>`
NextLang uses `fr` with angle brackets containing `initialization : condition : update`:
```nextlang
fr<tem j:<INT32> = 0: j < 10: j++> {
    vout(j);
}
```

### Do-While Loop: `Do ... As`
```nextlang
tem i:<INT32> = 5;
Do {
    vout(i);
    i--;
} As i > 0;
```

---

## 8. Functions

Function declarations use the `fn` keyword with typed parameters and optional return types:
```nextlang
fn add(a:<INT32>, b:<INT32>):<INT32> {
    return a + b;
}

fn greet(name:<STRING>) {
    vout("Hello", name);
}
```

---

---

## 9. Semantic Analysis & Scope Rules

NextLang enforces strict static semantic validation prior to execution:

### Scope & Shadowing Rules
1. **Block Scopes**: Every block `{ ... }`, function body, loop (`fr`, `As`, `Do ... As`), and condition branch (`if`, `otherwise`, `else`) creates a new lexical scope.
2. **Leakage Prevention**: Variables declared inside block scopes are strictly isolated and cannot leak outside their block.
3. **Loop Variables**: Declarations in for-loop headers (`fr<tem i:<INT32> = 0: ...>`) are scoped to the loop header and body only.
4. **Shadowing**: Local declarations in nested scopes can shadow outer declarations, but duplicate declarations within the *same* scope are rejected (`NX-SCOPE-001`).
5. **Forward References for Functions**: Top-level function declarations are hoisted during semantic analysis; functions can be called before or after their definition in source order.

### Declaration Semantics
- `var`: Mutable variable. Can be reassigned to values matching or assignable to its declared type.
- `const`: Immutable binding. Attempted reassignment or mutation triggers error `NX-CONST-001`.
- `tem`: Temporary local variable. Scoped strictly to its enclosing block, indicating a stack-only, short-lived allocation to future optimization passes.

### The `ANY` Type
`ANY` acts as an explicit dynamic escape hatch:
- Any value of any type can be assigned to an `ANY` variable: `var dyn:<ANY> = 123; dyn = "hello";`.
- Reading an `ANY` value into a statically typed target requires explicit conversion/guarding.
- Static type checks allow `ANY` where dynamic values are intentionally modeled.

### Function Return Path Validation
- Functions with non-`VOID` return types (e.g. `fn foo():<INT32>`) must return a value on **all** execution paths (`NX-FUNC-005`).
- Conditional blocks (`if ... otherwise ... else`) satisfy the return requirement only when every branch contains a return statement.
- Return statements outside of function bodies are strictly rejected (`NX-FUNC-004`).

---

---

## 10. Object-Oriented Programming (OOP) System

NextLang provides a complete, class-based object-oriented programming model designed for predictability, safety, and modular abstraction.

### 10.1 Class Declarations
Classes are declared with the `class` keyword and contain fields and methods:
```nextlang
class Person {
    private var name:<STRING> = "";
    private var age:<INT32> = 0;

    public fn init(name:<STRING>, age:<INT32>):<VOID> {
        self.name = name;
        self.age = age;
        return;
    }

    public fn get_name():<STRING> {
        return self.name;
    }
}
```

### 10.2 Interfaces
Interfaces define pure method signatures that conforming classes must implement:
```nextlang
interface Serializable {
    fn serialize():<STRING>;
}
```

### 10.3 Inheritance (`extends`) and Implementation (`implements`)
- NextLang supports **single class inheritance** via `extends` and **multiple interface implementation** via `implements`:
```nextlang
class Employee extends Person implements Serializable {
    private var role:<STRING> = "Engineer";

    public fn init(name:<STRING>, age:<INT32>, role:<STRING>):<VOID> {
        self.name = name;
        self.age = age;
        self.role = role;
        return;
    }

    public fn serialize():<STRING> {
        return self.get_name() + ":" + self.role;
    }
}
```
- Circular inheritance (e.g. `A extends B` and `B extends A`) is statically rejected (`NX-OOP-002`).
- Extending an undeclared type is rejected (`NX-OOP-001`).

### 10.4 Access Modifiers
NextLang supports three visibility levels:
- `public`: Accessible from any code context (default if omitted).
- `private`: Accessible only within the lexical body of the declaring class (`NX-OOP-005`).
- `protected`: Accessible within the declaring class and its subclass hierarchy (`NX-OOP-006`).

### 10.5 Constructors & Instantiation
- Constructors are declared using the explicit `init` method name and must return `<VOID>`.
- Object instantiation uses call syntax on the class name: `var e:<Employee> = Employee("Ada", 36, "Architect");`.
- Invocations are validated against the constructor's parameter types and count (`NX-OOP-014`).
- If no `init` is defined, a default zero-argument constructor `ClassName()` is provided.

### 10.6 `self` and `super`
- Inside instance methods, `self` represents the current instance.
- Inside subclass instance methods, `super` allows invoking overridden parent class methods: `super.describe()`.
- Static methods do not possess `self` or `super` bindings (`NX-RESOLVE-001`).

### 10.7 Static Members
- Declared with the `static` keyword on fields or methods.
- Accessed directly through the class name: `MathUtil.pi` or `MathUtil.sqrt(9.0)`.
- Accessing instance members on class names or static members on instances is rejected (`NX-OOP-013`).

### 10.8 Abstract Classes and Methods
- Declared with `abstract class` and `abstract fn`.
- Abstract methods cannot contain a body and can only be declared inside an `abstract class` (`NX-OOP-017`).
- Abstract classes cannot be directly instantiated (`NX-OOP-003`).
- Concrete subclasses must implement all inherited abstract methods (`NX-OOP-010`).

### 10.9 Final Classes and Methods
- A `final class` cannot be extended by any other class (`NX-OOP-007`).
- A `final fn` method cannot be overridden by subclasses (`NX-OOP-008`).

---

## 11. Static Diagnostic Error Codes

NextLang diagnostics use stable error codes formatted with source line snippets, caret markers, and actionable help:

### General & Type System Error Codes
| Code | Category | Description | Example Trigger |
|---|---|---|---|
| `NX-TYPE-001` | Type Mismatch | Incompatible assignment or initialization | `var a:<INT32> = "text";` |
| `NX-TYPE-002` | Condition Error | Condition expression does not evaluate to `BOOL` | `if 42 { ... }` |
| `NX-TYPE-003` | Literal Overflow | Integer literal exceeds target type range | `var x:<INT8> = 300;` |
| `NX-TYPE-004` | Invalid Operand | Operator applied to incompatible types | `"a" - 2` or `1 \| 2` |
| `NX-TYPE-005` | Collection Type | Element does not match declared collection type | `var l:<LIST<INT32>> = [1, "hi"];` |
| `NX-TYPE-006` | Inference Error | Empty collection literal without type annotation | `var l = [];` |
| `NX-RESOLVE-001` | Name Resolution | Identifier used without prior declaration | `vout(undeclared);` |
| `NX-SCOPE-001` | Scope Error | Duplicate declaration in the same scope | `var x = 1; var x = 2;` |
| `NX-CONST-001` | Immutability | Reassigning or mutating a `const` symbol | `const PI = 3.14; PI = 4;` |
| `NX-FUNC-001` | Function Arity | Call argument count does not match signature | `add(1)` when `add(a, b)` |
| `NX-FUNC-002` | Argument Type | Argument type mismatch in call | `greet(123)` when `greet(name:<STRING>)` |
| `NX-FUNC-003` | Return Type | Return expression does not match return type | `return "no";` in `<INT32>` fn |
| `NX-FUNC-004` | Return Context | Return statement used outside a function | `return 1;` at top-level |
| `NX-FUNC-005` | Return Path | Missing return statement in non-`VOID` function | Fall-through in `<INT32>` fn |
| `NX-FUNC-006` | Parameter Error | Duplicate parameter name in function header | `fn f(a:<INT32>, a:<INT32>)` |
| `NX-SAFETY-001` | Static Safety | Division by constant zero detected statically | `10 / 0` |
| `NX-SAFETY-002` | Static Safety | Indexing known literal out of bounds | `[1, 2][5]` |

### OOP Error Gates (Phase 3)
| Code | Category | Description | Example Trigger |
|---|---|---|---|
| `NX-OOP-001` | Inheritance Error | Unknown parent class in `extends` | `class Dog extends Missing {}` |
| `NX-OOP-002` | Inheritance Error | Circular inheritance detected | `class A extends B {} class B extends A {}` |
| `NX-OOP-003` | Instantiation Error | Cannot instantiate abstract class directly | `var s:<Shape> = Shape();` |
| `NX-OOP-004` | Instantiation Error | Cannot instantiate interface directly | `var p:<Printable> = Printable();` |
| `NX-OOP-005` | Visibility Error | Private member accessed outside declaring class | `obj.private_field` |
| `NX-OOP-006` | Visibility Error | Protected member accessed outside class hierarchy | `obj.protected_field` |
| `NX-OOP-007` | Finality Error | Cannot extend final class | `class Sub extends FinalClass {}` |
| `NX-OOP-008` | Finality Error | Cannot override final method | `public fn final_method():<VOID>` in subclass |
| `NX-OOP-009` | Override Error | Incompatible method override signature | Mismatched parameter or return types |
| `NX-OOP-010` | Contract Error | Unimplemented abstract method in concrete class | Subclass omits abstract method body |
| `NX-OOP-011` | Contract Error | Unimplemented or incompatible interface method | Class fails interface method contract |
| `NX-OOP-012` | Member Error | Member not found on class or instance | `obj.non_existent` |
| `NX-OOP-013` | Member Error | Static/instance access mismatch | `Class.instance_method()` |
| `NX-OOP-014` | Constructor Error| Constructor argument count or type mismatch | `Point(1)` when `init(x, y)` |
| `NX-OOP-015` | Declaration Error| Duplicate class or interface declaration | Two types with identical name |
| `NX-OOP-016` | Member Error | Duplicate field or method in class body | Two fields with same name in class |
| `NX-OOP-017` | Modifier Error | Abstract method declared in non-abstract class | `class C { public abstract fn f(); }` |

### Ownership & Memory Error Gates (Phase 5)
| Code | Category | Description | Example Trigger |
|---|---|---|---|
| `NX-OWN-001` | Ownership Error | Use of moved value | `var b = a; vout(a);` (when `a` is move-type) |
| `NX-OWN-002` | Deallocation Error | Use-after-free or double-free | `free(p); vout(p);` or `free(p); free(p);` |
| `NX-OWN-003` | Escape Error | Temporary `tem` variable escaping its scope | `return tem_var;` |

### Borrow Checking Error Gates (Phase 5)
| Code | Category | Description | Example Trigger |
|---|---|---|---|
| `NX-BORROW-001` | Borrow Conflict | Mutating variable while borrowed | `var r = &x; x = 10;` |
| `NX-BORROW-002` | Borrow Conflict | Moving variable while borrowed | `var r = &x; var y = x;` |
| `NX-BORROW-003` | Borrow Exclusivity | Mutable borrow of immutable value or multiple mutable borrows | `var r = &mut const_val;` |

### Pattern Matching Error Gates (Phase 5)
| Code | Category | Description | Example Trigger |
|---|---|---|---|
| `NX-MATCH-001` | Pattern Type | Pattern type incompatible with match target | `match num { case "str" { ... } }` |
| `NX-MATCH-002` | Pattern Guard | Case guard expression does not evaluate to `BOOL` | `case v if 42 { ... }` |
| `NX-MATCH-003` | Pattern Redundancy | Duplicate or unreachable case pattern | `case 1: ... case 1: ...` |
| `NX-MATCH-004` | Pattern Ordering | Wildcard `_` pattern is not the final case | `case _: ... case 1: ...` |

---

## 12. Pattern Matching: `match ... case`

NextLang provides first-class pattern matching with exhaustiveness and type checking:

```nextlang
match status_code {
    case 200 {
        vout("OK");
    }
    case 201 | 202 {
        vout("Accepted or Created");
    }
    case code if code >= 400 & code < 500 {
        vout("Client Error:", code);
    }
    case _ {
        vout("Other status");
    }
}
```

Features:
- **Literal Patterns**: Integer, float, string, boolean, char literals.
- **Variable Binding**: Identifiers bind the matched value into the case block scope.
- **Alternative Patterns (`|`)**: Matches any of multiple comma/pipe-separated patterns.
- **Pattern Guards (`if <expr>`)**: Boolean expressions that must evaluate to true for the arm to match.
- **Wildcard (`_`)**: Catch-all default pattern (enforced to be the final case by `NX-MATCH-004`).

---

## 13. Ownership, References & Manual Memory Management

NextLang combines zero-overhead manual memory management with static ownership safety:

```nextlang
# Explicit allocation
var buffer:<HEAP<STRING>> = alloc("Managed Buffer");

# Immutable and mutable references
var ref_buf:<&HEAP<STRING>> = &buffer;
vout(ref_buf);

# Explicit deallocation
free(buffer);
```

- **Move Semantics**: Assigning or passing non-primitive types transfers ownership.
- **Borrowing (`&T`, `&mut T`)**: Temporary references checked statically for aliasing and safety.
- **Temporaries (`tem`)**: Lexically-bounded variables that cannot escape function scope.
- **Generation-Counted Heap**: Prevents use-after-free and dangling pointer bugs at runtime.

For complete details, see [docs/memory-model.md](file:///home/hexx/Projects/NextLang/docs/memory-model.md).

---

## 14. Execution Engine & Performance Timing

NextLang programs are executed via the CLI:
```bash
nx run <file.nx> [--time]
```

### Execution Timing
The `--time` flag outputs high-precision monotonic runtime benchmarks to `stderr`:
```
Status 200: OK
Execution time: 80.53 µs
```
- Standard output (`stdout`) contains only user `vout(...)` data.
- Execution metrics (`stderr`) report duration formatted automatically as `ns`, `µs`, `ms`, or `s`.

---

## 15. JavaScript Compiler Backend (`nx compile-js`)

NextLang programs can be compiled to clean ECMAScript 2020+ (ES6+) JavaScript via the `compile-js` CLI command:

```bash
# Compile to <stem>.js in current directory
nx compile-js <file.nx>

# Compile with explicit output path
nx compile-js <file.nx> -o <out.js>

# Emit generated JavaScript directly to stdout
nx compile-js <file.nx> --stdout

# Target web browser environments
nx compile-js <file.nx> --target browser
```

The compiled JavaScript is zero-dependency and executable directly via Node.js (`node <file.js>`) or inside modern web browsers. For detailed specifications on runtime helpers, OOP translation, and ownership-to-GC divergence, see [docs/javascript-backend.md](file:///home/hexx/Projects/NextLang/docs/javascript-backend.md).

---

## 16. Native Compiler Backend (`nx build` & `nx run-native`)

NextLang programs can be compiled directly into standalone, statically-linked ELF x86_64 executables via the Cranelift-powered native code generation backend (`nx-codegen`):

```bash
# Compile to native ELF executable in current directory
nx build <file.nx>

# Compile with custom output executable name and optimization level
nx build <file.nx> -o <output_exe> --opt-level 3 --time

# Compile and run source file directly or execute an existing binary
nx run-native <file.nx> --time

# Dump NextLang Intermediate Representation (NIR) control flow graph
nx dump-nir <file.nx> --opt-level 2
```

### Execution Target Comparison

NextLang supports three distinct execution targets tailored for different use cases:

| Feature / Target | Native Executable (`nx build` / `nx run-native`) | JavaScript Target (`nx compile-js`) | Runtime Engine (`nx run`) |
| :--- | :--- | :--- | :--- |
| **Output** | Standalone ELF x86_64 native binary | Standalone ECMAScript 2020+ `.js` file | Direct in-process execution |
| **Dependencies** | Self-contained, links with system C runtime | Node.js or modern web browser | None (built into `nx` CLI) |
| **Performance** | **Fastest** (265x+ faster than interpreter) | High (V8 JIT compilation) | Development / debug interpretation |
| **Memory Model** | Native stack slots, manual heap alloc/free | JavaScript Garbage Collector (GC) | Generational heap manager |
| **Primary Use** | High-performance CLI tools, servers, compute | Web applications, Node.js microservices | Rapid iteration, testing, diagnostics |

For detailed architectural specifications, type mappings, C ABI declarations, and benchmark data, see [docs/native-backend.md](file:///home/hexx/Projects/NextLang/docs/native-backend.md).

---

## 17. Error-Gate and Validation Architecture

NextLang incorporates a staged error-gate architecture that guarantees code validity before execution:

1. **Gate 1: Lexical Analysis**: Source code is verified for valid UTF-8 characters, properly terminated strings and comments, with byte-precise spans.
2. **Gate 2: Syntactic Validation**: Verified against canonical NextLang grammar; strictly enforces statement termination (`;`), matching delimiters, pattern matching, and correct control-flow and OOP syntax.
3. **Gate 3: Scope & Name Resolution**: Enforces lexical scopes, declaration before use, and constant immutability.
4. **Gate 4: Semantic Type Checking**: Statically verifies hybrid typing (static types by default, explicit `ANY` for dynamic flexibility), function signatures, return paths, and collection types.
5. **Gate 5: OOP System Validation (`NX-OOP-001` - `NX-OOP-017`)**: Enforces inheritance hierarchy validity, contract implementation, visibility restrictions, and static/instance dispatch consistency.
6. **Gate 6: Ownership & Borrow Checking (`NX-OWN-001..003`, `NX-BORROW-001..003`, `NX-MATCH-001..004`)**: Statically validates move semantics, use-after-move, use-after-free, borrow exclusivity, scoped temporary escape, and pattern consistency.
7. **Gate 7: Multi-Backend Target Generation**: Safe runtime tree-walking execution, ahead-of-time JavaScript compilation, or Cranelift native machine code emission.

---

## 18. Multi-Phase Roadmap

- [x] **Phase 1: Language Foundation**: Lexer, AST, Pratt parser, diagnostics, `nx` CLI (`tokenize`, `parse`, `check`), and syntax validation.
- [x] **Phase 2: Semantic Analysis & Type System**: Name resolution, lexical scopes, hybrid type checking, function arity/type validation, return path verification, and static error gates.
- [x] **Phase 3: Complete Object-Oriented Programming (OOP) System**: Classes, interfaces, single inheritance (`extends`), interface implementation (`implements`), access modifiers (`public`, `private`, `protected`), constructors (`init`), `self`/`super`, static members, abstract and final modifiers, and 17 static OOP error gates.
- [x] **Phase 4: Execution Engine**: Tree-walking runtime evaluating AST programs, control flow, functions, OOP, and built-in standard I/O (`nx run <file.nx> [--time]`).
- [x] **Phase 5: Memory Model & Ownership**: Ownership semantics, move vs copy, borrow checking (`&`, `&mut`), `tem` scoped temporaries, generation-counted heap allocation (`alloc`/`free`), match-case pattern matching, and execution timing statistics.
- [x] **Phase 6: JavaScript Target**: Code generation for Node.js and browser environments with zero external dependencies (`nx compile-js <file.nx> [-o <out.js>]`).
- [x] **Phase 7: Native Compilation (NIR)**: Typed NextLang Intermediate Representation (`nx-nir`), Cranelift native code generator (`nx-codegen`), C ABI runtime (`nx-native-runtime`), linker integration, CLI commands (`nx build`, `nx run-native`, `nx dump-nir`), and 265x performance speedup.
- [ ] **Phase 8: Backend Standard Library**: Asynchronous runtime, networking, I/O, and server primitives.
- [ ] **Phase 9: System Programming**: Low-level abstractions, pointer manipulation, and operating system interfaces.
- [ ] **Phase 10: Tooling & Ecosystem**: Content-aware build caching, formatter, Language Server Protocol (LSP), debugger, and package manager.


