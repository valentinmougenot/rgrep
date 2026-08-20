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
            let start = text[offset..].find(&self.pattern)? + offset;
            let end = start + self.pattern.len();

            if !self.whole_word {
                return Some(start..end);
            }

            if (start > 0 && text.chars().nth(start - 1).is_some_and(is_word_char))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exact_literal_match() {
        let m = LiteralMatcher::new("abc", false, false);
        assert_eq!(m.find("abc"), Some(0..3));
    }

    #[test]
    fn finds_substring_anywhere_in_text() {
        let m = LiteralMatcher::new("ab", false, false);
        assert_eq!(m.find("xxabxx"), Some(2..4));
    }

    #[test]
    fn returns_none_when_pattern_is_absent() {
        let m = LiteralMatcher::new("ab", false, false);
        assert_eq!(m.find("xyz"), None);
    }

    #[test]
    fn treats_regex_metacharacters_as_plain_text() {
        let m = LiteralMatcher::new("a.c", false, false);
        assert_eq!(m.find("abc"), None);
        assert_eq!(m.find("xxa.cxx"), Some(2..5));
    }

    #[test]
    fn case_sensitive_by_default_rejects_different_case() {
        let m = LiteralMatcher::new("abc", false, false);
        assert_eq!(m.find("ABC"), None);
    }

    #[test]
    fn case_insensitive_matches_different_case() {
        let m = LiteralMatcher::new("abc", true, false);
        assert_eq!(m.find("xxABCxx"), Some(2..5));
    }

    #[test]
    fn whole_word_false_matches_a_substring_inside_a_longer_word() {
        let m = LiteralMatcher::new("cat", false, false);
        assert_eq!(m.find("concatenate"), Some(3..6));
    }

    #[test]
    fn whole_word_true_matches_a_standalone_word() {
        let m = LiteralMatcher::new("cat", false, true);
        assert_eq!(m.find("a cat sat"), Some(2..5));
    }

    #[test]
    fn whole_word_true_rejects_a_word_inside_a_longer_word() {
        let m = LiteralMatcher::new("cat", false, true);
        assert_eq!(m.find("concatenate"), None);
    }

    #[test]
    fn whole_word_true_skips_an_embedded_occurrence_and_finds_a_later_standalone_one() {
        let m = LiteralMatcher::new("cat", false, true);
        assert_eq!(m.find("concatenate cat here"), Some(12..15));
    }

    #[test]
    fn whole_word_true_matches_at_the_very_start_and_end_of_text() {
        let m = LiteralMatcher::new("cat", false, true);
        assert_eq!(m.find("cat"), Some(0..3));
    }

    #[test]
    fn whole_word_combines_with_case_insensitive() {
        let m = LiteralMatcher::new("cat", true, true);
        assert_eq!(m.find("a CAT sat"), Some(2..5));
        assert_eq!(m.find("concatenate"), None);
    }
}
