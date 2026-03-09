use crate::diagnostics::Diagnostic;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Program {
    pub imports: Vec<Import>,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<Function>,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Import {
    pub path: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: TypeName,
    pub body: Vec<Stmt>,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructField {
    pub name: String,
    pub ty: TypeName,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<EnumVariantDecl>,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EnumVariantDecl {
    pub name: String,
    pub payload_tys: Vec<TypeName>,
    pub payload_names: Vec<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: TypeName,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Stmt {
    Let {
        name: String,
        ty: TypeName,
        expr: Expr,
        line: usize,
        column: usize,
    },
    Print {
        expr: Expr,
        line: usize,
        column: usize,
    },
    If {
        cond: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
        line: usize,
        column: usize,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
        line: usize,
        column: usize,
    },
    Match {
        expr: Expr,
        arms: Vec<MatchArm>,
        line: usize,
        column: usize,
    },
    Return {
        expr: Expr,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MatchArm {
    pub variant: String,
    pub bindings: Vec<String>,
    pub is_named: bool,
    pub body: Vec<Stmt>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Bool,
    String,
    Named(String),
    Option(Box<TypeName>),
    Result(Box<TypeName>, Box<TypeName>),
    Tuple(Vec<TypeName>),
    Map(Box<TypeName>, Box<TypeName>),
    Array(Box<TypeName>),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Expr {
    Literal(Literal),
    VarRef {
        name: String,
        line: usize,
        column: usize,
    },
    Call {
        name: String,
        args: Vec<Expr>,
        line: usize,
        column: usize,
    },
    BinaryAdd {
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        line: usize,
        column: usize,
    },
    BinaryCompare {
        op: CompareOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        line: usize,
        column: usize,
    },
    StructLiteral {
        name: String,
        fields: Vec<StructFieldValue>,
        line: usize,
        column: usize,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        line: usize,
        column: usize,
    },
    TupleLiteral {
        elements: Vec<Expr>,
        line: usize,
        column: usize,
    },
    TupleIndex {
        base: Box<Expr>,
        index: usize,
        line: usize,
        column: usize,
    },
    MapLiteral {
        entries: Vec<MapEntry>,
        line: usize,
        column: usize,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        line: usize,
        column: usize,
    },
    Slice {
        base: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        line: usize,
        column: usize,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct StructFieldValue {
    pub name: String,
    pub expr: Expr,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MapEntry {
    pub key: Expr,
    pub value: Expr,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Literal {
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

pub fn parse_program(source: &str, path: &Path) -> Result<Program, Diagnostic> {
    let lines: Vec<&str> = source.lines().collect();
    let mut index = 0;
    let mut imports = Vec::new();
    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut functions = Vec::new();
    let mut stmts = Vec::new();
    while index < lines.len() {
        let line_no = index + 1;
        let trimmed = lines[index].trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }
        match trimmed {
            "}" => {
                return Err(Diagnostic::new("parse", "unexpected closing brace")
                    .with_path(path.display().to_string())
                    .with_span(line_no, 1));
            }
            "} else {" | "else {" => {
                return Err(Diagnostic::new("parse", "unexpected else block")
                    .with_path(path.display().to_string())
                    .with_span(line_no, 1));
            }
            _ => {}
        }
        if let Some(import) = parse_import(trimmed, path, line_no)? {
            imports.push(import);
            index += 1;
            continue;
        }
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            functions.push(parse_function(&lines, &mut index, path)?);
            continue;
        }
        if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
            structs.push(parse_struct(&lines, &mut index, path)?);
            continue;
        }
        if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
            enums.push(parse_enum(&lines, &mut index, path)?);
            continue;
        }
        stmts.push(parse_stmt(&lines, &mut index, path, false)?);
    }
    Ok(Program {
        imports,
        structs,
        enums,
        functions,
        stmts,
    })
}

fn parse_import(trimmed: &str, path: &Path, line_no: usize) -> Result<Option<Import>, Diagnostic> {
    let Some(rest) = trimmed.strip_prefix("import ") else {
        return Ok(None);
    };
    let import_path = serde_json::from_str::<String>(rest).map_err(|_| {
        Diagnostic::new("parse", "import must use a quoted relative path")
            .with_path(path.display().to_string())
            .with_span(line_no, 1)
    })?;
    Ok(Some(Import {
        path: import_path,
        line: line_no,
        column: 1,
    }))
}

fn parse_stmt_list(
    lines: &[&str],
    index: &mut usize,
    path: &Path,
) -> Result<Vec<Stmt>, Diagnostic> {
    let mut stmts = Vec::new();
    while *index < lines.len() {
        let line_no = *index + 1;
        let trimmed = lines[*index].trim();
        if trimmed.is_empty() {
            *index += 1;
            continue;
        }
        if trimmed == "}" {
            *index += 1;
            return Ok(stmts);
        }
        if trimmed == "} else {" {
            return Ok(stmts);
        }
        if trimmed == "else {" {
            return Err(Diagnostic::new("parse", "unexpected else block")
                .with_path(path.display().to_string())
                .with_span(line_no, 1));
        }
        if trimmed.starts_with("import ") {
            return Err(Diagnostic::new(
                "parse",
                "stage1 bootstrap only supports imports at the top level",
            )
            .with_path(path.display().to_string())
            .with_span(line_no, 1));
        }
        if trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ") {
            return Err(Diagnostic::new(
                "parse",
                "stage1 bootstrap only supports top-level function declarations",
            )
            .with_path(path.display().to_string())
            .with_span(line_no, 1));
        }
        if trimmed.starts_with("struct ") || trimmed.starts_with("pub struct ") {
            return Err(Diagnostic::new(
                "parse",
                "stage1 bootstrap only supports top-level struct declarations",
            )
            .with_path(path.display().to_string())
            .with_span(line_no, 1));
        }
        if trimmed.starts_with("enum ") || trimmed.starts_with("pub enum ") {
            return Err(Diagnostic::new(
                "parse",
                "stage1 bootstrap only supports top-level enum declarations",
            )
            .with_path(path.display().to_string())
            .with_span(line_no, 1));
        }
        stmts.push(parse_stmt(lines, index, path, true)?);
    }
    Err(Diagnostic::new("parse", "missing closing brace for block")
        .with_path(path.display().to_string())
        .with_span(lines.len().max(1), 1))
}

fn parse_stmt(
    lines: &[&str],
    index: &mut usize,
    path: &Path,
    in_block: bool,
) -> Result<Stmt, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    if trimmed.starts_with("if ") {
        return parse_if_stmt(lines, index, path);
    }
    if trimmed.starts_with("while ") {
        return parse_while_stmt(lines, index, path);
    }
    if trimmed.starts_with("match ") {
        return parse_match_stmt(lines, index, path);
    }
    if let Some(rest) = trimmed.strip_prefix("let ") {
        let stmt = parse_let_stmt(rest, path, line_no)?;
        *index += 1;
        return Ok(stmt);
    }
    if let Some(rest) = trimmed.strip_prefix("print ") {
        let expr = parse_expr(rest, path, line_no, 7)?;
        *index += 1;
        return Ok(Stmt::Print {
            expr,
            line: line_no,
            column: 1,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("return ") {
        let expr = parse_expr(rest, path, line_no, 8)?;
        *index += 1;
        return Ok(Stmt::Return {
            expr,
            line: line_no,
            column: 1,
        });
    }
    let message = if in_block {
        "stage1 bootstrap currently supports let, print, if/else, while, match, and return statements inside blocks"
    } else {
        "stage1 bootstrap currently supports top-level import, struct, enum, fn, let, print, if/else, while, and match statements"
    };
    Err(Diagnostic::new("parse", message)
        .with_path(path.display().to_string())
        .with_span(line_no, 1))
}

fn parse_function(lines: &[&str], index: &mut usize, path: &Path) -> Result<Function, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    let (is_public, header) = if let Some(rest) = trimmed.strip_prefix("pub fn ") {
        (true, rest)
    } else {
        let rest = trimmed.strip_prefix("fn ").ok_or_else(|| {
            Diagnostic::new("parse", "invalid function declaration")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
        (false, rest)
    };
    let open_paren = find_top_level_char(header, '(').ok_or_else(|| {
        Diagnostic::new("parse", "function declaration is missing '('")
            .with_path(path.display().to_string())
            .with_span(line_no, 1)
    })?;
    let close_paren = find_matching_paren(header, open_paren).ok_or_else(|| {
        Diagnostic::new("parse", "function declaration is missing ')'")
            .with_path(path.display().to_string())
            .with_span(line_no, 1)
    })?;
    let name = header[..open_paren].trim();
    validate_ident(name, path, line_no, if is_public { 8 } else { 4 })?;
    let params = parse_params(&header[open_paren + 1..close_paren], path, line_no)?;
    let after_paren = header[close_paren + 1..].trim();
    let after_colon = after_paren.strip_prefix(':').ok_or_else(|| {
        Diagnostic::new("parse", "function declaration must include a return type")
            .with_path(path.display().to_string())
            .with_span(line_no, close_paren + 2)
    })?;
    let return_text = after_colon
        .strip_suffix('{')
        .map(str::trim)
        .ok_or_else(|| {
            Diagnostic::new(
                "parse",
                "function declaration must use `fn name(args): type {` syntax",
            )
            .with_path(path.display().to_string())
            .with_span(line_no, 1)
        })?;
    let return_ty = parse_type_name(return_text, path, line_no, 1)?;
    *index += 1;
    let body = parse_stmt_list(lines, index, path)?;
    Ok(Function {
        name: name.to_string(),
        params,
        return_ty,
        body,
        is_public,
        line: line_no,
        column: 1,
    })
}

fn parse_struct(lines: &[&str], index: &mut usize, path: &Path) -> Result<StructDecl, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    let (is_public, header) = if let Some(rest) = trimmed.strip_prefix("pub struct ") {
        (true, rest)
    } else {
        let rest = trimmed.strip_prefix("struct ").ok_or_else(|| {
            Diagnostic::new("parse", "invalid struct declaration")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
        (false, rest)
    };
    let name = header.strip_suffix('{').map(str::trim).ok_or_else(|| {
        Diagnostic::new(
            "parse",
            "struct declaration must use `struct Name {` syntax",
        )
        .with_path(path.display().to_string())
        .with_span(line_no, 1)
    })?;
    validate_ident(name, path, line_no, if is_public { 12 } else { 8 })?;
    *index += 1;
    let fields = parse_struct_fields(lines, index, path)?;
    Ok(StructDecl {
        name: name.to_string(),
        fields,
        is_public,
        line: line_no,
        column: 1,
    })
}

fn parse_struct_fields(
    lines: &[&str],
    index: &mut usize,
    path: &Path,
) -> Result<Vec<StructField>, Diagnostic> {
    let mut fields = Vec::new();
    while *index < lines.len() {
        let line_no = *index + 1;
        let trimmed = lines[*index].trim();
        if trimmed.is_empty() {
            *index += 1;
            continue;
        }
        if trimmed == "}" {
            *index += 1;
            return Ok(fields);
        }
        let colon = find_top_level_char(trimmed, ':').ok_or_else(|| {
            Diagnostic::new("parse", "struct field is missing ':'")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
        let name = trimmed[..colon].trim();
        validate_ident(name, path, line_no, 1)?;
        let ty = parse_type_name(trimmed[colon + 1..].trim(), path, line_no, colon + 2)?;
        fields.push(StructField {
            name: name.to_string(),
            ty,
            line: line_no,
            column: 1,
        });
        *index += 1;
    }
    Err(Diagnostic::new("parse", "missing closing brace for struct")
        .with_path(path.display().to_string())
        .with_span(lines.len().max(1), 1))
}

fn parse_enum(lines: &[&str], index: &mut usize, path: &Path) -> Result<EnumDecl, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    let (is_public, header) = if let Some(rest) = trimmed.strip_prefix("pub enum ") {
        (true, rest)
    } else {
        let rest = trimmed.strip_prefix("enum ").ok_or_else(|| {
            Diagnostic::new("parse", "invalid enum declaration")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
        (false, rest)
    };
    let name = header.strip_suffix('{').map(str::trim).ok_or_else(|| {
        Diagnostic::new("parse", "enum declaration must use `enum Name {` syntax")
            .with_path(path.display().to_string())
            .with_span(line_no, 1)
    })?;
    validate_ident(name, path, line_no, if is_public { 10 } else { 6 })?;
    *index += 1;
    let variants = parse_enum_variants(lines, index, path)?;
    Ok(EnumDecl {
        name: name.to_string(),
        variants,
        is_public,
        line: line_no,
        column: 1,
    })
}

fn parse_enum_variants(
    lines: &[&str],
    index: &mut usize,
    path: &Path,
) -> Result<Vec<EnumVariantDecl>, Diagnostic> {
    let mut variants = Vec::new();
    while *index < lines.len() {
        let line_no = *index + 1;
        let trimmed = lines[*index].trim();
        if trimmed.is_empty() {
            *index += 1;
            continue;
        }
        if trimmed == "}" {
            *index += 1;
            return Ok(variants);
        }
        let (name, payload_tys, payload_names) = if trimmed.ends_with('}')
            && let Some(open_brace) = find_top_level_char(trimmed, '{')
            && matches!(find_matching_brace(trimmed, open_brace), Some(close) if close == trimmed.len() - 1)
        {
            let name = trimmed[..open_brace].trim();
            validate_ident(name, path, line_no, 1)?;
            let payload_raw = trimmed[open_brace + 1..trimmed.len() - 1].trim();
            let fields = parse_named_enum_payload_fields(
                payload_raw,
                path,
                line_no,
                open_brace + 2,
                "enum variant payload field",
            )?;
            (
                name.to_string(),
                fields.iter().map(|field| field.1.clone()).collect(),
                fields.into_iter().map(|field| field.0).collect(),
            )
        } else if trimmed.ends_with(')')
            && let Some(open_paren) = find_top_level_char(trimmed, '(')
            && matches!(find_matching_paren(trimmed, open_paren), Some(close) if close == trimmed.len() - 1)
        {
            let name = trimmed[..open_paren].trim();
            validate_ident(name, path, line_no, 1)?;
            let payload_raw = trimmed[open_paren + 1..trimmed.len() - 1].trim();
            if payload_raw.is_empty() {
                return Err(
                    Diagnostic::new("parse", "enum variant payload type is empty")
                        .with_path(path.display().to_string())
                        .with_span(line_no, open_paren + 2),
                );
            }
            let payload_tys = split_top_level_type(payload_raw, ',')
                .into_iter()
                .map(|ty| {
                    let ty = ty.trim();
                    if ty.is_empty() {
                        return Err(
                            Diagnostic::new("parse", "enum variant payload type is empty")
                                .with_path(path.display().to_string())
                                .with_span(line_no, open_paren + 2),
                        );
                    }
                    parse_type_name(ty, path, line_no, open_paren + 2)
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            (name.to_string(), payload_tys, Vec::new())
        } else {
            validate_ident(trimmed, path, line_no, 1)?;
            (trimmed.to_string(), Vec::new(), Vec::new())
        };
        variants.push(EnumVariantDecl {
            name,
            payload_tys,
            payload_names,
            line: line_no,
            column: 1,
        });
        *index += 1;
    }
    Err(Diagnostic::new("parse", "missing closing brace for enum")
        .with_path(path.display().to_string())
        .with_span(lines.len().max(1), 1))
}

fn parse_params(raw: &str, path: &Path, line_no: usize) -> Result<Vec<Param>, Diagnostic> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut params = Vec::new();
    for param_text in split_top_level_type(raw, ',') {
        let param_text = param_text.trim();
        let colon = find_top_level_char(param_text, ':').ok_or_else(|| {
            Diagnostic::new("parse", "function parameter is missing ':'")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
        let name = param_text[..colon].trim();
        validate_ident(name, path, line_no, 1)?;
        let ty = parse_type_name(param_text[colon + 1..].trim(), path, line_no, 1)?;
        params.push(Param {
            name: name.to_string(),
            ty,
            line: line_no,
            column: 1,
        });
    }
    Ok(params)
}

fn parse_if_stmt(lines: &[&str], index: &mut usize, path: &Path) -> Result<Stmt, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    let cond_raw = trimmed
        .strip_prefix("if ")
        .and_then(|raw| raw.strip_suffix('{'))
        .map(str::trim)
        .ok_or_else(|| {
            Diagnostic::new("parse", "if statement must use `if <expr> {` syntax")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
    let cond = parse_expr(cond_raw, path, line_no, 4)?;
    *index += 1;
    let then_block = parse_stmt_list(lines, index, path)?;
    skip_blank_lines(lines, index);
    let else_block = if *index < lines.len() {
        match lines[*index].trim() {
            "} else {" => {
                *index += 1;
                Some(parse_stmt_list(lines, index, path)?)
            }
            "else {" => {
                *index += 1;
                Some(parse_stmt_list(lines, index, path)?)
            }
            _ => None,
        }
    } else {
        None
    };
    Ok(Stmt::If {
        cond,
        then_block,
        else_block,
        line: line_no,
        column: 1,
    })
}

fn parse_while_stmt(lines: &[&str], index: &mut usize, path: &Path) -> Result<Stmt, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    let cond_raw = trimmed
        .strip_prefix("while ")
        .and_then(|raw| raw.strip_suffix('{'))
        .map(str::trim)
        .ok_or_else(|| {
            Diagnostic::new("parse", "while statement must use `while <expr> {` syntax")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
    let cond = parse_expr(cond_raw, path, line_no, 7)?;
    *index += 1;
    let body = parse_stmt_list(lines, index, path)?;
    Ok(Stmt::While {
        cond,
        body,
        line: line_no,
        column: 1,
    })
}

fn parse_match_stmt(lines: &[&str], index: &mut usize, path: &Path) -> Result<Stmt, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    let expr_raw = trimmed
        .strip_prefix("match ")
        .and_then(|raw| raw.strip_suffix('{'))
        .map(str::trim)
        .ok_or_else(|| {
            Diagnostic::new("parse", "match statement must use `match <expr> {` syntax")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
    let expr = parse_expr(expr_raw, path, line_no, 7)?;
    *index += 1;
    let arms = parse_match_arms(lines, index, path)?;
    Ok(Stmt::Match {
        expr,
        arms,
        line: line_no,
        column: 1,
    })
}

fn parse_match_arms(
    lines: &[&str],
    index: &mut usize,
    path: &Path,
) -> Result<Vec<MatchArm>, Diagnostic> {
    let mut arms = Vec::new();
    while *index < lines.len() {
        let line_no = *index + 1;
        let trimmed = lines[*index].trim();
        if trimmed.is_empty() {
            *index += 1;
            continue;
        }
        if trimmed == "}" {
            *index += 1;
            return Ok(arms);
        }
        let variant = trimmed.strip_suffix('{').map(str::trim).ok_or_else(|| {
            Diagnostic::new("parse", "match arm must use `Variant {` syntax")
                .with_path(path.display().to_string())
                .with_span(line_no, 1)
        })?;
        let (variant, bindings, is_named) = if variant.ends_with('}')
            && let Some(open_brace) = find_top_level_char(variant, '{')
            && matches!(find_matching_brace(variant, open_brace), Some(close) if close == variant.len() - 1)
        {
            let name = variant[..open_brace].trim();
            validate_ident(name, path, line_no, 1)?;
            let bindings_raw = variant[open_brace + 1..variant.len() - 1].trim();
            if bindings_raw.is_empty() {
                return Err(Diagnostic::new("parse", "match arm binding is empty")
                    .with_path(path.display().to_string())
                    .with_span(line_no, open_brace + 2));
            }
            let bindings = split_top_level(bindings_raw, ',')
                .into_iter()
                .map(|binding| {
                    let binding = binding.trim();
                    if binding.is_empty() {
                        return Err(Diagnostic::new("parse", "match arm binding is empty")
                            .with_path(path.display().to_string())
                            .with_span(line_no, open_brace + 2));
                    }
                    validate_ident(binding, path, line_no, open_brace + 2)?;
                    Ok(binding.to_string())
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            (name.to_string(), bindings, true)
        } else if variant.ends_with(')')
            && let Some(open_paren) = find_top_level_char(variant, '(')
            && matches!(find_matching_paren(variant, open_paren), Some(close) if close == variant.len() - 1)
        {
            let name = variant[..open_paren].trim();
            validate_ident(name, path, line_no, 1)?;
            let bindings_raw = variant[open_paren + 1..variant.len() - 1].trim();
            if bindings_raw.is_empty() {
                return Err(Diagnostic::new("parse", "match arm binding is empty")
                    .with_path(path.display().to_string())
                    .with_span(line_no, open_paren + 2));
            }
            let bindings = split_top_level(bindings_raw, ',')
                .into_iter()
                .map(|binding| {
                    let binding = binding.trim();
                    if binding.is_empty() {
                        return Err(Diagnostic::new("parse", "match arm binding is empty")
                            .with_path(path.display().to_string())
                            .with_span(line_no, open_paren + 2));
                    }
                    validate_ident(binding, path, line_no, open_paren + 2)?;
                    Ok(binding.to_string())
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            (name.to_string(), bindings, false)
        } else {
            validate_ident(variant, path, line_no, 1)?;
            (variant.to_string(), Vec::new(), false)
        };
        *index += 1;
        let body = parse_stmt_list(lines, index, path)?;
        arms.push(MatchArm {
            variant,
            bindings,
            is_named,
            body,
            line: line_no,
            column: 1,
        });
    }
    Err(Diagnostic::new("parse", "missing closing brace for match")
        .with_path(path.display().to_string())
        .with_span(lines.len().max(1), 1))
}

fn parse_let_stmt(rest: &str, path: &Path, line_no: usize) -> Result<Stmt, Diagnostic> {
    let colon = find_top_level_char(rest, ':').ok_or_else(|| {
        Diagnostic::new("parse", "let binding is missing ':'")
            .with_path(path.display().to_string())
            .with_span(line_no, 1)
    })?;
    let name = rest[..colon].trim();
    validate_ident(name, path, line_no, 5)?;
    let after_colon = &rest[colon + 1..];
    let eq = find_top_level_char(after_colon, '=').ok_or_else(|| {
        Diagnostic::new("parse", "let binding is missing '='")
            .with_path(path.display().to_string())
            .with_span(line_no, colon + 2)
    })?;
    let type_text = after_colon[..eq].trim();
    let expr_text = after_colon[eq + 1..].trim();
    let ty = parse_type_name(type_text, path, line_no, colon + 2)?;
    let expr = parse_expr(expr_text, path, line_no, colon + eq + 3)?;
    Ok(Stmt::Let {
        name: name.to_string(),
        ty,
        expr,
        line: line_no,
        column: 1,
    })
}

fn parse_type_name(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<TypeName, Diagnostic> {
    if raw.starts_with("Option<")
        && raw.ends_with('>')
        && matches!(find_matching_angle(raw, 6), Some(close) if close == raw.len() - 1)
    {
        let inner = raw[7..raw.len() - 1].trim();
        if inner.is_empty() {
            return Err(
                Diagnostic::new("parse", "Option type is missing an inner type")
                    .with_path(path.display().to_string())
                    .with_span(line_no, column),
            );
        }
        return Ok(TypeName::Option(Box::new(parse_type_name(
            inner,
            path,
            line_no,
            column + 7,
        )?)));
    }
    if raw.starts_with("Result<")
        && raw.ends_with('>')
        && matches!(find_matching_angle(raw, 6), Some(close) if close == raw.len() - 1)
    {
        let inner = raw[7..raw.len() - 1].trim();
        let parts = split_top_level_type(inner, ',');
        if parts.len() != 2 {
            return Err(
                Diagnostic::new("parse", "Result type must use `Result<ok, err>` syntax")
                    .with_path(path.display().to_string())
                    .with_span(line_no, column),
            );
        }
        let ok_raw = parts[0].trim();
        let err_raw = parts[1].trim();
        if ok_raw.is_empty() || err_raw.is_empty() {
            return Err(
                Diagnostic::new("parse", "Result type is missing an ok or error type")
                    .with_path(path.display().to_string())
                    .with_span(line_no, column),
            );
        }
        return Ok(TypeName::Result(
            Box::new(parse_type_name(ok_raw, path, line_no, column + 7)?),
            Box::new(parse_type_name(err_raw, path, line_no, column + 7)?),
        ));
    }
    if raw.starts_with('{')
        && raw.ends_with('}')
        && matches!(find_matching_brace(raw, 0), Some(close) if close == raw.len() - 1)
    {
        let inner = raw[1..raw.len() - 1].trim();
        if inner.is_empty() {
            return Err(Diagnostic::new("parse", "map type is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        let colon = find_top_level_char(inner, ':').ok_or_else(|| {
            Diagnostic::new("parse", "map type must use `{key: value}` syntax")
                .with_path(path.display().to_string())
                .with_span(line_no, column)
        })?;
        let key_raw = inner[..colon].trim();
        let value_raw = inner[colon + 1..].trim();
        if key_raw.is_empty() || value_raw.is_empty() {
            return Err(
                Diagnostic::new("parse", "map type is missing a key or value type")
                    .with_path(path.display().to_string())
                    .with_span(line_no, column),
            );
        }
        return Ok(TypeName::Map(
            Box::new(parse_type_name(key_raw, path, line_no, column + 1)?),
            Box::new(parse_type_name(
                value_raw,
                path,
                line_no,
                column + colon + 2,
            )?),
        ));
    }
    if is_wrapped_in_parens(raw) {
        let inner = raw[1..raw.len() - 1].trim();
        if inner.is_empty() {
            return Err(Diagnostic::new("parse", "tuple type is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        let items = split_top_level_type(inner, ',');
        if items.len() > 1 {
            return Ok(TypeName::Tuple(parse_tuple_type_names(
                inner,
                path,
                line_no,
                column + 1,
            )?));
        }
        return parse_type_name(inner, path, line_no, column + 1);
    }
    if raw.starts_with('[')
        && raw.ends_with(']')
        && matches!(find_matching_square(raw, 0), Some(close) if close == raw.len() - 1)
    {
        let inner = raw[1..raw.len() - 1].trim();
        if inner.is_empty() {
            return Err(
                Diagnostic::new("parse", "array type is missing an element type")
                    .with_path(path.display().to_string())
                    .with_span(line_no, column),
            );
        }
        return Ok(TypeName::Array(Box::new(parse_type_name(
            inner,
            path,
            line_no,
            column + 1,
        )?)));
    }
    match raw {
        "int" => Ok(TypeName::Int),
        "bool" => Ok(TypeName::Bool),
        "string" => Ok(TypeName::String),
        _ => {
            validate_ident(raw, path, line_no, column)?;
            Ok(TypeName::Named(raw.to_string()))
        }
    }
}

fn parse_expr(raw: &str, path: &Path, line_no: usize, column: usize) -> Result<Expr, Diagnostic> {
    let raw = raw.trim();
    if let Some((op, split_index)) = find_compare_operator(raw) {
        let lhs_raw = raw[..split_index].trim();
        let rhs_offset = split_index + op.lexeme().len();
        let rhs_raw = raw[rhs_offset..].trim();
        if lhs_raw.is_empty() || rhs_raw.is_empty() {
            return Err(
                Diagnostic::new("parse", "comparison expression is incomplete")
                    .with_path(path.display().to_string())
                    .with_span(line_no, column),
            );
        }
        return Ok(Expr::BinaryCompare {
            op,
            lhs: Box::new(parse_add(lhs_raw, path, line_no, column)?),
            rhs: Box::new(parse_add(rhs_raw, path, line_no, column)?),
            line: line_no,
            column,
        });
    }
    parse_add(raw, path, line_no, column)
}

fn parse_add(raw: &str, path: &Path, line_no: usize, column: usize) -> Result<Expr, Diagnostic> {
    let terms = split_top_level(raw, '+');
    if terms.len() > 1 {
        let mut expr = parse_term(terms[0].trim(), path, line_no, column)?;
        for term in &terms[1..] {
            let rhs = parse_term(term.trim(), path, line_no, column)?;
            expr = Expr::BinaryAdd {
                lhs: Box::new(expr),
                rhs: Box::new(rhs),
                line: line_no,
                column,
            };
        }
        return Ok(expr);
    }
    parse_term(raw.trim(), path, line_no, column)
}

fn parse_term(raw: &str, path: &Path, line_no: usize, column: usize) -> Result<Expr, Diagnostic> {
    if raw.is_empty() {
        return Err(Diagnostic::new("parse", "expression is empty")
            .with_path(path.display().to_string())
            .with_span(line_no, column));
    }
    if let Some(dot) = find_last_top_level_char(raw, '.') {
        let base_raw = raw[..dot].trim();
        let field = raw[dot + 1..].trim();
        if base_raw.is_empty() || field.is_empty() {
            return Err(Diagnostic::new("parse", "field access is incomplete")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        if field.chars().all(|ch| ch.is_ascii_digit()) {
            let index = field.parse::<usize>().map_err(|_| {
                Diagnostic::new("parse", format!("invalid tuple index {field:?}"))
                    .with_path(path.display().to_string())
                    .with_span(line_no, column + dot + 1)
            })?;
            return Ok(Expr::TupleIndex {
                base: Box::new(parse_term(base_raw, path, line_no, column)?),
                index,
                line: line_no,
                column,
            });
        }
        validate_ident(field, path, line_no, column + dot + 1)?;
        return Ok(Expr::FieldAccess {
            base: Box::new(parse_term(base_raw, path, line_no, column)?),
            field: field.to_string(),
            line: line_no,
            column,
        });
    }
    if raw.ends_with(']')
        && let Some(open_bracket) = find_last_top_level_char(raw, '[')
        && matches!(find_matching_square(raw, open_bracket), Some(close) if close == raw.len() - 1)
    {
        let base_raw = raw[..open_bracket].trim();
        let index_raw = raw[open_bracket + 1..raw.len() - 1].trim();
        if base_raw.is_empty() {
            // This is an array literal, handled below.
        } else if let Some(colon) = find_top_level_char(index_raw, ':') {
            let start_raw = index_raw[..colon].trim();
            let end_raw = index_raw[colon + 1..].trim();
            return Ok(Expr::Slice {
                base: Box::new(parse_term(base_raw, path, line_no, column)?),
                start: if start_raw.is_empty() {
                    None
                } else {
                    Some(Box::new(parse_expr(
                        start_raw,
                        path,
                        line_no,
                        column + open_bracket + 1,
                    )?))
                },
                end: if end_raw.is_empty() {
                    None
                } else {
                    Some(Box::new(parse_expr(
                        end_raw,
                        path,
                        line_no,
                        column + open_bracket + colon + 2,
                    )?))
                },
                line: line_no,
                column,
            });
        } else if index_raw.is_empty() {
            return Err(Diagnostic::new("parse", "array index is incomplete")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        } else {
            return Ok(Expr::Index {
                base: Box::new(parse_term(base_raw, path, line_no, column)?),
                index: Box::new(parse_expr(
                    index_raw,
                    path,
                    line_no,
                    column + open_bracket + 1,
                )?),
                line: line_no,
                column,
            });
        }
    }
    if is_wrapped_in_parens(raw) {
        let inner = raw[1..raw.len() - 1].trim();
        if split_top_level(inner, ',').len() > 1 {
            return Ok(Expr::TupleLiteral {
                elements: parse_tuple_literal_elements(inner, path, line_no, column + 1)?,
                line: line_no,
                column,
            });
        }
        return parse_expr(&raw[1..raw.len() - 1], path, line_no, column + 1);
    }
    if raw.starts_with('{')
        && raw.ends_with('}')
        && matches!(find_matching_brace(raw, 0), Some(close) if close == raw.len() - 1)
    {
        return Ok(Expr::MapLiteral {
            entries: parse_map_literal_entries(&raw[1..raw.len() - 1], path, line_no, column + 1)?,
            line: line_no,
            column,
        });
    }
    if raw.starts_with('[')
        && raw.ends_with(']')
        && matches!(find_matching_square(raw, 0), Some(close) if close == raw.len() - 1)
    {
        return Ok(Expr::ArrayLiteral {
            elements: parse_array_literal_elements(&raw[1..raw.len() - 1], path, line_no, column)?,
            line: line_no,
            column,
        });
    }
    if raw.ends_with('}')
        && let Some(open_brace) = find_top_level_char(raw, '{')
        && matches!(find_matching_brace(raw, open_brace), Some(close) if close == raw.len() - 1)
    {
        let name = raw[..open_brace].trim();
        if !name.is_empty() {
            validate_ident(name, path, line_no, column)?;
            let fields = parse_struct_literal_fields(
                &raw[open_brace + 1..raw.len() - 1],
                path,
                line_no,
                column,
            )?;
            return Ok(Expr::StructLiteral {
                name: name.to_string(),
                fields,
                line: line_no,
                column,
            });
        }
    }
    if raw.starts_with('"') {
        let parsed = serde_json::from_str::<String>(raw).map_err(|_| {
            Diagnostic::new("parse", "invalid string literal")
                .with_path(path.display().to_string())
                .with_span(line_no, column)
        })?;
        return Ok(Expr::Literal(Literal::String(parsed)));
    }
    if raw == "true" {
        return Ok(Expr::Literal(Literal::Bool(true)));
    }
    if raw == "false" {
        return Ok(Expr::Literal(Literal::Bool(false)));
    }
    if let Ok(value) = raw.parse::<i64>() {
        return Ok(Expr::Literal(Literal::Int(value)));
    }
    if raw.ends_with(')')
        && let Some(open_paren) = find_top_level_char(raw, '(')
    {
        let name = raw[..open_paren].trim();
        if !name.is_empty() {
            validate_ident(name, path, line_no, column)?;
            let args = parse_call_args(&raw[open_paren + 1..raw.len() - 1], path, line_no, column)?;
            return Ok(Expr::Call {
                name: name.to_string(),
                args,
                line: line_no,
                column,
            });
        }
    }
    validate_ident(raw, path, line_no, column)?;
    Ok(Expr::VarRef {
        name: raw.to_string(),
        line: line_no,
        column,
    })
}

fn parse_array_literal_elements(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<Vec<Expr>, Diagnostic> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut elements = Vec::new();
    for element_text in split_top_level_type(raw, ',') {
        let element_text = element_text.trim();
        if element_text.is_empty() {
            return Err(Diagnostic::new("parse", "array literal element is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        elements.push(parse_expr(element_text, path, line_no, column)?);
    }
    Ok(elements)
}

fn parse_tuple_type_names(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<Vec<TypeName>, Diagnostic> {
    let mut elements = Vec::new();
    for element_text in split_top_level(raw, ',') {
        let element_text = element_text.trim();
        if element_text.is_empty() {
            return Err(Diagnostic::new("parse", "tuple type element is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        elements.push(parse_type_name(element_text, path, line_no, column)?);
    }
    Ok(elements)
}

fn parse_tuple_literal_elements(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<Vec<Expr>, Diagnostic> {
    let mut elements = Vec::new();
    for element_text in split_top_level(raw, ',') {
        let element_text = element_text.trim();
        if element_text.is_empty() {
            return Err(Diagnostic::new("parse", "tuple literal element is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        elements.push(parse_expr(element_text, path, line_no, column)?);
    }
    Ok(elements)
}

fn parse_map_literal_entries(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<Vec<MapEntry>, Diagnostic> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry_text in split_top_level(raw, ',') {
        let entry_text = entry_text.trim();
        if entry_text.is_empty() {
            return Err(Diagnostic::new("parse", "map literal entry is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        let colon = find_top_level_char(entry_text, ':').ok_or_else(|| {
            Diagnostic::new("parse", "map literal entry is missing ':'")
                .with_path(path.display().to_string())
                .with_span(line_no, column)
        })?;
        let key = parse_expr(entry_text[..colon].trim(), path, line_no, column)?;
        let value = parse_expr(
            entry_text[colon + 1..].trim(),
            path,
            line_no,
            column + colon + 1,
        )?;
        entries.push(MapEntry {
            key,
            value,
            line: line_no,
            column,
        });
    }
    Ok(entries)
}

fn parse_struct_literal_fields(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<Vec<StructFieldValue>, Diagnostic> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut fields = Vec::new();
    for field_text in split_top_level_type(raw, ',') {
        let field_text = field_text.trim();
        if field_text.is_empty() {
            return Err(Diagnostic::new("parse", "struct literal field is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        let colon = find_top_level_char(field_text, ':').ok_or_else(|| {
            Diagnostic::new("parse", "struct literal field is missing ':'")
                .with_path(path.display().to_string())
                .with_span(line_no, column)
        })?;
        let name = field_text[..colon].trim();
        validate_ident(name, path, line_no, column)?;
        let expr = parse_expr(
            field_text[colon + 1..].trim(),
            path,
            line_no,
            column + colon + 1,
        )?;
        fields.push(StructFieldValue {
            name: name.to_string(),
            expr,
            line: line_no,
            column,
        });
    }
    Ok(fields)
}

fn parse_named_enum_payload_fields(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
    context: &str,
) -> Result<Vec<(String, TypeName)>, Diagnostic> {
    if raw.trim().is_empty() {
        return Err(Diagnostic::new("parse", format!("{context} is empty"))
            .with_path(path.display().to_string())
            .with_span(line_no, column));
    }
    let mut fields = Vec::new();
    for field_text in split_top_level(raw, ',') {
        let field_text = field_text.trim();
        if field_text.is_empty() {
            return Err(Diagnostic::new("parse", format!("{context} is empty"))
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        let colon = find_top_level_char(field_text, ':').ok_or_else(|| {
            Diagnostic::new("parse", format!("{context} is missing ':'"))
                .with_path(path.display().to_string())
                .with_span(line_no, column)
        })?;
        let name = field_text[..colon].trim();
        validate_ident(name, path, line_no, column)?;
        let ty = parse_type_name(
            field_text[colon + 1..].trim(),
            path,
            line_no,
            column + colon + 1,
        )?;
        fields.push((name.to_string(), ty));
    }
    Ok(fields)
}

fn parse_call_args(
    raw: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<Vec<Expr>, Diagnostic> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut args = Vec::new();
    for arg_text in split_top_level(raw, ',') {
        let arg_text = arg_text.trim();
        if arg_text.is_empty() {
            return Err(Diagnostic::new("parse", "call argument is empty")
                .with_path(path.display().to_string())
                .with_span(line_no, column));
        }
        args.push(parse_expr(arg_text, path, line_no, column)?);
    }
    Ok(args)
}

fn validate_ident(
    value: &str,
    path: &Path,
    line_no: usize,
    column: usize,
) -> Result<(), Diagnostic> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(Diagnostic::new("parse", "identifier is empty")
            .with_path(path.display().to_string())
            .with_span(line_no, column));
    };
    if !(first.is_ascii_alphabetic() || first == '_')
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(
            Diagnostic::new("parse", format!("invalid identifier {value:?}"))
                .with_path(path.display().to_string())
                .with_span(line_no, column),
        );
    }
    Ok(())
}

fn split_top_level(raw: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut start = 0;
    for (index, ch) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ if ch == delimiter && paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                parts.push(&raw[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&raw[start..]);
    parts
}

fn split_top_level_type(raw: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut start = 0;
    for (index, ch) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            _ if ch == delimiter
                && paren_depth == 0
                && brace_depth == 0
                && bracket_depth == 0
                && angle_depth == 0 =>
            {
                parts.push(&raw[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&raw[start..]);
    parts
}

fn find_top_level_char(raw: &str, target: char) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => {
                if target == '(' && paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    return Some(index);
                }
                paren_depth += 1;
            }
            ')' => {
                if target == ')' && paren_depth == 1 && brace_depth == 0 && bracket_depth == 0 {
                    return Some(index);
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            '{' => {
                if target == '{' && paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    return Some(index);
                }
                brace_depth += 1;
            }
            '}' => {
                if target == '}' && paren_depth == 0 && brace_depth == 1 && bracket_depth == 0 {
                    return Some(index);
                }
                brace_depth = brace_depth.saturating_sub(1);
            }
            '[' => {
                if target == '[' && paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    return Some(index);
                }
                bracket_depth += 1;
            }
            ']' => {
                if target == ']' && paren_depth == 0 && brace_depth == 0 && bracket_depth == 1 {
                    return Some(index);
                }
                bracket_depth = bracket_depth.saturating_sub(1);
            }
            _ if ch == target && paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn find_matching_paren(raw: &str, open_index: usize) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    for (index, ch) in raw
        .char_indices()
        .skip_while(|(index, _)| *index < open_index)
    {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                if paren_depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_matching_angle(raw: &str, open_index: usize) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let mut angle_depth = 0usize;
    for (index, ch) in raw
        .char_indices()
        .skip_while(|(index, _)| *index < open_index)
    {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '<' => angle_depth += 1,
            '>' => {
                angle_depth = angle_depth.saturating_sub(1);
                if angle_depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_matching_brace(raw: &str, open_index: usize) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    for (index, ch) in raw
        .char_indices()
        .skip_while(|(index, _)| *index < open_index)
    {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' if paren_depth == 0 => brace_depth += 1,
            '}' if paren_depth == 0 => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_matching_square(raw: &str, open_index: usize) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in raw
        .char_indices()
        .skip_while(|(index, _)| *index < open_index)
    {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' if paren_depth == 0 && brace_depth == 0 => bracket_depth += 1,
            ']' if paren_depth == 0 && brace_depth == 0 => {
                bracket_depth = bracket_depth.saturating_sub(1);
                if bracket_depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_last_top_level_char(raw: &str, target: char) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut found = None;
    for (index, ch) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => {
                if target == '[' && paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    found = Some(index);
                }
                bracket_depth += 1;
            }
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ if ch == target && paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                found = Some(index)
            }
            _ => {}
        }
    }
    found
}

fn is_wrapped_in_parens(raw: &str) -> bool {
    raw.starts_with('(')
        && raw.ends_with(')')
        && matches!(find_matching_paren(raw, 0), Some(close) if close == raw.len() - 1)
}

fn find_compare_operator(raw: &str) -> Option<(CompareOp, usize)> {
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let chars: Vec<(usize, char)> = raw.char_indices().collect();
    let mut cursor = 0;
    while cursor < chars.len() {
        let (index, ch) = chars[cursor];
        if escaped {
            escaped = false;
            cursor += 1;
            continue;
        }
        if ch == '\\' && in_string {
            escaped = true;
            cursor += 1;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            cursor += 1;
            continue;
        }
        if in_string {
            cursor += 1;
            continue;
        }
        match ch {
            '(' => {
                paren_depth += 1;
                cursor += 1;
                continue;
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                cursor += 1;
                continue;
            }
            '{' => {
                brace_depth += 1;
                cursor += 1;
                continue;
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                cursor += 1;
                continue;
            }
            '[' => {
                bracket_depth += 1;
                cursor += 1;
                continue;
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                cursor += 1;
                continue;
            }
            _ => {}
        }
        if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
            if let Some((_, next)) = chars.get(cursor + 1) {
                match (ch, *next) {
                    ('=', '=') => return Some((CompareOp::Eq, index)),
                    ('!', '=') => return Some((CompareOp::Ne, index)),
                    ('<', '=') => return Some((CompareOp::Le, index)),
                    ('>', '=') => return Some((CompareOp::Ge, index)),
                    _ => {}
                }
            }
            match ch {
                '<' => return Some((CompareOp::Lt, index)),
                '>' => return Some((CompareOp::Gt, index)),
                _ => {}
            }
        }
        cursor += 1;
    }
    None
}

fn skip_blank_lines(lines: &[&str], index: &mut usize) {
    while *index < lines.len() && lines[*index].trim().is_empty() {
        *index += 1;
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
