pub mod codegen;
pub mod diagnostics;
pub mod hir;
pub mod lockfile;
pub mod manifest;
pub mod mir;
pub mod new_project;
pub mod project;
pub mod stdlib;
pub mod syntax;

#[cfg(test)]
mod tests {
    use crate::codegen::render_rust;
    use crate::hir;
    use crate::lockfile::{render_lockfile, render_lockfile_for_project};
    use crate::manifest::{TestTarget, capability_descriptors, load_manifest, render_manifest};
    use crate::mir;
    use crate::new_project::create_project;
    use crate::project::{build_project, check_project, project_capabilities, run_project_tests};
    use crate::syntax::parse_program;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    fn render_manifest_with_capabilities(
        name: &str,
        fs: bool,
        net: bool,
        process: bool,
        env: bool,
        clock: bool,
        crypto: bool,
    ) -> String {
        format!(
            "[package]\nname = {name:?}\nversion = \"0.1.0\"\n\n[build]\nentry = \"src/main.ax\"\nout_dir = \"dist\"\n\n[capabilities]\nfs = {fs}\nnet = {net}\nprocess = {process}\nenv = {env}\nclock = {clock}\ncrypto = {crypto}\n"
        )
    }

    fn write_process_fixture(dir: &Path) -> String {
        #[cfg(windows)]
        {
            let path = dir.join("status.cmd");
            fs::write(&path, "@echo off\r\nexit /b 7\r\n").expect("write process fixture");
            path.to_string_lossy().into_owned()
        }
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;

            let path = dir.join("status.sh");
            fs::write(&path, "#!/bin/sh\nexit 7\n").expect("write process fixture");
            let mut permissions = fs::metadata(&path)
                .expect("read process fixture metadata")
                .permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).expect("chmod process fixture");
            path.to_string_lossy().into_owned()
        }
    }

    #[test]
    fn new_project_writes_manifest_lockfile_and_source() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("demo");
        create_project(&project, Some("demo-app")).expect("create project");
        assert!(project.join("axiom.toml").exists());
        assert!(project.join("axiom.lock").exists());
        assert!(project.join("src/main.ax").exists());
        assert!(project.join("src/main_test.ax").exists());
        assert!(project.join("src/main_test.stdout").exists());
        let manifest = load_manifest(&project).expect("load manifest");
        assert_eq!(manifest.tests, Vec::<TestTarget>::new());
    }

    #[test]
    fn parser_lowers_functions_calls_and_while() {
        let source = "fn banner(name: string): string {\nreturn \"hello \" + name\n}\n\nfn lucky(base: int): int {\nreturn base + 2\n}\n\nfn is_ready(value: int): bool {\nreturn value == 42\n}\n\nlet answer: int = lucky(40)\nlet ready: bool = is_ready(answer)\nwhile false {\nprint \"never\"\n}\nif ready {\nprint banner(\"from stage1\")\n} else {\nprint \"bad\"\n}\nprint answer\nprint ready\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        assert_eq!(parsed.functions.len(), 3);
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        assert_eq!(mir.functions.len(), 3);
        assert_eq!(mir.statement_count(), 11);
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
    fn parser_lowers_struct_literals_and_field_access() {
        let source = "struct BuildInfo {\nname: string\ncount: int\n}\n\nfn count_of(info: BuildInfo): int {\nreturn info.count\n}\n\nlet info: BuildInfo = BuildInfo { name: \"stage1\", count: 42 }\nprint count_of(info)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        assert_eq!(parsed.structs.len(), 1);
        let hir = hir::lower(&parsed).expect("lower");
        assert_eq!(hir.structs.len(), 1);
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("struct BuildInfo {"));
        assert!(rendered.contains("name: String,"));
        assert!(rendered.contains("count: i64,"));
        assert!(rendered.contains(
            "let info: BuildInfo = BuildInfo { name: String::from(\"stage1\"), count: 42 };"
        ));
        assert!(rendered.contains("return (info).count;"));
    }

    #[test]
    fn parser_lowers_arrays_and_indexing() {
        let source = "fn answer(values: [int]): int {\nreturn values[1]\n}\n\nlet values: [int] = [40, 42]\nprint answer(values)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("fn answer(values: Vec<i64>) -> i64 {"));
        assert!(rendered.contains("return axiom_array_get(&values, 1);"));
        assert!(rendered.contains("let values: Vec<i64> = vec![40, 42];"));
        assert!(rendered.contains("println!(\"{}\", answer(values));"));
    }

    #[test]
    fn parser_lowers_array_slices() {
        let source = "fn tail(values: &[int]): &[int] {\nreturn values[1:]\n}\n\nfn string_tail_len(values: &[string]): int {\nlet rest: &[string] = values[1:]\nreturn len(rest)\n}\n\nlet values: [int] = [3, 7, 9, 11]\nlet window: &[int] = tail(values[:])\nprint first(window)\nprint last(window)\nprint len(window)\nlet labels: [string] = [\"build\", \"test\", \"ship\"]\nprint string_tail_len(labels[:])\nlet words: [string] = [\"alpha\", \"beta\"]\nprint first(words)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("fn tail<'a>(values: &'a [i64]) -> &'a [i64] {"));
        assert!(rendered.contains("return axiom_slice_view(values, Some(1), None);"));
        assert!(rendered.contains("fn string_tail_len<'a>(values: &'a [String]) -> i64 {"));
        assert!(
            rendered.contains("let window: &[i64] = tail(axiom_slice_view(&values, None, None));")
        );
        assert!(
            rendered
                .contains(
                    "println!(\"{}\", { let values = window; let index = 0; axiom_array_get(values, index) });"
                )
        );
        assert!(
            rendered.contains(
                "println!(\"{}\", { let values = window; let index = axiom_last_index(values.len()); axiom_array_get(values, index) });"
            )
        );
        assert!(rendered.contains("return (rest).len() as i64;"));
        assert!(
            rendered
                .contains(
                    "println!(\"{}\", { let values = words; let index = 0; axiom_array_take(values, index) });"
                )
        );
    }

    #[test]
    fn parser_lowers_borrowed_structs_and_enums() {
        let source = "struct Window {\nview: &[int]\n}\n\nenum Snapshot {\nWindow(Window)\nNamed { window: Window }\n}\n\nfn tail(values: &[int]): Window {\nreturn Window { view: values[1:] }\n}\n\nfn read(snapshot: Snapshot): int {\nmatch snapshot {\nWindow(window) {\nreturn first(window.view)\n}\nNamed { window } {\nreturn last(window.view)\n}\n}\n}\n\nlet numbers: [int] = [3, 7, 9, 11]\nlet window: Window = tail(numbers[:])\nprint first(window.view)\nprint read(Named { window: tail(numbers[:]) })\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("struct Window<'a> {"));
        assert!(rendered.contains("view: &'a [i64],"));
        assert!(rendered.contains("enum Snapshot<'a> {"));
        assert!(rendered.contains("Window(Window<'a>),"));
        assert!(rendered.contains("window: Window<'a>,"));
        assert!(rendered.contains("fn tail<'a>(values: &'a [i64]) -> Window<'a> {"));
        assert!(rendered.contains("fn read<'a>(snapshot: Snapshot<'a>) -> i64 {"));
        assert!(
            rendered
                .contains("let window: Window<'_> = tail(axiom_slice_view(&numbers, None, None));")
        );
        assert!(
            rendered.contains(
                "println!(\"{}\", read(Snapshot::Named { window: tail(axiom_slice_view(&numbers, None, None)) }));"
            )
        );
    }

    #[test]
    fn parser_lowers_tuples_and_tuple_indexing() {
        let source = "fn label(pair: (int, string)): string {\nreturn pair.1\n}\n\nlet pair: (int, string) = (7, \"stage1 tuples\")\nprint pair.0\nprint label(pair)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("fn label(pair: (i64, String)) -> String {"));
        assert!(rendered.contains("return (pair).1;"));
        assert!(
            rendered.contains("let pair: (i64, String) = (7, String::from(\"stage1 tuples\"));")
        );
        assert!(rendered.contains("println!(\"{}\", (pair).0);"));
        assert!(rendered.contains("println!(\"{}\", label(pair));"));
    }

    #[test]
    fn parser_lowers_maps_and_indexing() {
        let source =
            "let scores: {string: int} = {\"build\": 7, \"deploy\": 9}\nprint scores[\"deploy\"]\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("let scores: HashMap<String, i64> = HashMap::from(["));
        assert!(rendered.contains("(String::from(\"build\"), 7)"));
        assert!(rendered.contains("(String::from(\"deploy\"), 9)"));
        assert!(
            rendered
                .contains("println!(\"{}\", axiom_map_get(&scores, &String::from(\"deploy\")));")
        );
    }

    #[test]
    fn parser_lowers_option_and_result() {
        let source = "struct BuildInfo {\nlabel: string\n}\n\nfn maybe(ready: bool): Option<BuildInfo> {\nif ready {\nreturn Some(BuildInfo { label: \"ok\" })\n}\nreturn None\n}\n\nfn load(ready: bool): Result<BuildInfo, string> {\nif ready {\nreturn Ok(BuildInfo { label: \"built\" })\n}\nreturn Err(\"boom\")\n}\n\nfn describe(value: Option<BuildInfo>): string {\nmatch value {\nSome(info) {\nreturn info.label\n}\nNone {\nreturn \"none\"\n}\n}\n}\n\nfn render(result: Result<BuildInfo, string>): string {\nmatch result {\nOk(info) {\nreturn info.label\n}\nErr(message) {\nreturn message\n}\n}\n}\n\nprint describe(maybe(true))\nprint render(load(false))\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("fn maybe(ready: bool) -> Option<BuildInfo> {"));
        assert!(
            rendered.contains("return Option::Some(BuildInfo { label: String::from(\"ok\") });")
        );
        assert!(rendered.contains("return Option::None;"));
        assert!(rendered.contains("fn load(ready: bool) -> Result<BuildInfo, String> {"));
        assert!(
            rendered.contains("return Result::Ok(BuildInfo { label: String::from(\"built\") });")
        );
        assert!(rendered.contains("return Result::Err(String::from(\"boom\"));"));
        assert!(rendered.contains("Option::Some(info) => {"));
        assert!(rendered.contains("Option::None => {"));
        assert!(rendered.contains("Result::Ok(info) => {"));
        assert!(rendered.contains("Result::Err(message) => {"));
    }

    #[test]
    fn parser_lowers_enums_and_match() {
        let source = "enum Status {\nReady\nFailed\n}\n\nfn label(status: Status): string {\nmatch status {\nReady {\nreturn \"ready\"\n}\nFailed {\nreturn \"failed\"\n}\n}\n}\n\nlet status: Status = Ready\nprint label(status)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        assert_eq!(parsed.enums.len(), 1);
        let hir = hir::lower(&parsed).expect("lower");
        assert_eq!(hir.enums.len(), 1);
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("enum Status {"));
        assert!(rendered.contains("Ready,"));
        assert!(rendered.contains("Failed,"));
        assert!(rendered.contains("match status {"));
        assert!(rendered.contains("Status::Ready => {"));
        assert!(rendered.contains("Status::Failed => {"));
        assert!(rendered.contains("let status: Status = Status::Ready;"));
    }

    #[test]
    fn parser_lowers_payload_enums_and_match_bindings() {
        let source = "enum Message {\nText(string)\nCount(int)\n}\n\nfn render(message: Message): string {\nmatch message {\nText(text) {\nreturn text\n}\nCount(count) {\nreturn \"count\"\n}\n}\n}\n\nlet message: Message = Text(\"ready\")\nprint render(message)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        assert_eq!(
            parsed.enums[0].variants[0].payload_tys,
            vec![crate::syntax::TypeName::String]
        );
        let crate::syntax::Stmt::Match { arms, .. } = &parsed.functions[0].body[0] else {
            panic!("expected match statement");
        };
        assert_eq!(arms[0].variant, "Text");
        assert_eq!(arms[0].bindings, vec![String::from("text")]);
        assert_eq!(arms[1].variant, "Count");
        assert_eq!(arms[1].bindings, vec![String::from("count")]);
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("Text(String),"));
        assert!(rendered.contains("Count(i64),"));
        assert!(rendered.contains("Message::Text(text) => {"));
        assert!(
            rendered.contains("let message: Message = Message::Text(String::from(\"ready\"));")
        );
    }

    #[test]
    fn parser_lowers_multi_payload_enums_and_match_bindings() {
        let source = "enum Message {\nPair(int, string)\nText(string)\n}\n\nfn render(message: Message): string {\nmatch message {\nPair(count, label) {\nprint count\nreturn label\n}\nText(text) {\nreturn text\n}\n}\n}\n\nlet message: Message = Pair(7, \"tuple payload\")\nprint render(message)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        assert_eq!(
            parsed.enums[0].variants[0].payload_tys,
            vec![
                crate::syntax::TypeName::Int,
                crate::syntax::TypeName::String
            ]
        );
        let crate::syntax::Stmt::Match { arms, .. } = &parsed.functions[0].body[0] else {
            panic!("expected match statement");
        };
        assert_eq!(
            arms[0].bindings,
            vec![String::from("count"), String::from("label")]
        );
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("Pair(i64, String),"));
        assert!(rendered.contains("Message::Pair(count, label) => {"));
        assert!(
            rendered.contains(
                "let message: Message = Message::Pair(7, String::from(\"tuple payload\"));"
            )
        );
    }

    #[test]
    fn parser_lowers_named_payload_enums_and_match_bindings() {
        let source = "enum Message {\nJob { id: int, label: string }\nText(string)\n}\n\nfn render(message: Message): string {\nmatch message {\nJob { id, label } {\nprint id\nreturn label\n}\nText(text) {\nreturn text\n}\n}\n}\n\nlet message: Message = Job { id: 7, label: \"named payload\" }\nprint render(message)\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        assert_eq!(
            parsed.enums[0].variants[0].payload_names,
            vec![String::from("id"), String::from("label")]
        );
        let crate::syntax::Stmt::Match { arms, .. } = &parsed.functions[0].body[0] else {
            panic!("expected match statement");
        };
        assert!(arms[0].is_named);
        assert_eq!(
            arms[0].bindings,
            vec![String::from("id"), String::from("label")]
        );
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("Job {"));
        assert!(rendered.contains("id: i64,"));
        assert!(rendered.contains("label: String,"));
        assert!(rendered.contains("Message::Job { id, label } => {"));
        assert!(rendered.contains(
            "let message: Message = Message::Job { id: 7, label: String::from(\"named payload\") };"
        ));
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
    fn build_project_emits_native_binary_with_structs() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("structs");
        create_project(&project, Some("structs-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "struct BuildInfo {\nlabel: string\ncount: int\n}\n\nlet info: BuildInfo = BuildInfo { label: \"hello from stage1\", count: 42 }\nprint info.count\nprint info.label\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "42\nhello from stage1\n"
        );
    }

    #[test]
    fn build_project_emits_native_binary_with_arrays() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("arrays");
        create_project(&project, Some("arrays-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn answer(values: [int]): int {\nreturn values[1]\n}\n\nlet values: [int] = [40, 42]\nprint answer(values)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_array_slices() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slices");
        create_project(&project, Some("slices-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn tail(values: &[int]): &[int] {\nreturn values[1:]\n}\n\nfn string_tail_len(values: &[string]): int {\nlet rest: &[string] = values[1:]\nreturn len(rest)\n}\n\nlet values: [int] = [3, 7, 9, 11]\nlet window: &[int] = tail(values[:])\nprint first(window)\nprint last(window)\nprint len(window)\nlet labels: [string] = [\"build\", \"test\", \"ship\"]\nprint string_tail_len(labels[:])\nlet words: [string] = [\"alpha\", \"beta\"]\nprint first(words)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "7\n11\n3\n2\nalpha\n"
        );
    }

    #[test]
    fn build_project_emits_native_binary_with_wrapped_borrow_returns() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("wrapped-borrow-returns");
        create_project(&project, Some("wrapped-borrow-returns-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn maybe_tail(values: &[int], ready: bool): Option<&[int]> {\nif ready {\nreturn Some(values[1:])\n}\nreturn None\n}\n\nfn describe(values: &[int]): (Option<&[int]>, int) {\nreturn (Some(values[1:]), len(values))\n}\n\nlet numbers: [int] = [3, 7, 9, 11]\nmatch maybe_tail(numbers[:], true) {\nSome(window) {\nprint first(window)\n}\nNone {\nprint 0\n}\n}\nlet summary: (Option<&[int]>, int) = describe(numbers[:])\nmatch summary.0 {\nSome(window) {\nprint last(window)\n}\nNone {\nprint 0\n}\n}\nprint summary.1\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n11\n4\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_match_payload_borrow_returns() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("match-payload-borrow-returns");
        create_project(&project, Some("match-payload-borrow-returns-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn choose(values: &[int]): Option<&[int]> {\nmatch Some(values[1:]) {\nSome(window) {\nreturn Some(window)\n}\nNone {\nreturn None\n}\n}\n}\n\nlet numbers: [int] = [3, 7, 9, 11]\nmatch choose(numbers[:]) {\nSome(window) {\nprint first(window)\n}\nNone {\nprint 0\n}\n}\nprint first(numbers)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n3\n");
    }

    #[test]
    fn build_project_emits_native_binary_after_match_temporary_borrow_ends() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("match-temporary-borrow-release");
        create_project(&project, Some("match-temporary-borrow-release-app"))
            .expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nmatch Some(values[:]) {\nSome(window) {\nprint len(window)\n}\nNone {\nprint 0\n}\n}\nprint first(values)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "2\nalpha\n");
    }

    #[test]
    fn build_project_emits_native_binary_after_if_false_dead_branch_is_ignored() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("if-false-dead-branch");
        create_project(&project, Some("if-false-dead-branch-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nif false {\nlet view: &[string] = values[:]\nprint len(view)\nprint first(values)\n} else {\nprint 0\n}\nprint first(values)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "0\nalpha\n");
    }

    #[test]
    fn build_project_emits_native_binary_after_while_false_dead_body_is_ignored() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("while-false-dead-body");
        create_project(&project, Some("while-false-dead-body-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nwhile false {\nlet view: &[string] = values[:]\nprint len(view)\nprint first(values)\n}\nprint first(values)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "alpha\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_multi_param_borrow_returns() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("multi-param-borrow-returns");
        create_project(&project, Some("multi-param-borrow-returns-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn choose(left: &[int], right: &[int], pick_left: bool): Option<&[int]> {\nif pick_left {\nreturn Some(left[1:])\n}\nreturn Some(right[1:])\n}\n\nlet left: [int] = [3, 7, 9]\nlet right: [int] = [40, 42, 44]\nmatch choose(left[:], right[:], false) {\nSome(window) {\nprint first(window)\n}\nNone {\nprint 0\n}\n}\nprint first(left)\nprint first(right)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n3\n40\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_borrowed_named_shapes() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("borrowed-named-shapes");
        create_project(&project, Some("borrowed-named-shapes-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "struct Window {\nview: &[int]\n}\n\nenum Snapshot {\nWindow(Window)\nNamed { window: Window }\n}\n\nfn tail(values: &[int]): Window {\nreturn Window { view: values[1:] }\n}\n\nfn read(snapshot: Snapshot): int {\nmatch snapshot {\nWindow(window) {\nreturn first(window.view)\n}\nNamed { window } {\nreturn last(window.view)\n}\n}\n}\n\nlet numbers: [int] = [3, 7, 9, 11]\nlet window: Window = tail(numbers[:])\nprint first(window.view)\nprint read(Window(tail(numbers[:])))\nprint read(Named { window: tail(numbers[:]) })\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n7\n11\n");
    }

    #[test]
    fn build_project_emits_native_binary_after_branch_local_slice_borrow_ends() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("borrow-scope");
        create_project(&project, Some("borrow-scope-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nif true {\nlet view: &[string] = values[:]\nprint len(view)\n}\nprint first(values)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "2\nalpha\n");
    }

    #[test]
    fn build_project_emits_native_binary_after_wrapped_borrow_scope_ends() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("wrapped-borrow-scope");
        create_project(&project, Some("wrapped-borrow-scope-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nif true {\nlet wrapped: (&[string], int) = (values[:], 1)\nprint len(wrapped.0)\n}\nprint first(values)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "2\nalpha\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_tuples() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("tuples");
        create_project(&project, Some("tuples-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn label(pair: (int, string)): string {\nreturn pair.1\n}\n\nlet pair: (int, string) = (7, \"stage1 tuples\")\nprint pair.0\nprint label(pair)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "7\nstage1 tuples\n"
        );
    }

    #[test]
    fn build_project_emits_native_binary_with_maps() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("maps");
        create_project(&project, Some("maps-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let scores: {string: int} = {\"build\": 7, \"deploy\": 9}\nprint scores[\"deploy\"]\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "9\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_option_and_result() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("outcomes");
        create_project(&project, Some("outcomes-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"models.ax\"\n\nfn maybe_job(ready: bool): Option<Job> {\nif ready {\nreturn Some(Job { id: 7, label: \"queued\" })\n}\nreturn None\n}\n\nfn load_job(ready: bool): Result<Job, string> {\nif ready {\nreturn Ok(Job { id: 9, label: \"built\" })\n}\nreturn Err(\"boom\")\n}\n\nfn describe(job: Option<Job>): string {\nmatch job {\nSome(info) {\nreturn info.label\n}\nNone {\nreturn \"none\"\n}\n}\n}\n\nfn render(result: Result<Job, string>): string {\nmatch result {\nOk(info) {\nreturn info.label\n}\nErr(message) {\nreturn message\n}\n}\n}\n\nprint describe(maybe_job(true))\nprint describe(maybe_job(false))\nprint render(load_job(true))\nprint render(load_job(false))\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/models.ax"),
            "pub struct Job {\nid: int\nlabel: string\n}\n",
        )
        .expect("write models");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "queued\nnone\nbuilt\nboom\n"
        );
    }

    #[test]
    fn build_project_emits_native_binary_with_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("enums");
        create_project(&project, Some("enums-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Status {\nReady\nFailed\n}\n\nfn label(status: Status): string {\nmatch status {\nReady {\nreturn \"ready\"\n}\nFailed {\nreturn \"failed\"\n}\n}\n}\n\nlet status: Status = Ready\nprint label(status)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "ready\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_enum_field_match() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("enum-field-match");
        create_project(&project, Some("enum-field-match-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum JobState {\nQueued\nRunning\nDone\n}\n\nstruct Job {\nid: int\nstate: JobState\n}\n\nfn label(job: Job): string {\nmatch job.state {\nQueued {\nreturn \"queued\"\n}\nRunning {\nreturn \"running\"\n}\nDone {\nreturn \"done\"\n}\n}\n}\n\nlet job: Job = Job { id: 7, state: Running }\nprint job.id\nprint label(job)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "7\nrunning\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_payload_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("payload-enums");
        create_project(&project, Some("payload-enums-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nText(string)\nCount(int)\n}\n\nfn render(message: Message): string {\nmatch message {\nText(text) {\nreturn text\n}\nCount(count) {\nprint count\nreturn \"count\"\n}\n}\n}\n\nlet message: Message = Text(\"ready\")\nprint render(message)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "ready\n");
    }

    #[test]
    fn build_project_emits_native_binary_with_multi_payload_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("multi-payload-enums");
        create_project(&project, Some("multi-payload-enums-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nPair(int, string)\nText(string)\n}\n\nfn render(message: Message): string {\nmatch message {\nPair(count, label) {\nprint count\nreturn label\n}\nText(text) {\nreturn text\n}\n}\n}\n\nlet first: Message = Pair(7, \"multi payload\")\nprint render(first)\nlet second: Message = Text(\"payload enums\")\nprint render(second)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "7\nmulti payload\npayload enums\n"
        );
    }

    #[test]
    fn build_project_emits_native_binary_with_named_payload_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("named-payload-enums");
        create_project(&project, Some("named-payload-enums-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nJob { id: int, label: string }\nText(string)\n}\n\nfn render(message: Message): string {\nmatch message {\nJob { id, label } {\nprint id\nreturn label\n}\nText(text) {\nreturn text\n}\n}\n}\n\nlet first: Message = Job { id: 7, label: \"named payload\" }\nprint render(first)\nlet second: Message = Text(\"payload enums\")\nprint render(second)\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "7\nnamed payload\npayload enums\n"
        );
    }

    #[test]
    fn stage1_project_supports_local_path_dependencies() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("deps-app");
        let dependency = project.join("deps/core");
        create_project(&project, Some("deps-app")).expect("create project");
        create_project(&dependency, Some("core-lib")).expect("create dependency");
        fs::write(
            dependency.join("src/math.ax"),
            "pub fn answer(): int {\nreturn 42\n}\n",
        )
        .expect("write dependency source");
        let dependency_manifest = load_manifest(&dependency).expect("load dependency manifest");
        fs::write(
            dependency.join("axiom.lock"),
            render_lockfile_for_project(&dependency, &dependency_manifest)
                .expect("dependency lockfile"),
        )
        .expect("write dependency lockfile");
        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[dependencies]\ncore = {{ path = \"deps/core\" }}\n",
                render_manifest("deps-app")
            ),
        )
        .expect("write manifest");
        fs::write(
            project.join("src/main.ax"),
            "import \"core/math.ax\"\nprint answer()\n",
        )
        .expect("write root source");
        fs::write(
            project.join("src/main_test.ax"),
            "import \"core/math.ax\"\nprint answer()\n",
        )
        .expect("write root test");
        fs::write(project.join("src/main_test.stdout"), "42\n").expect("write expected stdout");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");

        check_project(&project).expect("check project");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");

        let tests = run_project_tests(&project).expect("run tests");
        assert_eq!(tests.passed, 1);
        assert_eq!(tests.failed, 0);
    }

    #[test]
    fn stage1_project_supports_workspace_members() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("workspace-root");
        let core = project.join("members/core");
        let util = project.join("members/util");
        create_project(&project, Some("workspace-root-app")).expect("create root project");
        create_project(&core, Some("workspace-core")).expect("create core member");
        create_project(&util, Some("workspace-util")).expect("create util member");

        fs::write(
            core.join("src/math.ax"),
            "pub fn answer(): int {\nreturn 42\n}\n",
        )
        .expect("write core source");
        fs::write(
            util.join("src/extra.ax"),
            "pub fn helper(): int {\nreturn 7\n}\n",
        )
        .expect("write util source");

        let core_manifest = load_manifest(&core).expect("load core manifest");
        fs::write(
            core.join("axiom.lock"),
            render_lockfile_for_project(&core, &core_manifest).expect("core lockfile"),
        )
        .expect("write core lockfile");
        let util_manifest = load_manifest(&util).expect("load util manifest");
        fs::write(
            util.join("axiom.lock"),
            render_lockfile_for_project(&util, &util_manifest).expect("util lockfile"),
        )
        .expect("write util lockfile");

        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[workspace]\nmembers = [\"members/core\", \"members/util\"]\n\n[dependencies]\ncore = {{ path = \"members/core\" }}\n",
                render_manifest("workspace-root-app")
            ),
        )
        .expect("write workspace manifest");
        fs::write(
            project.join("src/main.ax"),
            "import \"core/math.ax\"\nprint answer()\n",
        )
        .expect("write root source");
        fs::write(
            project.join("src/main_test.ax"),
            "import \"core/math.ax\"\nprint answer()\n",
        )
        .expect("write root test");
        fs::write(project.join("src/main_test.stdout"), "42\n").expect("write golden");

        let manifest = load_manifest(&project).expect("load root manifest");
        let lockfile = render_lockfile_for_project(&project, &manifest).expect("root lockfile");
        assert!(lockfile.contains("path:members/core"));
        assert!(lockfile.contains("path:members/util"));
        fs::write(project.join("axiom.lock"), lockfile).expect("write root lockfile");

        let checked = check_project(&project).expect("check workspace root");
        assert_eq!(checked.packages.len(), 3);
        let built = build_project(&project).expect("build workspace root");
        assert_eq!(built.packages.len(), 3);
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");

        let tests = run_project_tests(&project).expect("run workspace tests");
        assert_eq!(tests.packages.len(), 3);
        assert_eq!(tests.passed, 3);
        assert_eq!(tests.failed, 0);
    }

    #[test]
    fn workspace_members_must_appear_in_root_lockfile() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("workspace-lock");
        let core = project.join("members/core");
        let util = project.join("members/util");
        create_project(&project, Some("workspace-lock-app")).expect("create root project");
        create_project(&core, Some("workspace-lock-core")).expect("create core member");
        create_project(&util, Some("workspace-lock-util")).expect("create util member");

        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[workspace]\nmembers = [\"members/core\", \"members/util\"]\n\n[dependencies]\ncore = {{ path = \"members/core\" }}\n",
                render_manifest("workspace-lock-app")
            ),
        )
        .expect("write workspace manifest");
        fs::write(
            project.join("src/main.ax"),
            "import \"core/main.ax\"\nprint \"done\"\n",
        )
        .expect("write root source");

        let manifest = load_manifest(&project).expect("load root manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile(&manifest).expect("minimal lockfile"),
        )
        .expect("write incomplete lockfile");

        let error = check_project(&project).expect_err("workspace members should be locked");
        assert_eq!(error.kind, "lockfile");
        assert!(
            error
                .message
                .contains("axiom.lock does not match axiom.toml")
        );
    }

    #[test]
    fn workspace_members_reject_parent_traversal() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("workspace-invalid");
        create_project(&project, Some("workspace-invalid-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[workspace]\nmembers = [\"../outside\"]\n",
                render_manifest("workspace-invalid-app")
            ),
        )
        .expect("write manifest");

        let error = check_project(&project).expect_err("workspace member traversal should fail");
        assert_eq!(error.kind, "manifest");
        assert!(error.message.contains("must not use parent traversal"));
    }

    #[test]
    fn dependency_package_must_enable_its_own_capabilities() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("dep-cap-root");
        let dependency = project.join("deps/core");
        create_project(&project, Some("dep-cap-root-app")).expect("create root project");
        create_project(&dependency, Some("dep-cap-core")).expect("create dependency");

        fs::write(
            dependency.join("src/time.ax"),
            "pub fn tick(): int {\nreturn clock_now_ms()\n}\n",
        )
        .expect("write dependency source");
        let dependency_manifest = load_manifest(&dependency).expect("load dependency manifest");
        fs::write(
            dependency.join("axiom.lock"),
            render_lockfile_for_project(&dependency, &dependency_manifest)
                .expect("dependency lockfile"),
        )
        .expect("write dependency lockfile");

        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[dependencies]\ncore = {{ path = \"deps/core\" }}\n",
                render_manifest_with_capabilities(
                    "dep-cap-root-app",
                    false,
                    false,
                    false,
                    false,
                    true,
                    false,
                )
            ),
        )
        .expect("write root manifest");
        fs::write(
            project.join("src/main.ax"),
            "import \"core/time.ax\"\nprint tick()\n",
        )
        .expect("write root source");
        let manifest = load_manifest(&project).expect("load root manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("root lockfile"),
        )
        .expect("write root lockfile");

        let error = check_project(&project).expect_err("dependency capability should be required");
        assert_eq!(error.kind, "capability");
        assert!(
            error
                .path
                .as_ref()
                .is_some_and(|path| path.ends_with("deps/core/src/time.ax"))
        );
        assert!(
            error
                .message
                .contains("requires [capabilities].clock = true")
        );
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
    fn check_project_rejects_clock_intrinsic_without_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("clock-denied");
        create_project(&project, Some("clock-denied-app")).expect("create project");
        fs::write(project.join("src/main.ax"), "print clock_now_ms()\n").expect("write source");

        let error = check_project(&project).expect_err("clock capability should be required");
        assert_eq!(error.kind, "capability");
        assert!(
            error
                .message
                .contains("requires [capabilities].clock = true")
        );
    }

    #[test]
    fn check_project_rejects_env_intrinsic_without_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("env-denied");
        create_project(&project, Some("env-denied-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let value: Option<string> = env_get(\"PATH\")\nprint \"never\"\n",
        )
        .expect("write source");

        let error = check_project(&project).expect_err("env capability should be required");
        assert_eq!(error.kind, "capability");
        assert!(error.message.contains("requires [capabilities].env = true"));
    }

    #[test]
    fn check_project_rejects_fs_intrinsic_without_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("fs-denied");
        create_project(&project, Some("fs-denied-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "match fs_read(\"missing.txt\") {\nSome(value) {\nprint value\n}\nNone {\nprint \"none\"\n}\n}\n",
        )
        .expect("write source");

        let error = check_project(&project).expect_err("fs capability should be required");
        assert_eq!(error.kind, "capability");
        assert!(error.message.contains("requires [capabilities].fs = true"));
    }

    #[test]
    fn check_project_rejects_net_intrinsic_without_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("net-denied");
        create_project(&project, Some("net-denied-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "match net_resolve(\"localhost\") {\nSome(address) {\nprint address\n}\nNone {\nprint \"none\"\n}\n}\n",
        )
        .expect("write source");

        let error = check_project(&project).expect_err("net capability should be required");
        assert_eq!(error.kind, "capability");
        assert!(error.message.contains("requires [capabilities].net = true"));
    }

    #[test]
    fn check_project_rejects_process_intrinsic_without_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("process-denied");
        create_project(&project, Some("process-denied-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "print process_status(\"fixture\")\n",
        )
        .expect("write source");

        let error = check_project(&project).expect_err("process capability should be required");
        assert_eq!(error.kind, "capability");
        assert!(
            error
                .message
                .contains("requires [capabilities].process = true")
        );
    }

    #[test]
    fn check_project_rejects_crypto_intrinsic_without_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("crypto-denied");
        create_project(&project, Some("crypto-denied-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "print crypto_sha256(\"abc\")\n",
        )
        .expect("write source");

        let error = check_project(&project).expect_err("crypto capability should be required");
        assert_eq!(error.kind, "capability");
        assert!(
            error
                .message
                .contains("requires [capabilities].crypto = true")
        );
    }

    #[test]
    fn build_project_emits_native_binary_with_capability_intrinsics() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("capability-intrinsics");
        create_project(&project, Some("capability-intrinsics-app")).expect("create project");
        let fixture_path = project.join("fixture.txt");
        fs::write(&fixture_path, "fs ok\n").expect("write fs fixture");
        let process_path = write_process_fixture(&project);
        fs::write(
            project.join("axiom.toml"),
            render_manifest_with_capabilities(
                "capability-intrinsics-app",
                true,
                true,
                true,
                true,
                true,
                true,
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        fs::write(
            project.join("src/main.ax"),
            format!(
                "match fs_read({fixture:?}) {{\nSome(value) {{\nprint value\n}}\nNone {{\nprint \"missing\"\n}}\n}}\nmatch net_resolve(\"localhost\") {{\nSome(_address) {{\nprint true\n}}\nNone {{\nprint false\n}}\n}}\nprint process_status({process:?})\nprint crypto_sha256(\"abc\")\nlet now: int = clock_now_ms()\nprint now > 0\nmatch env_get(\"__AXIOM_STAGE1_MISSING__\") {{\nSome(value) {{\nprint value\n}}\nNone {{\nprint \"none\"\n}}\n}}\n",
                fixture = fixture_path.to_string_lossy(),
                process = process_path,
            ),
        )
        .expect("write source");
        fs::write(
            project.join("src/main_test.ax"),
            format!(
                "match fs_read({fixture:?}) {{\nSome(value) {{\nprint value\n}}\nNone {{\nprint \"missing\"\n}}\n}}\nmatch net_resolve(\"localhost\") {{\nSome(_address) {{\nprint true\n}}\nNone {{\nprint false\n}}\n}}\nprint process_status({process:?})\nprint crypto_sha256(\"abc\")\nlet now: int = clock_now_ms()\nprint now > 0\nmatch env_get(\"__AXIOM_STAGE1_MISSING__\") {{\nSome(value) {{\nprint value\n}}\nNone {{\nprint \"none\"\n}}\n}}\n",
                fixture = fixture_path.to_string_lossy(),
                process = process_path,
            ),
        )
        .expect("write test");
        fs::write(
            project.join("src/main_test.stdout"),
            "fs ok\n\ntrue\n7\nba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\ntrue\nnone\n",
        )
        .expect("write golden");

        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "fs ok\n\ntrue\n7\nba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\ntrue\nnone\n"
        );

        let tests = run_project_tests(&project).expect("run tests");
        assert_eq!(tests.passed, 1);
        assert_eq!(tests.failed, 0);
    }

    #[test]
    fn stage1_project_imports_synthetic_stdlib_time_module() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("stdlib-time-app");
        create_project(&project, Some("stdlib-time-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            render_manifest_with_capabilities(
                "stdlib-time-app",
                false,
                false,
                false,
                false,
                true,
                false,
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        fs::write(
            project.join("src/main.ax"),
            "import \"std/time.ax\"\nlet now: int = now_ms()\nprint now > 0\n",
        )
        .expect("write source");
        fs::write(
            project.join("src/main_test.ax"),
            "import \"std/time.ax\"\nlet now: int = now_ms()\nprint now > 0\n",
        )
        .expect("write test");
        fs::write(project.join("src/main_test.stdout"), "true\n").expect("write golden");

        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "true\n");

        let tests = run_project_tests(&project).expect("run tests");
        assert_eq!(tests.passed, 1);
        assert_eq!(tests.failed, 0);
    }

    #[test]
    fn stage1_project_rejects_stdlib_time_without_clock_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("stdlib-time-denied");
        create_project(&project, Some("stdlib-time-denied")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            render_manifest_with_capabilities(
                "stdlib-time-denied",
                false,
                false,
                false,
                false,
                false,
                false,
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        fs::write(
            project.join("src/main.ax"),
            "import \"std/time.ax\"\nlet now: int = now_ms()\nprint now > 0\n",
        )
        .expect("write source");

        let err = check_project(&project).expect_err("expected capability denial");
        assert!(
            err.message
                .contains("requires [capabilities].clock = true"),
            "unexpected diagnostic: {err:?}",
        );
    }

    #[test]
    fn stage1_project_imports_synthetic_stdlib_env_module() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("stdlib-env-app");
        create_project(&project, Some("stdlib-env-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            render_manifest_with_capabilities(
                "stdlib-env-app",
                false,
                false,
                false,
                true,
                false,
                false,
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        let source = "import \"std/env.ax\"\nmatch get_env(\"__AXIOM_STAGE1_MISSING__\") {\nSome(value) {\nprint value\n}\nNone {\nprint \"none\"\n}\n}\n";
        fs::write(project.join("src/main.ax"), source).expect("write source");
        fs::write(project.join("src/main_test.ax"), source).expect("write test");
        fs::write(project.join("src/main_test.stdout"), "none\n").expect("write golden");

        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .env_remove("__AXIOM_STAGE1_MISSING__")
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "none\n");

        let tests = run_project_tests(&project).expect("run tests");
        assert_eq!(tests.passed, 1);
        assert_eq!(tests.failed, 0);
    }

    #[test]
    fn stage1_project_rejects_stdlib_env_without_env_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("stdlib-env-denied");
        create_project(&project, Some("stdlib-env-denied")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            render_manifest_with_capabilities(
                "stdlib-env-denied",
                false,
                false,
                false,
                false,
                false,
                false,
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        fs::write(
            project.join("src/main.ax"),
            "import \"std/env.ax\"\nmatch get_env(\"X\") {\nSome(v) {\nprint v\n}\nNone {\nprint \"none\"\n}\n}\n",
        )
        .expect("write source");

        let err = check_project(&project).expect_err("expected capability denial");
        assert!(
            err.message.contains("requires [capabilities].env = true"),
            "unexpected diagnostic: {err:?}",
        );
    }

    #[test]
    fn stage1_project_imports_synthetic_stdlib_fs_module() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("stdlib-fs-app");
        create_project(&project, Some("stdlib-fs-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            render_manifest_with_capabilities(
                "stdlib-fs-app",
                true,
                false,
                false,
                false,
                false,
                false,
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        let fixture = project.join("src/fixture.txt");
        fs::write(&fixture, "hello stdlib fs\n").expect("write fixture");
        let fixture_literal = fixture.to_string_lossy().replace('\\', "\\\\");
        let source = format!(
            "import \"std/fs.ax\"\nmatch read_file(\"{fixture_literal}\") {{\nSome(value) {{\nprint value\n}}\nNone {{\nprint \"missing\"\n}}\n}}\n"
        );
        fs::write(project.join("src/main.ax"), &source).expect("write source");
        fs::write(project.join("src/main_test.ax"), &source).expect("write test");
        fs::write(project.join("src/main_test.stdout"), "hello stdlib fs\n\n")
            .expect("write golden");

        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "hello stdlib fs\n\n"
        );

        let tests = run_project_tests(&project).expect("run tests");
        assert_eq!(tests.passed, 1);
        assert_eq!(tests.failed, 0);
    }

    #[test]
    fn stage1_project_rejects_stdlib_fs_without_fs_capability() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("stdlib-fs-denied");
        create_project(&project, Some("stdlib-fs-denied")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            render_manifest_with_capabilities(
                "stdlib-fs-denied",
                false,
                false,
                false,
                false,
                false,
                false,
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        fs::write(
            project.join("axiom.lock"),
            render_lockfile_for_project(&project, &manifest).expect("lockfile"),
        )
        .expect("write lockfile");
        fs::write(
            project.join("src/main.ax"),
            "import \"std/fs.ax\"\nmatch read_file(\"x\") {\nSome(v) {\nprint v\n}\nNone {\nprint \"missing\"\n}\n}\n",
        )
        .expect("write source");

        let err = check_project(&project).expect_err("expected capability denial");
        assert!(
            err.message.contains("requires [capabilities].fs = true"),
            "unexpected diagnostic: {err:?}",
        );
    }

    #[test]
    fn stage1_project_rejects_unknown_stdlib_module() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("stdlib-unknown");
        create_project(&project, Some("stdlib-unknown")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"std/bogus.ax\"\nprint 1\n",
        )
        .expect("write source");

        let err = check_project(&project).expect_err("expected unknown stdlib module error");
        assert!(
            err.message.contains("unknown stdlib module"),
            "unexpected diagnostic: {err:?}",
        );
    }

    #[test]
    fn manifest_parses_test_targets() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("tests");
        create_project(&project, Some("tests-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[[tests]]\nname = \"math-smoke\"\nentry = \"src/math_test.ax\"\nstdout = \"42\\n\"\n",
                render_manifest("tests-app")
            ),
        )
        .expect("write manifest");
        let manifest = load_manifest(&project).expect("load manifest");
        assert_eq!(
            manifest.tests,
            vec![TestTarget {
                name: String::from("math-smoke"),
                entry: String::from("src/math_test.ax"),
                stdout: Some(String::from("42\n")),
            }]
        );
    }

    #[test]
    fn run_project_tests_executes_manifest_cases() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("runner");
        create_project(&project, Some("runner-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[[tests]]\nname = \"math-smoke\"\nentry = \"src/math_test.ax\"\nstdout = \"42\\n\"\n",
                render_manifest("runner-app")
            ),
        )
        .expect("write manifest");
        fs::write(
            project.join("src/math.ax"),
            "pub fn lucky(base: int): int {\nreturn base + 2\n}\n",
        )
        .expect("write module");
        fs::write(
            project.join("src/math_test.ax"),
            "import \"math.ax\"\nprint lucky(40)\n",
        )
        .expect("write test");

        let output = run_project_tests(&project).expect("run tests");
        assert_eq!(output.passed, 2);
        assert_eq!(output.failed, 0);
        assert_eq!(output.cases.len(), 2);
        let math_case = output
            .cases
            .iter()
            .find(|case| case.name == "math-smoke")
            .expect("math case");
        assert_eq!(math_case.stdout, "42\n");
        assert!(math_case.ok);
        assert!(
            output
                .cases
                .iter()
                .any(|case| case.entry == "src/main_test.ax")
        );
    }

    #[test]
    fn run_project_tests_reports_stdout_mismatch() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("runner-fail");
        create_project(&project, Some("runner-fail-app")).expect("create project");
        fs::write(
            project.join("axiom.toml"),
            format!(
                "{}\n[[tests]]\nname = \"math-smoke\"\nentry = \"src/math_test.ax\"\nstdout = \"99\\n\"\n",
                render_manifest("runner-fail-app")
            ),
        )
        .expect("write manifest");
        fs::write(
            project.join("src/math.ax"),
            "pub fn lucky(base: int): int {\nreturn base + 2\n}\n",
        )
        .expect("write module");
        fs::write(
            project.join("src/math_test.ax"),
            "import \"math.ax\"\nprint lucky(40)\n",
        )
        .expect("write test");

        let output = run_project_tests(&project).expect("run tests");
        assert_eq!(output.passed, 1);
        assert_eq!(output.failed, 1);
        let math_case = output
            .cases
            .iter()
            .find(|case| case.name == "math-smoke")
            .expect("math case");
        assert_eq!(math_case.stdout, "42\n");
        assert!(!math_case.ok);
        assert!(
            math_case
                .error
                .as_ref()
                .expect("error")
                .message
                .contains("stdout did not match")
        );
    }

    #[test]
    fn run_project_tests_discovers_src_suffix_cases() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("runner-discovery");
        create_project(&project, Some("runner-discovery-app")).expect("create project");
        fs::write(
            project.join("src/math.ax"),
            "pub fn lucky(base: int): int {\nreturn base + 2\n}\n",
        )
        .expect("write module");
        fs::write(
            project.join("src/math_test.ax"),
            "import \"math.ax\"\nprint lucky(40)\n",
        )
        .expect("write test");
        fs::write(project.join("src/math_test.stdout"), "42\n").expect("write golden");

        let output = run_project_tests(&project).expect("run tests");
        assert_eq!(output.passed, 2);
        assert_eq!(output.failed, 0);
        assert_eq!(output.cases.len(), 2);
        assert!(
            output
                .cases
                .iter()
                .any(|case| case.entry == "src/main_test.ax")
        );
        let math_case = output
            .cases
            .iter()
            .find(|case| case.entry == "src/math_test.ax")
            .expect("math test");
        assert_eq!(math_case.stdout, "42\n");
        assert!(math_case.ok);
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
    fn check_project_rejects_none_without_expected_option_context() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("none-context");
        create_project(&project, Some("none-context-app")).expect("create project");
        fs::write(project.join("src/main.ax"), "print None\n").expect("write source");
        let error = check_project(&project).expect_err("None should require an expected type");
        assert!(
            error
                .message
                .contains("None requires an expected Option<T> context")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_ok_without_expected_result_context() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("ok-context");
        create_project(&project, Some("ok-context-app")).expect("create project");
        fs::write(project.join("src/main.ax"), "print Ok(7)\n").expect("write source");
        let error = check_project(&project).expect_err("Ok should require an expected type");
        assert!(
            error
                .message
                .contains("Ok requires an expected Result<T, E> context")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_option_payload_type_mismatch() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("option-mismatch");
        create_project(&project, Some("option-mismatch-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let value: Option<int> = Some(\"nope\")\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("option payload mismatch should fail");
        assert!(
            error
                .message
                .contains("Option::Some expects payload type int, got string")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_result_payload_type_mismatch() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("result-mismatch");
        create_project(&project, Some("result-mismatch-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let value: Result<int, string> = Err(7)\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("result payload mismatch should fail");
        assert!(
            error
                .message
                .contains("Result::Err expects payload type string, got int")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_non_exhaustive_option_match() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("option-match");
        create_project(&project, Some("option-match-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn unwrap(value: Option<int>): int {\nmatch value {\nSome(count) {\nreturn count\n}\n}\n}\n\nprint 0\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("non-exhaustive option match should fail");
        assert!(error.message.contains("not exhaustive"));
        assert!(error.message.contains("None"));
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

    #[test]
    fn build_project_emits_native_binary_from_imported_modules() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("modules");
        create_project(&project, Some("modules-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"greetings.ax\"\nimport \"math.ax\"\n\nfn is_ready(value: int): bool {\nreturn value == 42\n}\n\nlet answer: int = lucky(40)\nlet ready: bool = is_ready(answer)\nif ready {\nprint banner(\"from modules\")\n} else {\nprint \"bad\"\n}\nprint answer\nprint ready\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/greetings.ax"),
            "pub fn banner(name: string): string {\nreturn prefix() + name\n}\n\nfn prefix(): string {\nreturn \"hello \"\n}\n",
        )
        .expect("write greetings");
        fs::write(
            project.join("src/math.ax"),
            "pub fn lucky(base: int): int {\nreturn bump(base)\n}\n\nfn bump(base: int): int {\nreturn base + 2\n}\n",
        )
        .expect("write math");
        let built = build_project(&project).expect("build imported modules");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "hello from modules\n42\ntrue\n"
        );
    }

    #[test]
    fn check_project_rejects_missing_import() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("missing-import");
        create_project(&project, Some("missing-import-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"nope.ax\"\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("missing import should fail");
        assert!(error.message.contains("missing import"));
        assert_eq!(error.kind, "import");
    }

    #[test]
    fn check_project_rejects_import_aliases_explicitly() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("import-alias");
        create_project(&project, Some("import-alias-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"math.ax\" as math\nprint \"skip\"\n",
        )
        .expect("write source");
        fs::write(
            project.join("src/math.ax"),
            "pub fn answer(): int {\nreturn 42\n}\n",
        )
        .expect("write module");

        let error = check_project(&project).expect_err("import aliases should fail");
        assert_eq!(error.kind, "parse");
        assert!(error.message.contains("does not support import aliases"));
    }

    #[test]
    fn check_project_rejects_re_exports_explicitly() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("re-export");
        create_project(&project, Some("re-export-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "pub use \"math.ax\"\nprint \"skip\"\n",
        )
        .expect("write source");

        let error = check_project(&project).expect_err("re-exports should fail");
        assert_eq!(error.kind, "parse");
        assert!(error.message.contains("does not support re-exports"));
    }

    #[test]
    fn check_project_rejects_namespace_qualified_calls_explicitly() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("qualified-call");
        create_project(&project, Some("qualified-call-app")).expect("create project");
        fs::write(project.join("src/main.ax"), "print math.answer()\n").expect("write source");

        let error = check_project(&project).expect_err("qualified calls should fail");
        assert_eq!(error.kind, "parse");
        assert!(error.message.contains("namespace-qualified calls"));
    }

    #[test]
    fn check_project_rejects_private_import_call() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("private-import");
        create_project(&project, Some("private-import-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"greetings.ax\"\nprint prefix()\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/greetings.ax"),
            "pub fn banner(name: string): string {\nreturn prefix() + name\n}\n\nfn prefix(): string {\nreturn \"hello \"\n}\n",
        )
        .expect("write greetings");
        let error = check_project(&project).expect_err("private import should fail");
        assert!(error.message.contains("is not exported"));
        assert_eq!(error.kind, "import");
    }

    #[test]
    fn check_project_rejects_imported_top_level_statements() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-module");
        create_project(&project, Some("bad-module-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"greetings.ax\"\nprint banner(\"x\")\n",
        )
        .expect("write main");
        fs::write(project.join("src/greetings.ax"), "print \"nope\"\n").expect("write greetings");
        let error = check_project(&project).expect_err("module top-level statements should fail");
        assert!(
            error.message.contains(
                "may only contain imports, struct declarations, enum declarations, and function declarations"
            )
        );
        assert_eq!(error.kind, "import");
    }

    #[test]
    fn check_project_rejects_circular_imports() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("cycle");
        create_project(&project, Some("cycle-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"a.ax\"\nprint \"skip\"\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/a.ax"),
            "import \"b.ax\"\npub fn call_a(): int {\nreturn call_b()\n}\n",
        )
        .expect("write a");
        fs::write(
            project.join("src/b.ax"),
            "import \"a.ax\"\npub fn call_b(): int {\nreturn call_a()\n}\n",
        )
        .expect("write b");
        let error = check_project(&project).expect_err("circular imports should fail");
        assert!(error.message.contains("circular import"));
        assert_eq!(error.kind, "import");
    }

    #[test]
    fn build_project_emits_native_binary_from_imported_public_structs() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("struct-modules");
        create_project(&project, Some("struct-modules-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"model.ax\"\n\nlet info: BuildInfo = BuildInfo { label: \"hello from modules\", count: 42 }\nprint info.count\nprint info.label\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/model.ax"),
            "pub struct BuildInfo {\nlabel: string\ncount: int\n}\n",
        )
        .expect("write model");
        let built = build_project(&project).expect("build imported structs");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "42\nhello from modules\n"
        );
    }

    #[test]
    fn check_project_rejects_missing_struct_field() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("missing-field");
        create_project(&project, Some("missing-field-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "struct BuildInfo {\nlabel: string\ncount: int\n}\n\nlet info: BuildInfo = BuildInfo { label: \"x\" }\nprint info.count\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("missing field should fail");
        assert!(error.message.contains("is missing field"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_field_access_on_non_struct() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-field-access");
        create_project(&project, Some("bad-field-access-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let answer: int = 42\nprint answer.count\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("field access should fail");
        assert!(
            error
                .message
                .contains("field access expects a struct value")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_mixed_array_literal_types() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-array-literal");
        create_project(&project, Some("bad-array-literal-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [int] = [1, true]\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("array literal should require matching types");
        assert!(
            error
                .message
                .contains("array literal expects matching element types")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_array_index_on_non_array() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-array-index");
        create_project(&project, Some("bad-array-index-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let answer: int = 42\nprint answer[0]\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("array index should require array");
        assert!(
            error
                .message
                .contains("index expects an array or map value")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_non_int_array_index() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-array-index-type");
        create_project(&project, Some("bad-array-index-type-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [int] = [1, 2]\nprint values[true]\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("array index should require int");
        assert!(error.message.contains("array index expects int"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_array_slice_on_non_array() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slice-non-array");
        create_project(&project, Some("slice-non-array-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let value: int = 42\nprint value[1:2]\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("slicing non-array should fail");
        assert!(
            error
                .message
                .contains("slice expects an array or slice value, got int")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_non_int_array_slice_bound() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slice-bound-type");
        create_project(&project, Some("slice-bound-type-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [int] = [1, 2, 3]\nprint values[true:2][0]\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("slice bound should require int");
        assert!(
            error
                .message
                .contains("array slice start expects int, got bool")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_non_copy_slice_index() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slice-move");
        create_project(&project, Some("slice-move-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"a\", \"b\", \"c\"]\nlet tail: &[string] = values[1:]\nprint tail[0]\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("non-copy slice indexing should fail");
        assert!(
            error
                .message
                .contains("borrowed slice indexing requires a Copy element type")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_slice_return_without_borrowed_param() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slice-return-owned");
        create_project(&project, Some("slice-return-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn tail(values: [int]): &[int] {\nreturn values[1:]\n}\n\nprint 0\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("slice returns should require a borrowed param");
        assert!(
            error
                .message
                .contains("borrowed return functions must take at least one borrowed parameter")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_slice_return_from_local_value() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slice-return-local");
        create_project(&project, Some("slice-return-local-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn tail(values: &[int]): &[int] {\nlet local: [int] = [7, 9, 11]\nreturn local[1:]\n}\n\nprint 0\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("local slice return should fail");
        assert!(error.message.contains(
            "returning borrowed values requires data derived from one of the borrowed parameters"
        ));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_wrapped_borrow_return_from_local_value() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("wrapped-borrow-return-local");
        create_project(&project, Some("wrapped-borrow-return-local-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn wrap(values: &[int]): Option<&[int]> {\nlet local: [int] = [7, 9, 11]\nreturn Some(local[1:])\n}\n\nprint 0\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("local wrapped borrow return should fail");
        assert!(error.message.contains(
            "returning borrowed values requires data derived from one of the borrowed parameters"
        ));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_wrapped_borrow_return_without_borrowed_params() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("wrapped-borrow-return-no-param");
        create_project(&project, Some("wrapped-borrow-return-no-param-app"))
            .expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn choose(values: [int]): Option<&[int]> {\nreturn Some(values[1:])\n}\n\nprint 0\n",
        )
        .expect("write source");
        let error = check_project(&project)
            .expect_err("borrowed returns should still require at least one borrowed param");
        assert!(
            error
                .message
                .contains("borrowed return functions must take at least one borrowed parameter")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_moving_owner_inside_match_while_temporary_borrow_is_live() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("match-temporary-borrow-move");
        create_project(&project, Some("match-temporary-borrow-move-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nmatch Some(values[:]) {\nSome(window) {\nprint len(window)\nprint first(values)\n}\nNone {\nprint 0\n}\n}\n",
        )
        .expect("write source");
        let error = check_project(&project)
            .expect_err("temporary match borrow should block owner move inside the arm");
        assert!(error.message.contains("cannot move"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_moving_owner_in_later_call_arg_after_temporary_borrow_arg() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("call-arg-temporary-borrow-move");
        create_project(&project, Some("call-arg-temporary-borrow-move-app"))
            .expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn consume(view: Option<&[string]>, values: [string]): string {\nreturn first(values)\n}\n\nlet values: [string] = [\"alpha\", \"beta\"]\nprint consume(Some(values[:]), values)\n",
        )
        .expect("write source");
        let error = check_project(&project)
            .expect_err("temporary borrow in an earlier call argument should block moving the owner later in the call");
        assert!(error.message.contains("cannot move"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_borrowing_owner_in_later_call_arg_after_move_arg() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("call-arg-move-then-borrow");
        create_project(&project, Some("call-arg-move-then-borrow-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn consume(values: [string], view: Option<&[string]>): string {\nreturn first(values)\n}\n\nlet values: [string] = [\"alpha\", \"beta\"]\nprint consume(values, Some(values[:]))\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err(
            "moving the owner first should still reject borrowing it later in the call",
        );
        assert!(error.message.contains("use of moved value"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_moving_owner_inside_while_while_local_borrow_is_live() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("while-live-borrow-move");
        create_project(&project, Some("while-live-borrow-move-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nwhile true {\nlet view: &[string] = values[:]\nprint len(view)\nprint first(values)\n}\n",
        )
        .expect("write source");
        let error = check_project(&project)
            .expect_err("loop-local borrow should block owner move inside the loop body");
        assert!(error.message.contains("cannot move"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_first_on_non_copy_slice() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slice-first-non-copy");
        create_project(&project, Some("slice-first-non-copy-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"a\", \"b\", \"c\"]\nlet tail: &[string] = values[1:]\nprint first(tail)\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("first on non-copy slice should fail");
        assert!(
            error
                .message
                .contains("first requires a Copy element type when called on a borrowed slice")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_moving_owned_array_while_slice_borrow_is_live() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("live-borrow-move");
        create_project(&project, Some("live-borrow-move-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nlet view: &[string] = values[:]\nprint len(view)\nprint first(values)\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("moving a borrowed owner should fail");
        assert!(
            error
                .message
                .contains("cannot move value \"values\" while borrowed slices are still live")
        );
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_moving_owner_while_tuple_wrapped_slice_is_live() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("tuple-wrapped-live-borrow");
        create_project(&project, Some("tuple-wrapped-live-borrow-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nlet wrapped: (&[string], int) = (values[:], 1)\nprint len(wrapped.0)\nprint first(values)\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("tuple-wrapped borrow should block owner move");
        assert!(
            error
                .message
                .contains("cannot move value \"values\" while borrowed slices are still live")
        );
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_moving_owner_while_option_wrapped_slice_is_live() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("option-wrapped-live-borrow");
        create_project(&project, Some("option-wrapped-live-borrow-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let values: [string] = [\"alpha\", \"beta\"]\nlet wrapped: Option<&[string]> = Some(values[:])\nmatch wrapped {\nSome(view) {\nprint len(view)\n}\nNone {\nprint 0\n}\n}\nprint first(values)\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("option-wrapped borrow should block owner move");
        assert!(
            error
                .message
                .contains("cannot move value \"values\" while borrowed slices are still live")
        );
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_moving_owner_while_struct_wrapped_slice_is_live() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("struct-wrapped-live-borrow");
        create_project(&project, Some("struct-wrapped-live-borrow-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "struct Window {\nview: &[string]\n}\n\nlet values: [string] = [\"alpha\", \"beta\"]\nlet window: Window = Window { view: values[:] }\nprint len(window.view)\nprint first(values)\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("struct-wrapped borrow should block owner move");
        assert!(
            error
                .message
                .contains("cannot move value \"values\" while borrowed slices are still live")
        );
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_moving_owner_while_enum_wrapped_slice_is_live() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("enum-wrapped-live-borrow");
        create_project(&project, Some("enum-wrapped-live-borrow-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Snapshot {\nWindow(&[string])\n}\n\nlet values: [string] = [\"alpha\", \"beta\"]\nlet snapshot: Snapshot = Window(values[:])\nmatch snapshot {\nWindow(view) {\nprint len(view)\n}\n}\nprint first(values)\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("enum-wrapped borrow should block owner move");
        assert!(
            error
                .message
                .contains("cannot move value \"values\" while borrowed slices are still live")
        );
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_mixed_map_literal_key_types() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-map-literal-keys");
        create_project(&project, Some("bad-map-literal-keys-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let scores: {string: int} = {\"build\": 7, 9: 10}\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("map literal should require matching key types");
        assert!(
            error
                .message
                .contains("map literal expects matching key types")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_unsupported_map_key_type() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-map-key-type");
        create_project(&project, Some("bad-map-key-type-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let bad: {[int]: int} = {[1, 2]: 7}\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("map type should reject unsupported key type");
        assert!(
            error
                .message
                .contains("map key type [int] is not supported")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_wrong_map_key_type_on_index() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-map-index-key");
        create_project(&project, Some("bad-map-index-key-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let scores: {string: int} = {\"build\": 7}\nprint scores[0]\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("map index should require matching key type");
        assert!(
            error
                .message
                .contains("map index expects key type string, got int")
        );
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_use_after_non_copy_map_index() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("map-move");
        create_project(&project, Some("map-move-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let labels: {string: string} = {\"build\": \"green\", \"deploy\": \"ready\"}\nprint labels[\"build\"]\nprint labels[\"deploy\"]\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("non-copy map index should consume owner");
        assert!(error.message.contains("use of moved value"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_tuple_index_on_non_tuple() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("bad-tuple-index");
        create_project(&project, Some("bad-tuple-index-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let answer: int = 42\nprint answer.0\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("tuple index should require tuple");
        assert!(error.message.contains("tuple index expects a tuple value"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_out_of_bounds_tuple_index() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("tuple-index-bounds");
        create_project(&project, Some("tuple-index-bounds-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let pair: (int, string) = (7, \"label\")\nprint pair.2\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("tuple index should enforce bounds");
        assert!(error.message.contains("tuple index 2 is out of bounds"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_use_after_non_copy_tuple_index() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("tuple-move");
        create_project(&project, Some("tuple-move-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let pair: (int, string) = (7, \"label\")\nprint pair.1\nprint pair.0\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("non-copy tuple index should consume owner");
        assert!(error.message.contains("use of moved value"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_use_after_non_copy_array_index() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("array-move");
        create_project(&project, Some("array-move-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "let labels: [string] = [\"a\", \"b\"]\nprint labels[0]\nprint labels[1]\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("non-copy array index should consume owner");
        assert!(error.message.contains("use of moved value"));
        assert_eq!(error.kind, "ownership");
    }

    #[test]
    fn check_project_rejects_non_exhaustive_match() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("non-exhaustive-match");
        create_project(&project, Some("non-exhaustive-match-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Status {\nReady\nFailed\n}\n\nfn label(status: Status): string {\nmatch status {\nReady {\nreturn \"ready\"\n}\n}\n}\n\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("match should be exhaustive");
        assert!(error.message.contains("not exhaustive"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_unknown_match_variant() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("unknown-match-variant");
        create_project(&project, Some("unknown-match-variant-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Status {\nReady\nFailed\n}\n\nfn label(status: Status): string {\nmatch status {\nUnknown {\nreturn \"nope\"\n}\nReady {\nreturn \"ready\"\n}\nFailed {\nreturn \"failed\"\n}\n}\n}\n\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("match should reject unknown variant");
        assert!(error.message.contains("has no variant"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_missing_payload_match_binding() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("missing-payload-binding");
        create_project(&project, Some("missing-payload-binding-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nText(string)\nCount(int)\n}\n\nfn render(message: Message): string {\nmatch message {\nText {\nreturn \"text\"\n}\nCount(count) {\nreturn \"count\"\n}\n}\n}\n\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("match should require payload binding");
        assert!(error.message.contains("expects 1 bindings, got 0"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_multi_payload_match_binding_count() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("multi-payload-binding-count");
        create_project(&project, Some("multi-payload-binding-count-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nPair(int, string)\nText(string)\n}\n\nfn render(message: Message): string {\nmatch message {\nPair(label) {\nreturn label\n}\nText(text) {\nreturn text\n}\n}\n}\n\nprint \"skip\"\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("match should enforce payload binding count");
        assert!(error.message.contains("expects 2 bindings, got 1"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_payload_constructor_type_mismatch() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("payload-constructor-type");
        create_project(&project, Some("payload-constructor-type-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nText(string)\nCount(int)\n}\n\nlet message: Message = Text(42)\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("payload constructor should typecheck");
        assert!(error.message.contains("expects payload type string"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_named_payload_constructor_with_positional_args() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("named-payload-constructor-positional");
        create_project(&project, Some("named-payload-constructor-positional-app"))
            .expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nJob { id: int, label: string }\n}\n\nlet message: Message = Job(7, \"x\")\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project)
            .expect_err("named payload variant should reject positional args");
        assert!(error.message.contains("requires named payload fields"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_named_payload_constructor_missing_field() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("named-payload-constructor-missing");
        create_project(&project, Some("named-payload-constructor-missing-app"))
            .expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nJob { id: int, label: string }\n}\n\nlet message: Message = Job { id: 7 }\nprint \"skip\"\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("named payload variant should require all fields");
        assert!(error.message.contains("is missing named payload"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_multi_payload_constructor_arity() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("multi-payload-constructor-arity");
        create_project(&project, Some("multi-payload-constructor-arity-app"))
            .expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nPair(int, string)\nText(string)\n}\n\nlet message: Message = Pair(7)\nprint \"skip\"\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("payload constructor should enforce arity");
        assert!(error.message.contains("expects 2 arguments, got 1"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn check_project_rejects_named_payload_match_with_positional_bindings() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("named-payload-match-positional");
        create_project(&project, Some("named-payload-match-positional-app"))
            .expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "enum Message {\nJob { id: int, label: string }\n}\n\nfn render(message: Message): string {\nmatch message {\nJob(id, label) {\nreturn label\n}\n}\n}\n\nprint \"skip\"\n",
        )
        .expect("write source");
        let error =
            check_project(&project).expect_err("named payload match should require named bindings");
        assert!(error.message.contains("must use named bindings"));
        assert_eq!(error.kind, "type");
    }

    #[test]
    fn build_project_emits_native_binary_from_imported_public_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("enum-modules");
        create_project(&project, Some("enum-modules-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"status.ax\"\n\nfn label(status: Status): string {\nmatch status {\nReady {\nreturn \"ready\"\n}\nFailed {\nreturn \"failed\"\n}\n}\n}\n\nlet status: Status = Ready\nprint label(status)\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/status.ax"),
            "pub enum Status {\nReady\nFailed\n}\n",
        )
        .expect("write status");
        let built = build_project(&project).expect("build imported enums");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "ready\n");
    }

    #[test]
    fn build_project_emits_native_binary_from_imported_payload_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("payload-enum-modules");
        create_project(&project, Some("payload-enum-modules-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"status.ax\"\n\nfn render(status: Status): string {\nmatch status {\nReady(label) {\nreturn label\n}\nFailed(label) {\nreturn label\n}\n}\n}\n\nlet status: Status = Ready(\"from import\")\nprint render(status)\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/status.ax"),
            "pub enum Status {\nReady(string)\nFailed(string)\n}\n",
        )
        .expect("write status");
        let built = build_project(&project).expect("build imported payload enums");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "from import\n");
    }

    #[test]
    fn build_project_emits_native_binary_from_imported_named_payload_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("named-payload-enum-modules");
        create_project(&project, Some("named-payload-enum-modules-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"status.ax\"\n\nfn render(status: Status): string {\nmatch status {\nReady { label } {\nreturn label\n}\nFailed { label } {\nreturn label\n}\n}\n}\n\nlet status: Status = Ready { label: \"from import\" }\nprint render(status)\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/status.ax"),
            "pub enum Status {\nReady { label: string }\nFailed { label: string }\n}\n",
        )
        .expect("write status");
        let built = build_project(&project).expect("build imported named payload enums");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "from import\n");
    }

    #[test]
    fn build_project_emits_native_binary_from_imported_multi_payload_enums() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("multi-payload-enum-modules");
        create_project(&project, Some("multi-payload-enum-modules-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "import \"message.ax\"\n\nfn render(message: Message): string {\nmatch message {\nPair(count, label) {\nprint count\nreturn label\n}\nText(text) {\nreturn text\n}\n}\n}\n\nlet message: Message = Pair(7, \"from import\")\nprint render(message)\n",
        )
        .expect("write main");
        fs::write(
            project.join("src/message.ax"),
            "pub enum Message {\nPair(int, string)\nText(string)\n}\n",
        )
        .expect("write module");
        let built = build_project(&project).expect("build imported multi payload enums");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "7\nfrom import\n");
    }
}
