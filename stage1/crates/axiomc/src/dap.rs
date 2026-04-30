use crate::diagnostics::Diagnostic;
use crate::codegen::NativeBackendKind;
use crate::project::{BuildOptions, build_project_with_options};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

const DAP_CAPABILITIES_SCHEMA_VERSION: &str = "axiom.stage1.dap.v1";
const MAX_DAP_FRAME_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct DebugAdapter {
    next_seq: i64,
    session: Option<DebugSession>,
}

#[derive(Debug, Clone)]
struct DebugSession {
    project: String,
    package: Option<String>,
    binary: String,
    debug_map: String,
    breakable_lines: BTreeSet<SourceLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourceLine {
    source: String,
    line: u64,
}

#[derive(Debug, Deserialize)]
struct DebugMap {
    schema_version: String,
    mappings: Vec<DebugMapping>,
}

#[derive(Debug, Deserialize)]
struct DebugMapping {
    source: String,
    line: u64,
}

impl DebugAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    fn seq(&mut self) -> i64 {
        self.next_seq += 1;
        self.next_seq
    }
}

pub fn dap_capabilities() -> Value {
    json!({
        "schema_version": DAP_CAPABILITIES_SCHEMA_VERSION,
        "supportsConfigurationDoneRequest": true,
        "supportsSetVariable": false,
        "supportsStepBack": false,
        "supportsSteppingGranularity": false,
        "supportsTerminateRequest": true,
        "supportsEvaluateForHovers": false,
        "supportsLoadedSourcesRequest": true,
        "supportsReadMemoryRequest": false,
        "exceptionBreakpointFilters": [],
    })
}

pub fn handle_request(adapter: &mut DebugAdapter, request: Value) -> Vec<Value> {
    let command = request
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let request_seq = request.get("seq").and_then(Value::as_i64).unwrap_or(0);
    let arguments = request
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    match command.as_str() {
        "initialize" => vec![success_response(
            adapter,
            request_seq,
            &command,
            dap_capabilities(),
        )],
        "launch" => match launch(adapter, &arguments) {
            Ok(body) => vec![
                success_response(adapter, request_seq, &command, body),
                event(adapter, "initialized", json!({})),
            ],
            Err(error) => vec![error_response(adapter, request_seq, &command, error)],
        },
        "setBreakpoints" => match set_breakpoints(adapter, &arguments) {
            Ok(body) => vec![success_response(adapter, request_seq, &command, body)],
            Err(error) => vec![error_response(adapter, request_seq, &command, error)],
        },
        "configurationDone" | "disconnect" | "terminate" => {
            vec![success_response(adapter, request_seq, &command, json!({}))]
        }
        "threads" => vec![success_response(
            adapter,
            request_seq,
            &command,
            json!({"threads": [{"id": 1, "name": "main"}]}),
        )],
        "loadedSources" => vec![success_response(
            adapter,
            request_seq,
            &command,
            loaded_sources_body(adapter),
        )],
        "stackTrace" => vec![success_response(
            adapter,
            request_seq,
            &command,
            json!({"stackFrames": [], "totalFrames": 0}),
        )],
        "scopes" => vec![success_response(
            adapter,
            request_seq,
            &command,
            json!({"scopes": []}),
        )],
        "variables" => vec![success_response(
            adapter,
            request_seq,
            &command,
            json!({"variables": []}),
        )],
        _ => vec![error_response(
            adapter,
            request_seq,
            &command,
            Diagnostic::new(
                "dap",
                format!("unsupported DAP command `{command}` in stage1 adapter"),
            ),
        )],
    }
}

pub fn serve_dap<R: Read, W: Write>(reader: R, mut writer: W) -> Result<(), Diagnostic> {
    let mut reader = BufReader::new(reader);
    let mut adapter = DebugAdapter::new();
    while let Some(message) = read_dap_message(&mut reader)? {
        let request: Value = serde_json::from_slice(&message)
            .map_err(|err| Diagnostic::new("dap", format!("invalid DAP JSON request: {err}")))?;
        for response in handle_request(&mut adapter, request) {
            write_dap_message(&mut writer, &response)?;
        }
    }
    Ok(())
}

pub fn run_stdio<R: BufRead, W: Write>(reader: R, writer: W) -> Result<(), Diagnostic> {
    serve_dap(reader, writer)
}

fn launch(adapter: &mut DebugAdapter, arguments: &Value) -> Result<Value, Diagnostic> {
    adapter.session = None;
    let program = arguments
        .get("program")
        .and_then(Value::as_str)
        .ok_or_else(|| Diagnostic::new("dap", "launch requires arguments.program"))?;
    let package = arguments
        .get("package")
        .and_then(Value::as_str)
        .map(str::to_string);
    let output = build_project_with_options(
        Path::new(program),
        &BuildOptions {
            backend: NativeBackendKind::GeneratedRust,
            target: None,
            package: package.clone(),
            debug: true,
        },
    )?;
    let debug_map = output
        .debug_map
        .clone()
        .ok_or_else(|| Diagnostic::new("dap", "debug launch did not produce a debug map"))?;
    let breakable_lines = load_breakable_lines(Path::new(&debug_map))?;
    adapter.session = Some(DebugSession {
        project: program.to_string(),
        package,
        binary: output.binary.clone(),
        debug_map: debug_map.clone(),
        breakable_lines,
    });
    Ok(json!({
        "project": program,
        "package": adapter.session.as_ref().and_then(|session| session.package.as_deref()),
        "binary": output.binary,
        "debugMap": debug_map,
        "statementCount": output.statement_count,
    }))
}

fn set_breakpoints(adapter: &DebugAdapter, arguments: &Value) -> Result<Value, Diagnostic> {
    let source = arguments
        .get("source")
        .and_then(|source| source.get("path"))
        .and_then(Value::as_str)
        .ok_or_else(|| Diagnostic::new("dap", "setBreakpoints requires source.path"))?;
    let canonical_source = canonical_source_text(source);
    let breakpoints = arguments
        .get("breakpoints")
        .and_then(Value::as_array)
        .ok_or_else(|| Diagnostic::new("dap", "setBreakpoints requires breakpoints[]"))?;
    let response_breakpoints = breakpoints
        .iter()
        .map(|breakpoint| {
            let line = breakpoint.get("line").and_then(Value::as_u64).unwrap_or(0);
            let verified = adapter
                .session
                .as_ref()
                .map(|session| {
                    session.breakable_lines.contains(&SourceLine {
                        source: canonical_source.clone(),
                        line,
                    })
                })
                .unwrap_or(false);
            if verified {
                json!({"verified": true, "source": {"path": canonical_source}, "line": line})
            } else {
                json!({
                    "verified": false,
                    "source": {"path": canonical_source},
                    "line": line,
                    "message": "No stage1 debug mapping exists for this source line"
                })
            }
        })
        .collect::<Vec<_>>();
    Ok(json!({"breakpoints": response_breakpoints}))
}

fn loaded_sources_body(adapter: &DebugAdapter) -> Value {
    let Some(session) = &adapter.session else {
        return json!({"sources": []});
    };
    let sources = session
        .breakable_lines
        .iter()
        .map(|line| line.source.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|path| json!({"path": path, "name": source_name(&path)}))
        .collect::<Vec<_>>();
    json!({
        "sources": sources,
        "project": session.project,
        "binary": session.binary,
        "debugMap": session.debug_map,
    })
}

fn load_breakable_lines(path: &Path) -> Result<BTreeSet<SourceLine>, Diagnostic> {
    let content = fs::read_to_string(path).map_err(|err| {
        Diagnostic::new(
            "dap",
            format!("failed to read debug map {}: {err}", path.display()),
        )
    })?;
    let map: DebugMap = serde_json::from_str(&content).map_err(|err| {
        Diagnostic::new(
            "dap",
            format!("failed to parse debug map {}: {err}", path.display()),
        )
    })?;
    if map.schema_version != "axiom.stage1.debug_map.v1" {
        return Err(Diagnostic::new(
            "dap",
            format!("unsupported debug map schema `{}`", map.schema_version),
        ));
    }
    Ok(map
        .mappings
        .into_iter()
        .map(|mapping| SourceLine {
            source: canonical_source_text(&mapping.source),
            line: mapping.line,
        })
        .collect())
}

fn canonical_source_text(source: &str) -> String {
    PathBuf::from(source)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(source))
        .display()
        .to_string()
}

fn source_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn success_response(
    adapter: &mut DebugAdapter,
    request_seq: i64,
    command: &str,
    body: Value,
) -> Value {
    json!({
        "seq": adapter.seq(),
        "type": "response",
        "request_seq": request_seq,
        "success": true,
        "command": command,
        "body": body,
    })
}

fn error_response(
    adapter: &mut DebugAdapter,
    request_seq: i64,
    command: &str,
    error: Diagnostic,
) -> Value {
    json!({
        "seq": adapter.seq(),
        "type": "response",
        "request_seq": request_seq,
        "success": false,
        "command": command,
        "message": error.message,
        "body": {"error": error},
    })
}

fn event(adapter: &mut DebugAdapter, name: &str, body: Value) -> Value {
    json!({
        "seq": adapter.seq(),
        "type": "event",
        "event": name,
        "body": body,
    })
}

fn read_dap_message<R: BufRead>(reader: &mut R) -> Result<Option<Vec<u8>>, Diagnostic> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let count = reader
            .read_line(&mut line)
            .map_err(|err| Diagnostic::new("dap", format!("failed to read DAP header: {err}")))?;
        if count == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().map_err(|err| {
                Diagnostic::new("dap", format!("invalid DAP Content-Length: {err}"))
            })?);
        }
    }
    let length = content_length
        .ok_or_else(|| Diagnostic::new("dap", "DAP message missing Content-Length header"))?;
    if length > MAX_DAP_FRAME_SIZE {
        return Err(Diagnostic::new(
            "dap",
            format!(
                "DAP message Content-Length {length} exceeds maximum frame size {MAX_DAP_FRAME_SIZE}"
            ),
        ));
    }
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .map_err(|err| Diagnostic::new("dap", format!("failed to read DAP body: {err}")))?;
    Ok(Some(body))
}

fn write_dap_message<W: Write>(writer: &mut W, message: &Value) -> Result<(), Diagnostic> {
    let body = serde_json::to_vec(message)
        .map_err(|err| Diagnostic::new("dap", format!("failed to serialize DAP message: {err}")))?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())
        .map_err(|err| Diagnostic::new("dap", format!("failed to write DAP header: {err}")))?;
    writer
        .write_all(&body)
        .map_err(|err| Diagnostic::new("dap", format!("failed to write DAP body: {err}")))?;
    writer
        .flush()
        .map_err(|err| Diagnostic::new("dap", format!("failed to flush DAP output: {err}")))
}
