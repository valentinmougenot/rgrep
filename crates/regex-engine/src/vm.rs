use std::collections::VecDeque;

use crate::nfa::{AssertKind, Nfa, State};

pub(crate) fn is_match(nfa: &Nfa, text: &str, case_insensitive: bool) -> bool {
    find(nfa, text, case_insensitive).is_some()
}

pub(crate) fn find(nfa: &Nfa, text: &str, case_insensitive: bool) -> Option<(usize, usize)> {
    let mut context = Context {
        at_start: true,
        at_end: text.is_empty(),
        at_word_boundary: is_word_boundary(text, 0),
        at_word_start: is_word_start(text, 0),
        at_word_end: is_word_end(text, 0),
    };
    let mut current = epsilon_closure(nfa, &[(nfa.start(), 0)], &context);

    for (i, c) in text.char_indices() {
        let mut collected_targets = Vec::new();
        for (state_idx, start_pos) in &current {
            match nfa.state(*state_idx) {
                State::Consuming(ranges, target)
                    if ranges.iter().any(|r| {
                        r.contains(c)
                            || (case_insensitive
                                && (r.contains(c.to_ascii_lowercase())
                                    || r.contains(c.to_ascii_uppercase())))
                    }) =>
                {
                    collected_targets.push((*target, *start_pos))
                }
                State::Match => return Some((*start_pos, i)),
                _ => {}
            }
        }

        collected_targets.push((nfa.start(), i + c.len_utf8()));

        context = Context {
            at_start: false,
            at_end: i + c.len_utf8() == text.len(),
            at_word_boundary: is_word_boundary(text, i + c.len_utf8()),
            at_word_start: is_word_start(text, i + c.len_utf8()),
            at_word_end: is_word_end(text, i + c.len_utf8()),
        };
        current = epsilon_closure(nfa, &collected_targets, &context);
    }

    current
        .iter()
        .find(|(state_idx, _)| nfa.accept() == *state_idx)
        .map(|(_, start)| (*start, text.len()))
}

fn epsilon_closure(nfa: &Nfa, states: &[(usize, usize)], context: &Context) -> Vec<(usize, usize)> {
    let mut visited = vec![false; nfa.states_count()];
    let mut result = Vec::new();

    let mut worklist = VecDeque::new();

    for &(state_idx, start_pos) in states {
        if !visited[state_idx] {
            worklist.push_back((state_idx, start_pos));
            visited[state_idx] = true;
        }
    }

    while let Some((state_idx, start_pos)) = worklist.pop_front() {
        let state = nfa.state(state_idx);

        match state {
            State::Split(values) => {
                for value in values {
                    if !visited[*value] {
                        worklist.push_back((*value, start_pos));
                        visited[*value] = true;
                    }
                }
            }
            State::Consuming(_, _) | State::Match => {
                result.push((state_idx, start_pos));
                visited[state_idx] = true;
            }
            State::Assert(AssertKind::Start, value) if context.at_start && !visited[*value] => {
                worklist.push_back((*value, start_pos));
                visited[*value] = true;
            }
            State::Assert(AssertKind::End, value) if context.at_end && !visited[*value] => {
                worklist.push_back((*value, start_pos));
                visited[*value] = true;
            }
            State::Assert(AssertKind::WordBoundary, value)
                if context.at_word_boundary && !visited[*value] =>
            {
                worklist.push_back((*value, start_pos));
                visited[*value] = true;
            }
            State::Assert(AssertKind::WordStart, value)
                if context.at_word_start && !visited[*value] =>
            {
                worklist.push_back((*value, start_pos));
                visited[*value] = true;
            }
            State::Assert(AssertKind::WordEnd, value)
                if context.at_word_end && !visited[*value] =>
            {
                worklist.push_back((*value, start_pos));
                visited[*value] = true;
            }
            State::Assert(_, _) => {}
        }
    }

    result
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_word_boundary(text: &str, pos: usize) -> bool {
    let before = text[..pos].chars().next_back().is_some_and(is_word_char);
    let after = text[pos..].chars().next().is_some_and(is_word_char);

    before != after
}

fn is_word_start(text: &str, pos: usize) -> bool {
    !text[..pos].chars().next_back().is_some_and(is_word_char)
}

fn is_word_end(text: &str, pos: usize) -> bool {
    !text[pos..].chars().next().is_some_and(is_word_char)
}

struct Context {
    at_start: bool,
    at_end: bool,
    at_word_boundary: bool,
    at_word_start: bool,
    at_word_end: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Compiler;
    use crate::parser::Parser;

    fn compile(pattern: &str) -> Nfa {
        let ast = Parser::new(pattern).parse().unwrap();
        Compiler::new().build(&ast)
    }

    #[test]
    fn matches_exact_literal() {
        assert!(is_match(&compile("abc"), "abc", false));
    }

    #[test]
    fn does_not_match_unrelated_text() {
        assert!(!is_match(&compile("abc"), "xyz", false));
    }

    #[test]
    fn matches_substring_anywhere_in_text() {
        assert!(is_match(&compile("ab"), "xxabxx", false));
    }

    #[test]
    fn unanchored_match_ending_exactly_at_end_of_text_is_detected() {
        assert!(is_match(&compile("bc"), "abc", false));
    }

    #[test]
    fn caret_rejects_match_not_at_start() {
        assert!(!is_match(&compile("^ab"), "xab", false));
    }

    #[test]
    fn caret_accepts_match_at_start() {
        assert!(is_match(&compile("^ab"), "abxx", false));
    }

    #[test]
    fn dollar_rejects_match_not_at_end() {
        assert!(!is_match(&compile("ab$"), "abxx", false));
    }

    #[test]
    fn dollar_accepts_match_at_end() {
        assert!(is_match(&compile("ab$"), "xxab", false));
    }

    #[test]
    fn both_anchors_require_exact_full_match() {
        let nfa = compile("^ab$");
        assert!(is_match(&nfa, "ab", false));
        assert!(!is_match(&nfa, "xab", false));
        assert!(!is_match(&nfa, "abx", false));
    }

    #[test]
    fn star_matches_empty_string() {
        assert!(is_match(&compile("a*"), "", false));
    }

    #[test]
    fn optional_group_matches_when_present_and_absent() {
        let nfa = compile("colou?r");
        assert!(is_match(&nfa, "color", false));
        assert!(is_match(&nfa, "colour", false));
    }

    #[test]
    fn alternation_matches_any_branch() {
        let nfa = compile("cat|dog");
        assert!(is_match(&nfa, "cat", false));
        assert!(is_match(&nfa, "dog", false));
        assert!(!is_match(&nfa, "bird", false));
    }

    #[test]
    fn multi_byte_utf8_characters_are_matched() {
        assert!(is_match(&compile("é"), "café", false));
        assert!(is_match(&compile("é$"), "café", false));
    }

    #[test]
    fn find_returns_none_when_there_is_no_match() {
        assert_eq!(find(&compile("abc"), "xyz", false), None);
    }

    #[test]
    fn find_returns_the_span_of_an_exact_literal_match() {
        assert_eq!(find(&compile("abc"), "abc", false), Some((0, 3)));
    }

    #[test]
    fn find_returns_the_span_of_an_unanchored_substring_match() {
        assert_eq!(find(&compile("ab"), "xxabxx", false), Some((2, 4)));
    }

    #[test]
    fn find_returns_the_span_of_a_match_ending_exactly_at_end_of_text() {
        assert_eq!(find(&compile("bc"), "abc", false), Some((1, 3)));
    }

    #[test]
    fn find_respects_the_caret_anchor() {
        assert_eq!(find(&compile("^ab"), "xab", false), None);
        assert_eq!(find(&compile("^ab"), "abxx", false), Some((0, 2)));
    }

    #[test]
    fn find_respects_the_dollar_anchor() {
        assert_eq!(find(&compile("ab$"), "abxx", false), None);
        assert_eq!(find(&compile("ab$"), "xxab", false), Some((2, 4)));
    }

    #[test]
    fn find_returns_a_zero_length_span_for_an_empty_match() {
        assert_eq!(find(&compile("a*"), "", false), Some((0, 0)));
    }

    #[test]
    fn find_returns_the_span_of_the_matching_alternation_branch() {
        assert_eq!(
            find(&compile("cat|dog"), "the dog ran", false),
            Some((4, 7))
        );
    }

    #[test]
    fn find_returns_byte_offsets_for_multi_byte_utf8_characters() {
        assert_eq!(find(&compile("é"), "café", false), Some((3, 5)));
    }

    #[test]
    fn find_prefers_the_leftmost_starting_match() {
        assert_eq!(find(&compile("a"), "xax a", false), Some((1, 2)));
    }

    #[test]
    fn case_insensitive_matches_uppercase_pattern_against_lowercase_text() {
        assert!(is_match(&compile("ABC"), "abc", true));
    }

    #[test]
    fn case_insensitive_matches_lowercase_pattern_against_uppercase_text() {
        assert!(is_match(&compile("abc"), "ABC", true));
    }

    #[test]
    fn case_insensitive_matches_mixed_case() {
        assert!(is_match(&compile("aBc"), "AbC", true));
    }

    #[test]
    fn case_sensitive_by_default_rejects_different_case() {
        assert!(!is_match(&compile("abc"), "ABC", false));
    }

    #[test]
    fn case_insensitive_find_returns_the_span_in_the_original_text() {
        assert_eq!(find(&compile("ab"), "xxABxx", true), Some((2, 4)));
    }

    #[test]
    fn case_insensitive_respects_char_ranges() {
        assert!(is_match(&compile("[a-z]+"), "ABC", true));
    }

    #[test]
    fn word_boundary_matches_standalone_word() {
        assert!(is_match(&compile(r"\bcat\b"), "a cat sat", false));
    }

    #[test]
    fn word_boundary_rejects_word_as_prefix_of_longer_word() {
        assert!(!is_match(&compile(r"\bcat\b"), "category", false));
    }

    #[test]
    fn word_boundary_rejects_word_as_suffix_of_longer_word() {
        assert!(!is_match(&compile(r"\bcat\b"), "bobcat", false));
    }

    #[test]
    fn word_boundary_rejects_word_in_the_middle_of_a_longer_word() {
        assert!(!is_match(&compile(r"\bcat\b"), "concatenate", false));
    }

    #[test]
    fn word_boundary_matches_at_the_very_start_and_end_of_text() {
        assert!(is_match(&compile(r"\bcat\b"), "cat", false));
    }

    #[test]
    fn word_boundary_treats_digits_and_underscore_as_word_characters() {
        assert!(!is_match(&compile(r"\bfoo\b"), "foo_bar", false));
        assert!(!is_match(&compile(r"\bfoo\b"), "foo1", false));
        assert!(is_match(&compile(r"\bfoo\b"), "foo 1", false));
    }

    #[test]
    fn word_boundary_matches_word_next_to_punctuation() {
        assert!(is_match(&compile(r"\bmain\b"), "fn main() {", false));
    }
}
