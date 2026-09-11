pub mod classify;
pub mod collect;
pub mod estimate;
pub mod render;
pub mod tree;

use std::path::Path;

use classify::classify_kind;
use collect::collect;
use render::{render, RenderOptions};
use tree::{allocate, build_tree};

pub fn run(root: &Path, budget: f64, include_tests: bool, excludes: &[String]) -> String {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());

    let (files, source) = collect(&root, excludes);
    if files.is_empty() {
        return format!("{root_name}/  [empty]");
    }

    let kind = classify_kind(&files);
    let tree = build_tree(&files, &root_name);
    let allocation = allocate(&tree, budget, include_tests);
    let mut lines = render(
        &tree,
        &allocation,
        &RenderOptions {
            kind,
            source,
            include_tests,
        },
    );

    let overage = if allocation.over_budget {
        " (over budget: top level alone exceeds it)"
    } else {
        ""
    };
    lines.push(String::new());
    lines.push(format!("~{} tokens{overage}", allocation.tokens.round()));
    lines.join("\n")
}
