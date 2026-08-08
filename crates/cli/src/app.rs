use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader},
    path::{Path, PathBuf},
};

use regex_engine::Regex;
use search::search;

use crate::{
    args::{Args, parse},
    colorizer::Colorizer,
    error::{AppError, AppResult},
    gitignore::Gitignore,
    output::{OutputMode, report},
    walk::walk,
};

pub struct App {
    args: Args,
    regex: Regex,
    root: Option<PathBuf>,
    gitignore: Gitignore,
    colorizer: Colorizer,
    output_mode: OutputMode,
}

impl App {
    pub fn new() -> AppResult<Self> {
        let args = parse(&mut std::env::args().skip(1))?;
        let regex = Regex::new(&args.pattern)?;
        let colorizer = Colorizer::from_stdout();

        let mut root = None;
        let gitignore = if let Some(ref path) = args.path
            && path.is_dir()
        {
            root = Some(path.clone());
            match std::fs::read_to_string(path.join(".gitignore")) {
                Ok(contents) => Gitignore::parse(&contents),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Gitignore::empty(),
                Err(e) => return Err(e.into()),
            }
        } else {
            Gitignore::empty()
        };

        let output_mode = if args.count_only {
            OutputMode::Count
        } else {
            OutputMode::Matches
        };

        Ok(Self {
            args,
            regex,
            root,
            gitignore,
            colorizer,
            output_mode,
        })
    }

    pub fn run(&self) -> AppResult<bool> {
        let had_error = match &self.args.path {
            Some(root) if root.is_dir() => {
                let mut separator_needed = false;
                let mut had_error = false;

                for entry in walk(root, |path, is_dir| self.should_skip_path(path, is_dir)) {
                    let result = entry.map_err(AppError::from).and_then(|file_path| {
                        self.search_file(&file_path, Some(&mut separator_needed))
                    });

                    if let Err(e) = result {
                        eprintln!("{}", e);
                        had_error = true;
                    }
                }

                had_error
            }
            Some(path) => {
                self.search_file(path, None)?;
                false
            }
            None => {
                self.search_stdin();
                false
            }
        };

        Ok(had_error)
    }

    fn search_file(&self, path: &Path, header: Option<&mut bool>) -> AppResult<()> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut matches = search(&self.regex, reader).peekable();

        report(
            self.output_mode,
            &self.colorizer,
            &mut matches,
            Some(path),
            header,
            &mut io::stdout(),
        );

        Ok(())
    }

    fn search_stdin(&self) {
        let stdin = io::stdin().lock();
        let reader = BufReader::new(stdin);

        let matches = &mut search(&self.regex, reader).peekable();

        report(
            self.output_mode,
            &self.colorizer,
            matches,
            None,
            None,
            &mut io::stdout(),
        );
    }

    fn should_skip_path(&self, path: &Path, is_dir: bool) -> bool {
        if is_dir && path.file_name() == Some(OsStr::new(".git")) {
            return true;
        }

        let relative = match &self.root {
            Some(root) => path.strip_prefix(root).unwrap_or(path),
            None => path,
        };
        self.gitignore.is_ignored(relative, is_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app_with(root: &str, gitignore: &str) -> App {
        App {
            args: Args {
                pattern: String::new(),
                path: None,
                count_only: false,
            },
            regex: Regex::new("x").unwrap(),
            root: Some(PathBuf::from(root)),
            gitignore: Gitignore::parse(gitignore),
            colorizer: Colorizer::from_stdout(),
            output_mode: OutputMode::Matches,
        }
    }

    #[test]
    fn skips_the_git_directory_regardless_of_gitignore_content() {
        let app = app_with("/repo", "");
        assert!(app.should_skip_path(Path::new("/repo/.git"), true));
    }

    #[test]
    fn does_not_skip_a_file_that_is_not_a_directory_named_dot_git() {
        let app = app_with("/repo", "");
        assert!(!app.should_skip_path(Path::new("/repo/.git"), false));
    }

    #[test]
    fn skips_files_matched_by_gitignore() {
        let app = app_with("/repo", "*.log");
        assert!(app.should_skip_path(Path::new("/repo/debug.log"), false));
    }

    #[test]
    fn does_not_skip_files_not_matched_by_gitignore() {
        let app = app_with("/repo", "*.log");
        assert!(!app.should_skip_path(Path::new("/repo/main.rs"), false));
    }

    #[test]
    fn matches_relative_to_root_not_the_full_path() {
        let app = app_with("/repo", "/build");
        assert!(app.should_skip_path(Path::new("/repo/build"), true));
        assert!(!app.should_skip_path(Path::new("/repo/sub/build"), true));
    }
}
