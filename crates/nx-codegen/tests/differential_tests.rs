use nx_codegen::*;
use nx_diagnostics::DiagnosticBag;
use nx_nir::OptLevel;
use nx_parser::parse_source;
use nx_runtime::eval_program_captured;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static DIFF_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run_differential_test(rel_path: &str, expected_stdout: Option<&str>) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let full_path = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(rel_path);

    let src = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", full_path.display(), e));

    // 1. Interpreter execution
    let mut interp_bag = DiagnosticBag::new();
    let interp_prog = parse_source(&src, &mut interp_bag);
    assert!(
        !interp_bag.has_errors(),
        "parser errors: {:?}",
        interp_bag.diagnostics()
    );

    let (_, interp_output) =
        eval_program_captured(&interp_prog).expect("interpreter execution failed");

    // 2. Native compilation and execution
    let id = DIFF_COUNTER.fetch_add(1, Ordering::SeqCst);
    let tmp_dir = std::env::temp_dir().join(format!("nx_diff_{}_{}", std::process::id(), id));
    let _ = std::fs::create_dir_all(&tmp_dir);
    let exe_path = tmp_dir.join(format!("diff_bin_{}", id));

    let mut codegen_bag = DiagnosticBag::new();
    let config = CodegenConfig {
        opt_level: OptLevel::O2,
        ..Default::default()
    };

    let ok = compile_source_to_executable(&src, &exe_path, &config, &mut codegen_bag)
        .expect("native compilation failed");
    assert!(ok, "codegen diagnostics: {:?}", codegen_bag.diagnostics());

    let output = Command::new(&exe_path)
        .output()
        .expect("failed to execute native binary");

    let _ = std::fs::remove_file(&exe_path);
    let _ = std::fs::remove_dir_all(&tmp_dir);

    assert!(output.status.success(), "native binary exited with failure");
    let native_stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if let Some(expected) = expected_stdout {
        assert!(
            native_stdout.contains(expected),
            "native output missing expected text"
        );
    } else {
        // Compare trimmed outputs
        assert_eq!(
            interp_output.trim(),
            native_stdout.trim(),
            "differential mismatch for {}",
            rel_path
        );
    }
}

#[test]
fn test_differential_hello() {
    run_differential_test("examples/hello.nx", None);
}

#[test]
fn test_differential_conditions() {
    run_differential_test("examples/conditions.nx", None);
}

#[test]
fn test_differential_loops() {
    run_differential_test("examples/loops.nx", None);
}

#[test]
fn test_differential_match_case() {
    run_differential_test("examples/match_case.nx", None);
}

#[test]
fn test_differential_oop() {
    run_differential_test("examples/oop.nx", None);
}

#[test]
fn test_differential_ownership() {
    // Ownership has different simulated heap representation in interpreter,
    // so verify expected native output assertions.
    run_differential_test(
        "examples/ownership.nx",
        Some("Successfully freed heap value!"),
    );
}

#[test]
fn test_differential_modern_syntax() {
    run_differential_test("examples/modern_syntax.nx", None);
}
