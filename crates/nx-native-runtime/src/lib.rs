pub const RUNTIME_C_SOURCE: &str = include_str!("../c/runtime.c");
pub const RUNTIME_H_SOURCE: &str = include_str!("../c/runtime.h");

use std::path::{Path, PathBuf};
use std::process::Command;

/// Ensures that the native runtime static library (or object file) exists in `out_dir`.
/// Compiles the embedded C runtime source using `gcc` or `cc` if needed.
pub fn get_or_build_runtime_obj(cache_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(cache_dir)
        .map_err(|e| format!("failed to create runtime cache directory: {}", e))?;

    let abs_cache_dir =
        std::fs::canonicalize(cache_dir).unwrap_or_else(|_| cache_dir.to_path_buf());
    let c_path = abs_cache_dir.join("nx_runtime.c");
    let h_path = abs_cache_dir.join("runtime.h");
    let obj_path = abs_cache_dir.join("nx_runtime.o");

    std::fs::write(&c_path, RUNTIME_C_SOURCE)
        .map_err(|e| format!("failed to write runtime.c: {}", e))?;
    std::fs::write(&h_path, RUNTIME_H_SOURCE)
        .map_err(|e| format!("failed to write runtime.h: {}", e))?;

    // Compile with clang, gcc, or cc
    let compiler = if Command::new("clang").arg("--version").output().is_ok() {
        "clang"
    } else if Command::new("gcc").arg("--version").output().is_ok() {
        "gcc"
    } else {
        "cc"
    };

    let status = Command::new(compiler)
        .arg("-c")
        .arg("-O3")
        .arg("-fPIC")
        .arg(&c_path)
        .arg("-o")
        .arg(&obj_path)
        .status()
        .map_err(|e| format!("failed to invoke {}: {}", compiler, e))?;

    if !status.success() {
        return Err(format!(
            "runtime compilation failed with status: {:?}",
            status
        ));
    }

    Ok(obj_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_sources() {
        assert!(!RUNTIME_C_SOURCE.is_empty());
        assert!(!RUNTIME_H_SOURCE.is_empty());
        assert!(RUNTIME_C_SOURCE.contains("nx_rt_vout_str"));
    }
}
