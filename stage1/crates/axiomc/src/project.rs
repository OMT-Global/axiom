use crate::codegen::{compile_native, render_rust};
use crate::diagnostics::Diagnostic;
use crate::hir;
use crate::lockfile::validate_lockfile;
use crate::manifest::{
    BuildSection, CapabilityConfig, CapabilityDescriptor, CapabilityKind, Manifest, PackageSection,
    binary_path, capability_descriptors, entry_path, generated_rust_path, load_manifest,
    manifest_path, out_dir_path,
};
use crate::mir;
use crate::stdlib;
use crate::syntax;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Clone, Serialize)]
pub struct CheckedPackage {
    pub package_root: String,
    pub manifest: String,
    pub entry: String,
    pub statement_count: usize,
    pub capabilities: Vec<CapabilityDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckOutput {
    pub manifest: String,
    pub entry: String,
    pub statement_count: usize,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub packages: Vec<CheckedPackage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuiltPackage {
    pub package_root: String,
    pub manifest: String,
    pub entry: String,
    pub binary: String,
    pub generated_rust: String,
    pub statement_count: usize,
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildOutput {
    pub manifest: String,
    pub entry: String,
    pub binary: String,
    pub generated_rust: String,
    pub statement_count: usize,
    pub target: Option<String>,
    pub packages: Vec<BuiltPackage>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestCaseResult {
    pub package_root: String,
    pub name: String,
    pub entry: String,
    pub ok: bool,
    pub binary: Option<String>,
    pub generated_rust: Option<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub expected_stdout: Option<String>,
    pub duration_ms: u64,
    pub error: Option<Diagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestOutput {
    pub manifest: String,
    pub packages: Vec<String>,
    pub cases: Vec<TestCaseResult>,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    pub package: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub target: Option<String>,
    pub package: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct RunOptions {
    pub package: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TestOptions {
    pub filter: Option<String>,
    pub package: Option<String>,
}

pub fn check_project(project_root: &Path) -> Result<CheckOutput, Diagnostic> {
    check_project_with_options(project_root, &CheckOptions::default())
}

pub fn check_project_with_options(
    project_root: &Path,
    options: &CheckOptions,
) -> Result<CheckOutput, Diagnostic> {
    let project_root = normalize_path(project_root);
    let graph = load_package_graph(&project_root)?;
    validate_workspace_root_lockfile(&graph, &project_root)?;
    let mut packages = Vec::new();
    for package_root in workspace_package_roots(&graph, &project_root, options.package.as_deref())?
    {
        let analyzed = analyze_package(&graph, &package_root)?;
        packages.push(CheckedPackage {
            package_root: package_root.display().to_string(),
            manifest: manifest_path(&package_root).display().to_string(),
            entry: analyzed.entry_path.display().to_string(),
            statement_count: analyzed.mir.statement_count(),
            capabilities: capability_descriptors(&analyzed.manifest.capabilities),
        });
    }
    let root = packages.first().cloned().ok_or_else(|| {
        Diagnostic::new(
            "manifest",
            format!(
                "internal error: no packages discovered for {}",
                project_root.display()
            ),
        )
    })?;
    Ok(CheckOutput {
        manifest: root.manifest,
        entry: root.entry,
        statement_count: root.statement_count,
        capabilities: root.capabilities,
        packages,
    })
}

pub fn build_project(project_root: &Path) -> Result<BuildOutput, Diagnostic> {
    build_project_with_options(project_root, &BuildOptions::default())
}

pub fn build_project_with_options(
    project_root: &Path,
    options: &BuildOptions,
) -> Result<BuildOutput, Diagnostic> {
    let project_root = normalize_path(project_root);
    let graph = load_package_graph(&project_root)?;
    validate_workspace_root_lockfile(&graph, &project_root)?;
    let mut packages = Vec::new();
    for package_root in workspace_package_roots(&graph, &project_root, options.package.as_deref())?
    {
        let analyzed = analyze_package(&graph, &package_root)?;
        let generated_rust = generated_rust_path(&package_root, &analyzed.manifest);
        let binary = binary_path(&package_root, &analyzed.manifest);
        build_artifacts(&analyzed, &generated_rust, &binary, options)?;
        packages.push(BuiltPackage {
            package_root: package_root.display().to_string(),
            manifest: manifest_path(&package_root).display().to_string(),
            entry: analyzed.entry_path.display().to_string(),
            binary: binary.display().to_string(),
            generated_rust: generated_rust.display().to_string(),
            statement_count: analyzed.mir.statement_count(),
            target: options.target.clone(),
        });
    }
    let root = packages.first().cloned().ok_or_else(|| {
        Diagnostic::new(
            "manifest",
            format!(
                "internal error: no packages discovered for {}",
                project_root.display()
            ),
        )
    })?;
    Ok(BuildOutput {
        manifest: root.manifest,
        entry: root.entry,
        binary: root.binary,
        generated_rust: root.generated_rust,
        statement_count: root.statement_count,
        target: root.target,
        packages,
    })
}

pub fn run_project(project_root: &Path) -> Result<i32, Diagnostic> {
    run_project_with_options(project_root, &RunOptions::default())
}

pub fn run_project_with_options(
    project_root: &Path,
    options: &RunOptions,
) -> Result<i32, Diagnostic> {
    let project_root = normalize_path(project_root);
    let graph = load_package_graph(&project_root)?;
    if options.package.is_none() && graph.context(&project_root)?.manifest.is_workspace_only() {
        return Err(Diagnostic::new(
            "run",
            "workspace-only manifests require -p/--package for `axiomc run`",
        )
        .with_path(manifest_path(&project_root).display().to_string()));
    }
    let built = build_project_with_options(
        &project_root,
        &BuildOptions {
            target: None,
            package: options.package.clone(),
        },
    )?;
    let status = Command::new(&built.binary).status().map_err(|err| {
        Diagnostic::new("run", format!("failed to execute {}: {err}", built.binary))
    })?;
    Ok(status.code().unwrap_or(1))
}

pub fn run_project_tests(project_root: &Path) -> Result<TestOutput, Diagnostic> {
    run_project_tests_with_options(project_root, &TestOptions::default())
}

pub fn run_project_tests_with_options(
    project_root: &Path,
    options: &TestOptions,
) -> Result<TestOutput, Diagnostic> {
    let project_root = normalize_path(project_root);
    let graph = load_package_graph(&project_root)?;
    validate_workspace_root_lockfile(&graph, &project_root)?;
    let manifest_path_text = manifest_path(&project_root).display().to_string();
    let mut packages = Vec::new();
    let mut cases = Vec::new();
    let started = Instant::now();
    for package_root in workspace_package_roots(&graph, &project_root, options.package.as_deref())?
    {
        let manifest = graph.context(&package_root)?.manifest.clone();
        validate_lockfile(&package_root, &manifest)?;
        let tests = collect_test_targets(&package_root, &manifest, options.filter.as_deref())?;
        if tests.is_empty() {
            continue;
        }
        packages.push(package_root.display().to_string());
        for test in &tests {
            cases.push(run_test_case(&package_root, &graph, &manifest, test));
        }
    }
    if cases.is_empty() {
        return Err(Diagnostic::new(
            "test",
            "no tests discovered under src/**/*_test.ax across the workspace and no [[tests]] configured in axiom.toml",
        )
        .with_path(manifest_path_text));
    }
    let passed = cases.iter().filter(|case| case.ok).count();
    let failed = cases.len().saturating_sub(passed);
    Ok(TestOutput {
        manifest: manifest_path(&project_root).display().to_string(),
        packages,
        cases,
        passed,
        failed,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn collect_test_targets(
    project_root: &Path,
    manifest: &Manifest,
    filter: Option<&str>,
) -> Result<Vec<crate::manifest::TestTarget>, Diagnostic> {
    let mut tests = manifest.tests.clone();
    let mut seen_entries = tests
        .iter()
        .map(|test| test.entry.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for discovered in discover_test_targets(project_root)? {
        if seen_entries.insert(discovered.entry.clone()) {
            tests.push(discovered);
        }
    }
    if let Some(filter) = filter {
        tests.retain(|test| test_matches_filter(test, filter));
    }
    Ok(tests)
}

fn discover_test_targets(
    project_root: &Path,
) -> Result<Vec<crate::manifest::TestTarget>, Diagnostic> {
    let src_root = project_root.join("src");
    if !src_root.exists() {
        return Ok(Vec::new());
    }
    let mut tests = Vec::new();
    collect_discovered_tests(project_root, &src_root, &mut tests)?;
    tests.sort_by(|left, right| left.entry.cmp(&right.entry));
    Ok(tests)
}

fn collect_discovered_tests(
    project_root: &Path,
    dir: &Path,
    tests: &mut Vec<crate::manifest::TestTarget>,
) -> Result<(), Diagnostic> {
    let entries = fs::read_dir(dir).map_err(|err| {
        Diagnostic::new("test", format!("failed to read {}: {err}", dir.display()))
            .with_path(dir.display().to_string())
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            Diagnostic::new("test", format!("failed to read {}: {err}", dir.display()))
                .with_path(dir.display().to_string())
        })?;
        let path = entry.path();
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect_discovered_tests(project_root, &path, tests)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("ax") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !stem.ends_with("_test") {
            continue;
        }
        let relative = normalize_path(path.strip_prefix(project_root).unwrap_or(&path));
        let stdout_path = path.with_extension("stdout");
        let stdout = if stdout_path.exists() {
            Some(fs::read_to_string(&stdout_path).map_err(|err| {
                Diagnostic::new(
                    "test",
                    format!("failed to read {}: {err}", stdout_path.display()),
                )
                .with_path(stdout_path.display().to_string())
            })?)
        } else {
            None
        };
        tests.push(crate::manifest::TestTarget {
            name: relative.with_extension("").display().to_string(),
            entry: relative.display().to_string(),
            stdout,
        });
    }
    Ok(())
}

pub fn project_capabilities(project_root: &Path) -> Result<Vec<CapabilityDescriptor>, Diagnostic> {
    let manifest = load_manifest(project_root)?;
    Ok(capability_descriptors(&manifest.capabilities))
}

#[derive(Debug, Clone)]
struct PackageContext {
    root: PathBuf,
    manifest: Manifest,
    source_root: PathBuf,
    dependencies: BTreeMap<String, PathBuf>,
    workspace_members: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
struct PackageGraph {
    packages: HashMap<PathBuf, PackageContext>,
}

impl PackageGraph {
    fn context(&self, package_root: &Path) -> Result<&PackageContext, Diagnostic> {
        self.packages.get(package_root).ok_or_else(|| {
            Diagnostic::new(
                "manifest",
                format!(
                    "internal error: unknown package root {}",
                    package_root.display()
                ),
            )
        })
    }
}

struct AnalyzedProject {
    manifest: Manifest,
    entry_path: PathBuf,
    mir: mir::Program,
}

#[derive(Debug, Clone)]
struct LoadedModule {
    path: PathBuf,
    program: syntax::Program,
    is_entry: bool,
    package_root: PathBuf,
    source_root: PathBuf,
    package_name: String,
}

#[derive(Debug, Clone)]
struct ModuleSymbols {
    module_id: String,
    functions: HashMap<String, String>,
    public_functions: HashMap<String, String>,
    private_functions: HashSet<String>,
    aliases: HashMap<String, String>,
    public_aliases: HashMap<String, String>,
    private_aliases: HashSet<String>,
    structs: HashMap<String, String>,
    public_structs: HashMap<String, String>,
    private_structs: HashSet<String>,
    enums: HashMap<String, String>,
    public_enums: HashMap<String, String>,
    private_enums: HashSet<String>,
}

fn analyze_package(
    graph: &PackageGraph,
    package_root: &Path,
) -> Result<AnalyzedProject, Diagnostic> {
    let package_root = normalize_path(package_root);
    let manifest = graph.context(&package_root)?.manifest.clone();
    if manifest.is_workspace_only() {
        return Err(Diagnostic::new(
            "manifest",
            format!(
                "workspace-only manifest at {} is not directly buildable",
                manifest_path(&package_root).display()
            ),
        )
        .with_path(manifest_path(&package_root).display().to_string()));
    }
    validate_lockfile(&package_root, &manifest)?;
    let entry = entry_path(&package_root, &manifest);
    analyze_entry(graph, &package_root, manifest, entry)
}

fn analyze_entry(
    graph: &PackageGraph,
    package_root: &Path,
    manifest: Manifest,
    entry: PathBuf,
) -> Result<AnalyzedProject, Diagnostic> {
    let modules = load_modules(graph, package_root, &entry)?;
    validate_module_capabilities(graph, &modules)?;
    let flattened = flatten_modules(graph, &modules)?;
    let hir = hir::lower_with_capabilities(&flattened, &manifest.capabilities)?;
    let mir = mir::lower(&hir);
    Ok(AnalyzedProject {
        manifest,
        entry_path: entry,
        mir,
    })
}

fn validate_workspace_root_lockfile(
    graph: &PackageGraph,
    project_root: &Path,
) -> Result<(), Diagnostic> {
    let manifest = graph.context(project_root)?.manifest.clone();
    if manifest.is_workspace_only() {
        validate_lockfile(project_root, &manifest)?;
    }
    Ok(())
}

fn load_package_graph(project_root: &Path) -> Result<PackageGraph, Diagnostic> {
    let mut graph = PackageGraph::default();
    let mut visiting = Vec::new();
    load_package_graph_recursive(project_root, &mut graph, &mut visiting)?;
    register_stdlib_package(&mut graph);
    Ok(graph)
}

/// Registers the synthetic `<stdlib>` package in the graph. The synthetic
/// manifest enables every capability so `validate_module_capabilities` does not
/// reject stdlib wrappers against their own package config; actual capability
/// enforcement still runs on the flattened program via
/// `hir::lower_with_capabilities`, which uses the **entry** package's
/// capabilities. That keeps stdlib wrappers transparent for capability rules:
/// an import of `std/time.ax` does not grant clock access unless the importing
/// package's manifest also declares `[capabilities] clock = true`.
fn register_stdlib_package(graph: &mut PackageGraph) {
    let root = stdlib::stdlib_root();
    if graph.packages.contains_key(&root) {
        return;
    }
    let manifest = Manifest {
        package: Some(PackageSection {
            name: stdlib::STDLIB_PACKAGE_NAME.to_string(),
            version: stdlib::STDLIB_PACKAGE_VERSION.to_string(),
        }),
        dependencies: BTreeMap::new(),
        workspace: None,
        build: BuildSection {
            entry: String::from("lib.ax"),
            out_dir: String::from("dist"),
        },
        tests: Vec::new(),
        capabilities: CapabilityConfig {
            fs: true,
            net: true,
            process: true,
            env: true,
            clock: true,
            crypto: true,
        },
    };
    graph.packages.insert(
        root.clone(),
        PackageContext {
            root: root.clone(),
            manifest,
            source_root: root,
            dependencies: BTreeMap::new(),
            workspace_members: Vec::new(),
        },
    );
}

fn load_package_graph_recursive(
    project_root: &Path,
    graph: &mut PackageGraph,
    visiting: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    let project_root = normalize_path(project_root);
    if graph.packages.contains_key(&project_root) {
        return Ok(());
    }
    if visiting.contains(&project_root) {
        return Err(Diagnostic::new(
            "manifest",
            format!("dependency cycle detected at {}", project_root.display()),
        )
        .with_path(manifest_path(&project_root).display().to_string()));
    }
    let manifest = load_manifest(&project_root)?;
    let workspace_members = resolve_workspace_members(&project_root, &manifest)?;
    let source_root = if manifest.package.is_some() {
        entry_path(&project_root, &manifest)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| project_root.clone())
    } else {
        let src_root = project_root.join("src");
        if src_root.exists() {
            src_root
        } else {
            project_root.clone()
        }
    };
    visiting.push(project_root.clone());
    let mut dependencies = BTreeMap::new();
    for (name, spec) in &manifest.dependencies {
        let dependency_root = normalize_path(project_root.join(&spec.path));
        if !dependency_root.exists() {
            return Err(Diagnostic::new(
                "manifest",
                format!(
                    "dependency {name:?} is missing at {}",
                    dependency_root.display()
                ),
            )
            .with_path(manifest_path(&project_root).display().to_string()));
        }
        load_package_graph_recursive(&dependency_root, graph, visiting)?;
        dependencies.insert(name.clone(), dependency_root);
    }
    for member_root in &workspace_members {
        load_package_graph_recursive(member_root, graph, visiting)?;
    }
    visiting.pop();
    graph.packages.insert(
        project_root.clone(),
        PackageContext {
            root: project_root,
            manifest,
            source_root,
            dependencies,
            workspace_members,
        },
    );
    Ok(())
}

fn resolve_workspace_members(
    project_root: &Path,
    manifest: &Manifest,
) -> Result<Vec<PathBuf>, Diagnostic> {
    let mut members = Vec::new();
    let mut seen = HashSet::new();
    for (index, member) in manifest
        .workspace
        .as_ref()
        .into_iter()
        .flat_map(|workspace| workspace.members.iter())
        .enumerate()
    {
        if member.trim().is_empty() {
            return Err(
                Diagnostic::new("manifest", "workspace member paths must not be empty")
                    .with_path(manifest_path(project_root).display().to_string()),
            );
        }
        let candidate = Path::new(member);
        if candidate.is_absolute() {
            return Err(Diagnostic::new(
                "manifest",
                format!("workspace.members[{index}] must be relative"),
            )
            .with_path(manifest_path(project_root).display().to_string()));
        }
        if candidate
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            return Err(Diagnostic::new(
                "manifest",
                format!("workspace.members[{index}] must not use parent traversal"),
            )
            .with_path(manifest_path(project_root).display().to_string()));
        }
        let member_root = normalize_path(project_root.join(member));
        if member_root == project_root {
            return Err(Diagnostic::new(
                "manifest",
                "workspace members must not include the root package",
            )
            .with_path(manifest_path(project_root).display().to_string()));
        }
        if !member_root.exists() {
            return Err(Diagnostic::new(
                "manifest",
                format!(
                    "workspace member {member:?} is missing at {}",
                    member_root.display()
                ),
            )
            .with_path(manifest_path(project_root).display().to_string()));
        }
        let member_manifest = manifest_path(&member_root);
        if !member_manifest.exists() {
            return Err(Diagnostic::new(
                "manifest",
                format!("workspace member {member:?} is missing axiom.toml"),
            )
            .with_path(manifest_path(project_root).display().to_string()));
        }
        if seen.insert(member_root.clone()) {
            members.push(member_root);
        }
    }
    Ok(members)
}

fn build_artifacts(
    analyzed: &AnalyzedProject,
    generated_rust: &Path,
    binary: &Path,
    options: &BuildOptions,
) -> Result<(), Diagnostic> {
    if let Some(parent) = generated_rust.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            Diagnostic::new(
                "build",
                format!("failed to create {}: {err}", parent.display()),
            )
        })?;
    }
    if let Some(parent) = binary.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            Diagnostic::new(
                "build",
                format!("failed to create {}: {err}", parent.display()),
            )
        })?;
    }
    let rust_source = render_rust(&analyzed.mir);
    fs::write(generated_rust, rust_source).map_err(|err| {
        Diagnostic::new(
            "build",
            format!("failed to write {}: {err}", generated_rust.display()),
        )
    })?;
    compile_native(generated_rust, binary, options.target.as_deref())
}

fn run_test_case(
    project_root: &Path,
    graph: &PackageGraph,
    manifest: &Manifest,
    test: &crate::manifest::TestTarget,
) -> TestCaseResult {
    let started = Instant::now();
    let entry_path = project_root.join(&test.entry);
    let generated_rust = test_generated_rust_path(project_root, manifest, &test.name);
    let binary = test_binary_path(project_root, manifest, &test.name);
    let analyzed = match analyze_entry(graph, project_root, manifest.clone(), entry_path.clone()) {
        Ok(analyzed) => analyzed,
        Err(error) => {
            return TestCaseResult {
                package_root: project_root.display().to_string(),
                name: test.name.clone(),
                entry: test.entry.clone(),
                ok: false,
                binary: None,
                generated_rust: None,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                expected_stdout: test.stdout.clone(),
                duration_ms: started.elapsed().as_millis() as u64,
                error: Some(error),
            };
        }
    };
    if let Err(error) = build_artifacts(
        &analyzed,
        &generated_rust,
        &binary,
        &BuildOptions::default(),
    ) {
        return TestCaseResult {
            package_root: project_root.display().to_string(),
            name: test.name.clone(),
            entry: test.entry.clone(),
            ok: false,
            binary: Some(binary.display().to_string()),
            generated_rust: Some(generated_rust.display().to_string()),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            expected_stdout: test.stdout.clone(),
            duration_ms: started.elapsed().as_millis() as u64,
            error: Some(error),
        };
    }

    match Command::new(&binary).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code();
            let error = if !output.status.success() {
                Some(
                    Diagnostic::new(
                        "test",
                        format!(
                            "test {:?} exited with status {}",
                            test.name,
                            exit_code.unwrap_or(1)
                        ),
                    )
                    .with_path(entry_path.display().to_string()),
                )
            } else if let Some(expected_stdout) = &test.stdout {
                if &stdout != expected_stdout {
                    Some(
                        Diagnostic::new(
                            "test",
                            format!("test {:?} stdout did not match expected output", test.name),
                        )
                        .with_path(entry_path.display().to_string()),
                    )
                } else {
                    None
                }
            } else {
                None
            };
            TestCaseResult {
                package_root: project_root.display().to_string(),
                name: test.name.clone(),
                entry: test.entry.clone(),
                ok: error.is_none(),
                binary: Some(binary.display().to_string()),
                generated_rust: Some(generated_rust.display().to_string()),
                exit_code,
                stdout,
                stderr,
                expected_stdout: test.stdout.clone(),
                duration_ms: started.elapsed().as_millis() as u64,
                error,
            }
        }
        Err(err) => TestCaseResult {
            package_root: project_root.display().to_string(),
            name: test.name.clone(),
            entry: test.entry.clone(),
            ok: false,
            binary: Some(binary.display().to_string()),
            generated_rust: Some(generated_rust.display().to_string()),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            expected_stdout: test.stdout.clone(),
            duration_ms: started.elapsed().as_millis() as u64,
            error: Some(
                Diagnostic::new(
                    "test",
                    format!("failed to execute {}: {err}", binary.display()),
                )
                .with_path(entry_path.display().to_string()),
            ),
        },
    }
}

fn workspace_package_roots(
    graph: &PackageGraph,
    project_root: &Path,
    selected_package: Option<&str>,
) -> Result<Vec<PathBuf>, Diagnostic> {
    let mut roots = Vec::new();
    let mut seen = BTreeSet::new();
    collect_workspace_package_roots(graph, project_root, &mut seen, &mut roots)?;
    if let Some(selected_package) = selected_package {
        let matched = roots
            .into_iter()
            .filter(|root| {
                graph
                    .context(root)
                    .ok()
                    .and_then(|package| package.manifest.package.as_ref())
                    .map(|package| package.name == selected_package)
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        if matched.is_empty() {
            return Err(Diagnostic::new(
                "manifest",
                format!("workspace package {selected_package:?} was not found"),
            )
            .with_path(manifest_path(project_root).display().to_string()));
        }
        if matched.len() > 1 {
            return Err(Diagnostic::new(
                "manifest",
                format!("workspace package name {selected_package:?} is ambiguous"),
            )
            .with_path(manifest_path(project_root).display().to_string()));
        }
        return Ok(matched);
    }
    Ok(roots)
}

fn collect_workspace_package_roots(
    graph: &PackageGraph,
    package_root: &Path,
    seen: &mut BTreeSet<PathBuf>,
    roots: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    let package_root = normalize_path(package_root);
    if !seen.insert(package_root.clone()) {
        return Ok(());
    }
    let package = graph.context(&package_root)?;
    if package.manifest.package.is_some() {
        roots.push(package_root.clone());
    }
    for member in &package.workspace_members {
        collect_workspace_package_roots(graph, member, seen, roots)?;
    }
    Ok(())
}

fn test_generated_rust_path(project_root: &Path, manifest: &Manifest, test_name: &str) -> PathBuf {
    out_dir_path(project_root, manifest)
        .join("tests")
        .join(format!(
            "{}.generated.rs",
            test_artifact_name(manifest, test_name)
        ))
}

fn test_binary_path(project_root: &Path, manifest: &Manifest, test_name: &str) -> PathBuf {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    out_dir_path(project_root, manifest)
        .join("tests")
        .join(format!(
            "{}{}",
            test_artifact_name(manifest, test_name),
            suffix
        ))
}

fn test_artifact_name(manifest: &Manifest, test_name: &str) -> String {
    let package = manifest
        .package
        .as_ref()
        .expect("test artifacts require a package manifest");
    format!("{}-{}", package.name, slugify_test_name(test_name))
}

fn slugify_test_name(value: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        String::from("test")
    } else {
        trimmed.to_string()
    }
}

fn test_matches_filter(test: &crate::manifest::TestTarget, filter: &str) -> bool {
    test.name.contains(filter) || test.entry.contains(filter)
}

fn load_modules(
    graph: &PackageGraph,
    package_root: &Path,
    entry_path: &Path,
) -> Result<Vec<LoadedModule>, Diagnostic> {
    let mut ordered = Vec::new();
    let mut loaded = HashMap::new();
    let mut visiting = Vec::new();
    load_module_recursive(
        graph,
        package_root,
        entry_path,
        true,
        &mut ordered,
        &mut loaded,
        &mut visiting,
    )?;
    Ok(ordered)
}

fn load_module_recursive(
    graph: &PackageGraph,
    package_root: &Path,
    module_path: &Path,
    is_entry: bool,
    ordered: &mut Vec<LoadedModule>,
    loaded: &mut HashMap<PathBuf, ()>,
    visiting: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    let module_path = normalize_path(module_path);
    if visiting.contains(&module_path) {
        return Err(Diagnostic::new(
            "import",
            format!("circular import detected at {}", module_path.display()),
        )
        .with_path(module_path.display().to_string()));
    }
    if loaded.contains_key(&module_path) {
        return Ok(());
    }
    let package = graph.context(package_root)?;

    let source = if stdlib::is_stdlib_path(&module_path) {
        stdlib::stdlib_source_for(&module_path)
            .map(str::to_string)
            .ok_or_else(|| {
                Diagnostic::new(
                    "source",
                    format!(
                        "internal error: missing stdlib source for {}",
                        module_path.display()
                    ),
                )
                .with_path(module_path.display().to_string())
            })?
    } else {
        fs::read_to_string(&module_path).map_err(|err| {
            Diagnostic::new(
                "source",
                format!("failed to read {}: {err}", module_path.display()),
            )
            .with_path(module_path.display().to_string())
        })?
    };
    let program = syntax::parse_program(&source, &module_path)?;
    if !is_entry && !program.stmts.is_empty() {
        let stmt = &program.stmts[0];
        return Err(Diagnostic::new(
            "import",
            "imported stage1 modules may only contain imports, type alias declarations, struct declarations, enum declarations, and function declarations",
        )
        .with_path(module_path.display().to_string())
        .with_span(stmt_line(stmt), stmt_column(stmt)));
    }

    visiting.push(module_path.clone());
    for import in &program.imports {
        let (import_package_root, import_path) =
            resolve_import_path(graph, package_root, &module_path, import)?;
        load_module_recursive(
            graph,
            &import_package_root,
            &import_path,
            false,
            ordered,
            loaded,
            visiting,
        )?;
    }
    visiting.pop();

    loaded.insert(module_path.clone(), ());
    let package_name = package
        .manifest
        .package
        .as_ref()
        .expect("loaded modules require a package manifest")
        .name
        .clone();
    ordered.push(LoadedModule {
        path: module_path,
        program,
        is_entry,
        package_root: package.root.clone(),
        source_root: package.source_root.clone(),
        package_name,
    });
    Ok(())
}

fn validate_module_capabilities(
    graph: &PackageGraph,
    modules: &[LoadedModule],
) -> Result<(), Diagnostic> {
    for module in modules {
        let package = graph.context(&module.package_root)?;
        validate_program_capabilities(
            &module.path,
            &module.program,
            &package.manifest.capabilities,
        )?;
    }
    Ok(())
}

fn validate_program_capabilities(
    module_path: &Path,
    program: &syntax::Program,
    capabilities: &CapabilityConfig,
) -> Result<(), Diagnostic> {
    for function in &program.functions {
        for stmt in &function.body {
            validate_stmt_capabilities(module_path, stmt, capabilities)?;
        }
    }
    for stmt in &program.stmts {
        validate_stmt_capabilities(module_path, stmt, capabilities)?;
    }
    Ok(())
}

fn validate_stmt_capabilities(
    module_path: &Path,
    stmt: &syntax::Stmt,
    capabilities: &CapabilityConfig,
) -> Result<(), Diagnostic> {
    match stmt {
        syntax::Stmt::Let { expr, .. }
        | syntax::Stmt::Print { expr, .. }
        | syntax::Stmt::Return { expr, .. } => {
            validate_expr_capabilities(module_path, expr, capabilities)?;
        }
        syntax::Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            validate_expr_capabilities(module_path, cond, capabilities)?;
            for stmt in then_block {
                validate_stmt_capabilities(module_path, stmt, capabilities)?;
            }
            if let Some(else_block) = else_block {
                for stmt in else_block {
                    validate_stmt_capabilities(module_path, stmt, capabilities)?;
                }
            }
        }
        syntax::Stmt::While { cond, body, .. } => {
            validate_expr_capabilities(module_path, cond, capabilities)?;
            for stmt in body {
                validate_stmt_capabilities(module_path, stmt, capabilities)?;
            }
        }
        syntax::Stmt::Match { expr, arms, .. } => {
            validate_expr_capabilities(module_path, expr, capabilities)?;
            for arm in arms {
                for stmt in &arm.body {
                    validate_stmt_capabilities(module_path, stmt, capabilities)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_expr_capabilities(
    module_path: &Path,
    expr: &syntax::Expr,
    capabilities: &CapabilityConfig,
) -> Result<(), Diagnostic> {
    match expr {
        syntax::Expr::Literal(_) | syntax::Expr::VarRef { .. } => Ok(()),
        syntax::Expr::Call {
            name,
            args,
            line,
            column,
        } => {
            if let Some(kind) = intrinsic_capability(name)
                && !capabilities.enabled(kind)
            {
                return Err(Diagnostic::new(
                    "capability",
                    format!(
                        "call to {name:?} requires [capabilities].{} = true",
                        kind.name()
                    ),
                )
                .with_path(module_path.display().to_string())
                .with_span(*line, *column));
            }
            for arg in args {
                validate_expr_capabilities(module_path, arg, capabilities)?;
            }
            Ok(())
        }
        syntax::Expr::BinaryAdd { lhs, rhs, .. } | syntax::Expr::BinaryCompare { lhs, rhs, .. } => {
            validate_expr_capabilities(module_path, lhs, capabilities)?;
            validate_expr_capabilities(module_path, rhs, capabilities)
        }
        syntax::Expr::StructLiteral { fields, .. } => {
            for field in fields {
                validate_expr_capabilities(module_path, &field.expr, capabilities)?;
            }
            Ok(())
        }
        syntax::Expr::FieldAccess { base, .. } | syntax::Expr::TupleIndex { base, .. } => {
            validate_expr_capabilities(module_path, base, capabilities)
        }
        syntax::Expr::TupleLiteral { elements, .. }
        | syntax::Expr::ArrayLiteral { elements, .. } => {
            for element in elements {
                validate_expr_capabilities(module_path, element, capabilities)?;
            }
            Ok(())
        }
        syntax::Expr::MapLiteral { entries, .. } => {
            for entry in entries {
                validate_expr_capabilities(module_path, &entry.key, capabilities)?;
                validate_expr_capabilities(module_path, &entry.value, capabilities)?;
            }
            Ok(())
        }
        syntax::Expr::Slice {
            base, start, end, ..
        } => {
            validate_expr_capabilities(module_path, base, capabilities)?;
            if let Some(start) = start {
                validate_expr_capabilities(module_path, start, capabilities)?;
            }
            if let Some(end) = end {
                validate_expr_capabilities(module_path, end, capabilities)?;
            }
            Ok(())
        }
        syntax::Expr::Index { base, index, .. } => {
            validate_expr_capabilities(module_path, base, capabilities)?;
            validate_expr_capabilities(module_path, index, capabilities)
        }
    }
}

fn intrinsic_capability(name: &str) -> Option<CapabilityKind> {
    match name {
        "fs_read" => Some(CapabilityKind::Fs),
        "net_resolve" => Some(CapabilityKind::Net),
        "http_get" => Some(CapabilityKind::Net),
        "process_status" => Some(CapabilityKind::Process),
        "clock_now_ms" => Some(CapabilityKind::Clock),
        "env_get" => Some(CapabilityKind::Env),
        "crypto_sha256" => Some(CapabilityKind::Crypto),
        _ => None,
    }
}

fn flatten_modules(
    graph: &PackageGraph,
    modules: &[LoadedModule],
) -> Result<syntax::Program, Diagnostic> {
    let mut symbols = HashMap::new();
    for module in modules {
        symbols.insert(module.path.clone(), build_module_symbols(module)?);
    }

    let mut flattened_functions = Vec::new();
    let mut flattened_type_aliases = Vec::new();
    let mut flattened_structs = Vec::new();
    let mut flattened_enums = Vec::new();
    let mut flattened_stmts = Vec::new();
    for module in modules {
        let Some(module_symbols) = symbols.get(&module.path) else {
            continue;
        };
        let mut visible_functions = module_symbols.functions.clone();
        let mut visible_aliases = module_symbols.aliases.clone();
        let mut visible_structs = module_symbols.structs.clone();
        let mut visible_enums = module_symbols.enums.clone();
        let mut private_imported = HashSet::new();
        let mut private_imported_types = HashSet::new();
        for import in &module.program.imports {
            let (_, import_path) =
                resolve_import_path(graph, &module.package_root, &module.path, import)?;
            let imported_symbols = symbols.get(&import_path).ok_or_else(|| {
                Diagnostic::new(
                    "import",
                    format!("internal error: missing module {}", import_path.display()),
                )
            })?;
            for name in &imported_symbols.private_functions {
                private_imported.insert(name.clone());
            }
            for (export_name, internal_name) in &imported_symbols.public_functions {
                if let Some(existing) = visible_functions.get(export_name)
                    && existing != internal_name
                {
                    return Err(Diagnostic::new(
                        "import",
                        format!("imported function {export_name:?} collides with an existing name"),
                    )
                    .with_path(module.path.display().to_string())
                    .with_span(import.line, import.column));
                }
                visible_functions.insert(export_name.clone(), internal_name.clone());
            }
            for name in &imported_symbols.private_structs {
                private_imported_types.insert(name.clone());
            }
            for name in &imported_symbols.private_enums {
                private_imported_types.insert(name.clone());
            }
            for name in &imported_symbols.private_aliases {
                private_imported_types.insert(name.clone());
            }
            for (export_name, internal_name) in &imported_symbols.public_aliases {
                if let Some(existing) = visible_aliases.get(export_name)
                    && existing != internal_name
                {
                    return Err(Diagnostic::new(
                        "import",
                        format!(
                            "imported type alias {export_name:?} collides with an existing name"
                        ),
                    )
                    .with_path(module.path.display().to_string())
                    .with_span(import.line, import.column));
                }
                visible_aliases.insert(export_name.clone(), internal_name.clone());
            }
            for (export_name, internal_name) in &imported_symbols.public_structs {
                if let Some(existing) = visible_structs.get(export_name)
                    && existing != internal_name
                {
                    return Err(Diagnostic::new(
                        "import",
                        format!("imported struct {export_name:?} collides with an existing name"),
                    )
                    .with_path(module.path.display().to_string())
                    .with_span(import.line, import.column));
                }
                visible_structs.insert(export_name.clone(), internal_name.clone());
            }
            for (export_name, internal_name) in &imported_symbols.public_enums {
                if let Some(existing) = visible_enums.get(export_name)
                    && existing != internal_name
                {
                    return Err(Diagnostic::new(
                        "import",
                        format!("imported enum {export_name:?} collides with an existing name"),
                    )
                    .with_path(module.path.display().to_string())
                    .with_span(import.line, import.column));
                }
                visible_enums.insert(export_name.clone(), internal_name.clone());
            }
        }

        let visible_types = merge_visible_types(
            &visible_aliases,
            &visible_structs,
            &visible_enums,
            &module.path,
        )?;

        for type_alias in &module.program.type_aliases {
            flattened_type_aliases.push(rewrite_type_alias(
                type_alias,
                module_symbols,
                &visible_types,
                &private_imported_types,
                &module.path,
            )?);
        }

        for struct_decl in &module.program.structs {
            flattened_structs.push(rewrite_struct(
                struct_decl,
                module_symbols,
                &visible_types,
                &private_imported_types,
                &module.path,
            )?);
        }
        for enum_decl in &module.program.enums {
            flattened_enums.push(rewrite_enum(
                enum_decl,
                module_symbols,
                &visible_types,
                &private_imported_types,
                &module.path,
            )?);
        }
        for function in &module.program.functions {
            flattened_functions.push(rewrite_function(
                function,
                module_symbols,
                &visible_functions,
                &visible_structs,
                &visible_types,
                &private_imported,
                &private_imported_types,
                &module.path,
            )?);
        }
        if module.is_entry {
            for stmt in &module.program.stmts {
                flattened_stmts.push(rewrite_stmt(
                    stmt,
                    &visible_functions,
                    &visible_structs,
                    &visible_types,
                    &private_imported,
                    &private_imported_types,
                    &module.path,
                )?);
            }
        }
    }

    Ok(syntax::Program {
        path: modules
            .iter()
            .find(|module| module.is_entry)
            .map(|module| module.path.display().to_string())
            .unwrap_or_default(),
        imports: Vec::new(),
        type_aliases: flattened_type_aliases,
        structs: flattened_structs,
        enums: flattened_enums,
        functions: flattened_functions,
        stmts: flattened_stmts,
    })
}

fn build_module_symbols(module: &LoadedModule) -> Result<ModuleSymbols, Diagnostic> {
    let module_id = module_id_for_path(&module.path, &module.source_root, &module.package_name);
    let mut functions = HashMap::new();
    let mut public_functions = HashMap::new();
    let mut private_functions = HashSet::new();
    let mut aliases = HashMap::new();
    let mut public_aliases = HashMap::new();
    let mut private_aliases = HashSet::new();
    let mut structs = HashMap::new();
    let mut public_structs = HashMap::new();
    let mut private_structs = HashSet::new();
    let mut enums = HashMap::new();
    let mut public_enums = HashMap::new();
    let mut private_enums = HashSet::new();
    for struct_decl in &module.program.structs {
        let internal_name = format!("{module_id}_{}", struct_decl.name);
        if structs
            .insert(struct_decl.name.clone(), internal_name.clone())
            .is_some()
        {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate struct {:?}", struct_decl.name),
            )
            .with_path(module.path.display().to_string())
            .with_span(struct_decl.line, struct_decl.column));
        }
        if struct_decl.is_public {
            public_structs.insert(struct_decl.name.clone(), internal_name);
        } else {
            private_structs.insert(struct_decl.name.clone());
        }
    }
    for enum_decl in &module.program.enums {
        if structs.contains_key(&enum_decl.name) {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate type name {:?}", enum_decl.name),
            )
            .with_path(module.path.display().to_string())
            .with_span(enum_decl.line, enum_decl.column));
        }
        let internal_name = format!("{module_id}_{}", enum_decl.name);
        if enums
            .insert(enum_decl.name.clone(), internal_name.clone())
            .is_some()
        {
            return Err(
                Diagnostic::new("type", format!("duplicate enum {:?}", enum_decl.name))
                    .with_path(module.path.display().to_string())
                    .with_span(enum_decl.line, enum_decl.column),
            );
        }
        if enum_decl.is_public {
            public_enums.insert(enum_decl.name.clone(), internal_name);
        } else {
            private_enums.insert(enum_decl.name.clone());
        }
    }
    for function in &module.program.functions {
        let internal_name = format!("{module_id}_{}", function.name);
        if functions
            .insert(function.name.clone(), internal_name.clone())
            .is_some()
        {
            return Err(
                Diagnostic::new("type", format!("duplicate function {:?}", function.name))
                    .with_path(module.path.display().to_string())
                    .with_span(function.line, function.column),
            );
        }
        if function.is_public {
            public_functions.insert(function.name.clone(), internal_name);
        } else {
            private_functions.insert(function.name.clone());
        }
    }
    for type_alias in &module.program.type_aliases {
        if structs.contains_key(&type_alias.name) || enums.contains_key(&type_alias.name) {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate type name {:?}", type_alias.name),
            )
            .with_path(module.path.display().to_string())
            .with_span(type_alias.line, type_alias.column));
        }
        let internal_name = format!("{module_id}_{}", type_alias.name);
        if aliases
            .insert(type_alias.name.clone(), internal_name.clone())
            .is_some()
        {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate type alias {:?}", type_alias.name),
            )
            .with_path(module.path.display().to_string())
            .with_span(type_alias.line, type_alias.column));
        }
        if type_alias.is_public {
            public_aliases.insert(type_alias.name.clone(), internal_name);
        } else {
            private_aliases.insert(type_alias.name.clone());
        }
    }
    Ok(ModuleSymbols {
        module_id,
        functions,
        public_functions,
        private_functions,
        aliases,
        public_aliases,
        private_aliases,
        structs,
        public_structs,
        private_structs,
        enums,
        public_enums,
        private_enums,
    })
}

fn rewrite_type_alias(
    type_alias: &syntax::TypeAliasDecl,
    module_symbols: &ModuleSymbols,
    visible_types: &HashMap<String, String>,
    private_imported_types: &HashSet<String>,
    module_path: &Path,
) -> Result<syntax::TypeAliasDecl, Diagnostic> {
    Ok(syntax::TypeAliasDecl {
        name: module_symbols
            .aliases
            .get(&type_alias.name)
            .cloned()
            .unwrap_or_else(|| format!("{}_{}", module_symbols.module_id, type_alias.name)),
        ty: rewrite_type_name(
            &type_alias.ty,
            visible_types,
            private_imported_types,
            module_path,
            type_alias.line,
            type_alias.column,
        )?,
        is_public: type_alias.is_public,
        line: type_alias.line,
        column: type_alias.column,
    })
}

fn rewrite_struct(
    struct_decl: &syntax::StructDecl,
    module_symbols: &ModuleSymbols,
    visible_types: &HashMap<String, String>,
    private_imported_types: &HashSet<String>,
    module_path: &Path,
) -> Result<syntax::StructDecl, Diagnostic> {
    let mut rewritten = struct_decl.clone();
    rewritten.name = module_symbols
        .structs
        .get(&struct_decl.name)
        .cloned()
        .unwrap_or_else(|| format!("{}_{}", module_symbols.module_id, struct_decl.name));
    rewritten.fields = struct_decl
        .fields
        .iter()
        .map(|field| {
            Ok(syntax::StructField {
                name: field.name.clone(),
                ty: rewrite_type_name(
                    &field.ty,
                    visible_types,
                    private_imported_types,
                    module_path,
                    field.line,
                    field.column,
                )?,
                line: field.line,
                column: field.column,
            })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    Ok(rewritten)
}

fn rewrite_enum(
    enum_decl: &syntax::EnumDecl,
    module_symbols: &ModuleSymbols,
    visible_types: &HashMap<String, String>,
    private_imported_types: &HashSet<String>,
    module_path: &Path,
) -> Result<syntax::EnumDecl, Diagnostic> {
    let mut rewritten = enum_decl.clone();
    rewritten.name = module_symbols
        .enums
        .get(&enum_decl.name)
        .cloned()
        .unwrap_or_else(|| format!("{}_{}", module_symbols.module_id, enum_decl.name));
    rewritten.variants = enum_decl
        .variants
        .iter()
        .map(|variant| {
            Ok(syntax::EnumVariantDecl {
                name: variant.name.clone(),
                payload_tys: variant
                    .payload_tys
                    .iter()
                    .map(|ty| {
                        rewrite_type_name(
                            ty,
                            visible_types,
                            private_imported_types,
                            module_path,
                            variant.line,
                            variant.column,
                        )
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
                payload_names: variant.payload_names.clone(),
                line: variant.line,
                column: variant.column,
            })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    Ok(rewritten)
}

fn rewrite_function(
    function: &syntax::Function,
    module_symbols: &ModuleSymbols,
    visible_functions: &HashMap<String, String>,
    visible_structs: &HashMap<String, String>,
    visible_types: &HashMap<String, String>,
    private_imported: &HashSet<String>,
    private_imported_types: &HashSet<String>,
    module_path: &Path,
) -> Result<syntax::Function, Diagnostic> {
    let mut rewritten = function.clone();
    rewritten.name = module_symbols
        .functions
        .get(&function.name)
        .cloned()
        .unwrap_or_else(|| format!("{}_{}", module_symbols.module_id, function.name));
    rewritten.body = function
        .body
        .iter()
        .map(|stmt| {
            rewrite_stmt(
                stmt,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    rewritten.params = function
        .params
        .iter()
        .map(|param| {
            Ok(syntax::Param {
                name: param.name.clone(),
                ty: rewrite_type_name(
                    &param.ty,
                    visible_types,
                    private_imported_types,
                    module_path,
                    param.line,
                    param.column,
                )?,
                line: param.line,
                column: param.column,
            })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    rewritten.return_ty = rewrite_type_name(
        &function.return_ty,
        visible_types,
        private_imported_types,
        module_path,
        function.line,
        function.column,
    )?;
    Ok(rewritten)
}

fn rewrite_stmt(
    stmt: &syntax::Stmt,
    visible_functions: &HashMap<String, String>,
    visible_structs: &HashMap<String, String>,
    visible_types: &HashMap<String, String>,
    private_imported: &HashSet<String>,
    private_imported_types: &HashSet<String>,
    module_path: &Path,
) -> Result<syntax::Stmt, Diagnostic> {
    Ok(match stmt {
        syntax::Stmt::Let {
            name,
            ty,
            expr,
            line,
            column,
        } => syntax::Stmt::Let {
            name: name.clone(),
            ty: rewrite_type_name(
                ty,
                visible_types,
                private_imported_types,
                module_path,
                *line,
                *column,
            )?,
            expr: rewrite_expr(
                expr,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?,
            line: *line,
            column: *column,
        },
        syntax::Stmt::Print { expr, line, column } => syntax::Stmt::Print {
            expr: rewrite_expr(
                expr,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?,
            line: *line,
            column: *column,
        },
        syntax::Stmt::If {
            cond,
            then_block,
            else_block,
            line,
            column,
        } => syntax::Stmt::If {
            cond: rewrite_expr(
                cond,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?,
            then_block: then_block
                .iter()
                .map(|stmt| {
                    rewrite_stmt(
                        stmt,
                        visible_functions,
                        visible_structs,
                        visible_types,
                        private_imported,
                        private_imported_types,
                        module_path,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
            else_block: else_block
                .as_ref()
                .map(|block| {
                    block
                        .iter()
                        .map(|stmt| {
                            rewrite_stmt(
                                stmt,
                                visible_functions,
                                visible_structs,
                                visible_types,
                                private_imported,
                                private_imported_types,
                                module_path,
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?,
            line: *line,
            column: *column,
        },
        syntax::Stmt::While {
            cond,
            body,
            line,
            column,
        } => syntax::Stmt::While {
            cond: rewrite_expr(
                cond,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?,
            body: body
                .iter()
                .map(|stmt| {
                    rewrite_stmt(
                        stmt,
                        visible_functions,
                        visible_structs,
                        visible_types,
                        private_imported,
                        private_imported_types,
                        module_path,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
            line: *line,
            column: *column,
        },
        syntax::Stmt::Match {
            expr,
            arms,
            line,
            column,
        } => syntax::Stmt::Match {
            expr: rewrite_expr(
                expr,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?,
            arms: arms
                .iter()
                .map(|arm| {
                    Ok(syntax::MatchArm {
                        variant: arm.variant.clone(),
                        bindings: arm.bindings.clone(),
                        is_named: arm.is_named,
                        body: arm
                            .body
                            .iter()
                            .map(|stmt| {
                                rewrite_stmt(
                                    stmt,
                                    visible_functions,
                                    visible_structs,
                                    visible_types,
                                    private_imported,
                                    private_imported_types,
                                    module_path,
                                )
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                        line: arm.line,
                        column: arm.column,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
            line: *line,
            column: *column,
        },
        syntax::Stmt::Return { expr, line, column } => syntax::Stmt::Return {
            expr: rewrite_expr(
                expr,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?,
            line: *line,
            column: *column,
        },
    })
}

fn rewrite_expr(
    expr: &syntax::Expr,
    visible_functions: &HashMap<String, String>,
    visible_structs: &HashMap<String, String>,
    visible_types: &HashMap<String, String>,
    private_imported: &HashSet<String>,
    private_imported_types: &HashSet<String>,
    module_path: &Path,
) -> Result<syntax::Expr, Diagnostic> {
    Ok(match expr {
        syntax::Expr::Literal(_) | syntax::Expr::VarRef { .. } => expr.clone(),
        syntax::Expr::Call {
            name,
            args,
            line,
            column,
        } => {
            if !visible_functions.contains_key(name) && private_imported.contains(name) {
                return Err(Diagnostic::new(
                    "import",
                    format!("function {name:?} is not exported by an imported module"),
                )
                .with_path(module_path.display().to_string())
                .with_span(*line, *column));
            }
            syntax::Expr::Call {
                name: visible_functions
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
                args: args
                    .iter()
                    .map(|arg| {
                        rewrite_expr(
                            arg,
                            visible_functions,
                            visible_structs,
                            visible_types,
                            private_imported,
                            private_imported_types,
                            module_path,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                line: *line,
                column: *column,
            }
        }
        syntax::Expr::BinaryAdd {
            lhs,
            rhs,
            line,
            column,
        } => syntax::Expr::BinaryAdd {
            lhs: Box::new(rewrite_expr(
                lhs,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            rhs: Box::new(rewrite_expr(
                rhs,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            line: *line,
            column: *column,
        },
        syntax::Expr::BinaryCompare {
            op,
            lhs,
            rhs,
            line,
            column,
        } => syntax::Expr::BinaryCompare {
            op: *op,
            lhs: Box::new(rewrite_expr(
                lhs,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            rhs: Box::new(rewrite_expr(
                rhs,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            line: *line,
            column: *column,
        },
        syntax::Expr::StructLiteral {
            name,
            fields,
            line,
            column,
        } => {
            if !visible_structs.contains_key(name) && private_imported_types.contains(name) {
                return Err(Diagnostic::new(
                    "import",
                    format!("struct {name:?} is not exported by an imported module"),
                )
                .with_path(module_path.display().to_string())
                .with_span(*line, *column));
            }
            syntax::Expr::StructLiteral {
                name: visible_structs
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
                fields: fields
                    .iter()
                    .map(|field| {
                        Ok(syntax::StructFieldValue {
                            name: field.name.clone(),
                            expr: rewrite_expr(
                                &field.expr,
                                visible_functions,
                                visible_structs,
                                visible_types,
                                private_imported,
                                private_imported_types,
                                module_path,
                            )?,
                            line: field.line,
                            column: field.column,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
                line: *line,
                column: *column,
            }
        }
        syntax::Expr::FieldAccess {
            base,
            field,
            line,
            column,
        } => syntax::Expr::FieldAccess {
            base: Box::new(rewrite_expr(
                base,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            field: field.clone(),
            line: *line,
            column: *column,
        },
        syntax::Expr::TupleLiteral {
            elements,
            line,
            column,
        } => syntax::Expr::TupleLiteral {
            elements: elements
                .iter()
                .map(|element| {
                    rewrite_expr(
                        element,
                        visible_functions,
                        visible_structs,
                        visible_types,
                        private_imported,
                        private_imported_types,
                        module_path,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
            line: *line,
            column: *column,
        },
        syntax::Expr::TupleIndex {
            base,
            index,
            line,
            column,
        } => syntax::Expr::TupleIndex {
            base: Box::new(rewrite_expr(
                base,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            index: *index,
            line: *line,
            column: *column,
        },
        syntax::Expr::MapLiteral {
            entries,
            line,
            column,
        } => syntax::Expr::MapLiteral {
            entries: entries
                .iter()
                .map(|entry| {
                    Ok(syntax::MapEntry {
                        key: rewrite_expr(
                            &entry.key,
                            visible_functions,
                            visible_structs,
                            visible_types,
                            private_imported,
                            private_imported_types,
                            module_path,
                        )?,
                        value: rewrite_expr(
                            &entry.value,
                            visible_functions,
                            visible_structs,
                            visible_types,
                            private_imported,
                            private_imported_types,
                            module_path,
                        )?,
                        line: entry.line,
                        column: entry.column,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            line: *line,
            column: *column,
        },
        syntax::Expr::ArrayLiteral {
            elements,
            line,
            column,
        } => syntax::Expr::ArrayLiteral {
            elements: elements
                .iter()
                .map(|element| {
                    rewrite_expr(
                        element,
                        visible_functions,
                        visible_structs,
                        visible_types,
                        private_imported,
                        private_imported_types,
                        module_path,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
            line: *line,
            column: *column,
        },
        syntax::Expr::Slice {
            base,
            start,
            end,
            line,
            column,
        } => syntax::Expr::Slice {
            base: Box::new(rewrite_expr(
                base,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            start: start
                .as_ref()
                .map(|expr| {
                    rewrite_expr(
                        expr,
                        visible_functions,
                        visible_structs,
                        visible_types,
                        private_imported,
                        private_imported_types,
                        module_path,
                    )
                    .map(Box::new)
                })
                .transpose()?,
            end: end
                .as_ref()
                .map(|expr| {
                    rewrite_expr(
                        expr,
                        visible_functions,
                        visible_structs,
                        visible_types,
                        private_imported,
                        private_imported_types,
                        module_path,
                    )
                    .map(Box::new)
                })
                .transpose()?,
            line: *line,
            column: *column,
        },
        syntax::Expr::Index {
            base,
            index,
            line,
            column,
        } => syntax::Expr::Index {
            base: Box::new(rewrite_expr(
                base,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            index: Box::new(rewrite_expr(
                index,
                visible_functions,
                visible_structs,
                visible_types,
                private_imported,
                private_imported_types,
                module_path,
            )?),
            line: *line,
            column: *column,
        },
    })
}

fn rewrite_type_name(
    ty: &syntax::TypeName,
    visible_types: &HashMap<String, String>,
    private_imported_types: &HashSet<String>,
    module_path: &Path,
    line: usize,
    column: usize,
) -> Result<syntax::TypeName, Diagnostic> {
    match ty {
        syntax::TypeName::Int => Ok(syntax::TypeName::Int),
        syntax::TypeName::Bool => Ok(syntax::TypeName::Bool),
        syntax::TypeName::String => Ok(syntax::TypeName::String),
        syntax::TypeName::Named(name) => {
            if !visible_types.contains_key(name) && private_imported_types.contains(name) {
                return Err(Diagnostic::new(
                    "import",
                    format!("type {name:?} is not exported by an imported module"),
                )
                .with_path(module_path.display().to_string())
                .with_span(line, column));
            }
            Ok(syntax::TypeName::Named(
                visible_types
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
            ))
        }
        syntax::TypeName::Option(inner) => {
            Ok(syntax::TypeName::Option(Box::new(rewrite_type_name(
                inner,
                visible_types,
                private_imported_types,
                module_path,
                line,
                column,
            )?)))
        }
        syntax::TypeName::Slice(inner) => Ok(syntax::TypeName::Slice(Box::new(rewrite_type_name(
            inner,
            visible_types,
            private_imported_types,
            module_path,
            line,
            column,
        )?))),
        syntax::TypeName::MutSlice(inner) => {
            Ok(syntax::TypeName::MutSlice(Box::new(rewrite_type_name(
                inner,
                visible_types,
                private_imported_types,
                module_path,
                line,
                column,
            )?)))
        }
        syntax::TypeName::Result(ok, err) => Ok(syntax::TypeName::Result(
            Box::new(rewrite_type_name(
                ok,
                visible_types,
                private_imported_types,
                module_path,
                line,
                column,
            )?),
            Box::new(rewrite_type_name(
                err,
                visible_types,
                private_imported_types,
                module_path,
                line,
                column,
            )?),
        )),
        syntax::TypeName::Tuple(elements) => Ok(syntax::TypeName::Tuple(
            elements
                .iter()
                .map(|element| {
                    rewrite_type_name(
                        element,
                        visible_types,
                        private_imported_types,
                        module_path,
                        line,
                        column,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        syntax::TypeName::Map(key, value) => Ok(syntax::TypeName::Map(
            Box::new(rewrite_type_name(
                key,
                visible_types,
                private_imported_types,
                module_path,
                line,
                column,
            )?),
            Box::new(rewrite_type_name(
                value,
                visible_types,
                private_imported_types,
                module_path,
                line,
                column,
            )?),
        )),
        syntax::TypeName::Array(inner) => Ok(syntax::TypeName::Array(Box::new(rewrite_type_name(
            inner,
            visible_types,
            private_imported_types,
            module_path,
            line,
            column,
        )?))),
    }
}

fn merge_visible_types(
    visible_aliases: &HashMap<String, String>,
    visible_structs: &HashMap<String, String>,
    visible_enums: &HashMap<String, String>,
    module_path: &Path,
) -> Result<HashMap<String, String>, Diagnostic> {
    let mut visible_types = visible_aliases.clone();
    for (name, internal_name) in visible_structs {
        if let Some(existing) = visible_types.get(name)
            && existing != internal_name
        {
            return Err(Diagnostic::new(
                "import",
                format!("imported type {name:?} collides with an existing name"),
            )
            .with_path(module_path.display().to_string()));
        }
        visible_types.insert(name.clone(), internal_name.clone());
    }
    for (name, internal_name) in visible_enums {
        if let Some(existing) = visible_types.get(name)
            && existing != internal_name
        {
            return Err(Diagnostic::new(
                "import",
                format!("imported type {name:?} collides with an existing name"),
            )
            .with_path(module_path.display().to_string()));
        }
        visible_types.insert(name.clone(), internal_name.clone());
    }
    Ok(visible_types)
}

fn resolve_import_path(
    graph: &PackageGraph,
    package_root: &Path,
    module_path: &Path,
    import: &syntax::Import,
) -> Result<(PathBuf, PathBuf), Diagnostic> {
    let package = graph.context(package_root)?;
    let relative = Path::new(&import.path);
    if relative.is_absolute() {
        return Err(Diagnostic::new("import", "stage1 imports must be relative")
            .with_path(module_path.display().to_string())
            .with_span(import.line, import.column));
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(Diagnostic::new(
            "import",
            "stage1 imports may not traverse parent directories",
        )
        .with_path(module_path.display().to_string())
        .with_span(import.line, import.column));
    }
    let mut components = relative.components();
    if let Some(Component::Normal(first)) = components.next() {
        let first_name = first.to_string_lossy().to_string();
        if first_name == stdlib::STDLIB_IMPORT_PREFIX {
            let mut remainder = PathBuf::new();
            for component in components {
                remainder.push(component.as_os_str());
            }
            if remainder.as_os_str().is_empty() {
                return Err(Diagnostic::new(
                    "import",
                    "stdlib import must include a module path (e.g. import \"std/time.ax\")",
                )
                .with_path(module_path.display().to_string())
                .with_span(import.line, import.column));
            }
            if !stdlib::stdlib_has_module(&remainder) {
                return Err(Diagnostic::new(
                    "import",
                    format!("unknown stdlib module {:?}", import.path),
                )
                .with_path(module_path.display().to_string())
                .with_span(import.line, import.column));
            }
            let virtual_path = stdlib::stdlib_source_path(&remainder.to_string_lossy());
            return Ok((stdlib::stdlib_root(), virtual_path));
        }
        let dependency_name = first_name;
        if let Some(dependency_root) = package.dependencies.get(&dependency_name) {
            let dependency = graph.context(dependency_root)?;
            let mut remainder = PathBuf::new();
            for component in components {
                remainder.push(component.as_os_str());
            }
            if remainder.as_os_str().is_empty() {
                return Err(Diagnostic::new(
                    "import",
                    format!("dependency import {dependency_name:?} must include a module path"),
                )
                .with_path(module_path.display().to_string())
                .with_span(import.line, import.column));
            }
            let candidate = normalize_path(dependency.source_root.join(remainder));
            if !candidate.starts_with(&dependency.source_root) {
                return Err(Diagnostic::new(
                    "import",
                    "dependency imports must stay inside the package",
                )
                .with_path(module_path.display().to_string())
                .with_span(import.line, import.column));
            }
            if !candidate.exists() {
                return Err(Diagnostic::new(
                    "import",
                    format!("missing import {}", candidate.display()),
                )
                .with_path(module_path.display().to_string())
                .with_span(import.line, import.column));
            }
            return Ok((dependency.root.clone(), candidate));
        }
    }
    let base_dir = module_path.parent().unwrap_or(&package.source_root);
    let candidate = normalize_path(base_dir.join(relative));
    if !candidate.starts_with(&package.root) {
        return Err(
            Diagnostic::new("import", "stage1 imports must stay inside the package")
                .with_path(module_path.display().to_string())
                .with_span(import.line, import.column),
        );
    }
    if !candidate.exists() {
        return Err(
            Diagnostic::new("import", format!("missing import {}", candidate.display()))
                .with_path(module_path.display().to_string())
                .with_span(import.line, import.column),
        );
    }
    Ok((package.root.clone(), candidate))
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn module_id_for_path(path: &Path, source_root: &Path, package_name: &str) -> String {
    let relative = path.strip_prefix(source_root).unwrap_or(path);
    let stem = relative.with_extension("");
    let mut out = slug_identifier(package_name);
    for component in stem.components() {
        let component = component.as_os_str().to_string_lossy();
        if !out.is_empty() {
            out.push('_');
        }
        for ch in component.chars() {
            if ch.is_ascii_alphanumeric() {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('_');
            }
        }
    }
    if out.is_empty() {
        String::from("module")
    } else {
        out
    }
}

fn slug_identifier(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        String::from("package")
    } else {
        out
    }
}

fn stmt_line(stmt: &syntax::Stmt) -> usize {
    match stmt {
        syntax::Stmt::Let { line, .. }
        | syntax::Stmt::Print { line, .. }
        | syntax::Stmt::If { line, .. }
        | syntax::Stmt::While { line, .. }
        | syntax::Stmt::Match { line, .. }
        | syntax::Stmt::Return { line, .. } => *line,
    }
}

fn stmt_column(stmt: &syntax::Stmt) -> usize {
    match stmt {
        syntax::Stmt::Let { column, .. }
        | syntax::Stmt::Print { column, .. }
        | syntax::Stmt::If { column, .. }
        | syntax::Stmt::While { column, .. }
        | syntax::Stmt::Match { column, .. }
        | syntax::Stmt::Return { column, .. } => *column,
    }
}
