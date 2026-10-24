use std::path::{Path, PathBuf};
use std::process::Command;

pub struct Linker {
    runtime_obj: PathBuf,
}

impl Linker {
    pub fn new(cache_dir: &Path) -> Result<Self, String> {
        let runtime_obj = nx_native_runtime::get_or_build_runtime_obj(cache_dir)?;
        Ok(Self { runtime_obj })
    }

    pub fn link(&self, object_path: &Path, output_exe: &Path) -> Result<(), String> {
        // Safety check: Prevent accidental overwriting of NextLang source files
        if let Some(ext) = output_exe.extension() {
            if ext == "nx" {
                return Err(format!(
                    "refusing to overwrite NextLang source file: {}",
                    output_exe.display()
                ));
            }
        }

        // Ensure parent directory exists
        if let Some(parent) = output_exe.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create output directory: {}", e))?;
            }
        }

        let compiler = if Command::new("gcc").arg("--version").output().is_ok() {
            "gcc"
        } else if Command::new("cc").arg("--version").output().is_ok() {
            "cc"
        } else {
            return Err("no system C compiler (gcc or cc) found in PATH".to_string());
        };

        let mut cmd = Command::new(compiler);
        cmd.arg("-no-pie")
            .arg(object_path)
            .arg(&self.runtime_obj)
            .arg("-lm")
            .arg("-o")
            .arg(output_exe);

        let output = cmd
            .output()
            .map_err(|e| format!("failed to execute {}: {}", compiler, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "native linker failed with exit code {:?}:\n{}",
                output.status.code(),
                stderr.trim()
            ));
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(output_exe) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(output_exe, perms);
            }
        }

        Ok(())
    }
}
