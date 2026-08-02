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

pub fn manifest_dir() -> Result<PathBuf, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot get current directory: {}", e))?;
    let manifest = cwd.join("citrus.toml");
    if manifest.exists() {
        Ok(cwd)
    } else {
        Err("citrus.toml not found in current directory".to_string())
    }
}

fn target_dir(project_root: &Path, profile: Profile) -> PathBuf {
    project_root.join("target").join(profile.dir_name())
}

pub fn build(profile: Profile) -> Result<BuildResult, String> {
    let project_root = manifest_dir()?;
    let cfg = parse_citrus_toml(&project_root.join("citrus.toml").to_string_lossy())?;
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

    // Change to project root so compile_pipeline writes output.* there.
    std::env::set_current_dir(&project_root)
        .map_err(|e| format!("failed to chdir to project root: {}", e))?;
    let report = compile_pipeline("citrus.toml", CompileMode::Build, &opts)?;

    let out_base = project_root.join("output");

    // Move .ll -> target/<profile>/ir/
    let ll_src = out_base.with_extension("ll");
    if ll_src.exists() {
        let ll_dst = ir_dir.join(format!("{}.ll", project_name));
        let _ = fs::rename(&ll_src, &ll_dst);
    }

    // Move .opt.ll -> target/<profile>/ir/
    let opt_ll_src = project_root.join("output.opt.ll");
    if opt_ll_src.exists() {
        let opt_ll_dst = ir_dir.join(format!("{}.opt.ll", project_name));
        let _ = fs::rename(&opt_ll_src, &opt_ll_dst);
    }

    // Move .obj -> target/<profile>/obj/
    let obj_ext = if cfg!(target_os = "windows") { "obj" } else { "o" };
    let obj_src = out_base.with_extension(obj_ext);
    let mut objects = Vec::new();
    if obj_src.exists() {
        let obj_dst = obj_dir.join(format!("{}.{}", project_name, obj_ext));
        let _ = fs::rename(&obj_src, &obj_dst);
        objects.push(obj_dst);
    }

    // Move .exe -> target/<profile>/app.exe
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

pub fn run(release: bool) -> Result<(), String> {
    let profile = if release { Profile::Release } else { Profile::Debug };
    let result = build(profile)?;

    if !result.executable.exists() {
        return Err(format!(
            "executable not found at '{}'",
            result.executable.display()
        ));
    }

    let project_root = manifest_dir()?;
    let status = Command::new(&result.executable)
        .current_dir(&project_root)
        .status()
        .map_err(|e| format!("failed to run executable: {}", e))?;

    if !status.success() {
        return Err(format!("program exited with code {:?}", status.code()));
    }

    Ok(())
}