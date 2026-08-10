use std::io::BufRead;

use regex_engine::Regex;

use crate::line_match::LineMatch;

pub fn search<'r, R: BufRead + 'r>(
    regex: &'r Regex,
    reader: R,
    invert_match: bool,
) -> impl Iterator<Item = LineMatch> + 'r {
    Matches {
        regex,
        line_number: 0,
        reader,
        buf: String::new(),
        invert_match,
    }
}

pub struct Matches<'r, R: BufRead> {
    regex: &'r Regex,
    line_number: usize,
    reader: R,
    buf: String,
    invert_match: bool,
}

impl<'r, R: BufRead> Iterator for Matches<'r, R> {
    type Item = LineMatch;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.line_number += 1;
            self.buf.clear();

            let bytes_read = self.reader.read_line(&mut self.buf).ok()?;
            if bytes_read == 0 {
                return None;
            }

            let maybe_line_match = self.regex.find(&self.buf);
            if let Some(ref line_match) = maybe_line_match
                && !self.invert_match
            {
                return Some(LineMatch {
                    line_number: self.line_number,
                    line: self.buf.clone(),
                    match_span: Some(line_match.start()..line_match.end()),
                });
            } else if maybe_line_match.is_none() && self.invert_match {
                return Some(LineMatch {
                    line_number: self.line_number,
                    line: self.buf.clone(),
                    match_span: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex_engine::Regex;

    #[test]
    fn returns_no_matches_for_empty_input() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "".as_bytes(), false).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn returns_no_matches_when_pattern_is_absent() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxx\nyyy\nzzz".as_bytes(), false).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn line_numbers_are_one_indexed() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab".as_bytes(), false).collect();
        assert_eq!(matches[0].line_number, 1);
    }

    #[test]
    fn finds_matches_on_multiple_lines() {
        let re = Regex::new("ab").unwrap();
        let input = "xx\nab\nxxabxx\nno match\nab";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false).collect();

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line_number, 2);
        assert_eq!(matches[1].line_number, 3);
        assert_eq!(matches[2].line_number, 5);
    }

    #[test]
    fn reports_the_start_and_end_of_the_match_within_the_line() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxabxx".as_bytes(), false).collect();

        assert_eq!(matches[0].match_span, Some(2..4));
    }

    #[test]
    fn line_includes_the_trailing_newline_except_on_the_last_line() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab\nab".as_bytes(), false).collect();

        assert_eq!(matches[0].line, "ab\n");
        assert_eq!(matches[1].line, "ab");
    }

    #[test]
    fn works_with_a_single_line_and_no_trailing_newline() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxabxx".as_bytes(), false).collect();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn invert_match_excludes_lines_that_match() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab\nxxabxx".as_bytes(), true).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn invert_match_returns_lines_that_do_not_match() {
        let re = Regex::new("ab").unwrap();
        let input = "ab\nno match\nxxabxx\nstill no match";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), true).collect();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 2);
        assert_eq!(matches[0].line, "no match\n");
        assert_eq!(matches[1].line_number, 4);
        assert_eq!(matches[1].line, "still no match");
    }

    #[test]
    fn invert_match_span_is_none() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "no match".as_bytes(), true).collect();
        assert_eq!(matches[0].match_span, None);
    }
}
