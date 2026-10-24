use nx_ast::AstFormatter;
use nx_codegen::{compile_source_to_executable, CodegenConfig};
use nx_diagnostics::{DiagnosticBag, SourceFile};
use nx_lexer::Lexer;
use nx_nir::{optimize_module, print_module, AstLowering, OptLevel};
use nx_parser::parse_source;
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

Commands:
    run <file.nx> [--time]   Execute source file via the NextLang runtime engine
    build <file.nx> [-o <out>] [--opt-level <0|1|2|3>] [--dump-nir] [--time]
                             Compile source file to a standalone native ELF executable
    run-native <file|binary> [--time]
                             Compile and run source natively or execute native binary
    dump-nir <file.nx> [--opt-level <0|1|2|3>]
                             Dump NextLang typed Intermediate Representation (NIR)
    dump-llvm <file.nx> [--opt-level <0|1|2|3>]
                             Dump generated LLVM IR (.ll) representation
    compile-js <file.nx> [-o <out.js>] [--target node|browser] [--stdout]
                             Compile source file to executable JavaScript
    tokenize <file.nx>       Tokenize source file and display token stream
    parse <file.nx>          Parse source file and display abstract syntax tree (AST)
    check <file.nx>          Validate source file for lexical, syntactic, and semantic errors
    --version, -V            Print version information
    --help, -h               Print this help message

Options:
    --time                   Measure and display execution duration to stderr
    -o, --output <file>      Specify output binary or JavaScript file path
    --opt-level <0|1|2|3>    Optimization level for native codegen (default: 0)
    --dump-nir               Dump intermediate NIR during compilation
    --emit-llvm              Emit LLVM IR during native compilation
    --target <node|browser>  Target JS platform (default: node)
    --stdout                 Output generated JavaScript to stdout

Examples:
    nx run examples/hello.nx
    nx build examples/hello.nx -o hello
    nx run-native examples/hello.nx
    nx run-native hello --time
    nx dump-nir examples/hello.nx --opt-level 2
    nx compile-js examples/hello.nx -o dist/hello.js
    nx tokenize examples/hello.nx
    nx parse examples/hello.nx
    nx check examples/hello.nx
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

fn read_source_file(path: &str) -> Result<SourceFile, String> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(SourceFile::new(path, content)),
        Err(err) => Err(format!("error: failed to read file '{}': {}", path, err)),
    }
}

fn cmd_tokenize(path: &str) -> Result<(), ()> {
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

fn cmd_parse(path: &str) -> Result<(), ()> {
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

fn cmd_check(path: &str) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source(&file.content, &mut diagnostics);

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

fn cmd_run(path: &str, time_flag: bool) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source(&file.content, &mut diagnostics);

    if !diagnostics.has_errors() {
        nx_analyzer::analyze(&program, &mut diagnostics);
    }

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    // Monotonic elapsed-time measurement immediately around execution
    let start_time = Instant::now();
    let run_res = nx_runtime::eval_program(&program);
    let elapsed = start_time.elapsed();

    // Duration displayed after program output, to stderr by default
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

fn cmd_compile_js(
    path: &str,
    output_path: Option<&str>,
    target: nx_js_backend::JsTarget,
    to_stdout: bool,
) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source(&file.content, &mut diagnostics);

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
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let p = std::path::Path::new(path);
            p.with_extension("js")
        }
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

    println!("Compiled '{}' -> '{}'", path, out_file.display());
    Ok(())
}

fn cmd_build(
    path: &str,
    output_path: Option<&str>,
    opt_level: OptLevel,
    dump_nir: bool,
    emit_llvm: bool,
    time: bool,
) -> Result<(), ()> {
    let start_time = Instant::now();
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let out_exe: PathBuf = match output_path {
        Some(out) => PathBuf::from(out),
        None => {
            let path_obj = Path::new(path);
            let stem = path_obj.file_stem().unwrap_or_default();
            path_obj.with_file_name(stem)
        }
    };

    let mut diagnostics = DiagnosticBag::new();
    let config = CodegenConfig {
        opt_level,
        dump_nir,
        emit_llvm,
        ..Default::default()
    };

    let result = compile_source_to_executable(&file.content, &out_exe, &config, &mut diagnostics);

    if diagnostics.has_errors() {
        eprint!("{}", diagnostics.render_all(&file));
        return Err(());
    }

    match result {
        Ok(true) => {
            println!("Built '{}' -> '{}'", path, out_exe.display());
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

fn cmd_run_native(path: &str, time: bool, passthrough_args: &[String]) -> Result<(), ()> {
    let path_obj = Path::new(path);
    let is_nx_source = path_obj.extension().is_some_and(|ext| ext == "nx");

    let exe_to_run: PathBuf;
    let _temp_guard: Option<TempFileGuard>;

    if is_nx_source {
        let file = read_source_file(path).map_err(|err| {
            eprintln!("{}", err);
        })?;

        let stem = path_obj
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("run_bin");
        let tmp_dir = env::temp_dir().join(format!("nx_run_{}_{}", process::id(), stem));
        let _ = fs::create_dir_all(&tmp_dir);
        let tmp_exe = tmp_dir.join(stem);

        let mut diagnostics = DiagnosticBag::new();
        let config = CodegenConfig::default();

        let ok = compile_source_to_executable(&file.content, &tmp_exe, &config, &mut diagnostics)
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
        if !path_obj.exists() {
            eprintln!("error: binary '{}' does not exist", path);
            return Err(());
        }
        exe_to_run = PathBuf::from(path);
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

fn cmd_dump_nir(path: &str, opt_level: OptLevel) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let program = parse_source(&file.content, &mut diagnostics);
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

    let stem = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("program");

    let lowering = AstLowering::new(stem);
    let mut nir = lowering.lower_program(&program);
    optimize_module(&mut nir, opt_level);

    println!("{}", print_module(&nir));
    Ok(())
}

fn cmd_dump_llvm(path: &str, opt_level: OptLevel) -> Result<(), ()> {
    let file = read_source_file(path).map_err(|err| {
        eprintln!("{}", err);
    })?;

    let mut diagnostics = DiagnosticBag::new();
    let config = CodegenConfig {
        opt_level,
        ..Default::default()
    };

    match nx_codegen::emit_llvm_ir_from_source(&file.content, &config, &mut diagnostics) {
        Ok(Some(ir)) => {
            println!("{}", ir);
            Ok(())
        }
        Ok(None) => {
            eprint!("{}", diagnostics.render_all(&file));
            Err(())
        }
        Err(err) => {
            eprintln!("error: failed to emit LLVM IR: {}", err);
            Err(())
        }
    }
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
        "run" => {
            let mut file_path = None;
            let mut time_flag = false;

            for arg in &args[2..] {
                if arg == "--time" {
                    time_flag = true;
                } else if !arg.starts_with("--") && file_path.is_none() {
                    file_path = Some(arg.as_str());
                }
            }

            let path = match file_path {
                Some(p) => p,
                None => {
                    eprintln!(
                        "error: 'run' requires a source file path\n\nUsage: nx run <file.nx> [--time]"
                    );
                    process::exit(1);
                }
            };

            if cmd_run(path, time_flag).is_err() {
                process::exit(1);
            }
        }
        "compile-js" | "compile" => {
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
                                    "error: unknown target '{}', supported: node, browser",
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

            let path = match file_path {
                Some(p) => p,
                None => {
                    eprintln!(
                        "error: 'compile-js' requires a source file path\n\nUsage: nx compile-js <file.nx> [-o <out.js>]"
                    );
                    process::exit(1);
                }
            };

            if cmd_compile_js(path, output_path, target, to_stdout).is_err() {
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
            if cmd_tokenize(&args[2]).is_err() {
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
            if cmd_parse(&args[2]).is_err() {
                process::exit(1);
            }
        }
        "check" => {
            if args.len() < 3 {
                eprintln!(
                    "error: 'check' requires a source file path\n\nUsage: nx check <file.nx>"
                );
                process::exit(1);
            }
            if cmd_check(&args[2]).is_err() {
                process::exit(1);
            }
        }
        "build" => {
            let mut file_path = None;
            let mut output_path = None;
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

            let path = match file_path {
                Some(p) => p,
                None => {
                    eprintln!("error: 'build' requires a source file path\n\nUsage: nx build <file.nx> [-o <out>] [--opt-level <0|1|2|3>] [--dump-nir] [--emit-llvm] [--time]");
                    process::exit(1);
                }
            };

            if cmd_build(path, output_path, opt_level, dump_nir, emit_llvm, time_flag).is_err() {
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

            let path = match file_path {
                Some(p) => p,
                None => {
                    eprintln!("error: 'run-native' requires a file path\n\nUsage: nx run-native <file|binary> [--time]");
                    process::exit(1);
                }
            };

            if cmd_run_native(path, time_flag, &passthrough).is_err() {
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

            let path = match file_path {
                Some(p) => p,
                None => {
                    eprintln!("error: 'dump-nir' requires a source file path\n\nUsage: nx dump-nir <file.nx> [--opt-level <0|1|2|3>]");
                    process::exit(1);
                }
            };

            if cmd_dump_nir(path, opt_level).is_err() {
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

            let path = match file_path {
                Some(p) => p,
                None => {
                    eprintln!("error: 'dump-llvm' requires a source file path\n\nUsage: nx dump-llvm <file.nx> [--opt-level <0|1|2|3>]");
                    process::exit(1);
                }
            };

            if cmd_dump_llvm(path, opt_level).is_err() {
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
