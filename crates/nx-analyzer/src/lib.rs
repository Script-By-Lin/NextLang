pub mod analyzer;
pub mod oop;
pub mod symbol;
pub mod types;

pub use analyzer::SemanticAnalyzer;
pub use oop::{
    ClassHierarchy, ClassMethodSymbol, ClassSymbol, InterfaceMethodSymbol, InterfaceSymbol,
};
pub use symbol::{Symbol, SymbolKind, SymbolTable};
pub use types::SemType;

use nx_ast::Program;
use nx_diagnostics::DiagnosticBag;
use nx_parser::parse_source;

/// Performs comprehensive semantic analysis and type checking on an AST.
pub fn analyze(program: &Program, diagnostics: &mut DiagnosticBag) {
    let mut analyzer = SemanticAnalyzer::new(diagnostics);
    analyzer.analyze_program(program);
}

/// Tokenizes, parses, and runs semantic analysis and type checking on source text.
pub fn analyze_source(source: &str, diagnostics: &mut DiagnosticBag) -> Program {
    let program = parse_source(source, diagnostics);
    if !diagnostics.has_errors() {
        analyze(&program, diagnostics);
    }
    program
}
