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
    pub dependencies: BTreeMap<String, DependencySpec>,
    pub workspace: Option<WorkspaceSection>,
    pub build: BuildSection,
    pub tests: Vec<TestTarget>,
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DependencySpec {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TestTarget {
    pub name: String,
    pub entry: String,
    pub stdout: Option<String>,
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
    dependencies: Option<BTreeMap<String, RawDependencySpec>>,
    workspace: Option<RawWorkspaceSection>,
    build: Option<RawBuildSection>,
    tests: Option<Vec<RawTestTarget>>,
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawDependencySpec {
    Path(String),
    Detailed(RawDependencyDetail),
}

#[derive(Debug, Deserialize)]
struct RawDependencyDetail {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawTestTarget {
    name: Option<String>,
    entry: Option<String>,
    stdout: Option<String>,
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
    let dependencies = normalize_dependencies(raw.dependencies.unwrap_or_default(), path)?;
    let tests = normalize_tests(raw.tests.unwrap_or_default(), path)?;
    let workspace = normalize_workspace(raw.workspace, path)?;
    let capabilities = raw.capabilities.unwrap_or_default();
    Ok(Manifest {
        package: PackageSection {
            name: package_name,
            version: package_version,
        },
        dependencies,
        workspace,
        build: BuildSection { entry, out_dir },
        tests,
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

fn normalize_workspace(
    raw_workspace: Option<RawWorkspaceSection>,
    path: &Path,
) -> Result<Option<WorkspaceSection>, Diagnostic> {
    let Some(raw_workspace) = raw_workspace else {
        return Ok(None);
    };
    let mut members = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (index, member) in raw_workspace
        .members
        .unwrap_or_default()
        .into_iter()
        .enumerate()
    {
        let field_name = format!("workspace.members[{index}]");
        let member = required_field(Some(member), path, &field_name)?;
        validate_relative_path(path, &field_name, &member)?;
        if !seen.insert(member.clone()) {
            return Err(Diagnostic::new(
                "manifest",
                format!("duplicate workspace member {member:?}"),
            )
            .with_path(path.display().to_string()));
        }
        members.push(member);
    }
    Ok(Some(WorkspaceSection { members }))
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

fn normalize_dependencies(
    raw_dependencies: BTreeMap<String, RawDependencySpec>,
    path: &Path,
) -> Result<BTreeMap<String, DependencySpec>, Diagnostic> {
    let mut dependencies = BTreeMap::new();
    for (name, raw_spec) in raw_dependencies {
        if name.trim().is_empty() {
            return Err(
                Diagnostic::new("manifest", "dependency names must not be empty")
                    .with_path(path.display().to_string()),
            );
        }
        let raw_path = match raw_spec {
            RawDependencySpec::Path(value) => value,
            RawDependencySpec::Detailed(detail) => {
                required_field(detail.path, path, &format!("dependencies.{name}.path"))?
            }
        };
        validate_dependency_path(path, &format!("dependencies.{name}.path"), &raw_path)?;
        dependencies.insert(name, DependencySpec { path: raw_path });
    }
    Ok(dependencies)
}

fn normalize_tests(
    raw_tests: Vec<RawTestTarget>,
    path: &Path,
) -> Result<Vec<TestTarget>, Diagnostic> {
    let mut tests = Vec::new();
    let mut names = std::collections::BTreeSet::new();
    for (index, raw_test) in raw_tests.into_iter().enumerate() {
        let field_prefix = format!("tests[{index}]");
        let name = required_field(raw_test.name, path, &format!("{field_prefix}.name"))?;
        if !names.insert(name.clone()) {
            return Err(
                Diagnostic::new("manifest", format!("duplicate test target {name:?}"))
                    .with_path(path.display().to_string()),
            );
        }
        let entry = required_field(raw_test.entry, path, &format!("{field_prefix}.entry"))?;
        validate_relative_path(path, &format!("{field_prefix}.entry"), &entry)?;
        tests.push(TestTarget {
            name,
            entry,
            stdout: raw_test.stdout,
        });
    }
    Ok(tests)
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

fn validate_dependency_path(path: &Path, field_name: &str, value: &str) -> Result<(), Diagnostic> {
    if Path::new(value).is_absolute() {
        return Err(
            Diagnostic::new("manifest", format!("{field_name} must be relative"))
                .with_path(path.display().to_string()),
        );
    }
    Ok(())
}
