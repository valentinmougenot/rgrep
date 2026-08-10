use std::{io, iter::Peekable, path::Path};

use search::LineMatch;

use crate::colorizer::Colorizer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Matches,
    Count,
    Files,
}

pub struct Output<W: io::Write> {
    mode: OutputMode,
    colorizer: Colorizer,
    writer: W,
    show_header: bool,
    separator_needed: bool,
}

impl<W: io::Write> Output<W> {
    pub fn new(mode: OutputMode, writer: W, show_header: bool) -> Self {
        Self {
            mode,
            colorizer: Colorizer::from_stdout(),
            writer,
            show_header,
            separator_needed: false,
        }
    }

    pub fn report(
        &mut self,
        matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
        path: Option<&Path>,
    ) {
        match self.mode {
            OutputMode::Matches => self.report_matches(matches, path),
            OutputMode::Count => self.report_count(matches, path),
            OutputMode::Files => self.report_files(matches, path),
        }
    }

    fn report_matches(
        &mut self,
        matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
        path: Option<&Path>,
    ) {
        if matches.peek().is_some()
            && self.show_header
            && let Some(path) = path
        {
            if self.separator_needed {
                writeln!(self.writer).unwrap();
            } else {
                self.separator_needed = true;
            }

            writeln!(
                self.writer,
                "{}",
                self.colorizer.path(&path.display().to_string())
            )
            .unwrap();
        }

        for line_match in matches {
            if let Some(std::ops::Range { start, end }) = line_match.match_span {
                write!(
                    self.writer,
                    "{}:",
                    self.colorizer.line_number(line_match.line_number)
                )
                .unwrap();
                let line = line_match.line.trim_end();
                write!(self.writer, "{}", &line[..start]).unwrap();
                write!(self.writer, "{}", self.colorizer.matched(&line[start..end])).unwrap();
                writeln!(self.writer, "{}", &line[end..]).unwrap();
            } else {
                writeln!(
                    self.writer,
                    "{}:{}",
                    self.colorizer.line_number(line_match.line_number),
                    line_match.line.trim_end()
                )
                .unwrap();
            }
        }
    }

    fn report_count(
        &mut self,
        matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
        path: Option<&Path>,
    ) {
        if matches.peek().is_some() {
            if let Some(path) = path {
                write!(
                    self.writer,
                    "{}:",
                    self.colorizer.path(&path.display().to_string())
                )
                .unwrap();
            }
            writeln!(self.writer, "{}", matches.count()).unwrap();
        }
    }

    fn report_files(
        &mut self,
        matches: &mut Peekable<impl Iterator<Item = LineMatch>>,
        path: Option<&Path>,
    ) {
        if matches.peek().is_some() {
            let label = path
                .map(|p| p.display().to_string())
                .unwrap_or("<stdin>".to_string());
            writeln!(self.writer, "{}", self.colorizer.path(&label)).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_match(line_number: usize, line: &str, start: usize, end: usize) -> LineMatch {
        LineMatch {
            line_number,
            line: line.to_string(),
            match_span: Some(start..end),
        }
    }

    fn inverted_line_match(line_number: usize, line: &str) -> LineMatch {
        LineMatch {
            line_number,
            line: line.to_string(),
            match_span: None,
        }
    }

    fn report_to_string(
        mode: OutputMode,
        matches: Vec<LineMatch>,
        path: Option<&Path>,
        show_header: bool,
    ) -> String {
        let mut output = Output::new(mode, Vec::new(), show_header);
        output.report(&mut matches.into_iter().peekable(), path);
        String::from_utf8(output.writer).unwrap()
    }

    #[test]
    fn matches_mode_prints_each_line_with_number() {
        let matches = vec![
            line_match(1, "hello world", 0, 5),
            line_match(3, "hello again", 0, 5),
        ];

        let output = report_to_string(OutputMode::Matches, matches, None, false);

        assert_eq!(output, "1:hello world\n3:hello again\n");
    }

    #[test]
    fn matches_mode_prints_nothing_when_there_are_no_matches() {
        let output = report_to_string(OutputMode::Matches, vec![], None, false);
        assert!(output.is_empty());
    }

    #[test]
    fn matches_mode_prints_the_header_when_a_path_is_given_and_there_are_matches() {
        let matches = vec![line_match(1, "hello", 0, 5)];
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Matches, matches, Some(path), true);

        assert_eq!(output, "a.txt\n1:hello\n");
    }

    #[test]
    fn matches_mode_omits_the_header_when_there_are_no_matches() {
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Matches, vec![], Some(path), true);

        assert!(output.is_empty());
    }

    #[test]
    fn matches_mode_does_not_print_a_header_when_show_header_is_false() {
        let matches = vec![line_match(1, "hello", 0, 5)];
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Matches, matches, Some(path), false);

        assert_eq!(output, "1:hello\n");
    }

    #[test]
    fn matches_mode_prints_the_whole_line_without_highlight_when_match_span_is_none() {
        let matches = vec![inverted_line_match(1, "no match here")];

        let output = report_to_string(OutputMode::Matches, matches, None, false);

        assert_eq!(output, "1:no match here\n");
    }

    #[test]
    fn matches_mode_separates_successive_headers_with_a_blank_line() {
        let mut output = Output::new(OutputMode::Matches, Vec::new(), true);
        let path = Path::new("a.txt");

        output.report(
            &mut vec![line_match(1, "hello", 0, 5)].into_iter().peekable(),
            Some(path),
        );
        output.report(
            &mut vec![line_match(1, "hello", 0, 5)].into_iter().peekable(),
            Some(path),
        );

        assert_eq!(
            String::from_utf8(output.writer).unwrap(),
            "a.txt\n1:hello\n\na.txt\n1:hello\n"
        );
    }

    #[test]
    fn count_mode_prints_the_total_with_the_path() {
        let matches = vec![
            line_match(1, "hello", 0, 5),
            line_match(2, "hello", 0, 5),
            line_match(4, "hello", 0, 5),
        ];
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Count, matches, Some(path), false);

        assert_eq!(output, "a.txt:3\n");
    }

    #[test]
    fn count_mode_prints_just_the_total_without_a_path() {
        let matches = vec![line_match(1, "hello", 0, 5), line_match(2, "hello", 0, 5)];

        let output = report_to_string(OutputMode::Count, matches, None, false);

        assert_eq!(output, "2\n");
    }

    #[test]
    fn count_mode_prints_nothing_when_there_are_no_matches() {
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Count, vec![], Some(path), false);

        assert!(output.is_empty());
    }

    #[test]
    fn files_mode_prints_the_path_when_there_are_matches() {
        let matches = vec![line_match(1, "hello", 0, 5), line_match(2, "hello", 0, 5)];
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Files, matches, Some(path), false);

        assert_eq!(output, "a.txt\n");
    }

    #[test]
    fn files_mode_prints_nothing_when_there_are_no_matches() {
        let path = Path::new("a.txt");

        let output = report_to_string(OutputMode::Files, vec![], Some(path), false);

        assert!(output.is_empty());
    }

    #[test]
    fn files_mode_prints_stdin_placeholder_when_there_is_no_path() {
        let matches = vec![line_match(1, "hello", 0, 5)];

        let output = report_to_string(OutputMode::Files, matches, None, false);

        assert_eq!(output, "<stdin>\n");
    }
}
