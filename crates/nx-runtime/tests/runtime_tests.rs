use nx_diagnostics::DiagnosticBag;
use nx_parser::parse_source;
use nx_runtime::eval_program_captured;

fn run_nx(source: &str) -> (Result<nx_runtime::Value, nx_runtime::RuntimeError>, String) {
    let mut bag = DiagnosticBag::new();
    let program = parse_source(source, &mut bag);
    assert!(!bag.has_errors(), "Parse errors: {:?}", bag.diagnostics());

    eval_program_captured(&program)
        .map_or_else(|err| (Err(err), String::new()), |(val, out)| (Ok(val), out))
}

#[test]
fn test_runtime_hello_world() {
    let src = r#"
vout("Hello, NextLang!");
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "Hello, NextLang!");
}

#[test]
fn test_runtime_arithmetic_and_variables() {
    let src = r#"
var a:<INT32> = 10;
var b:<INT32> = 20;
var c:<INT32> = a + b * 2;
vout(c);
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "50");
}

#[test]
fn test_runtime_loops() {
    let src = r#"
var count:<INT32> = 0;
As count < 3 {
    count++;
}
vout(count);
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "3");
}

#[test]
fn test_runtime_functions_and_recursion() {
    let src = r#"
fn factorial(n:<INT32>):<INT32> {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

vout(factorial(5));
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "120");
}

#[test]
fn test_runtime_oop_class_and_methods() {
    let src = r#"
class Counter {
    var val:<INT32> = 0;

    fn inc():<VOID> {
        self.val++;
    }

    fn get():<INT32> {
        return self.val;
    }
}

var c:<Counter> = Counter();
c.inc();
c.inc();
vout(c.get());
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "2");
}

#[test]
fn test_runtime_match_literal_and_wildcard() {
    let src = r#"
fn classify(x:<INT32>):<STRING> {
    match x {
        case 1 {
            return "one";
        }
        case 2 {
            return "two";
        }
        case _ {
            return "many";
        }
    }
    return "unknown";
}

vout(classify(1));
vout(classify(2));
vout(classify(10));
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "one\ntwo\nmany");
}

#[test]
fn test_runtime_match_alternative_and_guard() {
    let src = r#"
fn test_match(x:<INT32>):<STRING> {
    match x {
        case 1 | 2 {
            return "small";
        }
        case v if v > 10 {
            return "large";
        }
        case _ {
            return "medium";
        }
    }
    return "none";
}

vout(test_match(1));
vout(test_match(2));
vout(test_match(5));
vout(test_match(20));
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "small\nsmall\nmedium\nlarge");
}

#[test]
fn test_runtime_alloc_and_free() {
    let src = r#"
var s:<STRING> = alloc("manual heap memory");
vout(s);
free(s);
vout("freed successfully");
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert!(out.contains("freed successfully"));
}

#[test]
fn test_runtime_oop_constructor_and_super_dispatch() {
    let src = r#"
class Animal {
    public var name:<STRING> = "";

    public fn init(n:<STRING>):<VOID> {
        self.name = n;
        return;
    }

    public fn speak():<STRING> {
        return self.name + " speaks";
    }
}

class Dog extends Animal {
    public fn init(n:<STRING>):<VOID> {
        self.name = n;
        return;
    }

    public fn speak():<STRING> {
        return super.speak() + ": woof";
    }
}

var d:<Dog> = Dog("Rover");
vout(d.speak());
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "Rover speaks: woof");
}

#[test]
fn test_runtime_logical_short_circuit() {
    let src = r#"
var zero:<INT32> = 0;
# If short-circuiting works, 10 / zero is never executed
if false & (10 / zero == 0) {
    vout("should not reach");
} else {
    vout("AND short-circuited");
}

if true | (10 / zero == 0) {
    vout("OR short-circuited");
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "AND short-circuited\nOR short-circuited");
}

#[test]
fn test_runtime_main_entry_point() {
    let src = r#"
fn compute(x:<INT32>):<INT32> {
    return x * 2;
}

fn main():<VOID> {
    var ans:<INT32> = compute(21);
    vout("Answer:", ans);
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "Answer: 42");
}

#[test]
fn test_runtime_recursion_depth_limit() {
    let mut bag = DiagnosticBag::new();
    let src = r#"
fn infinite_recurse(n:<INT32>):<INT32> {
    return infinite_recurse(n + 1);
}

infinite_recurse(1);
"#;
    let prog = parse_source(src, &mut bag);
    let mut interp = nx_runtime::Interpreter::new(Vec::new()).with_max_call_depth(50);
    let res = interp.eval_program(&prog);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.message.contains("maximum recursion depth"));
}

#[test]
fn test_runtime_step_limit() {
    let mut bag = DiagnosticBag::new();
    let src = r#"
var i:<INT32> = 0;
As i < 1000 {
    i++;
}
"#;
    let prog = parse_source(src, &mut bag);
    let mut interp = nx_runtime::Interpreter::new(Vec::new()).with_max_steps(20);
    let res = interp.eval_program(&prog);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(err.message.contains("step limit exceeded"));
}

#[test]
fn test_runtime_len_builtin() {
    let src = r#"
var list:<LIST<INT32>> = [10, 20, 30, 40];
var map:<MAP<STRING, INT32>> = {"a": 1, "b": 2};
var str:<STRING> = "NextLang";

vout("list len:", len(list));
vout("map len:", len(map));
vout("str len:", len(str));
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok());
    assert_eq!(out.trim(), "list len: 4\nmap len: 2\nstr len: 8");
}

#[test]
fn test_runtime_input_and_conversion() {
    let mut bag = DiagnosticBag::new();
    let src = r#"
var name:<STRING> = input("Name: ");
var age_str:<STRING> = input("Age: ");
var age:<INT32> = to_int(age_str);
var score:<FL64> = to_float("98.5");
vout("Welcome", name);
vout("Age next year:", age + 1);
vout("Score:", score);
"#;
    let prog = parse_source(src, &mut bag);
    let mock_input = "Alice\n25\n";
    let stdout_buf = Vec::new();
    let mut interp = nx_runtime::Interpreter::new(stdout_buf)
        .with_stdin(Box::new(std::io::Cursor::new(mock_input.as_bytes())));
    let res = interp.eval_program(&prog);
    assert!(res.is_ok());
    let out = String::from_utf8_lossy(&interp.stdout).to_string();
    assert!(out.contains("Name: Age: Welcome Alice"));
    assert!(out.contains("Age next year: 26"));
    assert!(out.contains("Score: 98.5"));
}

#[test]
fn test_runtime_systems_and_bitwise() {
    let src = r#"
var a:<INT32> = 5 ^ 3;
var b:<INT32> = 1 << 4;
var c:<INT32> = 32 >> 2;
var d:<INT32> = ~0;
var e:<INT32> = 12 & 10;
var f:<INT32> = 12 | 10;
vout("xor:", a);
vout("shl:", b);
vout("shr:", c);
vout("not:", d);
vout("and:", e);
vout("or:", f);

var buf:<PTR> = buffer_new(8);
buffer_set_u32_le(buf, 0, 1337);
var val:<INT32> = buffer_get_u32_le(buf, 0);
vout("buf val:", val);
buffer_free(buf);
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("xor: 6"));
    assert!(out.contains("shl: 16"));
    assert!(out.contains("shr: 8"));
    assert!(out.contains("not: -1"));
    assert!(out.contains("and: 8"));
    assert!(out.contains("or: 14"));
    assert!(out.contains("buf val: 1337"));
}

#[test]
fn test_runtime_struct_and_unsafe() {
    let src = r#"
struct Point {
    x:<INT32>,
    y:<INT32>
}

var p:<Point> = Point { x: 10, y: 20 };
vout("pt:", p.x, p.y);
p.x = 99;
vout("pt updated:", p.x, p.y);

unsafe {
    var pid:<INT32> = get_pid();
    var ok:<BOOL> = pid > 0;
    vout("valid pid:", ok);
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("pt: 10 20"));
    assert!(out.contains("pt updated: 99 20"));
    assert!(out.contains("valid pid: true"));
}

#[test]
fn test_runtime_phase9_stdlib_and_assertions() {
    let src = r#"
var s:<FL64> = math_sqrt(16.0);
var p:<FL64> = math_pow(2.0, 3.0);
var upper:<STRING> = str_upper("hello nextlang");
var hex:<STRING> = hex_encode("abc");
var b64:<STRING> = base64_encode("hello");
var now:<INT64> = time_now_ms();

vout("sqrt:", s);
vout("pow:", p);
vout("upper:", upper);
vout("hex:", hex);
vout("b64:", b64);
vout("time valid:", now > 0);

test "assert works" {
    assert(s == 4.0, "sqrt of 16 is 4");
    assert_eq(upper, "HELLO NEXTLANG", "str_upper matches");
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("sqrt: 4"));
    assert!(out.contains("pow: 8"));
    assert!(out.contains("upper: HELLO NEXTLANG"));
    assert!(out.contains("hex: 616263"));
    assert!(out.contains("b64: aGVsbG8="));
    assert!(out.contains("time valid: true"));
}

#[test]
fn test_phase10_async_function_and_await() {
    let src = r#"
async fn compute(a:<INT32>, b:<INT32>):<INT32> {
    await sleep(5);
    return a * b;
}

async fn main():<VOID> {
    var fut:<FUTURE<INT32>> = compute(6, 7);
    var res:<INT32> = await fut;
    vout("async compute:", res);
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("async compute: 42"));
}

#[test]
fn test_phase10_async_sleep_concurrency() {
    let src = r#"
async fn main():<VOID> {
    var t1:<FUTURE<VOID>> = async_sleep(10);
    var t2:<FUTURE<VOID>> = async_sleep(10);
    await t1;
    await t2;
    vout("concurrent sleeps finished");
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("concurrent sleeps finished"));
}

#[test]
fn test_phase10_task_spawn_and_join() {
    let src = r#"
async fn work(val:<STRING>):<STRING> {
    await sleep(5);
    return "done:" + val;
}

async fn main():<VOID> {
    var task:<FUTURE<STRING>> = task_spawn(work("task1"));
    var res:<STRING> = task_join(task);
    vout("result:", res);
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("result: done:task1"));
}

#[test]
fn test_phase10_task_cancel_and_timeout() {
    let src = r#"
async fn main():<VOID> {
    var long_task:<FUTURE<VOID>> = async_sleep(5000);
    var cancelled:<BOOL> = task_cancel(long_task);
    vout("cancelled:", cancelled);

    var fast_task:<FUTURE<VOID>> = async_sleep(5);
    var timed:<FUTURE<VOID>> = timeout(500, fast_task);
    await timed;
    vout("timeout completed successfully");
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("cancelled: true"));
    assert!(out.contains("timeout completed successfully"));
}

#[test]
fn test_phase10_tcp_loopback() {
    let src = r#"
async fn main():<VOID> {
    var server_fut:<FUTURE<INT64>> = tcp_listen("127.0.0.1:0");
    var server_fd:<INT64> = await server_fut;
    vout("server fd ready:", server_fd > 0);
    tcp_close(server_fd);
    vout("tcp loopback closed");
}
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("server fd ready: true"));
    assert!(out.contains("tcp loopback closed"));
}

#[test]
fn test_modern_syntax_execution() {
    let src = r#"
var age: Int32 = 25;
const PI: Float64 = 3.14159;

fn add(a: Int32, b: Int32) -> Int32 {
    return a + b;
}

fn greet(name: String) -> Void {
    vout("Hello", name);
}

class User {
    var name: String;
    var age: Int32;

    fn init(name: String, age: Int32) -> Void {
        self.name = name;
        self.age = age;
    }

    fn introduce() -> Void {
        vout("Hello, I am", self.name, "and age is", self.age);
    }
}

var user = User("Alex", 25);
user.introduce();

var sum = add(age, 10);
vout("sum is", sum);
greet("NextLang");
vout("PI is", PI);
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("Hello, I am Alex and age is 25"));
    assert!(out.contains("sum is 35"));
    assert!(out.contains("Hello NextLang"));
    assert!(out.contains("PI is 3.14159"));
}

#[test]
fn test_runtime_phase11_nextapi_builtins() {
    let src = r#"
var parsed: String = http_parse("GET /users/42?page=1 HTTP/1.1\r\nHost: localhost\r\nX-Custom: hello\r\n\r\n");
vout("http_parse valid:", str_contains(parsed, "\"valid\":true"));
vout("http_parse method:", str_contains(parsed, "\"method\":\"GET\""));
vout("http_parse path:", str_contains(parsed, "\"path\":\"/users/42\""));
vout("http_parse query:", str_contains(parsed, "\"query\":\"page=1\""));

var msg_200: String = http_status_message(200);
var msg_404: String = http_status_message(404);
var msg_422: String = http_status_message(422);
vout("status 200:", msg_200);
vout("status 404:", msg_404);
vout("status 422:", msg_422);

var s_int: String = int_to_str(1337);
vout("int_to_str:", s_int);
"#;
    let (res, out) = run_nx(src);
    assert!(res.is_ok(), "runtime error: {:?}", res.err());
    assert!(out.contains("http_parse valid: true"));
    assert!(out.contains("http_parse method: true"));
    assert!(out.contains("http_parse path: true"));
    assert!(out.contains("http_parse query: true"));
    assert!(out.contains("status 200: OK"));
    assert!(out.contains("status 404: Not Found"));
    assert!(out.contains("status 422: Unprocessable Entity"));
    assert!(out.contains("int_to_str: 1337"));
}
