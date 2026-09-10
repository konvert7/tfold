use std::path::Path;

use ignore::WalkBuilder;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    Git,
    Walk,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Git => "git",
            Source::Walk => "walk",
        }
    }
}

const ALWAYS_SKIP: &[&str] = &[".git"];

const NEVER_INTERESTING: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    "vendor",
    ".cache",
    ".turbo",
    ".gradle",
    "coverage",
    ".idea",
];

pub fn collect(root: &Path) -> (Vec<String>, Source) {
    let source = if is_inside_git_work_tree(root) {
        Source::Git
    } else {
        Source::Walk
    };

    let trust_ignore_files = source == Source::Git;
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .parents(true)
        .filter_entry(move |entry| !is_skipped_dir(entry, trust_ignore_files))
        .build();

    let mut files = Vec::new();
    for entry in walker.flatten() {
        if !entry
            .file_type()
            .is_some_and(|kind| kind.is_file() || kind.is_symlink())
        {
            continue;
        }
        if let Ok(relative) = entry.path().strip_prefix(root) {
            let path = relative.to_string_lossy().replace('\\', "/");
            if !path.is_empty() {
                files.push(path);
            }
        }
    }
    files.sort();
    (files, source)
}

fn is_inside_git_work_tree(root: &Path) -> bool {
    let start_device = device_of(root);
    for ancestor in root.ancestors() {
        if device_of(ancestor) != start_device {
            return false;
        }
        if ancestor.join(".git").exists() {
            return true;
        }
    }
    false
}

#[cfg(unix)]
fn device_of(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|metadata| metadata.dev())
}

#[cfg(not(unix))]
fn device_of(_path: &Path) -> Option<u64> {
    None
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn device_of_distinguishes_separate_filesystems() {
        let (root, temp) = (device_of(Path::new("/")), device_of(Path::new("/tmp")));
        assert!(root.is_some() && temp.is_some(), "stat failed: {root:?} {temp:?}");
        if root == temp {
            eprintln!("skipped: / and /tmp share a filesystem on this machine");
            return;
        }
        assert_ne!(
            root, temp,
            "the mount-boundary guard cannot fire if device ids do not differ across filesystems"
        );
    }

    #[test]
    fn a_repo_root_is_recognised_as_a_work_tree() {
        assert!(
            is_inside_git_work_tree(Path::new(env!("CARGO_MANIFEST_DIR"))),
            "treefold's own checkout should be detected as a git work tree"
        );
    }
}

fn is_skipped_dir(entry: &ignore::DirEntry, trust_ignore_files: bool) -> bool {
    if !entry.file_type().is_some_and(|kind| kind.is_dir()) {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    if ALWAYS_SKIP.contains(&name.as_ref()) {
        return true;
    }
    !trust_ignore_files && NEVER_INTERESTING.contains(&name.as_ref())
}
