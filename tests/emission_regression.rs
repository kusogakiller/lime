//! Phase 11 backend stabilization: LLVM emission regression tests.
//!
//! These exercise the full `lime build` pipeline (IR → object → executable)
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
//!   refuse to produce an executable rather than emit a silent stub.

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
/// return produced `ret void` inside `define i64 ...` — which clang rejects
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
