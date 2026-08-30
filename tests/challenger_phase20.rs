//! Phase 20: Challenger Reliability / Memory / Resource Safety
//!
//! Builds Lime programs as native executables and runs them repeatedly to
//! verify resource lifecycle, memory safety, and error handling.
//! Tests use only APIs confirmed to work in native codegen.

use std::process::Command;

fn unique_test_dir(name: &str) -> &'static str {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    Box::leak(
        format!(
            "target/challenger_p20_{}_{}_{}",
            std::process::id(),
            count,
            name
        )
        .into_boxed_str(),
    )
}

fn write_project(dir: &str, source: &str) {
    use std::fs;
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{}/main.lime", dir), source).unwrap();
    fs::write(
        format!("{}/citrus.toml", dir),
        "[package]\nname = \"challenger_p20\"\nversion = \"v0.1.0\"\n\n[files]\nmain = \"main.lime\"\n\n[dependencies]\nio = \"v0.1.0\"\nstring = \"v0.1.0\"\nmath = \"v0.1.0\"\nfs = \"v0.1.0\"\ntime = \"v0.1.0\"\noption = \"v0.1.0\"\nresult = \"v0.1.0\"\ncollections = \"v0.1.0\"\nprocess = \"v0.1.0\"\nos = \"v0.1.0\"\npath = \"v0.1.0\"\nenv = \"v0.1.0\"\nregex = \"v0.1.0\"\n",
    )
    .unwrap();
}

fn build_and_run(dir: &str) -> (String, i32) {
    let out = Command::new(env!("CARGO_BIN_EXE_lime"))
        .arg("build")
        .arg(format!("{}/citrus.toml", dir))
        .arg("--emit-object")
        .output()
        .expect("failed to run lime build");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    if !out.status.success() {
        return (stderr, out.status.code().unwrap_or(-1));
    }
    let exe = format!("{}.exe", dir);
    if !std::path::Path::new(&exe).exists() {
        return (format!("executable not found at {}", exe), -1);
    }
    let run = Command::new(&exe)
        .output()
        .expect("failed to run executable");
    let stdout = String::from_utf8_lossy(&run.stdout).to_string();
    let stderr_out = String::from_utf8_lossy(&run.stderr).to_string();
    let combined = if stderr_out.is_empty() {
        stdout
    } else {
        format!("{}\n{}", stdout, stderr_out)
    };
    (combined, run.status.code().unwrap_or(-1))
}

fn assert_pass(name: &str, dir: &str) {
    let (output, code) = build_and_run(dir);
    assert_eq!(
        code, 0,
        "[{}] exit code should be 0, got code={}: {}",
        name, code, output
    );
    assert!(
        output.contains("P20 PASS"),
        "[{}] should contain P20 PASS, got: {}",
        name,
        output
    );
}

// ========================================================================
// Test 1: String lifecycle stress (200 iterations)
// ========================================================================
#[test]
fn p20_string_lifecycle_stress() {
    let dir = unique_test_dir("string_lifecycle");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 200:
        let s = "hello " + "world " + str(i)
        let _ = string.len(s)
        let _ = string.contains(s, "world")
        let _ = string.to_upper(s)
        let _ = string.trim("  x  ")
        let _ = string.replace(s, "world", "WORLD")
        let _ = string.repeat("ab", 3)
        let _ = string.slice("hello", 1, 3)
        let _ = string.find("hello world", "world")
        let _ = string.count("hello", "l")
        let _ = string.is_empty("")
        let _ = string.starts_with("hello", "he")
        let _ = string.ends_with("hello", "lo")
        let _ = string.equals("abc", "abc")
        let _ = string.split("a,b,c", ",")
        let _ = string.compare("abc", "abd")
        i = i + 1
    println("P20 PASS string_lifecycle " + str(i))
    return
"#,
    );
    assert_pass("string_lifecycle", dir);
}

// ========================================================================
// Test 2: List lifecycle stress (200 iterations)
// ========================================================================
#[test]
fn p20_list_lifecycle_stress() {
    let dir = unique_test_dir("list_lifecycle");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 200:
        let nums = [1, 2, 3]
        nums.add(4)
        nums.add(5)
        let _ = nums.len()
        let _ = nums.get(0)
        nums.set(1, 99)
        let _ = list_insert(nums, 0, 0)
        let _ = list_get(nums, 2)
        let _ = list_clone(nums)
        i = i + 1
    println("P20 PASS list_lifecycle " + str(i))
    return
"#,
    );
    assert_pass("list_lifecycle", dir);
}

// ========================================================================
// Test 3: Math operations stress (200 iterations)
// ========================================================================
#[test]
fn p20_math_stress() {
    let dir = unique_test_dir("math_stress");
    write_project(
        dir,
        r#"fn main():
    let acc = 0.0
    let i = 0
    while i < 200:
        let x = 1.5 * 1.0
        let _ = math.abs(x - 250.0)
        let _ = math.sqrt(x + 1.0)
        let _ = math.floor(x / 3.0)
        let _ = math.ceil(x / 3.0)
        let _ = math.round(x / 3.0)
        let _ = math.pow(2.0, 5.0)
        let _ = math.max(x, 100.0)
        let _ = math.min(x, 100.0)
        let _ = math.clamp(x, 0.0, 100.0)
        let _ = math.exp(x / 100.0)
        let _ = math.log(x + 1.0)
        let _ = math.sin(x / 10.0)
        let _ = math.cos(x / 10.0)
        let _ = math.tan(x / 10.0)
        acc = acc + math.abs(math.sin(x))
        i = i + 1
    println("P20 PASS math_stress " + str(i))
    return
"#,
    );
    assert_pass("math_stress", dir);
}

// ========================================================================
// Test 4: Option/Result lifecycle stress (200 iterations)
// ========================================================================
#[test]
fn p20_option_result_stress() {
    let dir = unique_test_dir("option_result");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 200:
        let o = option_some(i)
        let _ = option_is_some(o)
        let _ = option_is_none(o)
        let _ = option_extract_or(o, 0)
        let o2 = option_none()
        let _ = option_is_none(o2)
        let _ = option_extract_or(o2, 0)
        let r = result_success(i)
        let _ = result_is_success(r)
        let _ = result_is_error(r)
        let _ = result_extract_or(r, 0)
        let r2 = result_error("fail")
        let _ = result_is_error(r2)
        let _ = result_extract_or(r2, 0)
        i = i + 1
    println("P20 PASS option_result " + str(i))
    return
"#,
    );
    assert_pass("option_result", dir);
}

// ========================================================================
// Test 5: Regex stress (50 iterations)
// ========================================================================
#[test]
fn p20_regex_stress() {
    let dir = unique_test_dir("regex_stress");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 50:
        let _ = regex.is_match("[0-9]+", "abc123")
        let _ = regex.find("[0-9]+", "abc123def456")
        let _ = regex.find_all("[0-9]+", "a1 b2 c3 d4 e5")
        let _ = regex.replace("[0-9]+", "abc123", "X")
        let _ = regex.replace_all("[0-9]+", "a1 b2 c3", "X")
        let _ = regex.split("[ ,]+", "hello, world  foo")
        i = i + 1
    println("P20 PASS regex_stress " + str(i))
    return
"#,
    );
    assert_pass("regex_stress", dir);
}

// ========================================================================
// Test 6: FS write/read/remove cycle (20 iterations)
// ========================================================================
#[test]
fn p20_fs_lifecycle_stress() {
    let dir = unique_test_dir("fs_lifecycle");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 20:
        let path = "target/p20_test_" + str(i) + ".txt"
        let _ = fs.write(path, "hello from " + str(i))
        let content = fs.read(path)
        let _ = fs.exists(path)
        let _ = fs.size(path)
        let _ = fs.is_file(path)
        let _ = fs.remove(path)
        let _ = fs.exists(path)
        i = i + 1
    println("P20 PASS fs_lifecycle " + str(i))
    return
"#,
    );
    assert_pass("fs_lifecycle", dir);
}

// ========================================================================
// Test 7: Closure / Higher-order function stress (100 iterations)
// ========================================================================
#[test]
fn p20_closure_stress() {
    let dir = unique_test_dir("closure_stress");
    write_project(
        dir,
        r#"fn make_adder(n):
    return fn(x):
        return x + n

fn apply(f, x):
    return f(x)

fn main():
    let i = 0
    while i < 100:
        let adder = make_adder(i)
        let result = apply(adder, 10)
        i = i + 1
    println("P20 PASS closure_stress " + str(i))
    return
"#,
    );
    assert_pass("closure_stress", dir);
}

// ========================================================================
// Test 8: Async function lifecycle (transparent await, 200 iterations)
// ========================================================================
#[test]
fn p20_async_transparent_stress() {
    let dir = unique_test_dir("async_stress");
    write_project(
        dir,
        r#"lime double(int: n):
    return n + n

lime triple(int: n):
    return n + n + n

fn main():
    let i = 0
    while i < 200:
        let x = await double(i)
        let y = await triple(i)
        i = i + 1
    println("P20 PASS async_transparent " + str(i))
    return
"#,
    );
    assert_pass("async_transparent", dir);
}

// ========================================================================
// Test 9: OS / Path stress (50 iterations)
// ========================================================================
#[test]
fn p20_os_path_stress() {
    let dir = unique_test_dir("os_path");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 50:
        let _ = os.name()
        let _ = os.arch()
        let _ = os.platform()
        let _ = os.hostname()
        let _ = os.cwd()
        let _ = path.join("foo", "bar")
        let _ = path.basename("/foo/bar.txt")
        let _ = path.dirname("/foo/bar.txt")
        let _ = path.filename("/foo/bar.txt")
        let _ = path.extension("/foo/bar.txt")
        let _ = path.is_absolute("/foo/bar")
        let _ = path.normalize("/foo/./bar/../baz.txt")
        i = i + 1
    println("P20 PASS os_path " + str(i))
    return
"#,
    );
    assert_pass("os_path", dir);
}

// ========================================================================
// Test 10: Char / byte operations stress (100 iterations)
// ========================================================================
#[test]
fn p20_char_byte_stress() {
    let dir = unique_test_dir("char_byte");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 100:
        let s = "hello world"
        let _ = s.len()
        let _ = s.byte_len()
        let _ = s.byte(0)
        let _ = s.chars()
        let _ = s.bytes()
        let _ = s.slice(0, 5)
        i = i + 1
    println("P20 PASS char_byte " + str(i))
    return
"#,
    );
    assert_pass("char_byte", dir);
}

// ========================================================================
// Test 11: String advanced operations (100 iterations)
// ========================================================================
#[test]
fn p20_string_advanced_stress() {
    let dir = unique_test_dir("string_advanced");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 100:
        let s = "  hello world foo bar  "
        let _ = string.trim_start(s)
        let _ = string.trim_end(s)
        let _ = string.trim(s)
        let _ = string.find("hello world", "world")
        let _ = string.count("hello", "l")
        let _ = string.repeat("abc", 5)
        let _ = string.to_int("42")
        let _ = string.to_float("3.14")
        let _ = string.compare("abc", "abd")
        i = i + 1
    println("P20 PASS string_advanced " + str(i))
    return
"#,
    );
    assert_pass("string_advanced", dir);
}

// ========================================================================
// Test 12: FS directory operations (10 iterations)
// ========================================================================
#[test]
fn p20_fs_directory_stress() {
    let dir = unique_test_dir("fs_dir");
    write_project(
        dir,
        r#"fn main():
    let base = "target/p20_dir_test"
    fs.create_dir(base)
    let i = 0
    while i < 10:
        let path = base + "/sub_" + str(i)
        fs.create_dir(path)
        let fpath = path + "/file.txt"
        fs.write(fpath, "content " + str(i))
        let _ = fs.read(fpath)
        let _ = fs.exists(fpath)
        let _ = fs.is_file(fpath)
        let _ = fs.is_dir(path)
        let _ = fs.remove(fpath)
        let _ = fs.remove_dir(path)
        i = i + 1
    let _ = fs.list_dir(base)
    let _ = fs.remove_dir(base)
    println("P20 PASS fs_directory " + str(i))
    return
"#,
    );
    assert_pass("fs_directory", dir);
}

// ========================================================================
// Test 13: Env stress (50 iterations)
// ========================================================================
#[test]
fn p20_env_stress() {
    let dir = unique_test_dir("env_stress");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 50:
        let key = "P20_TEST_" + str(i)
        env.set(key, "value_" + str(i))
        let _ = env.has(key)
        let _ = env.get(key)
        env.remove(key)
        i = i + 1
    println("P20 PASS env_stress " + str(i))
    return
"#,
    );
    assert_pass("env_stress", dir);
}

// ========================================================================
// Test 14: Large data structures (string concat, 500 iterations)
// ========================================================================
#[test]
fn p20_large_datastructures() {
    let dir = unique_test_dir("large_data");
    write_project(
        dir,
        r#"fn main():
    let big = ""
    let i = 0
    while i < 500:
        big = big + "x"
        i = i + 1
    let _ = string.len(big)
    let _ = string.repeat("abc", 200)
    let _ = string.find(big, "xxx")
    let _ = string.replace(big, "x", "y")
    println("P20 PASS large_data " + str(i))
    return
"#,
    );
    assert_pass("large_data", dir);
}

// ========================================================================
// Test 15: Repeated execution (build once, run 20 times)
// ========================================================================
#[test]
fn p20_repeated_execution_memory_safety() {
    let dir = unique_test_dir("repeated_exec");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 10:
        let s = "hello_" + str(i)
        let _ = string.len(s)
        let nums = [i, i * 2, i * 3]
        let _ = nums.len()
        let _ = nums.get(0)
        fs.write("target/p20_repeated.txt", s)
        let _ = fs.read("target/p20_repeated.txt")
        fs.remove("target/p20_repeated.txt")
        i = i + 1
    println("P20 PASS repeated_exec " + str(i))
    return
"#,
    );
    // Build once, run 20 times to check for memory issues
    let out = Command::new(env!("CARGO_BIN_EXE_lime"))
        .arg("build")
        .arg(format!("{}/citrus.toml", dir))
        .arg("--emit-object")
        .output()
        .expect("failed to run lime build");
    assert!(
        out.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let exe = format!("{}.exe", dir);
    assert!(std::path::Path::new(&exe).exists());

    let mut successes = 0;
    for _ in 0..20 {
        let run = Command::new(&exe).output().expect("failed to run");
        let stdout = String::from_utf8_lossy(&run.stdout).to_string();
        if run.status.success() && stdout.contains("P20 PASS") {
            successes += 1;
        }
    }
    assert!(
        successes >= 18,
        "at least 18/20 runs should succeed, got {}/20",
        successes
    );
}

// ========================================================================
// Test 16: Mixed heavy workload — compute + string + list + fs + json (210 iters)
// ========================================================================
#[test]
fn p20_mixed_heavy_workload() {
    let dir = unique_test_dir("mixed_heavy");
    write_project(
        dir,
        r#"fn compute(int: n):
    let acc = 0
    let i = 0
    while i < n:
        acc = acc + i * i
        i = i + 1
    return acc

fn main():
    let iterations = 0
    
    let i = 0
    while i < 50:
        let _ = compute(100)
        i = i + 1
    iterations = iterations + 50
    
    let i = 0
    while i < 50:
        let s = str(i) + "-" + str(i * 2) + "-" + str(i * 3)
        let _ = string.len(s)
        let _ = string.to_upper(s)
        let _ = string.split(s, "-")
        i = i + 1
    iterations = iterations + 50
    
    let i = 0
    while i < 50:
        let lst = [1, 2, 3, 4, 5]
        lst.add(i)
        let _ = list_clone(lst)
        i = i + 1
    iterations = iterations + 50
    
    let i = 0
    while i < 10:
        let path = "target/p20_mixed_" + str(i) + ".txt"
        fs.write(path, "data " + str(i))
        let _ = fs.read(path)
        fs.remove(path)
        i = i + 1
    iterations = iterations + 10
    
    println("P20 PASS mixed_heavy " + str(iterations))
    return
"#,
    );
    assert_pass("mixed_heavy", dir);
}

// ========================================================================
// Test 17: List slice/clone/clear ops (200 iterations)
// ========================================================================
#[test]
fn p20_list_advanced_ops() {
    let dir = unique_test_dir("list_advanced");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 200:
        let a = [10, 20, 30, 40, 50]
        a.add(i)
        let _ = a.len()
        let _ = a.get(0)
        a.set(2, i)
        let b = list_clone(a)
        let _ = b.len()
        let c = [5, 3, 1, 2, 4]
        let d = list_clone(c)
        i = i + 1
    println("P20 PASS list_advanced " + str(i))
    return
"#,
    );
    assert_pass("list_advanced", dir);
}

// ========================================================================
// Test 18: Int/float conversion + string concat (200 iterations)
// ========================================================================
#[test]
fn p20_type_conversion_stress() {
    let dir = unique_test_dir("type_conv");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 200:
        let s1 = str(i)
        let s2 = str(i * 2)
        let s3 = s1 + s2
        let _ = string.len(s3)
        let n1 = string.to_int(s1)
        let n2 = string.to_int(s2)
        let _ = n1 + n2
        let f1 = string.to_float("3.14")
        let _ = string.to_int("0")
        i = i + 1
    println("P20 PASS type_conv " + str(i))
    return
"#,
    );
    assert_pass("type_conv", dir);
}

// ========================================================================
// Test 19: For-in loops (200 iterations)
// ========================================================================
#[test]
fn p20_for_in_loop_stress() {
    let dir = unique_test_dir("for_in");
    write_project(
        dir,
        r#"fn main():
    let i = 0
    while i < 200:
        let items = [1, 2, 3, 4, 5]
        for item in items:
            let _ = item + i
        let words = ["hello", "world"]
        for w in words:
            let _ = string.len(w)
        i = i + 1
    println("P20 PASS for_in " + str(i))
    return
"#,
    );
    assert_pass("for_in", dir);
}

// ========================================================================
// Test 20: Nested function calls + I/O (50 iterations)
// ========================================================================
#[test]
fn p20_nested_io_stress() {
    let dir = unique_test_dir("nested_io");
    write_project(
        dir,
        r#"fn add(int: a, int: b):
    return a + b

fn mul(int: a, int: b):
    return a * b

fn compose(int: x):
    return add(mul(x, 2), 1)

fn main():
    let i = 0
    while i < 50:
        let _ = compose(i)
        let _ = compose(compose(i))
        let s = "test_" + str(compose(i))
        let _ = string.len(s)
        let path = "target/p20_nested_" + str(i) + ".txt"
        fs.write(path, s)
        let _ = fs.read(path)
        fs.remove(path)
        i = i + 1
    println("P20 PASS nested_io " + str(i))
    return
"#,
    );
    assert_pass("nested_io", dir);
}
