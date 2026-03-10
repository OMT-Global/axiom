use crate::diagnostics::Diagnostic;
use crate::manifest::{Manifest, lockfile_path};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lockfile {
    pub version: u32,
    pub package: Vec<LockedPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String,
}

pub fn expected_lockfile(manifest: &Manifest) -> Lockfile {
    Lockfile {
        version: 1,
        package: vec![LockedPackage {
            name: manifest.package.name.clone(),
            version: manifest.package.version.clone(),
            source: String::from("path"),
        }],
    }
}

pub fn render_lockfile(manifest: &Manifest) -> Result<String, Diagnostic> {
    toml::to_string_pretty(&expected_lockfile(manifest))
        .map_err(|err| Diagnostic::new("lockfile", format!("failed to render axiom.lock: {err}")))
}

pub fn validate_lockfile(project_root: &Path, manifest: &Manifest) -> Result<(), Diagnostic> {
    let path = lockfile_path(project_root);
    let content = std::fs::read_to_string(&path).map_err(|err| {
        Diagnostic::new("lockfile", format!("failed to read axiom.lock: {err}"))
            .with_path(path.display().to_string())
    })?;
    let lockfile: Lockfile = toml::from_str(&content).map_err(|err| {
        Diagnostic::new("lockfile", format!("invalid axiom.lock: {err}"))
            .with_path(path.display().to_string())
    })?;
    let expected = expected_lockfile(manifest);
    if lockfile != expected {
        return Err(
            Diagnostic::new(
                "lockfile",
                "axiom.lock does not match axiom.toml; regenerate it with `axiomc new` or update it manually",
            )
            .with_path(path.display().to_string()),
        );
    }
    Ok(())
}
