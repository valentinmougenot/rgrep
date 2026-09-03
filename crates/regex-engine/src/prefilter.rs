use crate::ast::Ast;

pub(crate) fn required_literal(ast: &Ast) -> Option<String> {
    let mut best_run = None;
    let mut current_run: Option<String> = None;

    match ast {
        Ast::Literal(c) => return Some(c.to_string()),
        Ast::Group(inner) => return required_literal(inner),
        Ast::Concat(items) => {
            for item in items {
                if let Some(result) = exact_literal(item) {
                    if let Some(ref mut current) = current_run {
                        current.push_str(&result);
                    } else {
                        current_run = Some(result);
                    }
                } else {
                    best_run = longer(best_run, current_run.take());

                    let literal_run = required_literal(item);
                    best_run = longer(best_run, literal_run);
                }
            }
        }
        Ast::Repetition { kind: _, inner: _ }
        | Ast::CharClass {
            negative: _,
            items: _,
        }
        | Ast::Dot
        | Ast::Alternation(_) => return None,
        Ast::StartAnchor | Ast::EndAnchor | Ast::WordBoundary | Ast::WordStart | Ast::WordEnd => {
            return Some(String::new());
        }
    }

    longer(best_run, current_run)
}

fn exact_literal(ast: &Ast) -> Option<String> {
    let mut current_run = String::new();

    match ast {
        Ast::Literal(c) => return Some(c.to_string()),
        Ast::Group(inner) => return exact_literal(inner),
        Ast::Concat(items) => {
            for item in items {
                let result = exact_literal(item);

                if let Some(value) = result {
                    current_run.push_str(&value);
                } else {
                    return None;
                }
            }

            Some(current_run)
        }
        Ast::Repetition { kind: _, inner: _ }
        | Ast::CharClass {
            negative: _,
            items: _,
        }
        | Ast::Dot
        | Ast::Alternation(_) => return None,
        Ast::StartAnchor | Ast::EndAnchor | Ast::WordBoundary | Ast::WordStart | Ast::WordEnd => {
            return Some(String::new());
        }
    }
}

fn longer(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) if a.len() >= b.len() => Some(a),
        (Some(_), Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    fn literal(pattern: &str) -> Option<String> {
        let ast = Parser::new(pattern).parse().unwrap();
        required_literal(&ast)
    }

    #[test]
    fn returns_the_whole_pattern_when_it_is_fully_literal() {
        assert_eq!(literal("abc"), Some("abc".to_string()));
    }

    #[test]
    fn returns_none_for_a_single_dot() {
        assert_eq!(literal("."), None);
    }

    #[test]
    fn returns_none_for_a_bare_alternation() {
        assert_eq!(literal("cat|dog"), None);
    }

    #[test]
    fn picks_the_longer_run_across_a_dot() {
        assert_eq!(literal("abc.efgh"), Some("efgh".to_string()));
    }

    #[test]
    fn picks_the_longer_run_across_a_char_class() {
        assert_eq!(literal("ab[0-9]cde"), Some("cde".to_string()));
    }

    #[test]
    fn picks_the_longer_run_across_a_repetition() {
        assert_eq!(literal("ab+cde"), Some("cde".to_string()));
    }

    #[test]
    fn a_repetition_never_bridges_its_neighbors_into_one_run() {
        // "ab+c" never guarantees "abc" as a contiguous substring: "abbbc"
        // matches and does not contain it. The runs on either side of the
        // repetition ("a" and "c") are equal length, and the first one
        // encountered wins the tie.
        assert_eq!(literal("ab+c"), Some("a".to_string()));
    }

    #[test]
    fn an_alternation_nested_in_a_concat_still_lets_the_surrounding_literals_through() {
        assert_eq!(literal("foo(cat|dog)bar"), Some("foo".to_string()));
    }

    #[test]
    fn a_group_around_a_fully_literal_subpattern_merges_transparently() {
        assert_eq!(literal("a(bc)d"), Some("abcd".to_string()));
    }

    #[test]
    fn a_group_with_an_internal_break_still_surfaces_its_best_run() {
        // Regression test: the group as a whole is not exact (it has a dot
        // in it), so it must not be treated as if it always produced a
        // fixed string, but the "abcdefgh" run inside it must still be
        // found rather than lost.
        assert_eq!(
            literal("(abcdefgh.ij)xy"),
            Some("abcdefgh".to_string())
        );
    }

    #[test]
    fn anchors_do_not_break_contiguity() {
        assert_eq!(literal("^abc$"), Some("abc".to_string()));
    }

    #[test]
    fn word_boundaries_do_not_break_contiguity() {
        assert_eq!(literal(r"\bcat\b"), Some("cat".to_string()));
    }

    #[test]
    fn exact_literal_returns_the_concatenation_of_a_fully_literal_pattern() {
        let ast = Parser::new("abc").parse().unwrap();
        assert_eq!(exact_literal(&ast), Some("abc".to_string()));
    }

    #[test]
    fn exact_literal_returns_none_as_soon_as_one_item_is_not_exact() {
        let ast = Parser::new("abc.def").parse().unwrap();
        assert_eq!(exact_literal(&ast), None);
    }

    #[test]
    fn exact_literal_passes_through_a_group_around_exact_content() {
        let ast = Parser::new("a(bc)d").parse().unwrap();
        assert_eq!(exact_literal(&ast), Some("abcd".to_string()));
    }

    #[test]
    fn exact_literal_returns_none_when_a_group_is_not_exact() {
        let ast = Parser::new("(a.b)").parse().unwrap();
        assert_eq!(exact_literal(&ast), None);
    }

    #[test]
    fn longer_keeps_the_first_argument_when_it_is_strictly_longer() {
        assert_eq!(
            longer(Some("ab".to_string()), Some("c".to_string())),
            Some("ab".to_string())
        );
    }

    #[test]
    fn longer_keeps_the_second_argument_when_it_is_strictly_longer() {
        assert_eq!(
            longer(Some("a".to_string()), Some("bb".to_string())),
            Some("bb".to_string())
        );
    }

    #[test]
    fn longer_keeps_the_first_argument_on_a_tie() {
        assert_eq!(
            longer(Some("ab".to_string()), Some("cd".to_string())),
            Some("ab".to_string())
        );
    }

    #[test]
    fn longer_keeps_whichever_side_is_some() {
        assert_eq!(longer(Some("a".to_string()), None), Some("a".to_string()));
        assert_eq!(longer(None, Some("b".to_string())), Some("b".to_string()));
    }

    #[test]
    fn longer_returns_none_when_both_sides_are_none() {
        assert_eq!(longer(None, None), None);
    }
}
