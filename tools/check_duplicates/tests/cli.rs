use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);

impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "rustshot-duplicates-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        for (name, content) in files {
            let file = path.join(name);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, content).unwrap();
        }
        Self(path)
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_check_duplicates"))
            .arg(&self.0)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const BODY: &str = "step_one();\nstep_two();\nstep_three();\nstep_four();\nstep_five();";

#[test]
fn detects_five_line_clone_across_files() {
    let first = format!("fn first() {{\n{BODY}\n}}");
    let second = format!("fn second() {{\n{BODY}\n}}");
    let fixture = Fixture::new(&[("a.rs", &first), ("nested/b.rs", &second)]);
    let result = fixture.run();
    assert_eq!(result.status.code(), Some(1));
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("a.rs:2"));
    assert!(report.contains("b.rs:2"));
}

#[test]
fn detects_clones_in_tests_and_other_platforms() {
    let code = format!(
        "#[cfg(test)] mod tests {{\n#[test] fn first() {{\n{BODY}\n}}\n}}\n\
         #[cfg(windows)] fn second() {{\n{BODY}\n}}"
    );
    assert_eq!(
        Fixture::new(&[("test.rs", &code)]).run().status.code(),
        Some(1)
    );
}

#[test]
fn ignores_comments_and_structural_boilerplate() {
    let fixture = Fixture::new(&[
        (
            "a.rs",
            "// shared\n// documentation\n// is\n// not\n// logic\n\
             #[cfg(test)]\n#[allow(dead_code)]\nmod tests {\nuse super::*;\n}\n",
        ),
        (
            "b.rs",
            "// shared\n// documentation\n// is\n// not\n// logic\n\
             #[cfg(test)]\n#[allow(dead_code)]\nmod tests {\nuse super::*;\n}\n",
        ),
    ]);
    assert_eq!(fixture.run().status.code(), Some(0));
}

#[test]
fn does_not_join_different_functions_into_a_clone() {
    let source = "fn one() {\na();\nb();\n}\nfn two() {\nc();\nd();\ne();\n}";
    assert_eq!(
        Fixture::new(&[("a.rs", source), ("b.rs", source)])
            .run()
            .status
            .code(),
        Some(0)
    );
}

#[test]
fn partial_argument_lists_are_not_duplicated_statements() {
    let first = "fn first() {\nalpha(\n1,\n2,\n3,\n4,\n5,\n);\n}";
    let second = "fn second() {\nbeta(\n1,\n2,\n3,\n4,\n5,\n);\n}";
    assert_eq!(
        Fixture::new(&[("a.rs", first), ("b.rs", second)])
            .run()
            .status
            .code(),
        Some(0)
    );
}

#[test]
fn detects_complete_multiline_statement() {
    let body = "let data = Record {\na: 1,\nb: 2,\nc: 3,\nd: 4,\n};";
    let code = format!("fn first() {{\n{body}\n}}\nfn second() {{\n{body}\n}}");
    assert_eq!(
        Fixture::new(&[("a.rs", &code)]).run().status.code(),
        Some(1)
    );
}

#[test]
fn reports_a_long_clone_once() {
    let body = format!("{BODY}\nstep_six();\nstep_seven();");
    let code = format!("fn first() {{\n{body}\n}}\nfn second() {{\n{body}\n}}");
    let result = Fixture::new(&[("a.rs", &code)]).run();
    assert_eq!(result.status.code(), Some(1));
    let report = String::from_utf8(result.stdout).unwrap();
    assert_eq!(report.matches("Duplicated block").count(), 1);
}

#[test]
fn invalid_rust_fails_instead_of_passing_silently() {
    assert_eq!(
        Fixture::new(&[("broken.rs", "fn broken( {")])
            .run()
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn missing_directory_is_an_error() {
    let fixture = Fixture::new(&[]);
    let result = Command::new(env!("CARGO_BIN_EXE_check_duplicates"))
        .arg(fixture.0.join("missing"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
}

#[test]
fn skipped_build_directories_do_not_affect_scan() {
    assert_eq!(
        Fixture::new(&[
            ("src.rs", "fn valid() {}"),
            ("target/generated.rs", "invalid"),
            (".git/generated.rs", "invalid"),
        ])
        .run()
        .status
        .code(),
        Some(0)
    );
}

#[test]
fn repeated_helper_calls_are_one_code_line_each() {
    let body = "shared(\nfirst,\nsecond,\nthird,\nfourth,\nfifth,\n);";
    let code = format!("fn first() {{\n{body}\n}}\nfn second() {{\n{body}\n}}");
    assert_eq!(
        Fixture::new(&[("a.rs", &code)]).run().status.code(),
        Some(0)
    );
}

#[test]
fn five_calls_to_a_shared_helper_are_still_a_clone() {
    let body = "shared(a);\nshared(b);\nshared(c);\nshared(d);\nshared(e);";
    let code = format!("fn first() {{\n{body}\n}}\nfn second() {{\n{body}\n}}");
    assert_eq!(
        Fixture::new(&[("a.rs", &code)]).run().status.code(),
        Some(1)
    );
}

#[test]
fn detects_rust_bodies_inside_macro_dsl() {
    let code = format!(
        "cases! {{ fn first(value in arbitrary()) {{\n{BODY}\n}} }}\n\
         cases! {{ fn second(value in arbitrary()) {{\n{BODY}\n}} }}"
    );
    assert_eq!(
        Fixture::new(&[("a.rs", &code)]).run().status.code(),
        Some(1)
    );
}

#[test]
fn comments_between_statements_cannot_hide_a_clone() {
    let first = format!("fn first() {{\n{BODY}\n}}");
    let body = BODY.replace(';', "; /* a nested /* block */ comment */");
    let second = format!("fn second() {{\n{body}\n}}");
    assert_eq!(
        Fixture::new(&[("a.rs", &first), ("b.rs", &second)])
            .run()
            .status
            .code(),
        Some(1)
    );
}

#[test]
fn nested_occurrence_is_not_a_clone_of_itself() {
    let code = format!("fn only() {{\nif ready {{\n{BODY}\n}}\n}}");
    assert_eq!(
        Fixture::new(&[("a.rs", &code)]).run().status.code(),
        Some(0)
    );
}

#[test]
fn empty_scan_is_an_error() {
    assert_eq!(Fixture::new(&[]).run().status.code(), Some(2));
}
