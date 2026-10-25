mod package;
mod reload;

use nx_ast::AstFormatter;
use nx_codegen::{compile_source_with_modules_to_executable, CodegenConfig};
use nx_diagnostics::{DiagnosticBag, SourceFile};
use nx_lexer::Lexer;
use nx_nir::{optimize_module, print_module, AstLowering, OptLevel};
use nx_parser::{parse_source, parse_source_with_modules};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{Duration, Instant};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        r#"NextLang CLI (nx) - Version {VERSION}

Usage:
    nx <command> [options] [arguments]

Project & Package Commands:
    init [name] [opts]       Initialize a new NextLang project (--template minimal|api|fullstack, --force)
    new <name>               Create a new NextLang package directory with template
    add <pkg> [opts]         Add a dependency (options: --path <dir>, --git <url>, --version <ver>)
    remove <pkg>             Remove a dependency from manifest and lockfile
    update                   Re-resolve dependencies and update nextlang.lock
    tree                     Display project dependency tree

Developer Ecosystem:
    fmt [paths...] [--check] Format NextLang source code (or verify with --check)
    test [path] [--filter f] Discover and execute NextLang unit tests

Build & Execution:
    run [file.nx] [opts]     Execute project or source file via NextLang runtime (--reload, --host <h>, --port <p>, --time)
    build [file.nx] [opts]   Compile source or project to a native executable or JS
                             Opts: -o <out>, --target <native|js>, --opt-level <0|1|2|3>, --dump-nir, --emit-llvm, --time
    run-native <file|binary> Compile and run source natively or execute native binary
    check [file.nx]          Validate source file or project for errors
    compile-js [file.nx]     Compile source file to executable JavaScript (-o <out.js>, --target node|browser)
    api dev [entry] [opts]   Start NextAPI server (--reload, --host <h>, --port <p>)

Inspection & Debugging:
    dump-nir <file.nx>       Dump NextLang typed Intermediate Representation (NIR)
    dump-llvm <file.nx>      Dump generated LLVM IR (.ll) representation
    tokenize <file.nx>       Tokenize source file and display token stream
    parse <file.nx>          Parse source file and display abstract syntax tree (AST)
    --version, -V            Print version information
    --help, -h               Print this help message

Examples:
    nx init my-api --template api
    cd my-api && nx run --reload
    nx api dev --reload
    nx test
    nx fmt --check
    nx build --target native -o my-app
"#
    );
}

pub fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    if nanos < 1_000 {
        format!("{} ns", nanos)
    } else if nanos < 1_000_000 {
        let micros = duration.as_secs_f64() * 1_000_000.0;
        format!("{:.2} µs", micros)
    } else if nanos < 1_000_000_000 {
        let millis = duration.as_secs_f64() * 1_000.0;
        format!("{:.2} ms", millis)
    } else {
        let secs = duration.as_secs_f64();
        format!("{:.3} s", secs)
    }
}

fn read_source_file(path: &Path) -> Result<SourceFile, String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(SourceFile::new(path.to_string_lossy().as_ref(), content)),
        Err(err) => Err(format!(
            "error: failed to read file '{}': {}",
            path.display(),
            err
        )),
    }
}

fn resolve_entry_path(specified_path: Option<&str>) -> Result<PathBuf, String> {
    if let Some(p) = specified_path {
        return Ok(PathBuf::from(p));
    }
    let cur_dir = env::current_dir().map_err(|e| format!("{}", e))?;
    if let Some(root) = package::find_project_root(&cur_dir) {
        let manifest_path = root.join("nextlang.toml");
        if let Ok(content) = fs::read_to_string(&manifest_path) {
            if let Ok(manifest) = package::Manifest::parse(&content) {
                let entry = root.join(&manifest.build.entry);
                if entry.exists() {
                    return Ok(entry);
                } else {
                    return Err(format!(
                        "error: project entry point '{}' not found",
                        entry.display()
                    ));
                }
            }
        }
    }
    Err("error: no source file specified and no `nextlang.toml` found in current or parent directories".to_string())
}

fn cmd_init(
    project_name: Option<&str>,
    template: package::ProjectTemplate,
    force: bool,
) -> Result<(), ()> {
    let cur = env::current_dir().map_err(|e| {
        eprintln!("error: failed to get current directory: {}", e);
    })?;
    package::init_project(&cur, project_name, template, force).map_err(|e| {
        eprintln!("{}", e);
    })
}

fn cmd_new(name: &str) -> Result<(), ()> {
    package::new_project(name).map_err(|e| {
        eprintln!("{}", e);
    })
}

fn cmd_add(
    pkg: &str,
    path: Option<&str>,
    git: Option<&str>,
    version: Option<&str>,
) -> Result<(), ()> {
    let cur_dir = env::current_dir().map_err(|e| eprintln!("error: {}", e))?;
    let proj_root = package::find_project_root(&cur_dir).unwrap_or(cur_dir);
    package::add_dependency(&proj_root, pkg, path, git, version).map_err(|e| eprintln!("{}", e))
}

fn cmd_remove(pkg: &str) -> Result<(), ()> {
    let cur_dir = env::current_dir().map_err(|e| eprintln!("error: {}", e))?;
    let proj_root = package::find_project_root(&cur_dir).unwrap_or(cur_dir);
    package::remove_dependency(&proj_root, pkg).map_err(|e| eprintln!("{}", e))
}

fn cmd_update() -> Result<(), ()> {
    let cur_dir = env::current_dir().map_err(|e| eprintln!("error: {}", e))?;
    let proj_root = package::find_project_root(&cur_dir).unwrap_or(cur_dir);
    package::update_lockfile(&proj_root).map_err(|e| eprintln!("{}", e))
}

fn cmd_tree() -> Result<(), ()> {
    let cur_dir = env::current_dir().map_err(|e| eprintln!("error: {}", e))?;
    let proj_root = package::find_project_root(&cur_dir).unwrap_or(cur_dir);
    package::print_tree(&proj_root).map_err(|e| eprintln!("{}", e))
}

fn collect_nx_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.')
                || name == "target"
                || name == "dist"
                || name == "node_modules"
                || name == ".nx_cache"
            {
                continue;
            }
            if p.is_dir() {
                collect_nx_files(&p, out);
            } else if p.is_file() && p.extension().is_some_and(|ext| ext == "nx") {
                out.push(p);
            }
        }
    }
}

fn cmd_fmt(paths: &[String], check_mode: bool) -> Result<(), ()> {
    let mut files_to_format = Vec::new();
    if paths.is_empty() {
        let cur_dir = env::current_dir().map_err(|e| eprintln!("error: {}", e))?;
        let root = package::find_project_root(&cur_dir).unwrap_or(cur_dir);
        collect_nx_files(&root, &mut files_to_format);
    } else {
        for p in paths {
            let path = Path::new(p);
            if path.is_file() {
                files_to_format.push(path.to_path_buf());
            } else if path.is_dir() {
                collect_nx_files(path, &mut files_to_format);
            }
        }
    }

    files_to_format.sort();

    if files_to_format.is_empty() {
        println!("No .nx files found to format.");
        return Ok(());
    }

    let mut unformatted_count = 0;
    let mut parse_error = false;

    for file_path in &files_to_format {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to read '{}': {}", file_path.display(), e);
                parse_error = true;
                continue;
            }
        };

        let mut diagnostics = DiagnosticBag::new();
        let program = parse_source(&content, &mut diagnostics);
        if diagnostics.has_errors() {
            let src_file = SourceFile::new(file_path.to_string_lossy().as_ref(), content);
            eprint!("{}", diagnostics.render_all(&src_file));
            parse_error = true;
            continue;
        }

        let formatted = nx_ast::format_source(&program);
        if formatted != content {
            if check_mode {
                println!("Diff in {}", file_path.display());
                unformatted_count += 1;
            } else if let Err(e) = fs::write(file_path, &formatted) {
                eprintln!("error: failed to write '{}': {}", file_path.display(), e);
                parse_error = true;
            } else {
                println!("Formatted {}", file_path.display());
            }
        }
    }

    if parse_error {
        return Err(());
    }

    if check_mode {
        if unformatted_count > 0 {
            eprintln!(
                "{} file(s) need formatting. Run 'nx fmt' to format.",
                unformatted_count
            );
            return Err(());
        } else {
            println!(
                "All {} file(s) are correctly formatted.",
                files_to_format.len()
            );
        }
    }

    Ok(())
}

fn collect_test_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.')
                || name == "target"
                || name == "dist"
                || name == ".nx_cache"
                || name == "node_modules"
            {
                continue;
            }
            if p.is_dir() {
                collect_test_files(&p, out);
            } else if p.is_file()
                && p.extension().is_some_and(|ext| ext == "nx")
                && (name.ends_with("_test.nx") || dir.ends_with("tests"))
            {
                out.push(p);
            }
        }
    }
}

fn cmd_test(path_arg: Option<&str>, filter_opt: Option<&str>) -> Result<(), ()> {
    let mut test_files = Vec::new();

    if let Some(p) = path_arg {
        let path = Path::new(p);
        if path.is_file() {
            test_files.push(path.to_path_buf());
        } else if path.is_dir() {
            collect_test_files(path, &mut test_files);
        } else {
            eprintln!("error: test path '{}' does not exist", p);
            return Err(());
        }
    } else {
        let cur_dir = env::current_dir().map_err(|e| eprintln!("error: {}", e))?;
        let root = package::find_project_root(&cur_dir).unwrap_or(cur_dir);
        let tests_dir = root.join("tests");
        if tests_dir.is_dir() {
            collect_test_files(&tests_dir, &mut test_files);
        }
        let src_dir = root.join("src");
        if src_dir.is_dir() {
            collect_test_files(&src_dir, &mut test_files);
        }
        let packages_dir = root.join("packages");
        if packages_dir.is_dir() {
            collect_test_files(&packages_dir, &mut test_files);
        }
    }

    test_files.sort();

    if test_files.is_empty() {
        println!("No tests found.");
        return Ok(());
    }

    let mut total_passed = 0;
    let mut total_failed = 0;
    let mut total_filtered = 0;
    let start_all = Instant::now();

    for test_file in &test_files {
        let content = match fs::read_to_string(test_file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "error: failed to read test file '{}': {}",
                    test_file.display(),
                    e
                );
                total_failed += 1;
                continue;
            }
        };

        let mut diagnostics = DiagnosticBag::new();
        let program = parse_source_with_modules(test_file, &content, &mut diagnostics);
        if diagnostics.has_errors() {
            let src_file = SourceFile::new(test_file.to_string_lossy().as_ref(), content);
            eprint!("{}", diagnostics.render_all(&src_file));
            total_failed += 1;
            continue;
        }

        let mut test_decls = Vec::new();
        let mut non_test_stmts = Vec::new();

        for stmt in program.statements {
            if let nx_ast::Stmt::TestDecl(t) = stmt {
                test_decls.push(t);
            } else {
                non_test_stmts.push(stmt);
            }
        }

        if test_decls.is_empty() {
            let test_name = test_file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("test");
            if let Some(pattern) = filter_opt {
                if !test_name.contains(pattern) {
                    total_filtered += 1;
                    continue;
                }
            }

            print!("test {} ... ", test_name);
            let single_prog = nx_ast::Program::new(non_test_stmts, program.span);
            match nx_runtime::eval_program_captured(&single_prog) {
                Ok(_) => {
                    println!("ok");
                    total_passed += 1;
                }
                Err(err) => {
                    println!("FAILED");
                    eprintln!("  Error: {}", err);
                    total_failed += 1;
                }
            }
        } else {
            for test_decl in test_decls {
                if let Some(pattern) = filter_opt {
                    if !test_decl.name.contains(pattern) {
                        total_filtered += 1;
                        continue;
                    }
                }

                print!("test \"{}\" ... ", test_decl.name);
                let mut stmts = non_test_stmts.clone();
                stmts.extend(test_decl.body.statements);
                let test_prog = nx_ast::Program::new(stmts, test_decl.span);

                match nx_runtime::eval_program_captured(&test_prog) {
                    Ok(_) => {
                        println!("ok");
                        total_passed += 1;
                    }
                    Err(err) => {
                        println!("FAILED");
                        eprintln!("  Error: {}", err);
                        total_failed += 1;
                    }
                }
            }
        }
    }

    let elapsed = start_all.elapsed();
    let status = if total_failed == 0 { "ok" } else { "FAILED" };
    println!(
        "\ntest result: {}. {} passed; {} failed; {} filtered out; finished in {}",
        status,
        total_passed,
        total_failed,
        total_filtered,
        format_duration(elapsed)
    );

    if total_failed > 0 {
        Err(())
    } else {
        Ok(())
    }
}

fn cmd_tokenize(path: &Path) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let mut lexer = Lexer::new(&file.content);
    let tokens = lexer.tokenize(&mut diagnostics);

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    println!("Tokens for '{}' ({} tokens):", file.name, tokens.len());
    for token in tokens {
        let (line, col) = file.location(token.span.start);
        println!("{:>4}:{:<4} {}", line, col, token);
    }

    Ok(())
}

fn cmd_parse(path: &Path) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source(&file.content, &mut diagnostics);

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    let formatted = AstFormatter::new().format_program(&program);
    print!("{}", formatted);

    Ok(())
}

fn cmd_check(path: &Path) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source_with_modules(path, &file.content, &mut diagnostics);

    if !diagnostics.has_errors() {
        nx_analyzer::analyze(&program, &mut diagnostics);
    }

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    println!("Check passed: no syntax errors found in {}", file.name);
    Ok(())
}

fn cmd_run(path: &Path, time_flag: bool) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source_with_modules(path, &file.content, &mut diagnostics);

    if !diagnostics.has_errors() {
        nx_analyzer::analyze(&program, &mut diagnostics);
    }

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    let start_time = Instant::now();
    let run_res = nx_runtime::eval_program(&program);
    let elapsed = start_time.elapsed();

    if time_flag {
        eprintln!("Execution time: {}", format_duration(elapsed));
    }

    match run_res {
        Ok(_) => Ok(()),
        Err(runtime_err) => {
            eprintln!("{}", runtime_err);
            Err(())
        }
    }
}

fn cmd_api_dev(
    entry_path: Option<&str>,
    host_opt: Option<&str>,
    port_opt: Option<&str>,
    reload: bool,
) -> Result<(), ()> {
    let path = match resolve_entry_path(entry_path) {
        Ok(p) => p,
        Err(err) => {
            eprintln!("{}", err);
            return Err(());
        }
    };

    let cur_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let proj_root = package::find_project_root(&cur_dir)
        .or_else(|| path.parent().map(|p| p.to_path_buf()))
        .unwrap_or(cur_dir);

    let manifest_path = proj_root.join("nextlang.toml");
    let manifest_run = if let Ok(content) = fs::read_to_string(&manifest_path) {
        package::Manifest::parse(&content).map(|m| m.run).ok()
    } else {
        None
    };

    let host = host_opt
        .or_else(|| manifest_run.as_ref().map(|r| r.host.as_str()))
        .unwrap_or("127.0.0.1");
    let default_port_str = manifest_run
        .as_ref()
        .map(|r| r.port.to_string())
        .unwrap_or_else(|| "8000".to_string());
    let port = port_opt.unwrap_or(&default_port_str);
    let should_reload = reload || manifest_run.as_ref().map(|r| r.reload).unwrap_or(false);

    if should_reload {
        return reload::run_with_reload(&proj_root, &path, Some(host), Some(port));
    }

    println!("============================================================");
    println!("  NextAPI Development Server (nx api dev)");
    println!("  Entry point: {}", path.display());
    println!("  Target: http://{}:{}", host, port);
    println!("  Interactive Swagger UI: http://{}:{}/docs", host, port);
    println!("  ReDoc: http://{}:{}/redoc", host, port);
    println!(
        "  OpenAPI Specification: http://{}:{}/openapi.json",
        host, port
    );
    println!("============================================================");

    std::env::set_var("APP_HOST", host);
    std::env::set_var("APP_PORT", port);
    cmd_run(&path, false)
}

fn cmd_compile_js(
    path: &Path,
    output_path: Option<&str>,
    target: nx_js_backend::JsTarget,
    to_stdout: bool,
) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source_with_modules(path, &file.content, &mut diagnostics);

    if !diagnostics.has_errors() {
        nx_analyzer::analyze(&program, &mut diagnostics);
    }

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    let config = nx_js_backend::CompilerConfig::new().with_target(target);
    let js_code = nx_js_backend::compile_program(&program, config);

    if to_stdout {
        print!("{}", js_code);
        return Ok(());
    }

    let out_file = match output_path {
        Some(p) => PathBuf::from(p),
        None => path.with_extension("js"),
    };

    if let Some(parent) = out_file.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }

    if let Err(err) = fs::write(&out_file, &js_code) {
        eprintln!(
            "error: failed to write output file '{}': {}",
            out_file.display(),
            err
        );
        return Err(());
    }

    println!("Compiled '{}' -> '{}'", path.display(), out_file.display());
    Ok(())
}

fn cmd_build(
    path: &Path,
    output_path: Option<&str>,
    target: &str,
    opt_level: OptLevel,
    dump_nir: bool,
    emit_llvm: bool,
    time: bool,
) -> Result<(), ()> {
    if target == "js" {
        return cmd_compile_js(path, output_path, nx_js_backend::JsTarget::Node, false);
    }

    let start_time = Instant::now();
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let out_exe: PathBuf = match output_path {
        Some(out) => PathBuf::from(out),
        None => {
            let stem = path.file_stem().unwrap_or_default();
            path.with_file_name(stem)
        }
    };

    let mut diagnostics = DiagnosticBag::new();
    let config = CodegenConfig {
        opt_level,
        dump_nir,
        emit_llvm,
        ..Default::default()
    };

    let result = compile_source_with_modules_to_executable(
        path,
        &file.content,
        &out_exe,
        &config,
        &mut diagnostics,
    );

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    match result {
        Ok(true) => {
            println!("Built '{}' -> '{}'", path.display(), out_exe.display());
            if time {
                let duration = start_time.elapsed();
                eprintln!("Build time: {}", format_duration(duration));
            }
            Ok(())
        }
        Ok(false) => {
            eprint!("{}", diagnostics.render_all(&file));
            Err(())
        }
        Err(err) => {
            eprintln!("error: native compilation failed: {}", err);
            Err(())
        }
    }
}

struct TempFileGuard(PathBuf);
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cmd_run_native(path: &Path, time: bool, passthrough_args: &[String]) -> Result<(), ()> {
    let is_nx_source = path.extension().is_some_and(|ext| ext == "nx");

    let exe_to_run: PathBuf;
    let _temp_guard: Option<TempFileGuard>;

    if is_nx_source {
        let file = read_source_file(path).map_err(|err| {
            eprintln!("{}", err);
        })?;

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("run_bin");
        let tmp_dir = env::temp_dir().join(format!("nx_run_{}_{}", process::id(), stem));
        let _ = fs::create_dir_all(&tmp_dir);
        let tmp_exe = tmp_dir.join(stem);

        let mut diagnostics = DiagnosticBag::new();
        let config = CodegenConfig::default();

        let ok = compile_source_with_modules_to_executable(
            path,
            &file.content,
            &tmp_exe,
            &config,
            &mut diagnostics,
        )
        .map_err(|e| {
            eprintln!("error: compilation failed: {}", e);
        })?;

        if !ok || diagnostics.has_errors() {
            eprint!("{}", diagnostics.render_all(&file));
            let _ = fs::remove_dir_all(&tmp_dir);
            return Err(());
        }

        exe_to_run = tmp_exe;
        _temp_guard = Some(TempFileGuard(tmp_dir));
    } else {
        if !path.exists() {
            eprintln!("error: binary '{}' does not exist", path.display());
            return Err(());
        }
        exe_to_run = path.to_path_buf();
        _temp_guard = None;
    }

    let start_time = Instant::now();
    let mut cmd = Command::new(&exe_to_run);
    cmd.args(passthrough_args);

    let status = cmd.status().map_err(|err| {
        eprintln!("error: failed to execute native binary: {}", err);
    })?;

    if time {
        let duration = start_time.elapsed();
        eprintln!("Execution time: {}", format_duration(duration));
    }

    if status.success() {
        Ok(())
    } else {
        let code = status.code().unwrap_or(1);
        process::exit(code);
    }
}

fn cmd_dump_nir(path: &Path, opt_level: OptLevel) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source_with_modules(path, &file.content, &mut diagnostics);
    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    let mut analyzer = nx_analyzer::SemanticAnalyzer::new(&mut diagnostics);
    analyzer.analyze_program(&program);
    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");

    let lowering = AstLowering::new(stem);
    let mut nir = lowering.lower_program(&program);
    optimize_module(&mut nir, opt_level);

    println!("{}", print_module(&nir));
    Ok(())
}

fn cmd_dump_llvm(path: &Path, opt_level: OptLevel) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source_with_modules(path, &file.content, &mut diagnostics);
    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    let mut analyzer = nx_analyzer::SemanticAnalyzer::new(&mut diagnostics);
    analyzer.analyze_program(&program);
    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("module");
    let lowering = AstLowering::new(stem);
    let mut nir = lowering.lower_program(&program);
    optimize_module(&mut nir, opt_level);

    let llvm = nx_codegen::LlvmCompiler::new();
    let ir = llvm.emit_llvm_ir(&nir);
    println!("{}", ir);
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        process::exit(1);
    }

    match args[1].as_str() {
        "--version" | "-V" | "version" => {
            println!("nx {}", VERSION);
        }
        "--help" | "-h" | "help" => {
            print_help();
        }
        "init" => {
            let mut project_name = None;
            let mut template = package::ProjectTemplate::Minimal;
            let mut force = false;

            let mut iter = args[2..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "--template" {
                    if let Some(t) = iter.next() {
                        template = match t.parse::<package::ProjectTemplate>() {
                            Ok(tmpl) => tmpl,
                            Err(err) => {
                                eprintln!("error: {}", err);
                                process::exit(1);
                            }
                        };
                    } else {
                        eprintln!("error: '--template' requires an argument (minimal, api, fullstack)");
                        process::exit(1);
                    }
                } else if arg == "--force" {
                    force = true;
                } else if !arg.starts_with("--") && project_name.is_none() {
                    project_name = Some(arg.as_str());
                }
            }

            if cmd_init(project_name, template, force).is_err() {
                process::exit(1);
            }
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("error: 'new' requires a project name\n\nUsage: nx new <project-name>");
                process::exit(1);
            }
            if cmd_new(&args[2]).is_err() {
                process::exit(1);
            }
        }
        "add" => {
            if args.len() < 3 {
                eprintln!("error: 'add' requires a package name\n\nUsage: nx add <package> [--path <p>] [--git <g>] [--version <v>]");
                process::exit(1);
            }
            let pkg = &args[2];
            let mut path_opt = None;
            let mut git_opt = None;
            let mut version_opt = None;

            let mut iter = args[3..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "--path" {
                    path_opt = iter.next().map(|s| s.as_str());
                } else if arg == "--git" {
                    git_opt = iter.next().map(|s| s.as_str());
                } else if arg == "--version" {
                    version_opt = iter.next().map(|s| s.as_str());
                }
            }

            if cmd_add(pkg, path_opt, git_opt, version_opt).is_err() {
                process::exit(1);
            }
        }
        "remove" => {
            if args.len() < 3 {
                eprintln!("error: 'remove' requires a package name\n\nUsage: nx remove <package>");
                process::exit(1);
            }
            if cmd_remove(&args[2]).is_err() {
                process::exit(1);
            }
        }
        "update" => {
            if cmd_update().is_err() {
                process::exit(1);
            }
        }
        "tree" => {
            if cmd_tree().is_err() {
                process::exit(1);
            }
        }
        "fmt" => {
            let mut check_mode = false;
            let mut paths = Vec::new();

            for arg in &args[2..] {
                if arg == "--check" {
                    check_mode = true;
                } else if !arg.starts_with("--") {
                    paths.push(arg.clone());
                }
            }

            if cmd_fmt(&paths, check_mode).is_err() {
                process::exit(1);
            }
        }
        "test" => {
            let mut path_arg = None;
            let mut filter_opt = None;

            let mut iter = args[2..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "--filter" {
                    filter_opt = iter.next().map(|s| s.as_str());
                } else if !arg.starts_with("--") && path_arg.is_none() {
                    path_arg = Some(arg.as_str());
                }
            }

            if cmd_test(path_arg, filter_opt).is_err() {
                process::exit(1);
            }
        }
        "run" => {
            let mut file_path = None;
            let mut time_flag = false;
            let mut reload_flag = false;
            let mut host_opt = None;
            let mut port_opt = None;

            let mut iter = args[2..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "--time" {
                    time_flag = true;
                } else if arg == "--reload" {
                    reload_flag = true;
                } else if arg == "--host" {
                    host_opt = iter.next().map(|s| s.as_str());
                } else if arg == "--port" {
                    port_opt = iter.next().map(|s| s.as_str());
                } else if !arg.starts_with("--") && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            let path = match resolve_entry_path(file_path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            let cur_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let proj_root = package::find_project_root(&cur_dir)
                .or_else(|| path.parent().map(|p| p.to_path_buf()))
                .unwrap_or(cur_dir);

            let manifest_path = proj_root.join("nextlang.toml");
            let manifest_run = if let Ok(content) = fs::read_to_string(&manifest_path) {
                package::Manifest::parse(&content).map(|m| m.run).ok()
            } else {
                None
            };

            let should_reload = reload_flag || manifest_run.as_ref().map(|r| r.reload).unwrap_or(false);

            if let Some(h) = host_opt {
                std::env::set_var("APP_HOST", h);
            }
            if let Some(p) = port_opt {
                std::env::set_var("APP_PORT", p);
            }

            if should_reload {
                if reload::run_with_reload(&proj_root, &path, host_opt, port_opt).is_err() {
                    process::exit(1);
                }
            } else {
                if cmd_run(&path, time_flag).is_err() {
                    process::exit(1);
                }
            }
        }
        "api" => {
            let subcommand = args.get(2).map(|s| s.as_str());
            let (is_dev, arg_offset) = match subcommand {
                Some("dev") => (true, 3),
                Some(other) if other.starts_with("--") || other.ends_with(".nx") => (true, 2),
                None => (true, 2),
                _ => (false, 2),
            };

            if is_dev {
                let mut entry = None;
                let mut host = None;
                let mut port = None;
                let mut reload = false;

                let mut iter = args[arg_offset..].iter().peekable();
                while let Some(arg) = iter.next() {
                    if arg == "--host" {
                        host = iter.next().map(|s| s.as_str());
                    } else if arg == "--port" {
                        port = iter.next().map(|s| s.as_str());
                    } else if arg == "--reload" {
                        reload = true;
                    } else if !arg.starts_with("--") && entry.is_none() {
                        entry = Some(arg.as_str());
                    }
                }

                if cmd_api_dev(entry, host, port, reload).is_err() {
                    process::exit(1);
                }
            } else {
                eprintln!(
                    "error: 'api' requires a subcommand, e.g. 'nx api dev'\n\nUsage:\n    nx api dev [entry.nx] [--host <host>] [--port <port>] [--reload]"
                );
                process::exit(1);
            }
        }
        "compile-js" => {
            let mut file_path = None;
            let mut output_path = None;
            let mut target = nx_js_backend::JsTarget::Node;
            let mut to_stdout = false;

            let mut iter = args[2..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "-o" || arg == "--output" {
                    if let Some(out) = iter.next() {
                        output_path = Some(out.as_str());
                    } else {
                        eprintln!("error: '-o' requires an output file path");
                        process::exit(1);
                    }
                } else if arg == "--target" {
                    if let Some(t) = iter.next() {
                        match t.as_str() {
                            "node" => target = nx_js_backend::JsTarget::Node,
                            "browser" => target = nx_js_backend::JsTarget::Browser,
                            other => {
                                eprintln!(
                                    "error: invalid target '{}', expected 'node' or 'browser'",
                                    other
                                );
                                process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("error: '--target' requires a target name (node or browser)");
                        process::exit(1);
                    }
                } else if arg == "--stdout" {
                    to_stdout = true;
                } else if !arg.starts_with("--") && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            let path = match resolve_entry_path(file_path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            if cmd_compile_js(&path, output_path, target, to_stdout).is_err() {
                process::exit(1);
            }
        }
        "tokenize" => {
            if args.len() < 3 {
                eprintln!(
                    "error: 'tokenize' requires a source file path\n\nUsage: nx tokenize <file.nx>"
                );
                process::exit(1);
            }
            if cmd_tokenize(Path::new(&args[2])).is_err() {
                process::exit(1);
            }
        }
        "parse" => {
            if args.len() < 3 {
                eprintln!(
                    "error: 'parse' requires a source file path\n\nUsage: nx parse <file.nx>"
                );
                process::exit(1);
            }
            if cmd_parse(Path::new(&args[2])).is_err() {
                process::exit(1);
            }
        }
        "check" => {
            let mut file_path = None;
            for arg in &args[2..] {
                if !arg.starts_with("--") && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            let path = match resolve_entry_path(file_path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            if cmd_check(&path).is_err() {
                process::exit(1);
            }
        }
        "build" => {
            let mut file_path = None;
            let mut output_path = None;
            let mut target = "native";
            let mut opt_level = OptLevel::O0;
            let mut dump_nir = false;
            let mut emit_llvm = false;
            let mut time_flag = false;

            let mut iter = args[2..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "-o" || arg == "--output" {
                    if let Some(out) = iter.next() {
                        output_path = Some(out.as_str());
                    } else {
                        eprintln!("error: '-o' requires an output file path");
                        process::exit(1);
                    }
                } else if arg == "--target" {
                    if let Some(t) = iter.next() {
                        match t.as_str() {
                            "native" => target = "native",
                            "js" => target = "js",
                            other => {
                                eprintln!(
                                    "error: invalid build target '{}', expected 'native' or 'js'",
                                    other
                                );
                                process::exit(1);
                            }
                        }
                    } else {
                        eprintln!("error: '--target' requires 'native' or 'js'");
                        process::exit(1);
                    }
                } else if arg == "--opt-level" {
                    if let Some(lvl) = iter.next() {
                        opt_level = match lvl.as_str() {
                            "0" => OptLevel::O0,
                            "1" => OptLevel::O1,
                            "2" => OptLevel::O2,
                            "3" => OptLevel::O3,
                            other => {
                                eprintln!(
                                    "error: invalid opt-level '{}', expected 0, 1, 2, or 3",
                                    other
                                );
                                process::exit(1);
                            }
                        };
                    } else {
                        eprintln!(
                            "error: '--opt-level' requires an optimization level (0, 1, 2, 3)"
                        );
                        process::exit(1);
                    }
                } else if arg == "--dump-nir" {
                    dump_nir = true;
                } else if arg == "--emit-llvm" {
                    emit_llvm = true;
                } else if arg == "--time" {
                    time_flag = true;
                } else if !arg.starts_with("--") && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            let path = match resolve_entry_path(file_path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            if cmd_build(
                &path,
                output_path,
                target,
                opt_level,
                dump_nir,
                emit_llvm,
                time_flag,
            )
            .is_err()
            {
                process::exit(1);
            }
        }
        "run-native" => {
            let mut file_path = None;
            let mut time_flag = false;
            let mut passthrough = Vec::new();

            for arg in &args[2..] {
                if arg == "--time" {
                    time_flag = true;
                } else if file_path.is_none() && !arg.starts_with("--") {
                    file_path = Some(arg.as_str());
                } else {
                    passthrough.push(arg.clone());
                }
            }

            let path = match resolve_entry_path(file_path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            if cmd_run_native(&path, time_flag, &passthrough).is_err() {
                process::exit(1);
            }
        }
        "dump-nir" => {
            let mut file_path = None;
            let mut opt_level = OptLevel::O0;

            let mut iter = args[2..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "--opt-level" {
                    if let Some(lvl) = iter.next() {
                        opt_level = match lvl.as_str() {
                            "0" => OptLevel::O0,
                            "1" => OptLevel::O1,
                            "2" => OptLevel::O2,
                            "3" => OptLevel::O3,
                            other => {
                                eprintln!(
                                    "error: invalid opt-level '{}', expected 0, 1, 2, or 3",
                                    other
                                );
                                process::exit(1);
                            }
                        };
                    } else {
                        eprintln!(
                            "error: '--opt-level' requires an optimization level (0, 1, 2, 3)"
                        );
                        process::exit(1);
                    }
                } else if !arg.starts_with("--") && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            let path = match resolve_entry_path(file_path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            if cmd_dump_nir(&path, opt_level).is_err() {
                process::exit(1);
            }
        }
        "dump-llvm" => {
            let mut file_path = None;
            let mut opt_level = OptLevel::O1;

            let mut iter = args[2..].iter().peekable();
            while let Some(arg) = iter.next() {
                if arg == "--opt-level" {
                    if let Some(lvl) = iter.next() {
                        opt_level = match lvl.as_str() {
                            "0" => OptLevel::O0,
                            "1" => OptLevel::O1,
                            "2" => OptLevel::O2,
                            "3" => OptLevel::O3,
                            other => {
                                eprintln!(
                                    "error: invalid opt-level '{}', expected 0, 1, 2, or 3",
                                    other
                                );
                                process::exit(1);
                            }
                        };
                    } else {
                        eprintln!(
                            "error: '--opt-level' requires an optimization level (0, 1, 2, 3)"
                        );
                        process::exit(1);
                    }
                } else if !arg.starts_with("--") && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            let path = match resolve_entry_path(file_path) {
                Ok(p) => p,
                Err(err) => {
                    eprintln!("{}", err);
                    process::exit(1);
                }
            };

            if cmd_dump_llvm(&path, opt_level).is_err() {
                process::exit(1);
            }
        }
        unknown => {
            eprintln!(
                "error: unknown command or flag '{}'\nRun 'nx --help' for available commands.",
                unknown
            );
            process::exit(1);
        }
    }
}
