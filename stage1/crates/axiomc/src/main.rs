use axiomc::diagnostics::Diagnostic;
use axiomc::new_project::create_project;
use axiomc::project::{
    build_project, check_project, project_capabilities, run_project, run_project_tests,
};
use clap::{Parser, Subcommand};
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "axiomc", about = "Axiom stage1 bootstrap compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    New {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    Check {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Build {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Run {
        path: PathBuf,
    },
    Test {
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Caps {
        path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::New { path, name } => match create_project(&path, name.as_deref()) {
            Ok(()) => {
                println!("initialized stage1 project in {}", path.display());
                0
            }
            Err(error) => print_error(error, false),
        },
        Command::Check { path, json } => match check_project(&path) {
            Ok(output) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&json!({
                            "ok": true,
                            "command": "check",
                            "project": path.display().to_string(),
                            "manifest": output.manifest,
                            "entry": output.entry,
                            "statement_count": output.statement_count,
                            "capabilities": output.capabilities,
                            "packages": output.packages,
                        }))
                        .expect("json")
                    );
                } else {
                    eprintln!("OK");
                }
                0
            }
            Err(error) => print_error(error, json),
        },
        Command::Build { path, json } => match build_project(&path) {
            Ok(output) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&json!({
                            "ok": true,
                            "command": "build",
                            "project": path.display().to_string(),
                            "manifest": output.manifest,
                            "entry": output.entry,
                            "binary": output.binary,
                            "generated_rust": output.generated_rust,
                            "statement_count": output.statement_count,
                            "packages": output.packages,
                        }))
                        .expect("json")
                    );
                } else {
                    eprintln!("wrote {}", output.binary);
                }
                0
            }
            Err(error) => print_error(error, json),
        },
        Command::Run { path } => match run_project(&path) {
            Ok(code) => code,
            Err(error) => print_error(error, false),
        },
        Command::Test { path, json } => match run_project_tests(&path) {
            Ok(output) => {
                let ok = output.failed == 0;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&json!({
                            "ok": ok,
                            "command": "test",
                            "project": path.display().to_string(),
                            "manifest": output.manifest,
                            "packages": output.packages,
                            "passed": output.passed,
                            "failed": output.failed,
                            "cases": output.cases,
                        }))
                        .expect("json")
                    );
                } else {
                    for case in &output.cases {
                        let status = if case.ok { "PASS" } else { "FAIL" };
                        eprintln!("{status} {} ({})", case.name, case.entry);
                        if let Some(error) = &case.error {
                            eprintln!("  {}", error);
                        }
                    }
                    eprintln!("passed: {} failed: {}", output.passed, output.failed);
                }
                if ok { 0 } else { 1 }
            }
            Err(error) => print_error(error, json),
        },
        Command::Caps { path, json } => {
            let project = path.unwrap_or_else(|| PathBuf::from("."));
            match project_capabilities(&project) {
                Ok(capabilities) => {
                    let payload = json!({
                        "ok": true,
                        "command": "caps",
                        "project": project.display().to_string(),
                        "capabilities": capabilities,
                    });
                    if json {
                        println!("{}", serde_json::to_string(&payload).expect("json"));
                    } else {
                        println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
                    }
                    0
                }
                Err(error) => print_error(error, json),
            }
        }
    };
    std::process::exit(code);
}

fn print_error(error: Diagnostic, json: bool) -> i32 {
    if json {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "ok": false,
                "error": error,
            }))
            .expect("json")
        );
    } else {
        eprintln!("{error}");
    }
    1
}
