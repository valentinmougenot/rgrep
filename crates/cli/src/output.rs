use std::{io, iter::Peekable, ops::Range, path::Path};

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
    show_separator: bool,
    matched: bool,
    only_matching: bool,
}

impl<W: io::Write> Output<W> {
    pub fn new(
        mode: OutputMode,
        writer: W,
        show_header: bool,
        show_separator: bool,
        colorizer: Colorizer,
        only_matching: bool,
    ) -> Self {
        Self {
            mode,
            colorizer,
            writer,
            show_header,
            separator_needed: false,
            show_separator,
            matched: false,
            only_matching,
        }
    }

    pub fn matched(&self) -> bool {
        self.matched
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
        if matches.peek().is_some() {
            self.matched = true;

            if self.show_header
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
        }

        let mut last_line_number = None;

        for line_match in matches {
            if self.show_separator
                && let Some(last) = last_line_number
                && line_match.line_number > last + 1
            {
                writeln!(self.writer, "--").unwrap();
            }
            last_line_number = Some(line_match.line_number);

            let line = line_match.line.trim_end();

            if line_match.match_spans.is_empty() {
                write!(
                    self.writer,
                    "{}:",
                    self.colorizer.line_number(line_match.line_number)
                )
                .unwrap();
                writeln!(self.writer, "{}", line).unwrap();
                continue;
            }

            if !self.only_matching {
                write!(
                    self.writer,
                    "{}:",
                    self.colorizer.line_number(line_match.line_number)
                )
                .unwrap();
            }

            let mut last_end = 0;

            for Range { start, end } in &line_match.match_spans {
                if self.only_matching {
                    write!(
                        self.writer,
                        "{}:",
                        self.colorizer.line_number(line_match.line_number)
                    )
                    .unwrap();
                    writeln!(
                        self.writer,
                        "{}",
                        self.colorizer.matched(&line[*start..*end])
                    )
                    .unwrap();
                    continue;
                }

                if last_end < *start && !self.only_matching {
                    write!(self.writer, "{}", &line[last_end..*start]).unwrap();
                }
                write!(
                    self.writer,
                    "{}",
                    self.colorizer.matched(&line[*start..*end])
                )
                .unwrap();
                last_end = *end;
            }

            if last_end < line.len() && !self.only_matching {
                writeln!(self.writer, "{}", &line[last_end..]).unwrap();
            } else if !self.only_matching {
                writeln!(self.writer).unwrap();
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
            self.matched = true;
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
            self.matched = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_match(line_number: usize, line: &str, start: usize, end: usize) -> LineMatch {
        #[allow(clippy::single_range_in_vec_init)]
        LineMatch {
            line_number,
            line: line.to_string(),
            match_spans: vec![start..end],
            is_context: false,
        }
    }

    fn inverted_line_match(line_number: usize, line: &str) -> LineMatch {
        LineMatch {
            line_number,
            line: line.to_string(),
            match_spans: Vec::new(),
            is_context: false,
        }
    }

    fn report_to_string(
        mode: OutputMode,
        matches: Vec<LineMatch>,
        path: Option<&Path>,
        show_header: bool,
    ) -> String {
        let mut output = Output::new(
            mode,
            Vec::new(),
            show_header,
            false,
            Colorizer::new(false),
            false,
        );
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
    fn matches_mode_prints_only_the_matched_substring_when_only_matching_is_enabled() {
        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            true,
        );

        output.report(
            &mut vec![line_match(1, "hello world", 0, 5)]
                .into_iter()
                .peekable(),
            None,
        );

        assert_eq!(String::from_utf8(output.writer).unwrap(), "1:hello\n");
    }

    #[test]
    fn matches_mode_prints_the_whole_line_when_only_matching_is_disabled() {
        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            false,
        );

        output.report(
            &mut vec![line_match(1, "hello world", 0, 5)]
                .into_iter()
                .peekable(),
            None,
        );

        assert_eq!(String::from_utf8(output.writer).unwrap(), "1:hello world\n");
    }

    #[test]
    fn matches_mode_prints_the_whole_line_when_only_matching_is_enabled_but_match_span_is_none() {
        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            true,
        );

        output.report(
            &mut vec![inverted_line_match(1, "no match here")]
                .into_iter()
                .peekable(),
            None,
        );

        assert_eq!(
            String::from_utf8(output.writer).unwrap(),
            "1:no match here\n"
        );
    }

    #[test]
    fn matches_mode_reports_matched_even_when_show_header_is_false() {
        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            false,
        );

        output.report(
            &mut vec![line_match(1, "hello", 0, 5)].into_iter().peekable(),
            None,
        );

        assert!(output.matched());
    }

    #[test]
    fn matches_mode_reports_not_matched_when_there_are_no_matches() {
        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            true,
            false,
            Colorizer::new(false),
            false,
        );

        output.report(
            &mut Vec::new().into_iter().peekable(),
            Some(Path::new("a.txt")),
        );

        assert!(!output.matched());
    }

    #[test]
    fn matches_mode_prints_the_whole_line_without_highlight_when_match_span_is_none() {
        let matches = vec![inverted_line_match(1, "no match here")];

        let output = report_to_string(OutputMode::Matches, matches, None, false);

        assert_eq!(output, "1:no match here\n");
    }

    #[test]
    fn matches_mode_separates_successive_headers_with_a_blank_line() {
        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            true,
            false,
            Colorizer::new(false),
            false,
        );
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
    fn matches_mode_inserts_a_separator_between_non_adjacent_line_numbers_when_enabled() {
        let matches = vec![
            line_match(2, "b", 0, 1),
            line_match(3, "MATCH1", 0, 5),
            line_match(10, "i", 0, 1),
            line_match(11, "MATCH2", 0, 5),
        ];

        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            false,
            true,
            Colorizer::new(false),
            false,
        );
        output.report(&mut matches.into_iter().peekable(), None);

        assert_eq!(
            String::from_utf8(output.writer).unwrap(),
            "2:b\n3:MATCH1\n--\n10:i\n11:MATCH2\n"
        );
    }

    #[test]
    fn matches_mode_does_not_insert_a_separator_for_adjacent_line_numbers() {
        let matches = vec![line_match(1, "a", 0, 1), line_match(2, "b", 0, 1)];

        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            false,
            true,
            Colorizer::new(false),
            false,
        );
        output.report(&mut matches.into_iter().peekable(), None);

        assert_eq!(String::from_utf8(output.writer).unwrap(), "1:a\n2:b\n");
    }

    #[test]
    fn matches_mode_does_not_insert_a_separator_when_show_separator_is_false() {
        let matches = vec![line_match(2, "b", 0, 1), line_match(10, "i", 0, 1)];

        let mut output = Output::new(
            OutputMode::Matches,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            false,
        );
        output.report(&mut matches.into_iter().peekable(), None);

        assert_eq!(String::from_utf8(output.writer).unwrap(), "2:b\n10:i\n");
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
    fn count_mode_reports_matched_when_there_are_matches() {
        let mut output = Output::new(
            OutputMode::Count,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            false,
        );

        output.report(
            &mut vec![line_match(1, "hello", 0, 5)].into_iter().peekable(),
            None,
        );

        assert!(output.matched());
    }

    #[test]
    fn count_mode_reports_not_matched_when_there_are_no_matches() {
        let mut output = Output::new(
            OutputMode::Count,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            false,
        );

        output.report(&mut Vec::new().into_iter().peekable(), None);

        assert!(!output.matched());
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
    fn files_mode_reports_matched_when_there_are_matches() {
        let mut output = Output::new(
            OutputMode::Files,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            false,
        );

        output.report(
            &mut vec![line_match(1, "hello", 0, 5)].into_iter().peekable(),
            Some(Path::new("a.txt")),
        );

        assert!(output.matched());
    }

    #[test]
    fn files_mode_reports_not_matched_when_there_are_no_matches() {
        let mut output = Output::new(
            OutputMode::Files,
            Vec::new(),
            false,
            false,
            Colorizer::new(false),
            false,
        );

        output.report(
            &mut Vec::new().into_iter().peekable(),
            Some(Path::new("a.txt")),
        );

        assert!(!output.matched());
    }

    #[test]
    fn files_mode_prints_stdin_placeholder_when_there_is_no_path() {
        let matches = vec![line_match(1, "hello", 0, 5)];

        let output = report_to_string(OutputMode::Files, matches, None, false);

        assert_eq!(output, "<stdin>\n");
    }
}
