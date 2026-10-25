use nx_codegen::*;
use nx_diagnostics::DiagnosticBag;
use std::process::Command;

use std::sync::atomic::{AtomicUsize, Ordering};
static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn compile_and_run(src: &str, input_stdin: Option<&str>) -> (String, String, bool) {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir().join(format!(
        "nx_test_{}_{}_{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t"),
        id
    ));
    let _ = std::fs::create_dir_all(&tmp_dir);
    let exe_path = tmp_dir.join(format!("test_bin_{}", id));

    let bag = DiagnosticBag::new();
    let mut bag = bag;
    let config = CodegenConfig {
        dump_nir: true,
        ..Default::default()
    };
    let ok = compile_source_to_executable(src, &exe_path, &config, &mut bag)
        .expect("compilation failed");
    assert!(ok, "diagnostics: {:?}", bag.diagnostics());
    assert!(exe_path.exists(), "executable not produced");

    let mut cmd = Command::new(&exe_path);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn native binary");

    if let Some(input) = input_stdin {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
    } else {
        drop(child.stdin.take());
    }

    let output = child.wait_with_output().expect("wait output");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let _ = std::fs::remove_file(&exe_path);
    let _ = std::fs::remove_dir_all(&tmp_dir);

    (stdout, stderr, output.status.success())
}

#[test]
fn test_native_hello_world() {
    let src = r#"
var name:<STRING> = "Alex";
vout("Hello", name);
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "Hello Alex");
}

#[test]
fn test_native_arithmetic_and_conditions() {
    let src = r#"
var a:<INT32> = 10;
var b:<INT32> = 25;
var c:<INT32> = a + b * 2;
vout("c =", c);

if c > 50 {
    vout("greater than 50");
} otherwise c == 50 {
    vout("equal to 50");
} else {
    vout("less than 50");
}
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("c = 60"));
    assert!(stdout.contains("greater than 50"));
}

#[test]
fn test_native_loops() {
    let src = r#"
var i:<INT32> = 0;
As i < 3 {
    vout("while:", i);
    i++;
}

fr<var j:<INT32> = 0: j < 3: j++> {
    vout("for:", j);
}
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("while: 0\nwhile: 1\nwhile: 2"));
    assert!(stdout.contains("for: 0\nfor: 1\nfor: 2"));
}

#[test]
fn test_native_match_case() {
    let src = r#"
var code:<INT32> = 200;
match code {
    case 200 { vout("OK"); }
    case 404 | 500 { vout("Error"); }
    case _ { vout("Unknown"); }
}
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "OK");
}

#[test]
fn test_native_functions_and_calls() {
    let src = r#"
fn multiply(x:<INT32>, y:<INT32>):<INT32> {
    return x * y;
}

var res:<INT32> = multiply(6, 7);
vout("Product:", res);
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert_eq!(stdout.trim(), "Product: 42");
}

#[test]
fn test_native_input_and_conversions() {
    let src = r#"
var name:<STRING> = input("Name: ");
var age_str:<STRING> = input("Age: ");
var age:<INT64> = to_int(age_str);

vout("Welcome", name);
vout("Age next year:", age + 1);
"#;
    let (stdout, stderr, success) = compile_and_run(src, Some("Bob\n30\n"));
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Welcome Bob"));
    assert!(stdout.contains("Age next year: 31"));
}

#[test]
fn test_native_oop() {
    let src = r#"
class Rectangle {
    var width:<FL64> = 0.0;
    var height:<FL64> = 0.0;

    public fn init(w:<FL64>, h:<FL64>):<VOID> {
        self.width = w;
        self.height = h;
        return;
    }

    public fn area():<FL64> {
        return self.width * self.height;
    }
}

var rect:<Rectangle> = Rectangle(4.0, 5.0);
var a:<FL64> = rect.area();
vout("Area:", a);
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Area: 20"));
}

#[test]
fn test_native_alloc_free() {
    let src = r#"
var ptr:<STRING> = alloc("Explicit Heap Buffer");
vout("Heap string:", ptr);
free(ptr);
vout("Freed successfully");
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Heap string: Explicit Heap Buffer"));
    assert!(stdout.contains("Freed successfully"));
}

#[test]
fn test_native_bitwise_operations() {
    let src = r#"
var a:<INT32> = 12; // 1100
var b:<INT32> = 5;  // 0101

var and_res:<INT32> = a & b;   // 0100 = 4
var or_res:<INT32> = a | b;    // 1101 = 13
var xor_res:<INT32> = a ^ b;   // 1001 = 9
var shl_res:<INT32> = a << 1;  // 24
var shr_res:<INT32> = a >> 1;  // 6
var not_res:<INT32> = ~a;      // -13

vout("AND:", and_res);
vout("OR:", or_res);
vout("XOR:", xor_res);
vout("SHL:", shl_res);
vout("SHR:", shr_res);
vout("NOT:", not_res);
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("AND: 4"));
    assert!(stdout.contains("OR: 13"));
    assert!(stdout.contains("XOR: 9"));
    assert!(stdout.contains("SHL: 24"));
    assert!(stdout.contains("SHR: 6"));
    assert!(stdout.contains("NOT: -13"));
}

#[test]
fn test_native_structs() {
    let src = r#"
struct Vector3 {
    x:<INT32>,
    y:<INT32>,
    z:<INT32>
}

var v:<Vector3> = Vector3 { x: 10, y: 20, z: 30 };
vout("Vector:", v.x, v.y, v.z);
var dot:<INT32> = v.x * v.x + v.y * v.y + v.z * v.z;
vout("Dot:", dot);
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Vector: 10 20 30"));
    assert!(stdout.contains("Dot: 1400"));
}

#[test]
fn test_native_ffi_and_unsafe() {
    let src = r#"
extern "C" {
    fn abs(n:<INT32>):<INT32>;
}

unsafe {
    var neg:<INT32> = -42;
    var pos:<INT32> = abs(neg);
    vout("Abs value:", pos);
}
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Abs value: 42"));
}

#[test]
fn test_native_file_io() {
    let src = r#"
var test_path:<STRING> = "/tmp/nx_test_phase8_file.txt";
var content:<STRING> = "NextLang Phase 8 LLVM Native I/O!";

var written:<BOOL> = file_write(test_path, content);
vout("Written:", written);

var exists:<BOOL> = file_exists(test_path);
vout("Exists:", exists);

var read_back:<STRING> = file_read_all(test_path);
vout("Read:", read_back);

var removed:<BOOL> = file_remove(test_path);
vout("Removed:", removed);

var exists_after:<BOOL> = file_exists(test_path);
vout("Exists after:", exists_after);
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Written: true"));
    assert!(stdout.contains("Exists: true"));
    assert!(stdout.contains("Read: NextLang Phase 8 LLVM Native I/O!"));
    assert!(stdout.contains("Removed: true"));
    assert!(stdout.contains("Exists after: false"));
}

#[test]
fn test_native_byte_buffers() {
    let src = r#"
var buf:<PTR> = buffer_new(16);
var sz:<UINT64> = buffer_size(buf);
vout("Buffer size:", sz);

buffer_set_u8(buf, 0, 0xDE);
buffer_set_u8(buf, 1, 0xAD);
var b0:<UINT8> = buffer_get_u8(buf, 0);
var b1:<UINT8> = buffer_get_u8(buf, 1);
vout("Bytes:", b0, b1);

buffer_set_u32_le(buf, 4, 0x12345678);
var val32:<UINT32> = buffer_get_u32_le(buf, 4);
vout("Val32:", val32);

buffer_free(buf);
vout("Buffer freed successfully");
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Buffer size: 16"));
    assert!(stdout.contains("Bytes: 222 173")); // 0xDE = 222, 0xAD = 173
    assert!(stdout.contains("Val32: 305419896")); // 0x12345678 = 305419896
    assert!(stdout.contains("Buffer freed successfully"));
}

#[test]
fn test_native_env_and_process() {
    let src = r#"
var pid:<INT32> = get_pid();
vout("PID valid:", pid > 0);

var ok_set:<BOOL> = env_set("NEXTLANG_SYS_TEST", "LLVM_NATIVE_OK");
vout("Set env:", ok_set);

var got:<STRING> = env_get("NEXTLANG_SYS_TEST");
vout("Got env:", got);
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("PID valid: true"));
    assert!(stdout.contains("Set env: true"));
    assert!(stdout.contains("Got env: LLVM_NATIVE_OK"));
}

#[test]
fn test_native_phase10_async_await() {
    let src = r#"
async fn compute_async(x:<INT32>):<INT32> {
    await sleep(5);
    return x * 3;
}

async fn main() {
    var fut:<FUTURE<INT32>> = compute_async(14);
    var res:<INT32> = await fut;
    vout("Native async result:", res);
}
"#;
    let (stdout, stderr, success) = compile_and_run(src, None);
    assert!(success, "stderr: {}", stderr);
    assert!(stdout.contains("Native async result: 42"));
}
