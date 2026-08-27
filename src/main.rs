use lime::{
    CompileMode, CompileOptions, CompileReport, abiverify, charger, compile_pipeline,
    format_lime_source,
};
use std::env;
use std::fs;

fn print_usage() {
    eprintln!("Lime compiler");
    eprintln!("Usage:");
    eprintln!("  lime build <path> [--emit-ll] [--emit-object] [--release]  Build to binary");
    eprintln!(
        "  lime run   <path> [--emit-ll]                              Execute via interpreter (deprecated)"
    );
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
    let path = rest
        .iter()
        .find(|a| !a.starts_with("--") && a != &"-v")
        .cloned()?;
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

fn cli_build(path: &str, emit_ll: bool, _emit_object: bool, release: bool, verbose: bool) {
    let opts = CompileOptions {
        emit_ll,
        // `lime build` always produces an executable; the `--emit-object` flag
        // is retained for API compatibility but does not gate executable output.
        emit_object: true,
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

// Detect the LLVM toolchain bindir from the environment or a known location.
// This mirrors the logic in lib.rs::llvm_bindir but returns a String (fallback ".").
fn llvm_bindir() -> String {
    if let Ok(p) = std::env::var("LIME_LLVM_BIN") {
        return p;
    }
    // Check explicit prefix env vars.
    for var in &["LLVM_SYS_221_PREFIX", "LIME_LLVM_PREFIX"] {
        if let Ok(prefix) = std::env::var(var) {
            let bindir = std::path::Path::new(&prefix).join("bin");
            if bindir.join("opt.exe").exists() || bindir.join("opt").exists() {
                return bindir.to_str().unwrap().to_string();
            }
        }
    }
    // Check if opt/llc are directly on PATH.
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            if dir.join("opt.exe").exists() || dir.join("opt").exists() {
                return dir.to_str().unwrap().to_string();
            }
        }
    }
    // Fallback: derive from `clang` on PATH if present.
    ".".to_string()
}

fn cli_charger(sub: &str, rest: &[String]) {
    match sub {
        "install" => {
            let source = match rest.iter().find(|a| !a.starts_with("--")) {
                Some(s) => s.clone(),
                None => {
                    eprintln!("charger install <library-source-or-dir>");
                    std::process::exit(1);
                }
            };
            match charger::install(&source, &llvm_bindir()) {
                Ok(r) => {
                    println!(
                        "charger: installed '{}' -> {}",
                        r.lib_name,
                        r.store_path.display()
                    );
                    println!("  functions: {}", r.api.functions.len());
                    println!("  structs:   {}", r.api.structs.len());
                }
                Err(e) => {
                    eprintln!("charger install failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "list" => {
            let installed = charger::list_installed();
            if installed.is_empty() {
                println!("charger: no libraries installed");
            } else {
                println!("charger: installed libraries:");
                for l in installed {
                    println!("  {}", l);
                }
            }
        }
        "verify-abi" => {
            let lib = match rest.iter().find(|a| !a.starts_with("--")) {
                Some(s) => s.clone(),
                None => {
                    eprintln!("charger verify-abi <library>");
                    std::process::exit(1);
                }
            };
            match charger::verify_abi(&lib, &llvm_bindir()) {
                Ok(checks) => {
                    let mut all_pass = true;
                    for c in &checks {
                        if !c.pass {
                            all_pass = false;
                        }
                        println!(
                            "  [{}] {} : expected={} measured={}",
                            if c.pass { "PASS" } else { "FAIL" },
                            c.item,
                            c.expected,
                            c.measured
                        );
                    }
                    if all_pass {
                        println!("verify-abi: ALL {} CHECKS PASS", checks.len());
                    } else {
                        eprintln!("verify-abi: MISMATCH DETECTED");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("charger verify-abi failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "verify-semantics" => {
            let lib = match rest.iter().find(|a| !a.starts_with("--")) {
                Some(s) => s.clone(),
                None => {
                    eprintln!("charger verify-semantics <library>");
                    std::process::exit(1);
                }
            };
            match charger::verify_semantics(&lib) {
                Ok(checks) => {
                    let mut all_pass = true;
                    for c in &checks {
                        if !c.pass {
                            all_pass = false;
                        }
                        println!(
                            "  [{}] {} : {}",
                            if c.pass { "PASS" } else { "FAIL" },
                            c.item,
                            c.detail
                        );
                    }
                    if checks.is_empty() {
                        println!(
                            "verify-semantics: no semantic metadata for '{}' (all unknown — correct)",
                            lib
                        );
                    } else if all_pass {
                        println!("verify-semantics: ALL {} CHECKS PASS", checks.len());
                    } else {
                        eprintln!("verify-semantics: INVALID METADATA DETECTED");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("charger verify-semantics failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "verify-contract" => {
            // Iteration 31: permanent ABI CONTRACT gate. Compares a
            // human-reviewed frozen contract (bench_clang/abi_contracts/<lib>.json)
            // against the generated lime-iface.lime + manifest.toml of the
            // newest store entry. Detects signature drift, dangling shim
            // references, platform drift, and forbidden regression shapes.
            let libs: Vec<String> = rest
                .iter()
                .filter(|a| !a.starts_with("--"))
                .cloned()
                .collect();
            if libs.is_empty() {
                eprintln!("charger verify-contract <library> [library ...]");
                std::process::exit(1);
            }
            let refs: Vec<&str> = libs.iter().map(|s| s.as_str()).collect();
            let (pass, fail, _fns, _refs, _syms) = abiverify::verify_all(&refs);
            println!("verify-contract: PASS={} FAIL={}", pass, fail);
            if fail > 0 {
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("charger: unknown subcommand '{}'", other);
            eprintln!("usage: lime charger install <source> | list");
            std::process::exit(1);
        }
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
            let path = args[2..]
                .iter()
                .find(|a| !a.starts_with("--") && a != &"-v")
                .cloned();
            match path {
                Some(p) => cli_build(&p, emit_ll, emit_object, release, verbose),
                None => {
                    print_usage();
                    return;
                }
            }
        }
        "run" => {
            let (path, emit_ll, verbose) = match cli_target(&args[2..]) {
                Some(t) => t,
                None => {
                    print_usage();
                    return;
                }
            };
            cli_run(&path, emit_ll, verbose);
        }
        "check" => {
            let (path, _, verbose) = match cli_target(&args[2..]) {
                Some(t) => t,
                None => {
                    print_usage();
                    return;
                }
            };
            cli_check(&path, verbose);
        }
        "fmt" => {
            let write = args.iter().any(|a| a == "--write");
            let path = args[2..].iter().find(|a| !a.starts_with("--")).cloned();
            match path {
                Some(p) => cli_fmt(&p, write),
                None => {
                    print_usage();
                    return;
                }
            }
        }
        "charger" => {
            let sub = args.get(2).map(|s| s.as_str());
            let rest = if args.len() > 3 { &args[3..] } else { &[] };
            match sub {
                Some(s) => cli_charger(s, rest),
                None => {
                    print_usage();
                    return;
                }
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
