use std::{io::Write, iter::Peekable, path::Path};

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
    writer: &mut impl Write,
) {
    match mode {
        OutputMode::Matches => report_matches(colorizer, matches, path, header, writer),
        OutputMode::Count => report_count(colorizer, matches, path, writer),
    }
}

fn report_matches(
    colorizer: &Colorizer,
    matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
    path: Option<&Path>,
    header: Option<&mut bool>,
    writer: &mut impl Write,
) {
    if matches.peek().is_some()
        && let Some(path) = path
    {
        if let Some(separator_needed) = header {
            if *separator_needed {
                writeln!(writer).unwrap();
            } else {
                *separator_needed = true;
            }
        }

        writeln!(writer, "{}", colorizer.path(path)).unwrap();
    }

    for line_match in matches {
        write!(writer, "{}:", colorizer.line_number(line_match.line_number)).unwrap();
        let line = line_match.line.trim_end();
        write!(writer, "{}", &line[..line_match.start]).unwrap();
        write!(
            writer,
            "{}",
            colorizer.matched(&line[line_match.start..line_match.end])
        )
        .unwrap();
        writeln!(writer, "{}", &line[line_match.end..]).unwrap();
    }
}

fn report_count(
    colorizer: &Colorizer,
    matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
    path: Option<&Path>,
    writer: &mut impl Write,
) {
    if matches.peek().is_some() {
        if let Some(path) = path {
            write!(writer, "{}:", colorizer.path(path)).unwrap();
        }
        writeln!(writer, "{}", matches.count()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_match(line_number: usize, line: &str, start: usize, end: usize) -> LineMatch {
        LineMatch {
            line_number,
            line: line.to_string(),
            start,
            end,
        }
    }

    fn report_to_string(
        mode: OutputMode,
        colorizer: &Colorizer,
        matches: Vec<LineMatch>,
        path: Option<&Path>,
        header: Option<&mut bool>,
    ) -> String {
        let mut buf = Vec::new();
        report(
            mode,
            colorizer,
            &mut matches.into_iter().peekable(),
            path,
            header,
            &mut buf,
        );
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn matches_mode_prints_each_line_with_number() {
        let colorizer = Colorizer::new(false);
        let matches = vec![
            line_match(1, "hello world", 0, 5),
            line_match(3, "hello again", 0, 5),
        ];

        let output = report_to_string(OutputMode::Matches, &colorizer, matches, None, None);

        assert_eq!(output, "1:hello world\n3:hello again\n");
    }

    #[test]
    fn matches_mode_prints_nothing_when_there_are_no_matches() {
        let colorizer = Colorizer::new(false);
        let output = report_to_string(OutputMode::Matches, &colorizer, vec![], None, None);
        assert!(output.is_empty());
    }

    #[test]
    fn matches_mode_colorizes_the_matched_span() {
        let colorizer = Colorizer::new(true);
        let matches = vec![line_match(1, "hello world", 0, 5)];

        let output = report_to_string(OutputMode::Matches, &colorizer, matches, None, None);

        assert_eq!(
            output,
            "\x1b[32m1\x1b[0m:\x1b[1;31mhello\x1b[0m world\n"
        );
    }

    #[test]
    fn matches_mode_prints_the_header_when_a_path_is_given_and_there_are_matches() {
        let colorizer = Colorizer::new(false);
        let matches = vec![line_match(1, "hello", 0, 5)];
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Matches, &colorizer, matches, Some(path), None);

        assert_eq!(output, "a.txt\n1:hello\n");
    }

    #[test]
    fn matches_mode_omits_the_header_when_there_are_no_matches() {
        let colorizer = Colorizer::new(false);
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Matches, &colorizer, vec![], Some(path), None);

        assert!(output.is_empty());
    }

    #[test]
    fn matches_mode_separates_successive_headers_with_a_blank_line() {
        let colorizer = Colorizer::new(false);
        let mut separator_needed = false;
        let path = Path::new("a.txt");

        let first = report_to_string(
            OutputMode::Matches,
            &colorizer,
            vec![line_match(1, "hello", 0, 5)],
            Some(path),
            Some(&mut separator_needed),
        );
        let second = report_to_string(
            OutputMode::Matches,
            &colorizer,
            vec![line_match(1, "hello", 0, 5)],
            Some(path),
            Some(&mut separator_needed),
        );

        assert_eq!(first, "a.txt\n1:hello\n");
        assert_eq!(second, "\na.txt\n1:hello\n");
    }

    #[test]
    fn count_mode_prints_the_total_with_the_path() {
        let colorizer = Colorizer::new(false);
        let matches = vec![
            line_match(1, "hello", 0, 5),
            line_match(2, "hello", 0, 5),
            line_match(4, "hello", 0, 5),
        ];
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Count, &colorizer, matches, Some(path), None);

        assert_eq!(output, "a.txt:3\n");
    }

    #[test]
    fn count_mode_prints_just_the_total_without_a_path() {
        let colorizer = Colorizer::new(false);
        let matches = vec![line_match(1, "hello", 0, 5), line_match(2, "hello", 0, 5)];

        let output = report_to_string(OutputMode::Count, &colorizer, matches, None, None);

        assert_eq!(output, "2\n");
    }

    #[test]
    fn count_mode_prints_nothing_when_there_are_no_matches() {
        let colorizer = Colorizer::new(false);
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Count, &colorizer, vec![], Some(path), None);

        assert!(output.is_empty());
    }
}
