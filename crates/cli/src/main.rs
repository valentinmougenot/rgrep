use std::{
    ffi::OsStr,
    fs::File,
    io::{self, BufReader},
    path::Path,
};

use regex_engine::Regex;
use search::search;

use crate::{args::parse, error::AppError, gitignore::Gitignore, walk::walk};

mod args;
mod error;
mod gitignore;
mod walk;

fn run() -> Result<bool, AppError> {
    let args = parse(&mut std::env::args().skip(1))?;

    let regex = Regex::new(&args.pattern)?;

    let had_error = match args.path {
        Some(root) if root.is_dir() => {
            let contents = match std::fs::read_to_string(root.join(".gitignore")) {
                Ok(contents) => contents,
                Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
                Err(e) => return Err(e.into()),
            };
            let gitignore = Gitignore::parse(&contents);
            let should_skip = |path: &Path, is_dir: bool| {
                if is_dir && path.file_name() == Some(OsStr::new(".git")) {
                    return true;
                }

                let relative = path.strip_prefix(&root).unwrap_or(path);
                gitignore.is_ignored(relative, is_dir)
            };

            let mut separator_needed = false;
            let mut had_error = false;

            for entry in walk(&root, should_skip) {
                let result = entry.map_err(AppError::from).and_then(|file_path| {
                    search_file(&regex, &file_path, Some(&mut separator_needed))
                });

                if let Err(e) = result {
                    eprintln!("{}", e);
                    had_error = true;
                }
            }

            had_error
        }
        Some(path) => {
            search_file(&regex, &path, None)?;
            false
        }
        None => {
            search_stdin(&regex);
            false
        }
    };

    Ok(had_error)
}

fn search_file(regex: &Regex, path: &Path, header: Option<&mut bool>) -> Result<(), AppError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut matches = search(regex, reader).peekable();

    if matches.peek().is_some() {
        if let Some(separator_needed) = header {
            if *separator_needed {
                println!();
            }
            println!("{}", path.display());
            *separator_needed = true;
        }

        for m in matches {
            println!("{}:{}", m.line_number, m.line.trim_end());
        }
    }

    Ok(())
}

fn search_stdin(regex: &Regex) {
    let stdin = io::stdin().lock();
    let reader = BufReader::new(stdin);

    for m in search(regex, reader) {
        println!("{}:{}", m.line_number, m.line.trim_end());
    }
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
