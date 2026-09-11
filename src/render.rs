use crate::classify::Kind;
use crate::collect::Source;
use crate::tree::{Allocation, NodeId, ROOT, Tree, collapsed_summary, visible_children};

pub struct RenderOptions {
    pub kind: Kind,
    pub source: Source,
    pub include_tests: bool,
}

pub fn render(tree: &Tree, allocation: &Allocation, options: &RenderOptions) -> Vec<String> {
    let mut lines = vec![header(tree, options)];
    append_children(
        tree,
        ROOT,
        "",
        &mut lines,
        allocation,
        options.include_tests,
    );
    lines
}

fn header(tree: &Tree, options: &RenderOptions) -> String {
    let root = &tree.nodes[ROOT];
    let mut facts = vec![
        options.kind.label().to_string(),
        options.source.label().to_string(),
        format!("{} files", root.file_count),
    ];
    if root.test_count > 0 {
        facts.push(if options.include_tests {
            format!("{} tests", root.test_count)
        } else {
            format!("{} tests hidden", root.test_count)
        });
    }
    format!("{}/  [{}]", tree.root_name, facts.join(" · "))
}

fn append_children(
    tree: &Tree,
    id: NodeId,
    prefix: &str,
    lines: &mut Vec<String>,
    allocation: &Allocation,
    include_tests: bool,
) {
    let children = visible_children(tree, id, include_tests);
    let last_index = children.len().saturating_sub(1);

    for (position, child_id) in children.iter().enumerate() {
        let child = &tree.nodes[*child_id];
        let is_last = position == last_index;
        let expanded = allocation.expanded.contains(child_id);
        let label = if child.is_dir {
            format!("{}/", child.name)
        } else {
            child.name.clone()
        };
        let summary = if child.is_dir && !expanded {
            collapsed_summary(child, include_tests)
        } else {
            String::new()
        };
        let branch = if is_last { "└── " } else { "├── " };
        lines.push(format!("{prefix}{branch}{label}{summary}"));

        if child.is_dir && expanded {
            let continuation = if is_last { "    " } else { "│   " };
            append_children(
                tree,
                *child_id,
                &format!("{prefix}{continuation}"),
                lines,
                allocation,
                include_tests,
            );
        }
    }
}
