use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader},
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, mpsc},
};

use regex_engine::{ParseError, RegexBuilder};
use search::{LiteralMatcher, Matcher, MultiMatcher, search};

use crate::{
    args::{Args, parse},
    colorizer::Colorizer,
    error::{AppError, AppResult},
    gitignore::GitignoreCache,
    output::{Output, OutputMode},
    pool,
    walk::walk,
};

pub struct App {
    args: Args,
    matcher: Arc<dyn Matcher>,
    gitignore: Rc<GitignoreCache>,
    output: Output<io::Stdout>,
}

impl App {
    pub fn new() -> AppResult<Self> {
        let args = parse(&mut std::env::args().skip(1))?;
        let matcher = create_matcher_from_args(&args)?;

        let mut root = None;
        if let Some(ref path) = args.path
            && path.is_dir()
        {
            root = Some(path.clone());
        }
        let gitignore = Rc::new(GitignoreCache::new(root.clone().unwrap_or(PathBuf::new())));
        let output = Output::new(
            args.output_mode,
            io::stdout(),
            args.output_mode == OutputMode::Matches && root.is_some(),
            args.before_context > 0 || args.after_context > 0,
            Colorizer::from_stdout(),
            args.only_matching,
        );

        Ok(Self {
            args,
            matcher,
            gitignore,
            output,
        })
    }

    pub fn run(&mut self) -> AppResult<RunOutcome> {
        match self.args.path.clone() {
            Some(root) if root.is_dir() => {
                let mut had_error = false;

                let gitignore = self.gitignore.clone();

                let (paths_tx, paths_rx) = mpsc::channel();
                let (results, handles) = pool::search_files(
                    paths_rx,
                    Arc::clone(&self.matcher),
                    self.args.invert_match,
                    self.args.before_context,
                    self.args.after_context,
                    self.args.threads_count,
                );

                for entry in walk(&root, |path, is_dir| {
                    Self::should_skip_path(path, is_dir, &root, &gitignore)
                }) {
                    match entry {
                        Ok(path) => paths_tx.send(path).unwrap(),
                        Err(e) => {
                            eprintln!("{}", AppError::from(e));
                            had_error = true;
                        }
                    }
                }
                drop(paths_tx);

                for file_result in results {
                    match file_result.matches {
                        Ok(matches) => self
                            .output
                            .report(&mut matches.into_iter().peekable(), Some(&file_result.path)),
                        Err(e) => {
                            eprintln!("{}", e);
                            had_error = true;
                        }
                    }
                }

                for handle in handles {
                    if handle.join().is_err() {
                        had_error = true;
                    }
                }

                Ok(self.run_outcome(had_error))
            }
            Some(path) => {
                self.search_file(&path)?;
                Ok(self.run_outcome(false))
            }
            None => {
                self.search_stdin();
                Ok(self.run_outcome(false))
            }
        }
    }

    fn run_outcome(&self, had_error: bool) -> RunOutcome {
        if had_error {
            RunOutcome::HadError
        } else if self.output.matched() {
            RunOutcome::Matched
        } else {
            RunOutcome::NoMatches
        }
    }

    fn search_file(&mut self, path: &Path) -> AppResult<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut matches = search(
            self.matcher.as_ref(),
            reader,
            self.args.invert_match,
            self.args.before_context,
            self.args.after_context,
        )
        .peekable();

        self.output.report(&mut matches, Some(path));

        Ok(())
    }

    fn search_stdin(&mut self) {
        let stdin = io::stdin().lock();
        let reader = BufReader::new(stdin);

        let matches = &mut search(
            self.matcher.as_ref(),
            reader,
            self.args.invert_match,
            self.args.before_context,
            self.args.after_context,
        )
        .peekable();

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

fn create_matcher_from_args(args: &Args) -> AppResult<Arc<dyn Matcher>> {
    let matchers = args
        .patterns
        .iter()
        .map(|pattern| build_matcher(pattern, args))
        .collect::<Result<Vec<Box<dyn Matcher>>, ParseError>>()?;

    Ok(Arc::new(MultiMatcher::new(matchers)))
}

fn build_matcher(pattern: &str, args: &Args) -> Result<Box<dyn Matcher>, ParseError> {
    if args.fixed_strings {
        return Ok(Box::new(LiteralMatcher::new(
            pattern,
            args.case_insensitive,
            args.whole_word,
        )));
    }

    Ok(Box::new(
        RegexBuilder::new(pattern.to_string())
            .case_insensitive(args.case_insensitive)
            .whole_word(args.whole_word)
            .build()?,
    ))
}

pub enum RunOutcome {
    Matched,
    NoMatches,
    HadError,
}

#[cfg(test)]
mod tests {
    use super::*;
    use search::LineMatch;
    use std::fs;

    fn args_with_patterns(patterns: Vec<&str>) -> Args {
        Args {
            patterns: patterns.into_iter().map(String::from).collect(),
            path: None,
            output_mode: OutputMode::Matches,
            case_insensitive: false,
            whole_word: false,
            invert_match: false,
            before_context: 0,
            after_context: 0,
            only_matching: false,
            fixed_strings: false,
            threads_count: None,
        }
    }

    fn make_app(matched: bool) -> App {
        let args = args_with_patterns(vec!["x"]);
        let matcher: Arc<dyn Matcher> =
            Arc::new(RegexBuilder::new(args.patterns[0].clone()).build().unwrap());
        let gitignore = Rc::new(GitignoreCache::new(PathBuf::new()));
        let mut output = Output::new(
            OutputMode::Matches,
            io::stdout(),
            false,
            false,
            Colorizer::new(false),
            false,
        );

        if matched {
            output.report(
                #[allow(clippy::single_range_in_vec_init)]
                &mut vec![LineMatch {
                    line_number: 1,
                    line: "hello".to_string(),
                    match_spans: vec![0..5],
                    is_context: false,
                }]
                .into_iter()
                .peekable(),
                None,
            );
        }

        App {
            args,
            matcher,
            gitignore,
            output,
        }
    }

    #[test]
    fn run_outcome_is_had_error_when_had_error_is_true_regardless_of_matches() {
        assert!(matches!(
            make_app(true).run_outcome(true),
            RunOutcome::HadError
        ));
        assert!(matches!(
            make_app(false).run_outcome(true),
            RunOutcome::HadError
        ));
    }

    #[test]
    fn run_outcome_is_matched_when_there_was_no_error_and_output_matched() {
        assert!(matches!(
            make_app(true).run_outcome(false),
            RunOutcome::Matched
        ));
    }

    #[test]
    fn run_outcome_is_no_matches_when_there_was_no_error_and_output_did_not_match() {
        assert!(matches!(
            make_app(false).run_outcome(false),
            RunOutcome::NoMatches
        ));
    }

    #[test]
    fn build_matcher_returns_a_regex_matcher_by_default() {
        let args = args_with_patterns(vec!["ab"]);
        let matcher = build_matcher("ab", &args).unwrap();
        assert_eq!(matcher.find("xxabxx"), Some(2..4));
    }

    #[test]
    fn build_matcher_returns_a_literal_matcher_when_fixed_strings_is_set() {
        let mut args = args_with_patterns(vec!["a.c"]);
        args.fixed_strings = true;
        let matcher = build_matcher("a.c", &args).unwrap();
        assert_eq!(matcher.find("abc"), None);
        assert_eq!(matcher.find("xxa.cxx"), Some(2..5));
    }

    #[test]
    fn build_matcher_propagates_invalid_regex_errors() {
        let args = args_with_patterns(vec!["("]);
        assert!(build_matcher("(", &args).is_err());
    }

    #[test]
    fn create_matcher_from_args_matches_any_of_multiple_patterns() {
        let args = args_with_patterns(vec!["foo", "bar"]);
        let matcher = create_matcher_from_args(&args).unwrap();

        assert_eq!(matcher.find("xx bar xx"), Some(3..6));
        assert_eq!(matcher.find("xx foo xx"), Some(3..6));
        assert_eq!(matcher.find("xx baz xx"), None);
    }

    #[test]
    fn create_matcher_from_args_propagates_parse_errors() {
        let args = args_with_patterns(vec!["("]);
        assert!(create_matcher_from_args(&args).is_err());
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("rgrep_app_test_{name}_{}", std::process::id()));
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
