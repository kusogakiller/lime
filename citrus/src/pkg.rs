use std::fs;
use std::path::{Path, PathBuf};

use lime::{parse_citrus_toml, CitrusToml};

/// Root of the stdlib packages bundled with citrus. Overridable via
/// `CITRUS_PACKAGE_ROOT`; otherwise resolves relative to this crate's manifest
/// directory (the lime repository layout: `<repo>/packages`).
fn bundled_registry_root() -> PathBuf {
    if let Some(root) = std::env::var_os("CITRUS_PACKAGE_ROOT") {
        return PathBuf::from(root);
    }
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../packages"))
}

fn project_registry(project_root: &Path) -> PathBuf {
    project_root.join("packages")
}

fn normalize_version(v: &str) -> String {
    if v.starts_with('v') {
        v.to_string()
    } else {
        format!("v{}", v)
    }
}

fn version_key(v: &str) -> (u32, u32, u32) {
    let s = v.trim_start_matches('v');
    let parts: Vec<&str> = s.split('.').collect();
    let get = |i: usize| parts.get(i).and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
    (get(0), get(1), get(2))
}

/// Highest available version of `name` across the bundled stdlib and the
/// project-local registry.
fn find_latest_version(name: &str) -> Option<String> {
    let mut best: Option<String> = None;
    let mut registries: Vec<PathBuf> = vec![bundled_registry_root()];
    if let Ok(root) = super::build::find_project_root() {
        registries.push(project_registry(&root));
    }
    for reg in &registries {
        let dir = reg.join(name);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let Some(ver) = entry.file_name().to_str().map(String::from) else {
                continue;
            };
            if !ver.starts_with('v') {
                continue;
            }
            best = match best {
                None => Some(ver.clone()),
                Some(ref cur) if version_key(&ver) > version_key(cur) => Some(ver),
                _ => best,
            };
        }
    }
    best
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("failed to create {}: {}", dst.display(), e))?;
    for entry in fs::read_dir(src)
        .map_err(|e| format!("failed to read {}: {}", src.display(), e))?
        .flatten()
    {
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target)?;
        } else {
            fs::copy(&path, &target)
                .map_err(|e| format!("failed to copy {}: {}", path.display(), e))?;
        }
    }
    Ok(())
}

/// Read a project manifest, applying version normalization to every declared
/// dependency (accepts both `0.1.0` and `v0.1.0`).
fn normalized_manifest(toml_path: &Path) -> Result<CitrusToml, String> {
    let mut cfg = parse_citrus_toml(&toml_path.to_string_lossy())?;
    let names: Vec<String> = cfg.imports.keys().cloned().collect();
    for name in names {
        let v = cfg.imports[&name].clone();
        cfg.imports.insert(name, normalize_version(&v));
    }
    Ok(cfg)
}

/// Materialize every declared dependency into the project registry and
/// (re)write `citrus.lock`. When `force` is true, project-local copies are
/// refreshed from the bundled stdlib first. Returns the number of packages
/// newly installed (or refreshed when `force`).
fn sync_deps(project_root: &Path, force: bool) -> Result<usize, String> {
    let toml_path = project_root.join("citrus.toml");
    let cfg = normalized_manifest(&toml_path)?;
    let registry = project_registry(project_root);
    let bundled = bundled_registry_root();

    let mut count = 0usize;
    for (name, ver) in &cfg.imports {
        let dest = registry.join(name).join(ver);
        if force && dest.join("citrus.toml").exists() {
            let _ = fs::remove_dir_all(registry.join(name));
        }
        if dest.join("citrus.toml").exists() {
            continue;
        }
        let src = bundled.join(name).join(ver);
        if !src.join("citrus.toml").exists() {
            return Err(format!(
                "unresolved dependency: '{}' version '{}' not found in bundled stdlib",
                name, ver
            ));
        }
        copy_dir_recursive(&src, &dest)?;
        count += 1;
    }

    let base = project_root.to_string_lossy().to_string();
    let reg = registry.to_string_lossy().to_string();
    lime::resolve_and_write_lock(&base, &cfg, &reg)?;
    Ok(count)
}

pub fn add(name: &str) -> Result<(), String> {
    let version = find_latest_version(name)
        .ok_or_else(|| format!("package '{}' not found in bundled stdlib or local registry", name))?;
    let project_root = super::build::discover_project()?;
    let toml_path = project_root.join("citrus.toml");
    let text = fs::read_to_string(&toml_path)
        .map_err(|e| format!("failed to read {}: {}", toml_path.display(), e))?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    if add_to_manifest(&mut lines, name, &version) {
        println!("{} is already a dependency; updated to {}", name, version);
    } else {
        println!("Adding {} {}", name, version);
    }
    fs::write(&toml_path, lines.join("\n") + "\n")
        .map_err(|e| format!("failed to write {}: {}", toml_path.display(), e))?;
    sync_deps(&project_root, false)?;
    Ok(())
}

pub fn remove(name: &str) -> Result<(), String> {
    let project_root = super::build::discover_project()?;
    let toml_path = project_root.join("citrus.toml");
    let text = fs::read_to_string(&toml_path)
        .map_err(|e| format!("failed to read {}: {}", toml_path.display(), e))?;
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    if !remove_from_manifest(&mut lines, name) {
        return Err(format!("'{}' is not a declared dependency", name));
    }
    fs::write(&toml_path, lines.join("\n") + "\n")
        .map_err(|e| format!("failed to write {}: {}", toml_path.display(), e))?;
    let cfg = normalized_manifest(&toml_path)?;
    let registry = project_registry(&project_root);
    let base = project_root.to_string_lossy().to_string();
    let reg = registry.to_string_lossy().to_string();
    lime::resolve_and_write_lock(&base, &cfg, &reg)?;
    println!("Removed {}", name);
    Ok(())
}

pub fn install() -> Result<(), String> {
    let project_root = super::build::discover_project()?;
    let count = sync_deps(&project_root, false)?;
    if count == 0 {
        println!("Dependencies are already installed");
    } else {
        println!("Installed {} package(s)", count);
    }
    Ok(())
}

/// Ensure every declared dependency is materialized in the project registry
/// (used by `build` / `run` so a fresh checkout compiles without a prior
/// explicit `citrus install`). Prints nothing when nothing changed.
pub fn ensure_deps(project_root: &std::path::Path) -> Result<(), String> {
    sync_deps(project_root, false)?;
    Ok(())
}

pub fn update() -> Result<(), String> {
    let project_root = super::build::discover_project()?;
    let count = sync_deps(&project_root, true)?;
    println!("Updated {} package(s)", count);
    Ok(())
}

/// Insert or update `name = "version"` in the `[dependencies]` section of the
/// manifest lines. Creates the section if absent. Returns true when an entry
/// already existed (updated in place).
fn add_to_manifest(lines: &mut Vec<String>, name: &str, version: &str) -> bool {
    let mut deps_start: Option<usize> = None;
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with('[') && t.ends_with(']') {
            if t == "[dependencies]" {
                deps_start = Some(i + 1);
            } else if deps_start.is_some() {
                break;
            }
        }
        i += 1;
    }
    if let Some(start) = deps_start {
        let mut insert_at = start;
        let mut j = start;
        while j < lines.len() {
            let t = lines[j].trim();
            if t.starts_with('[') && t.ends_with(']') {
                break;
            }
            if let Some(eq) = t.find('=') {
                if t[..eq].trim() == name {
                    lines[j] = format!("{} = \"{}\"", name, version);
                    return true;
                }
            }
            insert_at = j + 1;
            j += 1;
        }
        lines.insert(insert_at, format!("{} = \"{}\"", name, version));
        return false;
    }
    lines.push(String::new());
    lines.push("[dependencies]".to_string());
    lines.push(format!("{} = \"{}\"", name, version));
    false
}

/// Remove `name = "version"` from the `[dependencies]` section. Returns false
/// when the name is not present there.
fn remove_from_manifest(lines: &mut Vec<String>, name: &str) -> bool {
    let mut in_deps = false;
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with('[') && t.ends_with(']') {
            in_deps = t == "[dependencies]";
            i += 1;
            continue;
        }
        if in_deps {
            if let Some(eq) = t.find('=') {
                if t[..eq].trim() == name {
                    lines.remove(i);
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}
