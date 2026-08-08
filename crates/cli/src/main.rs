use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader},
    path::Path,
};

use regex_engine::Regex;
use search::{LineMatch, search};

use crate::{args::parse, colorizer::Colorizer, error::AppError, gitignore::Gitignore, walk::walk};

mod args;
mod colorizer;
mod error;
mod gitignore;
mod walk;

fn run() -> Result<bool, AppError> {
    let args = parse(&mut std::env::args().skip(1))?;

    let regex = Regex::new(&args.pattern)?;

    let colorizer = Colorizer::from_stdout();

    let had_error = match args.path {
        Some(root) if root.is_dir() => {
            let contents = match std::fs::read_to_string(root.join(".gitignore")) {
                Ok(contents) => contents,
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(e.into()),
            };
            let gitignore = Gitignore::parse(&contents);
            let should_skip =
                |path: &Path, is_dir: bool| should_skip_path(path, is_dir, &root, &gitignore);

            let mut separator_needed = false;
            let mut had_error = false;

            for entry in walk(&root, should_skip) {
                let result = entry.map_err(AppError::from).and_then(|file_path| {
                    search_file(&regex, &file_path, Some(&mut separator_needed), &colorizer)
                });

                if let Err(e) = result {
                    eprintln!("{}", e);
                    had_error = true;
                }
            }

            had_error
        }
        Some(path) => {
            search_file(&regex, &path, None, &colorizer)?;
            false
        }
        None => {
            search_stdin(&regex, &colorizer);
            false
        }
    };

    Ok(had_error)
}

fn search_file(
    regex: &Regex,
    path: &Path,
    header: Option<&mut bool>,
    colorizer: &Colorizer,
) -> Result<(), AppError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut matches = search(regex, reader).peekable();

    if matches.peek().is_some() {
        if let Some(separator_needed) = header {
            if *separator_needed {
                println!();
            }
            println!("{}", colorizer.path(&path.display().to_string()));
            *separator_needed = true;
        }

        for m in matches {
            print_match(&m, colorizer);
        }
    }

    Ok(())
}

fn should_skip_path(path: &Path, is_dir: bool, root: &Path, gitignore: &Gitignore) -> bool {
    if is_dir && path.file_name() == Some(OsStr::new(".git")) {
        return true;
    }

    let relative = path.strip_prefix(root).unwrap_or(path);
    gitignore.is_ignored(relative, is_dir)
}

fn search_stdin(regex: &Regex, colorizer: &Colorizer) {
    let stdin = io::stdin().lock();
    let reader = BufReader::new(stdin);

    for m in search(regex, reader) {
        print_match(&m, colorizer);
    }
}

fn print_match(line_match: &LineMatch, colorizer: &Colorizer) {
    print!("{}:", colorizer.line_number(line_match.line_number));
    let line = line_match.line.trim_end();
    print!("{}", &line[..line_match.start]);
    print!(
        "{}",
        colorizer.matched(&line[line_match.start..line_match.end])
    );
    println!("{}", &line[line_match.end..]);
}

fn main() {
    match run() {
        Ok(false) => {}
        Ok(true) => std::process::exit(2),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_the_git_directory_regardless_of_gitignore_content() {
        let gitignore = Gitignore::parse("");
        let root = Path::new("/repo");
        assert!(should_skip_path(
            Path::new("/repo/.git"),
            true,
            root,
            &gitignore
        ));
    }

    #[test]
    fn does_not_skip_a_file_that_is_not_a_directory_named_dot_git() {
        let gitignore = Gitignore::parse("");
        let root = Path::new("/repo");
        assert!(!should_skip_path(
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
        assert!(should_skip_path(
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
        assert!(!should_skip_path(
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
        assert!(should_skip_path(
            Path::new("/repo/build"),
            true,
            root,
            &gitignore
        ));
        assert!(!should_skip_path(
            Path::new("/repo/sub/build"),
            true,
            root,
            &gitignore
        ));
    }
}
