//! Iteration 33 P1 regression tests: `lime build` must NEVER report success
//! (exit 0 / "ok:") when the executable was not produced.
//!
//! Test A — pure-Lime program builds and runs as before.
//! Test B — forced link failure (the expected output path is pre-created as a
//!          DIRECTORY, so lld-link cannot open `/out:<path>.exe`) must yield
//!          a non-zero exit, no "ok:", and no success classification.
//!
//! Test B is meaningful in ANY environment: an environment-induced link
//! failure is exactly the class of failure that used to be silently
//! swallowed. Test A requires a working MSVC/Windows SDK environment; it
//! skips with guidance when that is absent.

use std::fs;
use std::process::Command;

fn llvm_toolchain_available() -> bool {
    std::env::var("LLVM_SYS_221_PREFIX").is_ok() || std::env::var("LIME_LLVM_PREFIX").is_ok()
}

/// Best-effort detection of an MSVC/Windows SDK link environment.
/// The official requirement is running from an x64 Developer Command Prompt;
/// `INCLUDE`/`LIB` being set is its observable signature.
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

struct BuildOutcome {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn lime_build(toml: &str) -> BuildOutcome {
    let out = Command::new(env!("CARGO_BIN_EXE_lime"))
        .arg("build")
        .arg(toml)
        .arg("--emit-object")
        .output()
        .unwrap();
    BuildOutcome {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

const HELLO: &str = "fn main():\n    println(\"P1_OK\")\n    return\n";

#[test]
fn build_success_still_works_pure_lime() {
    if !llvm_toolchain_available() {
        eprintln!("skipping: no LLVM toolchain env");
        return;
    }
    if !msvc_link_env_available() {
        eprintln!(
            "skipping: MSVC/Windows SDK environment not detected \
             (run from x64 Developer Command Prompt / vcvarsall.bat x64)"
        );
        return;
    }
    let dir = "target/test_p1_link_ok";
    write_project(dir, HELLO);
    let out = lime_build(&format!("{}/citrus.toml", dir));
    assert!(out.code == Some(0), "build should exit 0:\n{}", out.stderr);
    assert!(
        out.stdout.contains("ok:"),
        "expected ok line:\n{}",
        out.stdout
    );
    let exe = format!("{}.exe", dir);
    assert!(fs::metadata(&exe).is_ok(), "exe must exist: {}", exe);
    let run = Command::new(&exe).output().unwrap();
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "P1_OK");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn link_failure_is_a_hard_error() {
    let dir = "target/test_p1_link_fail";
    write_project(dir, HELLO);

    // Sabotage deterministically: the expected executable path is occupied by
    // a DIRECTORY, so lld-link cannot open its `/out:` target. This fails the
    // link step itself, regardless of toolchain environment quality.
    let exe_path = format!("{}.exe", dir);
    // A prior run may have left a real executable here — remove it so the
    // sabotage DIRECTORY can be created.
    let _ = fs::remove_file(&exe_path);
    fs::create_dir_all(&exe_path).unwrap();

    let out = lime_build(&format!("{}/citrus.toml", dir));

    // The old behaviour printed `ok:` and exited 0 with no exe — forbidden.
    assert_ne!(out.code, Some(0), "link failure must not exit 0");
    assert!(
        !out.stdout.contains("ok:"),
        "link failure must not print ok:\nstdout:\n{}",
        out.stdout
    );
    let combined = format!("{}{}", out.stdout, out.stderr);
    assert!(
        combined.contains("error[E0501]"),
        "expected error[E0501] diagnostic, got:\n{}\n--- stderr ---\n{}",
        combined,
        out.stderr
    );
    assert!(
        fs::metadata(&exe_path).unwrap().is_dir(),
        "sabotage directory must remain (no exe was written over it)"
    );

    // Recovery: removing the obstruction restores a normal build.
    if llvm_toolchain_available() && msvc_link_env_available() {
        fs::remove_dir_all(&exe_path).unwrap();
        let out = lime_build(&format!("{}/citrus.toml", dir));
        assert!(out.code == Some(0) && out.stdout.contains("ok:"));
    }
    let _ = fs::remove_dir_all(dir);
}
