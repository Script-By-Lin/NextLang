use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Computes SHA-256 hex digest for arbitrary byte slice (pure standard Rust).
#[allow(clippy::chunks_exact_to_as_chunks)]
pub fn sha256_digest(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, part) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([part[0], part[1], part[2], part[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(k[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    let mut out = String::with_capacity(64);
    for val in h {
        use std::fmt::Write;
        let _ = write!(out, "{:08x}", val);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Version(String),
    Path {
        path: String,
    },
    Git {
        git: String,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    pub edition: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildMeta {
    pub entry: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunMeta {
    pub host: String,
    pub port: u16,
    pub reload: bool,
}

impl Default for RunMeta {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
            reload: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplate {
    Minimal,
    Api,
    Fullstack,
}

impl std::str::FromStr for ProjectTemplate {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "minimal" => Ok(ProjectTemplate::Minimal),
            "api" => Ok(ProjectTemplate::Api),
            "fullstack" => Ok(ProjectTemplate::Fullstack),
            other => Err(format!(
                "unknown template '{}'. Available: minimal, api, fullstack",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub package: PackageMeta,
    pub build: BuildMeta,
    pub run: RunMeta,
    pub dependencies: BTreeMap<String, DependencySource>,
}

impl Manifest {
    pub fn new(name: &str) -> Self {
        Self {
            package: PackageMeta {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                edition: "2024".to_string(),
                description: format!("A NextLang package {}", name),
            },
            build: BuildMeta {
                entry: "src/main.nx".to_string(),
                target: "native".to_string(),
            },
            run: RunMeta::default(),
            dependencies: BTreeMap::new(),
        }
    }

    pub fn parse(content: &str) -> Result<Self, String> {
        let mut name = None;
        let mut version = None;
        let mut edition = "2024".to_string();
        let mut description = String::new();
        let mut entry = "src/main.nx".to_string();
        let mut target = "native".to_string();
        let mut run_host = "127.0.0.1".to_string();
        let mut run_port = 8000u16;
        let mut run_reload = false;
        let mut dependencies = BTreeMap::new();

        let mut current_section = "";
        let mut sub_dep_name: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let section = &trimmed[1..trimmed.len() - 1];
                if let Some(stripped) = section.strip_prefix("dependencies.") {
                    current_section = "dependencies.sub";
                    sub_dep_name = Some(stripped.trim().to_string());
                } else {
                    current_section = section;
                    sub_dep_name = None;
                }
                continue;
            }

            if let Some((key_raw, val_raw)) = trimmed.split_once('=') {
                let key = key_raw.trim();
                let val = val_raw.trim();

                match current_section {
                    "package" | "project" => match key {
                        "name" => name = Some(parse_string_val(val)?),
                        "version" => version = Some(parse_string_val(val)?),
                        "edition" | "language_version" => edition = parse_string_val(val)?,
                        "description" => description = parse_string_val(val)?,
                        "entry" => entry = parse_string_val(val)?,
                        _ => {}
                    },
                    "build" => match key {
                        "entry" => entry = parse_string_val(val)?,
                        "target" => target = parse_string_val(val)?,
                        _ => {}
                    },
                    "run" => match key {
                        "host" => run_host = parse_string_val(val)?,
                        "port" => run_port = parse_u16_val(val)?,
                        "reload" => run_reload = parse_bool_val(val)?,
                        _ => {}
                    },
                    "dependencies" => {
                        let dep = parse_dependency_val(val)?;
                        dependencies.insert(key.to_string(), dep);
                    }
                    "dependencies.sub" => {
                        if let Some(ref dep_name) = sub_dep_name {
                            let dep_entry =
                                dependencies.entry(dep_name.clone()).or_insert_with(|| {
                                    DependencySource::Path {
                                        path: String::new(),
                                    }
                                });
                            match dep_entry {
                                DependencySource::Path { path } => {
                                    if key == "path" {
                                        *path = parse_string_val(val)?;
                                    }
                                }
                                DependencySource::Git {
                                    git,
                                    branch,
                                    tag,
                                    rev,
                                } => {
                                    if key == "git" {
                                        *git = parse_string_val(val)?;
                                    } else if key == "branch" {
                                        *branch = Some(parse_string_val(val)?);
                                    } else if key == "tag" {
                                        *tag = Some(parse_string_val(val)?);
                                    } else if key == "rev" {
                                        *rev = Some(parse_string_val(val)?);
                                    }
                                }
                                DependencySource::Version(_) => {
                                    if key == "path" {
                                        *dep_entry = DependencySource::Path {
                                            path: parse_string_val(val)?,
                                        };
                                    } else if key == "git" {
                                        *dep_entry = DependencySource::Git {
                                            git: parse_string_val(val)?,
                                            branch: None,
                                            tag: None,
                                            rev: None,
                                        };
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let name = name
            .ok_or_else(|| "missing required field `project.name` or `package.name` in nextlang.toml".to_string())?;
        let version = version.ok_or_else(|| {
            "missing required field `project.version` or `package.version` in nextlang.toml".to_string()
        })?;

        Ok(Manifest {
            package: PackageMeta {
                name,
                version,
                edition,
                description,
            },
            build: BuildMeta { entry, target },
            run: RunMeta {
                host: run_host,
                port: run_port,
                reload: run_reload,
            },
            dependencies,
        })
    }

    pub fn to_toml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("[project]\n");
        out.push_str(&format!("name = \"{}\"\n", self.package.name));
        out.push_str(&format!("version = \"{}\"\n", self.package.version));
        out.push_str(&format!("entry = \"{}\"\n", self.build.entry));
        out.push_str(&format!("language_version = \"{}\"\n", self.package.edition));
        if !self.package.description.is_empty() {
            out.push_str(&format!("description = \"{}\"\n", self.package.description));
        }
        out.push('\n');

        out.push_str("[run]\n");
        out.push_str(&format!("host = \"{}\"\n", self.run.host));
        out.push_str(&format!("port = {}\n", self.run.port));
        out.push_str(&format!("reload = {}\n", self.run.reload));
        out.push('\n');

        out.push_str("[build]\n");
        out.push_str(&format!("entry = \"{}\"\n", self.build.entry));
        out.push_str(&format!("target = \"{}\"\n", self.build.target));
        out.push('\n');

        out.push_str("[dependencies]\n");
        for (name, dep) in &self.dependencies {
            match dep {
                DependencySource::Version(v) => {
                    out.push_str(&format!("{} = \"{}\"\n", name, v));
                }
                DependencySource::Path { path } => {
                    out.push_str(&format!("{} = {{ path = \"{}\" }}\n", name, path));
                }
                DependencySource::Git {
                    git,
                    branch,
                    tag,
                    rev,
                } => {
                    out.push_str(&format!("{} = {{ git = \"{}\"", name, git));
                    if let Some(b) = branch {
                        out.push_str(&format!(", branch = \"{}\"", b));
                    }
                    if let Some(t) = tag {
                        out.push_str(&format!(", tag = \"{}\"", t));
                    }
                    if let Some(r) = rev {
                        out.push_str(&format!(", rev = \"{}\"", r));
                    }
                    out.push_str(" }\n");
                }
            }
        }

        out
    }
}

fn parse_string_val(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        Ok(s[1..s.len() - 1].to_string())
    } else {
        Err(format!("invalid string literal: {}", s))
    }
}

fn parse_bool_val(s: &str) -> Result<bool, String> {
    let trimmed = s.trim();
    let b_str = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    match b_str {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("expected boolean (true/false), found '{}'", other)),
    }
}

fn parse_u16_val(s: &str) -> Result<u16, String> {
    let trimmed = s.trim();
    let num_str = if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    num_str
        .parse::<u16>()
        .map_err(|e| format!("invalid port number '{}': {}", s, e))
}

fn parse_dependency_val(val: &str) -> Result<DependencySource, String> {
    let val = val.trim();
    if val.starts_with('"') && val.ends_with('"') {
        return Ok(DependencySource::Version(parse_string_val(val)?));
    }

    if val.starts_with('{') && val.ends_with('}') {
        let inner = &val[1..val.len() - 1];
        let mut path = None;
        let mut git = None;
        let mut branch = None;
        let mut tag = None;
        let mut rev = None;
        let mut version = None;

        for pair in inner.split(',') {
            if let Some((k, v)) = pair.split_once('=') {
                let k = k.trim();
                let v = parse_string_val(v)?;
                match k {
                    "path" => path = Some(v),
                    "git" => git = Some(v),
                    "branch" => branch = Some(v),
                    "tag" => tag = Some(v),
                    "rev" => rev = Some(v),
                    "version" => version = Some(v),
                    _ => {}
                }
            }
        }

        if let Some(p) = path {
            return Ok(DependencySource::Path { path: p });
        }
        if let Some(g) = git {
            return Ok(DependencySource::Git {
                git: g,
                branch,
                tag,
                rev,
            });
        }
        if let Some(v) = version {
            return Ok(DependencySource::Version(v));
        }
    }

    Err(format!("invalid dependency specification: {}", val))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lockfile {
    pub version: u32,
    pub packages: Vec<LockedPackage>,
}

impl Lockfile {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            version: 1,
            packages: Vec::new(),
        }
    }

    pub fn parse(content: &str) -> Result<Self, String> {
        let mut version = 1;
        let mut packages = Vec::new();
        let mut cur_pkg: Option<LockedPackage> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed == "[[package]]" {
                if let Some(p) = cur_pkg.take() {
                    packages.push(p);
                }
                cur_pkg = Some(LockedPackage {
                    name: String::new(),
                    version: String::new(),
                    source: String::new(),
                    checksum: String::new(),
                    dependencies: Vec::new(),
                });
                continue;
            }

            if let Some((k, v)) = trimmed.split_once('=') {
                let k = k.trim();
                let v = v.trim();

                if cur_pkg.is_none() && k == "version" {
                    if let Ok(n) = v.parse::<u32>() {
                        version = n;
                    }
                    continue;
                }

                if let Some(ref mut p) = cur_pkg {
                    match k {
                        "name" => p.name = parse_string_val(v)?,
                        "version" => p.version = parse_string_val(v)?,
                        "source" => p.source = parse_string_val(v)?,
                        "checksum" => p.checksum = parse_string_val(v)?,
                        "dependencies" => {
                            let inner = v.trim_start_matches('[').trim_end_matches(']').trim();
                            if !inner.is_empty() {
                                p.dependencies = inner
                                    .split(',')
                                    .filter_map(|s| parse_string_val(s).ok())
                                    .collect();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(p) = cur_pkg {
            packages.push(p);
        }

        Ok(Lockfile { version, packages })
    }

    pub fn to_toml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("# This file is automatically generated by NextLang package manager.\n");
        out.push_str("# Manual edits will be overwritten.\n\n");
        out.push_str(&format!("version = {}\n\n", self.version));

        for p in &self.packages {
            out.push_str("[[package]]\n");
            out.push_str(&format!("name = \"{}\"\n", p.name));
            out.push_str(&format!("version = \"{}\"\n", p.version));
            out.push_str(&format!("source = \"{}\"\n", p.source));
            if !p.checksum.is_empty() {
                out.push_str(&format!("checksum = \"{}\"\n", p.checksum));
            }
            if !p.dependencies.is_empty() {
                let deps_fmt: Vec<String> = p
                    .dependencies
                    .iter()
                    .map(|d| format!("\"{}\"", d))
                    .collect();
                out.push_str(&format!("dependencies = [{}]\n", deps_fmt.join(", ")));
            } else {
                out.push_str("dependencies = []\n");
            }
            out.push('\n');
        }

        out
    }
}

/// Computes an integrity hash for a path dependency.
pub fn compute_path_integrity(dir: &Path) -> String {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files);
    files.sort();

    let mut combined = Vec::new();
    for f in files {
        if let Ok(rel) = f.strip_prefix(dir) {
            combined.extend_from_slice(rel.to_string_lossy().as_bytes());
            combined.push(0);
        }
        if let Ok(bytes) = fs::read(&f) {
            combined.extend_from_slice(&bytes);
            combined.push(0);
        }
    }

    format!("sha256:{}", sha256_digest(&combined))
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == "target" || name == "dist" {
                continue;
            }
            if p.is_dir() {
                collect_files_recursive(&p, out);
            } else if p.is_file() {
                out.push(p);
            }
        }
    }
}

/// Resolves dependencies and builds a deterministic lockfile.
pub fn resolve_lockfile(manifest_dir: &Path, manifest: &Manifest) -> Result<Lockfile, String> {
    let mut packages = Vec::new();

    // Root package
    let root_deps: Vec<String> = manifest.dependencies.keys().cloned().collect();
    packages.push(LockedPackage {
        name: manifest.package.name.clone(),
        version: manifest.package.version.clone(),
        source: "root".to_string(),
        checksum: String::new(),
        dependencies: root_deps,
    });

    for (dep_name, dep) in &manifest.dependencies {
        match dep {
            DependencySource::Path { path } => {
                let resolved_path = manifest_dir.join(path);
                let integrity = compute_path_integrity(&resolved_path);
                let mut dep_ver = "0.1.0".to_string();
                let sub_manifest_path = resolved_path.join("nextlang.toml");
                if sub_manifest_path.exists() {
                    if let Ok(sub_src) = fs::read_to_string(&sub_manifest_path) {
                        if let Ok(sub_m) = Manifest::parse(&sub_src) {
                            dep_ver = sub_m.package.version;
                        }
                    }
                }

                packages.push(LockedPackage {
                    name: dep_name.clone(),
                    version: dep_ver,
                    source: format!("path+{}", path),
                    checksum: integrity,
                    dependencies: Vec::new(),
                });
            }
            DependencySource::Git {
                git,
                branch,
                tag,
                rev,
            } => {
                let mut source_str = format!("git+{}", git);
                if let Some(r) = rev {
                    source_str.push_str(&format!("#{}", r));
                } else if let Some(t) = tag {
                    source_str.push_str(&format!("?tag={}", t));
                } else if let Some(b) = branch {
                    source_str.push_str(&format!("?branch={}", b));
                }

                let git_hash = format!("sha256:{}", sha256_digest(source_str.as_bytes()));

                packages.push(LockedPackage {
                    name: dep_name.clone(),
                    version: "0.1.0".to_string(),
                    source: source_str,
                    checksum: git_hash,
                    dependencies: Vec::new(),
                });
            }
            DependencySource::Version(v) => {
                packages.push(LockedPackage {
                    name: dep_name.clone(),
                    version: v.clone(),
                    source: format!("registry+{}", v),
                    checksum: format!("sha256:{}", sha256_digest(v.as_bytes())),
                    dependencies: Vec::new(),
                });
            }
        }
    }

    Ok(Lockfile {
        version: 1,
        packages,
    })
}

/// Locates `nextlang.toml` by walking upwards from `start_dir`.
pub fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    loop {
        let candidate = current.join("nextlang.toml");
        if candidate.is_file() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Validates project name: must be non-empty, alphanumeric with '_' or '-', cannot start with '-' or '.'.
pub fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("error: project name cannot be empty".to_string());
    }
    if name.starts_with('-') || name.starts_with('.') {
        return Err(format!(
            "error: project name '{}' cannot start with '-' or '.'",
            name
        ));
    }
    for c in name.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' {
            return Err(format!(
                "error: project name '{}' contains invalid character '{}'. Only alphanumeric, '-' and '_' are allowed",
                name, c
            ));
        }
    }
    Ok(())
}

/// Implements `nx init`: creates project files in `dir` with specified template.
pub fn init_project(
    base_dir: &Path,
    project_name_opt: Option<&str>,
    template: ProjectTemplate,
    force: bool,
) -> Result<(), String> {
    let (target_dir, pkg_name) = match project_name_opt {
        Some(name) => {
            validate_project_name(name)?;
            (base_dir.join(name), name.to_string())
        }
        None => {
            let derived = base_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("my_project")
                .replace([' ', '-'], "_");
            (base_dir.to_path_buf(), derived)
        }
    };

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("failed to create directory '{}': {}", target_dir.display(), e))?;
    }

    let manifest_path = target_dir.join("nextlang.toml");
    if manifest_path.exists() && !force {
        return Err(format!(
            "error: `nextlang.toml` already exists in '{}'. Use --force to overwrite.",
            target_dir.display()
        ));
    }

    let mut manifest = Manifest::new(&pkg_name);
    if template == ProjectTemplate::Api || template == ProjectTemplate::Fullstack {
        manifest.run.reload = true;
        manifest.dependencies.insert(
            "nextapi".to_string(),
            DependencySource::Version("*".to_string()),
        );
    }

    fs::write(&manifest_path, manifest.to_toml_string())
        .map_err(|e| format!("failed to write nextlang.toml: {}", e))?;

    let src_dir = target_dir.join("src");
    fs::create_dir_all(&src_dir).map_err(|e| format!("failed to create src/: {}", e))?;

    let main_nx = src_dir.join("main.nx");
    if !main_nx.exists() || force {
        let code = match template {
            ProjectTemplate::Minimal => r#"import math;
import strings;

fn main() -> Void {
    vout("Hello from NextLang!");
    var root: Float64 = sqrt(16.0);
    vout("sqrt(16.0) = ", root);
    return;
}

main();
"#.to_string(),
            ProjectTemplate::Api | ProjectTemplate::Fullstack => format!(
                r#"import nextapi;

async fn main() -> Void {{
    var app: NextAPI = NextAPI("{}", "0.1.0", "NextAPI Application");

    app.get("/", "Root Welcome", "Welcome endpoint", "default");
    app.set_response("GET", "/", 200, "{{\"message\":\"Hello from NextLang NextAPI!\"}}");

    app.get("/health", "Health Status", "Health check endpoint", "system");
    app.set_response("GET", "/health", 200, "{{\"status\":\"healthy\"}}");

    await app.run("127.0.0.1", 8000);
    return;
}}

main();
"#,
                pkg_name
            ),
        };
        fs::write(&main_nx, code).map_err(|e| format!("failed to write src/main.nx: {}", e))?;
    }

    let tests_dir = target_dir.join("tests");
    fs::create_dir_all(&tests_dir).map_err(|e| format!("failed to create tests/: {}", e))?;

    match template {
        ProjectTemplate::Minimal => {
            let test_nx = tests_dir.join("main_test.nx");
            if !test_nx.exists() || force {
                let test_code = r#"import testing;
import math;

test "basic math assertion" {
    assert(2 + 2 == 4, "addition works");
    assert(abs(-5.0) == 5.0, "math abs works");
}
"#;
                fs::write(&test_nx, test_code)
                    .map_err(|e| format!("failed to write tests/main_test.nx: {}", e))?;
            }
        }
        ProjectTemplate::Api | ProjectTemplate::Fullstack => {
            let test_nx = tests_dir.join("api_test.nx");
            if !test_nx.exists() || force {
                let test_code = r#"import testing;
import nextapi;

test "health check returns 200" {
    var app: NextAPI = NextAPI("Test App", "0.1.0", "API Test");
    app.get("/health", "Health", "Service health check", "system");
    app.set_response("GET", "/health", 200, "{\"status\":\"healthy\"}");

    var client: TestClient = TestClient(app);
    var res: Response = client.get("/health");
    assert(res.status == 200, "Health check should return 200 OK");
    assert(str_contains(res.body, "healthy"), "Must contain healthy status");
}
"#;
                fs::write(&test_nx, test_code)
                    .map_err(|e| format!("failed to write tests/api_test.nx: {}", e))?;
            }
        }
    }

    if template == ProjectTemplate::Fullstack {
        let frontend_dir = target_dir.join("frontend");
        let _ = fs::create_dir_all(&frontend_dir);
        let fe_readme = frontend_dir.join("README.md");
        if !fe_readme.exists() || force {
            let _ = fs::write(
                fe_readme,
                "# Frontend Application\n\nFrontend directory prepared for future fullstack integration.\n",
            );
        }
    }

    let gitignore = target_dir.join(".gitignore");
    if !gitignore.exists() || force {
        let gitignore_content = "target/\ndist/\n.nx_cache/\n.nextlang/\n.env\n.env.local\n*.log\n";
        let _ = fs::write(gitignore, gitignore_content);
    }

    let env_example = target_dir.join(".env.example");
    if !env_example.exists() || force {
        let env_content = match template {
            ProjectTemplate::Minimal => "APP_HOST=127.0.0.1\nAPP_PORT=8000\n",
            ProjectTemplate::Api | ProjectTemplate::Fullstack => {
                "APP_HOST=127.0.0.1\nAPP_PORT=8000\nDATABASE_URL=postgresql://user:password@localhost:5432/mydb\n"
            }
        };
        let _ = fs::write(env_example, env_content);
    }

    let readme = target_dir.join("README.md");
    if !readme.exists() || force {
        let readme_content = match template {
            ProjectTemplate::Minimal => format!(
                "# {}\n\nA minimal NextLang application.\n\n## Running\n\n```bash\nnx run\n```\n\n## Testing\n\n```bash\nnx test\n```\n",
                pkg_name
            ),
            ProjectTemplate::Api => format!(
                "# {}\n\nA NextAPI REST API built with NextLang.\n\n## Development with Auto-Reload\n\n```bash\nnx run --reload\n# or\nnx api dev --reload\n```\n\n## Interactive Documentation\n- Swagger UI: http://127.0.0.1:8000/docs\n- ReDoc: http://127.0.0.1:8000/redoc\n- OpenAPI JSON: http://127.0.0.1:8000/openapi.json\n\n## Testing\n\n```bash\nnx test\n```\n",
                pkg_name
            ),
            ProjectTemplate::Fullstack => format!(
                "# {}\n\nA NextLang fullstack application (NextAPI backend + frontend placeholder).\n\n## Running API with Auto-Reload\n\n```bash\nnx run --reload\n# or\nnx api dev --reload\n```\n\n## Testing\n\n```bash\nnx test\n```\n",
                pkg_name
            ),
        };
        let _ = fs::write(readme, readme_content);
    }

    let lockfile = resolve_lockfile(&target_dir, &manifest)?;
    let lock_path = target_dir.join("nextlang.lock");
    fs::write(&lock_path, lockfile.to_toml_string())
        .map_err(|e| format!("failed to write nextlang.lock: {}", e))?;

    println!(
        "Initialized NextLang project '{}' (template: {:?}) in {}",
        pkg_name,
        template,
        target_dir.display()
    );
    Ok(())
}

/// Implements `nx new <name>`: creates a directory and initializes files.
pub fn new_project(name: &str) -> Result<(), String> {
    let cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    init_project(&cur, Some(name), ProjectTemplate::Minimal, false)
}

/// Implements `nx add <pkg> [--path <p>] [--git <g>] [--version <v>]`.
pub fn add_dependency(
    project_dir: &Path,
    pkg: &str,
    path: Option<&str>,
    git: Option<&str>,
    version: Option<&str>,
) -> Result<(), String> {
    let manifest_path = project_dir.join("nextlang.toml");
    if !manifest_path.exists() {
        return Err("error: could not find `nextlang.toml` in project".to_string());
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read nextlang.toml: {}", e))?;
    let mut manifest = Manifest::parse(&content)?;

    let dep_source = if let Some(p) = path {
        DependencySource::Path {
            path: p.to_string(),
        }
    } else if let Some(g) = git {
        DependencySource::Git {
            git: g.to_string(),
            branch: None,
            tag: None,
            rev: None,
        }
    } else if let Some(v) = version {
        DependencySource::Version(v.to_string())
    } else {
        DependencySource::Version("0.1.0".to_string())
    };

    manifest.dependencies.insert(pkg.to_string(), dep_source);
    fs::write(&manifest_path, manifest.to_toml_string())
        .map_err(|e| format!("failed to update nextlang.toml: {}", e))?;

    let lockfile = resolve_lockfile(project_dir, &manifest)?;
    let lock_path = project_dir.join("nextlang.lock");
    fs::write(&lock_path, lockfile.to_toml_string())
        .map_err(|e| format!("failed to write nextlang.lock: {}", e))?;

    println!("Added dependency '{}' to {}", pkg, manifest_path.display());
    Ok(())
}

/// Implements `nx remove <pkg>`.
pub fn remove_dependency(project_dir: &Path, pkg: &str) -> Result<(), String> {
    let manifest_path = project_dir.join("nextlang.toml");
    if !manifest_path.exists() {
        return Err("error: could not find `nextlang.toml` in project".to_string());
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read nextlang.toml: {}", e))?;
    let mut manifest = Manifest::parse(&content)?;

    if manifest.dependencies.remove(pkg).is_none() {
        return Err(format!(
            "error: dependency '{}' not found in nextlang.toml",
            pkg
        ));
    }

    fs::write(&manifest_path, manifest.to_toml_string())
        .map_err(|e| format!("failed to update nextlang.toml: {}", e))?;

    let lockfile = resolve_lockfile(project_dir, &manifest)?;
    let lock_path = project_dir.join("nextlang.lock");
    fs::write(&lock_path, lockfile.to_toml_string())
        .map_err(|e| format!("failed to write nextlang.lock: {}", e))?;

    println!(
        "Removed dependency '{}' from {}",
        pkg,
        manifest_path.display()
    );
    Ok(())
}

/// Implements `nx update`.
pub fn update_lockfile(project_dir: &Path) -> Result<(), String> {
    let manifest_path = project_dir.join("nextlang.toml");
    if !manifest_path.exists() {
        return Err("error: could not find `nextlang.toml` in project".to_string());
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read nextlang.toml: {}", e))?;
    let lock_path = project_dir.join("nextlang.lock");
    if lock_path.exists() {
        if let Ok(existing_content) = fs::read_to_string(&lock_path) {
            let _ = Lockfile::parse(&existing_content);
        }
    }

    let manifest = Manifest::parse(&content)?;
    let lockfile = resolve_lockfile(project_dir, &manifest)?;
    fs::write(&lock_path, lockfile.to_toml_string())
        .map_err(|e| format!("failed to write nextlang.lock: {}", e))?;

    println!("Updated {}", lock_path.display());
    Ok(())
}

/// Implements `nx tree`.
pub fn print_tree(project_dir: &Path) -> Result<(), String> {
    let manifest_path = project_dir.join("nextlang.toml");
    if !manifest_path.exists() {
        return Err("error: could not find `nextlang.toml` in project".to_string());
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("failed to read nextlang.toml: {}", e))?;
    let manifest = Manifest::parse(&content)?;

    println!("{} v{}", manifest.package.name, manifest.package.version);
    let count = manifest.dependencies.len();
    for (idx, (name, dep)) in manifest.dependencies.iter().enumerate() {
        let is_last = idx + 1 == count;
        let prefix = if is_last { "└── " } else { "├── " };
        match dep {
            DependencySource::Path { path } => {
                println!("{}{} (path) ({})", prefix, name, path);
            }
            DependencySource::Git { git, .. } => {
                println!("{}{} (git) ({})", prefix, name, git);
            }
            DependencySource::Version(v) => {
                println!("{}{} v{}", prefix, name, v);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        assert_eq!(
            sha256_digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_digest(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_manifest_roundtrip() {
        let toml_str = r#"[package]
name = "demo_pkg"
version = "0.2.0"
edition = "2024"
description = "Demo package"

[build]
entry = "src/main.nx"
target = "native"

[dependencies]
local_lib = { path = "../local_lib" }
git_lib = { git = "https://github.com/foo/bar.git" }
reg_lib = "1.2.3"
"#;
        let manifest = Manifest::parse(toml_str).expect("parse manifest");
        assert_eq!(manifest.package.name, "demo_pkg");
        assert_eq!(manifest.package.version, "0.2.0");
        assert_eq!(manifest.dependencies.len(), 3);

        let out = manifest.to_toml_string();
        let parsed_again = Manifest::parse(&out).expect("roundtrip parse");
        assert_eq!(manifest, parsed_again);
    }

    #[test]
    fn test_lockfile_roundtrip() {
        let mut lockfile = Lockfile::new();
        lockfile.packages.push(LockedPackage {
            name: "test_pkg".to_string(),
            version: "0.1.0".to_string(),
            source: "root".to_string(),
            checksum: "sha256:123456".to_string(),
            dependencies: vec!["dep1".to_string()],
        });

        let toml_str = lockfile.to_toml_string();
        let parsed = Lockfile::parse(&toml_str).expect("parse lockfile");
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.packages[0].name, "test_pkg");
        assert_eq!(parsed.packages[0].dependencies, vec!["dep1".to_string()]);
    }
}
