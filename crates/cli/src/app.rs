use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader},
    path::Path,
};

use regex_engine::{Regex, RegexBuilder};
use search::search;

use crate::{
    args::{Args, parse},
    error::{AppError, AppResult},
    gitignore::Gitignore,
    output::{Output, OutputMode},
    walk::walk,
};

pub struct App {
    args: Args,
    regex: Regex,
    gitignore: Gitignore,
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

    fn should_skip_path(path: &Path, is_dir: bool, root: &Path, gitignore: &Gitignore) -> bool {
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

    #[test]
    fn skips_the_git_directory_regardless_of_gitignore_content() {
        let gitignore = Gitignore::empty();
        let root = Path::new("/repo");
        assert!(App::should_skip_path(
            Path::new("/repo/.git"),
            true,
            root,
            &gitignore
        ));
    }

    #[test]
    fn does_not_skip_a_file_that_is_not_a_directory_named_dot_git() {
        let gitignore = Gitignore::empty();
        let root = Path::new("/repo");
        assert!(!App::should_skip_path(
            Path::new("/repo/.git"),
            false,
            root,
            &gitignore
        ));
    }

    #[test]
    fn skips_files_matched_by_gitignore() {
        let gitignore = Gitignore::parse("*.log");
        let root = Path::new("/repo");
        assert!(App::should_skip_path(
            Path::new("/repo/debug.log"),
            false,
            root,
            &gitignore
        ));
    }

    #[test]
    fn does_not_skip_files_not_matched_by_gitignore() {
        let gitignore = Gitignore::parse("*.log");
        let root = Path::new("/repo");
        assert!(!App::should_skip_path(
            Path::new("/repo/main.rs"),
            false,
            root,
            &gitignore
        ));
    }

    #[test]
    fn matches_relative_to_root_not_the_full_path() {
        let gitignore = Gitignore::parse("/build");
        let root = Path::new("/repo");
        assert!(App::should_skip_path(
            Path::new("/repo/build"),
            true,
            root,
            &gitignore
        ));
        assert!(!App::should_skip_path(
            Path::new("/repo/sub/build"),
            true,
            root,
            &gitignore
        ));
    }
}
