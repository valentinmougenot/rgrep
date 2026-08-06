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
