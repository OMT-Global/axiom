use crate::diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILENAME: &str = "axiom.toml";
pub const LOCK_FILENAME: &str = "axiom.lock";
pub const KNOWN_CAPABILITIES: [CapabilityKind; 6] = [
    CapabilityKind::Fs,
    CapabilityKind::Net,
    CapabilityKind::Process,
    CapabilityKind::Env,
    CapabilityKind::Clock,
    CapabilityKind::Crypto,
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Manifest {
    pub package: PackageSection,
    pub dependencies: BTreeMap<String, String>,
    pub workspace: Option<WorkspaceSection>,
    pub build: BuildSection,
    pub capabilities: CapabilityConfig,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PackageSection {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkspaceSection {
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BuildSection {
    pub entry: String,
    pub out_dir: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct CapabilityConfig {
    pub fs: bool,
    pub net: bool,
    pub process: bool,
    pub env: bool,
    pub clock: bool,
    pub crypto: bool,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Fs,
    Net,
    Process,
    Env,
    Clock,
    Crypto,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub name: String,
    pub enabled: bool,
    pub description: &'static str,
}

#[derive(Debug, Deserialize)]
struct RawManifest {
    package: Option<RawPackageSection>,
    dependencies: Option<BTreeMap<String, String>>,
    workspace: Option<RawWorkspaceSection>,
    build: Option<RawBuildSection>,
    capabilities: Option<RawCapabilityConfig>,
}

#[derive(Debug, Deserialize)]
struct RawPackageSection {
    name: Option<String>,
    version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawWorkspaceSection {
    members: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawBuildSection {
    entry: Option<String>,
    out_dir: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RawCapabilityConfig {
    fs: Option<bool>,
    net: Option<bool>,
    process: Option<bool>,
    env: Option<bool>,
    clock: Option<bool>,
    crypto: Option<bool>,
}

pub fn load_manifest(project_root: &Path) -> Result<Manifest, Diagnostic> {
    let path = manifest_path(project_root);
    let content = std::fs::read_to_string(&path).map_err(|err| {
        Diagnostic::new(
            "manifest",
            format!("failed to read {}: {err}", MANIFEST_FILENAME),
        )
        .with_path(path.display().to_string())
    })?;
    let raw: RawManifest = toml::from_str(&content).map_err(|err| {
        Diagnostic::new("manifest", format!("invalid {MANIFEST_FILENAME}: {err}"))
            .with_path(path.display().to_string())
    })?;
    normalize_manifest(raw, &path)
}

pub fn manifest_path(project_root: &Path) -> PathBuf {
    project_root.join(MANIFEST_FILENAME)
}

pub fn lockfile_path(project_root: &Path) -> PathBuf {
    project_root.join(LOCK_FILENAME)
}

pub fn entry_path(project_root: &Path, manifest: &Manifest) -> PathBuf {
    project_root.join(&manifest.build.entry)
}

pub fn out_dir_path(project_root: &Path, manifest: &Manifest) -> PathBuf {
    project_root.join(&manifest.build.out_dir)
}

pub fn binary_path(project_root: &Path, manifest: &Manifest) -> PathBuf {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    out_dir_path(project_root, manifest).join(format!("{}{}", manifest.package.name, suffix))
}

pub fn generated_rust_path(project_root: &Path, manifest: &Manifest) -> PathBuf {
    out_dir_path(project_root, manifest).join(format!("{}.generated.rs", manifest.package.name))
}

pub fn capability_descriptors(config: &CapabilityConfig) -> Vec<CapabilityDescriptor> {
    KNOWN_CAPABILITIES
        .iter()
        .map(|kind| CapabilityDescriptor {
            name: kind.name().to_string(),
            enabled: config.enabled(*kind),
            description: kind.description(),
        })
        .collect()
}

pub fn render_manifest(name: &str) -> String {
    format!(
        "[package]\nname = {name:?}\nversion = \"0.1.0\"\n\n[build]\nentry = \"src/main.ax\"\nout_dir = \"dist\"\n\n[capabilities]\nfs = false\nnet = false\nprocess = false\nenv = false\nclock = false\ncrypto = false\n"
    )
}

impl CapabilityConfig {
    pub fn enabled(&self, kind: CapabilityKind) -> bool {
        match kind {
            CapabilityKind::Fs => self.fs,
            CapabilityKind::Net => self.net,
            CapabilityKind::Process => self.process,
            CapabilityKind::Env => self.env,
            CapabilityKind::Clock => self.clock,
            CapabilityKind::Crypto => self.crypto,
        }
    }
}

impl CapabilityKind {
    pub fn name(self) -> &'static str {
        match self {
            CapabilityKind::Fs => "fs",
            CapabilityKind::Net => "net",
            CapabilityKind::Process => "process",
            CapabilityKind::Env => "env",
            CapabilityKind::Clock => "clock",
            CapabilityKind::Crypto => "crypto",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            CapabilityKind::Fs => "filesystem access",
            CapabilityKind::Net => "network access",
            CapabilityKind::Process => "child process execution",
            CapabilityKind::Env => "environment variable access",
            CapabilityKind::Clock => "wall-clock time access",
            CapabilityKind::Crypto => "hashing and cryptography primitives",
        }
    }
}

fn normalize_manifest(raw: RawManifest, path: &Path) -> Result<Manifest, Diagnostic> {
    let package = raw.package.ok_or_else(|| {
        Diagnostic::new("manifest", "missing [package] section")
            .with_path(path.display().to_string())
    })?;
    let package_name = required_field(package.name, path, "package.name")?;
    let package_version = required_field(package.version, path, "package.version")?;
    let build = raw.build.unwrap_or(RawBuildSection {
        entry: Some(String::from("src/main.ax")),
        out_dir: Some(String::from("dist")),
    });
    let entry = required_field(build.entry, path, "build.entry")?;
    let out_dir = required_field(build.out_dir, path, "build.out_dir")?;
    validate_relative_path(path, "build.entry", &entry)?;
    validate_relative_path(path, "build.out_dir", &out_dir)?;
    let workspace = raw.workspace.map(|workspace| WorkspaceSection {
        members: workspace.members.unwrap_or_default(),
    });
    let capabilities = raw.capabilities.unwrap_or_default();
    Ok(Manifest {
        package: PackageSection {
            name: package_name,
            version: package_version,
        },
        dependencies: raw.dependencies.unwrap_or_default(),
        workspace,
        build: BuildSection { entry, out_dir },
        capabilities: CapabilityConfig {
            fs: capabilities.fs.unwrap_or(false),
            net: capabilities.net.unwrap_or(false),
            process: capabilities.process.unwrap_or(false),
            env: capabilities.env.unwrap_or(false),
            clock: capabilities.clock.unwrap_or(false),
            crypto: capabilities.crypto.unwrap_or(false),
        },
    })
}

fn required_field(
    value: Option<String>,
    path: &Path,
    field_name: &str,
) -> Result<String, Diagnostic> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(
            Diagnostic::new("manifest", format!("missing or empty {field_name}"))
                .with_path(path.display().to_string()),
        ),
    }
}

fn validate_relative_path(path: &Path, field_name: &str, value: &str) -> Result<(), Diagnostic> {
    let candidate = Path::new(value);
    if candidate.is_absolute() {
        return Err(
            Diagnostic::new("manifest", format!("{field_name} must be relative"))
                .with_path(path.display().to_string()),
        );
    }
    if candidate
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(Diagnostic::new(
            "manifest",
            format!("{field_name} must not use parent traversal"),
        )
        .with_path(path.display().to_string()));
    }
    Ok(())
}
