use crate::hir;
use serde::Serialize;

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
    Literal(LiteralValue),
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
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructFieldValue {
    pub name: String,
    pub expr: Expr,
}

pub fn lower(program: &hir::Program) -> Program {
    Program {
        structs: program.structs.iter().map(lower_struct).collect(),
        enums: program.enums.iter().map(lower_enum).collect(),
        functions: program.functions.iter().map(lower_function).collect(),
        stmts: program.stmts.iter().map(lower_stmt).collect(),
    }
}

impl Program {
    pub fn statement_count(&self) -> usize {
        self.functions
            .iter()
            .map(|function| function.body.iter().map(count_stmt).sum::<usize>())
            .sum::<usize>()
            + self.stmts.iter().map(count_stmt).sum::<usize>()
    }
}

impl Expr {
    pub fn ty(&self) -> Type {
        match self {
            Expr::Literal(LiteralValue::Int(_)) => Type::Int,
            Expr::Literal(LiteralValue::Bool(_)) => Type::Bool,
            Expr::Literal(LiteralValue::String(_)) => Type::String,
            Expr::VarRef { ty, .. } => ty.clone(),
            Expr::Call { ty, .. } => ty.clone(),
            Expr::BinaryAdd { ty, .. } => ty.clone(),
            Expr::BinaryCompare { ty, .. } => ty.clone(),
            Expr::StructLiteral { ty, .. } => ty.clone(),
            Expr::FieldAccess { ty, .. } => ty.clone(),
            Expr::TupleLiteral { ty, .. } => ty.clone(),
            Expr::TupleIndex { ty, .. } => ty.clone(),
            Expr::MapLiteral { ty, .. } => ty.clone(),
            Expr::EnumVariant { ty, .. } => ty.clone(),
            Expr::ArrayLiteral { ty, .. } => ty.clone(),
            Expr::Slice { ty, .. } => ty.clone(),
            Expr::Index { ty, .. } => ty.clone(),
        }
    }
}

fn count_stmt(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Let { .. } | Stmt::Print(_) | Stmt::Return(_) => 1,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            1 + then_block.iter().map(count_stmt).sum::<usize>()
                + else_block
                    .as_ref()
                    .map(|block| block.iter().map(count_stmt).sum::<usize>())
                    .unwrap_or(0)
        }
        Stmt::While { body, .. } => 1 + body.iter().map(count_stmt).sum::<usize>(),
        Stmt::Match { arms, .. } => {
            1 + arms
                .iter()
                .map(|arm| arm.body.iter().map(count_stmt).sum::<usize>())
                .sum::<usize>()
        }
    }
}

fn lower_function(function: &hir::Function) -> Function {
    Function {
        name: function.name.clone(),
        params: function.params.iter().map(lower_param).collect(),
        return_ty: lower_type(&function.return_ty),
        body: function.body.iter().map(lower_stmt).collect(),
    }
}

fn lower_struct(struct_def: &hir::StructDef) -> StructDef {
    StructDef {
        name: struct_def.name.clone(),
        fields: struct_def.fields.iter().map(lower_struct_field).collect(),
    }
}

fn lower_enum(enum_def: &hir::EnumDef) -> EnumDef {
    EnumDef {
        name: enum_def.name.clone(),
        variants: enum_def.variants.iter().map(lower_enum_variant).collect(),
    }
}

fn lower_enum_variant(variant: &hir::EnumVariantDef) -> EnumVariantDef {
    EnumVariantDef {
        name: variant.name.clone(),
        payload_tys: variant.payload_tys.iter().map(lower_type).collect(),
        payload_names: variant.payload_names.clone(),
    }
}

fn lower_struct_field(field: &hir::StructField) -> StructField {
    StructField {
        name: field.name.clone(),
        ty: lower_type(&field.ty),
    }
}

fn lower_param(param: &hir::Param) -> Param {
    Param {
        name: param.name.clone(),
        ty: lower_type(&param.ty),
    }
}

fn lower_stmt(stmt: &hir::Stmt) -> Stmt {
    match stmt {
        hir::Stmt::Let { name, ty, expr } => Stmt::Let {
            name: name.clone(),
            ty: lower_type(ty),
            expr: lower_expr(expr),
        },
        hir::Stmt::Print(expr) => Stmt::Print(lower_expr(expr)),
        hir::Stmt::If {
            cond,
            then_block,
            else_block,
        } => Stmt::If {
            cond: lower_expr(cond),
            then_block: then_block.iter().map(lower_stmt).collect(),
            else_block: else_block
                .as_ref()
                .map(|block| block.iter().map(lower_stmt).collect()),
        },
        hir::Stmt::While { cond, body } => Stmt::While {
            cond: lower_expr(cond),
            body: body.iter().map(lower_stmt).collect(),
        },
        hir::Stmt::Match { expr, arms } => Stmt::Match {
            expr: lower_expr(expr),
            arms: arms
                .iter()
                .map(|arm| MatchArm {
                    enum_name: arm.enum_name.clone(),
                    variant: arm.variant.clone(),
                    bindings: arm.bindings.clone(),
                    is_named: arm.is_named,
                    body: arm.body.iter().map(lower_stmt).collect(),
                })
                .collect(),
        },
        hir::Stmt::Return(expr) => Stmt::Return(lower_expr(expr)),
    }
}

fn lower_expr(expr: &hir::Expr) -> Expr {
    match expr {
        hir::Expr::Literal { value, .. } => Expr::Literal(match value {
            hir::LiteralValue::Int(value) => LiteralValue::Int(*value),
            hir::LiteralValue::Bool(value) => LiteralValue::Bool(*value),
            hir::LiteralValue::String(value) => LiteralValue::String(value.clone()),
        }),
        hir::Expr::VarRef { name, ty } => Expr::VarRef {
            name: name.clone(),
            ty: lower_type(ty),
        },
        hir::Expr::Call { name, args, ty } => Expr::Call {
            name: name.clone(),
            args: args.iter().map(lower_expr).collect(),
            ty: lower_type(ty),
        },
        hir::Expr::BinaryAdd { lhs, rhs, ty } => Expr::BinaryAdd {
            lhs: Box::new(lower_expr(lhs)),
            rhs: Box::new(lower_expr(rhs)),
            ty: lower_type(ty),
        },
        hir::Expr::BinaryCompare { op, lhs, rhs, ty } => Expr::BinaryCompare {
            op: lower_compare_op(*op),
            lhs: Box::new(lower_expr(lhs)),
            rhs: Box::new(lower_expr(rhs)),
            ty: lower_type(ty),
        },
        hir::Expr::StructLiteral { name, fields, ty } => Expr::StructLiteral {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|field| StructFieldValue {
                    name: field.name.clone(),
                    expr: lower_expr(&field.expr),
                })
                .collect(),
            ty: lower_type(ty),
        },
        hir::Expr::FieldAccess { base, field, ty } => Expr::FieldAccess {
            base: Box::new(lower_expr(base)),
            field: field.clone(),
            ty: lower_type(ty),
        },
        hir::Expr::TupleLiteral { elements, ty } => Expr::TupleLiteral {
            elements: elements.iter().map(lower_expr).collect(),
            ty: lower_type(ty),
        },
        hir::Expr::TupleIndex { base, index, ty } => Expr::TupleIndex {
            base: Box::new(lower_expr(base)),
            index: *index,
            ty: lower_type(ty),
        },
        hir::Expr::MapLiteral { entries, ty } => Expr::MapLiteral {
            entries: entries
                .iter()
                .map(|entry| MapEntry {
                    key: lower_expr(&entry.key),
                    value: lower_expr(&entry.value),
                })
                .collect(),
            ty: lower_type(ty),
        },
        hir::Expr::EnumVariant {
            enum_name,
            variant,
            field_names,
            payloads,
            ty,
        } => Expr::EnumVariant {
            enum_name: enum_name.clone(),
            variant: variant.clone(),
            field_names: field_names.clone(),
            payloads: payloads.iter().map(lower_expr).collect(),
            ty: lower_type(ty),
        },
        hir::Expr::ArrayLiteral { elements, ty } => Expr::ArrayLiteral {
            elements: elements.iter().map(lower_expr).collect(),
            ty: lower_type(ty),
        },
        hir::Expr::Slice {
            base,
            start,
            end,
            ty,
        } => Expr::Slice {
            base: Box::new(lower_expr(base)),
            start: start.as_ref().map(|expr| Box::new(lower_expr(expr))),
            end: end.as_ref().map(|expr| Box::new(lower_expr(expr))),
            ty: lower_type(ty),
        },
        hir::Expr::Index { base, index, ty } => Expr::Index {
            base: Box::new(lower_expr(base)),
            index: Box::new(lower_expr(index)),
            ty: lower_type(ty),
        },
    }
}

fn lower_type(ty: &hir::Type) -> Type {
    match ty {
        hir::Type::Int => Type::Int,
        hir::Type::Bool => Type::Bool,
        hir::Type::String => Type::String,
        hir::Type::Struct(name) => Type::Struct(name.clone()),
        hir::Type::Enum(name) => Type::Enum(name.clone()),
        hir::Type::Slice(inner) => Type::Slice(Box::new(lower_type(inner))),
        hir::Type::MutSlice(inner) => Type::MutSlice(Box::new(lower_type(inner))),
        hir::Type::Option(inner) => Type::Option(Box::new(lower_type(inner))),
        hir::Type::Result(ok, err) => {
            Type::Result(Box::new(lower_type(ok)), Box::new(lower_type(err)))
        }
        hir::Type::Tuple(elements) => Type::Tuple(elements.iter().map(lower_type).collect()),
        hir::Type::Map(key, value) => {
            Type::Map(Box::new(lower_type(key)), Box::new(lower_type(value)))
        }
        hir::Type::Array(inner) => Type::Array(Box::new(lower_type(inner))),
    }
}

fn lower_compare_op(op: hir::CompareOp) -> CompareOp {
    match op {
        hir::CompareOp::Eq => CompareOp::Eq,
        hir::CompareOp::Ne => CompareOp::Ne,
        hir::CompareOp::Lt => CompareOp::Lt,
        hir::CompareOp::Le => CompareOp::Le,
        hir::CompareOp::Gt => CompareOp::Gt,
        hir::CompareOp::Ge => CompareOp::Ge,
    }
}
