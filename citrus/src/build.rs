use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use lime::{compile_pipeline, CompileMode, CompileOptions, parse_citrus_toml};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    fn dir_name(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }

    fn opt_level(self) -> &'static str {
        match self {
            Profile::Debug => "0",
            Profile::Release => "2",
        }
    }

    fn optimize(self) -> bool {
        match self {
            Profile::Debug => false,
            Profile::Release => true,
        }
    }
}

pub struct BuildConfig {
    pub project_root: PathBuf,
    pub profile: Profile,
    pub target_dir: PathBuf,
}

pub struct BuildResult {
    pub executable: PathBuf,
    pub objects: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

pub fn find_project_root() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot get current directory: {}", e))?;
    let mut dir = cwd.as_path();
    loop {
        let manifest = dir.join("citrus.toml");
        if manifest.exists() {
            return Ok(dir.to_path_buf());
        }
        dir = match dir.parent() {
            Some(parent) => parent,
            None => {
                return Err(format!(
                    "citrus.toml not found in '{}' or any parent directory",
                    cwd.display()
                ));
            }
        };
    }
}

pub fn manifest_dir() -> Result<PathBuf, String> {
    find_project_root()
}

fn target_dir(project_root: &Path, profile: Profile) -> PathBuf {
    project_root.join("target").join(profile.dir_name())
}

pub fn discover_project() -> Result<PathBuf, String> {
    find_project_root()
}

pub fn load_manifest(project_root: &Path) -> Result<lime::CitrusToml, String> {
    let toml_path = project_root.join("citrus.toml");
    let toml_str = toml_path.to_string_lossy();
    let cfg = parse_citrus_toml(&toml_str)?;
    if cfg.name.is_empty() {
        return Err(format!("{}: [package].name is required", toml_path.display()));
    }
    if cfg.version.is_empty() {
        return Err(format!("{}: [package].version is required", toml_path.display()));
    }
    if !cfg.files.contains_key("main") {
        return Err(format!(
            "{}: [files] main is required (e.g. main = \"src/main.lime\")",
            toml_path.display()
        ));
    }
    Ok(cfg)
}

pub fn build(profile: Profile) -> Result<BuildResult, String> {
    let project_root = discover_project()?;
    let cfg = load_manifest(&project_root)?;
    let project_name = cfg.name.clone();

    let target = target_dir(&project_root, profile);

    let obj_dir = target.join("obj");
    let ir_dir = target.join("ir");
    fs::create_dir_all(&obj_dir)
        .map_err(|e| format!("failed to create {}: {}", obj_dir.display(), e))?;
    fs::create_dir_all(&ir_dir)
        .map_err(|e| format!("failed to create {}: {}", ir_dir.display(), e))?;

    let opts = CompileOptions {
        emit_ll: true,
        emit_object: true,
        optimize: profile.optimize(),
        release: profile == Profile::Release,
        verbose: false,
    };

    std::env::set_current_dir(&project_root)
        .map_err(|e| format!("failed to chdir to project root: {}", e))?;
    let report = compile_pipeline("citrus.toml", CompileMode::Build, &opts)?;

    let out_base = project_root.join("output");

    let ll_src = out_base.with_extension("ll");
    if ll_src.exists() {
        let ll_dst = ir_dir.join(format!("{}.ll", project_name));
        let _ = fs::rename(&ll_src, &ll_dst);
    }

    let opt_ll_src = project_root.join("output.opt.ll");
    if opt_ll_src.exists() {
        let opt_ll_dst = ir_dir.join(format!("{}.opt.ll", project_name));
        let _ = fs::rename(&opt_ll_src, &opt_ll_dst);
    }

    let obj_ext = if cfg!(target_os = "windows") { "obj" } else { "o" };
    let obj_src = out_base.with_extension(obj_ext);
    let mut objects = Vec::new();
    if obj_src.exists() {
        let obj_dst = obj_dir.join(format!("{}.{}", project_name, obj_ext));
        let _ = fs::rename(&obj_src, &obj_dst);
        objects.push(obj_dst);
    }

    let exe_ext = "exe";
    let exe_src = out_base.with_extension(exe_ext);
    let mut executable = PathBuf::new();
    if exe_src.exists() {
        let exe_dst = target.join("app.exe");
        let _ = fs::rename(&exe_src, &exe_dst);
        executable = exe_dst;
    }

    let mut warnings = report.warnings;
    warnings.extend(report.codegen_warnings);

    println!(
        "Finished `{}` profile [{}] target(s)",
        profile.dir_name(),
        project_name,
    );

    Ok(BuildResult {
        executable,
        objects,
        warnings,
    })
}

pub fn run(release: bool, args: &[String]) -> Result<(), String> {
    let profile = if release { Profile::Release } else { Profile::Debug };
    let result = build(profile)?;

    if !result.executable.exists() {
        return Err(format!(
            "executable not found at '{}'",
            result.executable.display()
        ));
    }

    let project_root = find_project_root()?;
    let mut cmd = Command::new(&result.executable);
    cmd.current_dir(&project_root);
    for arg in args {
        cmd.arg(arg);
    }
    let status = cmd
        .status()
        .map_err(|e| format!("failed to run executable: {}", e))?;

    if !status.success() {
        return Err(format!("program exited with code {:?}", status.code()));
    }

    Ok(())
}