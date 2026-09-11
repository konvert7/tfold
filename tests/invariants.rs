use std::fs;
use std::path::PathBuf;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("tfold-{name}-{}", std::process::id()));
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
        self.map_excluding(budget, include_tests, &[])
    }

    fn map_excluding(&self, budget: f64, include_tests: bool, excludes: &[&str]) -> String {
        let excludes: Vec<String> = excludes.iter().map(|glob| glob.to_string()).collect();
        tfold::run(&self.root, budget, include_tests, &excludes)
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
    assert!(
        !map.contains("schema.rs"),
        "nested ignore not applied:\n{map}"
    );
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

    let map = tfold::run(&fixture.root.join("src"), 500.0, false, &[]);
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

#[test]
fn an_exclude_glob_removes_matching_paths_and_keeps_the_rest() {
    let fixture = Fixture::new("exclude-glob");
    fixture
        .file("src/main.rs", "")
        .file("docs/guide.md", "")
        .file("docs/api.md", "")
        .file("vendor/lib.rs", "");

    let map = fixture.map_excluding(500.0, false, &["docs/**", "vendor"]);
    assert!(map.contains("src/"), "kept directory missing:\n{map}");
    assert!(!map.contains("guide.md"), "docs/** not excluded:\n{map}");
    assert!(!map.contains("api.md"), "docs/** not excluded:\n{map}");
    assert!(!map.contains("vendor"), "vendor not excluded:\n{map}");
}

#[test]
fn excludes_layer_on_top_of_gitignore_rather_than_replacing_it() {
    let fixture = Fixture::new("exclude-with-gitignore");
    fixture
        .file(".gitignore", "*.log\n")
        .file("README.md", "")
        .file("app.log", "")
        .file("notes/draft.md", "");

    let map = fixture.map_excluding(500.0, false, &["notes"]);
    assert!(map.contains("README.md"), "kept file missing:\n{map}");
    assert!(
        !map.contains("app.log"),
        "gitignore stopped applying:\n{map}"
    );
    assert!(!map.contains("draft.md"), "exclude not applied:\n{map}");
}

#[test]
fn no_exclude_leaves_the_map_untouched() {
    let fixture = Fixture::new("exclude-none");
    fixture.file("src/main.rs", "").file("docs/guide.md", "");

    assert_eq!(
        fixture.map(500.0, false),
        fixture.map_excluding(500.0, false, &[]),
        "passing an empty exclude list changed the output"
    );
}

#[test]
fn an_expanded_directory_still_reports_its_recursive_total() {
    let fixture = Fixture::new("expanded-total");
    fixture
        .file("src/main.rs", "")
        .file("src/deep/alpha.rs", "")
        .file("src/deep/beta.rs", "");

    let map = fixture.map(500.0, false);
    assert!(
        map.contains("main.rs"),
        "src/ was not expanded, so this test proves nothing:\n{map}"
    );
    let line = map
        .lines()
        .find(|line| line.contains("src/"))
        .expect("src directory missing");
    assert!(
        line.contains("(3 files)"),
        "expanded directory dropped its recursive total: {line}"
    );
}

#[test]
fn the_token_footer_accounts_for_every_printed_line() {
    let fixture = Fixture::new("footer-honest");
    for index in 0..40 {
        fixture.file(&format!("area{}/nested/file{index:02}.rs", index % 5), "");
    }

    let map = fixture.map(900.0, false);
    let lines: Vec<&str> = map.lines().collect();
    let claimed: f64 = lines
        .last()
        .and_then(|line| line.trim_start_matches('~').split(' ').next())
        .and_then(|number| number.parse().ok())
        .expect("token footer missing");
    let printed: f64 = lines[1..lines.len() - 2]
        .iter()
        .map(|line| tfold::estimate::line_tokens(line.chars().count()))
        .sum();

    assert!(
        claimed >= printed,
        "footer under-reports: claimed {claimed}, body alone is {printed}\n{map}"
    );
}
