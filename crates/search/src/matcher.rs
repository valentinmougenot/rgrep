use std::ops::Range;

use regex_engine::Regex;

pub trait Matcher: Send + Sync {
    fn find(&self, text: &str) -> Option<Range<usize>>;
    fn find_all(&self, text: &str) -> Vec<Range<usize>> {
        let mut result = Vec::new();
        let mut offset = 0;

        while offset <= text.len() {
            let Some(relative) = self.find(&text[offset..]) else {
                break;
            };
            let start = offset + relative.start;
            let end = offset + relative.end;

            result.push(start..end);
            offset = if end > start { end } else { end + 1 };
        }

        result
    }
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

pub struct MultiMatcher {
    matchers: Vec<Box<dyn Matcher>>,
}

impl MultiMatcher {
    pub fn new(matchers: Vec<Box<dyn Matcher>>) -> Self {
        Self { matchers }
    }
}

impl Matcher for MultiMatcher {
    fn find(&self, text: &str) -> Option<Range<usize>> {
        self.matchers
            .iter()
            .filter_map(|m| m.find(text))
            .min_by_key(|range| range.start)
    }
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

    #[test]
    fn multi_matcher_returns_none_when_no_matcher_matches() {
        let m = MultiMatcher::new(vec![
            Box::new(LiteralMatcher::new("foo", false, false)),
            Box::new(LiteralMatcher::new("bar", false, false)),
        ]);
        assert_eq!(m.find("xyz"), None);
    }

    #[test]
    fn multi_matcher_returns_the_only_match_from_a_single_matcher() {
        let m = MultiMatcher::new(vec![Box::new(LiteralMatcher::new("ab", false, false))]);
        assert_eq!(m.find("xxabxx"), Some(2..4));
    }

    #[test]
    fn multi_matcher_returns_the_leftmost_match_across_matchers() {
        let m = MultiMatcher::new(vec![
            Box::new(LiteralMatcher::new("bar", false, false)),
            Box::new(LiteralMatcher::new("foo", false, false)),
        ]);
        assert_eq!(m.find("foo bar"), Some(0..3));
    }

    #[test]
    fn multi_matcher_works_with_mixed_matcher_kinds() {
        let regex = Regex::new("f.o").unwrap();
        let matchers: Vec<Box<dyn Matcher>> = vec![
            Box::new(regex),
            Box::new(LiteralMatcher::new("bar", false, false)),
        ];
        let m = MultiMatcher::new(matchers);

        assert_eq!(m.find("xx bar xx"), Some(3..6));
        assert_eq!(m.find("xx foo xx"), Some(3..6));
        assert_eq!(m.find("xx baz xx"), None);
    }

    #[test]
    fn find_all_returns_an_empty_vec_when_there_is_no_match() {
        let re = Regex::new("ab").unwrap();
        assert_eq!(re.find_all("xyz"), Vec::<Range<usize>>::new());
    }

    #[test]
    fn find_all_returns_every_non_overlapping_match_in_order() {
        let re = Regex::new("ab").unwrap();
        assert_eq!(re.find_all("ab xx ab ab"), vec![0..2, 6..8, 9..11]);
    }

    #[test]
    fn find_all_forces_progress_on_a_zero_length_match() {
        let re = Regex::new("a*").unwrap();
        // "a*" accepts the empty string immediately at every position, so
        // every match here is zero-length; each one must still advance the
        // offset by at least one byte or this loops forever.
        assert_eq!(re.find_all("ab"), vec![0..0, 1..1, 2..2]);
    }

    #[test]
    fn find_all_works_for_literal_matcher() {
        let m = LiteralMatcher::new("cat", false, true);
        assert_eq!(m.find_all("concatenate cat here cat"), vec![12..15, 21..24]);
    }

    #[test]
    fn find_all_uses_the_leftmost_match_from_each_matcher_in_multi_matcher() {
        let m = MultiMatcher::new(vec![
            Box::new(LiteralMatcher::new("foo", false, false)),
            Box::new(LiteralMatcher::new("bar", false, false)),
        ]);

        assert_eq!(m.find_all("foo xx bar xx foo"), vec![0..3, 7..10, 14..17]);
    }
}
