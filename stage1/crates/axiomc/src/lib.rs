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
        let source = "fn tail_len(values: &[string]): int {\nlet tail: &[string] = values[1:]\nreturn len(tail)\n}\n\nlet values: [int] = [3, 7, 9, 11]\nlet middle: &[int] = values[1:3]\nlet prefix: &[int] = values[:2]\nlet tail: &[int] = values[2:]\nprint middle[0]\nprint prefix[1]\nprint tail[0]\nprint len(values[:])\nlet labels: [string] = [\"build\", \"test\", \"ship\"]\nprint tail_len(labels[:])\n";
        let parsed = parse_program(source, Path::new("main.ax")).expect("parse");
        let hir = hir::lower(&parsed).expect("lower");
        let mir = mir::lower(&hir);
        let rendered = render_rust(&mir);
        assert!(rendered.contains("fn tail_len(values: &[String]) -> i64 {"));
        assert!(
            rendered.contains("let middle: &[i64] = axiom_slice_view(&values, Some(1), Some(3));")
        );
        assert!(
            rendered.contains("let prefix: &[i64] = axiom_slice_view(&values, None, Some(2));")
        );
        assert!(rendered.contains("let tail: &[i64] = axiom_slice_view(&values, Some(2), None);"));
        assert!(rendered.contains("return (tail).len() as i64;"));
        assert!(
            rendered.contains(
                "println!(\"{}\", (axiom_slice_view(&values, None, None)).len() as i64);"
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
            "fn tail_len(values: &[string]): int {\nlet tail: &[string] = values[1:]\nreturn len(tail)\n}\n\nlet values: [int] = [3, 7, 9, 11]\nlet middle: &[int] = values[1:3]\nlet prefix: &[int] = values[:2]\nlet tail: &[int] = values[2:]\nprint middle[0]\nprint prefix[1]\nprint tail[0]\nprint len(values[:])\nlet labels: [string] = [\"build\", \"test\", \"ship\"]\nprint tail_len(labels[:])\n",
        )
        .expect("write source");
        let built = build_project(&project).expect("build project");
        let output = Command::new(&built.binary)
            .output()
            .expect("run compiled binary");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n7\n9\n4\n2\n");
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
    fn check_project_rejects_slice_return_type() {
        let dir = tempdir().expect("tempdir");
        let project = dir.path().join("slice-return");
        create_project(&project, Some("slice-return-app")).expect("create project");
        fs::write(
            project.join("src/main.ax"),
            "fn tail(values: &[int]): &[int] {\nreturn values[1:]\n}\n\nprint 0\n",
        )
        .expect("write source");
        let error = check_project(&project).expect_err("slice returns should fail in stage1");
        assert!(
            error
                .message
                .contains("function return types cannot contain borrowed slices")
        );
        assert_eq!(error.kind, "type");
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
