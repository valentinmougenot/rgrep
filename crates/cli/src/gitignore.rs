use std::{
    cell::RefCell,
    collections::HashMap,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Gitignore {
    patterns: Vec<Pattern>,
}

impl Gitignore {
    pub fn empty() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

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

    pub fn verdict(&self, relative_path: &Path, is_dir: bool) -> Option<bool> {
        let mut ignored = None;

        for pattern in &self.patterns {
            if pattern.dir_only && !is_dir {
                continue;
            }

            if pattern.matches(relative_path) {
                ignored = Some(!pattern.negative);
            }
        }

        ignored
    }

    pub fn is_ignored(&self, relative_path: &Path, is_dir: bool) -> bool {
        self.verdict(relative_path, is_dir).unwrap_or(false)
    }
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct GitignoreCache {
    root: PathBuf,
    cache: RefCell<HashMap<PathBuf, Gitignore>>,
}

impl GitignoreCache {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn is_ignored(&self, relative_path: &Path, is_dir: bool) -> bool {
        let mut ancestors: Vec<&Path> = relative_path
            .parent()
            .into_iter()
            .flat_map(Path::ancestors)
            .collect();

        ancestors.reverse();

        let mut ignored = false;
        for ancestor in ancestors {
            self.ensure_loaded(ancestor);

            let cache = self.cache.borrow();
            let gitignore = cache.get(ancestor).unwrap();
            let relative_to_level = relative_path
                .strip_prefix(ancestor)
                .unwrap_or(relative_path);
            ignored = gitignore
                .verdict(relative_to_level, is_dir)
                .unwrap_or(ignored);
        }

        ignored
    }

    fn ensure_loaded(&self, dir: &Path) {
        if self.cache.borrow().contains_key(dir) {
            return;
        }

        let gitignore = match std::fs::read_to_string(self.root.join(dir).join(".gitignore")) {
            Ok(contents) => Gitignore::parse(&contents),
            Err(_) => Gitignore::empty(),
        };

        self.cache.borrow_mut().insert(dir.to_path_buf(), gitignore);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_blank_lines_and_comments() {
        let gi = Gitignore::parse("\n# comment\n\ntarget\n");
        assert_eq!(gi.patterns.len(), 1);
        assert_eq!(gi.patterns[0].glob, "target");
    }

    #[test]
    fn basename_pattern_matches_at_any_depth() {
        let gi = Gitignore::parse("*.log");
        assert!(gi.is_ignored(Path::new("a.log"), false));
        assert!(gi.is_ignored(Path::new("sub/a.log"), false));
        assert!(!gi.is_ignored(Path::new("a.txt"), false));
    }

    #[test]
    fn leading_slash_anchors_to_root_only() {
        let gi = Gitignore::parse("/build");
        assert!(gi.is_ignored(Path::new("build"), true));
        assert!(!gi.is_ignored(Path::new("sub/build"), true));
    }

    #[test]
    fn internal_slash_anchors_and_keeps_the_slash() {
        let gi = Gitignore::parse("src/gen");
        assert!(gi.is_ignored(Path::new("src/gen"), true));
        assert!(!gi.is_ignored(Path::new("other/src/gen"), true));
    }

    #[test]
    fn trailing_slash_only_matches_directories() {
        let gi = Gitignore::parse("build/");
        assert!(gi.is_ignored(Path::new("build"), true));
        assert!(!gi.is_ignored(Path::new("build"), false));
    }

    #[test]
    fn negation_overrides_a_previous_broader_match() {
        let gi = Gitignore::parse("*.log\n!important.log\n");
        assert!(gi.is_ignored(Path::new("a.log"), false));
        assert!(!gi.is_ignored(Path::new("important.log"), false));
    }

    #[test]
    fn later_pattern_overrides_earlier_negation() {
        let gi = Gitignore::parse("!keep.txt\nkeep.txt\n");
        assert!(gi.is_ignored(Path::new("keep.txt"), false));
    }

    #[test]
    fn unmatched_path_is_not_ignored() {
        let gi = Gitignore::parse("*.log");
        assert!(!gi.is_ignored(Path::new("readme.md"), false));
    }
}
