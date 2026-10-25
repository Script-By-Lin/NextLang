use std::process::Command;

fn nx_bin() -> Command {
    let mut path = std::env::current_exe().expect("failed to get test binary path");
    path.pop(); // remove binary name
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("nx");
    let mut cmd = Command::new(path);
    cmd.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    cmd
}

#[test]
fn test_cli_version() {
    let output = nx_bin()
        .arg("--version")
        .output()
        .expect("failed to execute nx");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nx 0.1.0"));
}

#[test]
fn test_cli_help() {
    let output = nx_bin()
        .arg("--help")
        .output()
        .expect("failed to execute nx");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("tokenize"));
    assert!(stdout.contains("parse"));
    assert!(stdout.contains("check"));
}

#[test]
fn test_cli_check_valid_hello() {
    let output = nx_bin()
        .arg("check")
        .arg("examples/hello.nx")
        .output()
        .expect("failed to execute nx");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Check passed: no syntax errors found in examples/hello.nx"));
}

#[test]
fn test_cli_parse_valid_hello() {
    let output = nx_bin()
        .arg("parse")
        .arg("examples/hello.nx")
        .output()
        .expect("failed to execute nx");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Program"));
    assert!(stdout.contains("VarDecl (const)"));
    assert!(stdout.contains("ForLoop (fr)"));
}

#[test]
fn test_cli_tokenize_valid_hello() {
    let output = nx_bin()
        .arg("tokenize")
        .arg("examples/hello.nx")
        .output()
        .expect("failed to execute nx");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tokens for 'examples/hello.nx'"));
}

#[test]
fn test_cli_check_all_examples() {
    for example in &["variables.nx", "conditions.nx", "loops.nx", "oop.nx"] {
        let path = format!("examples/{}", example);
        let output = nx_bin()
            .arg("check")
            .arg(&path)
            .output()
            .expect("failed to execute nx");

        assert!(
            output.status.success(),
            "failed on example {}: {}",
            example,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_cli_run_hello() {
    let output = nx_bin()
        .arg("run")
        .arg("examples/hello.nx")
        .output()
        .expect("failed to execute nx run");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Hello Alex"));
}

#[test]
fn test_cli_run_with_time_flag() {
    let output = nx_bin()
        .arg("run")
        .arg("examples/hello.nx")
        .arg("--time")
        .output()
        .expect("failed to execute nx run --time");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify stdout has program output without timing info
    assert!(stdout.contains("Hello Alex"));
    assert!(!stdout.contains("Execution time"));

    // Verify stderr contains the monotonic Execution time metric
    assert!(stderr.contains("Execution time:"));
    assert!(
        stderr.contains("ns")
            || stderr.contains("µs")
            || stderr.contains("ms")
            || stderr.contains("s")
    );
}

#[test]
fn test_cli_run_runtime_error_with_timing() {
    let abs_err_file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/div_zero_test.nx"
    );
    std::fs::write(
        abs_err_file,
        "var zero:<INT32> = 0;\nvar a:<INT32> = 10 / zero;\n",
    )
    .expect("write test file");

    let output = nx_bin()
        .arg("run")
        .arg("examples/div_zero_test.nx")
        .arg("--time")
        .output()
        .expect("failed to execute nx run with error");

    let _ = std::fs::remove_file(abs_err_file);

    // Should fail with non-zero exit status
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Timing information must be displayed even on runtime error per exec_timing.md
    assert!(stderr.contains("Execution time:"));
    assert!(stderr.contains("division by zero"));
}

#[test]
fn test_cli_compile_js_stdout() {
    let output = nx_bin()
        .arg("compile-js")
        .arg("examples/hello.nx")
        .arg("--stdout")
        .output()
        .expect("failed to execute nx compile-js --stdout");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("console.log(\"Hello\", name);"));
}

#[test]
fn test_cli_compile_js_file_and_run_node() {
    let out_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/test_hello.js");
    let output = nx_bin()
        .arg("compile-js")
        .arg("examples/hello.nx")
        .arg("-o")
        .arg(out_file)
        .output()
        .expect("failed to execute nx compile-js -o");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::path::Path::new(out_file).exists());

    let node_output = Command::new("node")
        .arg(out_file)
        .output()
        .expect("failed to execute node on compiled js");

    let _ = std::fs::remove_file(out_file);

    assert!(node_output.status.success());
    let stdout = String::from_utf8_lossy(&node_output.stdout);
    assert!(stdout.contains("Hello Alex"));
}

#[test]
fn test_cli_compile_js_semantic_error_rejected() {
    let bad_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/bad_type_test.nx");
    std::fs::write(bad_file, "var x:<INT32> = \"mismatch\";\n").expect("write bad file");

    let output = nx_bin()
        .arg("compile-js")
        .arg(bad_file)
        .arg("--stdout")
        .output()
        .expect("failed to execute nx compile-js on bad file");

    let _ = std::fs::remove_file(bad_file);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("NX-TYPE-001") || stderr.contains("cannot be assigned"));
}

#[test]
fn test_cli_build_and_run_native() {
    let out_file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/cli_test_hello_bin"
    );
    let _ = std::fs::remove_file(out_file);

    let output = nx_bin()
        .arg("build")
        .arg("examples/hello.nx")
        .arg("-o")
        .arg(out_file)
        .arg("--time")
        .output()
        .expect("failed to execute nx build");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::path::Path::new(out_file).exists());

    let run_output = Command::new(out_file)
        .output()
        .expect("failed to execute compiled native binary");

    let _ = std::fs::remove_file(out_file);

    assert!(run_output.status.success());
    let stdout = String::from_utf8_lossy(&run_output.stdout);
    assert!(stdout.contains("Hello Alex"));
    assert!(stdout.contains("Adult"));
}

#[test]
fn test_cli_run_native_source() {
    let output = nx_bin()
        .arg("run-native")
        .arg("examples/hello.nx")
        .arg("--time")
        .output()
        .expect("failed to execute nx run-native");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Hello Alex"));
    assert!(stderr.contains("Execution time:"));
}

#[test]
fn test_cli_dump_nir() {
    let output = nx_bin()
        .arg("dump-nir")
        .arg("examples/hello.nx")
        .arg("--opt-level")
        .arg("1")
        .output()
        .expect("failed to execute nx dump-nir");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("; NIR Module:"));
    assert!(stdout.contains("fn main() -> I32"));
}

#[test]
fn test_cli_build_semantic_error_rejected() {
    let bad_file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/bad_native_test.nx"
    );
    std::fs::write(bad_file, "var x:<INT32> = \"mismatch\";\n").expect("write bad file");

    let output = nx_bin()
        .arg("build")
        .arg(bad_file)
        .output()
        .expect("failed to execute nx build on bad file");

    let _ = std::fs::remove_file(bad_file);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("NX-TYPE-001") || stderr.contains("cannot be assigned"));
}

#[test]
fn test_cli_dump_llvm() {
    let output = nx_bin()
        .arg("dump-llvm")
        .arg("examples/hello.nx")
        .arg("--opt-level")
        .arg("1")
        .output()
        .expect("failed to execute nx dump-llvm");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("target triple ="));
    assert!(stdout.contains("define i32 @main()"));
    assert!(stdout.contains("declare void @nx_rt_vout_str(ptr)"));
}

#[test]
fn test_cli_build_with_emit_llvm() {
    let out_file = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../target/test_llvm_cli_bin"
    );
    let output = nx_bin()
        .arg("build")
        .arg("examples/hello.nx")
        .arg("-o")
        .arg(out_file)
        .arg("--emit-llvm")
        .output()
        .expect("failed to execute nx build with --emit-llvm");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("target triple ="));
    assert!(stdout.contains("define i32 @main()"));
    assert!(std::path::Path::new(out_file).exists());

    let bin_output = Command::new(out_file)
        .output()
        .expect("failed to execute built binary");
    let _ = std::fs::remove_file(out_file);

    assert!(bin_output.status.success());
    let bin_stdout = String::from_utf8_lossy(&bin_output.stdout);
    assert!(bin_stdout.contains("Hello Alex"));
}

#[test]
fn test_cli_phase9_project_lifecycle() {
    let temp_root = std::env::temp_dir().join(format!("nx_test_proj_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("create temp dir");

    let proj_dir = temp_root.join("sample_pkg");

    // 1. nx new sample_pkg
    let output = nx_bin()
        .current_dir(&temp_root)
        .arg("new")
        .arg("sample_pkg")
        .output()
        .expect("nx new");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(proj_dir.join("nextlang.toml").exists());
    assert!(proj_dir.join("nextlang.lock").exists());
    assert!(proj_dir.join("src/main.nx").exists());
    assert!(proj_dir.join("tests/main_test.nx").exists());

    // 2. nx check
    let chk = nx_bin()
        .current_dir(&proj_dir)
        .arg("check")
        .output()
        .expect("nx check");
    assert!(
        chk.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&chk.stderr)
    );

    // 3. nx test
    let test_out = nx_bin()
        .current_dir(&proj_dir)
        .arg("test")
        .output()
        .expect("nx test");
    assert!(
        test_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&test_out.stderr)
    );
    let test_stdout = String::from_utf8_lossy(&test_out.stdout);
    assert!(test_stdout.contains("1 passed; 0 failed"));

    // 4. nx run
    let run_out = nx_bin()
        .current_dir(&proj_dir)
        .arg("run")
        .output()
        .expect("nx run");
    assert!(
        run_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run_out.stderr)
    );
    let run_stdout = String::from_utf8_lossy(&run_out.stdout);
    assert!(run_stdout.contains("Hello from NextLang!"));
    assert!(run_stdout.contains("sqrt(16.0) =  4"));

    // 5. nx build --target js
    let b_js = nx_bin()
        .current_dir(&proj_dir)
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("-o")
        .arg("dist/main.js")
        .output()
        .expect("nx build --target js");
    assert!(
        b_js.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&b_js.stderr)
    );
    assert!(proj_dir.join("dist/main.js").exists());

    // 6. nx build --target native
    let b_nat = nx_bin()
        .current_dir(&proj_dir)
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("-o")
        .arg("dist/main_bin")
        .output()
        .expect("nx build --target native");
    assert!(
        b_nat.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&b_nat.stderr)
    );
    assert!(proj_dir.join("dist/main_bin").exists());

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn test_cli_phase9_package_management() {
    let temp_root = std::env::temp_dir().join(format!("nx_pkg_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("create temp dir");

    let output = nx_bin()
        .current_dir(&temp_root)
        .arg("init")
        .output()
        .expect("nx init");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(temp_root.join("nextlang.toml").exists());

    // nx add helper --path ../helper
    let add_out = nx_bin()
        .current_dir(&temp_root)
        .arg("add")
        .arg("helper")
        .arg("--path")
        .arg("../helper")
        .output()
        .expect("nx add");
    assert!(
        add_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&add_out.stderr)
    );

    // nx tree
    let tree_out = nx_bin()
        .current_dir(&temp_root)
        .arg("tree")
        .output()
        .expect("nx tree");
    assert!(
        tree_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&tree_out.stderr)
    );
    let tree_stdout = String::from_utf8_lossy(&tree_out.stdout);
    assert!(tree_stdout.contains("helper"));

    // nx update
    let upd_out = nx_bin()
        .current_dir(&temp_root)
        .arg("update")
        .output()
        .expect("nx update");
    assert!(
        upd_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&upd_out.stderr)
    );

    // nx remove helper
    let rem_out = nx_bin()
        .current_dir(&temp_root)
        .arg("remove")
        .arg("helper")
        .output()
        .expect("nx remove");
    assert!(
        rem_out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&rem_out.stderr)
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn test_cli_phase9_fmt_and_check() {
    let temp_file = std::env::temp_dir().join(format!("nx_fmt_test_{}.nx", std::process::id()));
    let ugly_code = "fn   add( a:<INT> , b:<INT> ):<INT>   { return   a+b ; }   \n";
    std::fs::write(&temp_file, ugly_code).expect("write file");

    // nx fmt --check should detect difference and exit with code 1
    let check_fail = nx_bin()
        .arg("fmt")
        .arg(&temp_file)
        .arg("--check")
        .output()
        .expect("nx fmt --check");
    assert!(!check_fail.status.success());

    // nx fmt formats the file
    let fmt_exec = nx_bin()
        .arg("fmt")
        .arg(&temp_file)
        .output()
        .expect("nx fmt");
    assert!(fmt_exec.status.success());

    // nx fmt --check now passes
    let check_pass = nx_bin()
        .arg("fmt")
        .arg(&temp_file)
        .arg("--check")
        .output()
        .expect("nx fmt --check again");
    assert!(check_pass.status.success());

    let _ = std::fs::remove_file(temp_file);
}
