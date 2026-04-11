use crate::diagnostics::Diagnostic;
use crate::manifest::{CapabilityConfig, CapabilityKind};
use crate::syntax;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Program {
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub functions: Vec<Function>,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnumVariantDef {
    pub name: String,
    pub payload_tys: Vec<Type>,
    pub payload_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Type,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Stmt {
    Let {
        name: String,
        ty: Type,
        expr: Expr,
    },
    Print(Expr),
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
    },
    Return(Expr),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MatchArm {
    pub enum_name: String,
    pub variant: String,
    pub bindings: Vec<String>,
    pub is_named: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MapEntry {
    pub key: Expr,
    pub value: Expr,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Expr {
    Literal {
        ty: Type,
        value: LiteralValue,
    },
    VarRef {
        name: String,
        ty: Type,
    },
    Call {
        name: String,
        args: Vec<Expr>,
        ty: Type,
    },
    BinaryAdd {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        ty: Type,
    },
    BinaryCompare {
        op: CompareOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        ty: Type,
    },
    StructLiteral {
        name: String,
        fields: Vec<StructFieldValue>,
        ty: Type,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        ty: Type,
    },
    TupleLiteral {
        elements: Vec<Expr>,
        ty: Type,
    },
    TupleIndex {
        base: Box<Expr>,
        index: usize,
        ty: Type,
    },
    MapLiteral {
        entries: Vec<MapEntry>,
        ty: Type,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        field_names: Vec<String>,
        payloads: Vec<Expr>,
        ty: Type,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        ty: Type,
    },
    Slice {
        base: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        ty: Type,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        ty: Type,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Type {
    Int,
    Bool,
    String,
    Struct(String),
    Enum(String),
    Slice(Box<Type>),
    MutSlice(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Map(Box<Type>, Box<Type>),
    Array(Box<Type>),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum LiteralValue {
    Int(i64),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructFieldValue {
    pub name: String,
    pub expr: Expr,
}

#[derive(Debug, Clone)]
struct Binding {
    ty: Type,
    moved: bool,
    borrow_origin: Option<BorrowOrigin>,
    borrowed_owners: HashSet<String>,
    active_borrow_count: usize,
}

#[derive(Debug, Clone)]
struct FunctionSig {
    params: Vec<Type>,
    return_ty: Type,
    borrow_return_params: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BorrowOrigin {
    Param(String),
    Local,
}

struct LowerContext<'a> {
    structs: &'a HashMap<String, StructDef>,
    enums: &'a HashMap<String, EnumDef>,
    variants: &'a HashMap<String, VariantInfo>,
    functions: &'a HashMap<String, FunctionSig>,
    capabilities: &'a CapabilityConfig,
    current_return: Option<Type>,
    current_borrow_return_params: HashSet<String>,
}

#[derive(Debug, Clone)]
struct VariantInfo {
    enum_name: String,
    payload_tys: Vec<Type>,
    payload_names: Vec<String>,
}

pub fn lower(program: &syntax::Program) -> Result<Program, Diagnostic> {
    let capabilities = CapabilityConfig::default();
    lower_with_capabilities(program, &capabilities)
}

pub fn lower_with_capabilities(
    program: &syntax::Program,
    capabilities: &CapabilityConfig,
) -> Result<Program, Diagnostic> {
    let (struct_names, enum_names) = collect_type_names(&program.structs, &program.enums)?;
    let (enums, variants) = collect_enum_definitions(&program.enums, &struct_names, &enum_names)?;
    let structs = collect_struct_definitions(&program.structs, &enum_names)?;
    let functions = collect_function_signatures(&program.functions, &structs, &enums)?;
    let mut lowered_structs = structs.values().cloned().collect::<Vec<_>>();
    lowered_structs.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
    let mut lowered_enums = enums.values().cloned().collect::<Vec<_>>();
    lowered_enums.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
    let mut lowered_functions = Vec::new();
    for function in &program.functions {
        lowered_functions.push(lower_function(
            function,
            &structs,
            &enums,
            &variants,
            &functions,
            capabilities,
        )?);
    }
    let ctx = LowerContext {
        structs: &structs,
        enums: &enums,
        variants: &variants,
        functions: &functions,
        capabilities,
        current_return: None,
        current_borrow_return_params: HashSet::new(),
    };
    let mut env = HashMap::new();
    let (stmts, _, _) = lower_block(&program.stmts, &mut env, &ctx)?;
    Ok(Program {
        structs: lowered_structs,
        enums: lowered_enums,
        functions: lowered_functions,
        stmts,
    })
}

impl Type {
    pub fn is_copy(&self) -> bool {
        match self {
            Type::Int | Type::Bool | Type::Slice(_) => true,
            Type::MutSlice(_) => false,
            Type::Option(inner) => inner.is_copy(),
            Type::Result(ok, err) => ok.is_copy() && err.is_copy(),
            Type::Tuple(elements) => elements.iter().all(Type::is_copy),
            Type::String | Type::Struct(_) | Type::Enum(_) | Type::Map(_, _) | Type::Array(_) => {
                false
            }
        }
    }

    fn supports_map_key(&self) -> bool {
        match self {
            Type::Int | Type::Bool | Type::String => true,
            Type::Tuple(elements) => elements.iter().all(Type::supports_map_key),
            Type::Struct(_)
            | Type::Enum(_)
            | Type::Slice(_)
            | Type::MutSlice(_)
            | Type::Option(_)
            | Type::Result(_, _)
            | Type::Map(_, _)
            | Type::Array(_) => false,
        }
    }
}

fn collect_struct_definitions(
    structs: &[syntax::StructDecl],
    enums: &HashMap<String, ()>,
) -> Result<HashMap<String, StructDef>, Diagnostic> {
    let mut names = HashMap::new();
    for struct_decl in structs {
        if enums.contains_key(&struct_decl.name) {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate type name {:?}", struct_decl.name),
            )
            .with_span(struct_decl.line, struct_decl.column));
        }
        if names
            .insert(struct_decl.name.clone(), struct_decl.clone())
            .is_some()
        {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate struct {:?}", struct_decl.name),
            )
            .with_span(struct_decl.line, struct_decl.column));
        }
    }

    let mut lowered = HashMap::new();
    for struct_decl in structs {
        let mut fields = Vec::new();
        let mut seen = HashMap::new();
        for field in &struct_decl.fields {
            if seen.insert(field.name.clone(), ()).is_some() {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "duplicate field {:?} in struct {:?}",
                        field.name, struct_decl.name
                    ),
                )
                .with_span(field.line, field.column));
            }
            let ty = lower_type(&field.ty, &names, enums, field.line, field.column)?;
            if matches!(&ty, Type::Struct(name) if name == &struct_decl.name) {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "recursive field {:?} in struct {:?} is not supported yet",
                        field.name, struct_decl.name
                    ),
                )
                .with_span(field.line, field.column));
            }
            fields.push(StructField {
                name: field.name.clone(),
                ty,
            });
        }
        lowered.insert(
            struct_decl.name.clone(),
            StructDef {
                name: struct_decl.name.clone(),
                fields,
            },
        );
    }
    Ok(lowered)
}

fn collect_type_names(
    structs: &[syntax::StructDecl],
    enums: &[syntax::EnumDecl],
) -> Result<(HashMap<String, syntax::StructDecl>, HashMap<String, ()>), Diagnostic> {
    let mut struct_names = HashMap::new();
    for struct_decl in structs {
        if struct_names
            .insert(struct_decl.name.clone(), struct_decl.clone())
            .is_some()
        {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate struct {:?}", struct_decl.name),
            )
            .with_span(struct_decl.line, struct_decl.column));
        }
    }
    let mut enum_names = HashMap::new();
    for enum_decl in enums {
        if struct_names.contains_key(&enum_decl.name) {
            return Err(Diagnostic::new(
                "type",
                format!("duplicate type name {:?}", enum_decl.name),
            )
            .with_span(enum_decl.line, enum_decl.column));
        }
        if enum_names.insert(enum_decl.name.clone(), ()).is_some() {
            return Err(
                Diagnostic::new("type", format!("duplicate enum {:?}", enum_decl.name))
                    .with_span(enum_decl.line, enum_decl.column),
            );
        }
    }
    Ok((struct_names, enum_names))
}

fn collect_enum_definitions(
    enums: &[syntax::EnumDecl],
    structs: &HashMap<String, syntax::StructDecl>,
    enum_names: &HashMap<String, ()>,
) -> Result<(HashMap<String, EnumDef>, HashMap<String, VariantInfo>), Diagnostic> {
    let mut lowered = HashMap::new();
    let mut variants = HashMap::new();
    for enum_decl in enums {
        if enum_decl.variants.is_empty() {
            return Err(Diagnostic::new(
                "type",
                format!(
                    "enum {:?} must declare at least one variant",
                    enum_decl.name
                ),
            )
            .with_span(enum_decl.line, enum_decl.column));
        }
        let mut seen = HashMap::new();
        let mut lowered_variants = Vec::new();
        for variant in &enum_decl.variants {
            if seen.insert(variant.name.clone(), ()).is_some() {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "duplicate variant {:?} in enum {:?}",
                        variant.name, enum_decl.name
                    ),
                )
                .with_span(variant.line, variant.column));
            }
            if let Some(existing_enum) = variants.insert(
                variant.name.clone(),
                VariantInfo {
                    enum_name: enum_decl.name.clone(),
                    payload_tys: variant
                        .payload_tys
                        .iter()
                        .map(|ty| lower_type(ty, structs, enum_names, variant.line, variant.column))
                        .collect::<Result<Vec<_>, Diagnostic>>()?,
                    payload_names: variant.payload_names.clone(),
                },
            ) && existing_enum.enum_name != enum_decl.name
            {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "duplicate variant name {:?} across enums {:?} and {:?} is not yet supported",
                        variant.name, existing_enum.enum_name, enum_decl.name
                    ),
                )
                .with_span(variant.line, variant.column));
            }
            let payload_tys = variant
                .payload_tys
                .iter()
                .map(|ty| lower_type(ty, structs, enum_names, variant.line, variant.column))
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            if !variant.payload_names.is_empty() && variant.payload_names.len() != payload_tys.len()
            {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "internal error: enum variant {:?} has mismatched named payload metadata",
                        variant.name
                    ),
                )
                .with_span(variant.line, variant.column));
            }
            let mut seen_payload_names = HashMap::new();
            for payload_name in &variant.payload_names {
                if seen_payload_names
                    .insert(payload_name.clone(), ())
                    .is_some()
                {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "duplicate payload field {:?} in enum variant {:?}",
                            payload_name, variant.name
                        ),
                    )
                    .with_span(variant.line, variant.column));
                }
            }
            if payload_tys
                .iter()
                .any(|ty| matches!(ty, Type::Enum(name) if name == &enum_decl.name))
            {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "recursive payload variant {:?} in enum {:?} is not supported yet",
                        variant.name, enum_decl.name
                    ),
                )
                .with_span(variant.line, variant.column));
            }
            lowered_variants.push(EnumVariantDef {
                name: variant.name.clone(),
                payload_tys,
                payload_names: variant.payload_names.clone(),
            });
        }
        lowered.insert(
            enum_decl.name.clone(),
            EnumDef {
                name: enum_decl.name.clone(),
                variants: lowered_variants,
            },
        );
    }
    Ok((lowered, variants))
}

fn collect_function_signatures(
    functions: &[syntax::Function],
    structs: &HashMap<String, StructDef>,
    enums: &HashMap<String, EnumDef>,
) -> Result<HashMap<String, FunctionSig>, Diagnostic> {
    let mut signatures = HashMap::new();
    for function in functions {
        let return_ty = lower_type(
            &function.return_ty,
            structs,
            enums,
            function.line,
            function.column,
        )?;
        let mut params = Vec::new();
        for param in &function.params {
            params.push(lower_type(
                &param.ty,
                structs,
                enums,
                param.line,
                param.column,
            )?);
        }
        let borrow_return_params = classify_borrow_return(
            &params,
            &return_ty,
            structs,
            enums,
            function.line,
            function.column,
        )?;
        if signatures
            .insert(
                function.name.clone(),
                FunctionSig {
                    params,
                    return_ty,
                    borrow_return_params,
                },
            )
            .is_some()
        {
            return Err(
                Diagnostic::new("type", format!("duplicate function {:?}", function.name))
                    .with_span(function.line, function.column),
            );
        }
    }
    Ok(signatures)
}

fn lower_function(
    function: &syntax::Function,
    structs: &HashMap<String, StructDef>,
    enums: &HashMap<String, EnumDef>,
    variants: &HashMap<String, VariantInfo>,
    functions: &HashMap<String, FunctionSig>,
    capabilities: &CapabilityConfig,
) -> Result<Function, Diagnostic> {
    let return_ty = lower_type(
        &function.return_ty,
        structs,
        enums,
        function.line,
        function.column,
    )?;
    let signature = functions
        .get(&function.name)
        .expect("function signatures collected before lowering");
    let ctx = LowerContext {
        structs,
        enums,
        variants,
        functions,
        capabilities,
        current_return: Some(return_ty.clone()),
        current_borrow_return_params: signature
            .borrow_return_params
            .iter()
            .map(|index| function.params[*index].name.clone())
            .collect(),
    };
    let mut env: HashMap<String, Binding> = HashMap::new();
    let mut params = Vec::new();
    for param in &function.params {
        if functions.contains_key(&param.name) {
            return Err(Diagnostic::new(
                "type",
                format!(
                    "binding name {:?} conflicts with a function name",
                    param.name
                ),
            )
            .with_span(param.line, param.column));
        }
        if env.contains_key(&param.name) {
            return Err(
                Diagnostic::new("type", format!("duplicate parameter {:?}", param.name))
                    .with_span(param.line, param.column),
            );
        }
        let ty = lower_type(&param.ty, structs, enums, param.line, param.column)?;
        env.insert(
            param.name.clone(),
            Binding {
                ty: ty.clone(),
                moved: false,
                borrow_origin: binding_borrow_origin(&ty, Some(&param.name), structs, enums),
                borrowed_owners: HashSet::new(),
                active_borrow_count: 0,
            },
        );
        params.push(Param {
            name: param.name.clone(),
            ty,
        });
    }
    let (body, _, guaranteed_return) = lower_block(&function.body, &mut env, &ctx)?;
    if !guaranteed_return {
        return Err(Diagnostic::new(
            "control",
            format!(
                "function {:?} does not return along all paths",
                function.name
            ),
        )
        .with_span(function.line, function.column));
    }
    Ok(Function {
        name: function.name.clone(),
        params,
        return_ty,
        body,
    })
}

fn lower_block(
    block: &[syntax::Stmt],
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Result<(Vec<Stmt>, HashMap<String, Binding>, bool), Diagnostic> {
    let scope_names = env.keys().cloned().collect::<HashSet<_>>();
    let mut lowered = Vec::new();
    let mut guaranteed_return = false;
    for stmt in block {
        if guaranteed_return {
            return Err(Diagnostic::new(
                "control",
                "unreachable statements after return are not yet supported in stage1",
            )
            .with_span(stmt.line(), stmt.column()));
        }
        let lowered_stmt = lower_stmt(stmt, env, ctx)?;
        guaranteed_return = lowered_stmt.always_returns();
        lowered.push(lowered_stmt);
    }
    let mut after = env.clone();
    release_scope_borrows(&mut after, &scope_names);
    Ok((lowered, after, guaranteed_return))
}

fn lower_stmt(
    stmt: &syntax::Stmt,
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Result<Stmt, Diagnostic> {
    match stmt {
        syntax::Stmt::Let {
            name,
            ty,
            expr,
            line,
            column,
        } => {
            if ctx.functions.contains_key(name) {
                return Err(Diagnostic::new(
                    "type",
                    format!("binding name {name:?} conflicts with a function name"),
                )
                .with_span(*line, *column));
            }
            if env.contains_key(name) {
                return Err(Diagnostic::new(
                    "type",
                    format!("rebinding existing name {name:?} is not yet supported in stage1"),
                )
                .with_span(*line, *column));
            }
            let expected = lower_type(ty, ctx.structs, ctx.enums, *line, *column)?;
            let lowered_expr = lower_expr_with_expected(expr, Some(&expected), env, ctx)?;
            let actual = lowered_expr.ty().clone();
            if actual != expected {
                return Err(Diagnostic::new(
                    "type",
                    format!("let binding {name:?} expects {expected}, got {actual}"),
                )
                .with_span(*line, *column));
            }
            let borrowed_owners =
                binding_borrowed_owners_from_expr(&expected, &lowered_expr, env, ctx);
            increment_active_borrows(&borrowed_owners, env)?;
            if !actual.is_copy() {
                move_lowered_value(&lowered_expr, env)?;
            }
            env.insert(
                name.clone(),
                Binding {
                    ty: expected.clone(),
                    moved: false,
                    borrow_origin: binding_borrow_origin_from_expr(
                        &expected,
                        &lowered_expr,
                        env,
                        ctx,
                    ),
                    borrowed_owners,
                    active_borrow_count: 0,
                },
            );
            Ok(Stmt::Let {
                name: name.clone(),
                ty: expected,
                expr: lowered_expr,
            })
        }
        syntax::Stmt::Print { expr, line, column } => {
            let lowered = lower_expr(expr, env, ctx)?;
            if !matches!(lowered.ty(), Type::Int | Type::Bool | Type::String) {
                return Err(Diagnostic::new(
                    "type",
                    format!("print expects int, bool, or string, got {}", lowered.ty()),
                )
                .with_span(*line, *column));
            }
            Ok(Stmt::Print(lowered))
        }
        syntax::Stmt::If {
            cond,
            then_block,
            else_block,
            line,
            column,
        } => {
            let lowered_cond = lower_expr(cond, env, ctx)?;
            if lowered_cond.ty() != &Type::Bool {
                return Err(Diagnostic::new(
                    "type",
                    format!("if condition expects bool, got {}", lowered_cond.ty()),
                )
                .with_span(*line, *column));
            }
            if let Some(known_cond) = static_bool_value(&lowered_cond) {
                if known_cond {
                    let mut then_env = env.clone();
                    let (then_block, then_after, _) = lower_block(then_block, &mut then_env, ctx)?;
                    *env = then_after;
                    return Ok(Stmt::If {
                        cond: lowered_cond,
                        then_block,
                        else_block: else_block.as_ref().map(|_| Vec::new()),
                    });
                }
                if let Some(else_block) = else_block {
                    let mut else_env = env.clone();
                    let (block, after, _) = lower_block(else_block, &mut else_env, ctx)?;
                    *env = after;
                    return Ok(Stmt::If {
                        cond: lowered_cond,
                        then_block: Vec::new(),
                        else_block: Some(block),
                    });
                }
                return Ok(Stmt::If {
                    cond: lowered_cond,
                    then_block: Vec::new(),
                    else_block: None,
                });
            }
            let before = env.clone();
            let mut then_env = before.clone();
            let (then_block, then_after, then_returns) =
                lower_block(then_block, &mut then_env, ctx)?;
            let (else_block, else_after, else_returns) = if let Some(else_block) = else_block {
                let mut else_env = before.clone();
                let (block, after, returns) = lower_block(else_block, &mut else_env, ctx)?;
                (Some(block), Some(after), returns)
            } else {
                (None, None, false)
            };
            merge_branch_state(
                env,
                &before,
                &then_after,
                then_returns,
                else_after.as_ref(),
                else_returns,
            );
            Ok(Stmt::If {
                cond: lowered_cond,
                then_block,
                else_block,
            })
        }
        syntax::Stmt::While {
            cond,
            body,
            line,
            column,
        } => {
            let lowered_cond = lower_expr(cond, env, ctx)?;
            if lowered_cond.ty() != &Type::Bool {
                return Err(Diagnostic::new(
                    "type",
                    format!("while condition expects bool, got {}", lowered_cond.ty()),
                )
                .with_span(*line, *column));
            }
            if static_bool_value(&lowered_cond) == Some(false) {
                return Ok(Stmt::While {
                    cond: lowered_cond,
                    body: Vec::new(),
                });
            }
            let before = env.clone();
            let mut body_env = before.clone();
            let (body, body_after, body_returns) = lower_block(body, &mut body_env, ctx)?;
            // AG1.1: reject moves of outer non-Copy variables inside the loop
            // body — on subsequent iterations the value would not be available.
            if !body_returns {
                for (name, pre_binding) in &before {
                    if pre_binding.moved || pre_binding.ty.is_copy() {
                        continue;
                    }
                    if let Some(post_binding) = body_after.get(name) {
                        if post_binding.moved {
                            return Err(Diagnostic::new(
                                "ownership",
                                format!(
                                    "cannot move non-copy value `{}` inside loop body — \
                                     value would not be available on subsequent iterations",
                                    name
                                ),
                            )
                            .with_span(*line, *column));
                        }
                    }
                }
            }
            merge_loop_state(env, &before, &body_after, body_returns);
            Ok(Stmt::While {
                cond: lowered_cond,
                body,
            })
        }
        syntax::Stmt::Match {
            expr,
            arms,
            line,
            column,
        } => {
            let lowered_expr = lower_expr(expr, env, ctx)?;
            let match_borrowed_owners = expr_borrowed_owners(&lowered_expr, env, ctx);
            increment_active_borrows(&match_borrowed_owners, env)?;
            if matches!(lowered_expr, Expr::VarRef { .. }) && !lowered_expr.ty().is_copy() {
                move_lowered_owner_value(&lowered_expr, env)?;
            }
            let (enum_name, variant_defs) =
                match_variants(lowered_expr.ty(), ctx).ok_or_else(|| {
                    Diagnostic::new(
                        "type",
                        format!(
                            "match expects an enum-like value, got {}",
                            lowered_expr.ty()
                        ),
                    )
                    .with_span(*line, *column)
                })?;
            let before = env.clone();
            let mut seen = HashMap::new();
            let mut lowered_arms = Vec::new();
            let mut arm_states = Vec::new();
            for arm in arms {
                let variant_def = variant_defs
                    .iter()
                    .find(|variant| variant.name == arm.variant)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            "type",
                            format!("enum {enum_name:?} has no variant {:?}", arm.variant),
                        )
                        .with_span(arm.line, arm.column)
                    })?;
                if seen.insert(arm.variant.clone(), ()).is_some() {
                    return Err(Diagnostic::new(
                        "type",
                        format!("duplicate match arm {:?}", arm.variant),
                    )
                    .with_span(arm.line, arm.column));
                }
                let mut arm_env = before.clone();
                let binding_tys = if arm.is_named {
                    if variant_def.payload_names.is_empty() {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "match arm {:?} uses named bindings, but variant {:?} is positional",
                                arm.variant, arm.variant
                            ),
                        )
                        .with_span(arm.line, arm.column));
                    }
                    if arm.bindings.len() != variant_def.payload_names.len() {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "match arm {:?} expects {} named bindings, got {}",
                                arm.variant,
                                variant_def.payload_names.len(),
                                arm.bindings.len()
                            ),
                        )
                        .with_span(arm.line, arm.column));
                    }
                    let mut seen_named = HashMap::new();
                    let mut payload_tys = Vec::new();
                    for binding in &arm.bindings {
                        let Some(position) = variant_def
                            .payload_names
                            .iter()
                            .position(|name| name == binding)
                        else {
                            return Err(Diagnostic::new(
                                "type",
                                format!(
                                    "match arm {:?} has no named payload {:?}",
                                    arm.variant, binding
                                ),
                            )
                            .with_span(arm.line, arm.column));
                        };
                        if seen_named.insert(binding.clone(), ()).is_some() {
                            return Err(Diagnostic::new(
                                "type",
                                format!(
                                    "match arm {:?} repeats named payload {:?}",
                                    arm.variant, binding
                                ),
                            )
                            .with_span(arm.line, arm.column));
                        }
                        payload_tys.push(variant_def.payload_tys[position].clone());
                    }
                    payload_tys
                } else {
                    if !variant_def.payload_names.is_empty() {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "match arm {:?} must use named bindings for variant {:?}",
                                arm.variant, arm.variant
                            ),
                        )
                        .with_span(arm.line, arm.column));
                    }
                    if arm.bindings.len() != variant_def.payload_tys.len() {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "match arm {:?} expects {} bindings, got {}",
                                arm.variant,
                                variant_def.payload_tys.len(),
                                arm.bindings.len()
                            ),
                        )
                        .with_span(arm.line, arm.column));
                    }
                    variant_def.payload_tys.clone()
                };
                for (binding_index, (binding, payload_ty)) in
                    arm.bindings.iter().zip(binding_tys.iter()).enumerate()
                {
                    if ctx.functions.contains_key(binding) {
                        return Err(Diagnostic::new(
                            "type",
                            format!("match binding {binding:?} conflicts with a function name"),
                        )
                        .with_span(arm.line, arm.column));
                    }
                    if arm_env.contains_key(binding) {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "match binding {binding:?} reuses an existing name in the current scope"
                            ),
                        )
                        .with_span(arm.line, arm.column));
                    }
                    arm_env.insert(
                        binding.clone(),
                        Binding {
                            ty: payload_ty.clone(),
                            moved: false,
                            borrow_origin: match_binding_borrow_origin(
                                &lowered_expr,
                                &arm.variant,
                                binding,
                                binding_index,
                                payload_ty,
                                &before,
                                ctx,
                            ),
                            borrowed_owners: match_binding_borrowed_owners(
                                &lowered_expr,
                                &arm.variant,
                                binding,
                                binding_index,
                                payload_ty,
                                &before,
                                ctx,
                            ),
                            active_borrow_count: 0,
                        },
                    );
                }
                let (body, after, returns) = lower_block(&arm.body, &mut arm_env, ctx)?;
                lowered_arms.push(MatchArm {
                    enum_name: enum_name.clone(),
                    variant: arm.variant.clone(),
                    bindings: arm.bindings.clone(),
                    is_named: arm.is_named,
                    body,
                });
                arm_states.push((after, returns));
            }
            let missing = variant_defs
                .iter()
                .filter(|variant| !seen.contains_key(&variant.name))
                .map(|variant| variant.name.clone())
                .collect::<Vec<_>>();
            if !missing.is_empty() {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "match on {:?} is not exhaustive; missing {}",
                        enum_name,
                        missing.join(", ")
                    ),
                )
                .with_span(*line, *column));
            }
            merge_match_state(env, &before, &arm_states);
            release_active_borrow_owners(&match_borrowed_owners, env);
            Ok(Stmt::Match {
                expr: lowered_expr,
                arms: lowered_arms,
            })
        }
        syntax::Stmt::Return { expr, line, column } => {
            let Some(expected) = ctx.current_return.as_ref() else {
                return Err(
                    Diagnostic::new("control", "return is only valid inside a function")
                        .with_span(*line, *column),
                );
            };
            let lowered_expr = lower_expr_with_expected(expr, Some(expected), env, ctx)?;
            if lowered_expr.ty() != expected {
                return Err(Diagnostic::new(
                    "type",
                    format!("return expects {expected}, got {}", lowered_expr.ty()),
                )
                .with_span(*line, *column));
            }
            if contains_borrowed_slice_type(expected, ctx.structs, ctx.enums)
                && !ctx.current_borrow_return_params.is_empty()
            {
                match expr_borrow_origin(&lowered_expr, env, ctx) {
                    None => {}
                    Some(BorrowOrigin::Param(origin))
                        if ctx.current_borrow_return_params.contains(&origin) => {}
                    _ => {
                        return Err(Diagnostic::new(
                            "ownership",
                            format!(
                                "returning borrowed values requires data derived from one of the borrowed parameters in stage1"
                            ),
                        )
                        .with_span(*line, *column));
                    }
                }
            }
            Ok(Stmt::Return(lowered_expr))
        }
    }
}

fn merge_branch_state(
    env: &mut HashMap<String, Binding>,
    before: &HashMap<String, Binding>,
    then_after: &HashMap<String, Binding>,
    then_returns: bool,
    else_after: Option<&HashMap<String, Binding>>,
    else_returns: bool,
) {
    env.clear();
    for (name, binding) in before {
        let then_moved = if then_returns {
            binding.moved
        } else {
            then_after
                .get(name)
                .map(|entry| entry.moved)
                .unwrap_or(binding.moved)
        };
        let else_moved = if else_returns {
            binding.moved
        } else {
            else_after
                .and_then(|branch| branch.get(name).map(|entry| entry.moved))
                .unwrap_or(binding.moved)
        };
        env.insert(
            name.clone(),
            Binding {
                ty: binding.ty.clone(),
                moved: then_moved || else_moved,
                borrow_origin: binding.borrow_origin.clone(),
                borrowed_owners: binding.borrowed_owners.clone(),
                active_borrow_count: merge_borrow_count(
                    binding.active_borrow_count,
                    then_returns,
                    then_after.get(name).map(|entry| entry.active_borrow_count),
                    else_returns,
                    else_after
                        .and_then(|branch| branch.get(name).map(|entry| entry.active_borrow_count)),
                ),
            },
        );
    }
}

fn merge_loop_state(
    env: &mut HashMap<String, Binding>,
    before: &HashMap<String, Binding>,
    body_after: &HashMap<String, Binding>,
    body_returns: bool,
) {
    // AG1.1: the loop body may execute zero times, so post-loop ownership
    // state preserves the pre-loop moved flags.  Moves of outer non-Copy
    // values inside the body are rejected earlier (before this function is
    // called), so the only moved-state change that can reach here is for
    // values that were already moved before the loop.  Borrow counts still
    // take the max of pre-loop and body-after to stay conservative.
    env.clear();
    for (name, binding) in before {
        env.insert(
            name.clone(),
            Binding {
                ty: binding.ty.clone(),
                moved: binding.moved,
                borrow_origin: binding.borrow_origin.clone(),
                borrowed_owners: binding.borrowed_owners.clone(),
                active_borrow_count: if body_returns {
                    binding.active_borrow_count
                } else {
                    let body_count = body_after
                        .get(name)
                        .map(|entry| entry.active_borrow_count)
                        .unwrap_or(binding.active_borrow_count);
                    binding.active_borrow_count.max(body_count)
                },
            },
        );
    }
}

fn merge_match_state(
    env: &mut HashMap<String, Binding>,
    before: &HashMap<String, Binding>,
    arm_states: &[(HashMap<String, Binding>, bool)],
) {
    env.clear();
    for (name, binding) in before {
        let moved = arm_states.iter().any(|(after, returns)| {
            if *returns {
                binding.moved
            } else {
                after
                    .get(name)
                    .map(|entry| entry.moved)
                    .unwrap_or(binding.moved)
            }
        });
        env.insert(
            name.clone(),
            Binding {
                ty: binding.ty.clone(),
                moved,
                borrow_origin: binding.borrow_origin.clone(),
                borrowed_owners: binding.borrowed_owners.clone(),
                active_borrow_count: arm_states
                    .iter()
                    .filter_map(|(after, returns)| {
                        if *returns {
                            Some(binding.active_borrow_count)
                        } else {
                            after.get(name).map(|entry| entry.active_borrow_count)
                        }
                    })
                    .max()
                    .unwrap_or(binding.active_borrow_count),
            },
        );
    }
}

fn match_variants(ty: &Type, ctx: &LowerContext<'_>) -> Option<(String, Vec<EnumVariantDef>)> {
    match ty {
        Type::Enum(enum_name) => ctx
            .enums
            .get(enum_name)
            .map(|enum_def| (enum_name.clone(), enum_def.variants.clone())),
        Type::Option(inner) => Some((
            String::from("Option"),
            vec![
                EnumVariantDef {
                    name: String::from("Some"),
                    payload_tys: vec![(*inner.clone()).clone()],
                    payload_names: Vec::new(),
                },
                EnumVariantDef {
                    name: String::from("None"),
                    payload_tys: Vec::new(),
                    payload_names: Vec::new(),
                },
            ],
        )),
        Type::Result(ok, err) => Some((
            String::from("Result"),
            vec![
                EnumVariantDef {
                    name: String::from("Ok"),
                    payload_tys: vec![(*ok.clone()).clone()],
                    payload_names: Vec::new(),
                },
                EnumVariantDef {
                    name: String::from("Err"),
                    payload_tys: vec![(*err.clone()).clone()],
                    payload_names: Vec::new(),
                },
            ],
        )),
        _ => None,
    }
}

fn lower_expr(
    expr: &syntax::Expr,
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Result<Expr, Diagnostic> {
    lower_expr_with_expected(expr, None, env, ctx)
}

fn lower_expr_with_expected(
    expr: &syntax::Expr,
    expected: Option<&Type>,
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Result<Expr, Diagnostic> {
    match expr {
        syntax::Expr::Literal(literal) => Ok(lower_literal(literal)),
        syntax::Expr::VarRef { name, line, column } => {
            if let Some(binding) = env.get(name) {
                if binding.moved {
                    return Err(Diagnostic::new(
                        "ownership",
                        format!("use of moved value {name:?}"),
                    )
                    .with_span(*line, *column));
                }
                return Ok(Expr::VarRef {
                    name: name.clone(),
                    ty: binding.ty.clone(),
                });
            }
            if name == "None" {
                if let Some(Type::Option(inner)) = expected {
                    return Ok(Expr::EnumVariant {
                        enum_name: String::from("Option"),
                        variant: String::from("None"),
                        field_names: Vec::new(),
                        payloads: Vec::new(),
                        ty: Type::Option(inner.clone()),
                    });
                }
                return Err(
                    Diagnostic::new("type", "None requires an expected Option<T> context")
                        .with_span(*line, *column),
                );
            }
            if let Some(variant) = ctx.variants.get(name) {
                if !variant.payload_tys.is_empty() {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "enum variant {name:?} requires {} arguments",
                            variant.payload_tys.len()
                        ),
                    )
                    .with_span(*line, *column));
                }
                return Ok(Expr::EnumVariant {
                    enum_name: variant.enum_name.clone(),
                    variant: name.clone(),
                    field_names: Vec::new(),
                    payloads: Vec::new(),
                    ty: Type::Enum(variant.enum_name.clone()),
                });
            }
            Err(
                Diagnostic::new("type", format!("undefined variable {name:?}"))
                    .with_span(*line, *column),
            )
        }
        syntax::Expr::Call {
            name,
            args,
            line,
            column,
        } => {
            if name == "len" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("len expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr(&args[0], env, ctx)?;
                if !matches!(
                    lowered.ty(),
                    Type::Array(_) | Type::Slice(_) | Type::MutSlice(_)
                ) {
                    return Err(Diagnostic::new(
                        "type",
                        format!("len expects an array or slice value, got {}", lowered.ty()),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::Int,
                });
            }
            if name == "io_eprintln" {
                // Ungated: stderr output is ambient, matching `print`'s
                // ungated statement form. No capability check.
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("io_eprintln expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr_with_expected(&args[0], Some(&Type::String), env, ctx)?;
                if lowered.ty() != &Type::String {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "io_eprintln expects a string argument, got {}",
                            lowered.ty()
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                move_lowered_value(&lowered, env)?;
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::Int,
                });
            }
            if name == "fs_read" {
                require_capability(ctx.capabilities, CapabilityKind::Fs, name, *line, *column)?;
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("fs_read expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr_with_expected(&args[0], Some(&Type::String), env, ctx)?;
                if lowered.ty() != &Type::String {
                    return Err(Diagnostic::new(
                        "type",
                        format!("fs_read expects a string argument, got {}", lowered.ty()),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                move_lowered_value(&lowered, env)?;
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::Option(Box::new(Type::String)),
                });
            }
            if name == "net_resolve" {
                require_capability(ctx.capabilities, CapabilityKind::Net, name, *line, *column)?;
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("net_resolve expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr_with_expected(&args[0], Some(&Type::String), env, ctx)?;
                if lowered.ty() != &Type::String {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "net_resolve expects a string argument, got {}",
                            lowered.ty()
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                move_lowered_value(&lowered, env)?;
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::Option(Box::new(Type::String)),
                });
            }
            if name == "http_get" {
                // HTTP GET shares the `net` capability surface: any code that
                // can open a raw TCP socket could implement HTTP itself, so a
                // separate `http` manifest flag would not add meaningful
                // isolation in stage1.
                require_capability(ctx.capabilities, CapabilityKind::Net, name, *line, *column)?;
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("http_get expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr_with_expected(&args[0], Some(&Type::String), env, ctx)?;
                if lowered.ty() != &Type::String {
                    return Err(Diagnostic::new(
                        "type",
                        format!("http_get expects a string argument, got {}", lowered.ty()),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                move_lowered_value(&lowered, env)?;
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::Option(Box::new(Type::String)),
                });
            }
            if name == "process_status" {
                require_capability(
                    ctx.capabilities,
                    CapabilityKind::Process,
                    name,
                    *line,
                    *column,
                )?;
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("process_status expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr_with_expected(&args[0], Some(&Type::String), env, ctx)?;
                if lowered.ty() != &Type::String {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "process_status expects a string argument, got {}",
                            lowered.ty()
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                move_lowered_value(&lowered, env)?;
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::Int,
                });
            }
            if name == "clock_now_ms" {
                require_capability(
                    ctx.capabilities,
                    CapabilityKind::Clock,
                    name,
                    *line,
                    *column,
                )?;
                if !args.is_empty() {
                    return Err(Diagnostic::new(
                        "type",
                        format!("clock_now_ms expects 0 arguments, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: Vec::new(),
                    ty: Type::Int,
                });
            }
            if name == "env_get" {
                require_capability(ctx.capabilities, CapabilityKind::Env, name, *line, *column)?;
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("env_get expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr_with_expected(&args[0], Some(&Type::String), env, ctx)?;
                if lowered.ty() != &Type::String {
                    return Err(Diagnostic::new(
                        "type",
                        format!("env_get expects a string argument, got {}", lowered.ty()),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                move_lowered_value(&lowered, env)?;
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::Option(Box::new(Type::String)),
                });
            }
            if name == "crypto_sha256" {
                require_capability(
                    ctx.capabilities,
                    CapabilityKind::Crypto,
                    name,
                    *line,
                    *column,
                )?;
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("crypto_sha256 expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr_with_expected(&args[0], Some(&Type::String), env, ctx)?;
                if lowered.ty() != &Type::String {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "crypto_sha256 expects a string argument, got {}",
                            lowered.ty()
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                move_lowered_value(&lowered, env)?;
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: Type::String,
                });
            }
            if name == "first" || name == "last" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("{name} expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let lowered = lower_expr(&args[0], env, ctx)?;
                let element_ty = match lowered.ty() {
                    Type::Array(element_ty)
                    | Type::Slice(element_ty)
                    | Type::MutSlice(element_ty) => (*element_ty.clone()).clone(),
                    _ => {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "{name} expects an array or slice value, got {}",
                                lowered.ty()
                            ),
                        )
                        .with_span(args[0].line(), args[0].column()));
                    }
                };
                if matches!(lowered.ty(), Type::Slice(_) | Type::MutSlice(_))
                    && !element_ty.is_copy()
                {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "{name} requires a Copy element type when called on a borrowed slice, got {element_ty}"
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                if matches!(lowered.ty(), Type::Array(_)) && !element_ty.is_copy() {
                    move_lowered_owner_value(&lowered, env)?;
                }
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: vec![lowered],
                    ty: element_ty,
                });
            }
            if let Some(signature) = ctx.functions.get(name) {
                if args.len() != signature.params.len() {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "function {name:?} expects {} arguments, got {}",
                            signature.params.len(),
                            args.len()
                        ),
                    )
                    .with_span(*line, *column));
                }
                let mut lowered_args = Vec::new();
                let mut temporary_borrows = Vec::new();
                for (arg, expected) in args.iter().zip(signature.params.iter()) {
                    let lowered = lower_expr_with_expected(arg, Some(expected), env, ctx)?;
                    if lowered.ty() != expected {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "function {name:?} expects argument type {expected}, got {}",
                                lowered.ty()
                            ),
                        )
                        .with_span(arg.line(), arg.column()));
                    }
                    record_temporary_borrows(&lowered, env, ctx, &mut temporary_borrows)?;
                    if !expected.is_copy() {
                        move_lowered_value(&lowered, env)?;
                    }
                    lowered_args.push(lowered);
                }
                release_temporary_borrows(&temporary_borrows, env);
                return Ok(Expr::Call {
                    name: name.clone(),
                    args: lowered_args,
                    ty: signature.return_ty.clone(),
                });
            }
            if name == "Some" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("Option::Some expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let inner_expected = match expected {
                    Some(Type::Option(inner)) => Some(inner.as_ref()),
                    _ => None,
                };
                let lowered = lower_expr_with_expected(&args[0], inner_expected, env, ctx)?;
                let inner_ty = lowered.ty().clone();
                if let Some(expected_inner) = inner_expected
                    && &inner_ty != expected_inner
                {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "Option::Some expects payload type {expected_inner}, got {inner_ty}"
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                if !inner_ty.is_copy() {
                    move_lowered_value(&lowered, env)?;
                }
                return Ok(Expr::EnumVariant {
                    enum_name: String::from("Option"),
                    variant: String::from("Some"),
                    field_names: Vec::new(),
                    payloads: vec![lowered],
                    ty: Type::Option(Box::new(inner_ty)),
                });
            }
            if name == "Ok" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("Result::Ok expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let Some(Type::Result(ok_ty, err_ty)) = expected else {
                    return Err(Diagnostic::new(
                        "type",
                        "Ok requires an expected Result<T, E> context",
                    )
                    .with_span(*line, *column));
                };
                let lowered = lower_expr_with_expected(&args[0], Some(ok_ty.as_ref()), env, ctx)?;
                if lowered.ty() != ok_ty.as_ref() {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "Result::Ok expects payload type {ok_ty}, got {}",
                            lowered.ty()
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                if !ok_ty.is_copy() {
                    move_lowered_value(&lowered, env)?;
                }
                return Ok(Expr::EnumVariant {
                    enum_name: String::from("Result"),
                    variant: String::from("Ok"),
                    field_names: Vec::new(),
                    payloads: vec![lowered],
                    ty: Type::Result(ok_ty.clone(), err_ty.clone()),
                });
            }
            if name == "Err" {
                if args.len() != 1 {
                    return Err(Diagnostic::new(
                        "type",
                        format!("Result::Err expects 1 argument, got {}", args.len()),
                    )
                    .with_span(*line, *column));
                }
                let Some(Type::Result(ok_ty, err_ty)) = expected else {
                    return Err(Diagnostic::new(
                        "type",
                        "Err requires an expected Result<T, E> context",
                    )
                    .with_span(*line, *column));
                };
                let lowered = lower_expr_with_expected(&args[0], Some(err_ty.as_ref()), env, ctx)?;
                if lowered.ty() != err_ty.as_ref() {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "Result::Err expects payload type {err_ty}, got {}",
                            lowered.ty()
                        ),
                    )
                    .with_span(args[0].line(), args[0].column()));
                }
                if !err_ty.is_copy() {
                    move_lowered_value(&lowered, env)?;
                }
                return Ok(Expr::EnumVariant {
                    enum_name: String::from("Result"),
                    variant: String::from("Err"),
                    field_names: Vec::new(),
                    payloads: vec![lowered],
                    ty: Type::Result(ok_ty.clone(), err_ty.clone()),
                });
            }
            if let Some(variant) = ctx.variants.get(name) {
                return lower_variant_constructor(name, args, *line, *column, variant, env, ctx);
            }
            Err(
                Diagnostic::new("type", format!("undefined function {name:?}"))
                    .with_span(*line, *column),
            )
        }
        syntax::Expr::BinaryAdd {
            lhs,
            rhs,
            line,
            column,
        } => {
            let lhs = lower_expr(lhs, env, ctx)?;
            let rhs = lower_expr(rhs, env, ctx)?;
            let lhs_ty = lhs.ty().clone();
            let rhs_ty = rhs.ty().clone();
            if lhs_ty != rhs_ty || !matches!(lhs_ty, Type::Int | Type::String) {
                return Err(
                    Diagnostic::new(
                        "type",
                        format!(
                            "operator '+' expects matching int or string operands, got {lhs_ty} and {rhs_ty}"
                        ),
                    )
                    .with_span(*line, *column),
                );
            }
            Ok(Expr::BinaryAdd {
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                ty: lhs_ty,
            })
        }
        syntax::Expr::BinaryCompare {
            op,
            lhs,
            rhs,
            line,
            column,
        } => {
            let lhs = lower_expr(lhs, env, ctx)?;
            let rhs = lower_expr(rhs, env, ctx)?;
            let lhs_ty = lhs.ty().clone();
            let rhs_ty = rhs.ty().clone();
            match op {
                syntax::CompareOp::Eq | syntax::CompareOp::Ne => {
                    if lhs_ty != rhs_ty {
                        return Err(
                            Diagnostic::new(
                                "type",
                                format!(
                                    "operator '{}' expects matching operand types, got {lhs_ty} and {rhs_ty}",
                                    op.lexeme()
                                ),
                            )
                            .with_span(*line, *column),
                        );
                    }
                }
                syntax::CompareOp::Lt
                | syntax::CompareOp::Le
                | syntax::CompareOp::Gt
                | syntax::CompareOp::Ge => {
                    if lhs_ty != Type::Int || rhs_ty != Type::Int {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "operator '{}' expects int operands, got {lhs_ty} and {rhs_ty}",
                                op.lexeme()
                            ),
                        )
                        .with_span(*line, *column));
                    }
                }
            }
            Ok(Expr::BinaryCompare {
                op: lower_compare_op(*op),
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                ty: Type::Bool,
            })
        }
        syntax::Expr::StructLiteral {
            name,
            fields,
            line,
            column,
        } => {
            if let Some(variant) = ctx.variants.get(name)
                && !variant.payload_names.is_empty()
            {
                return lower_named_variant_constructor(
                    name, fields, *line, *column, variant, env, ctx,
                );
            }
            let struct_def = ctx.structs.get(name).ok_or_else(|| {
                Diagnostic::new("type", format!("undefined struct {name:?}"))
                    .with_span(*line, *column)
            })?;
            let mut field_defs = HashMap::new();
            for field in &struct_def.fields {
                field_defs.insert(field.name.clone(), field.ty.clone());
            }
            let mut lowered_fields = HashMap::new();
            for field in fields {
                let expected = field_defs.get(&field.name).ok_or_else(|| {
                    Diagnostic::new(
                        "type",
                        format!("struct {name:?} has no field {:?}", field.name),
                    )
                    .with_span(field.line, field.column)
                })?;
                if lowered_fields.contains_key(&field.name) {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "duplicate field {:?} in struct literal {name:?}",
                            field.name
                        ),
                    )
                    .with_span(field.line, field.column));
                }
                let lowered = lower_expr_with_expected(&field.expr, Some(expected), env, ctx)?;
                if lowered.ty() != expected {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "struct {name:?} field {:?} expects {expected}, got {}",
                            field.name,
                            lowered.ty()
                        ),
                    )
                    .with_span(field.line, field.column));
                }
                if !expected.is_copy() {
                    move_lowered_owner_value(&lowered, env)?;
                }
                lowered_fields.insert(
                    field.name.clone(),
                    StructFieldValue {
                        name: field.name.clone(),
                        expr: lowered,
                    },
                );
            }
            let mut ordered_fields = Vec::new();
            for field in &struct_def.fields {
                let lowered = lowered_fields.remove(&field.name).ok_or_else(|| {
                    Diagnostic::new(
                        "type",
                        format!("struct literal {name:?} is missing field {:?}", field.name),
                    )
                    .with_span(*line, *column)
                })?;
                ordered_fields.push(lowered);
            }
            Ok(Expr::StructLiteral {
                name: name.clone(),
                fields: ordered_fields,
                ty: Type::Struct(name.clone()),
            })
        }
        syntax::Expr::TupleLiteral {
            elements,
            line,
            column,
        } => {
            let mut lowered_elements = Vec::new();
            let mut element_tys = Vec::new();
            let mut temporary_borrows = Vec::new();
            for element in elements {
                let lowered = lower_expr(element, env, ctx)?;
                record_temporary_borrows(&lowered, env, ctx, &mut temporary_borrows)?;
                if !lowered.ty().is_copy() {
                    move_lowered_owner_value(&lowered, env)?;
                }
                element_tys.push(lowered.ty().clone());
                lowered_elements.push(lowered);
            }
            release_temporary_borrows(&temporary_borrows, env);
            if lowered_elements.len() < 2 {
                return Err(Diagnostic::new(
                    "type",
                    "tuple literals require at least two elements",
                )
                .with_span(*line, *column));
            }
            Ok(Expr::TupleLiteral {
                elements: lowered_elements,
                ty: Type::Tuple(element_tys),
            })
        }
        syntax::Expr::FieldAccess {
            base,
            field,
            line,
            column,
        } => {
            let lowered_base = lower_expr(base, env, ctx)?;
            let Type::Struct(struct_name) = lowered_base.ty() else {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "field access expects a struct value, got {}",
                        lowered_base.ty()
                    ),
                )
                .with_span(*line, *column));
            };
            let struct_def = ctx.structs.get(struct_name).ok_or_else(|| {
                Diagnostic::new(
                    "type",
                    format!("internal error: missing struct definition {struct_name:?}"),
                )
                .with_span(*line, *column)
            })?;
            let field_ty = struct_def
                .fields
                .iter()
                .find(|entry| entry.name == *field)
                .map(|entry| entry.ty.clone())
                .ok_or_else(|| {
                    Diagnostic::new(
                        "type",
                        format!("struct {struct_name:?} has no field {field:?}"),
                    )
                    .with_span(*line, *column)
                })?;
            if !field_ty.is_copy() {
                move_lowered_owner_value(&lowered_base, env)?;
            }
            Ok(Expr::FieldAccess {
                base: Box::new(lowered_base),
                field: field.clone(),
                ty: field_ty,
            })
        }
        syntax::Expr::TupleIndex {
            base,
            index,
            line,
            column,
        } => {
            let lowered_base = lower_expr(base, env, ctx)?;
            let Type::Tuple(element_tys) = lowered_base.ty() else {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "tuple index expects a tuple value, got {}",
                        lowered_base.ty()
                    ),
                )
                .with_span(*line, *column));
            };
            let element_ty = element_tys.get(*index).cloned().ok_or_else(|| {
                Diagnostic::new(
                    "type",
                    format!(
                        "tuple index {} is out of bounds for {}",
                        index,
                        lowered_base.ty()
                    ),
                )
                .with_span(*line, *column)
            })?;
            if !element_ty.is_copy() {
                move_lowered_owner_value(&lowered_base, env)?;
            }
            Ok(Expr::TupleIndex {
                base: Box::new(lowered_base),
                index: *index,
                ty: element_ty,
            })
        }
        syntax::Expr::MapLiteral {
            entries,
            line,
            column,
        } => {
            if entries.is_empty() {
                return Err(Diagnostic::new(
                    "type",
                    "empty map literals are not yet supported in stage1",
                )
                .with_span(*line, *column));
            }
            let mut lowered_entries = Vec::new();
            let mut key_ty = None;
            let mut value_ty = None;
            let mut temporary_borrows = Vec::new();
            for entry in entries {
                let lowered_key = lower_expr(&entry.key, env, ctx)?;
                let lowered_value = lower_expr(&entry.value, env, ctx)?;
                if let Some(expected) = key_ty.as_ref() {
                    if lowered_key.ty() != expected {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "map literal expects matching key types, got {expected} and {}",
                                lowered_key.ty()
                            ),
                        )
                        .with_span(entry.line, entry.column));
                    }
                } else {
                    key_ty = Some(lowered_key.ty().clone());
                }
                if let Some(expected) = value_ty.as_ref() {
                    if lowered_value.ty() != expected {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "map literal expects matching value types, got {expected} and {}",
                                lowered_value.ty()
                            ),
                        )
                        .with_span(entry.line, entry.column));
                    }
                } else {
                    value_ty = Some(lowered_value.ty().clone());
                }
                if !lowered_key.ty().supports_map_key() {
                    return Err(Diagnostic::new(
                        "type",
                        format!("map literal key type {} is not supported", lowered_key.ty()),
                    )
                    .with_span(entry.line, entry.column));
                }
                record_temporary_borrows(&lowered_key, env, ctx, &mut temporary_borrows)?;
                record_temporary_borrows(&lowered_value, env, ctx, &mut temporary_borrows)?;
                if !lowered_key.ty().is_copy() {
                    move_lowered_owner_value(&lowered_key, env)?;
                }
                if !lowered_value.ty().is_copy() {
                    move_lowered_owner_value(&lowered_value, env)?;
                }
                lowered_entries.push(MapEntry {
                    key: lowered_key,
                    value: lowered_value,
                });
            }
            release_temporary_borrows(&temporary_borrows, env);
            let key_ty = key_ty.expect("non-empty map literal must have a key type");
            let value_ty = value_ty.expect("non-empty map literal must have a value type");
            Ok(Expr::MapLiteral {
                entries: lowered_entries,
                ty: Type::Map(Box::new(key_ty), Box::new(value_ty)),
            })
        }
        syntax::Expr::ArrayLiteral {
            elements,
            line,
            column,
        } => {
            if elements.is_empty() {
                return Err(Diagnostic::new(
                    "type",
                    "empty array literals are not yet supported in stage1",
                )
                .with_span(*line, *column));
            }
            let mut lowered_elements = Vec::new();
            let mut element_ty = None;
            let mut temporary_borrows = Vec::new();
            for element in elements {
                let lowered = lower_expr(element, env, ctx)?;
                if let Some(expected) = element_ty.as_ref() {
                    if lowered.ty() != expected {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "array literal expects matching element types, got {expected} and {}",
                                lowered.ty()
                            ),
                        )
                        .with_span(element.line(), element.column()));
                    }
                } else {
                    element_ty = Some(lowered.ty().clone());
                }
                record_temporary_borrows(&lowered, env, ctx, &mut temporary_borrows)?;
                if !lowered.ty().is_copy() {
                    move_lowered_owner_value(&lowered, env)?;
                }
                lowered_elements.push(lowered);
            }
            release_temporary_borrows(&temporary_borrows, env);
            let element_ty = element_ty.expect("non-empty array literal must have an element type");
            Ok(Expr::ArrayLiteral {
                elements: lowered_elements,
                ty: Type::Array(Box::new(element_ty)),
            })
        }
        syntax::Expr::Slice {
            base,
            start,
            end,
            line,
            column,
        } => {
            let lowered_base = lower_expr(base, env, ctx)?;
            let element_ty = match lowered_base.ty() {
                Type::Array(element_ty) | Type::Slice(element_ty) | Type::MutSlice(element_ty) => {
                    (*element_ty.clone()).clone()
                }
                _ => {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "slice expects an array or slice value, got {}",
                            lowered_base.ty()
                        ),
                    )
                    .with_span(*line, *column));
                }
            };
            if !is_borrowable_slice_base(&lowered_base) {
                return Err(Diagnostic::new(
                    "type",
                    "borrowed slices currently require a named array, field, tuple field, or slice value",
                )
                .with_span(*line, *column));
            }
            let lowered_start = if let Some(start) = start {
                let lowered = lower_expr(start, env, ctx)?;
                if lowered.ty() != &Type::Int {
                    return Err(Diagnostic::new(
                        "type",
                        format!("array slice start expects int, got {}", lowered.ty()),
                    )
                    .with_span(start.line(), start.column()));
                }
                Some(Box::new(lowered))
            } else {
                None
            };
            let lowered_end = if let Some(end) = end {
                let lowered = lower_expr(end, env, ctx)?;
                if lowered.ty() != &Type::Int {
                    return Err(Diagnostic::new(
                        "type",
                        format!("array slice end expects int, got {}", lowered.ty()),
                    )
                    .with_span(end.line(), end.column()));
                }
                Some(Box::new(lowered))
            } else {
                None
            };
            let ty = match lowered_base.ty() {
                Type::MutSlice(_) => Type::MutSlice(Box::new(element_ty)),
                _ => Type::Slice(Box::new(element_ty)),
            };
            Ok(Expr::Slice {
                base: Box::new(lowered_base),
                start: lowered_start,
                end: lowered_end,
                ty,
            })
        }
        syntax::Expr::Index {
            base,
            index,
            line,
            column,
        } => {
            let lowered_base = lower_expr(base, env, ctx)?;
            let lowered_index = lower_expr(index, env, ctx)?;
            let result_ty = match lowered_base.ty() {
                Type::Array(element_ty) => {
                    if lowered_index.ty() != &Type::Int {
                        return Err(Diagnostic::new(
                            "type",
                            format!("array index expects int, got {}", lowered_index.ty()),
                        )
                        .with_span(*line, *column));
                    }
                    let element_ty = (*element_ty.clone()).clone();
                    if !element_ty.is_copy() {
                        move_lowered_owner_value(&lowered_base, env)?;
                    }
                    element_ty
                }
                Type::Slice(element_ty) | Type::MutSlice(element_ty) => {
                    if lowered_index.ty() != &Type::Int {
                        return Err(Diagnostic::new(
                            "type",
                            format!("slice index expects int, got {}", lowered_index.ty()),
                        )
                        .with_span(*line, *column));
                    }
                    let element_ty = (*element_ty.clone()).clone();
                    if !element_ty.is_copy() {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "borrowed slice indexing requires a Copy element type, got {element_ty}"
                            ),
                        )
                        .with_span(*line, *column));
                    }
                    element_ty
                }
                Type::Map(key_ty, value_ty) => {
                    if lowered_index.ty() != key_ty.as_ref() {
                        return Err(Diagnostic::new(
                            "type",
                            format!(
                                "map index expects key type {}, got {}",
                                key_ty,
                                lowered_index.ty()
                            ),
                        )
                        .with_span(*line, *column));
                    }
                    let value_ty = (*value_ty.clone()).clone();
                    if !value_ty.is_copy() {
                        move_lowered_owner_value(&lowered_base, env)?;
                    }
                    value_ty
                }
                _ => {
                    return Err(Diagnostic::new(
                        "type",
                        format!(
                            "index expects an array or map value, got {}",
                            lowered_base.ty()
                        ),
                    )
                    .with_span(*line, *column));
                }
            };
            Ok(Expr::Index {
                base: Box::new(lowered_base),
                index: Box::new(lowered_index),
                ty: result_ty,
            })
        }
    }
}

fn require_capability(
    capabilities: &CapabilityConfig,
    kind: CapabilityKind,
    intrinsic_name: &str,
    line: usize,
    column: usize,
) -> Result<(), Diagnostic> {
    if capabilities.enabled(kind) {
        return Ok(());
    }
    Err(Diagnostic::new(
        "capability",
        format!(
            "call to {intrinsic_name:?} requires [capabilities].{} = true",
            kind.name()
        ),
    )
    .with_span(line, column))
}

fn lower_variant_constructor(
    name: &str,
    args: &[syntax::Expr],
    line: usize,
    column: usize,
    variant: &VariantInfo,
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Result<Expr, Diagnostic> {
    if !variant.payload_names.is_empty() {
        return Err(Diagnostic::new(
            "type",
            format!("enum variant {name:?} requires named payload fields"),
        )
        .with_span(line, column));
    }
    if variant.payload_tys.is_empty() {
        return Err(Diagnostic::new(
            "type",
            format!("enum variant {name:?} does not take arguments"),
        )
        .with_span(line, column));
    }
    if args.len() != variant.payload_tys.len() {
        return Err(Diagnostic::new(
            "type",
            format!(
                "enum variant {name:?} expects {} arguments, got {}",
                variant.payload_tys.len(),
                args.len()
            ),
        )
        .with_span(line, column));
    }
    let mut lowered_payloads = Vec::new();
    for (arg, expected) in args.iter().zip(variant.payload_tys.iter()) {
        let lowered = lower_expr_with_expected(arg, Some(expected), env, ctx)?;
        if lowered.ty() != expected {
            return Err(Diagnostic::new(
                "type",
                format!(
                    "enum variant {name:?} expects payload type {expected}, got {}",
                    lowered.ty()
                ),
            )
            .with_span(arg.line(), arg.column()));
        }
        if !expected.is_copy() {
            move_lowered_value(&lowered, env)?;
        }
        lowered_payloads.push(lowered);
    }
    Ok(Expr::EnumVariant {
        enum_name: variant.enum_name.clone(),
        variant: name.to_string(),
        field_names: Vec::new(),
        payloads: lowered_payloads,
        ty: Type::Enum(variant.enum_name.clone()),
    })
}

fn lower_named_variant_constructor(
    name: &str,
    fields: &[syntax::StructFieldValue],
    line: usize,
    column: usize,
    variant: &VariantInfo,
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Result<Expr, Diagnostic> {
    let mut lowered_fields = HashMap::new();
    for field in fields {
        let Some(position) = variant
            .payload_names
            .iter()
            .position(|payload_name| payload_name == &field.name)
        else {
            return Err(Diagnostic::new(
                "type",
                format!(
                    "enum variant {name:?} has no named payload {:?}",
                    field.name
                ),
            )
            .with_span(field.line, field.column));
        };
        if lowered_fields.contains_key(&field.name) {
            return Err(Diagnostic::new(
                "type",
                format!(
                    "duplicate named payload {:?} in enum variant literal {name:?}",
                    field.name
                ),
            )
            .with_span(field.line, field.column));
        }
        let expected = &variant.payload_tys[position];
        let lowered = lower_expr_with_expected(&field.expr, Some(expected), env, ctx)?;
        if lowered.ty() != expected {
            return Err(Diagnostic::new(
                "type",
                format!(
                    "enum variant {name:?} payload {:?} expects {expected}, got {}",
                    field.name,
                    lowered.ty()
                ),
            )
            .with_span(field.line, field.column));
        }
        if !expected.is_copy() {
            move_lowered_owner_value(&lowered, env)?;
        }
        lowered_fields.insert(field.name.clone(), lowered);
    }
    let mut ordered_payloads = Vec::new();
    for payload_name in &variant.payload_names {
        let lowered = lowered_fields.remove(payload_name).ok_or_else(|| {
            Diagnostic::new(
                "type",
                format!(
                    "enum variant literal {name:?} is missing named payload {:?}",
                    payload_name
                ),
            )
            .with_span(line, column)
        })?;
        ordered_payloads.push(lowered);
    }
    Ok(Expr::EnumVariant {
        enum_name: variant.enum_name.clone(),
        variant: name.to_string(),
        field_names: variant.payload_names.clone(),
        payloads: ordered_payloads,
        ty: Type::Enum(variant.enum_name.clone()),
    })
}

fn move_lowered_value(expr: &Expr, env: &mut HashMap<String, Binding>) -> Result<(), Diagnostic> {
    let Expr::VarRef { name, .. } = expr else {
        return Ok(());
    };
    let binding = env.get_mut(name).ok_or_else(|| {
        Diagnostic::new(
            "type",
            format!("internal error: missing binding for moved value {name:?}"),
        )
    })?;
    if binding.active_borrow_count > 0 {
        return Err(Diagnostic::new(
            "ownership",
            format!("cannot move value {name:?} while borrowed slices are still live"),
        ));
    }
    if binding.moved {
        return Err(Diagnostic::new(
            "ownership",
            format!("use of moved value {name:?}"),
        ));
    }
    binding.moved = true;
    Ok(())
}

fn move_lowered_owner_value(
    expr: &Expr,
    env: &mut HashMap<String, Binding>,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::VarRef { .. } => move_lowered_value(expr, env),
        Expr::FieldAccess { base, .. } => move_lowered_owner_value(base, env),
        Expr::TupleIndex { base, .. } => move_lowered_owner_value(base, env),
        Expr::Index { base, .. } => move_lowered_owner_value(base, env),
        _ => Ok(()),
    }
}

fn is_borrowable_slice_base(expr: &Expr) -> bool {
    match expr {
        Expr::VarRef { .. } => true,
        Expr::FieldAccess { base, .. } => is_borrowable_slice_base(base),
        Expr::TupleIndex { base, .. } => is_borrowable_slice_base(base),
        Expr::Slice { .. } => true,
        _ => false,
    }
}

fn binding_borrow_origin(
    ty: &Type,
    param_name: Option<&str>,
    structs: &HashMap<String, StructDef>,
    enums: &HashMap<String, EnumDef>,
) -> Option<BorrowOrigin> {
    if !contains_borrowed_slice_type(ty, structs, enums) {
        return None;
    }
    Some(match param_name {
        Some(name) => BorrowOrigin::Param(name.to_string()),
        None => BorrowOrigin::Local,
    })
}

fn binding_borrow_origin_from_expr(
    ty: &Type,
    expr: &Expr,
    env: &HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Option<BorrowOrigin> {
    if !contains_borrowed_slice_type(ty, ctx.structs, ctx.enums) {
        return None;
    }
    expr_borrow_origin(expr, env, ctx)
}

fn binding_borrowed_owners_from_expr(
    ty: &Type,
    expr: &Expr,
    env: &HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> HashSet<String> {
    if !contains_borrowed_slice_type(ty, ctx.structs, ctx.enums) {
        return HashSet::new();
    }
    expr_borrowed_owners(expr, env, ctx)
}

fn expr_borrow_origin(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Option<BorrowOrigin> {
    if !contains_borrowed_slice_type(expr.ty(), ctx.structs, ctx.enums) {
        return None;
    }
    match expr {
        Expr::VarRef { name, .. } => env
            .get(name)
            .and_then(|binding| binding.borrow_origin.clone()),
        Expr::Slice { base, .. } => match base.ty() {
            Type::Slice(_) | Type::MutSlice(_) => expr_borrow_origin(base, env, ctx),
            Type::Array(_) => Some(BorrowOrigin::Local),
            _ => Some(BorrowOrigin::Local),
        },
        Expr::Call { name, args, .. } => ctx
            .functions
            .get(name)
            .map(|signature| {
                merge_borrow_origins(
                    signature
                        .borrow_return_params
                        .iter()
                        .map(|index| expr_borrow_origin(&args[*index], env, ctx)),
                )
            })
            .flatten(),
        Expr::TupleLiteral { elements, .. } => merge_borrow_origins(
            elements
                .iter()
                .map(|element| expr_borrow_origin(element, env, ctx)),
        ),
        Expr::TupleIndex { base, .. } => expr_borrow_origin(base, env, ctx),
        Expr::MapLiteral { entries, .. } => {
            merge_borrow_origins(entries.iter().flat_map(|entry| {
                [
                    expr_borrow_origin(&entry.key, env, ctx),
                    expr_borrow_origin(&entry.value, env, ctx),
                ]
            }))
        }
        Expr::EnumVariant { payloads, .. } => merge_borrow_origins(
            payloads
                .iter()
                .map(|payload| expr_borrow_origin(payload, env, ctx)),
        ),
        Expr::FieldAccess { base, .. } => expr_borrow_origin(base, env, ctx),
        Expr::ArrayLiteral { elements, .. } => merge_borrow_origins(
            elements
                .iter()
                .map(|element| expr_borrow_origin(element, env, ctx)),
        ),
        Expr::StructLiteral { fields, .. } => merge_borrow_origins(
            fields
                .iter()
                .map(|field| expr_borrow_origin(&field.expr, env, ctx)),
        ),
        Expr::Index { base, .. } => expr_borrow_origin(base, env, ctx),
        Expr::Literal { .. } | Expr::BinaryAdd { .. } | Expr::BinaryCompare { .. } => None,
    }
}

fn merge_borrow_origins<I>(origins: I) -> Option<BorrowOrigin>
where
    I: IntoIterator<Item = Option<BorrowOrigin>>,
{
    let mut merged = None;
    for origin in origins.into_iter().flatten() {
        match &merged {
            None => merged = Some(origin),
            Some(existing) if existing == &origin => {}
            Some(_) => return Some(BorrowOrigin::Local),
        }
    }
    merged
}

fn match_binding_payload_expr<'a>(
    matched_expr: &'a Expr,
    variant_name: &str,
    binding_name: &str,
    binding_index: usize,
) -> Option<&'a Expr> {
    let Expr::EnumVariant {
        variant,
        field_names,
        payloads,
        ..
    } = matched_expr
    else {
        return None;
    };
    if variant != variant_name {
        return None;
    }
    if field_names.is_empty() {
        return payloads.get(binding_index);
    }
    field_names
        .iter()
        .position(|field_name| field_name == binding_name)
        .and_then(|index| payloads.get(index))
}

fn match_binding_borrow_origin(
    matched_expr: &Expr,
    variant_name: &str,
    binding_name: &str,
    binding_index: usize,
    payload_ty: &Type,
    env: &HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Option<BorrowOrigin> {
    if !contains_borrowed_slice_type(payload_ty, ctx.structs, ctx.enums) {
        return None;
    }
    if let Some(payload_expr) =
        match_binding_payload_expr(matched_expr, variant_name, binding_name, binding_index)
    {
        return expr_borrow_origin(payload_expr, env, ctx);
    }
    expr_borrow_origin(matched_expr, env, ctx)
}

fn match_binding_borrowed_owners(
    matched_expr: &Expr,
    variant_name: &str,
    binding_name: &str,
    binding_index: usize,
    payload_ty: &Type,
    env: &HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> HashSet<String> {
    if !contains_borrowed_slice_type(payload_ty, ctx.structs, ctx.enums) {
        return HashSet::new();
    }
    if let Some(payload_expr) =
        match_binding_payload_expr(matched_expr, variant_name, binding_name, binding_index)
    {
        return expr_borrowed_owners(payload_expr, env, ctx);
    }
    expr_borrowed_owners(matched_expr, env, ctx)
}

fn expr_borrowed_owners(
    expr: &Expr,
    env: &HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> HashSet<String> {
    if !contains_borrowed_slice_type(expr.ty(), ctx.structs, ctx.enums) {
        return HashSet::new();
    }
    match expr {
        Expr::VarRef { name, .. } => env
            .get(name)
            .map(|binding| binding.borrowed_owners.clone())
            .unwrap_or_default(),
        Expr::Slice { base, .. } => match base.ty() {
            Type::Slice(_) | Type::MutSlice(_) => expr_borrowed_owners(base, env, ctx),
            Type::Array(_) => owned_borrow_root(base).into_iter().collect(),
            _ => HashSet::new(),
        },
        Expr::Call { name, args, .. } => ctx
            .functions
            .get(name)
            .map(|signature| {
                let mut owners = HashSet::new();
                for index in &signature.borrow_return_params {
                    owners.extend(expr_borrowed_owners(&args[*index], env, ctx));
                }
                owners
            })
            .unwrap_or_default(),
        Expr::TupleLiteral { elements, .. } => collect_expr_borrowed_owners(elements, env, ctx),
        Expr::TupleIndex { base, .. } => expr_borrowed_owners(base, env, ctx),
        Expr::MapLiteral { entries, .. } => {
            let mut owners = HashSet::new();
            for entry in entries {
                owners.extend(expr_borrowed_owners(&entry.key, env, ctx));
                owners.extend(expr_borrowed_owners(&entry.value, env, ctx));
            }
            owners
        }
        Expr::ArrayLiteral { elements, .. } => collect_expr_borrowed_owners(elements, env, ctx),
        Expr::EnumVariant { payloads, .. } => collect_expr_borrowed_owners(payloads, env, ctx),
        Expr::FieldAccess { base, .. } => expr_borrowed_owners(base, env, ctx),
        Expr::Index { base, .. } => expr_borrowed_owners(base, env, ctx),
        Expr::Literal { .. } | Expr::BinaryAdd { .. } | Expr::BinaryCompare { .. } => {
            HashSet::new()
        }
        Expr::StructLiteral { fields, .. } => {
            let mut owners = HashSet::new();
            for field in fields {
                owners.extend(expr_borrowed_owners(&field.expr, env, ctx));
            }
            owners
        }
    }
}

fn collect_expr_borrowed_owners(
    exprs: &[Expr],
    env: &HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> HashSet<String> {
    let mut owners = HashSet::new();
    for expr in exprs {
        owners.extend(expr_borrowed_owners(expr, env, ctx));
    }
    owners
}

fn owned_borrow_root(expr: &Expr) -> Option<String> {
    match expr {
        Expr::VarRef { name, ty } if !matches!(ty, Type::Slice(_) | Type::MutSlice(_)) => {
            Some(name.clone())
        }
        Expr::FieldAccess { base, .. } => owned_borrow_root(base),
        Expr::TupleIndex { base, .. } => owned_borrow_root(base),
        _ => None,
    }
}

fn contains_borrowed_slice_type(
    ty: &Type,
    structs: &HashMap<String, StructDef>,
    enums: &HashMap<String, EnumDef>,
) -> bool {
    contains_borrowed_slice_type_inner(ty, structs, enums, &mut HashSet::new(), &mut HashSet::new())
}

fn contains_borrowed_slice_type_inner(
    ty: &Type,
    structs: &HashMap<String, StructDef>,
    enums: &HashMap<String, EnumDef>,
    visiting_structs: &mut HashSet<String>,
    visiting_enums: &mut HashSet<String>,
) -> bool {
    match ty {
        Type::Slice(_) | Type::MutSlice(_) => true,
        Type::Option(inner) => contains_borrowed_slice_type_inner(
            inner,
            structs,
            enums,
            visiting_structs,
            visiting_enums,
        ),
        Type::Result(ok, err) => {
            contains_borrowed_slice_type_inner(ok, structs, enums, visiting_structs, visiting_enums)
                || contains_borrowed_slice_type_inner(
                    err,
                    structs,
                    enums,
                    visiting_structs,
                    visiting_enums,
                )
        }
        Type::Tuple(elements) => elements.iter().any(|element| {
            contains_borrowed_slice_type_inner(
                element,
                structs,
                enums,
                visiting_structs,
                visiting_enums,
            )
        }),
        Type::Map(key, value) => {
            contains_borrowed_slice_type_inner(
                key,
                structs,
                enums,
                visiting_structs,
                visiting_enums,
            ) || contains_borrowed_slice_type_inner(
                value,
                structs,
                enums,
                visiting_structs,
                visiting_enums,
            )
        }
        Type::Array(inner) => contains_borrowed_slice_type_inner(
            inner,
            structs,
            enums,
            visiting_structs,
            visiting_enums,
        ),
        Type::Struct(name) => {
            if !visiting_structs.insert(name.clone()) {
                return false;
            }
            let contains = structs.get(name).is_some_and(|struct_def| {
                struct_def.fields.iter().any(|field| {
                    contains_borrowed_slice_type_inner(
                        &field.ty,
                        structs,
                        enums,
                        visiting_structs,
                        visiting_enums,
                    )
                })
            });
            visiting_structs.remove(name);
            contains
        }
        Type::Enum(name) => {
            if !visiting_enums.insert(name.clone()) {
                return false;
            }
            let contains = enums.get(name).is_some_and(|enum_def| {
                enum_def.variants.iter().any(|variant| {
                    variant.payload_tys.iter().any(|payload_ty| {
                        contains_borrowed_slice_type_inner(
                            payload_ty,
                            structs,
                            enums,
                            visiting_structs,
                            visiting_enums,
                        )
                    })
                })
            });
            visiting_enums.remove(name);
            contains
        }
        Type::Int | Type::Bool | Type::String => false,
    }
}

fn classify_borrow_return(
    params: &[Type],
    return_ty: &Type,
    structs: &HashMap<String, StructDef>,
    enums: &HashMap<String, EnumDef>,
    line: usize,
    column: usize,
) -> Result<Vec<usize>, Diagnostic> {
    if !contains_borrowed_slice_type(return_ty, structs, enums) {
        return Ok(Vec::new());
    }
    let matches = params
        .iter()
        .enumerate()
        .filter_map(|(index, ty)| contains_borrowed_slice_type(ty, structs, enums).then_some(index))
        .collect::<Vec<_>>();
    if matches.is_empty() {
        return Err(Diagnostic::new(
            "type",
            "borrowed return functions must take at least one borrowed parameter in stage1",
        )
        .with_span(line, column));
    }
    Ok(matches)
}

fn increment_active_borrows(
    owner_names: &HashSet<String>,
    env: &mut HashMap<String, Binding>,
) -> Result<(), Diagnostic> {
    for owner_name in owner_names {
        let binding = env.get_mut(owner_name).ok_or_else(|| {
            Diagnostic::new(
                "type",
                format!("internal error: missing borrow owner {owner_name:?}"),
            )
        })?;
        binding.active_borrow_count += 1;
    }
    Ok(())
}

fn record_temporary_borrows(
    expr: &Expr,
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
    temporary_borrows: &mut Vec<HashSet<String>>,
) -> Result<(), Diagnostic> {
    let owners = expr_borrowed_owners(expr, env, ctx);
    increment_active_borrows(&owners, env)?;
    temporary_borrows.push(owners);
    Ok(())
}

fn release_temporary_borrows(
    temporary_borrows: &[HashSet<String>],
    env: &mut HashMap<String, Binding>,
) {
    for owner_names in temporary_borrows.iter().rev() {
        release_active_borrow_owners(owner_names, env);
    }
}

fn release_active_borrow_owners(owner_names: &HashSet<String>, env: &mut HashMap<String, Binding>) {
    for owner_name in owner_names {
        decrement_active_borrow(owner_name, env);
    }
}

fn decrement_active_borrow(owner_name: &str, env: &mut HashMap<String, Binding>) {
    let Some(binding) = env.get_mut(owner_name) else {
        return;
    };
    binding.active_borrow_count = binding.active_borrow_count.saturating_sub(1);
}

fn release_scope_borrows(env: &mut HashMap<String, Binding>, scope_names: &HashSet<String>) {
    let released = env
        .keys()
        .filter(|name| !scope_names.contains(*name))
        .cloned()
        .collect::<Vec<_>>();
    for name in &released {
        let owner_names = env
            .get(name)
            .map(|binding| binding.borrowed_owners.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for owner_name in owner_names {
            decrement_active_borrow(&owner_name, env);
        }
    }
    for name in released {
        env.remove(&name);
    }
}

fn merge_borrow_count(
    before: usize,
    then_returns: bool,
    then_after: Option<usize>,
    else_returns: bool,
    else_after: Option<usize>,
) -> usize {
    let then_count = if then_returns {
        before
    } else {
        then_after.unwrap_or(before)
    };
    let else_count = if else_returns {
        before
    } else {
        else_after.unwrap_or(before)
    };
    then_count.max(else_count)
}

fn lower_literal(literal: &syntax::Literal) -> Expr {
    match literal {
        syntax::Literal::Int(value) => Expr::Literal {
            ty: Type::Int,
            value: LiteralValue::Int(*value),
        },
        syntax::Literal::Bool(value) => Expr::Literal {
            ty: Type::Bool,
            value: LiteralValue::Bool(*value),
        },
        syntax::Literal::String(value) => Expr::Literal {
            ty: Type::String,
            value: LiteralValue::String(value.clone()),
        },
    }
}

fn lower_type<T, U>(
    ty: &syntax::TypeName,
    structs: &HashMap<String, T>,
    enums: &HashMap<String, U>,
    line: usize,
    column: usize,
) -> Result<Type, Diagnostic> {
    match ty {
        syntax::TypeName::Int => Ok(Type::Int),
        syntax::TypeName::Bool => Ok(Type::Bool),
        syntax::TypeName::String => Ok(Type::String),
        syntax::TypeName::Named(name) => {
            if structs.contains_key(name) {
                return Ok(Type::Struct(name.clone()));
            }
            if enums.contains_key(name) {
                return Ok(Type::Enum(name.clone()));
            }
            Err(Diagnostic::new("type", format!("unknown type {name:?}")).with_span(line, column))
        }
        syntax::TypeName::Slice(inner) => Ok(Type::Slice(Box::new(lower_type(
            inner, structs, enums, line, column,
        )?))),
        syntax::TypeName::MutSlice(inner) => Ok(Type::MutSlice(Box::new(lower_type(
            inner, structs, enums, line, column,
        )?))),
        syntax::TypeName::Option(inner) => Ok(Type::Option(Box::new(lower_type(
            inner, structs, enums, line, column,
        )?))),
        syntax::TypeName::Result(ok, err) => Ok(Type::Result(
            Box::new(lower_type(ok, structs, enums, line, column)?),
            Box::new(lower_type(err, structs, enums, line, column)?),
        )),
        syntax::TypeName::Tuple(elements) => Ok(Type::Tuple(
            elements
                .iter()
                .map(|element| lower_type(element, structs, enums, line, column))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        syntax::TypeName::Map(key, value) => {
            let key = lower_type(key, structs, enums, line, column)?;
            if !key.supports_map_key() {
                return Err(Diagnostic::new(
                    "type",
                    format!("map key type {key} is not supported"),
                )
                .with_span(line, column));
            }
            Ok(Type::Map(
                Box::new(key),
                Box::new(lower_type(value, structs, enums, line, column)?),
            ))
        }
        syntax::TypeName::Array(inner) => Ok(Type::Array(Box::new(lower_type(
            inner, structs, enums, line, column,
        )?))),
    }
}

fn lower_compare_op(op: syntax::CompareOp) -> CompareOp {
    match op {
        syntax::CompareOp::Eq => CompareOp::Eq,
        syntax::CompareOp::Ne => CompareOp::Ne,
        syntax::CompareOp::Lt => CompareOp::Lt,
        syntax::CompareOp::Le => CompareOp::Le,
        syntax::CompareOp::Gt => CompareOp::Gt,
        syntax::CompareOp::Ge => CompareOp::Ge,
    }
}

fn static_bool_value(expr: &Expr) -> Option<bool> {
    match expr {
        Expr::Literal {
            value: LiteralValue::Bool(value),
            ..
        } => Some(*value),
        Expr::BinaryCompare { op, lhs, rhs, .. } => {
            let lhs = literal_value(lhs)?;
            let rhs = literal_value(rhs)?;
            Some(match (lhs, rhs) {
                (LiteralValue::Int(lhs), LiteralValue::Int(rhs)) => match op {
                    CompareOp::Eq => lhs == rhs,
                    CompareOp::Ne => lhs != rhs,
                    CompareOp::Lt => lhs < rhs,
                    CompareOp::Le => lhs <= rhs,
                    CompareOp::Gt => lhs > rhs,
                    CompareOp::Ge => lhs >= rhs,
                },
                (LiteralValue::Bool(lhs), LiteralValue::Bool(rhs)) => match op {
                    CompareOp::Eq => lhs == rhs,
                    CompareOp::Ne => lhs != rhs,
                    _ => return None,
                },
                (LiteralValue::String(lhs), LiteralValue::String(rhs)) => match op {
                    CompareOp::Eq => lhs == rhs,
                    CompareOp::Ne => lhs != rhs,
                    _ => return None,
                },
                _ => return None,
            })
        }
        _ => None,
    }
}

fn literal_value(expr: &Expr) -> Option<&LiteralValue> {
    match expr {
        Expr::Literal { value, .. } => Some(value),
        _ => None,
    }
}

impl Expr {
    pub fn ty(&self) -> &Type {
        match self {
            Expr::Literal { ty, .. } => ty,
            Expr::VarRef { ty, .. } => ty,
            Expr::Call { ty, .. } => ty,
            Expr::BinaryAdd { ty, .. } => ty,
            Expr::BinaryCompare { ty, .. } => ty,
            Expr::StructLiteral { ty, .. } => ty,
            Expr::FieldAccess { ty, .. } => ty,
            Expr::TupleLiteral { ty, .. } => ty,
            Expr::TupleIndex { ty, .. } => ty,
            Expr::MapLiteral { ty, .. } => ty,
            Expr::EnumVariant { ty, .. } => ty,
            Expr::ArrayLiteral { ty, .. } => ty,
            Expr::Slice { ty, .. } => ty,
            Expr::Index { ty, .. } => ty,
        }
    }
}

impl Stmt {
    fn always_returns(&self) -> bool {
        match self {
            Stmt::Return(_) => true,
            Stmt::If {
                cond,
                then_block,
                else_block: Some(else_block),
            } => match static_bool_value(cond) {
                Some(true) => block_always_returns(then_block),
                Some(false) => block_always_returns(else_block),
                None => block_always_returns(then_block) && block_always_returns(else_block),
            },
            Stmt::If {
                cond,
                then_block,
                else_block: None,
            } => {
                static_bool_value(cond).is_some_and(|value| value)
                    && block_always_returns(then_block)
            }
            Stmt::Match { arms, .. } => arms.iter().all(|arm| block_always_returns(&arm.body)),
            _ => false,
        }
    }
}

fn block_always_returns(block: &[Stmt]) -> bool {
    block.last().is_some_and(Stmt::always_returns)
}

impl syntax::Stmt {
    fn line(&self) -> usize {
        match self {
            syntax::Stmt::Let { line, .. }
            | syntax::Stmt::Print { line, .. }
            | syntax::Stmt::If { line, .. }
            | syntax::Stmt::While { line, .. }
            | syntax::Stmt::Match { line, .. }
            | syntax::Stmt::Return { line, .. } => *line,
        }
    }

    fn column(&self) -> usize {
        match self {
            syntax::Stmt::Let { column, .. }
            | syntax::Stmt::Print { column, .. }
            | syntax::Stmt::If { column, .. }
            | syntax::Stmt::While { column, .. }
            | syntax::Stmt::Match { column, .. }
            | syntax::Stmt::Return { column, .. } => *column,
        }
    }
}

impl syntax::Expr {
    fn line(&self) -> usize {
        match self {
            syntax::Expr::Literal(_) => 1,
            syntax::Expr::VarRef { line, .. }
            | syntax::Expr::Call { line, .. }
            | syntax::Expr::BinaryAdd { line, .. }
            | syntax::Expr::BinaryCompare { line, .. }
            | syntax::Expr::StructLiteral { line, .. }
            | syntax::Expr::FieldAccess { line, .. }
            | syntax::Expr::TupleLiteral { line, .. }
            | syntax::Expr::TupleIndex { line, .. }
            | syntax::Expr::MapLiteral { line, .. }
            | syntax::Expr::ArrayLiteral { line, .. }
            | syntax::Expr::Slice { line, .. }
            | syntax::Expr::Index { line, .. } => *line,
        }
    }

    fn column(&self) -> usize {
        match self {
            syntax::Expr::Literal(_) => 1,
            syntax::Expr::VarRef { column, .. }
            | syntax::Expr::Call { column, .. }
            | syntax::Expr::BinaryAdd { column, .. }
            | syntax::Expr::BinaryCompare { column, .. }
            | syntax::Expr::StructLiteral { column, .. }
            | syntax::Expr::FieldAccess { column, .. }
            | syntax::Expr::TupleLiteral { column, .. }
            | syntax::Expr::TupleIndex { column, .. }
            | syntax::Expr::MapLiteral { column, .. }
            | syntax::Expr::ArrayLiteral { column, .. }
            | syntax::Expr::Slice { column, .. }
            | syntax::Expr::Index { column, .. } => *column,
        }
    }
}

impl CompareOp {
    pub fn lexeme(self) -> &'static str {
        match self {
            CompareOp::Eq => "==",
            CompareOp::Ne => "!=",
            CompareOp::Lt => "<",
            CompareOp::Le => "<=",
            CompareOp::Gt => ">",
            CompareOp::Ge => ">=",
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "string"),
            Type::Struct(name) => write!(f, "{name}"),
            Type::Enum(name) => write!(f, "{name}"),
            Type::Slice(inner) => write!(f, "&[{inner}]"),
            Type::MutSlice(inner) => write!(f, "&mut [{inner}]"),
            Type::Option(inner) => write!(f, "Option<{inner}>"),
            Type::Result(ok, err) => write!(f, "Result<{ok}, {err}>"),
            Type::Tuple(elements) => write!(
                f,
                "({})",
                elements
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Type::Map(key, value) => write!(f, "{{{key}: {value}}}"),
            Type::Array(inner) => write!(f, "[{inner}]"),
        }
    }
}
