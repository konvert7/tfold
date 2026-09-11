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
        self.map_with(tfold::Options {
            budget,
            include_tests,
            excludes: excludes.iter().map(|glob| glob.to_string()).collect(),
            ..Default::default()
        })
        .expect("map")
    }

    fn map_with(&self, options: tfold::Options) -> Result<String, String> {
        tfold::run(&self.root, &options)
    }

    fn map_since(&self, reference: &str) -> Result<String, String> {
        self.map_with(tfold::Options {
            budget: 500.0,
            since: Some(reference.to_string()),
            ..Default::default()
        })
    }

    fn map_grep(&self, pattern: &str, ignore_case: bool) -> String {
        self.map_with(tfold::Options {
            budget: 500.0,
            grep: Some(pattern.to_string()),
            ignore_case,
            ..Default::default()
        })
        .expect("grep map")
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

    let map = tfold::run(
        &fixture.root.join("src"),
        &tfold::Options {
            budget: 500.0,
            ..Default::default()
        },
    )
    .expect("map");
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
    let plain = fixture
        .map_with(tfold::Options {
            budget,
            ..Default::default()
        })
        .expect("plain map");
    assert!(
        plain.contains("file00.rs") && !plain.contains("unit00.rs"),
        "the budget must fit exactly one subtree, and bulk/ must win it without --since:\n{plain}"
    );

    let map = fixture
        .map_with(tfold::Options {
            budget,
            since: Some("main".to_string()),
            ..Default::default()
        })
        .expect("since map");
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

#[test]
fn grep_counts_matching_lines_and_reports_them_at_every_level() {
    let fixture = Fixture::new("grep-counts");
    fixture
        .file("src/hit.rs", "needle\nplain\nneedle needle\n")
        .file("src/miss.rs", "nothing here\n");

    let map = fixture.map_grep("needle", false);
    assert!(
        map.contains("2 matching lines in 1 file"),
        "header did not report the scan:\n{map}"
    );
    let directory = map
        .lines()
        .find(|line| line.contains("src/"))
        .expect("src missing");
    assert!(
        directory.contains("1 matched"),
        "directory did not report its matched files: {directory}"
    );
    let hit = map
        .lines()
        .find(|line| line.contains("hit.rs"))
        .expect("hit.rs missing");
    assert!(
        hit.contains("(2)"),
        "the third line holds two occurrences and must count once, so hit.rs is 2: {hit}"
    );
    let miss = map
        .lines()
        .find(|line| line.contains("miss.rs"))
        .expect("miss.rs missing");
    assert!(!miss.contains('('), "unmatched file was annotated: {miss}");
}

#[test]
fn grep_is_case_sensitive_until_ignore_case_is_asked_for() {
    let fixture = Fixture::new("grep-case");
    fixture.file("src/main.rs", "Needle\n");

    assert!(
        fixture
            .map_grep("needle", false)
            .contains("0 matching lines"),
        "case-sensitive search matched a different casing"
    );
    assert!(
        fixture.map_grep("needle", true).contains("1 matching line"),
        "-i did not match a different casing"
    );
}

#[test]
fn grep_moves_the_budget_to_the_matching_subtree() {
    let fixture = Fixture::new("grep-ranks");
    for index in 0..30 {
        fixture.file(&format!("bulk/file{index:02}.rs",), "nothing\n");
    }
    for index in 0..25 {
        fixture.file(&format!("quiet/unit{index:02}.rs"), "nothing\n");
    }
    fixture.file("quiet/unit00.rs", "needle\n");

    let budget = 300.0;
    let plain = fixture
        .map_with(tfold::Options {
            budget,
            ..Default::default()
        })
        .expect("plain map");
    assert!(
        plain.contains("file00.rs") && !plain.contains("unit00.rs"),
        "bulk/ must win the budget without --grep, or this proves nothing:\n{plain}"
    );

    let map = fixture
        .map_with(tfold::Options {
            budget,
            grep: Some("needle".to_string()),
            ..Default::default()
        })
        .expect("grep map");
    assert!(
        map.contains("unit00.rs") && !map.contains("file00.rs"),
        "--grep did not move the budget to the matching subtree:\n{map}"
    );
}

#[test]
fn grep_needs_no_git_repository() {
    let fixture = Fixture::new("grep-nogit");
    fixture.file("notes/a.md", "needle\n");

    let map = fixture.map_grep("needle", false);
    assert!(
        !fixture.root.join(".git").exists(),
        "fixture must have no .git for this test to mean anything"
    );
    assert!(
        map.contains("1 matching line in 1 file") && map.contains("walk"),
        "--grep did not work outside a repository:\n{map}"
    );
}

#[test]
fn grep_with_no_matches_still_produces_the_map() {
    let fixture = Fixture::new("grep-empty");
    fixture.file("src/main.rs", "nothing\n");

    let map = fixture.map_grep("absent", false);
    assert!(
        map.contains("0 matching lines in 0 files") && map.contains("main.rs"),
        "a fruitless search should still map the tree:\n{map}"
    );
}

#[test]
fn a_binary_file_is_never_scanned() {
    let fixture = Fixture::new("grep-binary");
    fixture.file("data/blob.bin", "needle\u{0}needle\n");

    let map = fixture.map_grep("needle", false);
    assert!(
        map.contains("0 matching lines"),
        "a NUL-bearing file was scanned:\n{map}"
    );
}

#[test]
fn the_default_map_never_looks_inside_a_file() {
    let fixture = Fixture::new("grep-off");
    fixture.file("src/main.rs", "needle\n");

    let map = fixture.map(500.0, false);
    assert!(
        !map.contains("matching") && !map.contains("(1)"),
        "the default map reported something only a scan could know:\n{map}"
    );
}

#[test]
fn grep_and_since_compose_on_the_same_file() {
    let fixture = Fixture::new("grep-since");
    fixture
        .file("src/both.rs", "nothing\n")
        .git(&["init", "-q", "-b", "main"])
        .git(&["add", "-A"])
        .git(&["commit", "-qm", "init"]);
    fixture.file("src/both.rs", "needle\n");

    let map = fixture
        .map_with(tfold::Options {
            budget: 500.0,
            since: Some("main".to_string()),
            grep: Some("needle".to_string()),
            ..Default::default()
        })
        .expect("combined map");
    let line = map
        .lines()
        .find(|line| line.contains("both.rs"))
        .expect("both.rs missing");
    assert!(
        line.contains('*') && line.contains("(1)"),
        "the two marks did not compose: {line}"
    );
}
