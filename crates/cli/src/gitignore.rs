use std::{os::unix::ffi::OsStrExt, path::Path};

pub struct Gitignore {
    patterns: Vec<Pattern>,
}

impl Gitignore {
    pub fn parse(contents: &str) -> Self {
        let mut patterns = Vec::new();

        for line in contents.lines() {
            let mut line = line;
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let negative = if line.starts_with('!') {
                line = line.trim_start_matches("!");
                true
            } else {
                false
            };

            let dir_only = if line.ends_with('/') {
                line = line.trim_end_matches("/");
                true
            } else {
                false
            };

            let anchored = if line.starts_with('/') {
                line = line.trim_start_matches("/");
                true
            } else {
                line.contains('/')
            };

            let glob = line.to_string();

            patterns.push(Pattern {
                negative,
                dir_only,
                anchored,
                glob,
            });
        }

        Self { patterns }
    }

    pub fn is_ignored(&self, relative_path: &Path, is_dir: bool) -> bool {
        let mut ignored = false;

        for pattern in &self.patterns {
            if pattern.dir_only && !is_dir {
                continue;
            }

            if pattern.matches(relative_path) {
                ignored = !pattern.negative;
            }
        }

        ignored
    }
}

struct Pattern {
    negative: bool,
    anchored: bool,
    dir_only: bool,
    glob: String,
}

impl Pattern {
    fn matches(&self, relative_path: &Path) -> bool {
        if self.anchored {
            glob_match(
                self.glob.as_bytes(),
                relative_path.to_string_lossy().as_bytes(),
            )
        } else {
            let name = relative_path.file_name().unwrap_or_default().as_bytes();
            glob_match(self.glob.as_bytes(), name)
        }
    }
}

fn glob_match(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.first() {
        Some(b'*') => {
            glob_match(&pattern[1..], text)
                || text.first().is_some_and(|&t| t != b'/') && glob_match(pattern, &text[1..])
        }
        Some(b'?') => {
            text.first().is_some_and(|&t| t != b'/') && glob_match(&pattern[1..], &text[1..])
        }
        Some(c) => text.first().is_some_and(|t| t == c) && glob_match(&pattern[1..], &text[1..]),
        None => text.is_empty(),
    }
}
