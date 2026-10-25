use crate::parse_source;
use nx_ast::{Program, Stmt};
use nx_diagnostics::{DiagnosticBag, Span};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const STDLIB_MATH: &str = include_str!("../../../stdlib/math.nx");
pub const STDLIB_STRINGS: &str = include_str!("../../../stdlib/strings.nx");
pub const STDLIB_TIME: &str = include_str!("../../../stdlib/time.nx");
pub const STDLIB_ENCODING: &str = include_str!("../../../stdlib/encoding.nx");
pub const STDLIB_JSON: &str = include_str!("../../../stdlib/json.nx");
pub const STDLIB_FS: &str = include_str!("../../../stdlib/fs.nx");
pub const STDLIB_TESTING: &str = include_str!("../../../stdlib/testing.nx");

/// Returns the embedded standard library source code for standard module names.
pub fn get_stdlib_source(name: &str) -> Option<&'static str> {
    match name {
        "math" => Some(STDLIB_MATH),
        "strings" => Some(STDLIB_STRINGS),
        "time" => Some(STDLIB_TIME),
        "encoding" => Some(STDLIB_ENCODING),
        "json" => Some(STDLIB_JSON),
        "fs" => Some(STDLIB_FS),
        "testing" => Some(STDLIB_TESTING),
        _ => None,
    }
}

pub struct ModuleResolver {
    loaded: HashSet<PathBuf>,
    in_progress: Vec<PathBuf>,
    merged_statements: Vec<Stmt>,
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self {
            loaded: HashSet::new(),
            in_progress: Vec::new(),
            merged_statements: Vec::new(),
        }
    }

    /// Resolves all imports recursively starting from the given entry file,
    /// detecting circular dependencies and merging declarations into a unified AST.
    pub fn resolve_entry(
        &mut self,
        entry_path: &Path,
        content: &str,
        diagnostics: &mut DiagnosticBag,
    ) -> Program {
        let abs_entry = fs::canonicalize(entry_path).unwrap_or_else(|_| entry_path.to_path_buf());
        self.resolve_module_internal(&abs_entry, content, diagnostics, true);

        let span = if let Some(first) = self.merged_statements.first() {
            let last = self.merged_statements.last().unwrap();
            first.span().merge(last.span())
        } else {
            Span::empty(0)
        };

        Program::new(self.merged_statements.clone(), span)
    }

    fn resolve_module_internal(
        &mut self,
        current_path: &Path,
        content: &str,
        diagnostics: &mut DiagnosticBag,
        is_entry: bool,
    ) {
        if self.loaded.contains(current_path) {
            return;
        }

        if let Some(pos) = self.in_progress.iter().position(|p| p == current_path) {
            let mut cycle_paths: Vec<String> = self.in_progress[pos..]
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            cycle_paths.push(current_path.display().to_string());
            let cycle_str = cycle_paths.join(" -> ");
            diagnostics.error(
                Span::empty(0),
                format!("circular dependency detected: {}", cycle_str),
            );
            return;
        }

        self.in_progress.push(current_path.to_path_buf());
        let program = parse_source(content, diagnostics);

        let parent_dir = current_path.parent().unwrap_or_else(|| Path::new("."));

        let mut local_stmts = Vec::new();

        for stmt in program.statements {
            match stmt {
                Stmt::Import(ref import_stmt) => {
                    let path_str = &import_stmt.path;
                    if let Some(stdlib_src) = get_stdlib_source(path_str) {
                        let stdlib_path = PathBuf::from(format!("<stdlib:{}>", path_str));
                        self.resolve_module_internal(&stdlib_path, stdlib_src, diagnostics, false);
                    } else {
                        // Relative or absolute file path
                        let mut target_file = parent_dir.join(path_str);
                        if !target_file.exists() && target_file.extension().is_none() {
                            target_file.set_extension("nx");
                        }

                        if !target_file.exists() {
                            diagnostics.error(
                                import_stmt.span,
                                format!(
                                    "cannot find module '{}' at path '{}'",
                                    path_str,
                                    target_file.display()
                                ),
                            );
                            continue;
                        }

                        let abs_target = fs::canonicalize(&target_file).unwrap_or(target_file);

                        match fs::read_to_string(&abs_target) {
                            Ok(mod_content) => {
                                self.resolve_module_internal(
                                    &abs_target,
                                    &mod_content,
                                    diagnostics,
                                    false,
                                );
                            }
                            Err(e) => {
                                diagnostics.error(
                                    import_stmt.span,
                                    format!(
                                        "failed to read module file '{}': {}",
                                        abs_target.display(),
                                        e
                                    ),
                                );
                            }
                        }
                    }
                }
                other => {
                    local_stmts.push(other);
                }
            }
        }

        self.in_progress.pop();
        self.loaded.insert(current_path.to_path_buf());

        if is_entry {
            self.merged_statements.extend(local_stmts);
        } else {
            // Prepend or add declarations from imported modules before entry-level execution
            self.merged_statements.extend(local_stmts);
        }
    }
}
