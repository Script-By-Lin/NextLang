# NextLang Memory Model & Safety Specification

NextLang delivers high performance and safe manual memory management without requiring a mandatory tracing garbage collector. This document describes the memory model, ownership semantics, borrowing rules, reference types, generation-counted heap allocation, and compatibility with JavaScript and native compilation targets.

---

## 1. Core Principles

NextLang is designed for predictable, deterministic latency and zero-overhead memory safety. The memory architecture relies on four foundational pillars:

1. **Affine Types & Move Semantics**: Non-copy values have a single unique owner at any given point in program execution. Transferring a value transfers ownership; accessing a moved value is a static compile-time error (`NX-OWN-001`).
2. **Explicit Borrowing & Lifetimes**: References (`&T` and `&mut T`) allow reading or writing data without taking ownership. The static analyzer enforces the Single-Writer / Multiple-Readers aliasing XOR mutability invariant at compile-time (`NX-BORROW-001..003`).
3. **Scoped Temporaries (`tem`)**: The `tem` keyword introduces local stack-bounded temporaries that are guaranteed never to escape their declaring scope or outlive their lexical block (`NX-OWN-003`).
4. **Generation-Counted Safe Manual Heap**: Explicit heap allocation (`alloc`) and deallocation (`free`) provide direct control over memory layout. Static ownership tracking prevents double-frees at compile-time (`NX-OWN-002`), while generational handles prevent use-after-free at runtime.

---

## 2. Value Categories: Copy vs. Move Semantics

NextLang partitions all types into two categories:

### 2.1 Copy Types
Primitive value types are small, fixed-size data stored directly on the stack or in registers. When assigned or passed into functions, their bits are trivially copied:

- **Signed Integers**: `<INT8>`, `<INT16>`, `<INT32>`, `<INT64>`, `<INT128>`
- **Unsigned Integers**: `<UINT8>`, `<UINT16>`, `<UINT32>`, `<UINT64>`, `<UINT128>`
- **Floating Point**: `<FL32>`, `<FL64>`, `<FL128>`
- **Booleans & Characters**: `<BOOL>`, `<CHAR>`

```nextlang
var a:<INT32> = 42;
var b:<INT32> = a; // Copy: both 'a' and 'b' remain valid
vout(a, b);        // Prints: 42 42
```

### 2.2 Move Types
Heap-allocated values, class instances, strings, and collection buffers represent owned resources. When assigned to a new variable or passed by value, ownership is transferred:

- **Class Instances**: `ObjectInstance`
- **Heap Handles**: `<HEAP<T>>`
- **Dynamic Collections**: `<LIST<T>>`, `<MAP<K, V>>`, `<SET<T>>`

```nextlang
var x:<HEAP<STRING>> = alloc("Resource");
var y:<HEAP<STRING>> = x; // Move: ownership transferred to 'y'
// vout(x); // ERROR: NX-OWN-001: use of moved value 'x'
vout(y);    // OK
```

---

## 3. Borrowing & Reference Types

References allow temporary access to data without transferring ownership.

### 3.1 Immutable References (`&T`)
An immutable reference provides read-only access. Multiple immutable references can coexist simultaneously.

```nextlang
var msg:<STRING> = "Hello";
var r1:<&STRING> = &msg;
var r2:<&STRING> = &msg;
vout(r1, r2); // Multiple readers allowed
```

### 3.2 Mutable References (`&mut T`)
A mutable reference allows reading and mutating the referenced target:

```nextlang
var counter:<INT32> = 10;
var ref_c:<&mut INT32> = &mut counter;
```

### 3.3 Borrow Rules & Error Gates
NextLang statically enforces the following invariants:

| Code | Invariant | Description |
|---|---|---|
| `NX-BORROW-001` | Aliasing XOR Mutability | Cannot mutate or assign to a variable while active borrows exist. |
| `NX-BORROW-002` | Preservation | Cannot move ownership of a variable while it is borrowed. |
| `NX-BORROW-003` | Exclusivity & Safety | Cannot create a mutable borrow of an immutable (`const`) variable, or have multiple concurrent mutable borrows. |

---

## 4. Scoped Temporaries (`tem`)

The `tem` keyword declares a temporary variable bound strictly to its enclosing lexical block:

```nextlang
fn process() {
    tem buffer:<INT32> = compute_temp();
    // buffer is guaranteed to exist only within this block
}
```

### Escape Analysis (`NX-OWN-003`)
A `tem` variable cannot be returned from a function, assigned to a variable in an outer scope, or stored into an external heap structure:

```nextlang
fn invalid_escape():<INT32> {
    tem local:<INT32> = 100;
    return local; // ERROR: NX-OWN-003: temporary variable 'local' cannot escape function scope
}
```

---

## 5. Safe Manual Heap Management (`alloc` / `free`)

NextLang provides explicit `alloc(expr)` and `free(target)` primitives.

### 5.1 Generational Handles
In the NextLang runtime and native code generation, heap allocations yield a `HeapHandle`:
- **Slot Index**: Identifies the memory block in the heap table.
- **Generation Counter**: A 64-bit monotonically incremented sequence number.

When `free(h)` is called:
1. The memory block is deallocated.
2. The slot's generation counter is incremented.
3. The slot is marked free for future reuse.

If any stale handle attempts to access a freed slot, the generation counter mismatch instantly traps with a clean error, preventing dangling pointer vulnerabilities.

### 5.2 Double-Free & Use-After-Free Prevention (`NX-OWN-002`)
The static analyzer tracks deallocation states at compile time:

```nextlang
var h:<HEAP<STRING>> = alloc("Data");
free(h);
// free(h); // ERROR: NX-OWN-002: value 'h' has already been deallocated
// vout(h); // ERROR: NX-OWN-002: use of deallocated value 'h'
```

---

## 6. Compilation Target Interoperability

### 6.1 JavaScript Target (Web & Node.js)
When compiling to JavaScript:
- `alloc` maps to garbage-collected JS object handles or typed array allocations (`ArrayBuffer`).
- `free` detaches handles and invokes cleanup hooks or explicit pool recycles.
- Ownership and borrow gates are verified at compile-time by `nx check`, guaranteeing safe execution in JavaScript without requiring runtime borrow counters.

### 6.2 Native Target (NIR & LLVM/Machine Code)
When compiling to native binary:
- `alloc` lowers directly to arena or jemalloc memory allocators.
- `free` lowers directly to allocator deallocations with zero runtime overhead.
- Generational counters can be enabled in debug builds and stripped in `--release` builds once proven sound by static ownership analysis.
