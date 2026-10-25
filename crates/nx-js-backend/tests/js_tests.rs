use nx_diagnostics::DiagnosticBag;
use nx_js_backend::{compile_source, CompilerConfig, JsTarget};
use std::io::Write;
use std::process::{Command, Stdio};

fn compile_to_js(source: &str) -> Option<String> {
    let mut bag = DiagnosticBag::new();
    let config = CompilerConfig::new().with_target(JsTarget::Node);
    let res = compile_source(source, config, &mut bag);
    if bag.has_errors() {
        eprintln!("Compile errors: {:?}", bag.diagnostics());
    }
    res
}

fn run_node(js_code: &str, stdin_input: Option<&str>) -> (String, String, bool) {
    let mut cmd = Command::new("node");
    cmd.arg("-e").arg(js_code);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if stdin_input.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }

    let mut child = cmd.spawn().expect("failed to spawn node process");
    if let Some(input) = stdin_input {
        if let Some(ref mut stdin) = child.stdin {
            let _ = stdin.write_all(input.as_bytes());
        }
    }

    let output = child
        .wait_with_output()
        .expect("failed to read node output");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

#[test]
fn test_compile_primitives_and_arithmetic() {
    let src = r#"
var a:<INT32> = 10;
var b:<INT32> = 20;
var c:<INT32> = a + b * 2;
vout("Arithmetic:", c);
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, _, success) = run_node(&js, None);
    assert!(success);
    assert_eq!(stdout.trim(), "Arithmetic: 50");
}

#[test]
fn test_compile_conditionals_and_loops() {
    let src = r#"
var count:<INT32> = 0;
As count < 3 {
    vout("while:", count);
    count++;
}

fr<tem j:<INT32> = 0: j < 2: j++> {
    vout("for:", j);
}

Do {
    count--;
} As count > 0;
vout("done count:", count);
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, _, success) = run_node(&js, None);
    assert!(success);
    assert_eq!(
        stdout.trim(),
        "while: 0\nwhile: 1\nwhile: 2\nfor: 0\nfor: 1\ndone count: 0"
    );
}

#[test]
fn test_compile_functions_and_entry_point() {
    let src = r#"
fn multiply(x:<INT32>, y:<INT32>):<INT32> {
    return x * y;
}

fn main():<VOID> {
    var res:<INT32> = multiply(6, 7);
    vout("Product:", res);
}
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, _, success) = run_node(&js, None);
    assert!(success);
    assert_eq!(stdout.trim(), "Product: 42");
}

#[test]
fn test_compile_oop_inheritance_and_super() {
    let src = r#"
class Animal {
    public var name:<STRING> = "";

    public fn init(name:<STRING>):<VOID> {
        self.name = name;
        return;
    }

    public fn speak():<STRING> {
        return self.name + " speaks";
    }
}

class Dog extends Animal {
    public fn init(name:<STRING>):<VOID> {
        self.name = name;
        return;
    }

    public fn speak():<STRING> {
        return super.speak() + ": woof";
    }
}

var d:<Dog> = Dog("Rover");
vout(d.speak());
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, _, success) = run_node(&js, None);
    assert!(success);
    assert_eq!(stdout.trim(), "Rover speaks: woof");
}

#[test]
fn test_compile_match_case() {
    let src = r#"
fn check_status(code:<INT32>):<STRING> {
    match code {
        case 200 {
            return "OK";
        }
        case 201 | 202 {
            return "Accepted";
        }
        case c if c >= 400 & c < 500 {
            return "Client Error";
        }
        case _ {
            return "Other";
        }
    }
    return "None";
}

vout(check_status(200));
vout(check_status(202));
vout(check_status(404));
vout(check_status(500));
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, _, success) = run_node(&js, None);
    assert!(success);
    assert_eq!(stdout.trim(), "OK\nAccepted\nClient Error\nOther");
}

#[test]
fn test_compile_collections_and_len() {
    let src = r#"
var nums:<LIST<INT32>> = [10, 20, 30];
var dict:<MAP<STRING, INT32>> = {"x": 100, "y": 200};
var msg:<STRING> = "Hello NextLang";

vout("list len:", len(nums));
vout("dict len:", len(dict));
vout("str len:", len(msg));
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, _, success) = run_node(&js, None);
    assert!(success);
    assert_eq!(stdout.trim(), "list len: 3\ndict len: 2\nstr len: 14");
}

#[test]
fn test_compile_input_and_conversions() {
    let src = r#"
var name:<STRING> = input("Name: ");
var age_str:<STRING> = input("Age: ");
var age:<INT64> = to_int(age_str);
var score:<FL64> = to_float("99.5");

vout("Welcome", name);
vout("Age next year:", age + 1);
vout("Score:", score);
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, _, success) = run_node(&js, Some("Alice\n25\n"));
    assert!(success);
    assert!(stdout.contains("Name: Age: Welcome Alice"));
    assert!(stdout.contains("Age next year: 26"));
    assert!(stdout.contains("Score: 99.5"));
}

#[test]
fn test_compile_semantic_error_rejected() {
    let src = r#"
var x:<INT32> = "invalid string type assignment";
"#;
    let js = compile_to_js(src);
    assert!(
        js.is_none(),
        "expected semantic error to reject compilation"
    );
}

#[test]
fn test_compile_phase10_async_await() {
    let src = r#"
async fn fetch_msg():<STRING> {
    await sleep(5);
    return "Async JS Hello";
}

async fn main():<VOID> {
    var msg:<STRING> = await fetch_msg();
    vout("Message:", msg);
}
"#;
    let js = compile_to_js(src).expect("compilation failed");
    let (stdout, stderr, success) = run_node(&js, None);
    assert!(success, "node error: {}", stderr);
    assert!(stdout.contains("Message: Async JS Hello"));
}
