use std::fs;
use std::path::Path;
use std::process::Command;

fn citrus_bin() -> String {
    std::env::var("CITRUS_BIN")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_citrus").to_string())
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

fn create_valid_project(base: &Path, name: &str) {
    let project_dir = base.join(name);
    fs::create_dir_all(&project_dir).unwrap();
    fs::create_dir_all(project_dir.join("src")).unwrap();
    let toml = r#"[package]
name = "test_project"
version = "0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(project_dir.join("citrus.toml"), toml).unwrap();
    fs::write(
        project_dir.join("src").join("main.lime"),
        "fn main():\n    println(\"Hello, Lime!\")\n",
    )
    .unwrap();
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
fn test_build_from_subdirectory() {
    let tmp = Path::new("/tmp/citrus_test_build_subdir");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    create_valid_project(tmp, "subdir_test");

    let subdir = tmp.join("subdir_test").join("src");
    let (code, stderr, _) = run_citrus(&["build"], &subdir);
    assert_eq!(code, 0, "citrus build from subdirectory failed: stderr={}", stderr);

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
fn test_run_with_arguments() {
    let tmp = Path::new("/tmp/citrus_test_run_args");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "run_args_test"
version = "v0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(
        tmp.join("src").join("main.lime"),
        "fn main():\n    println(\"Hello, Lime!\")\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run_citrus(&["run", "--", "arg1", "arg2"], tmp);
    assert_eq!(code, 0, "citrus run with args failed: stderr={}", stderr);
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

#[test]
fn test_build_invalid_toml_missing_name() {
    let tmp = Path::new("/tmp/citrus_test_invalid_toml_name");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
version = "0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("main.lime"), "fn main():\n    println(\"Hello\")\n").unwrap();

    let (code, _, stderr) = run_citrus(&["build"], tmp);
    assert_ne!(code, 0, "citrus build should fail with missing package name");
    assert!(stderr.contains("name"), "expected error about missing name: {}", stderr);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_build_invalid_toml_missing_version() {
    let tmp = Path::new("/tmp/citrus_test_invalid_toml_version");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "version_test"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("main.lime"), "fn main():\n    println(\"Hello\")\n").unwrap();

    let (code, _, stderr) = run_citrus(&["build"], tmp);
    assert_ne!(code, 0, "citrus build should fail with missing package version");
    assert!(stderr.contains("version"), "expected error about missing version: {}", stderr);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_build_missing_src_main_lime() {
    let tmp = Path::new("/tmp/citrus_test_missing_main");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "missing_main_test"
version = "0.1.0"

[files]
main = "src/main.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();
    fs::create_dir_all(tmp.join("src")).unwrap();

    let (code, _, _) = run_citrus(&["build"], tmp);
    assert_ne!(code, 0, "citrus build should fail with missing src/main.lime");

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_build_custom_main_file() {
    let tmp = Path::new("/tmp/citrus_test_custom_main");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let toml = r#"[package]
name = "custom_main_test"
version = "0.1.0"

[files]
main = "src/app.lime"

[import]
"#;
    fs::write(tmp.join("citrus.toml"), toml).unwrap();
    fs::create_dir_all(tmp.join("src")).unwrap();
    fs::write(tmp.join("src").join("app.lime"), "fn main():\n    println(\"Custom main\")\n").unwrap();

    let (code, stderr, _) = run_citrus(&["build"], tmp);
    assert_eq!(code, 0, "citrus build with custom main file failed: stderr={}", stderr);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_add_dependency() {
    let tmp = Path::new("/tmp/citrus_test_add");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    create_valid_project(tmp, "add_test");
    let project_dir = tmp.join("add_test");

    let (code, stdout, stderr) = run_citrus(&["add", "string"], &project_dir);
    assert_eq!(code, 0, "citrus add failed: stderr={}", stderr);
    assert!(stdout.contains("string"), "expected string in output: {}", stdout);

    let toml = fs::read_to_string(project_dir.join("citrus.toml")).unwrap();
    assert!(toml.contains("[dependencies]"), "citrus.toml missing [dependencies]: {}", toml);
    assert!(toml.contains("string = \"v0.1.0\""), "citrus.toml missing string dep: {}", toml);

    let pkg_dir = project_dir.join("packages").join("string").join("v0.1.0");
    assert!(pkg_dir.join("citrus.toml").exists(), "package citrus.toml not materialized");
    assert!(pkg_dir.join("src").join("string.lime").exists(), "package source not materialized");

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_remove_dependency() {
    let tmp = Path::new("/tmp/citrus_test_remove");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    create_valid_project(tmp, "remove_test");
    let project_dir = tmp.join("remove_test");

    let (code, _, stderr) = run_citrus(&["add", "math"], &project_dir);
    assert_eq!(code, 0, "citrus add failed: stderr={}", stderr);

    let (code, stdout, stderr) = run_citrus(&["remove", "math"], &project_dir);
    assert_eq!(code, 0, "citrus remove failed: stderr={}", stderr);
    assert!(stdout.contains("Removed math"), "expected Removed output: {}", stdout);

    let toml = fs::read_to_string(project_dir.join("citrus.toml")).unwrap();
    assert!(!toml.contains("math"), "math should be removed from citrus.toml: {}", toml);

    let (code, _, stderr) = run_citrus(&["remove", "math"], &project_dir);
    assert_ne!(code, 0, "removing a non-dependency should fail");
    assert!(stderr.contains("not a declared dependency"), "expected error: {}", stderr);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_add_unknown_package_fails() {
    let tmp = Path::new("/tmp/citrus_test_add_unknown");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    create_valid_project(tmp, "unknown_test");
    let project_dir = tmp.join("unknown_test");

    let (code, _, stderr) = run_citrus(&["add", "does_not_exist"], &project_dir);
    assert_ne!(code, 0, "adding an unknown package should fail");
    assert!(stderr.contains("not found"), "expected not found error: {}", stderr);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_install_generates_lock() {
    let tmp = Path::new("/tmp/citrus_test_install");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    create_valid_project(tmp, "install_test");
    let project_dir = tmp.join("install_test");

    let (code, stdout, stderr) = run_citrus(&["add", "string"], &project_dir);
    assert_eq!(code, 0, "citrus add failed: stderr={}", stderr);

    let lock = project_dir.join("citrus.lock");
    assert!(lock.exists(), "citrus.lock not generated");
    let lock_text = fs::read_to_string(&lock).unwrap();
    assert!(lock_text.contains("string"), "lock missing string package: {}", lock_text);
    assert!(lock_text.contains("[[package]]"), "lock missing [[package]] sections: {}", lock_text);

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_build_with_dependency() {
    let tmp = Path::new("/tmp/citrus_test_build_dep");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let project_dir = tmp.join("build_dep");
    fs::create_dir_all(&project_dir).unwrap();
    fs::create_dir_all(project_dir.join("src")).unwrap();
    let toml = r#"[package]
name = "build_dep"
version = "0.1.0"

[files]
main = "src/main.lime"

[dependencies]
string = "v0.1.0"
"#;
    fs::write(project_dir.join("citrus.toml"), toml).unwrap();
    fs::write(
        project_dir.join("src").join("main.lime"),
        "fn main():\n    println(\"hello\")\n",
    )
    .unwrap();

    let (code, stdout, stderr) = run_citrus(&["build", "--release"], &project_dir);
    assert_eq!(code, 0, "citrus build with dependency failed: stderr={}", stderr);
    assert!(stdout.contains("Finished"), "expected build to finish: {}", stdout);

    let pkg_dir = project_dir.join("packages").join("string").join("v0.1.0");
    assert!(pkg_dir.join("citrus.toml").exists(), "dependency not auto-materialized on build");
    assert!(project_dir.join("citrus.lock").exists(), "citrus.lock not generated on build");

    fs::remove_dir_all(tmp).unwrap();
}

#[test]
fn test_build_is_reproducible() {
    let tmp = Path::new("/tmp/citrus_test_reproducible");
    if tmp.exists() {
        fs::remove_dir_all(tmp).unwrap();
    }
    fs::create_dir_all(tmp).unwrap();

    let project_dir = tmp.join("repro");
    fs::create_dir_all(&project_dir).unwrap();
    fs::create_dir_all(project_dir.join("src")).unwrap();
    let toml = r#"[package]
name = "repro"
version = "0.1.0"

[files]
main = "src/main.lime"

[dependencies]
string = "v0.1.0"
"#;
    fs::write(project_dir.join("citrus.toml"), toml).unwrap();
    fs::write(project_dir.join("src").join("main.lime"), "fn main():\n    println(\"hi\")\n").unwrap();

    let (code, _, stderr) = run_citrus(&["build", "--release"], &project_dir);
    assert_eq!(code, 0, "first build failed: stderr={}", stderr);
    let first_lock = fs::read_to_string(project_dir.join("citrus.lock")).unwrap();

    let (code, _, stderr) = run_citrus(&["build", "--release"], &project_dir);
    assert_eq!(code, 0, "second build failed: stderr={}", stderr);
    let second_lock = fs::read_to_string(project_dir.join("citrus.lock")).unwrap();

    assert_eq!(first_lock, second_lock, "citrus.lock should be deterministic");

    fs::remove_dir_all(tmp).unwrap();
}
