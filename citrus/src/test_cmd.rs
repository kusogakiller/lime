use std::path::Path;
use std::process::Command;

use crate::build::manifest_dir;

pub fn run(release: bool) -> Result<(), String> {
    let project_root = manifest_dir()?;

    let cargo_toml = project_root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Err("no Cargo.toml found in project root; cannot run tests".to_string());
    }

    let profile = if release { "release" } else { "debug" };
    println!("Running tests in {} profile...", profile);

    let status = Command::new("cargo")
        .arg("test")
        .current_dir(&project_root)
        .status()
        .map_err(|e| format!("failed to run cargo test: {}", e))?;

    if !status.success() {
        return Err(format!("tests failed with exit code {:?}", status.code()));
    }

    println!("All tests passed.");
    Ok(())
}