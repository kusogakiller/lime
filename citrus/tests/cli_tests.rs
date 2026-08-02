use std::fs;
use std::path::Path;
use std::process::Command;

fn citrus_bin() -> String {
    std::env::var("CITRUS_BIN")
        .unwrap_or_else(|_| "target/release/citrus".to_string())
}

fn run_citrus(args: &[&str], cwd: &Path) -> (i32, String, String) {
    let output = Command::new(citrus_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run citrus");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.code().unwrap_or(1), stdout, stderr)
}

#[test]
fn test_new_creates_project_structure() {
    let tmp = Path::new("/tmp/citrus_test_new");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let (code, stdout, stderr) = run_citrus(&["new", "myproject"], tmp);
    assert_eq!(code, 0, "citrus new failed: stderr={}", stderr);
    assert!(stdout.contains("Created project 'myproject'"), "unexpected output: {}", stdout);

    let project_dir = tmp.join("myproject");
    assert!(project_dir.join("citrus.toml").exists(), "citrus.toml not created");
    assert!(project_dir.join("src").join("main.lime").exists(), "src/main.lime not created");

    let toml = fs::read_to_string(project_dir.join("citrus.toml")).unwrap();
    assert!(toml.contains("[package]"), "citrus.toml missing [package]");
    assert!(toml.contains("name = \"myproject\""), "citrus.toml missing project name");

    let main_lime = fs::read_to_string(project_dir.join("src").join("main.lime")).unwrap();
    assert!(main_lime.contains("Hello, Lime!"), "main.lime should say Hello, Lime!");

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_new_fails_for_existing_directory() {
    let tmp = Path::new("/tmp/citrus_test_new_exist");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let existing = tmp.join("existing");
    fs::create_dir_all(&existing).unwrap();

    let (code, _, stderr) = run_citrus(&["new", "existing"], tmp);
    assert_ne!(code, 0, "citrus new should fail for existing directory");
    assert!(stderr.contains("already exists"), "expected 'already exists' error: {}", stderr);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_build_finds_citrus_toml() {
    let tmp = Path::new("/tmp/citrus_test_build");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "build_test"
version = "v0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("main.lime"), "fn main():\n    println(\"Hello\")\n").unwrap();

    let (code, stderr, _) = run_citrus(&["build"], tmp);
    assert_eq!(code, 0, "citrus build failed: stderr={}", stderr);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_build_missing_citrus_toml() {
    let tmp = Path::new("/tmp/citrus_test_build_missing");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let (code, _, stderr) = run_citrus(&["build"], tmp);
    assert_ne!(code, 0, "citrus build should fail without citrus.toml");
    assert!(stderr.contains("citrus.toml not found"), "expected citrus.toml not found error");

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_run_builds_and_executes() {
    let tmp = Path::new("/tmp/citrus_test_run");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "run_test"
version = "v0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("main.lime"), "fn main():\n    println(\"Hello, Lime!\")\n").unwrap();

    let (code, stdout, stderr) = run_citrus(&["run"], tmp);
    assert_eq!(code, 0, "citrus run failed: stderr={}", stderr);
    assert!(stdout.contains("Hello, Lime!"), "expected Hello, Lime! in output, got: {}", stdout);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_fmt_reports_not_implemented() {
    let tmp = Path::new("/tmp/citrus_test_fmt");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "fmt_test"
version = "v0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();

    let (code, _, stderr) = run_citrus(&["fmt"], tmp);
    assert_ne!(code, 0, "citrus fmt should fail");
    assert!(stderr.contains("not implemented"), "expected 'not implemented' message");

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_test_reports_no_cargo_toml() {
    let tmp = Path::new("/tmp/citrus_test_test");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "test_test"
version = "v0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();

    let (code, _, stderr) = run_citrus(&["test"], tmp);
    assert_ne!(code, 0, "citrus test should fail without Cargo.toml");
    assert!(stderr.contains("Cargo.toml"), "expected Cargo.toml error message");

    fs::remove_dir_all(tmp).unwrap();
}