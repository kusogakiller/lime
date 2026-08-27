//! Diagnostic format regression tests.
//!
//! These verify that Lime's error diagnostics include:
//! - Stable error codes (E0xxx)
//! - Source snippets with caret pointers for type errors
//! - Consistent format across all phases (lexer, parser, type checker, etc.)

use std::process::Command;

fn lime_check(source: &str) -> (String, i32) {
    let dir = unique_test_dir("diag");
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(format!("{}/main.lime", dir), source).unwrap();
    let toml = format!(
        "[package]\nname = \"diag_test\"\nversion = \"0.0.1\"\n\n[files]\nmain = \"main.lime\""
    );
    std::fs::write(format!("{}/citrus.toml", dir), &toml).unwrap();
    let bin = env!("CARGO_BIN_EXE_lime");
    let output = Command::new(bin)
        .arg("check")
        .arg(format!("{}/citrus.toml", dir))
        .output()
        .expect("failed to run lime");
    let mut s = String::new();
    s.push_str(&String::from_utf8_lossy(&output.stdout));
    s.push_str(&String::from_utf8_lossy(&output.stderr));
    let code = output.status.code().unwrap_or(-1);
    (s, code)
}

fn unique_test_dir(name: &str) -> &'static str {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    Box::leak(
        format!("target/test_diag_{}_{}_{}", std::process::id(), count, name).into_boxed_str(),
    )
}

// ---- Lexer errors (E0001) ----

#[test]
fn lexer_error_has_e0001_code() {
    let (out, code) =
        lime_check("fn main():\n    let x = 9999999999999999999999999999999999999999999999\n");
    assert_eq!(code, 1, "lexer error must exit with code 1");
    assert!(
        out.contains("error[E0001]"),
        "expected error[E0001], got:\n{}",
        out
    );
}

#[test]
fn lexer_error_has_file_location() {
    let (out, _) =
        lime_check("fn main():\n    let x = 9999999999999999999999999999999999999999999999\n");
    assert!(
        out.contains("main.lime:"),
        "expected file location in lexer error, got:\n{}",
        out
    );
}

// ---- Parser errors (E0101) ----

#[test]
fn parser_error_has_e0101_code() {
    let (out, code) = lime_check("fn main():\n    let = 5\n");
    assert_eq!(code, 1, "parser error must exit with code 1");
    assert!(
        out.contains("error[E0101]"),
        "expected error[E0101], got:\n{}",
        out
    );
}

#[test]
fn parser_error_has_line_col() {
    let (out, _) = lime_check("fn main():\n    let = 5\n");
    assert!(
        out.contains("(at line") && out.contains("col"),
        "expected line/col in parser error, got:\n{}",
        out
    );
}

#[test]
fn parser_error_unexpected_eof() {
    let (out, _) = lime_check("fn main():\n    if true:\n");
    assert!(
        out.contains("error[E0101]"),
        "expected error[E0101] for unexpected EOF, got:\n{}",
        out
    );
    assert!(
        out.contains("Expected indented block"),
        "expected 'Expected indented block' message, got:\n{}",
        out
    );
}

// ---- Type checker errors (E02xx) ----

#[test]
fn type_error_undefined_variable_has_e0201() {
    let (out, code) = lime_check("fn main():\n    println(xyz)\n");
    assert_eq!(code, 1, "type error must exit with code 1");
    assert!(
        out.contains("error[E0201]"),
        "expected error[E0201], got:\n{}",
        out
    );
    assert!(
        out.contains("undefined variable 'xyz'"),
        "expected 'undefined variable' message, got:\n{}",
        out
    );
}

#[test]
fn type_error_unknown_function_has_e0202() {
    let (out, _) = lime_check("fn main():\n    foo(1, 2)\n");
    assert!(
        out.contains("error[E0202]"),
        "expected error[E0202], got:\n{}",
        out
    );
    assert!(
        out.contains("unknown function 'foo'"),
        "expected 'unknown function' message, got:\n{}",
        out
    );
}

#[test]
fn type_error_wrong_arg_count_has_e0207() {
    let (out, _) =
        lime_check("fn add(int: a, int: b):\n    return a + b\nfn main():\n    println(add(1))\n");
    assert!(
        out.contains("error[E0207]"),
        "expected error[E0207], got:\n{}",
        out
    );
    assert!(
        out.contains("expects 2 argument(s), got 1"),
        "expected argument count message, got:\n{}",
        out
    );
}

#[test]
fn type_error_wrong_arg_type_has_e0208() {
    let (out, _) = lime_check(
        "fn add(int: a, int: b):\n    return a + b\nfn main():\n    println(add(1, \"hello\"))\n",
    );
    assert!(
        out.contains("error[E0208]"),
        "expected error[E0208], got:\n{}",
        out
    );
    assert!(
        out.contains("expected:")
            && out.contains("int")
            && out.contains("received:")
            && out.contains("str"),
        "expected expected/received message, got:\n{}",
        out
    );
}

#[test]
fn type_error_non_exhaustive_match_has_e0209() {
    let (out, _) = lime_check(
        "enum Color:\n    Red\n    Green\nfn main():\n    let c = Red\n    match c:\n        Red:\n            println(\"red\")\n",
    );
    assert!(
        out.contains("error[E0209]"),
        "expected error[E0209], got:\n{}",
        out
    );
    assert!(
        out.contains("not exhaustive"),
        "expected 'not exhaustive' message, got:\n{}",
        out
    );
}

#[test]
fn type_error_tuple_index_out_of_bounds_has_e0205() {
    let (out, _) = lime_check("fn main():\n    let t = (1, 2)\n    println(t.5)\n");
    assert!(
        out.contains("error[E0205]"),
        "expected error[E0205], got:\n{}",
        out
    );
    assert!(
        out.contains("tuple index 5 out of bounds"),
        "expected 'tuple index out of bounds' message, got:\n{}",
        out
    );
}

#[test]
fn type_error_unknown_method_has_e0204() {
    let (out, _) = lime_check("fn main():\n    let x = 42\n    x.add(1)\n");
    assert!(
        out.contains("error[E0204]"),
        "expected error[E0204], got:\n{}",
        out
    );
    assert!(
        out.contains("unknown method 'add' on int"),
        "expected 'unknown method' message, got:\n{}",
        out
    );
}

// ---- Source snippets for type errors ----

#[test]
fn type_error_includes_source_snippet() {
    let (out, _) = lime_check("fn main():\n    println(xyz)\n");
    assert!(
        out.contains("|") && out.contains("println(xyz)"),
        "expected source snippet with pipe separator, got:\n{}",
        out
    );
}

#[test]
fn type_error_includes_caret_pointer() {
    let (out, _) = lime_check("fn main():\n    println(xyz)\n");
    assert!(
        out.contains("^"),
        "expected caret pointer in source snippet, got:\n{}",
        out
    );
}

// ---- Multiple errors collected ----

#[test]
fn multiple_type_errors_all_have_codes() {
    let (out, _) = lime_check(
        "fn main():\n    let x = 1\n    let y = x + \"s\"\n    let z = missing_var\n    println(z)\n",
    );
    let e0201_count = out.matches("error[E0201]").count();
    assert!(
        e0201_count >= 1,
        "expected at least one E0201 error, got {}:\n{}",
        e0201_count,
        out
    );
}

// ---- "did you mean" hints ----

#[test]
fn did_you_mean_hint_present() {
    let (out, _) = lime_check("fn main():\n    let counter = 1\n    let n = countre\n");
    assert!(
        out.contains("= help: did you mean 'counter'?"),
        "expected 'did you mean' hint, got:\n{}",
        out
    );
}

// ---- Exit codes ----

#[test]
fn error_exits_with_code_1() {
    let (_, code) = lime_check("fn main():\n    println(missing)\n");
    assert_eq!(code, 1, "error must exit with code 1");
}

#[test]
fn clean_program_exits_with_code_0() {
    let (_, code) = lime_check("fn main():\n    println(1)\n");
    assert_eq!(code, 0, "clean program must exit with code 0");
}
