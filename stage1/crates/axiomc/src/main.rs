use axiomc::diagnostics::Diagnostic;
use axiomc::new_project::create_project;
use axiomc::project::{build_project, check_project, project_capabilities, run_project};
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
