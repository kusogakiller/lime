use std::env;
use std::fs;
use lime::{compile_pipeline, CompileMode, CompileOptions, CompileReport, format_lime_source};

fn print_usage() {
    eprintln!("Lime compiler");
    eprintln!("Usage:");
    eprintln!("  lime build <path> [--emit-ll] [--emit-object] [--release]  Build to binary");
    eprintln!("  lime run   <path> [--emit-ll]                              Execute via interpreter (deprecated)");
    eprintln!("  lime check <path>                                          Type-check only");
    eprintln!("  lime fmt   <file.lime> [--write]                           Format source");
    eprintln!("  lime <path> [--emit-ll] [--verbose|-v]                     Shorthand for `run`");
    eprintln!();
    eprintln!("  <path> is a `.lime` file or a `citrus.toml` project manifest.");
    eprintln!("  For projects, `lime run` is deprecated; use `citrus run` instead.");
    eprintln!("  --emit-ll       Emit textual LLVM IR (.ll)");
    eprintln!("  --emit-object   Emit object file and link to executable");
    eprintln!("  --release       Enable optimizations (-O2 equivalent)");
    eprintln!("  --verbose, -v   Print compiler diagnostics to stderr.");
}

fn cli_target(rest: &[String]) -> Option<(String, bool, bool)> {
    let emit_ll = rest.iter().any(|a| a == "--emit-ll");
    let verbose = rest.iter().any(|a| a == "--verbose" || a == "-v");
    let path = rest.iter().find(|a| !a.starts_with("--") && a != &"-v").cloned()?;
    Some((path, emit_ll, verbose))
}

fn cli_finish(path: &str, mode: CompileMode, result: Result<CompileReport, String>) {
    match result {
        Ok(report) => {
            if report.removed_functions > 0 && mode != CompileMode::Run {
                eprintln!(
                    "optimizer: removed {} unused function(s)",
                    report.removed_functions
                );
            }
            for w in &report.warnings {
                eprintln!("{}", w);
            }
            if let Some(ll) = &report.emitted_ll {
                eprintln!("LLVM IR written to {}", ll);
            }
            if let Some(obj) = &report.emitted_obj {
                eprintln!("object written to {}", obj);
            }
            if let Some(exe) = &report.emitted_exe {
                eprintln!("executable written to {}", exe);
            }
            match mode {
                CompileMode::Build => println!("ok: {} built successfully", path),
                CompileMode::Check => println!("ok: {} type-checks cleanly", path),
                CompileMode::Run => {}
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

fn cli_build(path: &str, emit_ll: bool, emit_object: bool, release: bool, verbose: bool) {
    let opts = CompileOptions {
        emit_ll,
        emit_object,
        optimize: true,
        release,
        verbose,
    };
    let result = compile_pipeline(path, CompileMode::Build, &opts);
    cli_finish(path, CompileMode::Build, result);
}

fn cli_run(path: &str, emit_ll: bool, verbose: bool) {
    if path.ends_with("citrus.toml") {
        eprintln!(
            "warning: `lime run` on a project is deprecated; use `citrus run` to build and run"
        );
    }
    let opts = CompileOptions {
        emit_ll,
        emit_object: false,
        optimize: false,
        release: false,
        verbose,
    };
    let result = compile_pipeline(path, CompileMode::Run, &opts);
    cli_finish(path, CompileMode::Run, result);
}

fn cli_check(path: &str, verbose: bool) {
    let opts = CompileOptions {
        emit_ll: false,
        emit_object: false,
        optimize: false,
        release: false,
        verbose,
    };
    let result = compile_pipeline(path, CompileMode::Check, &opts);
    cli_finish(path, CompileMode::Check, result);
}

fn cli_fmt(path: &str, write: bool) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read '{}': {}", path, e);
            std::process::exit(1);
        }
    };
    let formatted = format_lime_source(&source);
    if write {
        match fs::write(path, &formatted) {
            Ok(_) => println!("formatted {}", path),
            Err(e) => {
                eprintln!("error: cannot write '{}': {}", path, e);
                std::process::exit(1);
            }
        }
    } else {
        print!("{}", formatted);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "build" => {
            let emit_ll = args.iter().any(|a| a == "--emit-ll");
            let emit_object = args.iter().any(|a| a == "--emit-object");
            let release = args.iter().any(|a| a == "--release");
            let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
            let path = args[2..].iter().find(|a| !a.starts_with("--") && a != &"-v").cloned();
            match path {
                Some(p) => cli_build(&p, emit_ll, emit_object, release, verbose),
                None => { print_usage(); return; }
            }
        }
        "run" => {
            let (path, emit_ll, verbose) = match cli_target(&args[2..]) {
                Some(t) => t,
                None => { print_usage(); return; }
            };
            cli_run(&path, emit_ll, verbose);
        }
        "check" => {
            let (path, _, verbose) = match cli_target(&args[2..]) {
                Some(t) => t,
                None => { print_usage(); return; }
            };
            cli_check(&path, verbose);
        }
        "fmt" => {
            let write = args.iter().any(|a| a == "--write");
            let path = args[2..].iter().find(|a| !a.starts_with("--")).cloned();
            match path {
                Some(p) => cli_fmt(&p, write),
                None => { print_usage(); return; }
            }
        }
        "-h" | "--help" | "help" => {
            print_usage();
        }
        _ => {
            let emit_ll = args.iter().any(|a| a == "--emit-ll");
            let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");
            let source_path = if args[1].starts_with("--") {
                print_usage();
                return;
            } else {
                args[1].clone()
            };
            cli_run(&source_path, emit_ll, verbose);
        }
    }
}
