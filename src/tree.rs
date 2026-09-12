use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Annotations {
    pub changed: HashSet<String>,
    pub matches: HashMap<String, usize>,
}

use crate::classify::{is_deprioritized, is_entrypoint, is_manifest, is_test_path};
use crate::estimate::line_tokens;

pub type NodeId = usize;

pub const ROOT: NodeId = 0;

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub children: Vec<NodeId>,
    pub file_count: usize,
    pub test_count: usize,
    pub has_entrypoint: bool,
    pub has_manifest: bool,
    pub is_test: bool,
    pub is_changed: bool,
    pub changed_count: usize,
    pub match_lines: usize,
    pub match_files: usize,
    pub score: f64,
}

pub struct Tree {
    pub nodes: Vec<Node>,
    pub root_name: String,
}

pub struct Allocation {
    pub expanded: HashSet<NodeId>,
    pub tokens: f64,
    pub over_budget: bool,
}

fn new_node(name: &str, depth: usize, is_dir: bool) -> Node {
    Node {
        name: name.to_string(),
        depth,
        is_dir,
        children: Vec::new(),
        file_count: 0,
        test_count: 0,
        has_entrypoint: false,
        has_manifest: false,
        is_test: false,
        is_changed: false,
        changed_count: 0,
        match_lines: 0,
        match_files: 0,
        score: 0.0,
    }
}

pub fn build_tree(files: &[String], root_name: &str, annotations: &Annotations) -> Tree {
    let mut nodes = vec![new_node(root_name, 0, true)];
    let mut index: HashMap<String, NodeId> = HashMap::new();
    index.insert(String::new(), ROOT);

    for file in files {
        let segments: Vec<&str> = file.split('/').collect();
        let mut parent = ROOT;
        let mut prefix = String::new();
        for (depth, segment) in segments.iter().enumerate() {
            let is_leaf = depth == segments.len() - 1;
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(segment);
            let id = match index.get(&prefix) {
                Some(existing) => *existing,
                None => {
                    let id = nodes.len();
                    nodes.push(new_node(segment, depth + 1, !is_leaf));
                    nodes[parent].children.push(id);
                    index.insert(prefix.clone(), id);
                    id
                }
            };
            parent = id;
        }
        nodes[parent].is_test = is_test_path(file);
        nodes[parent].is_changed = annotations.changed.contains(file);
        nodes[parent].match_lines = annotations.matches.get(file).copied().unwrap_or(0);
    }

    sort_children(&mut nodes);
    roll_up_counts(&mut nodes, ROOT);
    score_all(&mut nodes);
    Tree {
        nodes,
        root_name: root_name.to_string(),
    }
}

fn sort_children(nodes: &mut [Node]) {
    let keys: Vec<(bool, String, String)> = nodes
        .iter()
        .map(|node| (node.is_dir, node.name.to_lowercase(), node.name.clone()))
        .collect();
    for node in nodes.iter_mut() {
        node.children.sort_by(|left, right| {
            let a = &keys[*left];
            let b = &keys[*right];
            b.0.cmp(&a.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
    }
}

fn roll_up_counts(nodes: &mut Vec<Node>, id: NodeId) {
    if !nodes[id].is_dir {
        let is_test = nodes[id].is_test;
        nodes[id].file_count = usize::from(!is_test);
        nodes[id].test_count = usize::from(is_test);
        nodes[id].changed_count = usize::from(nodes[id].is_changed);
        nodes[id].match_files = usize::from(nodes[id].match_lines > 0);
        return;
    }
    let children = nodes[id].children.clone();
    for child_id in children {
        roll_up_counts(nodes, child_id);
        let (file_count, test_count, changed_count, matched, child_is_dir, child_name) = {
            let child = &nodes[child_id];
            (
                child.file_count,
                child.test_count,
                child.changed_count,
                (child.match_lines, child.match_files),
                child.is_dir,
                child.name.clone(),
            )
        };
        nodes[id].file_count += file_count;
        nodes[id].test_count += test_count;
        nodes[id].changed_count += changed_count;
        nodes[id].match_lines += matched.0;
        nodes[id].match_files += matched.1;
        if !child_is_dir {
            if is_entrypoint(&child_name) {
                nodes[id].has_entrypoint = true;
            }
            if is_manifest(&child_name) {
                nodes[id].has_manifest = true;
            }
        }
    }
}

fn score_all(nodes: &mut [Node]) {
    for node in nodes.iter_mut() {
        if !node.is_dir {
            continue;
        }
        let mut score =
            4.0 / (1.0 + node.depth as f64) + 0.8 * ((1 + node.file_count) as f64).log2();
        if node.has_entrypoint {
            score += 2.0;
        }
        if node.has_manifest {
            score += 1.5;
        }
        if is_deprioritized(&node.name) {
            score -= 2.0;
        }
        if node.name.starts_with('.') {
            score -= 2.5;
        }
        if node.file_count == 0 && node.test_count > 0 {
            score -= 3.0;
        }
        if node.changed_count > 0 {
            score += 1.5 * ((1 + node.changed_count) as f64).log2();
        }
        if node.match_files > 0 {
            score += 1.5 * ((1 + node.match_files) as f64).log2();
        }
        node.score = score;
    }
}

pub fn visible_children(tree: &Tree, id: NodeId, include_tests: bool) -> Vec<NodeId> {
    tree.nodes[id]
        .children
        .iter()
        .copied()
        .filter(|child_id| {
            let child = &tree.nodes[*child_id];
            if include_tests {
                true
            } else if child.is_dir {
                child.file_count > 0
            } else {
                !child.is_test
            }
        })
        .collect()
}

pub const CHANGED_MARK: &str = "  *";

pub fn dir_summary(node: &Node, include_tests: bool) -> String {
    if !node.is_dir {
        let mut marks = String::new();
        if node.is_changed {
            marks.push_str(CHANGED_MARK);
        }
        if node.match_lines > 0 {
            marks.push_str(&format!("  ({})", node.match_lines));
        }
        return marks;
    }
    let mut parts: Vec<String> = Vec::new();
    if node.file_count > 0 {
        parts.push(plural(node.file_count, "file"));
    }
    if !include_tests && node.test_count > 0 {
        parts.push(format!("{} hidden", plural(node.test_count, "test")));
    }
    if include_tests && node.test_count > 0 && node.file_count == 0 {
        parts.push(plural(node.test_count, "test"));
    }
    if node.changed_count > 0 {
        parts.push(format!("{} changed", node.changed_count));
    }
    if node.match_files > 0 {
        parts.push(format!("{} matched", node.match_files));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("  ({})", parts.join(", "))
    }
}

pub fn plural(count: usize, noun: &str) -> String {
    let suffix = if count == 1 { "" } else { "s" };
    format!("{count} {noun}{suffix}")
}

pub fn visible_char_count(tree: &Tree, id: NodeId, include_tests: bool) -> usize {
    let node = &tree.nodes[id];
    let label_len = node.name.chars().count() + usize::from(node.is_dir);
    let summary_len = dir_summary(node, include_tests).chars().count();
    label_len + summary_len
}

pub fn allocate(tree: &Tree, budget: f64, include_tests: bool) -> Allocation {
    let mut expanded = HashSet::from([ROOT]);
    let mut tokens = line_tokens(tree.root_name.chars().count() + 40);

    for child_id in visible_children(tree, ROOT, include_tests) {
        tokens += line_tokens(visible_char_count(tree, child_id, include_tests));
    }
    let over_budget = tokens > budget;

    while let Some((id, delta)) = best_candidate(tree, &expanded, budget - tokens, include_tests) {
        expanded.insert(id);
        tokens += delta;
    }

    Allocation {
        expanded,
        tokens,
        over_budget,
    }
}

fn best_candidate(
    tree: &Tree,
    expanded: &HashSet<NodeId>,
    remaining: f64,
    include_tests: bool,
) -> Option<(NodeId, f64)> {
    let mut best: Option<(NodeId, f64, f64)> = None;

    for parent_id in expanded {
        for id in visible_children(tree, *parent_id, include_tests) {
            let node = &tree.nodes[id];
            if !node.is_dir || expanded.contains(&id) {
                continue;
            }
            let children = visible_children(tree, id, include_tests);
            if children.is_empty() {
                continue;
            }
            let delta = expansion_delta(tree, &children, include_tests);
            if delta > remaining {
                continue;
            }
            if best.is_none_or(|(_, _, score)| node.score > score) {
                best = Some((id, delta, node.score));
            }
        }
    }
    best.map(|(id, delta, _)| (id, delta))
}

fn expansion_delta(tree: &Tree, children: &[NodeId], include_tests: bool) -> f64 {
    children
        .iter()
        .map(|child_id| line_tokens(visible_char_count(tree, *child_id, include_tests)))
        .sum()
}
