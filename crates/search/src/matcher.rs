use std::ops::Range;

use regex_engine::Regex;

pub trait Matcher {
    fn find(&self, text: &str) -> Option<Range<usize>>;
}

impl Matcher for Regex {
    fn find(&self, text: &str) -> Option<Range<usize>> {
        self.find(text).map(|m| m.start()..m.end())
    }
}

pub struct LiteralMatcher {
    pattern: String,
    case_insensitive: bool,
    whole_word: bool,
}

impl LiteralMatcher {
    pub fn new(pattern: &str, case_insensitive: bool, whole_word: bool) -> Self {
        let pattern = if case_insensitive {
            pattern.to_ascii_lowercase()
        } else {
            pattern.to_string()
        };
        Self {
            pattern,
            case_insensitive,
            whole_word,
        }
    }
}

impl Matcher for LiteralMatcher {
    fn find(&self, text: &str) -> Option<Range<usize>> {
        let text = if self.case_insensitive {
            text.to_ascii_lowercase()
        } else {
            text.to_string()
        };

        let mut offset = 0;
        loop {
            let start = text[offset..].find(&self.pattern)?;
            let end = start + self.pattern.len();

            if !self.whole_word {
                return Some(start..end);
            }

            if (offset > 0 && text.chars().nth(offset - 1).is_some_and(is_word_char))
                || text.chars().nth(end).is_some_and(is_word_char)
            {
                offset = start + 1;
            } else {
                return Some(start..end);
            }
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
