use axiomc::diagnostics::Diagnostic;
use axiomc::json_contract;
use axiomc::new_project::create_project;
use axiomc::project::{
    BuildOptions, CheckOptions, RunOptions, TestOptions, build_project_with_options,
    check_project_with_options, project_capabilities, run_project_with_options,
    run_project_tests_with_options,
};
use clap::{Parser, Subcommand};
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
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
    },
    Build {
        path: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        target: Option<String>,
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
    },
    Run {
        path: PathBuf,
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
    },
    Test {
        path: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        filter: Option<String>,
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
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
            Err(error) => print_error("new", error, false),
        },
        Command::Check {
            path,
            json,
            package,
        } => match check_project_with_options(
            &path,
            &CheckOptions {
                package: package.clone(),
            },
        ) {
            Ok(output) => {
                if json {
                    println!("{}", json_contract::check_success(&path, &output));
                } else {
                    eprintln!("OK");
                }
                0
            }
            Err(error) => print_error("check", error, json),
        },
        Command::Build {
            path,
            json,
            target,
            package,
        } => {
            match build_project_with_options(
                &path,
                &BuildOptions {
                    target,
                    package: package.clone(),
                },
            ) {
                Ok(output) => {
                    if json {
                        println!("{}", json_contract::build_success(&path, &output));
                    } else {
                        eprintln!("wrote {}", output.binary);
                    }
                    0
                }
                Err(error) => print_error("build", error, json),
            }
        }
        Command::Run { path, package } => match run_project_with_options(
            &path,
            &RunOptions {
                package: package.clone(),
            },
        ) {
            Ok(code) => code,
            Err(error) => print_error("run", error, false),
        },
        Command::Test {
            path,
            json,
            filter,
            package,
        } => match run_project_tests_with_options(
            &path,
            &TestOptions {
                filter: filter.clone(),
                package: package.clone(),
            },
        ) {
            Ok(output) => {
                let ok = output.failed == 0;
                if json {
                    println!(
                        "{}",
                        json_contract::test_success(&path, filter.as_deref(), &output)
                    );
                } else {
                    for case in &output.cases {
                        let status = if case.ok { "PASS" } else { "FAIL" };
                        eprintln!("{status} {} ({})", case.name, case.entry);
                        if let Some(error) = &case.error {
                            eprintln!("  {}", error);
                        }
                        eprintln!("  duration: {} ms", case.duration_ms);
                    }
                    eprintln!(
                        "passed: {} failed: {} duration: {} ms",
                        output.passed, output.failed, output.duration_ms
                    );
                }
                if ok { 0 } else { 1 }
            }
            Err(error) => print_error("test", error, json),
        },
        Command::Caps { path, json } => {
            let project = path.unwrap_or_else(|| PathBuf::from("."));
            match project_capabilities(&project) {
                Ok(capabilities) => {
                    if json {
                        println!("{}", json_contract::caps_success(&project, &capabilities));
                    } else {
                        let payload = json_contract::caps_success(&project, &capabilities);
                        println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
                    }
                    0
                }
                Err(error) => print_error("caps", error, json),
            }
        }
    };
    std::process::exit(code);
}

fn print_error(command: &str, error: Diagnostic, json: bool) -> i32 {
    if json {
        println!("{}", json_contract::error(command, &error));
    } else {
        eprintln!("{error}");
    }
    1
}
