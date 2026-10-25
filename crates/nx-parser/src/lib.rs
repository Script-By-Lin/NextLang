pub mod check;
pub mod modules;
pub mod parser;

pub use check::{ScopeChecker, Symbol, SymbolKind};
pub use modules::{get_stdlib_source, ModuleResolver};
pub use parser::Parser;

use nx_ast::Program;
use nx_diagnostics::DiagnosticBag;
use nx_lexer::Lexer;

/// Tokenizes and parses NextLang source code into an AST.
pub fn parse_source(source: &str, diagnostics: &mut DiagnosticBag) -> Program {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize(diagnostics);
    let mut parser = Parser::new(tokens, diagnostics);
    parser.parse_program()
}

/// Parses, resolves imports (including standard library and local files), and merges into a single AST.
pub fn parse_source_with_modules(
    entry_path: &std::path::Path,
    source: &str,
    diagnostics: &mut DiagnosticBag,
) -> Program {
    let mut resolver = ModuleResolver::new();
    resolver.resolve_entry(entry_path, source, diagnostics)
}

/// Tokenizes, parses, and performs scope/declaration validation.
pub fn check_source(source: &str, diagnostics: &mut DiagnosticBag) -> Program {
    let program = parse_source(source, diagnostics);
    let mut checker = ScopeChecker::new(diagnostics);
    checker.check_program(&program);
    program
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_hello() {
        let src = r#"
const PI:<FL64> = 3.14159;

var age:<INT32> = 25;
var name:<STRING> = "Alex";
var active:<BOOL> = true;

vout("Hello", name);

if age >= 18 & active {
    vout("Adult");
} otherwise age < 18 | not active {
    vout("Minor or inactive");
} else {
    vout("Other");
}

tem i:<INT32> = 0;

As i < 3 {
    vout(i);
    i++;
}

fr<tem j:<INT32> = 0: j < 3: j++> {
    vout(j);
}

Do {
    i--;
} As i > 0;
"#;
        let mut bag = DiagnosticBag::new();
        let _prog = check_source(src, &mut bag);
        assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
    }

    #[test]
    fn test_undeclared_variable_error() {
        let src = "age = 10;\n";
        let mut bag = DiagnosticBag::new();
        let _prog = check_source(src, &mut bag);
        assert!(bag.has_errors());
        let msgs: Vec<_> = bag.diagnostics().iter().map(|d| &d.message).collect();
        assert!(msgs.iter().any(|m| m.contains("undeclared variable 'age'")));
    }

    #[test]
    fn test_missing_semicolon_error() {
        let src = "var age:<INT32> = 10\n";
        let mut bag = DiagnosticBag::new();
        let _prog = check_source(src, &mut bag);
        assert!(bag.has_errors());
        let msgs: Vec<_> = bag.diagnostics().iter().map(|d| &d.message).collect();
        assert!(msgs.iter().any(|m| m.contains("expected ';'")));
    }

    #[test]
    fn test_const_reassignment_error() {
        let src = "const age:<INT32> = 10;\nage = 20;\n";
        let mut bag = DiagnosticBag::new();
        let _prog = check_source(src, &mut bag);
        assert!(bag.has_errors());
        let msgs: Vec<_> = bag.diagnostics().iter().map(|d| &d.message).collect();
        assert!(msgs
            .iter()
            .any(|m| m.contains("cannot reassign constant 'age'")));
    }

    #[test]
    fn test_generic_and_union_types() {
        let src = r#"
var numbers:<LIST<INT32>> = [1, 2, 3];
var users:<MAP<STRING, INT32>> = { "Alice": 100 };
var value:<INT32 | FL64> = 10;
"#;
        let mut bag = DiagnosticBag::new();
        let _prog = check_source(src, &mut bag);
        assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
    }
}
