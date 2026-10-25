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
pub const STDLIB_ASYNC: &str = include_str!("../../../stdlib/async.nx");

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
        "async" => Some(STDLIB_ASYNC),
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
                        // Relative or absolute file path or package resolution
                        let resolved = resolve_target_file(parent_dir, path_str);
                        let target_file = match resolved {
                            Some(p) => p,
                            None => {
                                diagnostics.error(
                                    import_stmt.span,
                                    format!(
                                        "cannot find module '{}' at path '{}'",
                                        path_str,
                                        parent_dir.join(path_str).display()
                                    ),
                                );
                                continue;
                            }
                        };

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

/// Resolves an import path to an actual file on disk.
/// Searches direct files, directory packages (src/lib.nx), parent packages/ directories,
/// and nextlang.toml dependency path mappings.
fn resolve_target_file(parent_dir: &Path, path_str: &str) -> Option<PathBuf> {
    // 1. Direct relative or absolute file path
    let direct = parent_dir.join(path_str);
    if direct.is_file() {
        return Some(direct);
    }
    let with_ext = parent_dir.join(format!("{}.nx", path_str));
    if with_ext.is_file() {
        return Some(with_ext);
    }

    // 2. Direct directory package: path_str/src/lib.nx or path_str/lib.nx
    let dir_lib = parent_dir.join(path_str).join("src").join("lib.nx");
    if dir_lib.is_file() {
        return Some(dir_lib);
    }
    let dir_lib2 = parent_dir.join(path_str).join("lib.nx");
    if dir_lib2.is_file() {
        return Some(dir_lib2);
    }

    // 3. Search packages/ and nextlang.toml dependencies upwards
    let mut cur = Some(parent_dir.to_path_buf());
    while let Some(dir) = cur {
        // Check nextlang.toml dependencies
        let manifest_path = dir.join("nextlang.toml");
        if manifest_path.is_file() {
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with(path_str) && trimmed.contains("path") {
                        if let Some(start) = trimmed.find('"') {
                            if let Some(end) = trimmed[start + 1..].find('"') {
                                let dep_path = &trimmed[start + 1..start + 1 + end];
                                let cand = dir.join(dep_path).join("src").join("lib.nx");
                                if cand.is_file() {
                                    return Some(cand);
                                }
                                let cand2 = dir.join(dep_path).join("lib.nx");
                                if cand2.is_file() {
                                    return Some(cand2);
                                }
                            }
                        }
                    }
                }
            }
        }

        let pkg_cand = dir
            .join("packages")
            .join(path_str)
            .join("src")
            .join("lib.nx");
        if pkg_cand.is_file() {
            return Some(pkg_cand);
        }
        let pkg_cand2 = dir.join("packages").join(path_str).join("lib.nx");
        if pkg_cand2.is_file() {
            return Some(pkg_cand2);
        }

        cur = dir.parent().map(|p| p.to_path_buf());
    }

    // 4. Current working directory fallback
    let fallback = Path::new("packages")
        .join(path_str)
        .join("src")
        .join("lib.nx");
    if fallback.is_file() {
        return Some(fallback);
    }

    None
}
