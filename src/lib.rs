pub mod classify;
pub mod collect;
pub mod estimate;
pub mod render;
pub mod scan;
pub mod tree;

use std::collections::HashSet;
use std::path::Path;
use std::time::{Duration, Instant};

use classify::classify_kind;
use collect::{Source, changed_since, collect};
use render::{RenderOptions, render};
use scan::{Pattern, scan};
use tree::{Annotations, allocate, build_tree};

pub struct Options {
    pub budget: f64,
    pub include_tests: bool,
    pub excludes: Vec<String>,
    pub since: Option<String>,
    pub grep: Option<String>,
    pub ignore_case: bool,
    pub show_cost: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            budget: 800.0,
            include_tests: false,
            excludes: Vec::new(),
            since: None,
            grep: None,
            ignore_case: false,
            show_cost: false,
        }
    }
}

pub enum Since {
    Off,
    Changes(String, HashSet<String>),
    Unavailable,
}

pub enum Grep {
    Off,
    Found { lines: usize, files: usize },
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

fn annotations(root: &Path, files: &[String], since: &Since, options: &Options) -> Annotations {
    Annotations {
        changed: match since {
            Since::Changes(_, paths) => paths.clone(),
            _ => HashSet::new(),
        },
        matches: match &options.grep {
            Some(needle) => scan(root, files, &Pattern::new(needle, options.ignore_case)),
            None => Default::default(),
        },
    }
}

fn grep_summary(options: &Options, matches: &Annotations) -> Grep {
    if options.grep.is_none() {
        return Grep::Off;
    }
    Grep::Found {
        lines: matches.matches.values().sum(),
        files: matches.matches.len(),
    }
}

const OVER_BUDGET: &str = "over budget: top level alone exceeds it";

fn milliseconds(elapsed: Duration) -> String {
    let millis = elapsed.as_secs_f64() * 1000.0;
    if millis < 10.0 {
        format!("{millis:.1} ms")
    } else {
        format!("{} ms", millis.round())
    }
}

fn footer(allocation: &tree::Allocation, show_cost: bool, elapsed: Duration) -> Option<String> {
    if show_cost {
        let overage = if allocation.over_budget {
            format!(" ({OVER_BUDGET})")
        } else {
            String::new()
        };
        return Some(format!(
            "~{} tokens · {}{overage}",
            allocation.tokens.round(),
            milliseconds(elapsed)
        ));
    }
    allocation.over_budget.then(|| OVER_BUDGET.to_string())
}

pub fn run(root: &Path, options: &Options) -> Result<String, String> {
    let started = Instant::now();
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let root_name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());

    let (files, source) = collect(&root, &options.excludes);
    let since = resolve_since(&root, source, options.since.as_deref())?;
    if files.is_empty() {
        return Ok(format!("{root_name}/  [empty]"));
    }

    let marks = annotations(&root, &files, &since, options);
    let grep = grep_summary(options, &marks);
    let kind = classify_kind(&files);
    let tree = build_tree(&files, &root_name, &marks);
    let allocation = allocate(&tree, options.budget, options.include_tests);
    let mut lines = render(
        &tree,
        &allocation,
        &RenderOptions {
            kind,
            source,
            include_tests: options.include_tests,
            since,
            grep,
        },
    );

    if let Some(footer) = footer(&allocation, options.show_cost, started.elapsed()) {
        lines.push(String::new());
        lines.push(footer);
    }
    Ok(lines.join("\n"))
}
