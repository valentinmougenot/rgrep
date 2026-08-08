use std::collections::VecDeque;

use crate::nfa::{AssertKind, Nfa, State};

pub(crate) fn is_match(nfa: &Nfa, text: &str, case_insensitive: bool) -> bool {
    find(nfa, text, case_insensitive).is_some()
}

pub(crate) fn find(nfa: &Nfa, text: &str, case_insensitive: bool) -> Option<(usize, usize)> {
    let mut current = epsilon_closure(nfa, &[(nfa.start(), 0)], true, text.is_empty());

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
        current = epsilon_closure(
            nfa,
            &collected_targets,
            false,
            i + c.len_utf8() == text.len(),
        );
    }

    current
        .iter()
        .find(|(state_idx, _)| nfa.accept() == *state_idx)
        .map(|(_, start)| (*start, text.len()))
}

fn epsilon_closure(
    nfa: &Nfa,
    states: &[(usize, usize)],
    at_start: bool,
    at_end: bool,
) -> Vec<(usize, usize)> {
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
            State::Assert(AssertKind::Start, value) if at_start && !visited[*value] => {
                worklist.push_back((*value, start_pos));
                visited[*value] = true;
            }
            State::Assert(AssertKind::End, value) if at_end && !visited[*value] => {
                worklist.push_back((*value, start_pos));
                visited[*value] = true;
            }
            State::Assert(_, _) => {}
        }
    }

    result
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
        assert_eq!(find(&compile("cat|dog"), "the dog ran", false), Some((4, 7)));
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
}
