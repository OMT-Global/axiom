use crate::hir;
use serde::Serialize;

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
    Array(Box<Type>),
}

impl Type {
    pub fn is_copy(&self) -> bool {
        matches!(self, Type::Int | Type::Bool)
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
        hir::Expr::ArrayLiteral { elements, ty } => Expr::ArrayLiteral {
            elements: elements.iter().map(lower_expr).collect(),
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
