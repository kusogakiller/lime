use std::fs;
use std::path::Path;

pub fn create_project(name: &str) -> Result<(), String> {
    let root = Path::new(name);
    if root.exists() {
        return Err(format!("directory '{}' already exists", name));
    }

    fs::create_dir(root).map_err(|e| format!("failed to create directory '{}': {}", name, e))?;
    fs::create_dir(root.join("src")).map_err(|e| format!("failed to create 'src' directory: {}", e))?;

    let manifest = format!(
        r#"[package]
name = "{}"
version = "0.1.0"

[files]
main = "src/main.lime"
"#,
        name
    );
    fs::write(root.join("citrus.toml"), &manifest)
        .map_err(|e| format!("failed to write citrus.toml: {}", e))?;

    let main_lime = "fn main():\n    println(\"Hello, Lime!\")\n";
    fs::write(root.join("src").join("main.lime"), main_lime)
        .map_err(|e| format!("failed to write src/main.lime: {}", e))?;

    println!("Created project '{}'", name);
    Ok(())
}
