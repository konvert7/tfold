use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;

const MAX_SCANNED_BYTES: u64 = 2 * 1024 * 1024;
const BINARY_SNIFF_BYTES: usize = 8000;

pub struct Pattern {
    needle: String,
    ignore_case: bool,
}

impl Pattern {
    pub fn new(needle: &str, ignore_case: bool) -> Self {
        Pattern {
            needle: if ignore_case {
                needle.to_lowercase()
            } else {
                needle.to_string()
            },
            ignore_case,
        }
    }

    fn matching_lines(&self, text: &str) -> usize {
        let haystack: Cow<str> = if self.ignore_case {
            Cow::Owned(text.to_lowercase())
        } else {
            Cow::Borrowed(text)
        };
        haystack
            .lines()
            .filter(|line| line.contains(&self.needle))
            .count()
    }
}

pub fn validate_pattern(pattern: &str) -> Result<String, String> {
    if pattern.is_empty() {
        return Err("pattern is empty".to_string());
    }
    Ok(pattern.to_string())
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(BINARY_SNIFF_BYTES).any(|byte| *byte == 0)
}

fn matching_lines_in(path: &Path, pattern: &Pattern) -> usize {
    let Ok(metadata) = std::fs::metadata(path) else {
        return 0;
    };
    if metadata.len() > MAX_SCANNED_BYTES {
        return 0;
    }
    let Ok(bytes) = std::fs::read(path) else {
        return 0;
    };
    if is_binary(&bytes) {
        return 0;
    }
    pattern.matching_lines(&String::from_utf8_lossy(&bytes))
}

fn worker_count(files: usize) -> usize {
    let cores = std::thread::available_parallelism().map_or(1, |count| count.get());
    cores.min(files).max(1)
}

pub fn scan(root: &Path, files: &[String], pattern: &Pattern) -> HashMap<String, usize> {
    if files.is_empty() {
        return HashMap::new();
    }
    let workers = worker_count(files.len());
    let chunk_size = files.len().div_ceil(workers);

    std::thread::scope(|scope| {
        let handles: Vec<_> = files
            .chunks(chunk_size)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|file| (file.clone(), matching_lines_in(&root.join(file), pattern)))
                        .filter(|(_, count)| *count > 0)
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_default())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_with_several_occurrences_counts_once() {
        let pattern = Pattern::new("ab", false);
        assert_eq!(pattern.matching_lines("ab ab ab\nnothing\nab\n"), 2);
    }

    #[test]
    fn ignore_case_matches_regardless_of_casing() {
        assert_eq!(Pattern::new("AbC", true).matching_lines("xxabcxx\n"), 1);
        assert_eq!(Pattern::new("AbC", false).matching_lines("xxabcxx\n"), 0);
    }

    #[test]
    fn a_nul_byte_marks_content_as_binary() {
        assert!(is_binary(b"head\0tail"));
        assert!(!is_binary(b"plain text"));
    }

    #[test]
    fn an_empty_pattern_is_rejected() {
        assert!(validate_pattern("").is_err());
        assert!(validate_pattern("x").is_ok());
    }
}
