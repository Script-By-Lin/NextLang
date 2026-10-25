<h1>NextLang Language Reference and Usage Guide</h1>

<p>
NextLang is a modern, statically typed systems and application programming language designed for high performance, memory efficiency, and versatile platform delivery. NextLang compiles directly to native machine code through <b>LLVM</b> and can also emit clean JavaScript for web runtimes.
</p>

<hr>

<h2>1. Lexical Structure and Source Conventions</h2>

<p>
NextLang source code files use the <b>.nx</b> file extension. Source files are encoded in standard UTF-8 text.
</p>

<h3>Comments</h3>
<p>
Comments in NextLang document code logic without affecting compiler execution. NextLang supports single-line comments and multi-line block comments.
</p>

<pre><code>// Single-line comment explaining the following statement
var status:&lt;INT32&gt; = 1;

// Multi-line explanations can be written
// using consecutive double-slash comment lines
var active:&lt;BOOL&gt; = true;
</code></pre>

<h3>Identifiers</h3>
<p>
Identifiers name variables, constants, functions, classes, interfaces, structs, and fields. An identifier must start with an ASCII letter (a-z, A-Z) or an underscore (_), followed by zero or more letters, digits, or underscores.
</p>

<h3>Semicolons</h3>
<p>
Statements in NextLang must end with a semicolon (<b>;</b>). Omitting a semicolon triggers a compiler diagnostic indicating the exact file location.
</p>

<hr>

<h2>2. Variables, Mutability, and Scoping</h2>

<p>
NextLang provides three explicit declaration keywords to manage state, mutability, and allocation scope.
</p>

<h3>Mutable Variables with var</h3>
<p>
The <b>var</b> keyword declares a standard mutable variable. The variable can be reassigned during its lifetime.
</p>
<pre><code>var counter:&lt;INT32&gt; = 0;
counter = counter + 1;
</code></pre>

<h3>Immutable Bindings with const</h3>
<p>
The <b>const</b> keyword declares an immutable variable binding. Reassignment to a const variable is strictly prohibited and caught at compile time.
</p>
<pre><code>const maximum_connections:&lt;INT32&gt; = 1000;
// Attempting maximum_connections = 2000 produces compile error NX-TYPE-001
</code></pre>

<h3>Temporary Block Variables with tem</h3>
<p>
The <b>tem</b> keyword declares a temporary scoped variable intended for short-lived intermediate computation.
</p>
<pre><code>tem temporary_token:&lt;STRING&gt; = "auth_session_token";
vout("Session:", temporary_token);
</code></pre>

<h3>Lexical Scope</h3>
<p>
Scopes are formed by curly braces <b>{</b> and <b>}</b>. Variables declared inside an inner block are isolated from the surrounding scope and cleaned up when execution leaves the block.
</p>

<hr>

<h2>3. Comprehensive Type System</h2>

<p>
Type annotations in NextLang are explicitly declared within angle brackets after a colon: <b>:&lt;TYPE&gt;</b>.
</p>

<h3>Exact Width Signed Integers</h3>
<p>
NextLang provides exact bit-width signed integers stored using two's complement representation:
</p>
- <b>INT8</b>: 8-bit signed integer (-128 to 127)
- <b>INT16</b>: 16-bit signed integer (-32768 to 32767)
- <b>INT32</b>: 32-bit signed integer (-2147483648 to 2147483647)
- <b>INT64</b>: 64-bit signed integer (standard full-width integer)
- <b>INT128</b>: 128-bit signed integer for high-precision arithmetic

<pre><code>var small_num:&lt;INT8&gt; = 127;
var large_num:&lt;INT64&gt; = 9223372036854775807;
</code></pre>

<h3>Exact Width Unsigned Integers</h3>
<p>
Unsigned integers represent non-negative binary values:
</p>
- <b>UINT8</b>: 8-bit unsigned integer / byte (0 to 255)
- <b>UINT16</b>: 16-bit unsigned integer (0 to 65535)
- <b>UINT32</b>: 32-bit unsigned integer (0 to 4294967295)
- <b>UINT64</b>: 64-bit unsigned integer
- <b>UINT128</b>: 128-bit unsigned integer

<pre><code>var byte_val:&lt;UINT8&gt; = 255;
var word_val:&lt;UINT32&gt; = 4000000000;
</code></pre>

<h3>Platform-Sized Integers and Raw Pointers</h3>
<p>
For systems programming and direct operating system interaction, NextLang supports architecture-native widths:
</p>
- <b>USIZE</b>: Pointer-sized unsigned integer (64-bit on x86_64 and aarch64)
- <b>ISIZE</b>: Pointer-sized signed integer (64-bit on x86_64 and aarch64)
- <b>PTR</b>: Raw untyped memory pointer address

<pre><code>var buffer_length:&lt;USIZE&gt; = 1024;
var memory_offset:&lt;ISIZE&gt; = -8;
var raw_address:&lt;PTR&gt; = buffer_new(64);
</code></pre>

<h3>Numeric Literal Formats</h3>
<p>
Integer literals can be written in decimal, hexadecimal, or binary format:
</p>
<pre><code>var decimal_val:&lt;INT32&gt; = 42;
var hex_val:&lt;INT32&gt; = 0x2A;
var binary_val:&lt;INT32&gt; = 0b101010;
</code></pre>

<h3>Floating Point Numbers</h3>
- <b>FL32</b>: IEEE-754 32-bit single-precision float
- <b>FL64</b>: IEEE-754 64-bit double-precision float
- <b>FL128</b>: 128-bit quad-precision floating point

<pre><code>var ratio:&lt;FL32&gt; = 1.25;
var precise:&lt;FL64&gt; = 3.141592653589793;
</code></pre>

<h3>Booleans, Characters, and Strings</h3>
- <b>BOOL</b>: Boolean value, either <b>true</b> or <b>false</b>
- <b>CHAR</b>: 32-bit Unicode character enclosed in single quotes, such as 'A' or '\n'
- <b>STRING</b>: Sequence of UTF-8 characters enclosed in double quotes, such as "Hello"

<pre><code>var is_running:&lt;BOOL&gt; = true;
var separator:&lt;CHAR&gt; = ':';
var greeting:&lt;STRING&gt; = "Welcome to NextLang";
</code></pre>

<h3>Generic Collections</h3>
<p>
NextLang provides built-in parameterized collection types:
</p>
- <b>LIST&lt;T&gt;</b>: Dynamic array storing ordered items of type T
- <b>SET&lt;T&gt;</b>: Collection of unique elements of type T
- <b>MAP&lt;K, V&gt;</b>: Key-value dictionary mapping keys of type K to values of type V

<pre><code>var scores:&lt;LIST&lt;INT32&gt;&gt; = [95, 88, 72, 100];
var allowed_tags:&lt;SET&lt;STRING&gt;&gt; = {"system", "admin", "user"};
var lookup:&lt;MAP&lt;STRING, INT32&gt;&gt; = {"alice": 101, "bob": 102};
</code></pre>

<h3>Union Types</h3>
<p>
A union type declares that a value can be one of several designated types:
</p>
<pre><code>var result:&lt;INT32 | STRING&gt; = 404;
result = "Not Found";
</code></pre>

<h3>Ownership References</h3>
<p>
NextLang implements static borrow checking:
</p>
- <b>&T</b>: Immutable reference to type T
- <b>&mut T</b>: Mutable reference allowing in-place modification

<pre><code>var original:&lt;INT32&gt; = 50;
var ref_view:&lt;&INT32&gt; = &original;
</code></pre>

<hr>

<h2>4. Operators and Expressions</h2>

<h3>Arithmetic Operators</h3>
<p>
NextLang supports standard mathematical operations:
</p>
- <b>+</b> : Addition and string concatenation
- <b>-</b> : Subtraction and unary negation
- <b>/</b> : Division
- <b>%</b> : Modulo remainder
- <b>++</b> : Increment
- <b>--</b> : Decrement

<pre><code>var sum:&lt;INT32&gt; = 10 + 20;
var difference:&lt;INT32&gt; = 50 - 15;
var quotient:&lt;INT32&gt; = 100 / 4;
var remainder:&lt;INT32&gt; = 17 % 5;
sum++;
</code></pre>

<h3>Relational and Equality Operators</h3>
- <b>==</b> : Equal to
- <b>!=</b> : Not equal to
- <b>&lt;</b> : Less than
- <b>&lt;=</b> : Less than or equal to
- <b>&gt;</b> : Greater than
- <b>&gt;=</b> : Greater than or equal to

<h3>Logical Operators</h3>
- <b>&</b> : Short-circuiting logical AND
- <b>|</b> : Short-circuiting logical OR
- <b>!</b> : Logical NOT

<pre><code>if (age &gt;= 18) & (has_permit) {
    vout("Eligible");
}
</code></pre>

<h3>Bitwise Systems Operators</h3>
<p>
For low-level hardware control, network protocol packing, and binary cryptography:
</p>
- <b>&</b> : Bitwise AND
- <b>|</b> : Bitwise OR
- <b>^</b> : Bitwise XOR
- <b>~</b> : Bitwise NOT (one's complement bit inversion)
- <b>&lt;&lt;</b> : Left bit shift
- <b>&gt;&gt;</b> : Right bit shift (arithmetic for signed, logical for unsigned)

<pre><code>var a:&lt;INT32&gt; = 12; // Binary 1100
var b:&lt;INT32&gt; = 5;  // Binary 0101

var bit_and:&lt;INT32&gt; = a & b;   // Result: 4  (0100)
var bit_or:&lt;INT32&gt;  = a | b;   // Result: 13 (1101)
var bit_xor:&lt;INT32&gt; = a ^ b;   // Result: 9  (1001)
var shifted_l:&lt;INT32&gt; = a &lt;&lt; 2; // Result: 48 (110000)
var shifted_r:&lt;INT32&gt; = a &gt;&gt; 1; // Result: 6  (0011)
var inverted:&lt;INT32&gt; = ~a;     // Result: -13
</code></pre>

<hr>

<h2>5. Control Flow Structures</h2>

<h3>Conditional Branching</h3>
<p>
NextLang uses <b>if</b>, <b>otherwise</b>, and <b>else</b> for conditional branches:
</p>
<pre><code>var score:&lt;INT32&gt; = 85;

if score &gt;= 90 {
    vout("Grade: A");
} otherwise score &gt;= 80 {
    vout("Grade: B");
} otherwise score &gt;= 70 {
    vout("Grade: C");
} else {
    vout("Grade: Needs Improvement");
}
</code></pre>

<h3>Three-Clause Loop with fr</h3>
<p>
The <b>fr</b> loop provides bounded iteration with initialization, condition, and step clauses enclosed in angle brackets:
</p>
<pre><code>fr&lt;var i:&lt;INT32&gt; = 0: i &lt; 5: i++&gt; {
    vout("Iteration:", i);
}
</code></pre>

<h3>Conditional Loop with As</h3>
<p>
The <b>As</b> loop executes continuously while its condition expression evaluates to true:
</p>
<pre><code>var count:&lt;INT32&gt; = 3;
As count &gt; 0 {
    vout("Countdown:", count);
    count--;
}
</code></pre>

<h3>Infinite Loop with loop, skip, and halt</h3>
<p>
The <b>loop</b> construct runs unconditionally until interrupted:
</p>
- <b>skip;</b>: Skips the remainder of the current loop iteration (analogous to continue).
- <b>halt;</b>: Immediately terminates the enclosing loop (analogous to break).

<pre><code>var n:&lt;INT32&gt; = 0;
loop {
    n++;
    if n == 2 {
        skip;
    }
    if n &gt; 4 {
        halt;
    }
    vout("Loop value:", n);
}
</code></pre>

<h3>Pattern Matching with match and case</h3>
<p>
NextLang provides robust pattern matching with literal values, wildcards, and conditional guards:
</p>
<pre><code>var code:&lt;INT32&gt; = 200;

match code {
    case 200 {
        vout("Status: Success");
    }
    case 404 {
        vout("Status: Resource Not Found");
    }
    case 500 {
        vout("Status: Internal Server Error");
    }
    case _ {
        vout("Status: Unrecognized Response Code");
    }
}
</code></pre>

<p>
Pattern matches can also incorporate guard conditions:
</p>
<pre><code>var temp:&lt;INT32&gt; = 35;
match temp {
    case t if t &gt;= 30 {
        vout("Hot climate");
    }
    case t if t &gt;= 15 {
        vout("Mild climate");
    }
    case _ {
        vout("Cold climate");
    }
}
</code></pre>

<hr>

<h2>6. Functions and Procedures</h2>

<p>
Functions are declared using the <b>fn</b> keyword, followed by parameter names and types, and an explicit return type.
</p>

<h3>Value-Returning Functions</h3>
<pre><code>fn compute_area(width:&lt;FL64&gt;, height:&lt;FL64&gt;):&lt;FL64&gt; {
    var total:&lt;FL64&gt; = width / 2.0;
    return width + height;
}

var result:&lt;FL64&gt; = compute_area(10.0, 20.0);
vout("Computed:", result);
</code></pre>

<h3>Procedures Returning VOID</h3>
<p>
Procedures that perform side effects without returning data declare return type <b>:&lt;VOID&gt;</b> and exit using <b>return;</b>:
</p>
<pre><code>fn print_user_badge(username:&lt;STRING&gt;, level:&lt;INT32&gt;):&lt;VOID&gt; {
    vout("User:", username, "| Security Level:", level);
    return;
}
</code></pre>

<h3>Program Entry Point</h3>
<p>
A NextLang executable program defines a parameterless <b>fn main()</b> function as its initial execution entry point:
</p>
<pre><code>fn main() {
    vout("NextLang Program Starting...");
}
</code></pre>

<hr>

<h2>7. Memory Management and Ownership</h2>

<p>
NextLang bridges high-level expressiveness with direct native systems memory control.
</p>

<h3>Explicit Heap Allocation with alloc</h3>
<p>
Allocates dynamic memory on the native heap and initializes it with the provided value:
</p>
<pre><code>var heap_string:&lt;STRING&gt; = alloc("Persisted Dynamic Content");
vout("Heap String:", heap_string);
</code></pre>

<h3>Explicit Deallocation with free</h3>
<p>
Releases previously allocated memory back to the system memory allocator:
</p>
<pre><code>free(heap_string);
vout("Memory successfully reclaimed");
</code></pre>

<h3>Ownership Transfer and Move Semantics</h3>
<p>
When an owned heap resource is assigned to another variable or passed by value, ownership transfers to the target. Attempting to access the moved resource in safe code is detected and prevented by the compiler semantic analyzer.
</p>

<hr>

<h2>8. Object-Oriented Programming (OOP)</h2>

<p>
NextLang features a complete Object-Oriented Programming model designed for clean domain architecture.
</p>

<h3>Interfaces</h3>
<p>
An <b>interface</b> defines a public contract of methods that implementing classes must satisfy:
</p>
<pre><code>interface Identifiable {
    fn get_id():&lt;INT32&gt;;
    fn get_name():&lt;STRING&gt;;
}
</code></pre>

<h3>Class Declarations and Constructors</h3>
<p>
Classes encapsulate fields and methods. The constructor method is named <b>init</b>:
</p>
<pre><code>class Person implements Identifiable {
    public var id:&lt;INT32&gt; = 0;
    public var name:&lt;STRING&gt; = "";
    public var email:&lt;STRING&gt; = "";

    public fn init(p_id:&lt;INT32&gt;, p_name:&lt;STRING&gt;, p_email:&lt;STRING&gt;):&lt;VOID&gt; {
        self.id = p_id;
        self.name = p_name;
        self.email = p_email;
        return;
    }

    public fn get_id():&lt;INT32&gt; {
        return self.id;
    }

    public fn get_name():&lt;STRING&gt; {
        return self.name;
    }

    public fn display():&lt;VOID&gt; {
        vout("Person ID:", self.id, "Name:", self.name, "Email:", self.email);
        return;
    }
}
</code></pre>

<h3>Inheritance with extends and Super Calls</h3>
<p>
A class can inherit fields and methods from a parent class using <b>extends</b>. The derived class can call parent methods using <b>super</b>:
</p>
<pre><code>class Student extends Person {
    public var major:&lt;STRING&gt; = "";
    public var gpa:&lt;FL64&gt; = 0.0;

    public fn init(s_id:&lt;INT32&gt;, s_name:&lt;STRING&gt;, s_email:&lt;STRING&gt;, s_major:&lt;STRING&gt;, s_gpa:&lt;FL64&gt;):&lt;VOID&gt; {
        self.id = s_id;
        self.name = s_name;
        self.email = s_email;
        self.major = s_major;
        self.gpa = s_gpa;
        return;
    }

    public fn get_gpa():&lt;FL64&gt; {
        return self.gpa;
    }

    public fn display():&lt;VOID&gt; {
        vout("Student ID:", self.id, "| Name:", self.name, "| Major:", self.major, "| GPA:", self.gpa);
        return;
    }
}
</code></pre>

<h3>Static Methods</h3>
<p>
Methods declared with <b>static</b> are called directly on the class name without requiring an instance:
</p>
<pre><code>class MathUtil {
    public static fn double_val(x:&lt;INT32&gt;):&lt;INT32&gt; {
        return x + x;
    }
}

var res:&lt;INT32&gt; = MathUtil.double_val(25);
vout("Doubled:", res);
</code></pre>

<h3>Polymorphism</h3>
<p>
Derived class instances can be assigned to base class references:
</p>
<pre><code>var s:&lt;Student&gt; = Student(101, "Alice", "alice@university.edu", "Computer Science", 3.9);
var p:&lt;Person&gt; = s; // Polymorphic upcast
p.display();
</code></pre>

<hr>

<h2>9. Systems Programming and Low-Level Control</h2>

<p>
NextLang provides direct low-level control while strictly establishing a clear safety boundary.
</p>

<h3>Native Structs</h3>
<p>
A <b>struct</b> defines a contiguous native memory layout:
</p>
<pre><code>struct Vector3 {
    x:&lt;FL64&gt;,
    y:&lt;FL64&gt;,
    z:&lt;FL64&gt;
}

// Instantiation via struct literal
var vec:&lt;Vector3&gt; = Vector3 {
    x: 10.0,
    y: 20.0,
    z: 30.0
};

vout("Vector x:", vec.x, "y:", vec.y, "z:", vec.z);
</code></pre>

<h3>Foreign Function Interface (FFI) with extern "C"</h3>
<p>
NextLang can link against any native C library or operating system ABI:
</p>
<pre><code>extern "C" {
    fn abs(n:&lt;INT32&gt;):&lt;INT32&gt;;
    fn puts(str:&lt;STRING&gt;):&lt;INT32&gt;;
    fn getpid():&lt;INT32&gt;;
}
</code></pre>

<h3>Unsafe Boundary and Gate 8 Safety Validation</h3>
<p>
Calling external C ABI functions or manipulating raw memory pointers is strictly forbidden in safe code. The compiler enforces safety rule <b>NX-SYS-001</b>: all FFI calls and raw pointer operations must be enclosed inside an explicit <b>unsafe { ... }</b> block.
</p>
<pre><code>unsafe {
    var raw_pid:&lt;INT32&gt; = getpid();
    vout("Operating System Process ID:", raw_pid);

    var positive:&lt;INT32&gt; = abs(-75);
    vout("Absolute value:", positive);

    puts("Direct C ABI output message");
}
</code></pre>

<h3>Native Systems Standard Library Builtins</h3>

<p><b>1. File Operating System I/O</b></p>
- <b>file_open(path:&lt;STRING&gt;, mode:&lt;STRING&gt;):&lt;INT64&gt;</b>: Opens file handle.
- <b>file_close(handle:&lt;INT64&gt;):&lt;BOOL&gt;</b>: Flushes and closes file handle.
- <b>file_read_all(path:&lt;STRING&gt;):&lt;STRING&gt;</b>: Reads entire file content as string.
- <b>file_write(path:&lt;STRING&gt;, data:&lt;STRING&gt;):&lt;BOOL&gt;</b>: Writes string content to file.
- <b>file_exists(path:&lt;STRING&gt;):&lt;BOOL&gt;</b>: Returns true if file exists on disk.
- <b>file_remove(path:&lt;STRING&gt;):&lt;BOOL&gt;</b>: Deletes file from filesystem.

<pre><code>var filepath:&lt;STRING&gt; = "output_data.txt";
file_write(filepath, "High-performance systems programming in NextLang");

if file_exists(filepath) {
    var content:&lt;STRING&gt; = file_read_all(filepath);
    vout("Read back:", content);
    file_remove(filepath);
}
</code></pre>

<p><b>2. Environment and Process Management</b></p>
- <b>get_pid():&lt;INT32&gt;</b>: Returns process ID of the current running process.
- <b>get_cwd():&lt;STRING&gt;</b>: Returns current working directory path string.
- <b>env_get(key:&lt;STRING&gt;):&lt;STRING&gt;</b>: Reads environment variable value.
- <b>env_set(key:&lt;STRING&gt;, val:&lt;STRING&gt;):&lt;BOOL&gt;</b>: Sets environment variable.

<pre><code>vout("Current Working Directory:", get_cwd());
vout("Process ID:", get_pid());
env_set("APP_MODE", "PRODUCTION");
vout("Environment APP_MODE:", env_get("APP_MODE"));
</code></pre>

<p><b>3. Byte Buffers and Binary Endianness</b></p>
- <b>buffer_new(size:&lt;USIZE&gt;):&lt;PTR&gt;</b>: Allocates a zero-initialized native byte buffer.
- <b>buffer_free(buf:&lt;PTR&gt;):&lt;VOID&gt;</b>: Deallocates byte buffer.
- <b>buffer_size(buf:&lt;PTR&gt;):&lt;USIZE&gt;</b>: Returns byte size of buffer.
- <b>buffer_get_u8(buf:&lt;PTR&gt;, offset:&lt;USIZE&gt;):&lt;UINT8&gt;</b>: Reads byte with bounds checking.
- <b>buffer_set_u8(buf:&lt;PTR&gt;, offset:&lt;USIZE&gt;, val:&lt;UINT8&gt;):&lt;VOID&gt;</b>: Writes byte with bounds checking.
- <b>buffer_get_u32_le(buf:&lt;PTR&gt;, offset:&lt;USIZE&gt;):&lt;UINT32&gt;</b>: Reads 32-bit unsigned little-endian word.
- <b>buffer_set_u32_le(buf:&lt;PTR&gt;, offset:&lt;USIZE&gt;, val:&lt;UINT32&gt;):&lt;VOID&gt;</b>: Writes 32-bit unsigned little-endian word.

<pre><code>var buf:&lt;PTR&gt; = buffer_new(16);
buffer_set_u8(buf, 0, 0xDE);
buffer_set_u8(buf, 1, 0xAD);
buffer_set_u32_le(buf, 4, 1000000);

var b0:&lt;UINT8&gt; = buffer_get_u8(buf, 0);
var val32:&lt;UINT32&gt; = buffer_get_u32_le(buf, 4);
vout("Byte 0:", b0, "Word at 4:", val32);

buffer_free(buf);
</code></pre>

<p><b>4. Raw Pointer Arithmetic and Direct Memory Access</b></p>
- <b>ptr_offset(base:&lt;PTR&gt;, byte_delta:&lt;ISIZE&gt;):&lt;PTR&gt;</b>: Performs native pointer arithmetic.
- <b>ptr_read_u8(addr:&lt;PTR&gt;):&lt;UINT8&gt;</b>: Reads single byte from raw memory address.
- <b>ptr_write_u8(addr:&lt;PTR&gt;, val:&lt;UINT8&gt;):&lt;VOID&gt;</b>: Writes single byte to raw memory address.

<pre><code>unsafe {
    var raw_mem:&lt;PTR&gt; = buffer_new(8);
    ptr_write_u8(raw_mem, 65); // ASCII 'A'
    var second_byte:&lt;PTR&gt; = ptr_offset(raw_mem, 1);
    ptr_write_u8(second_byte, 66); // ASCII 'B'

    vout("Raw byte 0:", ptr_read_u8(raw_mem));
    vout("Raw byte 1:", ptr_read_u8(second_byte));
    buffer_free(raw_mem);
}
</code></pre>

<hr>

<h2>10. Standard Input, Output, and Conversions</h2>

<h3>Formatted Printing with vout</h3>
<p>
The <b>vout</b> built-in function accepts an arbitrary list of arguments, outputs them to standard output separated by spaces, and appends a newline:
</p>
<pre><code>vout("Server status:", 200, "OK", true);
</code></pre>

<h3>Console Input with input</h3>
<p>
Prompts the user on standard output and reads a line from standard input as a string:
</p>
<pre><code>var user_input:&lt;STRING&gt; = input("Enter your username: ");
vout("Welcome,", user_input);
</code></pre>

<h3>Conversions and String Length</h3>
- <b>to_int(s:&lt;STRING&gt;):&lt;INT64&gt;</b>: Converts textual decimal representation to 64-bit integer.
- <b>to_float(s:&lt;STRING&gt;):&lt;FL64&gt;</b>: Converts text to 64-bit double-precision floating point.
- <b>len(s:&lt;STRING&gt;):&lt;INT64&gt;</b>: Returns number of characters in string.

<pre><code>var num_str:&lt;STRING&gt; = "450";
var parsed_int:&lt;INT64&gt; = to_int(num_str);
var parsed_flt:&lt;FL64&gt; = to_float("99.95");
var str_length:&lt;INT64&gt; = len(num_str);

vout("Parsed int:", parsed_int, "float:", parsed_flt, "length:", str_length);
</code></pre>

<hr>

<h2>11. Compiler Tooling and CLI Usage</h2>

<p>
The NextLang compiler CLI binary <b>nx</b> provides comprehensive tools for building, inspecting, and running applications.
</p>

<h3>Run Natively with LLVM Backend</h3>
<p>
Compiles NextLang source to typed LLVM IR, verifies IR with the LLVM optimizer, links with the native runtime via Clang, and executes immediately:
</p>
<pre><code>./target/release/nx run-native main.nx
</code></pre>

<h3>Build Standalone Optimized Native Binary</h3>
<p>
Produces a standalone executable linked for your operating system architecture:
</p>
<pre><code>// Build with maximum release optimization (-O3)
./target/release/nx build main.nx -o my_app --opt-level 3

// Build with execution timing diagnostics
./target/release/nx build main.nx -o my_app --opt-level 3 --time

// Build and inspect emitted textual LLVM IR
./target/release/nx build main.nx -o my_app --emit-llvm
</code></pre>

<h3>Dump LLVM Intermediate Representation</h3>
<p>
Prints the verified LLVM IR module generated from NextLang source directly to stdout:
</p>
<pre><code>./target/release/nx dump-llvm main.nx
</code></pre>

<h3>Run with Interactive Tree-Walk Interpreter</h3>
<p>
Executes NextLang source directly in the developer interpreter without compiling native binaries:
</p>
<pre><code>./target/release/nx run main.nx
</code></pre>

<h3>Semantic Static Analysis and Linting</h3>
<p>
Validates syntax, ownership, borrow checking, and type correctness without producing code:
</p>
<pre><code>./target/release/nx check main.nx
</code></pre>

<h3>Compile to Web JavaScript</h3>
<p>
Transpiles NextLang AST into standard ECMAScript for browser or Node.js runtimes:
</p>
<pre><code>./target/release/nx js main.nx -o bundle.js
</code></pre>

<hr>

<h2>12. Complete Systems Application Example</h2>

<p>
The following program integrates object-oriented domain models, native binary buffers, bitwise protocol encoding, and operating system calls:
</p>

<pre><code>// NextLang Systems Application Demonstration

class DeviceTelemetry {
    public var device_id:&lt;INT32&gt; = 0;
    public var flags:&lt;UINT8&gt; = 0;
    public var signal_strength:&lt;FL64&gt; = 0.0;

    public fn init(id:&lt;INT32&gt;, initial_flags:&lt;UINT8&gt;, signal:&lt;FL64&gt;):&lt;VOID&gt; {
        self.device_id = id;
        self.flags = initial_flags;
        self.signal_strength = signal;
        return;
    }

    public fn enable_sensor(mask:&lt;UINT8&gt;):&lt;VOID&gt; {
        self.flags = self.flags | mask;
        return;
    }

    public fn is_sensor_active(mask:&lt;UINT8&gt;):&lt;BOOL&gt; {
        return (self.flags & mask) != 0;
    }

    public fn serialize_packet():&lt;PTR&gt; {
        var packet:&lt;PTR&gt; = buffer_new(16);
        buffer_set_u32_le(packet, 0, self.device_id);
        buffer_set_u8(packet, 4, self.flags);
        return packet;
    }

    public fn display():&lt;VOID&gt; {
        vout("Device ID:", self.device_id, "Flags:", self.flags, "Signal:", self.signal_strength);
        return;
    }
}

fn main() {
    vout("=== Starting NextLang Telemetry Subsystem ===");
    vout("Host Process ID:", get_pid());
    vout("Working Directory:", get_cwd());

    // Instantiate telemetry model
    var dev:&lt;DeviceTelemetry&gt; = DeviceTelemetry(9001, 0b00000001, 98.6);
    dev.display();

    // Enable sensors using bitwise flags
    const GPS_FLAG:&lt;UINT8&gt; = 0b00000010;
    const LIDAR_FLAG:&lt;UINT8&gt; = 0b00000100;
    dev.enable_sensor(GPS_FLAG);
    dev.enable_sensor(LIDAR_FLAG);

    vout("After enabling sensors:");
    dev.display();
    vout("GPS Sensor Active:", dev.is_sensor_active(GPS_FLAG));
    vout("LIDAR Sensor Active:", dev.is_sensor_active(LIDAR_FLAG));

    // Serialize packet to native byte buffer
    var raw_packet:&lt;PTR&gt; = dev.serialize_packet();
    var read_id:&lt;UINT32&gt; = buffer_get_u32_le(raw_packet, 0);
    var read_flags:&lt;UINT8&gt; = buffer_get_u8(raw_packet, 4);

    vout("Serialized binary packet verified - ID:", read_id, "Flags byte:", read_flags);
    buffer_free(raw_packet);

    // Persist event log
    var log_file:&lt;STRING&gt; = "telemetry_run.log";
    file_write(log_file, "Telemetry packet serialized and verified successfully.");
    if file_exists(log_file) {
        vout("Log record confirmed on disk.");
        file_remove(log_file);
    }

    vout("=== Telemetry Subsystem Finished Successfully ===");
}
</code></pre>

<hr>

<h2>13. Modules and Standard Library</h2>

<p>
NextLang provides a modular standard library and import system. External files and standard library modules are brought into scope using the <b>import</b> statement.
</p>

<h3>Import Syntax</h3>
<p>
Standard modules are imported by name, while local modules are imported using relative quoted string paths:
</p>

<pre><code>import math;
import strings;
import time;
import encoding;
import json;
import fs;
import testing;
import "utils.nx";
</code></pre>

<h3>Standard Library Modules</h3>

<p>
<b>1. math:</b> Mathematical constants and computational functions including <b>abs</b>, <b>sqrt</b>, <b>pow</b>, <b>floor</b>, <b>ceil</b>, <b>min</b>, <b>max</b>, <b>sin</b>, <b>cos</b>, and <b>random</b>.
</p>

<pre><code>import math;

var root:&lt;FL64&gt; = sqrt(64.0);
var power:&lt;FL64&gt; = pow(2.0, 8.0);
var minimum:&lt;FL64&gt; = min(10.5, 4.2);
</code></pre>

<p>
<b>2. strings:</b> String utilities and inspection functions including <b>upper</b>, <b>lower</b>, <b>trim</b>, <b>sub</b>, <b>contains</b>, <b>starts_with</b>, and <b>ends_with</b>. Builtin primitives such as <b>str_len</b>, <b>str_upper</b>, and <b>str_contains</b> are also directly accessible.
</p>

<pre><code>import strings;

var raw:&lt;STRING&gt; = "   NextLang Systems   ";
var cleaned:&lt;STRING&gt; = trim(raw);
var is_present:&lt;BOOL&gt; = contains(cleaned, "Systems");
</code></pre>

<p>
<b>3. time:</b> High-resolution system time retrieval and process sleeping with <b>time_now_ms</b>, <b>time_sleep_ms</b>, <b>now_ms</b>, and <b>sleep_ms</b>.
</p>

<pre><code>import time;

var t0:&lt;INT64&gt; = now_ms();
sleep_ms(50);
var t1:&lt;INT64&gt; = now_ms();
vout("Elapsed milliseconds:", t1 - t0);
</code></pre>

<p>
<b>4. encoding:</b> Data encoding and decoding with <b>hex_encode</b>, <b>hex_decode</b>, <b>base64_encode</b>, and <b>base64_decode</b>.
</p>

<pre><code>import encoding;

var encoded:&lt;STRING&gt; = to_base64("NextLang");
var decoded:&lt;STRING&gt; = from_base64(encoded);
vout("Base64:", encoded, "Restored:", decoded);
</code></pre>

<p>
<b>5. json:</b> Serialization and parsing functions with <b>json_parse</b>, <b>json_stringify</b>, <b>parse_json</b>, and <b>stringify_json</b>.
</p>

<pre><code>import json;

var payload:&lt;STRING&gt; = "{\"name\": \"NextLang\", \"version\": 1}";
var parsed:&lt;STRING&gt; = parse_json(payload);
vout("Parsed payload:", parsed);
</code></pre>

<p>
<b>6. fs:</b> Filesystem interactions with <b>read_file</b>, <b>write_file</b>, <b>exists</b>, <b>remove</b>, and <b>current_dir</b>.
</p>

<pre><code>import fs;

var path:&lt;STRING&gt; = "sample.txt";
write_file(path, "NextLang filesystem test.");
if exists(path) {
    var content:&lt;STRING&gt; = read_file(path);
    vout("File content:", content);
    remove(path);
}
</code></pre>

<hr>

<h2>14. Project and Package Management</h2>

<p>
NextLang includes a built-in project manager driven by the <b>nextlang.toml</b> manifest and reproducible <b>nextlang.lock</b> lockfile.
</p>

<h3>Project Manifest (nextlang.toml)</h3>
<p>
The project manifest declares package metadata, build entry point, and local or remote dependencies:
</p>

<pre><code>[package]
name = "my_service"
version = "0.1.0"
edition = "2024"
description = "NextLang microservice"

[build]
entry = "src/main.nx"
target = "native"

[dependencies]
network_core = { path = "../network_core" }
utility = { git = "https://github.com/example/utility.git" }
</code></pre>

<h3>Package Manager Commands</h3>
<p>
<b>nx init [dir]</b>: Initializes a NextLang package in the current or target directory.<br>
<b>nx new &lt;name&gt;</b>: Scaffolds a new project directory with manifest, starter source, and test suite.<br>
<b>nx add &lt;pkg&gt; [--path &lt;path&gt;] [--git &lt;url&gt;]</b>: Adds a dependency and resolves lockfile.<br>
<b>nx remove &lt;pkg&gt;</b>: Removes a dependency from manifest and lockfile.<br>
<b>nx update</b>: Re-evaluates dependency graphs and updates nextlang.lock.<br>
<b>nx tree</b>: Renders the dependency tree.<br>
</p>

<hr>

<h2>15. Testing Framework</h2>

<p>
Unit tests are declared using the <b>test</b> keyword. NextLang discovers tests across files in the <b>tests</b> directory and files ending in <b>_test.nx</b>.
</p>

<pre><code>import testing;
import math;

test "verify absolute values" {
    assert(abs(-42.0) == 42.0, "positive result expected");
    assert_eq("hello", "hello", "string equality check");
}

test "verify square root" {
    assert(sqrt(100.0) == 10.0, "sqrt(100) must equal 10");
}
</code></pre>

<p>
Execute tests using the CLI test command:
</p>

<pre><code>nx test
nx test --filter "square root"
</code></pre>

<p>
The test runner reports execution durations, pass/fail totals, and exits with a non-zero status code if any assertion fails.
</p>

<hr>

<h2>16. Source Code Formatter</h2>

<p>
NextLang provides an automated, deterministic code formatter. The formatter parses the source AST and produces canonical indentation and spacing:
</p>

<pre><code>// Format all source and test files in project
nx fmt

// Verify formatting in continuous integration without modifying files
nx fmt --check
</code></pre>
