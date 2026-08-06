use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

use regex_engine::Regex;
use search::search;

use crate::{args::parse, error::AppError};

mod args;
mod error;

fn run() -> Result<(), AppError> {
    let args = parse(&mut std::env::args().skip(1))?;

    let regex = Regex::new(&args.pattern)?;

    let reader: Box<dyn BufRead> = match args.path {
        Some(path) => Box::new(BufReader::new(File::open(path)?)),
        None => Box::new(BufReader::new(io::stdin().lock())),
    };

    for result in search(&regex, reader) {
        println!("{}: {}", result.line_number, result.line.trim_end());
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
