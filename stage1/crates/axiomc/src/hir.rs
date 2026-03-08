use crate::diagnostics::Diagnostic;
use crate::syntax;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Program {
    pub structs: Vec<StructDef>,
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
    Return(Expr),
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
    ArrayLiteral {
        elements: Vec<Expr>,
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
}

#[derive(Debug, Clone)]
struct FunctionSig {
    params: Vec<Type>,
    return_ty: Type,
}

struct LowerContext<'a> {
    structs: &'a HashMap<String, StructDef>,
    functions: &'a HashMap<String, FunctionSig>,
    current_return: Option<Type>,
}

pub fn lower(program: &syntax::Program) -> Result<Program, Diagnostic> {
    let structs = collect_struct_definitions(&program.structs)?;
    let functions = collect_function_signatures(&program.functions, &structs)?;
    let mut lowered_structs = structs.values().cloned().collect::<Vec<_>>();
    lowered_structs.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
    let mut lowered_functions = Vec::new();
    for function in &program.functions {
        lowered_functions.push(lower_function(function, &structs, &functions)?);
    }
    let ctx = LowerContext {
        structs: &structs,
        functions: &functions,
        current_return: None,
    };
    let mut env = HashMap::new();
    let (stmts, _, _) = lower_block(&program.stmts, &mut env, &ctx)?;
    Ok(Program {
        structs: lowered_structs,
        functions: lowered_functions,
        stmts,
    })
}

impl Type {
    pub fn is_copy(&self) -> bool {
        matches!(self, Type::Int | Type::Bool)
    }
}

fn collect_struct_definitions(
    structs: &[syntax::StructDecl],
) -> Result<HashMap<String, StructDef>, Diagnostic> {
    let mut names = HashMap::new();
    for struct_decl in structs {
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
            let ty = lower_type(&field.ty, &names, field.line, field.column)?;
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

fn collect_function_signatures(
    functions: &[syntax::Function],
    structs: &HashMap<String, StructDef>,
) -> Result<HashMap<String, FunctionSig>, Diagnostic> {
    let mut signatures = HashMap::new();
    for function in functions {
        let return_ty = lower_type(&function.return_ty, structs, function.line, function.column)?;
        let mut params = Vec::new();
        for param in &function.params {
            params.push(lower_type(&param.ty, structs, param.line, param.column)?);
        }
        if signatures
            .insert(function.name.clone(), FunctionSig { params, return_ty })
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
    functions: &HashMap<String, FunctionSig>,
) -> Result<Function, Diagnostic> {
    let return_ty = lower_type(&function.return_ty, structs, function.line, function.column)?;
    let ctx = LowerContext {
        structs,
        functions,
        current_return: Some(return_ty.clone()),
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
        let ty = lower_type(&param.ty, structs, param.line, param.column)?;
        env.insert(
            param.name.clone(),
            Binding {
                ty: ty.clone(),
                moved: false,
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
    Ok((lowered, env.clone(), guaranteed_return))
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
            let expected = lower_type(ty, ctx.structs, *line, *column)?;
            let lowered_expr = lower_expr(expr, env, ctx)?;
            let actual = lowered_expr.ty().clone();
            if actual != expected {
                return Err(Diagnostic::new(
                    "type",
                    format!("let binding {name:?} expects {expected}, got {actual}"),
                )
                .with_span(*line, *column));
            }
            if matches!(expr, syntax::Expr::VarRef { .. }) && !actual.is_copy() {
                move_value(expr, env)?;
            }
            env.insert(
                name.clone(),
                Binding {
                    ty: expected.clone(),
                    moved: false,
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
            let before = env.clone();
            let mut body_env = before.clone();
            let (body, body_after, body_returns) = lower_block(body, &mut body_env, ctx)?;
            merge_loop_state(env, &before, &body_after, body_returns);
            Ok(Stmt::While {
                cond: lowered_cond,
                body,
            })
        }
        syntax::Stmt::Return { expr, line, column } => {
            let Some(expected) = ctx.current_return.as_ref() else {
                return Err(
                    Diagnostic::new("control", "return is only valid inside a function")
                        .with_span(*line, *column),
                );
            };
            let lowered_expr = lower_expr(expr, env, ctx)?;
            if lowered_expr.ty() != expected {
                return Err(Diagnostic::new(
                    "type",
                    format!("return expects {expected}, got {}", lowered_expr.ty()),
                )
                .with_span(*line, *column));
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
    env.clear();
    for (name, binding) in before {
        let body_moved = if body_returns {
            binding.moved
        } else {
            body_after
                .get(name)
                .map(|entry| entry.moved)
                .unwrap_or(binding.moved)
        };
        env.insert(
            name.clone(),
            Binding {
                ty: binding.ty.clone(),
                moved: binding.moved || body_moved,
            },
        );
    }
}

fn lower_expr(
    expr: &syntax::Expr,
    env: &mut HashMap<String, Binding>,
    ctx: &LowerContext<'_>,
) -> Result<Expr, Diagnostic> {
    match expr {
        syntax::Expr::Literal(literal) => Ok(lower_literal(literal)),
        syntax::Expr::VarRef { name, line, column } => {
            let binding = env.get(name).ok_or_else(|| {
                Diagnostic::new("type", format!("undefined variable {name:?}"))
                    .with_span(*line, *column)
            })?;
            if binding.moved {
                return Err(
                    Diagnostic::new("ownership", format!("use of moved value {name:?}"))
                        .with_span(*line, *column),
                );
            }
            Ok(Expr::VarRef {
                name: name.clone(),
                ty: binding.ty.clone(),
            })
        }
        syntax::Expr::Call {
            name,
            args,
            line,
            column,
        } => {
            let signature = ctx.functions.get(name).ok_or_else(|| {
                Diagnostic::new("type", format!("undefined function {name:?}"))
                    .with_span(*line, *column)
            })?;
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
            for (arg, expected) in args.iter().zip(signature.params.iter()) {
                let lowered = lower_expr(arg, env, ctx)?;
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
                if matches!(arg, syntax::Expr::VarRef { .. }) && !expected.is_copy() {
                    move_value(arg, env)?;
                }
                lowered_args.push(lowered);
            }
            Ok(Expr::Call {
                name: name.clone(),
                args: lowered_args,
                ty: signature.return_ty.clone(),
            })
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
                let lowered = lower_expr(&field.expr, env, ctx)?;
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
                    move_owner_value(&field.expr, env)?;
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
                move_owner_value(base, env)?;
            }
            Ok(Expr::FieldAccess {
                base: Box::new(lowered_base),
                field: field.clone(),
                ty: field_ty,
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
                if !lowered.ty().is_copy() {
                    move_owner_value(element, env)?;
                }
                lowered_elements.push(lowered);
            }
            let element_ty = element_ty.expect("non-empty array literal must have an element type");
            Ok(Expr::ArrayLiteral {
                elements: lowered_elements,
                ty: Type::Array(Box::new(element_ty)),
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
            if lowered_index.ty() != &Type::Int {
                return Err(Diagnostic::new(
                    "type",
                    format!("array index expects int, got {}", lowered_index.ty()),
                )
                .with_span(*line, *column));
            }
            let Type::Array(element_ty) = lowered_base.ty() else {
                return Err(Diagnostic::new(
                    "type",
                    format!(
                        "array index expects an array value, got {}",
                        lowered_base.ty()
                    ),
                )
                .with_span(*line, *column));
            };
            let element_ty = (*element_ty.clone()).clone();
            if !element_ty.is_copy() {
                move_owner_value(base, env)?;
            }
            Ok(Expr::Index {
                base: Box::new(lowered_base),
                index: Box::new(lowered_index),
                ty: element_ty,
            })
        }
    }
}

fn move_value(expr: &syntax::Expr, env: &mut HashMap<String, Binding>) -> Result<(), Diagnostic> {
    let syntax::Expr::VarRef { name, line, column } = expr else {
        return Ok(());
    };
    let binding = env.get_mut(name).ok_or_else(|| {
        Diagnostic::new("type", format!("undefined variable {name:?}")).with_span(*line, *column)
    })?;
    if binding.moved {
        return Err(
            Diagnostic::new("ownership", format!("use of moved value {name:?}"))
                .with_span(*line, *column),
        );
    }
    binding.moved = true;
    Ok(())
}

fn move_owner_value(
    expr: &syntax::Expr,
    env: &mut HashMap<String, Binding>,
) -> Result<(), Diagnostic> {
    match expr {
        syntax::Expr::VarRef { .. } => move_value(expr, env),
        syntax::Expr::FieldAccess { base, .. } => move_owner_value(base, env),
        syntax::Expr::Index { base, .. } => move_owner_value(base, env),
        _ => Ok(()),
    }
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

fn lower_type<T>(
    ty: &syntax::TypeName,
    structs: &HashMap<String, T>,
    line: usize,
    column: usize,
) -> Result<Type, Diagnostic> {
    match ty {
        syntax::TypeName::Int => Ok(Type::Int),
        syntax::TypeName::Bool => Ok(Type::Bool),
        syntax::TypeName::String => Ok(Type::String),
        syntax::TypeName::Named(name) => {
            if !structs.contains_key(name) {
                return Err(Diagnostic::new("type", format!("unknown type {name:?}"))
                    .with_span(line, column));
            }
            Ok(Type::Struct(name.clone()))
        }
        syntax::TypeName::Array(inner) => Ok(Type::Array(Box::new(lower_type(
            inner, structs, line, column,
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
            Expr::ArrayLiteral { ty, .. } => ty,
            Expr::Index { ty, .. } => ty,
        }
    }
}

impl Stmt {
    fn always_returns(&self) -> bool {
        match self {
            Stmt::Return(_) => true,
            Stmt::If {
                then_block,
                else_block: Some(else_block),
                ..
            } => block_always_returns(then_block) && block_always_returns(else_block),
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
            | syntax::Stmt::Return { line, .. } => *line,
        }
    }

    fn column(&self) -> usize {
        match self {
            syntax::Stmt::Let { column, .. }
            | syntax::Stmt::Print { column, .. }
            | syntax::Stmt::If { column, .. }
            | syntax::Stmt::While { column, .. }
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
            | syntax::Expr::ArrayLiteral { line, .. }
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
            | syntax::Expr::ArrayLiteral { column, .. }
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
            Type::Array(inner) => write!(f, "[{inner}]"),
        }
    }
}
