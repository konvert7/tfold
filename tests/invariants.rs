use std::fs;
use std::path::PathBuf;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("treefold-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture root");
        Fixture { root }
    }

    fn file(&self, relative: &str, contents: &str) -> &Self {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, contents).expect("write file");
        self
    }

    fn map(&self, budget: f64, include_tests: bool) -> String {
        treefold::run(&self.root, budget, include_tests)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn gitignore_applies_without_a_git_directory() {
    let fixture = Fixture::new("ignore-nogit");
    fixture
        .file(".gitignore", "*.log\nsecret.md\n")
        .file("README.md", "")
        .file("app.log", "")
        .file("secret.md", "");

    let map = fixture.map(500.0, false);
    assert!(map.contains("README.md"), "kept file missing:\n{map}");
    assert!(!map.contains("app.log"), "*.log not ignored:\n{map}");
    assert!(!map.contains("secret.md"), "secret.md not ignored:\n{map}");
    assert!(
        !fixture.root.join(".git").exists(),
        "fixture must have no .git for this test to mean anything"
    );
}

#[test]
fn nested_gitignore_scopes_to_its_own_subtree() {
    let fixture = Fixture::new("ignore-nested");
    fixture
        .file("src/.gitignore", "generated/\n")
        .file("src/main.rs", "")
        .file("src/generated/schema.rs", "")
        .file("other/generated/keep.rs", "");

    let map = fixture.map(500.0, false);
    assert!(map.contains("main.rs"), "sibling dropped:\n{map}");
    assert!(!map.contains("schema.rs"), "nested ignore not applied:\n{map}");
    assert!(
        map.contains("keep.rs"),
        "nested ignore leaked outside its subtree:\n{map}"
    );
}

#[test]
fn negation_restores_a_previously_ignored_file() {
    let fixture = Fixture::new("ignore-negate");
    fixture
        .file(".gitignore", "*.log\n!keep.log\n")
        .file("app.log", "")
        .file("keep.log", "");

    let map = fixture.map(500.0, false);
    assert!(map.contains("keep.log"), "negation ignored:\n{map}");
    assert!(!map.contains("app.log"), "*.log not ignored:\n{map}");
}

#[test]
fn a_subdirectory_still_honours_its_parents_gitignore() {
    let fixture = Fixture::new("subdir-parent-ignore");
    fixture
        .file(".gitignore", "generated/\n")
        .file("src/main.rs", "")
        .file("src/generated/schema.rs", "");

    let map = treefold::run(&fixture.root.join("src"), 500.0, false);
    assert!(map.contains("main.rs"), "sibling dropped:\n{map}");
    assert!(
        !map.contains("schema.rs"),
        "parent .gitignore not applied when pointed at a subdirectory:\n{map}"
    );
}

#[test]
fn tests_are_hidden_by_default_but_still_counted() {
    let fixture = Fixture::new("tests-hidden");
    fixture
        .file("src/main.rs", "")
        .file("tests/alpha_test.rs", "")
        .file("tests/beta_test.rs", "");

    let hidden = fixture.map(500.0, false);
    assert!(
        hidden.contains("2 tests hidden"),
        "hidden tests not counted in header:\n{hidden}"
    );
    assert!(
        !hidden.contains("alpha_test.rs"),
        "test file leaked into default output:\n{hidden}"
    );

    let shown = fixture.map(500.0, true);
    assert!(
        shown.contains("alpha_test.rs"),
        "--include-tests did not restore tests:\n{shown}"
    );
}

#[test]
fn a_collapsed_directory_reports_a_count_and_hides_every_child() {
    let fixture = Fixture::new("atomic");
    for index in 0..40 {
        fixture.file(&format!("bulk/file{index:02}.rs"), "");
    }
    fixture.file("src/main.rs", "");

    let map = fixture.map(120.0, false);
    let collapsed = map
        .lines()
        .find(|line| line.contains("bulk/"))
        .expect("bulk directory missing");
    assert!(
        collapsed.contains("(40 files)"),
        "collapsed directory did not report its count: {collapsed}"
    );
    assert!(
        !map.contains("file00.rs") && !map.contains("file39.rs"),
        "collapsed directory leaked children — breadth must never be partial:\n{map}"
    );
}

#[test]
fn output_stays_within_the_requested_budget() {
    let fixture = Fixture::new("budget");
    for index in 0..60 {
        fixture.file(&format!("area{}/file{index:02}.rs", index % 6), "");
    }

    for budget in [150.0, 400.0, 900.0] {
        let map = fixture.map(budget, false);
        let claimed: f64 = map
            .lines()
            .last()
            .and_then(|line| line.trim_start_matches('~').split(' ').next())
            .and_then(|number| number.parse().ok())
            .expect("token footer missing");
        assert!(
            claimed <= budget,
            "budget {budget} exceeded: claimed {claimed}\n{map}"
        );
    }
}
