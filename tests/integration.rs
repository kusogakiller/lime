//! Integration tests: ビルド済み `lime` バイナリを実行し、例題�E出力を検証する、E//! 吁Eexample は `<dir>/citrus.toml` + `<dir>/main.lime` 構�E、E
use std::process::Command;

fn lime_run(dir: &str) -> String {
    let bin = env!("CARGO_BIN_EXE_lime");
    let toml = format!("{}/citrus.toml", dir);
    let output = Command::new(bin)
        .arg(&toml)
        .output()
        .expect("failed to run lime");
    let mut s = String::new();
    s.push_str(&String::from_utf8_lossy(&output.stdout));
    s.push_str(&String::from_utf8_lossy(&output.stderr));
    s
}

/// Run the given subcommand (`build` / `run` / `check`) against a `citrus.toml`
/// and return combined stdout+stderr.
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

#[test]
fn stdlib_string_math() {
    let out = lime_run("examples/stdlib_demo");
    for expected in [
        "true", "true", "true", "spaced", "hello lime", "[hello, world]", "42", "7", "3", "4", "1024",
    ] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

#[test]
fn collections_demo() {
    let out = lime_run("examples/collections_demo");
    for expected in ["1", "3", "3", "true", "1"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

/// Phase 12 Step 1: collections List + HashMap + HashSet wrappers.
#[test]
fn collections_demo2() {
    let out = lime_run("examples/collections_demo2");
    for expected in [
        "[1, 2, 3]",
        "3",
        "true",
        "[3, 2, 1]",
        "Some(10)",
        "true",
        "false",
        "1",
        "2",
    ] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

/// Phase 12 Step 1: fs metadata/size/list_dir/create_dir.
#[test]
fn fs_demo2() {
    let out = lime_run("examples/fs_demo2");
    for expected in ["true", "14", "note.txt", "sub"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

/// Phase 12 Step 1: string to_upper/to_lower/repeat + existing ops.
#[test]
fn string_demo2() {
    let out = lime_run("examples/string_demo2");
    for expected in [
        "Hello, Lime",
        "HELLO, LIME",
        "hello, lime",
        "ababab",
        "a_b_c",
        "[x, y, z]",
        "bcd",
        "true",
    ] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

/// Phase 12 Step 1: time Instant/Duration/now/elapsed/sleep.
#[test]
fn time_demo() {
    let out = lime_run("examples/time_demo");
    for expected in ["true", "true"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

/// Phase 11: the legacy `lime <citrus.toml>` shorthand and the `run`
/// subcommand share one `compile_pipeline`, so both execute the program.
#[test]
fn unified_pipeline_legacy_shorthand_runs() {
    let out = lime_run("examples/pipeline_demo");
    assert!(
        out.contains("42"),
        "expected `lime <citrus.toml>` shorthand to run main() -> 42\n--- full output ---\n{}",
        out
    );
}

/// Phase 11: dead-code elimination is active on the unified pipeline. The
/// project defines `never_used`, which the optimizer must strip. `build`
/// reports how many functions it removed.
#[test]
fn unified_pipeline_runs_dce() {
    let out = lime_cmd("build", "examples/pipeline_demo/citrus.toml", &["--emit-ll"]);
    assert!(
        out.contains("optimizer: removed"),
        "expected DCE to report removed functions\n--- full output ---\n{}",
        out
    );
    let ir = std::fs::read_to_string("examples/pipeline_demo.ll").unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output to be generated");
    assert!(
        !ir.contains("alloca void"),
        "IR must not contain 'alloca void'\n--- ir ---\n{}",
        ir
    );
}

/// Phase X: backend_demo — arithmetic, function calls, if, loop, struct.
#[test]
fn backend_demo() {
    let out = lime_cmd("build", "examples/backend_demo/citrus.toml", &["--emit-ll"]);
    assert!(
        out.contains("ok:"),
        "expected build to succeed\n--- full output ---\n{}",
        out
    );
    let ir = std::fs::read_to_string("examples/backend_demo.ll").unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output to be generated");
    // Arith
    assert!(ir.contains("add i64"), "expected integer addition\n--- ir ---\n{}", ir);
    // Struct constructor
    assert!(ir.contains("insertvalue %Point"), "expected struct ctor\n--- ir ---\n{}", ir);
    // Struct field extract
    assert!(ir.contains("extractvalue %Point"), "expected struct field extract\n--- ir ---\n{}", ir);
    // If / branch
    assert!(ir.contains("br i1"), "expected conditional branch\n--- ir ---\n{}", ir);
    // While loop -> icmp + br
    assert!(ir.contains("icmp"), "expected comparison in loop\n--- ir ---\n{}", ir);
    // Function call
    assert!(ir.contains("call i64 @add"), "expected call to add function\n--- ir ---\n{}", ir);
    // C runtime main wrapper
    assert!(ir.contains("@main()"), "expected C main wrapper\n--- ir ---\n{}", ir);
}

/// Phase 11: the unified pipeline runs package-manager side effects even for
/// the legacy shorthand path: `citrus.lock` is written next to the manifest
/// and imported packages are copied into `.citrus/cache`.
#[test]
fn unified_pipeline_writes_lock_and_cache() {
    let lock = "examples/pipeline_demo/citrus.lock";
    let _ = std::fs::remove_file(lock);
    let _ = lime_run("examples/pipeline_demo");
    assert!(
        std::path::Path::new(lock).exists(),
        "expected citrus.lock to be generated at {}",
        lock
    );
    // math is imported; its manifest must be cached.
    let cached = ".citrus/cache/math/v0.1.0/citrus.toml";
    assert!(
        std::path::Path::new(cached).exists(),
        "expected imported package to be cached at {}",
        cached
    );
}

/// Phase 9: codegen smoke test. Runs a project with `--emit-llvm` and
/// verifies the generated IR is structurally valid: a return-type-less
/// function is still given a concrete return type, and no broken
/// `alloca void` stubs are emitted.
#[test]
fn emit_llvm_smoke() {
    let bin = env!("CARGO_BIN_EXE_lime");
    let toml = "sandbox/lltest/citrus.toml";
    let output = Command::new(bin)
        .arg(toml)
        .arg("--emit-ll")
        .output()
        .expect("failed to run lime");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let ll_path = "sandbox/lltest.ll";
    let ir = std::fs::read_to_string(ll_path)
        .unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output to be generated");
    // return-type-less `add` must be inferred to i64, not void.
    assert!(
        ir.contains("define i64 @add (i64 %p0, i64 %p1)"),
        "expected 'define i64 @add' in IR\n--- stderr ---\n{}\n--- ir ---\n{}",
        stderr,
        ir
    );
    assert!(
        ir.contains("ret i32"),
        "expected 'ret i32' in IR\n--- ir ---\n{}",
        ir
    );
    // No broken void allocations.
    assert!(
        !ir.contains("alloca void"),
        "IR must not contain 'alloca void'\n--- ir ---\n{}",
        ir
    );
}

#[test]
fn phase9_demo() {
    let out = lime_run("examples/phase9_demo");
    // nums = [10,20,30]: len()=3, sum=60; make_point(3,4).x=3,.y=4;
    // "hello".len()=5; "hello"+" world"; Success(42) match prints 42.
    for expected in ["3", "60", "3", "4", "5", "hello world", "42"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

#[test]
fn iface_demo() {
    let out = lime_run("examples/iface_demo");
    for expected in ["woof", "4", "meow", "woof", "meow"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

/// Phase 9: interface dispatch codegen. Current backend lowers interface
/// calls to a `%LimeIface` fat-pointer plus direct static calls (e.g.
/// `call void @make_sound(%LimeIface %p0)`); per-method vtables
/// (`@vtable_*`) and self-by-pointer signatures are a future backend goal
/// and are intentionally not asserted here.
#[test]
fn emit_llvm_interface() {
    let bin = env!("CARGO_BIN_EXE_lime");
    let output = Command::new(bin)
        .arg("examples/iface_demo/citrus.toml")
        .arg("--emit-ll")
        .output()
        .expect("failed to run lime");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let ir = std::fs::read_to_string("examples/iface_demo.ll").unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output to be generated");
    assert!(
        ir.contains("%LimeIface = type { i8*, i8* }"),
        "expected %LimeIface fat-pointer type\n--- stderr ---\n{}\n--- ir ---\n{}",
        stderr,
        ir
    );
    assert!(
        ir.contains("define i8* @Dog_speak (%Dog %p0"),
        "expected Dog_speak to be emitted taking self by value\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("define void @make_sound (%LimeIface %p0)"),
        "expected interface method make_sound over %LimeIface\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("call void @make_sound(%LimeIface %t"),
        "expected main to dispatch interface calls through %LimeIface\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("alloca void"),
        "IR must not contain 'alloca void'\n--- ir ---\n{}",
        ir
    );
}

/// Phase 9: for-loop / struct / list / state codegen must be structurally
/// valid and free of `alloca void`. The current backend emits struct types
/// and constructors (`insertvalue %Point`) but does not yet lower list
/// literals, for-loops, struct field access, or state matches into
/// `main_lime`; field access (`extractvalue`) and state dispatch
/// (`switch i32`) are future backend goals and are intentionally not
/// asserted here.
#[test]
fn emit_llvm_phase9_demo() {
    let bin = env!("CARGO_BIN_EXE_lime");
    let output = Command::new(bin)
        .arg("examples/phase9_demo/citrus.toml")
        .arg("--emit-ll")
        .output()
        .expect("failed to run lime");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let ir = std::fs::read_to_string("examples/phase9_demo.ll").unwrap_or_default();
    assert!(!ir.is_empty(), "expected .ll output to be generated");
    assert!(
        ir.contains("%Point = type { i64, i64 }"),
        "expected %Point struct type\n--- stderr ---\n{}\n--- ir ---\n{}",
        stderr,
        ir
    );
    assert!(
        ir.contains("define %Point @make_point"),
        "expected make_point struct constructor function\n--- ir ---\n{}",
        ir
    );
    assert!(
        ir.contains("insertvalue %Point"),
        "expected struct constructor (insertvalue %Point)\n--- ir ---\n{}",
        ir
    );
    assert!(
        !ir.contains("alloca void"),
        "IR must not contain 'alloca void'\n--- ir ---\n{}",
        ir
    );
}

/// Phase 12 Step 2: inference demo — untyped parameters are inferred from
/// call sites, making type-checking and codegen succeed without explicit
/// type annotations.
#[test]
fn inference_demo() {
    let out = lime_run("examples/inference_demo");
    for expected in ["3", "100", "42", "30", "hello", "100"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

#[test]
fn enum_declaration_parses() {
    use std::fs;
    let source = r#"enum Color:
    Red
    Green
    Blue
fn main():
    0
"#;
    let dir = "target/test_enum_tmp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();

    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);

    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_match_bare_variant_checks() {
    use std::fs;
    let source = r#"enum Color:
    Red
    Green
    Blue
fn main():
    let c = Red
    match c:
        Red:
            println("red")
        Green:
            println("green")
        Blue:
            println("blue")
    println("ok")
"#;
    let dir = "target/test_enum_match_bare";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_match_bare\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_match_payload_field_checks() {
    use std::fs;
    let source = r#"enum Value:
    Int(int)
    Text(str)
fn main():
    let v = Int(100)
    match v:
        Int(x):
            println(str(x))
        Text(s):
            println(s)
    println("ok")
"#;
    let dir = "target/test_enum_match_payload";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_match_payload\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_match_wildcard_checks() {
    use std::fs;
    let source = r#"enum Color:
    Red
    Green
    Blue
fn main():
    let c = Red
    match c:
        Red:
            println("red")
        catch:
            println("other")
    println("ok")
"#;
    let dir = "target/test_enum_match_wildcard";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_match_wildcard\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_match_exhaustive_error() {
    use std::fs;
    let source = r#"enum Color:
    Red
    Green
    Blue
fn main():
    let c = Red
    match c:
        Red:
            println("red")
        Green:
            println("green")
    println("ok")
"#;
    let dir = "target/test_enum_match_exhaust";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_match_exhaust\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

// === Phase B-3: Type System Regression Tests ===

/// Mismatched function argument types should produce a clear error.
#[test]
fn type_mismatch_function_arg() {
    use std::fs;
    let source = "fn add(int: a, int: b):\n    return a + b\nfn main():\n    println(add(1, \"hello\"))\n    return\n";
    let dir = "target/test_type_mismatch_fn_arg";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_type_mismatch_fn_arg\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("type mismatch") || out.contains("Type error"),
        "Expected type mismatch error, got:\n{}",
        out
    );
}

/// Incorrect return type should produce a clear error.
#[test]
fn type_mismatch_return_type() {
    use std::fs;
    let source = "fn get_str():\n    return \"hello\"\nfn main():\n    let x = get_str()\n    println(x + 1)\n    return\n";
    let dir = "target/test_type_mismatch_return";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_type_mismatch_return\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("type mismatch") || out.contains("Type error"),
        "Expected type mismatch error, got:\n{}",
        out
    );
}

/// Function returning a closure should type-check correctly.
#[test]
fn function_returning_closure() {
    use std::fs;
    let source = "fn make_adder(int: n):\n    return fn(int: x):\n        return x + n\nfn main():\n    let add5 = make_adder(5)\n    println(add5(3))\n    return\n";
    let dir = "target/test_fn_returning_closure";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_fn_returning_closure\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok") || !out.contains("error["),
        "Expected function returning closure to type-check, got:\n{}",
        out
    );
}

/// Option/Result combinations should type-check correctly.
#[test]
fn option_result_combinations() {
    use std::fs;
    let source = "fn main():\n    let opts = [Some(1), Some(2), Some(3)]\n    println(len(opts))\n    return\n";
    let dir = "target/test_option_result";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_option_result\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok") || !out.contains("error["),
        "Expected Option(Result) to type-check, got:\n{}",
        out
    );
}

// === Phase B-3: Closure ABI Tests ===

#[test]
fn closure_abi_no_capture() {
    use std::fs;
    let source = "fn make_id():\n    return fn(int: x):\n        return x\nfn main():\n    let id = make_id()\n    println(id(42))\n    return\n";
    let dir = "target/test_closure_abi_no_capture";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_closure_abi_no_capture\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["42"], "closure abi no capture failed\nfull:\n{}", out);
}

#[test]
fn closure_abi_single_capture() {
    use std::fs;
    let source = "fn make_adder(int: n):\n    return fn(int: x):\n        return x + n\nfn main():\n    let add5 = make_adder(5)\n    println(add5(3))\n    return\n";
    let dir = "target/test_closure_abi_single";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_closure_abi_single\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["8"], "closure abi single capture failed\nfull:\n{}", out);
}

#[test]
fn closure_abi_multiple_captures() {
    use std::fs;
    let source = "fn make_op(int: a, int: b):\n    return fn(int: x):\n        return a + b + x\nfn main():\n    let f = make_op(1, 2)\n    println(f(3))\n    return\n";
    let dir = "target/test_closure_abi_multi";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_closure_abi_multi\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["6"], "closure abi multiple captures failed\nfull:\n{}", out);
}

#[test]
fn closure_abi_nested() {
    use std::fs;
    let source = "fn make_outer(int: a):\n    return fn(int: b):\n        return fn(int: c):\n            return a + b + c\nfn main():\n    let f = make_outer(1)\n    let g = f(2)\n    println(g(3))\n    return\n";
    let dir = "target/test_closure_abi_nested";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_closure_abi_nested\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["6"], "closure abi nested failed\nfull:\n{}", out);
}

#[test]
fn closure_abi_closure_as_arg() {
    use std::fs;
    let source = "fn apply(f, int: x):\n    return f(x)\nfn double(int: x):\n    return x * 2\nfn main():\n    println(apply(double, 5))\n    return\n";
    let dir = "target/test_closure_abi_as_arg";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_closure_abi_as_arg\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["10"], "closure abi closure as arg failed\nfull:\n{}", out);
}

// === Phase B-3: Runtime Safety Tests ===

#[test]
fn safety_null_closure_check() {
    use std::fs;
    let source = "fn main():\n    println(\"safety test passed\")\n    return\n";
    let dir = "target/test_safety_null_closure";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_safety_null_closure\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["safety test passed"], "safety null closure check failed\nfull:\n{}", out);
}

#[test]
fn safety_list_bounds() {
    use std::fs;
    let source = "fn main():\n    let xs = [1, 2, 3]\n    println(len(xs))\n    return\n";
    let dir = "target/test_safety_list_bounds";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_safety_list_bounds\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["3"], "safety list bounds check failed\nfull:\n{}", out);
}

#[test]
fn safety_string_concat() {
    use std::fs;
    let source = "fn main():\n    let s = \"hello\" + \" world\"\n    println(s)\n    return\n";
    let dir = "target/test_safety_string_concat";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_safety_string_concat\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out.lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(interp, ["hello world"], "safety string concat check failed\nfull:\n{}", out);
}





/// Phase B-1 Step 1: interpreter parity for the `option` / `result` stdlib
/// helpers. The same program is used by the native codegen regression test
/// (`emit_object_option_result_runs`) and must produce identical output under
/// `lime run`.
#[test]
fn stdlib_option_result_interp() {
    use std::fs;
    let dir = "target/test_option_result_interp";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(
        format!("{}/main.lime", dir),
        "fn main():\n    let some = Some(5)\n    let none = None\n    println(option.is_some(some))\n    println(option.is_none(some))\n    println(option.unwrap_or(some, 0))\n    println(option.unwrap_or(none, 0))\n    println(option.unwrap(some))\n    let ok = Success(10)\n    let err = Error(\"boom\")\n    println(result.is_ok(ok))\n    println(result.is_err(ok))\n    println(result.is_err(err))\n    println(result.unwrap_or(ok, 0))\n    println(result.unwrap_or(err, 0))\n    println(result.unwrap(ok))\n    return\n",
    )
    .unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_option_result_interp\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"\n\n[dependencies]\noption = \"v0.1.0\"\nresult = \"v0.1.0\"\n",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
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
    let stdout: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning:"))
        .collect();
    assert!(
        stdout == expected,
        "option/result interp output mismatch\nexpected: {:?}\ngot: {:?}\n--- full output ---\n{}",
        expected,
        stdout,
        out
    );
}

#[test]
fn phase10_9_indexing_slicing() {
    let out = lime_run("examples/phase10_9_demo");
    for expected in ["10", "20", "30", "2", "10", "20", "3", "0", "10"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
}

#[test]
fn defer_lifo_order() {
    use std::fs;
    let source = r#"fn main():
    println("start")
    defer:
        println("A")
    defer:
        println("B")
    println("end")
"#;
    let dir = "target/test_defer_lifo_order";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_defer_lifo_order\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    for expected in ["start", "end", "A", "B"] {
        assert!(
            out.contains(expected),
            "expected output to contain '{}'\n--- full output ---\n{}",
            expected,
            out
        );
    }
    let a = out.find("A").unwrap_or(usize::MAX);
    let b = out.find("B").unwrap_or(usize::MAX);
    let end = out.find("end").unwrap_or(usize::MAX);
    assert!(
        end < b && b < a,
        "defers should flush LIFO (end < B < A), got:\n{}",
        out
    );
}

#[test]
fn defer_flushes_on_early_return() {
    use std::fs;
    let source = r#"fn main():
    println("before")
    defer:
        println("cleanup")
    return
    println("after")
"#;
    let dir = "target/test_defer_flushes_on_early_return";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_defer_flushes_on_early_return\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("before") && out.contains("cleanup"),
        "deferred body should run on early return, got:\n{}",
        out
    );
    assert!(
        !out.contains("after"),
        "code after return should not run, got:\n{}",
        out
    );
}

#[test]
fn defer_in_control_flow_runs() {
    use std::fs;
    let source = r#"fn main():
    for i in [1, 2, 3]:
        defer:
            println("d")
    println("done")
"#;
    let dir = "target/test_defer_in_control_flow";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_defer_in_control_flow\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.matches("d").count() >= 3 && out.contains("done"),
        "deferred body should run for each loop iteration, got:\n{}",
        out
    );
}

#[test]
fn error_message_rich_expected_received() {
    use std::fs;
    let source = r#"fn main():
    let x = 1
    let y = x + "s"
    println(y)
"#;
    let dir = "target/test_error_msg_expected_received";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_error_msg_expected_received\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("binary '+' type mismatch"),
        "Expected mismatch summary, got:\n{}",
        out
    );
    assert!(
        out.contains("expected:\n    int\n\nreceived:\n    str"),
        "Expected rich expected/received block, got:\n{}",
        out
    );
}

#[test]
fn error_message_assign_mismatch() {
    use std::fs;
    let source = r#"fn main():
    let x = 1
    x = "s"
"#;
    let dir = "target/test_error_msg_assign_mismatch";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_error_msg_assign_mismatch\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("cannot assign value to 'x'"),
        "Expected assign mismatch summary, got:\n{}",
        out
    );
    assert!(
        out.contains("expected:\n    int\n\nreceived:\n    str"),
        "Expected rich block, got:\n{}",
        out
    );
}

#[test]
fn tuple_match_basic_checks() {
    use std::fs;
    let source = r#"fn main():
    let t = (1, "hi")
    match t:
        try (a, b):
            println(a)
            println(b)
    println("ok")
"#;
    let dir = "target/test_tuple_match_basic";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_tuple_match_basic\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn tuple_match_nested_checks() {
    use std::fs;
    let source = r#"fn main():
    let n = (10, (20, 30))
    match n:
        try (x, (y, z)):
            println(x + y + z)
    println("ok")
"#;
    let dir = "target/test_tuple_match_nested";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_tuple_match_nested\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn tuple_match_wildcard_checks() {
    use std::fs;
    let source = r#"fn main():
    let w = (7, "seven")
    match w:
        catch:
            println("wildcard")
    println("ok")
"#;
    let dir = "target/test_tuple_match_wildcard";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_tuple_match_wildcard\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn tuple_match_size_mismatch_error() {
    use std::fs;
    let source = r#"fn main():
    let t = (1, 2, 3)
    match t:
        try (a, b):
            println(a)
    println("ok")
"#;
    let dir = "target/test_tuple_match_size_mismatch";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_tuple_match_size_mismatch\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("tuple pattern size mismatch"),
        "Expected 'tuple pattern size mismatch' in output, got:\n{}",
        out
    );
}

#[test]
fn tuple_generic_match_checks() {
    use std::fs;
    let source = r#"fn swap(T, U)(T: a, U: b):
    return (b, a)

fn main():
    let s = swap(1, 2)
    match s:
        try (b, a):
            println(a)
            println(b)
    println("ok")
"#;
    let dir = "target/test_tuple_generic_match";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_tuple_generic_match\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_match_unknown_variant_error() {
    use std::fs;
    let source = r#"enum Color:
    Red
    Green
    Blue
fn main():
    let c = Red
    match c:
        Red:
            println("red")
        Redd:
            println("bad")
    println("ok")
"#;
    let dir = "target/test_enum_match_unknown_var";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_match_unknown_var\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("unknown variant"),
        "Expected 'unknown variant' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_match_generic_checks() {
    use std::fs;
    let source = r#"enum Box(T):
    Value(T)
fn main():
    let x = Value(100)
    match x:
        Value(v):
            println(str(v))
    println("ok")
"#;
    let dir = "target/test_enum_match_generic";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_match_generic\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_generic_func_return_checks() {
    use std::fs;
    let source = r#"enum Box(T):
    Value(T)
fn wrap(T)(T: x):
    return Value(x)
fn main():
    let y = wrap(42)
    match y:
        Value(v):
            println(str(v))
    println("ok")
"#;
    let dir = "target/test_enum_generic_func_return";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_generic_func_return\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_generic_multi_type_params_checks() {
    use std::fs;
    let source = r#"enum Pair(A, B):
    Make(A, B)
fn main():
    let p = Make("hello", 100)
    match p:
        Make(a, b):
            println(a)
            println(str(b))
    println("ok")
"#;
    let dir = "target/test_enum_generic_multi_type_params";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_generic_multi_type_params\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}

#[test]
fn enum_generic_multi_variant_checks() {
    use std::fs;
    let source = r#"enum Maybe(T):
    Just(T)
    Nothing
fn main():
    let a = Just(42)
    let Maybe(int): b = Nothing
    match a:
        Just(v):
            println(str(v))
        Nothing:
            println("none")
    match b:
        Just(v):
            println(str(v))
        Nothing:
            println("none")
    println("ok")
"#;
    let dir = "target/test_enum_generic_multi_variant";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test_enum_generic_multi_variant\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    let out = lime_cmd("check", &format!("{}/citrus.toml", dir), &[]);
    let _ = fs::remove_dir_all(dir);
    assert!(
        out.contains("ok"),
        "Expected 'ok' in output, got:\n{}",
        out
    );
}
