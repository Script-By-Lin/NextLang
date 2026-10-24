# NextLang — Phase 6 JavaScript Compiler Backend

This document specifies the architecture, CLI commands, type and construct mappings, runtime helper system, and execution characteristics of the **NextLang JavaScript Compiler Backend** (`nx-js-backend`).

---

## 1. Overview & Architecture

The `nx-js-backend` crate translates verified NextLang programs into clean, standard ECMAScript 2020+ (ES6+) JavaScript. The generated JavaScript is zero-dependency, self-contained, and runnable in both Node.js and modern web browsers.

### Compilation Pipeline

```text
┌─────────────────┐
│ NextLang Source │ (*.nx)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     nx-lexer    │  Tokenization, span tracking, keyword classification
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    nx-parser    │  Recursive-descent & Pratt parser building AST
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   nx-analyzer   │  Static analysis & error gates:
│                 │  - Gate 1..4: Lexical, syntactic, scopes, types
│                 │  - Gate 5: OOP validation (NX-OOP-001..017)
│                 │  - Gate 6: Ownership & borrowing (NX-OWN, NX-BORROW, NX-MATCH)
└────────┬────────┘
         │ (Only valid ASTs proceed)
         ▼
┌─────────────────┐
│  nx-js-backend  │  AST Lowering & Code Emitter:
│                 │  - Inlined zero-dependency runtime helpers
│                 │  - Statement & expression lowering
│                 │  - OOP constructor & super-call transformation
│                 │  - Pattern match condition synthesis
│                 │  - Top-level and main() entry point invocation
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Output JS File  │ (*.js) Executable by Node.js or browser engines
└─────────────────┘
```

---

## 2. CLI Usage

NextLang provides the `compile-js` command in the `nx` CLI:

```bash
# Compile source file to <stem>.js in current directory
nx compile-js <file.nx>

# Compile with explicit output path
nx compile-js <file.nx> -o <out.js>
nx compile-js <file.nx> --output <out.js>

# Emit generated JavaScript directly to stdout
nx compile-js <file.nx> --stdout

# Specify compilation target platform (default: node)
nx compile-js <file.nx> --target node
nx compile-js <file.nx> --target browser
```

### CLI Options

| Flag | Argument | Description | Default |
|---|---|---|---|
| `-o`, `--output` | `<path>` | Path where the compiled JavaScript will be written | `<stem>.js` |
| `--target` | `node \| browser` | Target execution environment for runtime helpers | `node` |
| `--stdout` | *(none)* | Print compiled JavaScript to standard output | `false` |

---

## 3. Construct & Type Mappings

NextLang constructs map directly to ECMAScript standard features:

### Variable Declarations

| NextLang | JavaScript (ES6+) | Notes |
|---|---|---|
| `var x:<T> = val;` | `let x = val;` | Reassignable variable |
| `tem x:<T> = val;` | `let x = val;` | Scoped temporary variable |
| `const x:<T> = val;` | `const x = val;` | Immutable constant |

### Primitive & Collection Types

| NextLang Type | JavaScript Equivalent |
|---|---|
| `INT8`..`INT128`, `UINT8`..`UINT128` | `number` (64-bit IEEE 754 float) |
| `FL32`, `FL64`, `FL128` | `number` (64-bit IEEE 754 float) |
| `BOOL` | `boolean` (`true` / `false`) |
| `CHAR` | `string` (single character) |
| `STRING` | `string` |
| `LIST<T>` | `Array` (`[...]`) |
| `SET<T>` | `Array` (`[...]`) |
| `MAP<K, V>` | `Object` (`{ key: value }`) |
| `HEAP<T>` | `{ slot: 0, val: T }` |

### Operators

| NextLang Operator | JavaScript Operator |
|---|---|
| `+`, `-`, `*`, `/`, `%` | `+`, `-`, `*`, `/`, `%` |
| `==`, `!=` | `===`, `!==` |
| `<`, `<=`, `>`, `>=` | `<`, `<=`, `>`, `>=` |
| `&` | `&&` (short-circuiting logical AND) |
| `\|` | `\|\|` (short-circuiting logical OR) |
| `not` | `!` (logical NOT) |
| `a..b` (Range) | `Array.from({length: (b - a + 1)}, (_, i) => a + i)` |

### Control Flow

| NextLang | JavaScript Equivalent |
|---|---|
| `if cond { ... }` | `if (cond) { ... }` |
| `otherwise cond { ... }` | `else if (cond) { ... }` |
| `else { ... }` | `else { ... }` |
| `As cond { ... }` | `while (cond) { ... }` |
| `fr (init; cond; post) { ... }` | `for (init; cond; post) { ... }` |
| `Do { ... } As cond;` | `do { ... } while (cond);` |
| `break;` / `continue;` | `break;` / `continue;` |
| `return expr;` | `return expr;` |

### Object-Oriented Programming (OOP)

| NextLang OOP Construct | JavaScript Equivalent |
|---|---|
| `class Child extends Parent { ... }` | `class Child extends Parent { ... }` |
| `init(args) { ... }` | `constructor(args) { super(); ... }` |
| `self.field` / `self.method()` | `this.field` / `this.method()` |
| `super.method(args)` | `super.method(args)` |
| `static fn helper() { ... }` | `static helper() { ... }` |
| `ClassName(args)` (instantiation) | `new ClassName(args)` |

*Note: In derived classes extending a base class, `super()` is automatically inserted at the start of the constructor before any `this` access.*

### Pattern Matching (`match ... case`)

NextLang `match ... case` statements are lowered to clean conditional blocks using temporary evaluation variables:

```nextlang
match status {
    case 200 { vout("OK"); }
    case 404 | 500 { vout("Error"); }
    case code if code > 500 { vout("Critical"); }
    case _ { vout("Unknown"); }
}
```

Translates to:

```javascript
const __target_0 = status;
if (__target_0 === 200) {
    console.log("OK");
} else if ((__target_0 === 404 || __target_0 === 500)) {
    console.log("Error");
} else if (((() => { const code = __target_0; return (code > 500); })())) {
    const code = __target_0;
    console.log("Critical");
} else {
    console.log("Unknown");
}
```

---

## 4. Built-in Functions & Runtime Helpers

The generated JavaScript code embeds self-contained runtime helper functions at the top of the file:

### Output: `vout(...)`
Mapped directly to:
```javascript
console.log(...args);
```

### Collection & String Length: `len(x)`
Mapped to the runtime helper `__nx_len(x)`:
```javascript
function __nx_len(x) {
    if (x === null || x === undefined) return 0;
    if (typeof x === 'string' || Array.isArray(x)) return x.length;
    if (x instanceof Map || x instanceof Set) return x.size;
    if (typeof x === 'object') return Object.keys(x).length;
    return 0;
}
```

### Interactive Console Input: `input([prompt])`
Mapped to `__nx_input(prompt)`.

#### Node.js Target (`--target node`):
Uses synchronous line reading from file descriptor 0 (`stdin`) with zero external npm dependencies:
```javascript
function __nx_input(prompt) {
    if (prompt !== undefined && prompt !== null) {
        process.stdout.write(String(prompt));
    }
    const fs = require('fs');
    const buf = Buffer.alloc(1);
    let line = '';
    while (true) {
        let bytesRead = 0;
        try {
            bytesRead = fs.readSync(0, buf, 0, 1, null);
        } catch (e) {
            break;
        }
        if (bytesRead === 0) break;
        const char = buf.toString('utf8', 0, 1);
        if (char === '\n') break;
        if (char !== '\r') line += char;
    }
    return line;
}
```

#### Browser Target (`--target browser`):
Uses standard modal prompt dialog:
```javascript
function __nx_input(prompt) {
    return window.prompt(prompt !== undefined && prompt !== null ? String(prompt) : "") || "";
}
```

### Type Conversions: `to_int(x)` and `to_float(x)`

Mapped to `__nx_to_int(x)` and `__nx_to_float(x)`:
```javascript
function __nx_to_int(x) {
    if (typeof x === 'number') return Math.trunc(x);
    const n = parseInt(String(x).trim(), 10);
    if (Number.isNaN(n)) throw new Error("invalid integer format: '" + x + "'");
    return n;
}

function __nx_to_float(x) {
    if (typeof x === 'number') return x;
    const n = parseFloat(String(x).trim());
    if (Number.isNaN(n)) throw new Error("invalid float format: '" + x + "'");
    return n;
}
```

### Memory Helpers: `alloc(x)` and `free(x)`
Mapped to `__nx_alloc(x)` and `__nx_free(x)`:
```javascript
function __nx_alloc(x) {
    return { slot: 0, val: x };
}
function __nx_free(x) {
    if (x && typeof x === 'object') {
        x.val = null;
    }
}
```

---

## 5. Ownership Semantics vs. JavaScript Garbage Collection

NextLang's ownership model (`Phase 5`) enforces static compile-time rules:
- Values have single ownership.
- Passing or assigning a non-primitive transfers ownership (move).
- References (`&T`, `&mut T`) cannot outlive the referenced value.
- Explicit deallocation (`free(x)`) invalidates the allocation slot.

### Divergence in the JavaScript Target

1. **Compile-Time Guarantees**:
   The JavaScript backend only generates code for ASTs that have successfully passed all 6 error gates in `nx-analyzer`. Therefore, any program compiled to JavaScript is statically proven free of:
   - Use-after-move (`NX-OWN-001`)
   - Double-free (`NX-OWN-002`)
   - Temporary escape (`NX-OWN-003`)
   - Data-races and aliasing across borrows (`NX-BORROW-001..003`)

2. **Runtime Memory Reclamation**:
   While NextLang's native execution engine uses a generation-counted heap, JavaScript executes in a managed environment with a tracing garbage collector. When `free(buffer)` is executed in JavaScript, `__nx_free(buffer)` sets the internal `val` reference to `null`, ensuring memory is released for the JS engine's garbage collector.

---

## 6. Execution Examples

### Compiling and Running with Node.js

```bash
# 1. Compile NextLang program
nx compile-js examples/hello.nx -o dist/hello.js

# 2. Run with Node.js
node dist/hello.js
```

### Interactive Console Input Example

```nextlang
# greeting.nx
var name:<STRING> = input("What is your name? ");
var age_str:<STRING> = input("What is your age? ");
var age:<INT64> = to_int(age_str);

vout("Hello,", name);
vout("Next year you will be:", age + 1);
```

```bash
nx compile-js greeting.nx -o greeting.js
node greeting.js
```

Terminal interaction:
```text
What is your name? Alice
What is your age? 25
Hello, Alice
Next year you will be: 26
```

---

## 7. Known Differences & Limitations

1. **Integer Precision**:
   JavaScript numbers are 64-bit IEEE 754 floats, which represent safe integers precisely up to $2^{53} - 1$ (`Number.MAX_SAFE_INTEGER` = `9,007,199,254,740,991`). High-precision 64-bit and 128-bit NextLang integers exceeding this range will experience precision truncation unless converted to `BigInt`.
2. **Synchronous File I/O**:
   The Node.js implementation of `__nx_input` uses synchronous file descriptor reads (`fs.readSync`). This is optimal for CLI utilities and interactive scripts, but blocks the Node.js event loop during input pauses.
