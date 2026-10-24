pub mod environment;
pub mod error;
pub mod heap;
pub mod interpreter;
pub mod value;

pub use environment::Environment;
pub use error::RuntimeError;
pub use heap::HeapManager;
pub use interpreter::Interpreter;
pub use value::{ObjectInstance, Value};

use nx_ast::Program;

/// Evaluates a NextLang AST program writing `vout` output to `std::io::stdout()`.
pub fn eval_program(program: &Program) -> Result<Value, RuntimeError> {
    let mut interp = Interpreter::new(std::io::stdout());
    interp.eval_program(program)
}

/// Evaluates a NextLang AST program capturing `vout` output to a String buffer (useful for testing).
pub fn eval_program_captured(program: &Program) -> Result<(Value, String), RuntimeError> {
    let buf = Vec::new();
    let mut interp = Interpreter::new(buf);
    let val = interp.eval_program(program)?;
    let output = String::from_utf8_lossy(&interp.stdout).to_string();
    Ok((val, output))
}
