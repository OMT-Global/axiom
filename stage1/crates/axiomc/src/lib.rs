pub mod codegen;
pub mod diagnostics;
pub mod hir;
pub mod lockfile;
pub mod manifest;
pub mod mir;
pub mod new_project;
pub mod project;
pub mod syntax;

#[cfg(test)]
mod tests {
    use crate::codegen::render_rust;
    use crate::hir;
    use crate::lockfile::render_lockfile;
    use crate::manifest::{
        BuildSection, CapabilityConfig, Manifest, PackageSection, capability_descriptors,
        load_manifest, render_manifest,
    };
    use crate::mir;
    use crate::new_project::create_project;
    use crate::project::{build_project, check_project, project_capabilities};
    use crate::syntax::parse_program;
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn new_project_writes_manifest_lockfile_and_source() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("demo");
        create_project(&project, Some("demo-app")).expect("create project");
        assert!(project.join("axiom.toml").exists());
        assert!(project.join("axiom.lock").exists());
        assert!(project.join("src/main.ax").exists());
    }

    #[test]
    fn parser_lowers_functions_calls_and_while() {
        let source = "fn banner(name: string): string {\nreturn \"hello \" + name\n}\n\nfn lucky(base: int): int {\nreturn base + 2\n}\n\nfn is_ready(value: int): bool {\nreturn value == 42\n}\n\nlet answer: int = lucky(40)\nlet ready: bool = is_ready(answer)\nwhile false {\nprint \"never\"\n}\nif ready {\nprint banner(\"from stage1\")\n} else {\nprint \"bad\"\n}\nprint answer\nprint ready\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        assert_eq!(parsed.functions.len(), 3);
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        assert_eq!(mir.functions.len(), 3);
        assert_eq!(mir.statement_count(), 12);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("fn banner(name: String) -> String {"));
        assert!(rendered.contains("return format!(\"{}{}\", String::from(\"hello \"), name);"));
        assert!(rendered.contains("let answer: i64 = lucky(40);"));
        assert!(rendered.contains("let ready: bool = is_ready(answer);"));
        assert!(rendered.contains("while false {"));
        assert!(rendered.contains("if ready {"));
        assert!(rendered.contains("println!(\"{}\", banner(String::from(\"from stage1\")));"));
        assert!(rendered.contains("println!(\"{}\", ready);"));
    }

    #[test]
    fn build_project_emits_native_binary() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("native");
        create_project(&project, Some("native-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn banner(name: string): string {\nreturn \"hello \" + name\n}\n\nfn lucky(base: int): int {\nreturn base + 2\n}\n\nfn is_ready(value: int): bool {\nreturn value == 42\n}\n\nlet answer: int = lucky(40)\nlet ready: bool = is_ready(answer)\nwhile false {\nprint \"never\"\n}\nif ready {\nprint banner(\"from stage1\")\n} else {\nprint \"broken\"\n}\nprint answer\nprint ready\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        assert!(Path::new(&built.binary).exists());
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "hello from stage1\n42\ntrue\n"
        );
    }

    #[test]
    fn check_project_rejects_dependencies_in_bootstrap() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("deps");
        create_project(&project, Some("deps-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[dependencies]\nhttp = \"0.1.0\"\n",
                render_manifest("deps-app")
            ),
        )
        .expect("write manifest");
        let manifest = Manifest {
            package: PackageSection {
                name: String::from("deps-app"),
                version: String::from("0.1.0"),
            },
            dependencies: BTreeMap::new(),
            workspace: None,
            build: BuildSection {
                entry: String::from("src/main.ax"),
                out_dir: String::from("dist"),
            },
            capabilities: CapabilityConfig::default(),
        };
        fs::write(
            project.join("axiom.lock"),
            render_lockfile(&manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        let error = check_project(&project).expect_err("dependencies should fail");
        assert!(error.message.contains("does not support dependencies"));
    }

    #[test]
    fn capability_view_reflects_manifest_flags() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("caps");
        create_project(&project, Some("caps-app")).expect("create project");
        let manifest = load_manifest(&project).expect("load manifest");
        let caps = capability_descriptors(&manifest.capabilities);
        assert_eq!(caps.len(), 6);
        assert!(caps.iter().all(|cap| !cap.enabled));
        let project_caps = project_capabilities(&project).expect("project capabilities");
        assert_eq!(project_caps.len(), 6);
    }

    #[test]
    fn check_project_rejects_use_after_string_move() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("moves");
        create_project(&project, Some("moves-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let greeting: string = \"hello\"\nlet alias: string = greeting\nprint alias\nprint greeting\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("use after move should fail");
        assert!(error.message.contains("use of moved value"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_branch_move_followed_by_outer_use() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("branch-moves");
        create_project(&project, Some("branch-moves-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let greeting: string = \"hello\"\nlet ready: bool = true\nif ready {\nlet alias: string = greeting\nprint alias\n} else {\nprint \"skip\"\n}\nprint greeting\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("conditional move should fail");
        assert!(error.message.contains("use of moved value"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_allows_copy_reuse_after_binding() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("copy");
        create_project(&project, Some("copy-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let count: int = 21\nlet duplicate: int = count\nprint count + duplicate\n",
        )
        .expect("write source");
        let output = check_project(&project).expect("copy values should be reusable");
        assert_eq!(output.statement_count, 3);
    }

    #[test]
    fn check_project_rejects_type_mismatch() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("types");
        create_project(&project, Some("types-app")).expect("create project");
        fs::write(project.join("src/main.ax"), "let count: int = \"nope\"\n")
            .expect("write source");
        let error = check_project(&project).expect_err("type mismatch should fail");
        assert!(error.message.contains("expects int, got string"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_non_bool_if_condition() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("if-types");
        create_project(&project, Some("if-types-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let answer: int = 42\nif answer {\nprint answer\n}\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("if condition should require bool");
        assert!(error.message.contains("if condition expects bool, got int"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_return_outside_function() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("return-top");
        create_project(&project, Some("return-top-app")).expect("create project");
        fs::write(project.join("src/main.ax"), "return 42\n").expect("write source");
        let error = check_project(&project).expect_err("top-level return should fail");
        assert!(
            error
                .message
                .contains("return is only valid inside a function")
        );
        assert_eq!(error.kind, "control");
    }

    #[test]
    fn check_project_rejects_undefined_function_call() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("missing-call");
        create_project(&project, Some("missing-call-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let answer: int = lucky(40)\nprint answer\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("missing function should fail");
        assert!(error.message.contains("undefined function"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_wrong_function_arity() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("arity");
        create_project(&project, Some("arity-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn lucky(base: int): int {\nreturn base + 2\n}\n\nlet answer: int = lucky()\nprint answer\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("wrong arity should fail");
        assert!(error.message.contains("expects 1 arguments, got 0"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_function_return_mismatch() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("return-mismatch");
        create_project(&project, Some("return-mismatch-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn bad(): int {\nreturn \"nope\"\n}\n\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("return mismatch should fail");
        assert!(error.message.contains("return expects int, got string"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_missing_function_return() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("missing-return");
        create_project(&project, Some("missing-return-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn maybe(value: bool): int {\nif value {\nreturn 1\n}\n}\n\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("missing return should fail");
        assert!(error.message.contains("does not return along all paths"));
        assert_eq!(error.kind, "control");
    }
}
