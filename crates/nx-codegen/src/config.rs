use nx_nir::OptLevel;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Llvm,
    Cranelift,
}

#[derive(Debug, Clone)]
pub struct CodegenConfig {
    pub backend: Backend,
    pub opt_level: OptLevel,
    pub dump_nir: bool,
    pub emit_llvm: bool,
    pub output_path: Option<PathBuf>,
}

impl Default for CodegenConfig {
    fn default() -> Self {
        Self {
            backend: Backend::Llvm,
            opt_level: OptLevel::O1,
            dump_nir: false,
            emit_llvm: false,
            output_path: None,
        }
    }
}
