use crate::diagnostics::Diagnostic;
use crate::mir::{
    EnumDef, Expr, Function, LiteralValue, MatchArm, Param, Program, Stmt, StructDef, StructField,
    Type,
};
use std::path::Path;
use std::process::Command;

pub fn render_rust(program: &Program) -> String {
    let mut out = String::new();
    out.push_str("#[allow(unused_imports)]\n");
    out.push_str("use std::collections::HashMap;\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_array_get<T: Copy>(values: &[T], index: i64) -> T {\n");
    out.push_str(
        "    let index = usize::try_from(index).expect(\"array index must be non-negative\");\n",
    );
    out.push_str("    values[index]\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_array_take<T>(values: Vec<T>, index: i64) -> T {\n");
    out.push_str(
        "    let index = usize::try_from(index).expect(\"array index must be non-negative\");\n",
    );
    out.push_str("    values.into_iter().nth(index).expect(\"array index out of bounds\")\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str(
        "fn axiom_map_get<K: Eq + std::hash::Hash, V: Copy>(values: &HashMap<K, V>, key: &K) -> V {\n",
    );
    out.push_str("    *values.get(key).expect(\"map key not found\")\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str(
        "fn axiom_map_take<K: Eq + std::hash::Hash, V>(mut values: HashMap<K, V>, key: &K) -> V {\n",
    );
    out.push_str("    values.remove(key).expect(\"map key not found\")\n");
    out.push_str("}\n\n");
    for enum_def in &program.enums {
        render_enum(enum_def, &mut out);
        out.push('\n');
    }
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
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("#[derive(Debug, PartialEq)]\n");
    out.push_str(&format!("struct {} {{\n", struct_def.name));
    for field in &struct_def.fields {
        render_struct_field(field, out, 1);
    }
    out.push_str("}\n");
}

fn render_enum(enum_def: &EnumDef, out: &mut String) {
    out.push_str("#[allow(non_camel_case_types)]\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("#[derive(Debug, PartialEq)]\n");
    out.push_str(&format!("enum {} {{\n", enum_def.name));
    for variant in &enum_def.variants {
        if variant.payload_tys.is_empty() {
            out.push_str(&format!("    {},\n", variant.name));
        } else if !variant.payload_names.is_empty() {
            out.push_str(&format!("    {} {{\n", variant.name));
            for (payload_name, payload_ty) in
                variant.payload_names.iter().zip(variant.payload_tys.iter())
            {
                out.push_str(&format!(
                    "        {}: {},\n",
                    payload_name,
                    rust_type(payload_ty)
                ));
            }
            out.push_str("    },\n");
        } else {
            let payload_tys = variant
                .payload_tys
                .iter()
                .map(rust_type)
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("    {}({payload_tys}),\n", variant.name));
        }
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
        Stmt::Match { expr, arms } => {
            out.push_str(&format!("{pad}match {} {{\n", render_expr(expr)));
            for arm in arms {
                render_match_arm(arm, out, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        Stmt::Return(expr) => out.push_str(&format!("{pad}return {};\n", render_expr(expr))),
    }
}

fn render_match_arm(arm: &MatchArm, out: &mut String, indent: usize) {
    let pad = "    ".repeat(indent);
    if arm.bindings.is_empty() {
        out.push_str(&format!("{pad}{}::{} => {{\n", arm.enum_name, arm.variant));
    } else if arm.is_named {
        out.push_str(&format!(
            "{pad}{}::{} {{ {} }} => {{\n",
            arm.enum_name,
            arm.variant,
            arm.bindings.join(", ")
        ));
    } else {
        out.push_str(&format!(
            "{pad}{}::{}({}) => {{\n",
            arm.enum_name,
            arm.variant,
            arm.bindings.join(", ")
        ));
    }
    for stmt in &arm.body {
        render_stmt(stmt, out, indent + 1);
    }
    out.push_str(&format!("{pad}}},\n"));
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
            Type::Enum(_) => unreachable!("type checker rejects enum addition"),
            Type::Option(_) => unreachable!("type checker rejects option addition"),
            Type::Result(_, _) => unreachable!("type checker rejects result addition"),
            Type::Tuple(_) => unreachable!("type checker rejects tuple addition"),
            Type::Map(_, _) => unreachable!("type checker rejects map addition"),
            Type::Array(_) => unreachable!("type checker rejects array addition"),
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
        Expr::TupleLiteral { elements, .. } => {
            let rendered = elements
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({rendered})")
        }
        Expr::TupleIndex { base, index, .. } => format!("({}).{}", render_expr(base), index),
        Expr::MapLiteral { entries, .. } => {
            let rendered = entries
                .iter()
                .map(|entry| {
                    format!(
                        "({}, {})",
                        render_expr(&entry.key),
                        render_expr(&entry.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("HashMap::from([{rendered}])")
        }
        Expr::EnumVariant {
            enum_name,
            variant,
            field_names,
            payloads,
            ..
        } => {
            if payloads.is_empty() {
                format!("{enum_name}::{variant}")
            } else if !field_names.is_empty() {
                let rendered_fields = field_names
                    .iter()
                    .zip(payloads.iter())
                    .map(|(field_name, payload)| format!("{field_name}: {}", render_expr(payload)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{enum_name}::{variant} {{ {rendered_fields} }}")
            } else {
                let rendered_payloads = payloads
                    .iter()
                    .map(render_expr)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{enum_name}::{variant}({rendered_payloads})")
            }
        }
        Expr::ArrayLiteral { elements, .. } => {
            let rendered = elements
                .iter()
                .map(render_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("vec![{rendered}]")
        }
        Expr::Index { base, index, ty } => match base.ty() {
            Type::Array(_) => {
                if ty.is_copy() {
                    format!(
                        "axiom_array_get(&{}, {})",
                        render_expr(base),
                        render_expr(index)
                    )
                } else {
                    format!(
                        "axiom_array_take({}, {})",
                        render_expr(base),
                        render_expr(index)
                    )
                }
            }
            Type::Map(_, _) => {
                if ty.is_copy() {
                    format!(
                        "axiom_map_get(&{}, &{})",
                        render_expr(base),
                        render_expr(index)
                    )
                } else {
                    format!(
                        "axiom_map_take({}, &{})",
                        render_expr(base),
                        render_expr(index)
                    )
                }
            }
            _ => unreachable!("type checker rejects indexing non-collection values"),
        },
    }
}

fn rust_type(ty: &Type) -> String {
    match ty {
        Type::Int => String::from("i64"),
        Type::Bool => String::from("bool"),
        Type::String => String::from("String"),
        Type::Struct(name) => name.clone(),
        Type::Enum(name) => name.clone(),
        Type::Option(inner) => format!("Option<{}>", rust_type(inner)),
        Type::Result(ok, err) => format!("Result<{}, {}>", rust_type(ok), rust_type(err)),
        Type::Tuple(elements) => format!(
            "({})",
            elements
                .iter()
                .map(rust_type)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Map(key, value) => format!("HashMap<{}, {}>", rust_type(key), rust_type(value)),
        Type::Array(inner) => format!("Vec<{}>", rust_type(inner)),
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
