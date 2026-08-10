use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader},
    path::{Path, PathBuf},
};

use regex_engine::{Regex, RegexBuilder};
use search::search;

use crate::{
    args::{Args, parse},
    error::{AppError, AppResult},
    gitignore::GitignoreCache,
    output::{Output, OutputMode},
    walk::walk,
};

pub struct App {
    args: Args,
    regex: Regex,
    gitignore: GitignoreCache,
    output: Output<io::Stdout>,
}

impl App {
    pub fn new() -> AppResult<Self> {
        let args = parse(&mut std::env::args().skip(1))?;
        let regex = RegexBuilder::new(args.pattern.clone())
            .case_insensitive(args.case_insensitive)
            .whole_word(args.whole_word)
            .build()?;

        let mut root = None;
        if let Some(ref path) = args.path
            && path.is_dir()
        {
            root = Some(path.clone());
        }
        let gitignore = GitignoreCache::new(root.clone().unwrap_or(PathBuf::new()));
        let output = Output::new(
            args.output_mode,
            io::stdout(),
            args.output_mode == OutputMode::Matches && root.is_some(),
        );

        Ok(Self {
            args,
            regex,
            gitignore,
            output,
        })
    }

    pub fn run(&mut self) -> AppResult<bool> {
        let had_error = match self.args.path.clone() {
            Some(root) if root.is_dir() => {
                let mut had_error = false;

                let gitignore = self.gitignore.clone();
                for entry in walk(&root, |path, is_dir| {
                    Self::should_skip_path(path, is_dir, &root, &gitignore)
                }) {
                    let result = entry
                        .map_err(AppError::from)
                        .and_then(|file_path| self.search_file(&file_path));

                    if let Err(e) = result {
                        eprintln!("{}", e);
                        had_error = true;
                    }
                }

                had_error
            }
            Some(path) => {
                self.search_file(&path)?;
                false
            }
            None => {
                self.search_stdin();
                false
            }
        };

        Ok(had_error)
    }

    fn search_file(&mut self, path: &Path) -> AppResult<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut matches = search(&self.regex, reader, self.args.invert_match).peekable();

        self.output.report(&mut matches, Some(path));

        Ok(())
    }

    fn search_stdin(&mut self) {
        let stdin = io::stdin().lock();
        let reader = BufReader::new(stdin);

        let matches = &mut search(&self.regex, reader, self.args.invert_match).peekable();

        self.output.report(matches, None);
    }

    fn should_skip_path(
        path: &Path,
        is_dir: bool,
        root: &Path,
        gitignore: &GitignoreCache,
    ) -> bool {
        if is_dir && path.file_name() == Some(OsStr::new(".git")) {
            return true;
        }

        let relative = path.strip_prefix(root).unwrap_or(path);
        gitignore.is_ignored(relative, is_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("rgrep_app_test_{name}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }

        fn write_gitignore(&self, relative_dir: &str, contents: &str) {
            let dir = self.0.join(relative_dir);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(".gitignore"), contents).unwrap();
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn skips_the_git_directory_regardless_of_gitignore_content() {
        let dir = TempDir::new("skips_git_dir");
        let gitignore = GitignoreCache::new(dir.path().to_path_buf());

        assert!(App::should_skip_path(
            &dir.path().join(".git"),
            true,
            dir.path(),
            &gitignore
        ));
    }

    #[test]
    fn does_not_skip_a_file_that_is_not_a_directory_named_dot_git() {
        let dir = TempDir::new("does_not_skip_dot_git_file");
        let gitignore = GitignoreCache::new(dir.path().to_path_buf());

        assert!(!App::should_skip_path(
            &dir.path().join(".git"),
            false,
            dir.path(),
            &gitignore
        ));
    }

    #[test]
    fn skips_files_matched_by_gitignore() {
        let dir = TempDir::new("skips_matched");
        dir.write_gitignore("", "*.log");
        let gitignore = GitignoreCache::new(dir.path().to_path_buf());

        assert!(App::should_skip_path(
            &dir.path().join("debug.log"),
            false,
            dir.path(),
            &gitignore
        ));
    }

    #[test]
    fn does_not_skip_files_not_matched_by_gitignore() {
        let dir = TempDir::new("does_not_skip_unmatched");
        dir.write_gitignore("", "*.log");
        let gitignore = GitignoreCache::new(dir.path().to_path_buf());

        assert!(!App::should_skip_path(
            &dir.path().join("main.rs"),
            false,
            dir.path(),
            &gitignore
        ));
    }

    #[test]
    fn matches_relative_to_root_not_the_full_path() {
        let dir = TempDir::new("relative_to_root");
        dir.write_gitignore("", "/build");
        fs::create_dir_all(dir.path().join("sub")).unwrap();
        let gitignore = GitignoreCache::new(dir.path().to_path_buf());

        assert!(App::should_skip_path(
            &dir.path().join("build"),
            true,
            dir.path(),
            &gitignore
        ));
        assert!(!App::should_skip_path(
            &dir.path().join("sub/build"),
            true,
            dir.path(),
            &gitignore
        ));
    }

    #[test]
    fn a_nested_gitignore_can_re_include_a_file_ignored_by_a_parent() {
        let dir = TempDir::new("nested_negation");
        dir.write_gitignore("", "*.log");
        dir.write_gitignore("sub", "!important.log");
        let gitignore = GitignoreCache::new(dir.path().to_path_buf());

        assert!(App::should_skip_path(
            &dir.path().join("a.log"),
            false,
            dir.path(),
            &gitignore
        ));
        assert!(!App::should_skip_path(
            &dir.path().join("sub/important.log"),
            false,
            dir.path(),
            &gitignore
        ));
    }

    #[test]
    fn a_nested_gitignore_scopes_its_own_ignores_to_its_directory() {
        let dir = TempDir::new("nested_scope");
        dir.write_gitignore("sub", "*.tmp");
        let gitignore = GitignoreCache::new(dir.path().to_path_buf());

        assert!(App::should_skip_path(
            &dir.path().join("sub/cache.tmp"),
            false,
            dir.path(),
            &gitignore
        ));
        assert!(!App::should_skip_path(
            &dir.path().join("cache.tmp"),
            false,
            dir.path(),
            &gitignore
        ));
    }
}
