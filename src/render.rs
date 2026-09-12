use crate::classify::Kind;
use crate::collect::Source;
use crate::tree::{Allocation, NodeId, ROOT, Tree, dir_summary, plural, visible_children};
use crate::{Grep, Since};

const INDENT: &str = "  ";

pub struct RenderOptions {
    pub kind: Kind,
    pub source: Source,
    pub include_tests: bool,
    pub since: Since,
    pub grep: Grep,
}

pub fn render(tree: &Tree, allocation: &Allocation, options: &RenderOptions) -> Vec<String> {
    let mut lines = vec![header(tree, options)];
    append_children(
        tree,
        ROOT,
        INDENT,
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
    match &options.since {
        Since::Off => {}
        Since::Changes(reference, _) => {
            facts.push(format!("{} changed since {reference}", root.changed_count))
        }
        Since::Unavailable => facts.push("--since needs git, ignored".to_string()),
    }
    if let Grep::Found { lines, files } = options.grep {
        facts.push(format!(
            "{} in {}",
            plural(lines, "matching line"),
            plural(files, "file")
        ));
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
    for child_id in visible_children(tree, id, include_tests) {
        let child = &tree.nodes[child_id];
        let label = if child.is_dir {
            format!("{}/", child.name)
        } else {
            child.name.clone()
        };
        lines.push(format!(
            "{prefix}{label}{}",
            dir_summary(child, include_tests)
        ));

        if child.is_dir && allocation.expanded.contains(&child_id) {
            append_children(
                tree,
                child_id,
                &format!("{prefix}{INDENT}"),
                lines,
                allocation,
                include_tests,
            );
        }
    }
}
