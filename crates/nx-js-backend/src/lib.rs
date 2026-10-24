pub mod config;
pub mod emitter;
pub mod runtime_helpers;

pub use config::{CompilerConfig, JsTarget, ModuleFormat};
pub use emitter::JsEmitter;

use nx_analyzer::SemanticAnalyzer;
use nx_ast::Program;
use nx_diagnostics::DiagnosticBag;
use nx_parser::parse_source;

/// Compiles an already-parsed and semantically-validated NextLang AST Program into JavaScript.
pub fn compile_program(program: &Program, config: CompilerConfig) -> String {
    let emitter = JsEmitter::new(config);
    emitter.emit_program(program)
}

/// Parses, semantically checks, and compiles NextLang source code into JavaScript.
/// Returns None if syntax or semantic errors were encountered.
pub fn compile_source(
    source: &str,
    config: CompilerConfig,
    diagnostics: &mut DiagnosticBag,
) -> Option<String> {
    let program = parse_source(source, diagnostics);
    if diagnostics.has_errors() {
        return None;
    }

    let mut analyzer = SemanticAnalyzer::new(diagnostics);
    analyzer.analyze_program(&program);
    if diagnostics.has_errors() {
        return None;
    }

    Some(compile_program(&program, config))
}
