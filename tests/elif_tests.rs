//! Iteration 33 — `elif` regression tests.
//!
//! `elif` is desugared at parse time into nested if/else statements, so these
//! tests pin the full pipeline (check / interpreter / native codegen) against
//! the mission's five canonical cases plus interpreter==native parity.

use std::fs;
use std::process::Command;

fn llvm_toolchain_available() -> bool {
    std::env::var("LLVM_SYS_221_PREFIX").is_ok() || std::env::var("LIME_LLVM_PREFIX").is_ok()
}

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

fn lime_cmd(subcmd: &str, toml: &str, extra: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lime"));
    cmd.arg(subcmd).arg(toml);
    for a in extra {
        cmd.arg(a);
    }
    let out = cmd.output().unwrap();
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

/// check must pass; interpreter output must equal `expected`; when a full
/// toolchain is present, native build+run output must match as well.
fn assert_elif_case(dir: &str, code: &str, expected: &[&str], name: &str) {
    write_project(dir, code);
    let interp = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let lines: Vec<&str> = interp
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .collect();
    assert_eq!(lines, expected, "{} interpreter mismatch\n{}", name, interp);

    if !llvm_toolchain_available() || !msvc_link_env_available() {
        eprintln!(
            "skipping native for {} — requires LLVM + MSVC/Windows SDK env \
             (x64 Developer Command Prompt / vcvarsall.bat x64)",
            name
        );
        return;
    }
    let build = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        build.contains("ok:"),
        "{} native build failed:\n{}",
        name,
        build
    );
    let exe = format!("{}.exe", dir);
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout)
        .replace("\r", "")
        .trim()
        .to_string();
    assert_eq!(native_out, expected.join("\n"), "{} native mismatch", name);
}

#[test]
fn elif_true_first_branch() {
    assert_elif_case(
        "target/test_elif_1",
        "fn main():\n    if true:\n        println(\"A\")\n    elif true:\n        println(\"B\")\n    else:\n        println(\"C\")\n",
        &["A"],
        "elif_true_first_branch",
    );
}

#[test]
fn elif_middle_branch_taken() {
    assert_elif_case(
        "target/test_elif_2",
        "fn main():\n    let x = 2\n\n    if x == 1:\n        println(\"A\")\n    elif x == 2:\n        println(\"B\")\n    else:\n        println(\"C\")\n",
        &["B"],
        "elif_middle_branch_taken",
    );
}

#[test]
fn elif_multiple_chain() {
    assert_elif_case(
        "target/test_elif_3",
        "fn main():\n    let x = 3\n\n    if x == 1:\n        println(\"A\")\n    elif x == 2:\n        println(\"B\")\n    elif x == 3:\n        println(\"C\")\n    else:\n        println(\"D\")\n",
        &["C"],
        "elif_multiple_chain",
    );
}

#[test]
fn elif_falls_to_final_else() {
    assert_elif_case(
        "target/test_elif_4",
        "fn main():\n    let x = 99\n\n    if x == 1:\n        println(\"A\")\n    elif x == 2:\n        println(\"B\")\n    else:\n        println(\"C\")\n",
        &["C"],
        "elif_falls_to_final_else",
    );
}

#[test]
fn elif_nested_inside_if() {
    assert_elif_case(
        "target/test_elif_5",
        "fn main():\n    let x = 10\n\n    if x > 0:\n        if x > 20:\n            println(\"A\")\n        elif x > 5:\n            println(\"B\")\n        else:\n            println(\"C\")\n    else:\n        println(\"D\")\n",
        &["B"],
        "elif_nested_inside_if",
    );
}

#[test]
fn elif_with_function_call_and_arithmetic_condition() {
    assert_elif_case(
        "target/test_elif_6",
        "fn score(int: n):\n    return n * 10\nfn main():\n    let n = 5\n    if score(n) > 80:\n        println(\"HIGH\")\n    elif score(n) > 40:\n        println(\"MEDIUM\")\n    else:\n        println(\"LOW\")\n",
        &["MEDIUM"],
        "elif_with_function_call_and_arithmetic_condition",
    );
}
