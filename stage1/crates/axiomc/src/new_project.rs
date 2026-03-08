use crate::diagnostics::Diagnostic;
use crate::lockfile::render_lockfile;
use crate::manifest::{
    BuildSection, CapabilityConfig, LOCK_FILENAME, MANIFEST_FILENAME, Manifest, PackageSection,
    render_manifest,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn create_project(path: &Path, name: Option<&str>) -> Result<(), Diagnostic> {
    if path.exists() {
        let mut entries = fs::read_dir(path).map_err(|err| {
            Diagnostic::new("new", format!("failed to read {}: {err}", path.display()))
        })?;
        if entries.next().is_some() {
            return Err(Diagnostic::new("new", "project directory must be empty")
                .with_path(path.display().to_string()));
        }
    } else {
        fs::create_dir_all(path).map_err(|err| {
            Diagnostic::new("new", format!("failed to create {}: {err}", path.display()))
        })?;
    }

    let project_name = sanitize_name(name.unwrap_or_else(|| {
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("axiom-app")
    }));
    let src_dir = path.join("src");
    fs::create_dir_all(&src_dir).map_err(|err| {
        Diagnostic::new(
            "new",
            format!("failed to create {}: {err}", src_dir.display()),
        )
    })?;
    let manifest_text = render_manifest(&project_name);
    fs::write(path.join(MANIFEST_FILENAME), manifest_text).map_err(|err| {
        Diagnostic::new("new", format!("failed to write {MANIFEST_FILENAME}: {err}"))
            .with_path(path.join(MANIFEST_FILENAME).display().to_string())
    })?;
    let manifest = Manifest {
        package: PackageSection {
            name: project_name.clone(),
            version: String::from("0.1.0"),
        },
        dependencies: BTreeMap::new(),
        workspace: None,
        build: BuildSection {
            entry: String::from("src/main.ax"),
            out_dir: String::from("dist"),
        },
        capabilities: CapabilityConfig::default(),
    };
    let lock_text = render_lockfile(&manifest)?;
    fs::write(path.join(LOCK_FILENAME), lock_text).map_err(|err| {
        Diagnostic::new("new", format!("failed to write {LOCK_FILENAME}: {err}"))
            .with_path(path.join(LOCK_FILENAME).display().to_string())
    })?;
    fs::write(src_dir.join("main.ax"), "print \"hello from stage1\"\n").map_err(|err| {
        Diagnostic::new("new", format!("failed to write src/main.ax: {err}"))
            .with_path(src_dir.join("main.ax").display().to_string())
    })?;
    Ok(())
}

fn sanitize_name(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            last_dash = false;
            ch.to_ascii_lowercase()
        } else {
            if last_dash {
                continue;
            }
            last_dash = true;
            '-'
        };
        out.push(next);
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        String::from("axiom-app")
    } else {
        trimmed.to_string()
    }
}
