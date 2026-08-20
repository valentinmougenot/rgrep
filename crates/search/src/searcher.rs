use std::{collections::VecDeque, io::BufRead};

use crate::{Matcher, line_match::LineMatch};

pub fn search<R: BufRead>(
    matcher: &dyn Matcher,
    reader: R,
    invert_match: bool,
    before_context: usize,
    after_context: usize,
) -> impl Iterator<Item = LineMatch> {
    Matches {
        matcher,
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

pub struct Matches<'m, R: BufRead> {
    matcher: &'m dyn Matcher,
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

impl<'m, R: BufRead> Iterator for Matches<'m, R> {
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

            let match_range = self.matcher.find(&self.buf);
            let is_hit = match_range.is_some() != self.invert_match;

            if is_hit {
                let match_span = if self.invert_match { None } else { match_range };
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
                    match_span,
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
        let matches: Vec<LineMatch> =
            search(&re, "xxx\nyyy\nzzz".as_bytes(), false, 0, 0).collect();
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

    #[test]
    fn after_context_includes_the_lines_following_a_match() {
        let re = Regex::new("ab").unwrap();
        let input = "xx\nab\nc\nd\ne\nf";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 0, 2).collect();

        let line_numbers: Vec<usize> = matches.iter().map(|m| m.line_number).collect();
        assert_eq!(line_numbers, vec![2, 3, 4]);
        assert!(!matches[0].is_context);
        assert!(matches[1].is_context);
        assert!(matches[2].is_context);
    }

    #[test]
    fn after_context_is_truncated_at_the_end_of_input() {
        let re = Regex::new("ab").unwrap();
        let input = "x\nab\ny";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 0, 5).collect();

        let line_numbers: Vec<usize> = matches.iter().map(|m| m.line_number).collect();
        assert_eq!(line_numbers, vec![2, 3]);
    }

    #[test]
    fn before_context_includes_the_lines_preceding_a_match() {
        let re = Regex::new("ab").unwrap();
        let input = "w\nx\ny\nab\nz";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 2, 0).collect();

        let line_numbers: Vec<usize> = matches.iter().map(|m| m.line_number).collect();
        assert_eq!(line_numbers, vec![2, 3, 4]);
        assert!(matches[0].is_context);
        assert!(matches[1].is_context);
        assert!(!matches[2].is_context);
    }

    #[test]
    fn before_context_only_keeps_the_lines_closest_to_the_match() {
        let re = Regex::new("ab").unwrap();
        let input = "v\nw\nx\ny\nab";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 2, 0).collect();

        let line_numbers: Vec<usize> = matches.iter().map(|m| m.line_number).collect();
        assert_eq!(line_numbers, vec![3, 4, 5]);
    }

    #[test]
    fn before_and_after_context_can_be_combined() {
        let re = Regex::new("MATCH").unwrap();
        let input = "a\nb\nMATCH\nc\nd";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 1, 1).collect();

        let line_numbers: Vec<usize> = matches.iter().map(|m| m.line_number).collect();
        assert_eq!(line_numbers, vec![2, 3, 4]);
    }

    #[test]
    fn a_line_that_is_both_after_context_and_the_next_match_is_reported_once_as_a_match() {
        let re = Regex::new("ab").unwrap();
        let input = "ab\nab";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 0, 5).collect();

        assert_eq!(matches.len(), 2);
        assert!(!matches[0].is_context);
        assert!(!matches[1].is_context);
    }

    #[test]
    fn distinct_matches_with_context_leave_a_gap_in_line_numbers_when_far_apart() {
        let re = Regex::new("MATCH").unwrap();
        let input = "MATCH\nx\ny\nz\nw\nv\nu\nt\ns\nr\nMATCH";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes(), false, 1, 1).collect();

        let line_numbers: Vec<usize> = matches.iter().map(|m| m.line_number).collect();
        assert_eq!(line_numbers, vec![1, 2, 10, 11]);
    }
}
