pub mod ir;
pub mod lower;
pub mod opt;
pub mod printer;

pub use ir::*;
pub use lower::AstLowering;
pub use opt::{optimize_module, OptLevel};
pub use printer::print_module;

#[cfg(test)]
mod tests {
    use super::*;
    use nx_diagnostics::DiagnosticBag;
    use nx_parser::parse_source;

    #[test]
    fn test_lower_arithmetic_and_print() {
        let src = r#"
var a:<INT32> = 10 + 20;
vout(a);
"#;
        let mut bag = DiagnosticBag::new();
        let prog = parse_source(src, &mut bag);
        assert!(!bag.has_errors());
        let lowering = AstLowering::new("test");
        let mut nir = lowering.lower_program(&prog);
        optimize_module(&mut nir, OptLevel::O1);
        let output = print_module(&nir);
        assert!(output.contains("fn main()"));
        assert!(output.contains("vout("));
    }
}
