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
    structs: HashMap<String, String>,
    public_structs: HashMap<String, String>,
    private_structs: HashSet<String>,
    enums: HashMap<String, String>,
    public_enums: HashMap<String, String>,
    private_enums: HashSet<String>,
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
            "imported stage1 modules may only contain imports, struct declarations, enum declarations, and function declarations",
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
    let mut flattened_structs = Vec::new();
    let mut flattened_enums = Vec::new();
    let mut flattened_stmts = Vec::new();
    for module in modules {
        let Some(module_symbols) = symbols.get(&module.path) else {
            continue;
        };
        let mut visible_functions = module_symbols.functions.clone();
        let mut visible_structs = module_symbols.structs.clone();
        let mut visible_enums = module_symbols.enums.clone();
        let mut private_imported = HashSet::new();
        let mut private_imported_types = HashSet::new();
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

        let visible_types = merge_visible_types(&visible_structs, &visible_enums, &module.path)?;

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
        imports: Vec::new(),
        structs: flattened_structs,
        enums: flattened_enums,
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
    Ok(ModuleSymbols {
        module_id,
        functions,
        public_functions,
        private_functions,
        structs,
        public_structs,
        private_structs,
        enums,
        public_enums,
        private_enums,
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
    visible_structs: &HashMap<String, String>,
    visible_enums: &HashMap<String, String>,
    module_path: &Path,
) -> Result<HashMap<String, String>, Diagnostic> {
    let mut visible_types = visible_structs.clone();
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
