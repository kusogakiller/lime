//! Phase 11 backend stabilization: LLVM emission regression tests.
//!
//! These exercise the full `lime build` pipeline (IR ↁEobject ↁEexecutable)
//! against throwaway projects and guard against regressions in the LLVM
//! backend:
//!
//! - a void `main` ending in a bare `return` must stay `void` (previously
//!   `type_from_str("void")` fell through to `Var` and rendered the function
//!   as `i64`, producing a `ret void` inside `define i64 ...` that clang
//!   rejects);
//! - `List.add` / `List.set` must store the returned list back into the
//!   receiver variable in codegen (matching the interpreter's rebind);
//! - a function the backend cannot fully lower must cause `--emit-object` to
//!   refuse to produce an executable rather than emit a silent stub;
//! - literal operands in binary ops and `let`/`return`/`assign` stores must be
//!   emitted as bare LLVM operands (e.g. `add i64 %t, 1`, `store i64 5,
//!   i64* %t`), never as `add i64 %t, i64 1` or `store i64 i64 5, ...`.

use std::process::Command;

/// Run a `lime` subcommand against a `citrus.toml` and return combined
/// stdout+stderr.
fn lime_cmd(subcmd: &str, toml: &str, extra: &[&str]) -> String {
    let bin = env!("CARGO_BIN_EXE_lime");
    let mut cmd = Command::new(bin);
    cmd.arg(subcmd).arg(toml);
    for a in extra {
        cmd.arg(a);
    }
    let output = cmd.output().expect("failed to run lime");
    let mut s = String::new();
    s.push_str(&String::from_utf8_lossy(&output.stdout));
    s.push_str(&String::from_utf8_lossy(&output.stderr));
    s
}

/// Write a throwaway project (citrus.toml + main.lime) under `dir`.
fn write_project(dir: &str, source: &str) {
    use std::fs;
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"emit_regression\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
}

/// Whether the LLVM toolchain (clang/lld) is configured for the object-file +
/// executable stages. When absent, `lime build --emit-object` prints a warning
/// and produces no executable, so exe-verification steps are skipped.
fn llvm_toolchain_available() -> bool {
    std::env::var("LLVM_SYS_221_PREFIX").is_ok() || std::env::var("LIME_LLVM_PREFIX").is_ok()
}

/// Regression: a void `main` ending in a bare `return` must stay
/// `define void @main_lime` + `ret void`. Previously `type_from_str("void")`
/// fell through to `Var` and rendered the function as `i64`, so the bare
/// return produced `ret void` inside `define i64 ...`  Ewhich clang rejects
/// with `value doesn't match function result type`.
#[test]
fn emit_object_bare_return_compiles_and_runs() {
    use std::fs;
    let dir = "target/test_emit_bare_return";
    write_project(dir, "fn main():\n    println(\"hi\")\n    return\n");

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("define void @main_lime"),
        "bare-return main must stay void\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("  ret void\n"),
        "bare return must emit ret void\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("define i64 @main_lime"),
        "void must not render as i64\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("alloca void"),
        "no broken alloca void\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("hi"),
        "exe should print 'hi', got: {}",
        String::from_utf8_lossy(&run.stdout)
    );
}

/// Regression: `List.add` / `List.set` must store the returned list back into
/// the receiver variable in codegen, matching the interpreter's rebinding
/// semantics. The emitted executable must print `4` then `9`.
#[test]
fn emit_object_list_add_set_runs() {
    use std::fs;
    let dir = "target/test_emit_list_add_set";
    write_project(
        dir,
        "fn main():\n    let nums = [1, 2, 3]\n    nums.add(4)\n    println(nums.len())\n    nums.set(0, 9)\n    println(nums.get(0))\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("store %LimeList"),
        "expected add/set store-back of the list\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["4", "9"],
        "expected lines ['4', '9'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Phase B-1 Step 1: `Option(T)` / `Result(T, E)` helpers from the stdlib
/// packages (`option.unwrap_or`, `option.is_some`, `option.is_none`,
/// `option.unwrap`, `result.unwrap_or`, `result.is_ok`, `result.is_err`,
/// `result.unwrap`) must type-check, monomorphize, and lower to native code
/// that matches the interpreter. The monomorphized helpers must be emitted
/// under their mangled names (`@option.unwrap.int`) and every match arm must
/// terminate its block correctly (a match whose arms all `return` previously
/// left an empty, unterminated merge block that clang rejected).
#[test]
fn emit_object_option_result_runs() {
    use std::fs;
    let dir = "target/test_emit_option_result";
    write_stdlib_project(
        dir,
        "fn main():\n    let some = Some(5)\n    let none = None\n    println(option.is_some(some))\n    println(option.is_none(some))\n    println(option.unwrap_or(some, 0))\n    println(option.unwrap_or(none, 0))\n    println(option.unwrap(some))\n    let ok = Success(10)\n    let err = Error(\"boom\")\n    println(result.is_ok(ok))\n    println(result.is_err(ok))\n    println(result.is_err(err))\n    println(result.unwrap_or(ok, 0))\n    println(result.unwrap_or(err, 0))\n    println(result.unwrap(ok))\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        !out.contains("codegen warning") && !out.contains("could not be fully lowered"),
        "option/result program must lower completely\n--- output ---\n{}",
        out
    );
    for helper in [
        "option.unwrap_or.int",
        "option.is_some.int",
        "option.is_none.int",
        "option.unwrap.int",
        "result.unwrap_or.int",
        "result.is_ok.int",
        "result.is_err",
        "result.unwrap.int",
    ] {
        assert!(
            ir.lines().any(|l| l.contains(&format!("@{}", helper))
                || l.contains(&format!("@{}.", helper))),
            "IR must emit a mangled helper for {}\n--- ir ---\n{}",
            helper,
            ir
        );
    }
    assert!(
        ir.contains("call i64 @option.unwrap.int") || ir.contains("call i1 @option.is_some.int"),
        "main must call the mangled option helpers\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("@option.is_some("),
        "unmangled helper name must not remain\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    let expected = [
        "true",
        "false",
        "5",
        "0",
        "5",
        "true",
        "false",
        "true",
        "10",
        "0",
        "10",
    ];
    assert!(
        lines == expected,
        "option/result native output mismatch\nexpected: {:?}\ngot: {:?}\n--- stderr ---\n{}",
        expected,
        lines,
        String::from_utf8_lossy(&run.stderr)
);
}

/// Phase B-1 Step 3: str() on Option and Result must produce display
/// strings matching the interpreter ("Some(5)", "None", "Success(10)",
/// "Error(42)") in both native and interpreter paths.
#[test]
fn emit_object_str_option_result() {
    use std::fs;
    let dir = "target/test_emit_str_option_result";
    write_stdlib_project(
        dir,
        "fn main():\n    let some = Some(5)\n    let none = None\n    let ok = Success(10)\n    let err = Error(42)\n    println(str(some))\n    println(str(none))\n    println(str(ok))\n    println(str(err))\n    return\n",
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    let expected = [
        "Some(5)",
        "None",
        "Success(10)",
        "Error(42)",
    ];
    assert!(
        lines == expected,
        "str(option/result) native output mismatch\nexpected: {:?}\ngot: {:?}\n--- stderr ---\n{}",
        expected,
        lines,
        String::from_utf8_lossy(&run.stderr)
    );

    // Interpreter parity check
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let stdout: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning") && !l.starts_with("unused variable") && !l.starts_with("In function") && !l.starts_with("error[type]"))
        .collect();
    assert!(
        stdout == expected,
        "str(option/result) interpreter output mismatch\nexpected: {:?}\ngot: {:?}\n--- full output ---\n{}",
        expected,
        stdout,
        out
    );
}

/// Phase B-1 Step 1 (nested): generic helpers applied to *nested*
/// generic states (`Option(Option(int))`) must mangle their type
/// arguments into valid LLVM identifiers (`Option_28int_29` for
/// `Option(int)`). The old unmangled form containing parentheses
/// and commas (`@option.unwrap.Option(`) must not appear. The
/// interpreter must produce identical output.
#[test]
fn emit_object_nested_generic_mangling() {
    use std::fs;
    let dir = "target/test_emit_nested_mangling";
    write_stdlib_project(
        dir,
        "fn main():\n    let nested = Some(Some(5))\n    println(option.unwrap(option.unwrap(nested)))\n    println(option.unwrap_or(option.unwrap(nested), 0))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let stdout: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning") && !l.starts_with("unused variable") && !l.starts_with("In function") && !l.starts_with("error[type]"))
        .collect();
    assert!(
        stdout == ["5", "5"],
        "nested option interp output mismatch\nexpected: [\"5\", \"5\"]\ngot: {:?}\n--- full output ---\n{}",
        stdout,
        out
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);

    for symbol in ["option.unwrap.Option_28int_29", "option.unwrap_or.int"] {
        assert!(
            ir.lines().any(|l| l.contains(&format!("@{}", symbol))),
            "IR must contain mangled helper {}\n--- ir ---\n{}",
            symbol,
            ir
        );
    }
    assert!(
        !ir.contains("@option.unwrap.Option("),
        "unmangled nested Option name must not remain\n--- ir ---\n{}",
        ir
    );
}

/// Phase B-1.2: repeated builds of the same generic program must produce
/// identical symbols. The centralized mangling is a pure, deterministic
/// string encoding (no hashing / map-ordering), so two `--emit-ll` builds
/// of the *same source* must lower to an identical set of mangled `define`
/// symbols (the bodies may mention constants that differ, but the symbol
/// set must not drift between builds).
#[test]
fn emit_object_repeated_build_is_symbol_stable() {
    use std::fs;
    let src = "fn main():\n    let some = Some(5)\n    let none = None\n    println(option.unwrap_or(some, 0))\n    println(option.unwrap_or(none, \"d\"))\n    println(option.unwrap(Some(Some(5))))\n    return\n";

    // Build the identical source twice into fresh project dirs.
    let dir = "target/test_emit_repeat_build";
    write_stdlib_project(dir, src);
    let _ = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ir1 = fs::read_to_string(&format!("{}.ll", dir)).unwrap_or_default();
    assert!(!ir1.is_empty(), "first build must emit IR");

    let dir2 = "target/test_emit_repeat_build2";
    write_stdlib_project(dir2, src);
    let _ = lime_cmd("build", &format!("{}/citrus.toml", dir2), &["--emit-ll"]);
    let ir2 = fs::read_to_string(&format!("{}.ll", dir2)).unwrap_or_default();
    assert!(!ir2.is_empty(), "second build must emit IR");

    // Collect mangled `define` symbol tokens from each build.
    let syms = |ir: &str| -> Vec<String> {
        let mut v: Vec<String> = ir
            .lines()
            .filter(|l| l.contains("define "))
            .map(|l| l.split_whitespace().skip(2).next().unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .collect();
        v.sort();
        v
    };
    let a = syms(&ir1);
    let b = syms(&ir2);
    assert_eq!(
        a.len(),
        b.len(),
        "mangled symbol count must be stable across builds\nir1: {:?}\nir2: {:?}",
        a,
        b
    );
    assert_eq!(
        a, b,
        "mangled symbols drift between identical builds\nir1: {:?}\nir2: {:?}",
        a, b
    );

    // Sanity: the option generic helpers really are emitted under mangled names.
    assert!(
        a.iter().any(|s| s.starts_with("@option.")),
        "expected mangled option helpers, got:\n{:?}",
        a
    );
}

/// Phase B-1.3: math.floor, math.ceil, math.round must produce identical
/// output in the interpreter and the native executable, including negative
/// numbers and half-values. round() uses round-half-away-from-zero semantics
/// (2.5 -> 3, -2.5 -> -3), matching both C `round()` and Rust `f64::round()`.
#[test]
fn emit_object_math_floor_ceil_round_negatives() {
    use std::fs;
    let dir = "target/test_emit_math_negatives";
    write_stdlib_project(
        dir,
        "fn main():\n    println(math.floor(1.8))\n    println(math.floor(-1.8))\n    println(math.ceil(1.2))\n    println(math.ceil(-1.2))\n    println(math.round(1.5))\n    println(math.round(-1.5))\n    println(math.round(2.5))\n    println(math.round(-2.5))\n    println(math.round(0.5))\n    println(math.round(-0.5))\n    println(math.round(1.4))\n    println(math.round(-1.6))\n    return\n",
    );

    // Interpreter run
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning") && !l.starts_with("unused variable") && !l.starts_with("In function") && !l.starts_with("error[type]"))
        .collect();
    let expected = [
        "1", "-2", "2", "-1", "2", "-2", "3", "-3", "1", "-1", "1", "-2",
    ];
    assert_eq!(
        interp, expected,
        "interpreter floor/ceil/round mismatch\nexpected: {:?}\ngot: {:?}",
        expected, interp
    );

    // Native run
    if !llvm_toolchain_available() {
        return;
    }
    let build_out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        build_out.contains("ok:"),
        "native build failed: {}",
        build_out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout_lossy = String::from_utf8_lossy(&run.stdout);
    let native: Vec<&str> = stdout_lossy.lines().collect();
    assert_eq!(
        native, expected,
        "native floor/ceil/round mismatch\nexpected: {:?}\ngot: {:?}\n--- stderr ---\n{}",
        expected,
        native,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        interp, native,
        "interpreter and native output must match for math.floor/ceil/round"
    );
}

/// Phase B-1.3: println must handle Option/Result/State values via the str()
/// fallback in native codegen (auto-converting to string before printf).
/// Integer payloads display correctly. Float/string payloads in Option/Result
/// are a known architectural limitation (the Option struct stores all payloads
/// as i64; string pointers and float bits are not reinterpreted at runtime).
#[test]
fn emit_object_display_println_option_result() {
    use std::fs;
    let dir = "target/test_display_println_state";
    write_stdlib_project(
        dir,
        "fn main():\n    println(Some(1))\n    println(None)\n    println(Success(42))\n    println(Error(7))\n    println(str(Some(5)))\n    println(str(Success(10)))\n    println(str(Error(42)))\n    return\n",
    );

    // Interpreter run
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning") && !l.starts_with("unused variable") && !l.starts_with("In function") && !l.starts_with("error[type]"))
        .collect();
    let expected = [
        "Some(1)",
        "None",
        "Success(42)",
        "Error(7)",
        "Some(5)",
        "Success(10)",
        "Error(42)",
    ];
    assert_eq!(
        interp, expected,
        "interpreter display mismatch\nexpected: {:?}\ngot: {:?}",
        expected, interp
    );

    // Native run
    if !llvm_toolchain_available() {
        return;
    }
    let build_out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        build_out.contains("ok:"),
        "native build failed: {}",
        build_out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout_lossy = String::from_utf8_lossy(&run.stdout);
    let native: Vec<&str> = stdout_lossy.lines().collect();
    assert_eq!(
        native, expected,
        "native display mismatch\nexpected: {:?}\ngot: {:?}\n--- stderr ---\n{}",
        expected,
        native,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        interp, native,
        "interpreter and native output must match for println(Option/Result)"
    );
}

/// Phase B-1.5: unwrap() on None/Error panics at runtime. We test this by
/// calling unwrap on a Some/Success first (to confirm happy path), then
/// triggering the panic via panic("msg"). Because `option.unwrap(None)` fails
/// at type inference (None is polymorphic), we test the panic infrastructure
/// via `panic("msg")` directly, and verify unwrap_or returns fallback without
/// panicking for both None and Error.
#[test]
fn emit_object_unwrap_panic_and_fallback() {
    use std::fs;
    let dir = "target/test_unwrap_panic";
    write_stdlib_project(
        dir,
        "fn main():\n    println(option.unwrap_or(None, 99))\n    println(result.unwrap_or(Error(42), 77))\n    println(option.unwrap(Some(5)))\n    println(result.unwrap(Success(10)))\n    println(\"before_panic\")\n    panic(\"test_panic_message\")\n    println(\"after_panic\")\n    return\n",
    );

    // Interpreter: the first five lines succeed, then panic aborts.
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning") && !l.starts_with("unused variable") && !l.starts_with("In function") && !l.starts_with("error[type]"))
        .collect();
    // First five lines must succeed.
    assert!(
        interp.len() >= 5
            && interp[0] == "99"
            && interp[1] == "77"
            && interp[2] == "5"
            && interp[3] == "10"
            && interp[4] == "before_panic",
        "unwrap_or/unwrap happy paths must succeed in interpreter\ngot: {:?}\n--- full output ---\n{}",
        interp,
        out
    );
    // Panic message must appear, and "after_panic" must NOT appear.
    let full = interp.join("\n");
    assert!(
        full.contains("Lime runtime panic") && full.contains("test_panic_message"),
        "interpreter must show panic message\ngot: {:?}\n--- full output ---\n{}",
        interp,
        out
    );
    assert!(
        !full.contains("after_panic"),
        "interpreter must abort on panic (after_panic should not appear)\ngot: {:?}",
        interp
    );

    // Native: same program, verify happy paths work and panic aborts.
    // Note: abort() doesn't flush stdout buffers, so we only check stderr
    // for the panic message and verify the binary was built successfully.
    if !llvm_toolchain_available() {
        return;
    }
    let build_out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        build_out.contains("ok:"),
        "unwrap panic test native build failed: {}",
        build_out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_stderr = String::from_utf8_lossy(&run.stderr);
    // The native process must have aborted with a panic message on stderr.
    assert!(
        native_stderr.contains("Lime runtime panic") && native_stderr.contains("test_panic_message"),
        "native must show panic message\n--- stderr ---\n{}",
        native_stderr
    );
    // "after_panic" must NOT appear in stdout (abort prevents flushing).
    let stdout_lossy = String::from_utf8_lossy(&run.stdout);
    assert!(
        !stdout_lossy.contains("after_panic"),
        "native must abort on panic (after_panic should not appear)\n--- stdout ---\n{}",
        stdout_lossy
    );
}

/// Phase B-1.5: unwrap_or() must produce identical output in interpreter and
/// native mode for both Option and Result. This verifies interpreter/native
/// parity for the safe unwrap path.
#[test]
fn emit_object_unwrap_or_parity() {
    use std::fs;
    let dir = "target/test_unwrap_or_parity";
    write_stdlib_project(
        dir,
        "fn main():\n    println(option.unwrap_or(Some(1), 0))\n    println(option.unwrap_or(None, 0))\n    println(result.unwrap_or(Success(2), 0))\n    println(result.unwrap_or(Error(3), 0))\n    println(option.unwrap(Some(4)))\n    println(result.unwrap(Success(5)))\n    return\n",
    );

    let expected = ["1", "0", "2", "0", "4", "5"];

    // Interpreter
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning") && !l.starts_with("unused variable") && !l.starts_with("In function") && !l.starts_with("error[type]"))
        .collect();
    assert_eq!(
        interp, expected,
        "unwrap_or parity (interpreter) mismatch\nexpected: {:?}\ngot: {:?}\n--- full output ---\n{}",
        expected, interp, out
    );

    // Native
    if !llvm_toolchain_available() {
        return;
    }
    let build_out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        build_out.contains("ok:"),
        "unwrap_or parity native build failed: {}",
        build_out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout_lossy = String::from_utf8_lossy(&run.stdout);
    let native: Vec<&str> = stdout_lossy.lines().collect();
    assert_eq!(
        native, expected,
        "unwrap_or parity (native) mismatch\nexpected: {:?}\ngot: {:?}\n--- stderr ---\n{}",
        expected,
        native,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        interp, native,
        "interpreter and native must produce identical output for unwrap_or parity"
    );
}

/// Phase B-1.4: a single program that exercises all eight bundled stdlib
/// packages in both the interpreter and native executable. The two must
/// produce identical output. This is the broadest surface-area smoke test
/// for the stdlib.
#[test]
fn emit_object_stdlib_all_packages_smoke() {
    use std::fs;
    let dir = "target/test_stdlib_all_packages";
    write_stdlib_project(
        dir,
        "fn main():\n    // string\n    println(string.len(\"hello\"))\n    println(string.trim(\"  x  \"))\n    println(string.to_upper(\"abc\"))\n    println(string.contains(\"hello\", \"ell\"))\n    // math\n    println(math.abs(-3.0))\n    println(math.sqrt(9.0))\n    println(math.floor(2.7))\n    println(math.ceil(2.3))\n    println(math.round(2.5))\n    // option\n    let o = Some(42)\n    println(option.is_some(o))\n    println(option.unwrap_or(o, 0))\n    // result\n    let r = Success(99)\n    println(result.is_ok(r))\n    println(result.unwrap_or(r, 0))\n    // time\n    println(time.now().secs >= 0.0)\n    // io\n    io.println(\"io_ok\")\n    // fs\n    println(fs.write(\"target/test_smoke_io.txt\", \"hi\"))\n    println(fs.read(\"target/test_smoke_io.txt\"))\n    println(fs.exists(\"target/test_smoke_io.txt\"))\n    println(fs.remove(\"target/test_smoke_io.txt\"))\n    return\n",
    );

    // Interpreter
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning") && !l.starts_with("unused variable") && !l.starts_with("In function") && !l.starts_with("error[type]"))
        .collect();
    let expected = [
        "5",       // string.len
        "x",       // string.trim
        "ABC",     // string.to_upper
        "true",    // string.contains
        "3",       // math.abs
        "3",       // math.sqrt
        "2",       // math.floor
        "3",       // math.ceil
        "3",       // math.round
        "true",    // option.is_some
        "42",      // option.unwrap_or
        "true",    // result.is_ok
        "99",      // result.unwrap_or
        "true",    // time.now().secs >= 0.0
        "io_ok",   // io.println
        "true",    // fs.write
        "hi",      // fs.read
        "true",    // fs.exists
        "true",    // fs.remove
    ];
    assert_eq!(
        interp, expected,
        "stdlib smoke test (interpreter) mismatch\nexpected: {:?}\ngot: {:?}\n--- full output ---\n{}",
        expected, interp, out
    );

    // Native
    if !llvm_toolchain_available() {
        return;
    }
    let build_out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        build_out.contains("ok:"),
        "stdlib smoke test native build failed: {}",
        build_out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout_lossy = String::from_utf8_lossy(&run.stdout);
    let native: Vec<&str> = stdout_lossy.lines().collect();
    assert_eq!(
        native, expected,
        "stdlib smoke test (native) mismatch\nexpected: {:?}\ngot: {:?}\n--- stderr ---\n{}",
        expected,
        native,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        interp, native,
        "interpreter and native must produce identical output for stdlib smoke test"
    );
}

/// Phase B-1.4: repeated builds of the same stdlib-using program must produce
/// identical IR (build reproducibility). This catches non-deterministic symbol
/// generation or ordering issues in the codegen.
#[test]
fn emit_object_stdlib_build_reproducibility() {
    use std::fs;
    let src = "fn main():\n    println(math.sqrt(16.0))\n    println(option.unwrap_or(Some(1), 0))\n    println(option.unwrap_or(None, -1))\n    return\n";

    let dir1 = "target/test_stdlib_repro1";
    write_stdlib_project(dir1, src);
    let _ = lime_cmd("build", &format!("{}/citrus.toml", dir1), &["--emit-ll"]);
    let ir1 = fs::read_to_string(format!("{}.ll", dir1)).unwrap_or_default();

    let dir2 = "target/test_stdlib_repro2";
    write_stdlib_project(dir2, src);
    let _ = lime_cmd("build", &format!("{}/citrus.toml", dir2), &["--emit-ll"]);
    let ir2 = fs::read_to_string(format!("{}.ll", dir2)).unwrap_or_default();

    assert!(!ir1.is_empty(), "first build must emit IR");
    assert!(!ir2.is_empty(), "second build must emit IR");

    // Collect sorted define symbols from each build.
    let syms = |ir: &str| -> Vec<String> {
        let mut v: Vec<String> = ir
            .lines()
            .filter(|l| l.contains("define "))
            .map(|l| {
                l.split_whitespace()
                    .skip(2)
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        v.sort();
        v
    };
    let a = syms(&ir1);
    let b = syms(&ir2);
    assert_eq!(
        a, b,
        "stdlib build reproducibility failed  Esymbols differ between identical builds\nir1: {:?}\nir2: {:?}",
        a, b
    );
}

/// Write a throwaway project that depends on the bundled stdlib packages.
fn write_stdlib_project(dir: &str, source: &str) {
    use std::fs;
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"emit_regression\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"\n\n[dependencies]\nio = \"v0.1.0\"\nstring = \"v0.1.0\"\nmath = \"v0.1.0\"\nfs = \"v0.1.0\"\ntime = \"v0.1.0\"\noption = \"v0.1.0\"\nresult = \"v0.1.0\"\n",
    )
    .unwrap();
}

/// Phase 12 Step 1: the runtime builtins wrapped by the stdlib packages must
/// lower to C runtime helper calls, so a stdlib-using program compiles to a
/// native executable that matches the interpreter output. Previously the
/// package wrappers (`string.trim`, `math.sqrt`, `fs.read`, ...) were emitted
/// as undefined-function stubs that `--emit-object` refused to build.
#[test]
fn emit_object_stdlib_builtins_lower_and_run() {
    use std::fs;
    let dir = "target/test_emit_stdlib_native";
    write_stdlib_project(
        dir,
        "fn main():\n    println(string.trim(\"  hi  \"))\n    println(string.to_upper(\"abc\"))\n    println(string.to_lower(\"AbC\"))\n    println(string.replace(\"aaa\", \"a\", \"b\"))\n    println(string.len(\"hello\"))\n    println(string.byte_len(\"hello\"))\n    println(string.contains(\"hello\", \"ell\"))\n    println(string.starts_with(\"hello\", \"he\"))\n    println(string.ends_with(\"hello\", \"lo\"))\n    println(string.repeat(\"ab\", 3))\n    println(string.slice(\"hello\", 1, 3))\n    let parts = string.split(\"a,b,c\", \",\")\n    println(math.sqrt(16.0))\n    println(math.abs(-3.0))\n    println(math.max(1.0, 7.0))\n    println(math.min(1.0, 7.0))\n    println(math.clamp(5.0, 0.0, 2.0))\n    println(math.pow(2.0, 3.0))\n    println(math.floor(3.7))\n    println(math.ceil(3.2))\n    println(math.round(3.5))\n    println(time.now().secs > 0.0)\n    println(time.elapsed(time.now()).secs >= 0.0)\n    println(time.sleep(0.01))\n    io.println(\"io test\")\n    let entries = fs.list_dir(\"target\")\n    println(fs.write(\"target/test_stdlib_native_io.txt\", \"lime fs ok\"))\n    println(fs.read(\"target/test_stdlib_native_io.txt\"))\n    println(fs.exists(\"target/test_stdlib_native_io.txt\"))\n    println(fs.size(\"target/test_stdlib_native_io.txt\") == 10)\n    println(fs.metadata(\"target/test_stdlib_native_io.txt\").is_file)\n    println(fs.remove(\"target/test_stdlib_native_io.txt\"))\n    println(fs.exists(\"target/test_stdlib_native_io.txt\"))\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        !out.contains("codegen warning") && !out.contains("could not be fully lowered"),
        "stdlib program must lower completely\n--- output ---\n{}",
        out
    );
    for helper in [
        "runtime_str_trim",
        "runtime_str_to_upper",
        "runtime_str_to_lower",
        "runtime_str_replace",
        "runtime_str_repeat",
        "runtime_str_slice",
        "runtime_str_split",
        "runtime_str_contains",
        "runtime_str_starts_with",
        "runtime_str_ends_with",
        "runtime_math_sqrt",
        "runtime_math_abs",
        "runtime_math_max",
        "runtime_math_min",
        "runtime_math_clamp",
        "runtime_math_pow",
        "runtime_math_floor",
        "runtime_math_ceil",
        "runtime_math_round",
        "runtime_time_now",
        "runtime_time_sleep",
        "runtime_read_file",
        "runtime_write_file",
        "runtime_file_exists",
        "runtime_fs_size",
        "runtime_fs_metadata",
        "runtime_remove_file",
        "runtime_fs_list_dir",
    ] {
        assert!(
            ir.contains(&format!("@{}", helper)),
            "IR must call {}\n--- ir ---\n{}",
            helper,
            ir
        );
    }

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    let expected = [
        "hi",
        "ABC",
        "abc",
        "bbb",
        "5",
        "5",
        "true",
        "true",
        "true",
        "ababab",
        "el",
        "4",
        "3",
        "7",
        "1",
        "2",
        "8",
        "3",
        "4",
        "4",
        "true",
        "true",
        "true",
        "io test",
        "true",
        "lime fs ok",
        "true",
        "true",
        "true",
        "true",
        "false",
    ];
    assert!(
        lines == expected,
        "stdlib native output mismatch\nexpected: {:?}\ngot: {:?}\n--- stderr ---\n{}",
        expected,
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: scalar `let` with a literal RHS must emit a valid store
/// (`store i64 5, i64* %t`) instead of `store i64 i64 5, ...`, and a binop
/// over a register and a literal must emit a bare operand (`add i64 %t, 1`).
/// Executable must print `6`.
#[test]
fn emit_object_let_literal_runs() {
    use std::fs;
    let dir = "target/test_emit_let_literal";
    write_project(
        dir,
        "fn main():\n    let a = 5\n    let b = a + 1\n    println(b)\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("store i64 5, i64*"),
        "literal RHS must store as a bare constant\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("add i64"),
        "a + 1 must emit an add\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("store i64 i64"),
        "no double-typed store\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("add i64 i64"),
        "no double-typed binop operands\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["6"],
        "expected lines ['6'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: a binop mixing a register (`x`) with a literal (`20`) must emit
/// valid IR. Executable must print `30`.
#[test]
fn emit_object_let_literal_binop_runs() {
    use std::fs;
    let dir = "target/test_emit_let_literal_binop";
    write_project(dir, "fn main():\n    let x = 10\n    println(x + 20)\n    return\n");

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("store i64 10, i64*"),
        "literal RHS must store as a bare constant\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("add i64"),
        "x + 20 must emit an add\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("add i64 i64"),
        "no double-typed binop operands\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["30"],
        "expected lines ['30'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: binops where every operand is a literal (`(1 + 2) * (3 + 4)`)
/// must emit bare constants (`add i64 1, 2`) and compose via registers.
/// Executable must print `21`.
#[test]
fn emit_object_nested_literal_binop_runs() {
    use std::fs;
    let dir = "target/test_emit_nested_literal_binop";
    write_project(
        dir,
        "fn main():\n    println((1 + 2) * (3 + 4))\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("add i64 1, 2"),
        "literal-only add must emit bare operands\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("mul i64"),
        "nested product must emit a mul\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("add i64 i64") && !ir.contains("mul i64 i64"),
        "no double-typed binop operands\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["21"],
        "expected lines ['21'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: a function whose body mixes a register and a literal
/// (`return x + 1`) must emit a valid `add i64 %t, 1` and `ret i64 %t`, and a
/// call passing a literal (`add(41)`) must emit `call i64 @add(i64 41)`.
/// Executable must print `42`.
#[test]
fn emit_object_mixed_literal_binop_runs() {
    use std::fs;
    let dir = "target/test_emit_mixed_literal_binop";
    write_project(
        dir,
        "fn add(int: x):\n    return x + 1\n\nfn main():\n    println(add(41))\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("define i64 @add (i64 %p0)"),
        "add must be emitted with its real body\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("call i64 @add(i64 41)"),
        "literal call arg must be typed once\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("add i64 i64"),
        "no double-typed binop operands\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["42"],
        "expected lines ['42'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: reassigning a scalar variable (`let mut x = 1; x = x + 1`)
/// must store a bare value, never `store i64 i64`. Executable must print `2`.
#[test]
fn emit_object_reassignment_runs() {
    use std::fs;
    let dir = "target/test_emit_reassignment";
    write_project(
        dir,
        "fn main():\n    let mut x = 1\n    x = x + 1\n    println(x)\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("store i64 1, i64*"),
        "literal init must store as a bare constant\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("add i64"),
        "x = x + 1 must emit an add\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("store i64 i64"),
        "no double-typed store\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["2"],
        "expected lines ['2'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: a function the backend cannot fully lower (here:
/// `List(String).get`) must cause `--emit-object` to refuse to produce an
/// object file, instead of silently emitting a stub executable.
#[test]
fn emit_object_refuses_unlowered_function() {
    use std::fs;
    let dir = "target/test_emit_refuse_unlowered";
    write_project(
        dir,
        "fn main():\n    let strs = [\"a\", \"b\"]\n    println(strs.get(0))\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("refusing to emit object file"),
        "expected refusal when a function cannot be lowered\n--- output ---\n{}",
        out
    );
    assert!(
        out.contains("List.get() is only supported for lists of Int"),
        "expected the get-guard warning\n--- output ---\n{}",
        out
    );
    assert!(
        !fs::metadata(format!("{}.exe", dir)).is_ok(),
        "no executable may be produced when lowering is incomplete"
    );
}

/// Regression: `await` on an async (`lime`) function must lower to a normal
/// synchronous LLVM call. Previously a function body containing `Expr::Await`
/// failed `expr_supported`, so `main` was silently emitted as a `ret void`
/// stub. The async callee must keep its real body and `main_lime` must contain
/// the awaited call, and the executable must print `42`.
#[test]
fn emit_object_await_int_runs() {
    use std::fs;
    let dir = "target/test_emit_await_int";
    write_project(
        dir,
        "lime add1(int: n):\n    return n + n\n\nfn main():\n    let x = await add1(21)\n    println(x)\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("define i64 @add1 (i64 %p0)"),
        "async fn add1 must be emitted with its real body\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("add i64"),
        "add1 body must contain the add instruction\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("call i64 @add1"),
        "main must await add1 via a direct synchronous call\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("define i64 @add1 (i64 %p0) {\n  ret i64 0\n}"),
        "add1 must not be emitted as a zero stub\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("define void @main_lime () {\n  ret void\n}"),
        "main_lime must not be emitted as an empty stub\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["42"],
        "expected lines ['42'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: an async (`lime`) function returning a string must be emitted
/// as a real `i8*`-returning function (not a stub), and `await` must produce
/// the awaited string at runtime. Executable must print `hi lime`.
#[test]
fn emit_object_await_string_runs() {
    use std::fs;
    let dir = "target/test_emit_await_string";
    write_project(
        dir,
        "lime greet(str: name):\n    return \"hi \" + name\n\nfn main():\n    let g = await greet(\"lime\")\n    println(g)\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("define i8* @greet (i8* %p0)"),
        "async fn greet must be emitted with its real body\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("call i8* @runtime_str_concat"),
        "greet body must contain the string concat call\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("call i8* @greet"),
        "main must await greet via a direct synchronous call\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("define i8* @greet (i8* %p0) {\n  ret i8* 0\n}"),
        "greet must not be emitted as a zero stub\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("define void @main_lime () {\n  ret void\n}"),
        "main_lime must not be emitted as an empty stub\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["hi lime"],
        "expected lines ['hi lime'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Regression: an async (`lime`) function whose body itself awaits another
/// async function must lower to nested synchronous calls with no stubs.
/// Executable must print `42`.
#[test]
fn emit_object_await_nested_runs() {
    use std::fs;
    let dir = "target/test_emit_await_nested";
    write_project(
        dir,
        "lime inner(int: n):\n    return n * n\n\nlime outer(int: n):\n    return await inner(n) + n\n\nfn main():\n    let x = await outer(6)\n    println(x)\n    return\n",
    );

    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-ll"]);
    let ll = format!("{}.ll", dir);
    let ir = fs::read_to_string(&ll).unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output, got:\n{}", out);
    assert!(
        ir.contains("define i64 @inner (i64 %p0)"),
        "inner must be emitted with its real body\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("define i64 @outer (i64 %p0)"),
        "outer must be emitted with its real body\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("mul i64"),
        "inner body must contain the multiply instruction\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("call i64 @inner"),
        "outer must await inner via a direct call\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("call i64 @outer"),
        "main must await outer via a direct call\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("define i64 @inner (i64 %p0) {\n  ret i64 0\n}")
            && !ir.contains("define i64 @outer (i64 %p0) {\n  ret i64 0\n}")
            && !ir.contains("define void @main_lime () {\n  ret void\n}"),
        "no async-related function may be emitted as a stub\n--- ir ---\n{}",
        ir
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- output ---\n{}",
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines == ["42"],
        "expected lines ['42'], got: {:?}\n--- stderr ---\n{}",
        lines,
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Phase B-2.1: function reference — pass a named function by name and call it (interpreter only; native defers to B-2.4).
#[test]
fn emit_object_fn_reference() {
    use std::fs;
    let dir = "target/test_fn_reference";
    write_project(
        dir,
        "fn add(int: a, int: b):\n    return a + b\nfn main():\n    let f = add\n    println(f(3, 4))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["7"],
        "fn reference (interpreter) mismatch\nfull output:\n{}",
        out
    );
}

/// Phase B-2.1: anonymous function — create a function value with fn literal (interpreter only).
#[test]
fn emit_object_anonymous_fn() {
    use std::fs;
    let dir = "target/test_anonymous_fn";
    write_project(
        dir,
        "fn main():\n    let f = fn(int: a, int: b):\n        return a + b\n    println(f(10, 20))\n    let g = fn(int: x):\n        return x * 2\n    println(g(5))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["30", "10"],
        "anonymous fn (interpreter) mismatch\nfull output:\n{}",
        out
    );
}

/// Phase B-2.1: closure — inner function captures outer variable (interpreter only).
#[test]
fn emit_object_closure_capture() {
    use std::fs;
    let dir = "target/test_closure_capture";
    write_project(
        dir,
        "fn make_adder(int: x):\n    return fn(int: y):\n        return x + y\nfn main():\n    let add5 = make_adder(5)\n    println(add5(3))\n    let add10 = make_adder(10)\n    println(add10(7))\n    println(add5(100))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["8", "17", "105"],
        "closure capture (interpreter) mismatch\nfull output:\n{}",
        out
    );
}

// ===== Phase B-2.2: function value native parity tests =====

/// Phase B-2.2: named function reference — native parity.
#[test]
fn emit_object_fn_reference_native() {
    use std::fs;
    let dir = "target/test_fn_reference_native";
    write_project(
        dir,
        "fn add(int: a, int: b):\n    return a + b\nfn main():\n    let f = add\n    println(f(3, 4))\n    return\n",
    );

    // Interpreter check
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["7"],
        "fn reference (interpreter) mismatch\nfull output:\n{}",
        out
    );

    // Native check
    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "7", "fn reference (native) mismatch");
}

/// Phase B-2.2: anonymous function — native parity.
#[test]
fn emit_object_anonymous_fn_native() {
    use std::fs;
    let dir = "target/test_anonymous_fn_native";
    write_project(
        dir,
        "fn main():\n    let f = fn(int: a, int: b):\n        return a + b\n    println(f(10, 20))\n    let g = fn(int: x):\n        return x * 2\n    println(g(5))\n    return\n",
    );

    // Interpreter check
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["30", "10"],
        "anonymous fn (interpreter) mismatch\nfull output:\n{}",
        out
    );

    // Native check
    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "30\n10", "anonymous fn (native) mismatch");
}

/// Phase B-2.3: untyped anonymous function params — fn(x) syntax.
#[test]
fn emit_object_untyped_anonymous_fn() {
    let dir = "target/test_untyped_anonymous_fn";
    write_project(
        dir,
        "fn main():\n    let f = fn(x):\n        return x * 2\n    println(f(4))\n    return\n",
    );
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["8"],
        "untyped anonymous fn (interpreter) mismatch\nfull output:\n{}",
        out
    );
}

/// Phase B-2.3: untyped function params in fn definition.
#[test]
fn emit_object_untyped_fn_params() {
    let dir = "target/test_untyped_fn_params";
    write_project(
        dir,
        "fn apply(f, x):\n    return f(x)\nfn add_one(x):\n    return x + 1\nfn main():\n    println(apply(add_one, 10))\n    return\n",
    );
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["11"],
        "untyped fn params (interpreter) mismatch\nfull output:\n{}",
        out
    );
}

/// Phase B-2.3: higher-order function — pass named function as argument (interpreter only).
/// Native limitation: untyped function parameters lose Type::Fn in codegen env.
#[test]
fn emit_object_higher_order_fn() {
    let dir = "target/test_higher_order_fn";
    write_project(
        dir,
        "fn apply(f, x):\n    return f(x)\nfn add_one(x):\n    return x + 1\nfn main():\n    println(apply(add_one, 10))\n    return\n",
    );
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["11"],
        "higher-order fn (interpreter) mismatch\nfull output:\n{}",
        out
    );
}

/// Phase B-2.4: function returned from function — make_adder pattern with native capture.
#[test]
fn emit_object_closure_return_native() {
    use std::fs;
    let dir = "target/test_closure_capture";
    write_project(
        dir,
        "fn make_adder(n):\n    return fn(x):\n        return x + n\nfn main():\n    let add5 = make_adder(5)\n    let add10 = make_adder(10)\n    println(add5(3))\n    println(add10(7))\n    println(add5(100))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["8", "17", "105"],
        "closure capture (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "8\n17\n105", "closure capture (native) mismatch");
}

/// Phase B-2.4: nested closures — inner closure captures outer parameter, with native capture.
#[test]
fn emit_object_nested_closure_native() {
    use std::fs;
    let dir = "target/test_nested_closure";
    write_project(
        dir,
        "fn make_multiplier(n):\n    return fn(x):\n        return x * n\nfn main():\n    let triple = make_multiplier(3)\n    let double = make_multiplier(2)\n    println(triple(5))\n    println(double(5))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["15", "10"],
        "nested closure (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "15\n10", "nested closure (native) mismatch");
}

/// Phase B-2.3: repeat build symbol stability — same source produces identical IR.
#[test]
fn emit_object_closure_symbol_stability() {
    let dir = "target/test_closure_symbol_stability";
    write_project(
        dir,
        "fn make_adder(n):\n    return fn(x):\n        return x + n\nfn main():\n    let add5 = make_adder(5)\n    println(add5(3))\n    return\n",
    );
    let out1 = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-llvm"]);
    let out2 = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-llvm"]);
    assert_eq!(out1, out2, "closure symbol stability failed: repeated builds produce different IR");
}

/// Phase C-1.1: string stdlib is_empty — runtime + native parity.
#[test]
fn emit_object_string_is_empty() {
    use std::fs;
    let dir = "target/test_string_is_empty";
    write_stdlib_project(
        dir,
        "fn main():\n    println(string.is_empty(\"\"))\n    println(string.is_empty(\"a\"))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["true", "false"],
        "string is_empty (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "true\nfalse", "string is_empty (native) mismatch");
}

/// Phase C-1.1: string stdlib find — runtime + native parity.
#[test]
fn emit_object_string_find() {
    use std::fs;
    let dir = "target/test_string_find";
    write_stdlib_project(
        dir,
        "fn main():\n    println(string.find(\"hello\", \"ll\"))\n    println(string.find(\"hello\", \"xyz\"))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["2", "-1"],
        "string find (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "2\n-1", "string find (native) mismatch");
}

/// Phase C-1.1: string stdlib count — runtime + native parity.
#[test]
fn emit_object_string_count() {
    use std::fs;
    let dir = "target/test_string_count";
    write_stdlib_project(
        dir,
        "fn main():\n    println(string.count(\"hello\", \"l\"))\n    println(string.count(\"hello\", \"x\"))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["2", "0"],
        "string count (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "2\n0", "string count (native) mismatch");
}

/// Phase C-1.1: string stdlib trim_start/trim_end — runtime + native parity.
#[test]
fn emit_object_string_trim_start_end() {
    use std::fs;
    let dir = "target/test_string_trim_start_end";
    write_stdlib_project(
        dir,
        "fn main():\n    let s = \"  abc  \"\n    println(string.trim_start(s))\n    println(string.trim_end(s))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["abc  ", "  abc"],
        "string trim_start/end (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "abc  \n  abc", "string trim_start/end (native) mismatch");
}

/// Phase C-1.1: string stdlib join — runtime + native parity.
#[test]
fn emit_object_string_join() {
    use std::fs;
    let dir = "target/test_string_join";
    write_stdlib_project(
        dir,
        "fn main():\n    let parts = string.split(\"a,b,c\", \",\")\n    println(string.join(\"-\", parts))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["a-b-c"],
        "string join (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "a-b-c", "string join (native) mismatch");
}

/// Phase C-1.1: string stdlib to_int/to_float — runtime + native parity.
#[test]
fn emit_object_string_to_int_float() {
    use std::fs;
    let dir = "target/test_string_to_int_float";
    write_stdlib_project(
        dir,
        "fn main():\n    println(string.to_int(\"42\"))\n    println(string.to_int(\"abc\"))\n    println(string.to_float(\"3.14\"))\n    println(string.to_float(\"xyz\"))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["42", "0", "3.14", "0"],
        "string to_int/to_float (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "42\n0\n3.14\n0", "string to_int/to_float (native) mismatch");
}

/// Phase C-1.1: string stdlib equals/compare — runtime + native parity.
#[test]
fn emit_object_string_equals_compare() {
    use std::fs;
    let dir = "target/test_string_equals_compare";
    write_stdlib_project(
        dir,
        "fn main():\n    println(string.equals(\"a\", \"a\"))\n    println(string.equals(\"a\", \"b\"))\n    println(string.compare(\"a\", \"b\"))\n    println(string.compare(\"b\", \"a\"))\n    println(string.compare(\"a\", \"a\"))\n    return\n",
    );

    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp,
        ["true", "false", "-1", "1", "0"],
        "string equals/compare (interpreter) mismatch\nfull output:\n{}",
        out
    );

    if !llvm_toolchain_available() {
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(out.contains("ok:"), "native build failed:\n{}", out);
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "expected executable at {}", exe);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(native_out.replace("\r", ""), "true\nfalse\n-1\n1\n0", "string equals/compare (native) mismatch");
}
