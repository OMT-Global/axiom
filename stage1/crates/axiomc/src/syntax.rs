use crate::diagnostics::Diagnostic;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: TypeName,
    pub body: Vec<Stmt>,
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
    Return {
        expr: Expr,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum TypeName {
    Int,
    Bool,
    String,
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
        if trimmed.starts_with("fn ") {
            functions.push(parse_function(&lines, &mut index, path)?);
            continue;
        }
        stmts.push(parse_stmt(&lines, &mut index, path, false)?);
    }
    Ok(Program { functions, stmts })
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
        if trimmed.starts_with("fn ") {
            return Err(Diagnostic::new(
                "parse",
                "stage1 bootstrap only supports top-level function declarations",
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
        "stage1 bootstrap currently supports let, print, if/else, while, and return statements inside blocks"
    } else {
        "stage1 bootstrap currently supports top-level fn, let, print, if/else, and while statements"
    };
    Err(Diagnostic::new("parse", message)
        .with_path(path.display().to_string())
        .with_span(line_no, 1))
}

fn parse_function(lines: &[&str], index: &mut usize, path: &Path) -> Result<Function, Diagnostic> {
    let line_no = *index + 1;
    let trimmed = lines[*index].trim();
    let header = trimmed.strip_prefix("fn ").ok_or_else(|| {
        Diagnostic::new("parse", "invalid function declaration")
            .with_path(path.display().to_string())
            .with_span(line_no, 1)
    })?;
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
    validate_ident(name, path, line_no, 4)?;
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
        line: line_no,
        column: 1,
    })
}

fn parse_params(raw: &str, path: &Path, line_no: usize) -> Result<Vec<Param>, Diagnostic> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut params = Vec::new();
    for param_text in split_top_level(raw, ',') {
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
    match raw {
        "int" => Ok(TypeName::Int),
        "bool" => Ok(TypeName::Bool),
        "string" => Ok(TypeName::String),
        _ => Err(Diagnostic::new("parse", format!("unknown type {raw:?}"))
            .with_path(path.display().to_string())
            .with_span(line_no, column)),
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
    if is_wrapped_in_parens(raw) {
        return parse_expr(&raw[1..raw.len() - 1], path, line_no, column + 1);
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
            _ if ch == delimiter && paren_depth == 0 => {
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
                if target == '(' && paren_depth == 0 {
                    return Some(index);
                }
                paren_depth += 1;
            }
            ')' => {
                if target == ')' && paren_depth == 1 {
                    return Some(index);
                }
                paren_depth = paren_depth.saturating_sub(1);
            }
            _ if ch == target && paren_depth == 0 => return Some(index),
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

fn is_wrapped_in_parens(raw: &str) -> bool {
    raw.starts_with('(')
        && raw.ends_with(')')
        && matches!(find_matching_paren(raw, 0), Some(close) if close == raw.len() - 1)
}

fn find_compare_operator(raw: &str) -> Option<(CompareOp, usize)> {
    let mut in_string = false;
    let mut escaped = false;
    let mut paren_depth = 0usize;
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
            _ => {}
        }
        if paren_depth == 0 {
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
