
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



#[test]
fn unified_pipeline_legacy_shorthand_runs() {
    let out = lime_run("examples/pipeline_demo");
    assert!(
        out.contains("42"),
        "expected `lime <citrus.toml>` shorthand to run main() -> 42\n--- full output ---\n{}",
        out
    );
}




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
    
    assert!(ir.contains("add i64"), "expected integer addition\n--- ir ---\n{}", ir);
    
    assert!(ir.contains("insertvalue %Point"), "expected struct ctor\n--- ir ---\n{}", ir);
    
    assert!(ir.contains("extractvalue %Point"), "expected struct field extract\n--- ir ---\n{}", ir);
    
    assert!(ir.contains("br i1"), "expected conditional branch\n--- ir ---\n{}", ir);
    
    assert!(ir.contains("icmp"), "expected comparison in loop\n--- ir ---\n{}", ir);
    
    assert!(ir.contains("call i64 @add"), "expected call to add function\n--- ir ---\n{}", ir);
    
    assert!(ir.contains("@main()"), "expected C main wrapper\n--- ir ---\n{}", ir);
}




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
    
    let cached = ".citrus/cache/math/v0.1.0/citrus.toml";
    assert!(
        std::path::Path::new(cached).exists(),
        "expected imported package to be cached at {}",
        cached
    );
}





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
    
    assert!(
        !ir.contains("alloca void"),
        "IR must not contain 'alloca void'\n--- ir ---\n{}",
        ir
    );
}

#[test]
fn phase9_demo() {
    let out = lime_run("examples/phase9_demo");
    
    
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
        out.contains("not exhaustive"),
        "Expected 'not exhaustive' in output, got:\n{}",
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
