use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader},
    path::{Path, PathBuf},
};

use regex_engine::Regex;
use search::{LineMatch, search};

use crate::{
    args::{Args, parse},
    colorizer::Colorizer,
    error::AppError,
    gitignore::Gitignore,
    walk::walk,
};

pub struct App {
    args: Args,
    regex: Regex,
    root: Option<PathBuf>,
    gitignore: Gitignore,
    colorizer: Colorizer,
}

impl App {
    pub fn new() -> Result<Self, AppError> {
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

        Ok(Self {
            args,
            regex,
            root,
            gitignore,
            colorizer,
        })
    }

    pub fn run(&self) -> Result<bool, AppError> {
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

    fn search_file(&self, path: &Path, header: Option<&mut bool>) -> Result<(), AppError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let mut matches = search(&self.regex, reader).peekable();

        if matches.peek().is_some() {
            if let Some(separator_needed) = header {
                if *separator_needed {
                    println!();
                }
                println!("{}", self.colorizer.path(&path.display().to_string()));
                *separator_needed = true;
            }

            for m in matches {
                self.print_match(&m);
            }
        }

        Ok(())
    }

    fn search_stdin(&self) {
        let stdin = io::stdin().lock();
        let reader = BufReader::new(stdin);

        for m in search(&self.regex, reader) {
            self.print_match(&m);
        }
    }

    fn print_match(&self, line_match: &LineMatch) {
        print!("{}:", self.colorizer.line_number(line_match.line_number));
        let line = line_match.line.trim_end();
        print!("{}", &line[..line_match.start]);
        print!(
            "{}",
            self.colorizer
                .matched(&line[line_match.start..line_match.end])
        );
        println!("{}", &line[line_match.end..]);
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
            },
            regex: Regex::new("x").unwrap(),
            root: Some(PathBuf::from(root)),
            gitignore: Gitignore::parse(gitignore),
            colorizer: Colorizer::from_stdout(),
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
