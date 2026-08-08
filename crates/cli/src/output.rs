use std::{iter::Peekable, path::Path};

use search::LineMatch;

use crate::colorizer::Colorizer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Matches,
    Count,
}

pub fn report(
    mode: OutputMode,
    colorizer: &Colorizer,
    matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
    path: Option<&Path>,
    header: Option<&mut bool>,
) {
    match mode {
        OutputMode::Matches => report_matches(colorizer, matches, path, header),
        OutputMode::Count => report_count(colorizer, matches, path),
    }
}

fn report_matches(
    colorizer: &Colorizer,
    matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
    path: Option<&Path>,
    header: Option<&mut bool>,
) {
    if matches.peek().is_some()
        && let Some(path) = path
    {
        if let Some(separator_needed) = header {
            if *separator_needed {
                println!();
            } else {
                *separator_needed = true;
            }
        }

        println!("{}", colorizer.path(path));
    }

    for line_match in matches {
        print!("{}:", colorizer.line_number(line_match.line_number));
        let line = line_match.line.trim_end();
        print!("{}", &line[..line_match.start]);
        print!(
            "{}",
            colorizer.matched(&line[line_match.start..line_match.end])
        );
        println!("{}", &line[line_match.end..]);
    }
}

fn report_count(
    colorizer: &Colorizer,
    matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
    path: Option<&Path>,
) {
    if matches.peek().is_some() {
        if let Some(path) = path {
            print!("{}:", colorizer.path(path));
        }
        println!("{}", matches.into_iter().count());
    }
}
