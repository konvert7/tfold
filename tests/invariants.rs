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
        tfold::run(&self.root, budget, include_tests, &excludes, None).expect("map")
    }

    fn map_since(&self, reference: &str) -> Result<String, String> {
        tfold::run(&self.root, 500.0, false, &[], Some(reference))
    }

    fn git(&self, args: &[&str]) -> &Self {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .env("GIT_AUTHOR_NAME", "tfold")
            .env("GIT_AUTHOR_EMAIL", "tfold@example.com")
            .env("GIT_COMMITTER_NAME", "tfold")
            .env("GIT_COMMITTER_EMAIL", "tfold@example.com")
            .output()
            .expect("run git");
        assert!(
            status.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
        self
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

    let map = tfold::run(&fixture.root.join("src"), 500.0, false, &[], None).expect("map");
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

#[test]
fn since_marks_changed_files_and_counts_them_in_the_header() {
    let fixture = Fixture::new("since-marks");
    fixture
        .file("src/stable.rs", "")
        .file("src/touched.rs", "")
        .git(&["init", "-q", "-b", "main"])
        .git(&["add", "-A"])
        .git(&["commit", "-qm", "init"]);
    fixture.file("src/touched.rs", "changed\n");

    let map = fixture.map_since("main").expect("since map");
    assert!(
        map.contains("1 changed since main"),
        "header did not report the change set:\n{map}"
    );
    let line = map
        .lines()
        .find(|line| line.contains("src/"))
        .expect("src missing");
    assert!(
        line.contains("1 changed"),
        "directory did not report its changed count: {line}"
    );
}

#[test]
fn since_is_ignored_rather_than_fatal_outside_a_git_repository() {
    let fixture = Fixture::new("since-nogit");
    fixture.file("src/main.rs", "").file("README.md", "");

    let map = fixture
        .map_since("main")
        .expect("--since must not fail without git");
    assert!(
        map.contains("--since needs git, ignored"),
        "the ignored flag was not reported:\n{map}"
    );
    assert!(
        map.contains("main.rs") && map.contains("README.md"),
        "the map was not produced in full:\n{map}"
    );
    assert!(
        !map.contains("changed"),
        "a change count appeared without git:\n{map}"
    );
}

#[test]
fn an_unknown_revision_is_rejected_before_a_map_is_produced() {
    let fixture = Fixture::new("since-badref");
    fixture
        .file("src/main.rs", "")
        .git(&["init", "-q", "-b", "main"])
        .git(&["add", "-A"])
        .git(&["commit", "-qm", "init"]);

    let error = fixture
        .map_since("no-such-ref")
        .expect_err("an unknown revision must be rejected");
    assert!(
        error.contains("no-such-ref"),
        "the error did not name the revision: {error}"
    );
}

#[test]
fn since_ranks_a_changed_subtree_above_a_larger_untouched_one() {
    let fixture = Fixture::new("since-ranks");
    for index in 0..30 {
        fixture.file(&format!("bulk/file{index:02}.rs"), "");
    }
    for index in 0..25 {
        fixture.file(&format!("quiet/unit{index:02}.rs"), "");
    }
    fixture
        .git(&["init", "-q", "-b", "main"])
        .git(&["add", "-A"])
        .git(&["commit", "-qm", "init"]);
    fixture.file("quiet/unit00.rs", "changed\n");

    let budget = 300.0;
    let plain = tfold::run(&fixture.root, budget, false, &[], None).expect("plain map");
    assert!(
        plain.contains("file00.rs") && !plain.contains("unit00.rs"),
        "the budget must fit exactly one subtree, and bulk/ must win it without --since:\n{plain}"
    );

    let map = tfold::run(&fixture.root, budget, false, &[], Some("main")).expect("since map");
    assert!(
        map.contains("unit00.rs") && !map.contains("file00.rs"),
        "--since did not move the budget to the changed subtree:\n{map}"
    );
}

#[test]
fn a_changed_file_is_marked_where_it_sits() {
    let fixture = Fixture::new("since-file-mark");
    fixture
        .file("src/stable.rs", "")
        .file("src/touched.rs", "")
        .git(&["init", "-q", "-b", "main"])
        .git(&["add", "-A"])
        .git(&["commit", "-qm", "init"]);
    fixture.file("src/touched.rs", "changed\n");

    let map = fixture.map_since("main").expect("since map");
    let touched = map
        .lines()
        .find(|line| line.contains("touched.rs"))
        .expect("touched.rs missing");
    let stable = map
        .lines()
        .find(|line| line.contains("stable.rs"))
        .expect("stable.rs missing");
    assert!(
        touched.trim_end().ends_with('*'),
        "changed file was not marked: {touched}"
    );
    assert!(
        !stable.trim_end().ends_with('*'),
        "unchanged file was marked: {stable}"
    );
}
