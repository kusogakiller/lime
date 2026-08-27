use std::fs;
use std::process::Command;

fn llvm_toolchain_available() -> bool {
    std::env::var("LLVM_SYS_221_PREFIX").is_ok() || std::env::var("LIME_LLVM_PREFIX").is_ok()
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

fn lime_cmd(subcmd: &str, toml: &str, extra_args: &[&str]) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_lime"));
    cmd.arg(subcmd).arg(toml);
    for a in extra_args {
        cmd.arg(a);
    }
    let out = cmd.output().unwrap();
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

fn run_both(dir: &str, code: &str, expected: &[&str], test_name: &str) {
    write_project(dir, code);
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp, expected,
        "{} (interpreter) mismatch\nfull:\n{}",
        test_name, out
    );

    if !llvm_toolchain_available() {
        eprintln!("skipping native for {} -- no LLVM", test_name);
        return;
    }
    let out = lime_cmd("build", &format!("{}/citrus.toml", dir), &["--emit-object"]);
    assert!(
        out.contains("ok:"),
        "{} native build failed:\n{}",
        test_name,
        out
    );
    let exe = format!("{}.exe", dir);
    assert!(
        fs::metadata(&exe).is_ok(),
        "{} expected exe at {}",
        test_name,
        exe
    );
    let run = Command::new(&exe).output().unwrap();
    let native_out = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(
        native_out.replace("\r", ""),
        expected.join("\n"),
        "{} native mismatch",
        test_name
    );
}

fn run_interp_only(dir: &str, code: &str, expected: &[&str], test_name: &str) {
    write_project(dir, code);
    let out = lime_cmd("run", &format!("{}/citrus.toml", dir), &[]);
    let interp: Vec<&str> = out
        .lines()
        .filter(|l| !l.starts_with("warning"))
        .filter(|l| !l.contains("unused variable"))
        .filter(|l| !l.contains("error["))
        .collect();
    assert_eq!(
        interp, expected,
        "{} (interpreter) mismatch\nfull:\n{}",
        test_name, out
    );
}

#[test]
fn capture_int_value() {
    run_both(
        "target/test_capture_int",
        "fn make_adder(int: n):\n    return fn(int: x):\n        return x + n\nfn main():\n    let add5 = make_adder(5)\n    println(add5(3))\n    println(add5(10))\n    return\n",
        &["8", "15"],
        "capture_int_value",
    );
}

#[test]
fn capture_bool_value() {
    run_both(
        "target/test_capture_bool",
        "fn make_check(bool: flag):\n    return fn(int: x):\n        if flag:\n            return x + 100\n        return x\nfn main():\n    let yes = make_check(true)\n    let no = make_check(false)\n    println(yes(5))\n    println(no(5))\n    return\n",
        &["105", "5"],
        "capture_bool_value",
    );
}

#[test]
fn capture_multiple_values() {
    run_both(
        "target/test_capture_multi",
        "fn make_op(int: a, int: b):\n    return fn(int: x):\n        return a * x + b\nfn main():\n    let f = make_op(3, 7)\n    println(f(1))\n    println(f(2))\n    println(f(10))\n    return\n",
        &["10", "13", "37"],
        "capture_multiple_values",
    );
}

#[test]
fn capture_from_loop() {
    run_both(
        "target/test_capture_loop",
        "fn make_adder(int: n):\n    return fn(int: x):\n        return x + n\nfn main():\n    let i = 0\n    while i < 3:\n        let f = make_adder(i)\n        println(f(10))\n        i = i + 1\n    return\n",
        &["10", "11", "12"],
        "capture_from_loop",
    );
}

#[test]
fn closure_identity_no_capture() {
    run_both(
        "target/test_closure_identity",
        "fn make_id():\n    return fn(int: x):\n        return x\nfn main():\n    let id = make_id()\n    println(id(42))\n    println(id(99))\n    return\n",
        &["42", "99"],
        "closure_identity_no_capture",
    );
}

#[test]
fn higher_order_native_interp() {
    run_interp_only(
        "target/test_ho_native",
        "fn apply(f, x):\n    return f(x)\nfn add_one(int: x):\n    return x + 1\nfn main():\n    println(apply(add_one, 10))\n    return\n",
        &["11"],
        "higher_order_native_interp",
    );
}

#[test]
fn nested_closure_capture() {
    run_interp_only(
        "target/test_nested_capture",
        "fn make_outer(int: a):\n    return fn(int: b):\n        return fn(int: c):\n            return a + b + c\nfn main():\n    let f = make_outer(1)\n    let g = f(2)\n    println(g(3))\n    return\n",
        &["6"],
        "nested_closure_capture",
    );
}

#[test]
fn closure_string_capture() {
    run_interp_only(
        "target/test_closure_str",
        "fn make_greet(str: name):\n    return fn():\n        println(name)\nfn main():\n    let greet = make_greet(\"hello\")\n    greet()\n    return\n",
        &["hello"],
        "closure_string_capture",
    );
}
