pub mod compiler;
pub mod config;
pub mod linker;
pub mod llvm;
pub mod runtime;

pub use compiler::NativeCompiler;
pub use config::{Backend, CodegenConfig};
pub use linker::Linker;
pub use llvm::LlvmCompiler;

use nx_analyzer::SemanticAnalyzer;
use nx_diagnostics::DiagnosticBag;
use nx_nir::{optimize_module, print_module, AstLowering, NirModule};
use nx_parser::parse_source;
use std::path::Path;

/// Compiles a lowered NIR module into object file bytes using Cranelift.
pub fn compile_nir_to_object(nir: &NirModule, config: &CodegenConfig) -> Result<Vec<u8>, String> {
    let compiler = NativeCompiler::new(&nir.name, config)?;
    compiler.compile_module(nir)
}

/// Compiles a lowered NIR module directly to a native executable at `output_exe`.
/// Uses LLVM backend by default (`Backend::Llvm`), or Cranelift if specified.
pub fn compile_nir_to_executable(
    nir: &NirModule,
    output_exe: &Path,
    config: &CodegenConfig,
) -> Result<(), String> {
    let cache_dir = output_exe
        .parent()
        .map(|p| p.join(".nx_cache"))
        .unwrap_or_else(|| Path::new(".nx_cache").to_path_buf());

    std::fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("failed to create cache dir: {}", e))?;

    let stem = output_exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");

    match config.backend {
        Backend::Llvm => {
            let llvm = LlvmCompiler::new();
            let ir = llvm.emit_llvm_ir(nir);

            if config.emit_llvm {
                println!("{}", ir);
            }

            let ll_path = cache_dir.join(format!("{}.ll", stem));
            std::fs::write(&ll_path, &ir).map_err(|e| format!("failed to write LLVM IR: {}", e))?;

            // Verify LLVM IR with opt
            let _ = llvm.verify_ir(&ll_path);

            // Ensure native runtime object exists
            let runtime_obj = nx_native_runtime::get_or_build_runtime_obj(&cache_dir)?;

            // Compile & link via clang LLVM pipeline
            llvm.compile_and_link(&ll_path, &runtime_obj, output_exe, config.opt_level)?;
            Ok(())
        }
        Backend::Cranelift => {
            let obj_bytes = compile_nir_to_object(nir, config)?;
            let obj_path = cache_dir.join(format!("{}.o", stem));
            std::fs::write(&obj_path, obj_bytes)
                .map_err(|e| format!("failed to write object file: {}", e))?;

            let linker = Linker::new(&cache_dir)?;
            linker.link(&obj_path, output_exe)?;
            let _ = std::fs::remove_file(&obj_path);
            Ok(())
        }
    }
}

/// Fully parses, semantically checks, lowers to NIR, optimizes, and compiles NextLang source code to a native executable.
/// Returns Ok(true) if compilation succeeded, Ok(false) if syntax/semantic errors occurred, or Err on linker/system failure.
pub fn compile_source_to_executable(
    source: &str,
    output_exe: &Path,
    config: &CodegenConfig,
    diagnostics: &mut DiagnosticBag,
) -> Result<bool, String> {
    let program = parse_source(source, diagnostics);
    if diagnostics.has_errors() {
        return Ok(false);
    }

    let mut analyzer = SemanticAnalyzer::new(diagnostics);
    analyzer.analyze_program(&program);
    if diagnostics.has_errors() {
        return Ok(false);
    }

    let stem = output_exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");

    let lowering = AstLowering::new(stem);
    let mut nir = lowering.lower_program(&program);

    optimize_module(&mut nir, config.opt_level);

    if config.dump_nir {
        println!("{}", print_module(&nir));
    }

    compile_nir_to_executable(&nir, output_exe, config)?;
    Ok(true)
}

/// Parses, typechecks, lowers to NIR, optimizes, and emits LLVM IR textual representation.
pub fn emit_llvm_ir_from_source(
    source: &str,
    config: &CodegenConfig,
    diagnostics: &mut DiagnosticBag,
) -> Result<Option<String>, String> {
    let program = parse_source(source, diagnostics);
    if diagnostics.has_errors() {
        return Ok(None);
    }

    let mut analyzer = SemanticAnalyzer::new(diagnostics);
    analyzer.analyze_program(&program);
    if diagnostics.has_errors() {
        return Ok(None);
    }

    let lowering = AstLowering::new("module");
    let mut nir = lowering.lower_program(&program);

    optimize_module(&mut nir, config.opt_level);

    let llvm = LlvmCompiler::new();
    let ir = llvm.emit_llvm_ir(&nir);
    Ok(Some(ir))
}
