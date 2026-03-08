use crate::diagnostics::Diagnostic;
use crate::mir::{
    Expr, Function, LiteralValue, Param, Program, Stmt, StructDef, StructField, Type,
};
use std::path::Path;
use std::process::Command;

pub fn render_rust(program: &Program) -> String {
    let mut out = String::new();
    for struct_def in &program.structs {
        render_struct(struct_def, &mut out);
        out.push('\n');
    }
    for function in &program.functions {
        render_function(function, &mut out);
        out.push('\n');
    }
    out.push_str("fn main() {\n");
    for stmt in &program.stmts {
        render_stmt(stmt, &mut out, 1);
    }
    out.push_str("}\n");
    out
}

fn render_struct(struct_def: &StructDef, out: &mut String) {
    out.push_str("#[allow(non_camel_case_types)]\n");
    out.push_str("#[derive(Debug)]\n");
    out.push_str(&format!("struct {} {{\n", struct_def.name));
    for field in &struct_def.fields {
        render_struct_field(field, out, 1);
    }
    out.push_str("}\n");
}

fn render_struct_field(field: &StructField, out: &mut String, indent: usize) {
    let pad = "    ".repeat(indent);
    out.push_str(&format!("{pad}{}: {},\n", field.name, rust_type(&field.ty)));
}

fn render_function(function: &Function, out: &mut String) {
    let params = function
        .params
        .iter()
        .map(render_param)
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!(
        "fn {}({}) -> {} {{\n",
        function.name,
        params,
        rust_type(&function.return_ty)
    ));
    for stmt in &function.body {
        render_stmt(stmt, out, 1);
    }
    out.push_str("}\n");
}

fn render_param(param: &Param) -> String {
    format!("{}: {}", param.name, rust_type(&param.ty))
}

fn render_stmt(stmt: &Stmt, out: &mut String, indent: usize) {
    let pad = "    ".repeat(indent);
    match stmt {
        Stmt::Let { name, ty, expr } => out.push_str(&format!(
            "{pad}let {name}: {} = {};\n",
            rust_type(ty),
            render_expr(expr)
        )),
        Stmt::Print(expr) => out.push_str(&format!(
            "{pad}println!(\"{{}}\", {});\n",
            render_expr(expr)
        )),
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => {
            out.push_str(&format!("{pad}if {} {{\n", render_expr(cond)));
            for stmt in then_block {
                render_stmt(stmt, out, indent + 1);
            }
            if let Some(else_block) = else_block {
                out.push_str(&format!("{pad}}} else {{\n"));
                for stmt in else_block {
                    render_stmt(stmt, out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            } else {
                out.push_str(&format!("{pad}}}\n"));
            }
        }
        Stmt::While { cond, body } => {
            out.push_str(&format!("{pad}while {} {{\n", render_expr(cond)));
            for stmt in body {
                render_stmt(stmt, out, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        Stmt::Return(expr) => out.push_str(&format!("{pad}return {};\n", render_expr(expr))),
    }
}

fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(LiteralValue::Int(value)) => value.to_string(),
        Expr::Literal(LiteralValue::Bool(value)) => value.to_string(),
        Expr::Literal(LiteralValue::String(value)) => format!("String::from({value:?})"),
        Expr::VarRef { name, .. } => name.clone(),
        Expr::Call { name, args, .. } => {
            let rendered_args = args.iter().map(render_expr).collect::<Vec<_>>().join(", ");
            format!("{name}({rendered_args})")
        }
        Expr::BinaryAdd { lhs, rhs, ty } => match ty {
            Type::Int => format!("{} + {}", render_expr(lhs), render_expr(rhs)),
            Type::String => format!(
                "format!(\"{{}}{{}}\", {}, {})",
                render_expr(lhs),
                render_expr(rhs)
            ),
            Type::Bool => unreachable!("type checker rejects bool addition"),
            Type::Struct(_) => unreachable!("type checker rejects struct addition"),
        },
        Expr::BinaryCompare { op, lhs, rhs, .. } => {
            format!("{} {} {}", render_expr(lhs), op.lexeme(), render_expr(rhs))
        }
        Expr::StructLiteral { name, fields, .. } => {
            let rendered_fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_expr(&field.expr)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name} {{ {rendered_fields} }}")
        }
        Expr::FieldAccess { base, field, .. } => format!("({}).{}", render_expr(base), field),
    }
}

fn rust_type(ty: &Type) -> String {
    match ty {
        Type::Int => String::from("i64"),
        Type::Bool => String::from("bool"),
        Type::String => String::from("String"),
        Type::Struct(name) => name.clone(),
    }
}

impl crate::mir::CompareOp {
    fn lexeme(self) -> &'static str {
        match self {
            crate::mir::CompareOp::Eq => "==",
            crate::mir::CompareOp::Ne => "!=",
            crate::mir::CompareOp::Lt => "<",
            crate::mir::CompareOp::Le => "<=",
            crate::mir::CompareOp::Gt => ">",
            crate::mir::CompareOp::Ge => ">=",
        }
    }
}

pub fn compile_native(generated_rust: &Path, binary_path: &Path) -> Result<(), Diagnostic> {
    let status = Command::new("rustc")
        .arg("--crate-name")
        .arg("axiom_stage1_bootstrap")
        .arg("--edition=2024")
        .arg("-O")
        .arg(generated_rust)
        .arg("-o")
        .arg(binary_path)
        .status()
        .map_err(|err| {
            Diagnostic::new("build", format!("failed to invoke rustc: {err}"))
                .with_path(generated_rust.display().to_string())
        })?;
    if !status.success() {
        return Err(
            Diagnostic::new("build", "rustc failed to produce a native binary")
                .with_path(generated_rust.display().to_string()),
        );
    }
    Ok(())
}
