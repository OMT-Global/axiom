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
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
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

#[derive(Debug, Clone)]
struct LoadedModule {
    path: PathBuf,
    program: syntax::Program,
    is_entry: bool,
}

#[derive(Debug, Clone)]
struct ModuleSymbols {
    module_id: String,
    functions: HashMap<String, String>,
    public_functions: HashMap<String, String>,
    private_functions: HashSet<String>,
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
    let source_root = entry
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_root.to_path_buf());
    let modules = load_modules(project_root, &entry)?;
    let flattened = flatten_modules(&modules, &source_root)?;
    let hir = hir::lower(&flattened)?;
    let mir = mir::lower(&hir);
    Ok(AnalyzedProject {
        manifest,
        entry_path: entry,
        mir,
    })
}

fn load_modules(project_root: &Path, entry_path: &Path) -> Result<Vec<LoadedModule>, Diagnostic> {
    let mut ordered = Vec::new();
    let mut loaded = HashMap::new();
    let mut visiting = Vec::new();
    load_module_recursive(
        project_root,
        entry_path,
        true,
        &mut ordered,
        &mut loaded,
        &mut visiting,
    )?;
    Ok(ordered)
}

fn load_module_recursive(
    project_root: &Path,
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

    let source = fs::read_to_string(&module_path).map_err(|err| {
        Diagnostic::new(
            "source",
            format!("failed to read {}: {err}", module_path.display()),
        )
        .with_path(module_path.display().to_string())
    })?;
    let program = syntax::parse_program(&source, &module_path)?;
    if !is_entry && !program.stmts.is_empty() {
        let stmt = &program.stmts[0];
        return Err(Diagnostic::new(
            "import",
            "imported stage1 modules may only contain imports and function declarations",
        )
        .with_path(module_path.display().to_string())
        .with_span(stmt_line(stmt), stmt_column(stmt)));
    }

    visiting.push(module_path.clone());
    for import in &program.imports {
        let import_path = resolve_import_path(project_root, &module_path, import)?;
        load_module_recursive(project_root, &import_path, false, ordered, loaded, visiting)?;
    }
    visiting.pop();

    loaded.insert(module_path.clone(), ());
    ordered.push(LoadedModule {
        path: module_path,
        program,
        is_entry,
    });
    Ok(())
}

fn flatten_modules(
    modules: &[LoadedModule],
    source_root: &Path,
) -> Result<syntax::Program, Diagnostic> {
    let mut symbols = HashMap::new();
    for module in modules {
        symbols.insert(
            module.path.clone(),
            build_module_symbols(module, source_root)?,
        );
    }

    let mut flattened_functions = Vec::new();
    let mut flattened_stmts = Vec::new();
    for module in modules {
        let Some(module_symbols) = symbols.get(&module.path) else {
            continue;
        };
        let mut visible = module_symbols.functions.clone();
        let mut private_imported = HashSet::new();
        for import in &module.program.imports {
            let import_path = resolve_import_path(source_root, &module.path, import)?;
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
                if let Some(existing) = visible.get(export_name)
                    && existing != internal_name
                {
                    return Err(Diagnostic::new(
                        "import",
                        format!("imported function {export_name:?} collides with an existing name"),
                    )
                    .with_path(module.path.display().to_string())
                    .with_span(import.line, import.column));
                }
                visible.insert(export_name.clone(), internal_name.clone());
            }
        }

        for function in &module.program.functions {
            flattened_functions.push(rewrite_function(
                function,
                module_symbols,
                &visible,
                &private_imported,
                &module.path,
            )?);
        }
        if module.is_entry {
            for stmt in &module.program.stmts {
                flattened_stmts.push(rewrite_stmt(
                    stmt,
                    &visible,
                    &private_imported,
                    &module.path,
                )?);
            }
        }
    }

    Ok(syntax::Program {
        imports: Vec::new(),
        functions: flattened_functions,
        stmts: flattened_stmts,
    })
}

fn build_module_symbols(
    module: &LoadedModule,
    source_root: &Path,
) -> Result<ModuleSymbols, Diagnostic> {
    let module_id = module_id_for_path(&module.path, source_root);
    let mut functions = HashMap::new();
    let mut public_functions = HashMap::new();
    let mut private_functions = HashSet::new();
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
    Ok(ModuleSymbols {
        module_id,
        functions,
        public_functions,
        private_functions,
    })
}

fn rewrite_function(
    function: &syntax::Function,
    module_symbols: &ModuleSymbols,
    visible: &HashMap<String, String>,
    private_imported: &HashSet<String>,
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
        .map(|stmt| rewrite_stmt(stmt, visible, private_imported, module_path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rewritten)
}

fn rewrite_stmt(
    stmt: &syntax::Stmt,
    visible: &HashMap<String, String>,
    private_imported: &HashSet<String>,
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
            ty: ty.clone(),
            expr: rewrite_expr(expr, visible, private_imported, module_path)?,
            line: *line,
            column: *column,
        },
        syntax::Stmt::Print { expr, line, column } => syntax::Stmt::Print {
            expr: rewrite_expr(expr, visible, private_imported, module_path)?,
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
            cond: rewrite_expr(cond, visible, private_imported, module_path)?,
            then_block: then_block
                .iter()
                .map(|stmt| rewrite_stmt(stmt, visible, private_imported, module_path))
                .collect::<Result<Vec<_>, _>>()?,
            else_block: else_block
                .as_ref()
                .map(|block| {
                    block
                        .iter()
                        .map(|stmt| rewrite_stmt(stmt, visible, private_imported, module_path))
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
            cond: rewrite_expr(cond, visible, private_imported, module_path)?,
            body: body
                .iter()
                .map(|stmt| rewrite_stmt(stmt, visible, private_imported, module_path))
                .collect::<Result<Vec<_>, _>>()?,
            line: *line,
            column: *column,
        },
        syntax::Stmt::Return { expr, line, column } => syntax::Stmt::Return {
            expr: rewrite_expr(expr, visible, private_imported, module_path)?,
            line: *line,
            column: *column,
        },
    })
}

fn rewrite_expr(
    expr: &syntax::Expr,
    visible: &HashMap<String, String>,
    private_imported: &HashSet<String>,
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
            if !visible.contains_key(name) && private_imported.contains(name) {
                return Err(Diagnostic::new(
                    "import",
                    format!("function {name:?} is not exported by an imported module"),
                )
                .with_path(module_path.display().to_string())
                .with_span(*line, *column));
            }
            syntax::Expr::Call {
                name: visible.get(name).cloned().unwrap_or_else(|| name.clone()),
                args: args
                    .iter()
                    .map(|arg| rewrite_expr(arg, visible, private_imported, module_path))
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
            lhs: Box::new(rewrite_expr(lhs, visible, private_imported, module_path)?),
            rhs: Box::new(rewrite_expr(rhs, visible, private_imported, module_path)?),
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
            lhs: Box::new(rewrite_expr(lhs, visible, private_imported, module_path)?),
            rhs: Box::new(rewrite_expr(rhs, visible, private_imported, module_path)?),
            line: *line,
            column: *column,
        },
    })
}

fn resolve_import_path(
    project_root: &Path,
    module_path: &Path,
    import: &syntax::Import,
) -> Result<PathBuf, Diagnostic> {
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
    let base_dir = module_path.parent().unwrap_or(project_root);
    let candidate = normalize_path(base_dir.join(relative));
    if !candidate.starts_with(project_root) {
        return Err(
            Diagnostic::new("import", "stage1 imports must stay inside the project")
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
    Ok(candidate)
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

fn module_id_for_path(path: &Path, source_root: &Path) -> String {
    let relative = path.strip_prefix(source_root).unwrap_or(path);
    let stem = relative.with_extension("");
    let mut out = String::new();
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

fn stmt_line(stmt: &syntax::Stmt) -> usize {
    match stmt {
        syntax::Stmt::Let { line, .. }
        | syntax::Stmt::Print { line, .. }
        | syntax::Stmt::If { line, .. }
        | syntax::Stmt::While { line, .. }
        | syntax::Stmt::Return { line, .. } => *line,
    }
}

fn stmt_column(stmt: &syntax::Stmt) -> usize {
    match stmt {
        syntax::Stmt::Let { column, .. }
        | syntax::Stmt::Print { column, .. }
        | syntax::Stmt::If { column, .. }
        | syntax::Stmt::While { column, .. }
        | syntax::Stmt::Return { column, .. } => *column,
    }
}
