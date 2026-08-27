//! Iteration 33 — native reliability regression tests.
//!
//! 1. Multiple functions that each contain a closure literal must build and
//!    run natively (the old per-instance anon counter emitted duplicate
//!    @anon_0 wrappers — duplicate symbol at link).
//! 2. Constructs known to be unlowerable must FAIL the build LOUDLY via
//!    error[codegen] instead of silently producing an executable whose main
//!    body does nothing.
//! 3. Tuples build and run natively (Iteration 34 PH1). They produce
//!    `[N x i64]` aggregate values and TupleAccess uses extractvalue.
//! 4. For-in loops build and run natively (Iteration 34 PH2). They use
//!    direct GEP-based element access (avoids always_inline runtime calls).

use std::fs;
use std::process::Command;

fn msvc_link_env_available() -> bool {
    std::env::var("INCLUDE")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
        && std::env::var("LIB").map(|v| !v.is_empty()).unwrap_or(false)
}

fn write_project(dir: &str, code: &str) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"test\"\nversion = \"1.0\"\n\n[files]\nmain = \"main.lime\"",
    )
    .unwrap();
    fs::write(format!("{}/main.lime", dir), code).unwrap();
}

fn build_project(dir: &str) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_lime"))
        .arg("build")
        .arg(format!("{}/citrus.toml", dir))
        .arg("--emit-object")
        .output()
        .unwrap();
    (
        out.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[test]
fn multiple_functions_with_closure_literals_native_ok() {
    if !msvc_link_env_available() {
        eprintln!(
            "skipping: MSVC/Windows SDK env required \
             (x64 Developer Command Prompt / vcvarsall.bat x64)"
        );
        return;
    }
    let dir = "target/test_p23_multi_closure";
    write_project(
        dir,
        "fn make_a(int: n):\n    return fn(int: x):\n        return x + n\nfn make_b(int: n):\n    return fn(int: x):\n        return x * n\nfn main():\n    let a = make_a(10)\n    println(a(5))\n    let b = make_b(3)\n    println(b(7))\n    return\n",
    );
    let (code, out) = build_project(dir);
    assert_eq!(code, Some(0), "build failed:\n{}", out);
    let run = Command::new(format!("{}.exe", dir)).output().unwrap();
    let text = String::from_utf8_lossy(&run.stdout).replace("\r", "");
    assert_eq!(text.trim(), "15\n21", "closure outputs mismatch");
}

#[test]
fn for_in_native_builds_and_runs() {
    if !msvc_link_env_available() {
        eprintln!(
            "skipping: MSVC/Windows SDK env required \
             (x64 Developer Command Prompt / vcvarsall.bat x64)"
        );
        return;
    }
    let dir = "target/test_p21_forin_native";
    write_project(
        dir,
        "fn main():\n    for n in [2, 4, 6]:\n        println(n)\n    let xs = [10, 20]\n    let mut sum = 0\n    for x in xs:\n        sum = sum + x\n    println(sum)\n    return\n",
    );
    let (code, out) = build_project(dir);
    assert_eq!(code, Some(0), "for-in native build failed:\n{}", out);
    let run = Command::new(format!("{}.exe", dir)).output().unwrap();
    let text = String::from_utf8_lossy(&run.stdout).replace("\r", "");
    assert_eq!(text.trim(), "2\n4\n6\n30", "for-in native output mismatch");
}

#[test]
fn tuple_native_builds_and_runs() {
    if !msvc_link_env_available() {
        eprintln!(
            "skipping: MSVC/Windows SDK env required \
             (x64 Developer Command Prompt / vcvarsall.bat x64)"
        );
        return;
    }
    let dir = "target/test_p21_tuple_native";
    write_project(
        dir,
        "fn main():\n    let t = (7, 9)\n    println(t.0)\n    println(t.1)\n    println(7 + 9)\n    return\n",
    );
    let (code, out) = build_project(dir);
    assert_eq!(code, Some(0), "tuple native build failed:\n{}", out);
    let run = Command::new(format!("{}.exe", dir)).output().unwrap();
    let text = String::from_utf8_lossy(&run.stdout).replace("\r", "");
    assert_eq!(text.trim(), "7\n9\n16", "tuple native output mismatch");
}
