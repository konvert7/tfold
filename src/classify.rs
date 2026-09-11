#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Code,
    Docs,
    Mixed,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Code => "code",
            Kind::Docs => "docs",
            Kind::Mixed => "mixed",
        }
    }
}

const CODE_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "vue", "svelte", "rs", "go", "py", "rb", "java", "kt",
    "kts", "swift", "scala", "clj", "c", "h", "cc", "cpp", "hpp", "cs", "php", "ex", "exs", "erl",
    "hs", "sh", "bash", "ps1", "sql", "lua", "dart", "zig", "m", "mm",
];

const DOC_EXTENSIONS: &[&str] = &[
    "md", "mdx", "markdown", "txt", "rst", "adoc", "org", "pdf", "docx", "doc", "odt", "rtf",
    "epub", "tex",
];

const TEST_SEGMENTS: &[&str] = &[
    "test",
    "tests",
    "spec",
    "specs",
    "__tests__",
    "__test__",
    "e2e",
    "testdata",
];

const DEPRIORITIZED: &[&str] = &[
    "docs",
    "doc",
    "examples",
    "example",
    "samples",
    "demo",
    "fixtures",
    "vendor",
    "third_party",
    "generated",
    "dist",
    "build",
    "assets",
];

const ENTRYPOINT_STEMS: &[&str] = &[
    "index", "main", "mod", "lib", "cli", "app", "__init__", "readme",
];

const MANIFESTS: &[&str] = &[
    "package.json",
    "cargo.toml",
    "go.mod",
    "pyproject.toml",
    "setup.py",
    "pom.xml",
    "build.gradle",
    "gemfile",
    "composer.json",
    "deno.json",
];

fn extension_of(path: &str) -> String {
    let name = file_name(path);
    match name.rfind('.') {
        Some(dot) if dot > 0 => name[dot + 1..].to_lowercase(),
        _ => String::new(),
    }
}

fn file_name(path: &str) -> &str {
    match path.rfind('/') {
        Some(slash) => &path[slash + 1..],
        None => path,
    }
}

fn stem_of(name: &str) -> String {
    match name.rfind('.') {
        Some(dot) if dot > 0 => name[..dot].to_lowercase(),
        _ => name.to_lowercase(),
    }
}

pub fn classify_kind(files: &[String]) -> Kind {
    let mut code = 0usize;
    let mut docs = 0usize;
    for file in files {
        let extension = extension_of(file);
        if CODE_EXTENSIONS.contains(&extension.as_str()) {
            code += 1;
        } else if DOC_EXTENSIONS.contains(&extension.as_str()) {
            docs += 1;
        }
    }
    if code == 0 && docs == 0 {
        return Kind::Mixed;
    }
    let code_share = code as f64 / (code + docs) as f64;
    if code_share >= 0.35 {
        Kind::Code
    } else if code_share <= 0.1 {
        Kind::Docs
    } else {
        Kind::Mixed
    }
}

pub fn is_test_path(path: &str) -> bool {
    let segments: Vec<&str> = path.split('/').collect();
    let (name, parents) = match segments.split_last() {
        Some(parts) => parts,
        None => return false,
    };
    if parents
        .iter()
        .any(|segment| TEST_SEGMENTS.contains(&segment.to_lowercase().as_str()))
    {
        return true;
    }
    matches_test_file(name)
}

fn matches_test_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    let stem_before_extension = |suffix: &str| lower.contains(suffix);
    if stem_before_extension(".test.")
        || stem_before_extension(".spec.")
        || stem_before_extension("_test.")
        || stem_before_extension("_spec.")
        || lower.starts_with("test_")
    {
        return true;
    }
    matches!(extension_of(name).as_str(), "java" | "cs" | "kt" | "swift")
        && (stem_of(name).ends_with("test") || stem_of(name).ends_with("tests"))
}

pub fn is_deprioritized(name: &str) -> bool {
    DEPRIORITIZED.contains(&name.to_lowercase().as_str())
}

pub fn is_entrypoint(name: &str) -> bool {
    ENTRYPOINT_STEMS.contains(&stem_of(name).as_str())
}

pub fn is_manifest(name: &str) -> bool {
    MANIFESTS.contains(&name.to_lowercase().as_str())
}
