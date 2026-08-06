use std::io::BufRead;

use regex_engine::Regex;

use crate::line_match::LineMatch;

pub fn search<'r, R: BufRead + 'r>(
    regex: &'r Regex,
    reader: R,
) -> impl Iterator<Item = LineMatch> + 'r {
    Matches {
        regex,
        line_number: 0,
        reader,
        buf: String::new(),
    }
}

pub struct Matches<'r, R: BufRead> {
    regex: &'r Regex,
    line_number: usize,
    reader: R,
    buf: String,
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

            if let Some(line_match) = self.regex.find(&self.buf) {
                return Some(LineMatch {
                    line_number: self.line_number,
                    line: self.buf.clone(),
                    start: line_match.start(),
                    end: line_match.end(),
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
        let matches: Vec<LineMatch> = search(&re, "".as_bytes()).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn returns_no_matches_when_pattern_is_absent() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxx\nyyy\nzzz".as_bytes()).collect();
        assert!(matches.is_empty());
    }

    #[test]
    fn line_numbers_are_one_indexed() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab".as_bytes()).collect();
        assert_eq!(matches[0].line_number, 1);
    }

    #[test]
    fn finds_matches_on_multiple_lines() {
        let re = Regex::new("ab").unwrap();
        let input = "xx\nab\nxxabxx\nno match\nab";
        let matches: Vec<LineMatch> = search(&re, input.as_bytes()).collect();

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].line_number, 2);
        assert_eq!(matches[1].line_number, 3);
        assert_eq!(matches[2].line_number, 5);
    }

    #[test]
    fn reports_the_start_and_end_of_the_match_within_the_line() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxabxx".as_bytes()).collect();

        assert_eq!(matches[0].start, 2);
        assert_eq!(matches[0].end, 4);
    }

    #[test]
    fn line_includes_the_trailing_newline_except_on_the_last_line() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "ab\nab".as_bytes()).collect();

        assert_eq!(matches[0].line, "ab\n");
        assert_eq!(matches[1].line, "ab");
    }

    #[test]
    fn works_with_a_single_line_and_no_trailing_newline() {
        let re = Regex::new("ab").unwrap();
        let matches: Vec<LineMatch> = search(&re, "xxabxx".as_bytes()).collect();
        assert_eq!(matches.len(), 1);
    }
}
