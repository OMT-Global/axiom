use crate::diagnostics::Diagnostic;
use crate::manifest::{capability_descriptors, load_manifest, manifest_path};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const REGISTRY_METADATA_FILENAME: &str = "axiom-registry.toml";
const DEFAULT_ARCHIVE_FILENAME: &str = "package.axp";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryIndex {
    pub version: u32,
    pub packages: BTreeMap<String, Vec<RegistryRelease>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryCapability {
    pub name: String,
    pub enabled: bool,
    pub description: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub allowed: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub unsafe_unrestricted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegistryRelease {
    pub version: String,
    pub source: String,
    pub manifest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub yanked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yank_reason: Option<String>,
    pub capabilities: Vec<RegistryCapability>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRegistryMetadata {
    archive: Option<String>,
    signature: Option<String>,
    yanked: Option<bool>,
    yank_reason: Option<String>,
}

pub fn build_registry_index(
    packages_root: &Path,
    base_url: &str,
) -> Result<RegistryIndex, Diagnostic> {
    let base_url = normalize_base_url(base_url, packages_root)?;
    let mut packages = BTreeMap::new();
    for package_dir in read_sorted_dirs(packages_root)? {
        let package_name = file_name(&package_dir)?;
        let mut releases = Vec::new();
        for version_dir in read_sorted_dirs(&package_dir)? {
            let release = load_release(&package_name, &version_dir, &base_url)?;
            releases.push(release);
        }
        if !releases.is_empty() {
            packages.insert(package_name, releases);
        }
    }
    Ok(RegistryIndex {
        version: 1,
        packages,
    })
}

pub fn render_registry_index(packages_root: &Path, base_url: &str) -> Result<String, Diagnostic> {
    let index = build_registry_index(packages_root, base_url)?;
    serde_json::to_string_pretty(&index).map_err(|err| {
        Diagnostic::new(
            "registry",
            format!("failed to render registry index: {err}"),
        )
    })
}

pub fn load_registry_index(path: &Path) -> Result<RegistryIndex, Diagnostic> {
    let content = fs::read_to_string(path).map_err(|err| {
        Diagnostic::new("registry", format!("failed to read registry index: {err}"))
            .with_path(path.display().to_string())
    })?;
    let index: RegistryIndex = serde_json::from_str(&content).map_err(|err| {
        Diagnostic::new("registry", format!("invalid registry index: {err}"))
            .with_path(path.display().to_string())
    })?;
    validate_registry_index(&index, Some(path))?;
    Ok(index)
}

pub fn validate_registry_index(
    index: &RegistryIndex,
    path: Option<&Path>,
) -> Result<(), Diagnostic> {
    if index.version != 1 {
        return Err(registry_error(
            path,
            format!(
                "unsupported registry index version {}; expected 1",
                index.version
            ),
        ));
    }
    for (package, releases) in &index.packages {
        if package.trim().is_empty() {
            return Err(registry_error(path, "package names must not be empty"));
        }
        let mut seen_versions = std::collections::BTreeSet::new();
        for release in releases {
            if release.version.trim().is_empty() {
                return Err(registry_error(
                    path,
                    format!("package {package:?} contains an empty version string"),
                ));
            }
            if !seen_versions.insert(release.version.clone()) {
                return Err(registry_error(
                    path,
                    format!(
                        "package {package:?} contains duplicate version {:?}",
                        release.version
                    ),
                ));
            }
            if release.archive.is_some() && release.signature.is_none() {
                return Err(registry_error(
                    path,
                    format!(
                        "package {package:?} version {:?} declares an archive without a signature",
                        release.version
                    ),
                ));
            }
            if release.yank_reason.is_some() && !release.yanked {
                return Err(registry_error(
                    path,
                    format!(
                        "package {package:?} version {:?} has yank_reason but is not yanked",
                        release.version
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn load_release(
    package_name: &str,
    version_dir: &Path,
    base_url: &str,
) -> Result<RegistryRelease, Diagnostic> {
    let version = file_name(version_dir)?;
    let manifest = load_manifest(version_dir)?;
    let manifest_path = manifest_path(version_dir);
    let package = manifest.package.as_ref().ok_or_else(|| {
        Diagnostic::new(
            "registry",
            "registry release manifest requires a [package] section",
        )
        .with_path(manifest_path.display().to_string())
    })?;
    if package.name != package_name {
        return Err(Diagnostic::new(
            "registry",
            format!(
                "package directory {:?} does not match manifest package name {:?}",
                package_name, package.name
            ),
        )
        .with_path(manifest_path.display().to_string()));
    }
    if package.version != version {
        return Err(Diagnostic::new(
            "registry",
            format!(
                "version directory {:?} does not match manifest package version {:?}",
                version, package.version
            ),
        )
        .with_path(manifest_path.display().to_string()));
    }
    let metadata = load_registry_metadata(version_dir)?;
    let archive_file = match metadata.archive {
        Some(value) => Some(value),
        None => version_dir
            .join(DEFAULT_ARCHIVE_FILENAME)
            .exists()
            .then(|| String::from(DEFAULT_ARCHIVE_FILENAME)),
    };
    let signature_file = match metadata.signature {
        Some(value) => Some(value),
        None => archive_file.as_ref().and_then(|archive| {
            version_dir
                .join(format!("{archive}.sig"))
                .exists()
                .then(|| format!("{archive}.sig"))
        }),
    };
    if archive_file.is_some() && signature_file.is_none() {
        return Err(Diagnostic::new(
            "registry",
            format!(
                "registry release {package_name}@{version} includes an archive but no signature"
            ),
        )
        .with_path(version_dir.display().to_string()));
    }
    Ok(RegistryRelease {
        version: package.version.clone(),
        source: format!("registry+{}/{}/{}", base_url, package_name, version),
        manifest: format!("{}/{}/{}/axiom.toml", base_url, package_name, version),
        archive: archive_file
            .map(|file| format!("{}/{}/{}/{}", base_url, package_name, version, file)),
        signature: signature_file
            .map(|file| format!("{}/{}/{}/{}", base_url, package_name, version, file)),
        yanked: metadata.yanked.unwrap_or(false),
        yank_reason: metadata.yank_reason,
        capabilities: capability_descriptors(&manifest.capabilities)
            .into_iter()
            .map(|capability| RegistryCapability {
                name: capability.name,
                enabled: capability.enabled,
                description: capability.description.to_string(),
                allowed: capability.allowed,
                unsafe_unrestricted: capability.unsafe_unrestricted,
            })
            .collect(),
    })
}

fn load_registry_metadata(version_dir: &Path) -> Result<RawRegistryMetadata, Diagnostic> {
    let path = version_dir.join(REGISTRY_METADATA_FILENAME);
    if !path.exists() {
        return Ok(RawRegistryMetadata::default());
    }
    let content = fs::read_to_string(&path).map_err(|err| {
        Diagnostic::new(
            "registry",
            format!("failed to read {REGISTRY_METADATA_FILENAME}: {err}"),
        )
        .with_path(path.display().to_string())
    })?;
    toml::from_str(&content).map_err(|err| {
        Diagnostic::new(
            "registry",
            format!("invalid {REGISTRY_METADATA_FILENAME}: {err}"),
        )
        .with_path(path.display().to_string())
    })
}

fn read_sorted_dirs(path: &Path) -> Result<Vec<PathBuf>, Diagnostic> {
    let mut dirs = fs::read_dir(path)
        .map_err(|err| {
            Diagnostic::new(
                "registry",
                format!("failed to read {}: {err}", path.display()),
            )
            .with_path(path.display().to_string())
        })?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|entry| entry.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    Ok(dirs)
}

fn file_name(path: &Path) -> Result<String, Diagnostic> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
        .ok_or_else(|| Diagnostic::new("registry", format!("invalid path {}", path.display())))
}

fn normalize_base_url(base_url: &str, packages_root: &Path) -> Result<String, Diagnostic> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(Diagnostic::new("registry", "--base-url must not be empty")
            .with_path(packages_root.display().to_string()));
    }
    Ok(trimmed.to_string())
}

fn registry_error(path: Option<&Path>, message: impl Into<String>) -> Diagnostic {
    let diagnostic = Diagnostic::new("registry", message.into());
    if let Some(path) = path {
        diagnostic.with_path(path.display().to_string())
    } else {
        diagnostic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_release(root: &Path, package: &str, version: &str, manifest: &str) -> PathBuf {
        let dir = root.join(package).join(version);
        fs::create_dir_all(&dir).expect("create release dir");
        fs::write(dir.join("axiom.toml"), manifest).expect("write manifest");
        dir
    }

    #[test]
    fn builds_static_registry_index_with_capabilities_and_yanks() {
        let dir = tempdir().expect("tempdir");
        let release = write_release(
            dir.path(),
            "core",
            "1.2.3",
            "[package]\nname = \"core\"\nversion = \"1.2.3\"\n\n[build]\nentry = \"src/main.ax\"\nout_dir = \"dist\"\n\n[capabilities]\nnet = true\nenv = [\"API_TOKEN\"]\n",
        );
        fs::write(release.join("package.axp"), "archive").expect("write archive");
        fs::write(release.join("package.axp.sig"), "signature").expect("write signature");
        fs::write(
            release.join("axiom-registry.toml"),
            "yanked = true\nyank_reason = \"security fix required\"\n",
        )
        .expect("write metadata");

        let index = build_registry_index(dir.path(), "https://packages.example.test/registry/")
            .expect("build index");
        let release = &index.packages["core"][0];
        assert_eq!(
            release.source,
            "registry+https://packages.example.test/registry/core/1.2.3"
        );
        assert_eq!(
            release.archive.as_deref(),
            Some("https://packages.example.test/registry/core/1.2.3/package.axp")
        );
        assert_eq!(
            release.signature.as_deref(),
            Some("https://packages.example.test/registry/core/1.2.3/package.axp.sig")
        );
        assert!(release.yanked);
        assert_eq!(
            release.yank_reason.as_deref(),
            Some("security fix required")
        );
        assert!(
            release
                .capabilities
                .iter()
                .any(|cap| cap.name == "net" && cap.enabled)
        );
        let env = release
            .capabilities
            .iter()
            .find(|cap| cap.name == "env")
            .expect("env cap");
        assert_eq!(env.allowed, vec![String::from("API_TOKEN")]);
    }

    #[test]
    fn rejects_unsigned_archives() {
        let dir = tempdir().expect("tempdir");
        let release = write_release(
            dir.path(),
            "core",
            "1.0.0",
            "[package]\nname = \"core\"\nversion = \"1.0.0\"\n\n[build]\nentry = \"src/main.ax\"\nout_dir = \"dist\"\n",
        );
        fs::write(release.join("package.axp"), "archive").expect("write archive");
        let error = build_registry_index(dir.path(), "https://packages.example.test")
            .expect_err("unsigned archive should fail");
        assert_eq!(error.kind, "registry");
        assert!(error.message.contains("archive but no signature"));
    }

    #[test]
    fn validates_index_file_contract() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("index.json");
        fs::write(
            &path,
            r#"{"version":1,"packages":{"core":[{"version":"1.0.0","source":"registry+https://packages.example.test/core/1.0.0","manifest":"https://packages.example.test/core/1.0.0/axiom.toml","archive":"https://packages.example.test/core/1.0.0/package.axp","signature":"https://packages.example.test/core/1.0.0/package.axp.sig","yanked":false,"capabilities":[]}]}}"#,
        )
        .expect("write index");
        load_registry_index(&path).expect("valid index");
    }
}
