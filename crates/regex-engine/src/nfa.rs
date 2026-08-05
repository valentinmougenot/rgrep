#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Split(Vec<usize>),
    Consuming(Vec<CharRange>, usize),
    Assert(AssertKind, usize),
    Match,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssertKind {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharRange {
    start: char,
    end: char,
}

impl CharRange {
    pub fn new(start: char, end: char) -> Self {
        debug_assert!(start <= end);
        Self { start, end }
    }

    pub fn contains(&self, c: char) -> bool {
        self.start <= c && self.end >= c
    }

    pub(crate) fn merge(ranges: &[CharRange]) -> Vec<CharRange> {
        let mut ranges: Vec<_> = ranges.to_vec();
        ranges.sort_by_key(|c| c.start);

        let mut merged_ranges = Vec::new();
        for range in ranges {
            if merged_ranges.is_empty() {
                merged_ranges.push(range);
                continue;
            }

            let last_elt = merged_ranges.last_mut().unwrap();
            if range.start as i32 - last_elt.end as i32 > 1 {
                merged_ranges.push(range);
            } else if range.end > last_elt.end {
                last_elt.end = range.end;
            }
        }

        merged_ranges
    }

    pub(crate) fn complement(ranges: &[CharRange]) -> Vec<CharRange> {
        let ranges = Self::merge(ranges);

        if ranges.is_empty() {
            return vec![Self {
                start: char::MIN,
                end: char::MAX,
            }];
        }

        let mut result = Vec::new();
        let start = ranges[0].start;
        if start > char::MIN {
            result.push(Self {
                start: char::MIN,
                end: Self::char_before(start).unwrap(),
            });
        }

        let mut windows = ranges.windows(2);

        while let Some(&[r1, r2]) = windows.next() {
            result.push(Self {
                start: Self::char_after(r1.end).unwrap(),
                end: Self::char_before(r2.start).unwrap(),
            });
        }

        let end = ranges[ranges.len() - 1].end;
        if end < char::MAX {
            result.push(Self {
                start: Self::char_after(end).unwrap(),
                end: char::MAX,
            });
        }

        result
    }

    fn char_after(c: char) -> Option<char> {
        if c >= char::MAX {
            return None;
        }

        let n = c as u32 + 1;
        if n == 0xD800 {
            char::from_u32(0xE000)
        } else {
            char::from_u32(n)
        }
    }

    fn char_before(c: char) -> Option<char> {
        if c <= char::MIN {
            return None;
        }

        let n = c as u32 - 1;
        if n == 0xDFFF {
            char::from_u32(0xD7FF)
        } else {
            char::from_u32(n)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nfa {
    start: usize,
    accept: usize,
    states: Vec<State>,
}

impl Nfa {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn accept(&self) -> usize {
        self.accept
    }

    pub fn state(&self, index: usize) -> &State {
        &self.states[index]
    }

    pub fn states_count(&self) -> usize {
        self.states.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfaBuilder {
    states: Vec<State>,
}

impl NfaBuilder {
    pub fn new() -> Self {
        Self { states: Vec::new() }
    }

    pub fn push_state(&mut self, state: State) -> usize {
        let pos = self.states.len();
        self.states.push(state);
        pos
    }

    pub fn patch_state(&mut self, pos: usize, state: State) {
        self.states[pos] = state;
    }

    pub fn build(self, start: usize, accept: usize) -> Nfa {
        Nfa {
            states: self.states,
            start,
            accept,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Ast, ClassItem, RepetitionKind};
    use crate::compiler::Compiler;

    #[test]
    fn contains_within_range() {
        let range = CharRange::new('a', 'z');
        assert!(range.contains('a'));
        assert!(range.contains('m'));
        assert!(range.contains('z'));
    }

    #[test]
    fn contains_outside_range() {
        let range = CharRange::new('a', 'z');
        assert!(!range.contains('A'));
        assert!(!range.contains('0'));
    }

    #[test]
    fn merge_empty_is_empty() {
        assert_eq!(CharRange::merge(&[]), vec![]);
    }

    #[test]
    fn merge_single_range_is_unchanged() {
        let ranges = vec![CharRange::new('a', 'z')];
        assert_eq!(CharRange::merge(&ranges), ranges);
    }

    #[test]
    fn merge_overlapping_ranges() {
        let ranges = vec![CharRange::new('a', 'm'), CharRange::new('g', 'z')];
        assert_eq!(CharRange::merge(&ranges), vec![CharRange::new('a', 'z')]);
    }

    #[test]
    fn merge_adjacent_ranges() {
        let ranges = vec![CharRange::new('a', 'm'), CharRange::new('n', 'z')];
        assert_eq!(CharRange::merge(&ranges), vec![CharRange::new('a', 'z')]);
    }

    #[test]
    fn merge_disjoint_ranges_stay_separate() {
        let ranges = vec![CharRange::new('a', 'c'), CharRange::new('x', 'z')];
        assert_eq!(
            CharRange::merge(&ranges),
            vec![CharRange::new('a', 'c'), CharRange::new('x', 'z')]
        );
    }

    #[test]
    fn merge_sorts_unsorted_input() {
        let ranges = vec![CharRange::new('x', 'z'), CharRange::new('a', 'c')];
        assert_eq!(
            CharRange::merge(&ranges),
            vec![CharRange::new('a', 'c'), CharRange::new('x', 'z')]
        );
    }

    #[test]
    fn merge_absorbs_contained_range() {
        let ranges = vec![CharRange::new('a', 'z'), CharRange::new('c', 'e')];
        assert_eq!(CharRange::merge(&ranges), vec![CharRange::new('a', 'z')]);
    }

    #[test]
    fn complement_of_empty_is_everything() {
        assert_eq!(
            CharRange::complement(&[]),
            vec![CharRange::new(char::MIN, char::MAX)]
        );
    }

    #[test]
    fn complement_of_everything_is_empty() {
        let ranges = vec![CharRange::new(char::MIN, char::MAX)];
        assert_eq!(CharRange::complement(&ranges), vec![]);
    }

    #[test]
    fn complement_of_single_char_in_the_middle() {
        let ranges = vec![CharRange::new('\n', '\n')];
        assert_eq!(
            CharRange::complement(&ranges),
            vec![
                CharRange::new(char::MIN, '\t'),
                CharRange::new('\u{B}', char::MAX),
            ]
        );
    }

    #[test]
    fn complement_with_no_leading_gap() {
        let ranges = vec![CharRange::new(char::MIN, 'm')];
        assert_eq!(
            CharRange::complement(&ranges),
            vec![CharRange::new('n', char::MAX)]
        );
    }

    #[test]
    fn complement_with_no_trailing_gap() {
        let ranges = vec![CharRange::new('m', char::MAX)];
        assert_eq!(
            CharRange::complement(&ranges),
            vec![CharRange::new(char::MIN, 'l')]
        );
    }

    #[test]
    fn complement_skips_surrogate_hole_going_up() {
        let last_before_hole = char::from_u32(0xD7FF).unwrap();
        let first_after_hole = char::from_u32(0xE000).unwrap();
        let ranges = vec![CharRange::new(char::MIN, last_before_hole)];
        assert_eq!(
            CharRange::complement(&ranges),
            vec![CharRange::new(first_after_hole, char::MAX)]
        );
    }

    #[test]
    fn complement_skips_surrogate_hole_going_down() {
        let last_before_hole = char::from_u32(0xD7FF).unwrap();
        let first_after_hole = char::from_u32(0xE000).unwrap();
        let ranges = vec![CharRange::new(first_after_hole, char::MAX)];
        assert_eq!(
            CharRange::complement(&ranges),
            vec![CharRange::new(char::MIN, last_before_hole)]
        );
    }

    #[test]
    fn char_before_min_is_none() {
        assert_eq!(CharRange::char_before(char::MIN), None);
    }

    #[test]
    fn char_after_max_is_none() {
        assert_eq!(CharRange::char_after(char::MAX), None);
    }

    #[test]
    fn compile_single_literal() {
        let nfa = Compiler::new().build(&Ast::Literal('a'));
        assert_eq!(nfa.states.len(), 2);
        assert_eq!(
            nfa.states[nfa.start],
            State::Consuming(vec![CharRange::new('a', 'a')], nfa.accept)
        );
        assert_eq!(nfa.states[nfa.accept], State::Match);
    }

    #[test]
    fn compile_concat_chains_literals() {
        let ast = Ast::Concat(vec![Ast::Literal('a'), Ast::Literal('b')]);
        let nfa = Compiler::new().build(&ast);
        assert_eq!(nfa.states.len(), 4);
        assert_eq!(nfa.states[nfa.accept], State::Match);

        let State::Consuming(ranges_a, next) = &nfa.states[nfa.start] else {
            panic!("expected Consuming state at start");
        };
        assert_eq!(*ranges_a, vec![CharRange::new('a', 'a')]);

        let State::Split(targets) = &nfa.states[*next] else {
            panic!("expected Split state chaining to the second literal");
        };
        assert_eq!(targets.len(), 1);

        let State::Consuming(ranges_b, final_exit) = &nfa.states[targets[0]] else {
            panic!("expected Consuming state for second literal");
        };
        assert_eq!(*ranges_b, vec![CharRange::new('b', 'b')]);
        assert_eq!(*final_exit, nfa.accept);
    }

    #[test]
    fn compile_alternation_converges_to_shared_exit() {
        let ast = Ast::Alternation(vec![Ast::Literal('a'), Ast::Literal('b')]);
        let nfa = Compiler::new().build(&ast);

        let State::Split(branches) = &nfa.states[nfa.start] else {
            panic!("expected Split state at start");
        };
        assert_eq!(branches.len(), 2);

        for &branch_entry in branches {
            let State::Consuming(_, branch_exit) = &nfa.states[branch_entry] else {
                panic!("expected Consuming state for each branch");
            };
            let State::Split(targets) = &nfa.states[*branch_exit] else {
                panic!("expected Split state converging each branch");
            };
            assert_eq!(targets, &vec![nfa.accept]);
        }
    }

    #[test]
    fn compile_zero_or_more_loops_back_to_inner_entry() {
        let ast = Ast::Repetition {
            kind: RepetitionKind::ZeroOrMore,
            inner: Box::new(Ast::Literal('a')),
        };
        let nfa = Compiler::new().build(&ast);

        let State::Split(start_targets) = &nfa.states[nfa.start] else {
            panic!("expected Split state at start (bypass + inner)");
        };
        assert!(start_targets.contains(&nfa.accept));

        let inner_entry = *start_targets.iter().find(|&&s| s != nfa.accept).unwrap();
        let State::Consuming(_, inner_exit) = &nfa.states[inner_entry] else {
            panic!("expected Consuming state for inner");
        };

        let State::Split(loop_targets) = &nfa.states[*inner_exit] else {
            panic!("expected Split state after inner (loop back + exit)");
        };
        assert!(loop_targets.contains(&inner_entry));
        assert!(loop_targets.contains(&nfa.accept));
    }

    #[test]
    fn compile_negative_char_class_uses_complement() {
        let ast = Ast::CharClass {
            negative: true,
            items: vec![ClassItem::Char('a')],
        };
        let nfa = Compiler::new().build(&ast);

        let State::Consuming(ranges, _) = &nfa.states[nfa.start] else {
            panic!("expected Consuming state at start");
        };
        assert_eq!(*ranges, CharRange::complement(&[CharRange::new('a', 'a')]));
    }

    #[test]
    fn compile_start_anchor_produces_assert_state() {
        let nfa = Compiler::new().build(&Ast::StartAnchor);
        assert_eq!(
            nfa.states[nfa.start],
            State::Assert(AssertKind::Start, nfa.accept)
        );
    }

    #[test]
    fn compile_end_anchor_produces_assert_state() {
        let nfa = Compiler::new().build(&Ast::EndAnchor);
        assert_eq!(
            nfa.states[nfa.start],
            State::Assert(AssertKind::End, nfa.accept)
        );
    }
}
