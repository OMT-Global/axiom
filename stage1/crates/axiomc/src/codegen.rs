use crate::diagnostics::Diagnostic;
use crate::mir::{
    EnumDef, Expr, Function, LiteralValue, MatchArm, Param, Program, Stmt, StructDef, StructField,
    Type,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

pub fn render_rust(program: &Program) -> String {
    let type_context = TypeContext::new(program);
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
        "fn axiom_array_slice_bounds(len: usize, start: Option<i64>, end: Option<i64>) -> (usize, usize) {\n",
    );
    out.push_str("    let start = start.unwrap_or(0);\n");
    out.push_str("    let end = end.unwrap_or(len as i64);\n");
    out.push_str(
        "    let start = usize::try_from(start).expect(\"array slice start must be non-negative\");\n",
    );
    out.push_str(
        "    let end = usize::try_from(end).expect(\"array slice end must be non-negative\");\n",
    );
    out.push_str("    assert!(start <= end, \"array slice start must be <= end\");\n");
    out.push_str("    assert!(end <= len, \"array slice end out of bounds\");\n");
    out.push_str("    (start, end)\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_slice_view<'a, T>(values: &'a [T], start: Option<i64>, end: Option<i64>) -> &'a [T] {\n");
    out.push_str("    let (start, end) = axiom_array_slice_bounds(values.len(), start, end);\n");
    out.push_str("    &values[start..end]\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn axiom_last_index(len: usize) -> i64 {\n");
    out.push_str("    assert!(len > 0, \"collection must not be empty\");\n");
    out.push_str("    (len - 1) as i64\n");
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
        render_enum(enum_def, &type_context, &mut out);
        out.push('\n');
    }
    for struct_def in &program.structs {
        render_struct(struct_def, &type_context, &mut out);
        out.push('\n');
    }
    for function in &program.functions {
        render_function(function, &type_context, &mut out);
        out.push('\n');
    }
    out.push_str("fn main() {\n");
    for stmt in &program.stmts {
        render_stmt(stmt, &type_context, &mut out, 1);
    }
    out.push_str("}\n");
    out
}

struct TypeContext<'a> {
    structs: HashMap<&'a str, &'a StructDef>,
    enums: HashMap<&'a str, &'a EnumDef>,
}

impl<'a> TypeContext<'a> {
    fn new(program: &'a Program) -> Self {
        Self {
            structs: program
                .structs
                .iter()
                .map(|struct_def| (struct_def.name.as_str(), struct_def))
                .collect(),
            enums: program
                .enums
                .iter()
                .map(|enum_def| (enum_def.name.as_str(), enum_def))
                .collect(),
        }
    }

    fn type_contains_borrowed_slice(&self, ty: &Type) -> bool {
        self.type_contains_borrowed_slice_inner(ty, &mut HashSet::new(), &mut HashSet::new())
    }

    fn struct_uses_borrowed_slice(&self, name: &str) -> bool {
        self.type_contains_borrowed_slice(&Type::Struct(name.to_string()))
    }

    fn enum_uses_borrowed_slice(&self, name: &str) -> bool {
        self.type_contains_borrowed_slice(&Type::Enum(name.to_string()))
    }

    fn type_contains_borrowed_slice_inner(
        &self,
        ty: &Type,
        visiting_structs: &mut HashSet<String>,
        visiting_enums: &mut HashSet<String>,
    ) -> bool {
        match ty {
            Type::Int | Type::Bool | Type::String => false,
            Type::Slice(_) => true,
            Type::Struct(name) => {
                if !visiting_structs.insert(name.clone()) {
                    return false;
                }
                let contains = self.structs.get(name.as_str()).is_some_and(|struct_def| {
                    struct_def.fields.iter().any(|field| {
                        self.type_contains_borrowed_slice_inner(
                            &field.ty,
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
                let contains = self.enums.get(name.as_str()).is_some_and(|enum_def| {
                    enum_def.variants.iter().any(|variant| {
                        variant.payload_tys.iter().any(|payload_ty| {
                            self.type_contains_borrowed_slice_inner(
                                payload_ty,
                                visiting_structs,
                                visiting_enums,
                            )
                        })
                    })
                });
                visiting_enums.remove(name);
                contains
            }
            Type::Option(inner) => {
                self.type_contains_borrowed_slice_inner(inner, visiting_structs, visiting_enums)
            }
            Type::Result(ok, err) => {
                self.type_contains_borrowed_slice_inner(ok, visiting_structs, visiting_enums)
                    || self.type_contains_borrowed_slice_inner(
                        err,
                        visiting_structs,
                        visiting_enums,
                    )
            }
            Type::Tuple(elements) => elements.iter().any(|element| {
                self.type_contains_borrowed_slice_inner(element, visiting_structs, visiting_enums)
            }),
            Type::Map(key, value) => {
                self.type_contains_borrowed_slice_inner(key, visiting_structs, visiting_enums)
                    || self.type_contains_borrowed_slice_inner(
                        value,
                        visiting_structs,
                        visiting_enums,
                    )
            }
            Type::Array(inner) => {
                self.type_contains_borrowed_slice_inner(inner, visiting_structs, visiting_enums)
            }
        }
    }
}

fn render_struct(struct_def: &StructDef, type_context: &TypeContext<'_>, out: &mut String) {
    let lifetime = if type_context.struct_uses_borrowed_slice(&struct_def.name) {
        "<'a>"
    } else {
        ""
    };
    out.push_str("#[allow(non_camel_case_types)]\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("#[derive(Debug, PartialEq)]\n");
    out.push_str(&format!("struct {}{} {{\n", struct_def.name, lifetime));
    for field in &struct_def.fields {
        render_struct_field(field, type_context, out, 1, !lifetime.is_empty());
    }
    out.push_str("}\n");
}

fn render_enum(enum_def: &EnumDef, type_context: &TypeContext<'_>, out: &mut String) {
    let lifetime = if type_context.enum_uses_borrowed_slice(&enum_def.name) {
        "<'a>"
    } else {
        ""
    };
    out.push_str("#[allow(non_camel_case_types)]\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("#[derive(Debug, PartialEq)]\n");
    out.push_str(&format!("enum {}{} {{\n", enum_def.name, lifetime));
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
                    rust_type_inner(payload_ty, Some("'a"), type_context)
                ));
            }
            out.push_str("    },\n");
        } else {
            let payload_tys = variant
                .payload_tys
                .iter()
                .map(|payload_ty| rust_type_inner(payload_ty, Some("'a"), type_context))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("    {}({payload_tys}),\n", variant.name));
        }
    }
    out.push_str("}\n");
}

fn render_struct_field(
    field: &StructField,
    type_context: &TypeContext<'_>,
    out: &mut String,
    indent: usize,
    uses_slice_lifetime: bool,
) {
    let pad = "    ".repeat(indent);
    let lifetime = uses_slice_lifetime.then_some("'a");
    out.push_str(&format!(
        "{pad}{}: {},\n",
        field.name,
        rust_type_inner(&field.ty, lifetime, type_context)
    ));
}

fn render_function(function: &Function, type_context: &TypeContext<'_>, out: &mut String) {
    let uses_slice_lifetime = function_signature_uses_borrowed_slice(function, type_context);
    let params = function
        .params
        .iter()
        .map(|param| render_param(param, uses_slice_lifetime, type_context))
        .collect::<Vec<_>>()
        .join(", ");
    let lifetime = if uses_slice_lifetime { "<'a>" } else { "" };
    out.push_str(&format!(
        "fn {}{}({}) -> {} {{\n",
        function.name,
        lifetime,
        params,
        rust_type_in_signature(&function.return_ty, uses_slice_lifetime, type_context)
    ));
    for stmt in &function.body {
        render_stmt(stmt, type_context, out, 1);
    }
    out.push_str("}\n");
}

fn render_param(
    param: &Param,
    uses_slice_lifetime: bool,
    type_context: &TypeContext<'_>,
) -> String {
    format!(
        "{}: {}",
        param.name,
        rust_type_in_signature(&param.ty, uses_slice_lifetime, type_context)
    )
}

fn render_stmt(stmt: &Stmt, type_context: &TypeContext<'_>, out: &mut String, indent: usize) {
    let pad = "    ".repeat(indent);
    match stmt {
        Stmt::Let { name, ty, expr } => out.push_str(&format!(
            "{pad}let {name}: {} = {};\n",
            rust_type(ty, type_context),
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
                render_stmt(stmt, type_context, out, indent + 1);
            }
            if let Some(else_block) = else_block {
                out.push_str(&format!("{pad}}} else {{\n"));
                for stmt in else_block {
                    render_stmt(stmt, type_context, out, indent + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            } else {
                out.push_str(&format!("{pad}}}\n"));
            }
        }
        Stmt::While { cond, body } => {
            out.push_str(&format!("{pad}while {} {{\n", render_expr(cond)));
            for stmt in body {
                render_stmt(stmt, type_context, out, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        Stmt::Match { expr, arms } => {
            out.push_str(&format!("{pad}match {} {{\n", render_expr(expr)));
            for arm in arms {
                render_match_arm(arm, type_context, out, indent + 1);
            }
            out.push_str(&format!("{pad}}}\n"));
        }
        Stmt::Return(expr) => out.push_str(&format!("{pad}return {};\n", render_expr(expr))),
    }
}

fn render_match_arm(
    arm: &MatchArm,
    type_context: &TypeContext<'_>,
    out: &mut String,
    indent: usize,
) {
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
        render_stmt(stmt, type_context, out, indent + 1);
    }
    out.push_str(&format!("{pad}}},\n"));
}

fn render_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(LiteralValue::Int(value)) => value.to_string(),
        Expr::Literal(LiteralValue::Bool(value)) => value.to_string(),
        Expr::Literal(LiteralValue::String(value)) => format!("String::from({value:?})"),
        Expr::VarRef { name, .. } => name.clone(),
        Expr::Call { name, args, .. } if name == "len" => {
            format!("({}).len() as i64", render_expr(&args[0]))
        }
        Expr::Call { name, args, ty } if name == "first" => {
            render_collection_edge(&args[0], ty, false)
        }
        Expr::Call { name, args, ty } if name == "last" => {
            render_collection_edge(&args[0], ty, true)
        }
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
            Type::Slice(_) => unreachable!("type checker rejects slice addition"),
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
        Expr::Slice {
            base, start, end, ..
        } => {
            let start = start
                .as_ref()
                .map(|expr| format!("Some({})", render_expr(expr)))
                .unwrap_or_else(|| String::from("None"));
            let end = end
                .as_ref()
                .map(|expr| format!("Some({})", render_expr(expr)))
                .unwrap_or_else(|| String::from("None"));
            match base.ty() {
                Type::Array(_) => {
                    format!(
                        "axiom_slice_view(&{}, {}, {})",
                        render_expr(base),
                        start,
                        end
                    )
                }
                Type::Slice(_) => {
                    format!(
                        "axiom_slice_view({}, {}, {})",
                        render_expr(base),
                        start,
                        end
                    )
                }
                _ => unreachable!("type checker rejects slicing non-array values"),
            }
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
            Type::Slice(_) => {
                format!(
                    "axiom_array_get({}, {})",
                    render_expr(base),
                    render_expr(index)
                )
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

fn rust_type(ty: &Type, type_context: &TypeContext<'_>) -> String {
    rust_type_inner(ty, None, type_context)
}

fn rust_type_in_signature(
    ty: &Type,
    uses_slice_lifetime: bool,
    type_context: &TypeContext<'_>,
) -> String {
    if uses_slice_lifetime {
        rust_type_inner(ty, Some("'a"), type_context)
    } else {
        rust_type(ty, type_context)
    }
}

fn rust_type_inner(ty: &Type, lifetime: Option<&str>, type_context: &TypeContext<'_>) -> String {
    match ty {
        Type::Int => String::from("i64"),
        Type::Bool => String::from("bool"),
        Type::String => String::from("String"),
        Type::Struct(name) => {
            if type_context.struct_uses_borrowed_slice(name) {
                format!("{name}<{}>", lifetime.unwrap_or("'_"))
            } else {
                name.clone()
            }
        }
        Type::Enum(name) => {
            if type_context.enum_uses_borrowed_slice(name) {
                format!("{name}<{}>", lifetime.unwrap_or("'_"))
            } else {
                name.clone()
            }
        }
        Type::Slice(inner) => {
            let inner = rust_type_inner(inner, lifetime, type_context);
            match lifetime {
                Some(lifetime) => format!("&{lifetime} [{inner}]"),
                None => format!("&[{inner}]"),
            }
        }
        Type::Option(inner) => {
            format!("Option<{}>", rust_type_inner(inner, lifetime, type_context))
        }
        Type::Result(ok, err) => format!(
            "Result<{}, {}>",
            rust_type_inner(ok, lifetime, type_context),
            rust_type_inner(err, lifetime, type_context)
        ),
        Type::Tuple(elements) => format!(
            "({})",
            elements
                .iter()
                .map(|element| rust_type_inner(element, lifetime, type_context))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Type::Map(key, value) => format!(
            "HashMap<{}, {}>",
            rust_type_inner(key, lifetime, type_context),
            rust_type_inner(value, lifetime, type_context)
        ),
        Type::Array(inner) => format!("Vec<{}>", rust_type_inner(inner, lifetime, type_context)),
    }
}

fn function_signature_uses_borrowed_slice(
    function: &Function,
    type_context: &TypeContext<'_>,
) -> bool {
    type_context.type_contains_borrowed_slice(&function.return_ty)
        || function
            .params
            .iter()
            .any(|param| type_context.type_contains_borrowed_slice(&param.ty))
}

fn render_collection_edge(collection: &Expr, result_ty: &Type, from_end: bool) -> String {
    let rendered = render_expr(collection);
    let index = if from_end {
        String::from("axiom_last_index(values.len())")
    } else {
        String::from("0")
    };
    match collection.ty() {
        Type::Array(_) => {
            if result_ty.is_copy() {
                format!("{{ let values = {rendered}; axiom_array_get(&values, {index}) }}")
            } else {
                format!(
                    "{{ let values = {rendered}; let index = {index}; axiom_array_take(values, index) }}"
                )
            }
        }
        Type::Slice(_) => format!(
            "{{ let values = {rendered}; let index = {index}; axiom_array_get(values, index) }}"
        ),
        _ => unreachable!("type checker rejects first/last on non-collection values"),
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
