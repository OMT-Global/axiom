use crate::codegen::{compile_native, render_rust};
use crate::diagnostics::Diagnostic;
use crate::hir;
use crate::lockfile::validate_lockfile;
use crate::manifest::{
    CapabilityDescriptor, Manifest, binary_path, capability_descriptors, entry_path,
    generated_rust_path, load_manifest, manifest_path, out_dir_path,
};
use crate::mir;
use crate::syntax;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct CheckOutput {
    pub manifest: String,
    pub entry: String,
    pub statement_count: usize,
    pub capabilities: Vec<CapabilityDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildOutput {
    pub manifest: String,
    pub entry: String,
    pub binary: String,
    pub generated_rust: String,
    pub statement_count: usize,
}

pub fn check_project(project_root: &Path) -> Result<CheckOutput, Diagnostic> {
    let analyzed = analyze_project(project_root)?;
    Ok(CheckOutput {
        manifest: manifest_path(project_root).display().to_string(),
        entry: analyzed.entry_path.display().to_string(),
        statement_count: analyzed.mir.statement_count(),
        capabilities: capability_descriptors(&analyzed.manifest.capabilities),
    })
}

pub fn build_project(project_root: &Path) -> Result<BuildOutput, Diagnostic> {
    let analyzed = analyze_project(project_root)?;
    let out_dir = out_dir_path(project_root, &analyzed.manifest);
    fs::create_dir_all(&out_dir).map_err(|err| {
        Diagnostic::new(
            "build",
            format!("failed to create {}: {err}", out_dir.display()),
        )
    })?;
    let generated_rust = generated_rust_path(project_root, &analyzed.manifest);
    let rust_source = render_rust(&analyzed.mir);
    fs::write(&generated_rust, rust_source).map_err(|err| {
        Diagnostic::new(
            "build",
            format!("failed to write {}: {err}", generated_rust.display()),
        )
    })?;
    let binary = binary_path(project_root, &analyzed.manifest);
    compile_native(&generated_rust, &binary)?;
    Ok(BuildOutput {
        manifest: manifest_path(project_root).display().to_string(),
        entry: analyzed.entry_path.display().to_string(),
        binary: binary.display().to_string(),
        generated_rust: generated_rust.display().to_string(),
        statement_count: analyzed.mir.statement_count(),
    })
}

pub fn run_project(project_root: &Path) -> Result<i32, Diagnostic> {
    let built = build_project(project_root)?;
    let status = Command::new(&built.binary).status().map_err(|err| {
        Diagnostic::new("run", format!("failed to execute {}: {err}", built.binary))
    })?;
    Ok(status.code().unwrap_or(1))
}

pub fn project_capabilities(project_root: &Path) -> Result<Vec<CapabilityDescriptor>, Diagnostic> {
    let manifest = load_manifest(project_root)?;
    Ok(capability_descriptors(&manifest.capabilities))
}

struct AnalyzedProject {
    manifest: Manifest,
    entry_path: PathBuf,
    mir: mir::Program,
}

fn analyze_project(project_root: &Path) -> Result<AnalyzedProject, Diagnostic> {
    let manifest = load_manifest(project_root)?;
    if manifest
        .workspace
        .as_ref()
        .is_some_and(|workspace| !workspace.members.is_empty())
    {
        return Err(Diagnostic::new(
            "manifest",
            "stage1 bootstrap does not support workspace members yet",
        )
        .with_path(manifest_path(project_root).display().to_string()));
    }
    if !manifest.dependencies.is_empty() {
        return Err(Diagnostic::new(
            "manifest",
            "stage1 bootstrap does not support dependencies yet",
        )
        .with_path(manifest_path(project_root).display().to_string()));
    }
    validate_lockfile(project_root, &manifest)?;
    let entry = entry_path(project_root, &manifest);
    let source = fs::read_to_string(&entry).map_err(|err| {
        Diagnostic::new(
            "source",
            format!("failed to read {}: {err}", entry.display()),
        )
        .with_path(entry.display().to_string())
    })?;
    let syntax = syntax::parse_program(&source, &entry)?;
    let hir = hir::lower(&syntax)?;
    let mir = mir::lower(&hir);
    Ok(AnalyzedProject {
        manifest,
        entry_path: entry,
        mir,
    })
}
