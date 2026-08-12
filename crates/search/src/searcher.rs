use std::{collections::VecDeque, io::BufRead};

use regex_engine::Regex;

use crate::line_match::LineMatch;

pub fn search<'r, R: BufRead + 'r>(
    regex: &'r Regex,
    reader: R,
    invert_match: bool,
    before_context: usize,
    after_context: usize,
) -> impl Iterator<Item = LineMatch> + 'r {
    Matches {
        regex,
        line_number: 0,
        reader,
        buf: String::new(),
        invert_match,
        before_context,
        after_context,
        pending: VecDeque::new(),
        before_buffer: VecDeque::with_capacity(before_context),
        after_remaining: 0,
    }
}

pub struct Matches<'r, R: BufRead> {
    regex: &'r Regex,
    line_number: usize,
    reader: R,
    buf: String,
    invert_match: bool,
    before_context: usize,
    after_context: usize,
    pending: VecDeque<LineMatch>,
    before_buffer: VecDeque<(usize, String)>,
    after_remaining: usize,
}

impl<'r, R: BufRead> Iterator for Matches<'r, R> {
    type Item = LineMatch;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(pending_line) = self.pending.pop_front() {
                return Some(pending_line);
            }

            self.line_number += 1;

            self.buf.clear();
            let bytes_read = self.reader.read_line(&mut self.buf).ok()?;
            if bytes_read == 0 {
                return None;
            }

            let buf_clone = self.buf.clone();
            let maybe_line_match = self.regex.find(&buf_clone);

            if let Some(ref line_match) = maybe_line_match
                && !self.invert_match
            {
                let before_buffer = std::mem::take(&mut self.before_buffer);

                for (line_number, line) in before_buffer {
                    self.pending.push_back(LineMatch {
                        line_number,
                        line,
                        match_span: None,
                        is_context: true,
                    });
                }

                self.pending.push_back(LineMatch {
                    line_number: self.line_number,
                    line: std::mem::take(&mut self.buf),
                    match_span: Some(line_match.start()..line_match.end()),
                    is_context: false,
                });
                self.after_remaining = self.after_context;
            } else if maybe_line_match.is_none() && self.invert_match {
                let before_buffer = std::mem::take(&mut self.before_buffer);

                for (line_number, line) in before_buffer {
                    self.pending.push_back(LineMatch {
                        line_number,
                        line,
                        match_span: None,
                        is_context: true,
                    });
                }

                self.pending.push_back(LineMatch {
                    line_number: self.line_number,
                    line: std::mem::take(&mut self.buf),
                    match_span: None,
                    is_context: false,
                });
                self.after_remaining = self.after_context;
            } else if self.after_remaining > 0 {
                self.pending.push_back(LineMatch {
                    line_number: self.line_number,
                    line: std::mem::take(&mut self.buf),
                    match_span: None,
                    is_context: true,
                });
                self.after_remaining -= 1;
            } else if self.before_context > 0 {
                if self.before_buffer.len() == self.before_context {
                    self.before_buffer.pop_front();
                }
                self.before_buffer
                    .push_back((self.line_number, std::mem::take(&mut self.buf)));
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
        let matches: Vec<LineMatch> = search(&re, "".as_bytes(), false, 0, 0).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn returns_no_matches_when_pattern_is_absent() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxx\nyyy\nzzz".as_bytes(), false, 0, 0).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn line_numbers_are_one_indexed() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab".as_bytes(), false, 0, 0).collect();
        assert_eq!(matches[0].line_number, 1);
    }

    #[test]
    fn finds_matches_on_multiple_lines() {
        let re = Regex::new("ab").unwrap();
        let input = "xx\nab\nxxabxx\nno match\nab";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 0, 0).collect();

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line_number, 2);
        assert_eq!(matches[1].line_number, 3);
        assert_eq!(matches[2].line_number, 5);
    }

    #[test]
    fn reports_the_start_and_end_of_the_match_within_the_line() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxabxx".as_bytes(), false, 0, 0).collect();

        assert_eq!(matches[0].match_span, Some(2..4));
    }

    #[test]
    fn line_includes_the_trailing_newline_except_on_the_last_line() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab\nab".as_bytes(), false, 0, 0).collect();

        assert_eq!(matches[0].line, "ab\n");
        assert_eq!(matches[1].line, "ab");
    }

    #[test]
    fn works_with_a_single_line_and_no_trailing_newline() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxabxx".as_bytes(), false, 0, 0).collect();
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn invert_match_excludes_lines_that_match() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab\nxxabxx".as_bytes(), true, 0, 0).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn invert_match_returns_lines_that_do_not_match() {
        let re = Regex::new("ab").unwrap();
        let input = "ab\nno match\nxxabxx\nstill no match";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), true, 0, 0).collect();

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_number, 2);
        assert_eq!(matches[0].line, "no match\n");
        assert_eq!(matches[1].line_number, 4);
        assert_eq!(matches[1].line, "still no match");
    }

    #[test]
    fn invert_match_span_is_none() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "no match".as_bytes(), true, 0, 0).collect();
        assert_eq!(matches[0].match_span, None);
    }
}
