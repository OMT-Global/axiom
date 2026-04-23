use axiomc::diagnostics::Diagnostic;
use axiomc::json_contract;
use axiomc::new_project::create_project;
use axiomc::project::{
    BuildOptions, BuildOutput, CheckOptions, RunOptions, TestOptions, build_project_with_options,
    check_project_with_options, project_capabilities, run_project_tests_with_options,
    run_project_with_options,
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
    /// Create a new stage1 package with axiom.toml, axiom.lock, and starter source.
    New {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    /// Check a stage1 package or workspace member without building a binary.
    Check {
        path: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
    },
    /// Build a stage1 package into generated Rust and a native binary.
    Build {
        path: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        debug: bool,
        #[arg(long)]
        target: Option<String>,
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
    },
    /// Build and run a stage1 package native binary.
    Run {
        path: PathBuf,
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
    },
    /// Discover, build, and run package test entrypoints.
    Test {
        path: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        filter: Option<String>,
        #[arg(short = 'p', long = "package")]
        package: Option<String>,
    },
    /// Inspect manifest capability requirements.
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
                    for warning in &output.warnings {
                        eprintln!("{warning}");
                    }
                    eprintln!("OK");
                }
                0
            }
            Err(error) => print_error("check", error, json),
        },
        Command::Build {
            path,
            json,
            debug,
            target,
            package,
        } => {
            match build_project_with_options(
                &path,
                &BuildOptions {
                    target,
                    package: package.clone(),
                    debug,
                },
            ) {
                Ok(output) => {
                    if json {
                        println!("{}", json_contract::build_success(&path, &output));
                    } else {
                        for line in build_summary_lines(&output) {
                            eprintln!("{line}");
                        }
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
                        "passed: {} failed: {} skipped: {} duration: {} ms",
                        output.passed, output.failed, output.skipped, output.duration_ms
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
                        0
                    } else {
                        let payload = json_contract::caps_success(&project, &capabilities);
                        match json_contract::to_pretty_string(&payload) {
                            Ok(output) => {
                                println!("{output}");
                                0
                            }
                            Err(error) => print_error("caps", error, false),
                        }
                    }
                }
                Err(error) => print_error("caps", error, json),
            }
        }
    };
    std::process::exit(code);
}

fn build_summary_lines(output: &BuildOutput) -> Vec<String> {
    let mut lines = vec![format!("wrote {}", output.binary)];
    if let Some(debug_map) = &output.debug_map {
        lines.push(format!("wrote debug map {debug_map}"));
    }
    lines
}

fn print_error(command: &str, error: Diagnostic, json: bool) -> i32 {
    if json {
        println!("{}", json_contract::error(command, &error));
    } else {
        eprintln!("{error}");
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn help_describes_supported_stage1_workflows() {
        let help = Cli::command().render_long_help().to_string();
        assert!(help.contains("Create a new stage1 package"));
        assert!(help.contains("Check a stage1 package or workspace member"));
        assert!(help.contains("Build a stage1 package into generated Rust"));
        assert!(help.contains("Build and run a stage1 package native binary"));
        assert!(help.contains("Discover, build, and run package test entrypoints"));
        assert!(help.contains("Inspect manifest capability requirements"));
    }

    fn build_output(debug_map: Option<String>) -> BuildOutput {
        BuildOutput {
            manifest: String::from("axiom.toml"),
            entry: String::from("src/main.ax"),
            binary: String::from("dist/app"),
            generated_rust: String::from("target/main.rs"),
            debug_map,
            statement_count: 1,
            target: None,
            debug: true,
            cache_hits: 0,
            cache_misses: 1,
            duration_ms: 1,
            packages: Vec::new(),
        }
    }

    #[test]
    fn build_summary_mentions_debug_map_when_available() {
        assert_eq!(
            build_summary_lines(&build_output(Some(String::from(
                "target/main.debug-map.json"
            )))),
            vec![
                String::from("wrote dist/app"),
                String::from("wrote debug map target/main.debug-map.json"),
            ]
        );
    }

    #[test]
    fn build_summary_omits_debug_map_for_release_builds() {
        assert_eq!(
            build_summary_lines(&build_output(None)),
            vec![String::from("wrote dist/app")]
        );
    }
}
