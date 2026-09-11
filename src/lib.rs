pub mod classify;
pub mod collect;
pub mod estimate;
pub mod render;
pub mod tree;

use std::collections::HashSet;
use std::path::Path;

use classify::classify_kind;
use collect::{Source, changed_since, collect};
use render::{RenderOptions, render};
use tree::{allocate, build_tree};

pub enum Since {
    Off,
    Changes(String, HashSet<String>),
    Unavailable,
}

fn resolve_since(root: &Path, source: Source, reference: Option<&str>) -> Result<Since, String> {
    let Some(reference) = reference else {
        return Ok(Since::Off);
    };
    if source != Source::Git {
        return Ok(Since::Unavailable);
    }
    let changed = changed_since(root, reference)?;
    Ok(Since::Changes(
        reference.to_string(),
        changed.into_iter().collect(),
    ))
}

pub fn run(
    root: &Path,
    budget: f64,
    include_tests: bool,
    excludes: &[String],
    since: Option<&str>,
) -> Result<String, String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());

    let (files, source) = collect(&root, excludes);
    let since = resolve_since(&root, source, since)?;
    if files.is_empty() {
        return Ok(format!("{root_name}/  [empty]"));
    }

    let changed = match &since {
        Since::Changes(_, paths) => paths.clone(),
        _ => HashSet::new(),
    };
    let kind = classify_kind(&files);
    let tree = build_tree(&files, &root_name, &changed);
    let allocation = allocate(&tree, budget, include_tests);
    let mut lines = render(
        &tree,
        &allocation,
        &RenderOptions {
            kind,
            source,
            include_tests,
            since,
        },
    );

    let overage = if allocation.over_budget {
        " (over budget: top level alone exceeds it)"
    } else {
        ""
    };
    lines.push(String::new());
    lines.push(format!("~{} tokens{overage}", allocation.tokens.round()));
    Ok(lines.join("\n"))
}
