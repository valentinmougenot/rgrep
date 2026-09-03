use crate::{
    ParseError, ParseErrorKind,
    ast::{Ast, ClassItem, RepetitionKind},
    scanner::Scanner,
};

pub(crate) struct Parser<'a> {
    scanner: Scanner<'a>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            scanner: Scanner::new(input),
        }
    }

    pub fn parse(&mut self) -> Result<Ast, ParseError> {
        self.parse_alternation()
    }

    fn parse_alternation(&mut self) -> Result<Ast, ParseError> {
        let concat = self.parse_concat()?;
        if !self.scanner.peek().is_some_and(|c| c == '|') {
            return Ok(concat);
        }

        let mut items = vec![concat];
        while self.scanner.peek().is_some_and(|c| c == '|') {
            self.scanner.bump();
            items.push(self.parse_concat()?);
        }

        Ok(Ast::Alternation(items))
    }

    fn parse_concat(&mut self) -> Result<Ast, ParseError> {
        let mut items = Vec::new();
        while !matches!(self.scanner.peek(), None | Some('|' | ')')) {
            items.push(self.parse_repetition()?);
        }
        Ok(Ast::Concat(items))
    }

    fn parse_repetition(&mut self) -> Result<Ast, ParseError> {
        let inner = self.parse_atom()?;
        let kind = match self.scanner.peek() {
            Some('*') => {
                self.scanner.bump();
                RepetitionKind::ZeroOrMore
            }
            Some('+') => {
                self.scanner.bump();
                RepetitionKind::OneOrMore
            }
            Some('?') => {
                self.scanner.bump();
                RepetitionKind::ZeroOrOne
            }
            _ => return Ok(inner),
        };

        Ok(Ast::Repetition {
            kind,
            inner: Box::new(inner),
        })
    }

    fn parse_atom(&mut self) -> Result<Ast, ParseError> {
        match self.scanner.peek() {
            Some('.') => {
                self.scanner.bump();
                Ok(Ast::Dot)
            }
            Some('^') => {
                self.scanner.bump();
                Ok(Ast::StartAnchor)
            }
            Some('$') => {
                self.scanner.bump();
                Ok(Ast::EndAnchor)
            }
            Some('[') => self.parse_char_class(),
            Some('(') => {
                self.scanner.bump();
                let item = self.parse_alternation()?;
                match self.scanner.bump() {
                    Some(')') => Ok(Ast::Group(Box::new(item))),
                    Some(_) => Err(ParseError {
                        kind: ParseErrorKind::UnclosedGroup,
                        pos: self.scanner.position(),
                    }),
                    None => Err(ParseError {
                        kind: ParseErrorKind::UnexpectedEof,
                        pos: self.scanner.position(),
                    }),
                }
            }
            Some('\\') => {
                self.scanner.bump();
                match self.scanner.bump() {
                    Some('b') => Ok(Ast::WordBoundary),
                    Some(c) => Ok(Ast::Literal(c)),
                    None => Err(ParseError {
                        kind: ParseErrorKind::UnexpectedEof,
                        pos: self.scanner.position(),
                    }),
                }
            }
            Some('*' | '+' | '?') => Err(ParseError {
                kind: ParseErrorKind::DanglingRepetitionOperator,
                pos: self.scanner.position(),
            }),
            Some(c) => {
                self.scanner.bump();
                Ok(Ast::Literal(c))
            }
            None => Err(ParseError {
                kind: ParseErrorKind::UnexpectedEof,
                pos: self.scanner.position(),
            }),
        }
    }

    fn parse_char_class(&mut self) -> Result<Ast, ParseError> {
        debug_assert_eq!(self.scanner.bump(), Some('['));

        let negative = match self.scanner.peek() {
            Some('^') => {
                self.scanner.bump();
                true
            }
            Some(']') => {
                return Err(ParseError {
                    kind: ParseErrorKind::EmptyCharClass,
                    pos: self.scanner.position(),
                });
            }
            Some(_) => false,
            None => {
                return Err(ParseError {
                    kind: ParseErrorKind::UnexpectedEof,
                    pos: self.scanner.position(),
                });
            }
        };

        if self.scanner.peek().is_some_and(|c| c == ']') {
            return Err(ParseError {
                kind: ParseErrorKind::EmptyCharClass,
                pos: self.scanner.position(),
            });
        }

        let mut items = vec![self.parse_class_item()?];

        loop {
            match self.scanner.peek() {
                Some(']') => {
                    self.scanner.bump();
                    break;
                }
                Some(_) => items.push(self.parse_class_item()?),
                None => {
                    return Err(ParseError {
                        kind: ParseErrorKind::UnclosedCharClass,
                        pos: self.scanner.position(),
                    });
                }
            }
        }

        Ok(Ast::CharClass { negative, items })
    }

    fn parse_class_item(&mut self) -> Result<ClassItem, ParseError> {
        match self.scanner.bump() {
            Some('\\') => match self.scanner.bump() {
                Some(c) => Ok(ClassItem::Char(c)),
                None => Err(ParseError {
                    kind: ParseErrorKind::UnexpectedEof,
                    pos: self.scanner.position(),
                }),
            },
            Some(c1) => match self.scanner.peek() {
                Some('-') => {
                    if self.scanner.peek2().is_some_and(|c| c == ']') {
                        return Ok(ClassItem::Char(c1));
                    }

                    self.scanner.bump();
                    match self.scanner.bump() {
                        Some(c2) => {
                            if c1 > c2 {
                                Err(ParseError {
                                    kind: ParseErrorKind::InvalidRange { start: c1, end: c2 },
                                    pos: self.scanner.position(),
                                })
                            } else {
                                Ok(ClassItem::Range(c1, c2))
                            }
                        }
                        None => Err(ParseError {
                            kind: ParseErrorKind::UnexpectedEof,
                            pos: self.scanner.position(),
                        }),
                    }
                }
                _ => Ok(ClassItem::Char(c1)),
            },
            None => Err(ParseError {
                kind: ParseErrorKind::UnexpectedEof,
                pos: self.scanner.position(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Result<Ast, ParseError> {
        Parser::new(input).parse()
    }

    fn lit(c: char) -> Ast {
        Ast::Literal(c)
    }

    fn concat(items: Vec<Ast>) -> Ast {
        Ast::Concat(items)
    }

    fn alt(items: Vec<Ast>) -> Ast {
        Ast::Alternation(items)
    }

    #[test]
    fn parses_single_literal() {
        assert_eq!(parse("a"), Ok(concat(vec![lit('a')])));
    }

    #[test]
    fn parses_concat_of_literals() {
        assert_eq!(parse("ab"), Ok(concat(vec![lit('a'), lit('b')])));
    }

    #[test]
    fn parses_empty_pattern_as_empty_concat() {
        assert_eq!(parse(""), Ok(concat(vec![])));
    }

    #[test]
    fn parses_alternation() {
        assert_eq!(
            parse("a|b"),
            Ok(alt(vec![concat(vec![lit('a')]), concat(vec![lit('b')])]))
        );
    }

    #[test]
    fn parses_alternation_with_more_than_two_branches() {
        assert_eq!(
            parse("a|b|c"),
            Ok(alt(vec![
                concat(vec![lit('a')]),
                concat(vec![lit('b')]),
                concat(vec![lit('c')]),
            ]))
        );
    }

    #[test]
    fn parses_dot() {
        assert_eq!(parse("."), Ok(concat(vec![Ast::Dot])));
    }

    #[test]
    fn parses_anchors() {
        assert_eq!(
            parse("^a$"),
            Ok(concat(vec![Ast::StartAnchor, lit('a'), Ast::EndAnchor]))
        );
    }

    #[test]
    fn parses_repetition_operators() {
        for (op, kind) in [
            ("a*", RepetitionKind::ZeroOrMore),
            ("a+", RepetitionKind::OneOrMore),
            ("a?", RepetitionKind::ZeroOrOne),
        ] {
            assert_eq!(
                parse(op),
                Ok(concat(vec![Ast::Repetition {
                    kind,
                    inner: Box::new(lit('a')),
                }])),
                "parsing {op:?}"
            );
        }
    }

    #[test]
    fn repetition_only_binds_to_the_immediately_preceding_atom() {
        assert_eq!(
            parse("ab*"),
            Ok(concat(vec![
                lit('a'),
                Ast::Repetition {
                    kind: RepetitionKind::ZeroOrMore,
                    inner: Box::new(lit('b')),
                }
            ]))
        );
    }

    #[test]
    fn parses_group() {
        assert_eq!(
            parse("(a)"),
            Ok(concat(vec![Ast::Group(Box::new(concat(vec![lit('a')])))]))
        );
    }

    #[test]
    fn parses_group_with_alternation_followed_by_more_input() {
        assert_eq!(
            parse("(a|b)c"),
            Ok(concat(vec![
                Ast::Group(Box::new(alt(vec![
                    concat(vec![lit('a')]),
                    concat(vec![lit('b')]),
                ]))),
                lit('c'),
            ]))
        );
    }

    #[test]
    fn parses_escaped_metacharacter_as_literal() {
        assert_eq!(parse(r"\."), Ok(concat(vec![lit('.')])));
    }

    #[test]
    fn parses_char_class_of_literals() {
        assert_eq!(
            parse("[abc]"),
            Ok(concat(vec![Ast::CharClass {
                negative: false,
                items: vec![
                    ClassItem::Char('a'),
                    ClassItem::Char('b'),
                    ClassItem::Char('c'),
                ],
            }]))
        );
    }

    #[test]
    fn parses_char_class_range() {
        assert_eq!(
            parse("[a-z]"),
            Ok(concat(vec![Ast::CharClass {
                negative: false,
                items: vec![ClassItem::Range('a', 'z')],
            }]))
        );
    }

    #[test]
    fn parses_negated_char_class() {
        assert_eq!(
            parse("[^a-z]"),
            Ok(concat(vec![Ast::CharClass {
                negative: true,
                items: vec![ClassItem::Range('a', 'z')],
            }]))
        );
    }

    #[test]
    fn parses_escaped_bracket_inside_char_class() {
        assert_eq!(
            parse(r"[\]]"),
            Ok(concat(vec![Ast::CharClass {
                negative: false,
                items: vec![ClassItem::Char(']')],
            }]))
        );
    }

    #[test]
    fn dash_at_start_of_char_class_is_literal() {
        assert_eq!(
            parse("[-a]"),
            Ok(concat(vec![Ast::CharClass {
                negative: false,
                items: vec![ClassItem::Char('-'), ClassItem::Char('a')],
            }]))
        );
    }

    #[test]
    fn dash_at_end_of_char_class_is_literal() {
        assert_eq!(
            parse("[a-]"),
            Ok(concat(vec![Ast::CharClass {
                negative: false,
                items: vec![ClassItem::Char('a'), ClassItem::Char('-')],
            }]))
        );
    }

    #[test]
    fn lone_dash_char_class_is_literal() {
        assert_eq!(
            parse("[-]"),
            Ok(concat(vec![Ast::CharClass {
                negative: false,
                items: vec![ClassItem::Char('-')],
            }]))
        );
    }

    #[test]
    fn range_followed_by_trailing_literal_dash() {
        assert_eq!(
            parse("[a-z-]"),
            Ok(concat(vec![Ast::CharClass {
                negative: false,
                items: vec![ClassItem::Range('a', 'z'), ClassItem::Char('-')],
            }]))
        );
    }

    #[test]
    fn empty_char_class_is_an_error() {
        assert_eq!(
            parse("[]"),
            Err(ParseError {
                kind: ParseErrorKind::EmptyCharClass,
                pos: 1,
            })
        );
    }

    #[test]
    fn empty_negated_char_class_is_an_error() {
        assert_eq!(
            parse("[^]"),
            Err(ParseError {
                kind: ParseErrorKind::EmptyCharClass,
                pos: 2,
            })
        );
    }

    #[test]
    fn unclosed_char_class_is_an_error() {
        assert_eq!(
            parse("[a"),
            Err(ParseError {
                kind: ParseErrorKind::UnclosedCharClass,
                pos: 2,
            })
        );
    }

    #[test]
    fn inverted_range_bounds_are_an_error() {
        assert_eq!(
            parse("[z-a]"),
            Err(ParseError {
                kind: ParseErrorKind::InvalidRange {
                    start: 'z',
                    end: 'a'
                },
                pos: 4,
            })
        );
    }

    #[test]
    fn dangling_repetition_operator_at_start_is_an_error() {
        for op in ["*a", "+a", "?a"] {
            assert_eq!(
                parse(op),
                Err(ParseError {
                    kind: ParseErrorKind::DanglingRepetitionOperator,
                    pos: 0,
                }),
                "parsing {op:?}"
            );
        }
    }

    #[test]
    fn dangling_repetition_operator_after_alternation_separator_is_an_error() {
        assert_eq!(
            parse("a|*b"),
            Err(ParseError {
                kind: ParseErrorKind::DanglingRepetitionOperator,
                pos: 2,
            })
        );
    }

    #[test]
    fn dangling_repetition_operator_inside_group_is_an_error() {
        assert_eq!(
            parse("(*a)"),
            Err(ParseError {
                kind: ParseErrorKind::DanglingRepetitionOperator,
                pos: 1,
            })
        );
    }

    #[test]
    fn unclosed_group_is_reported_as_unexpected_eof() {
        // Note: ParseErrorKind::UnclosedGroup is currently unreachable.
        // parse_concat only stops at '|', ')' or EOF, and parse_alternation
        // only consumes '|', so by the time the group's closing paren is
        // expected, the only two possible remaining tokens are ')' or EOF.
        // Any other trailing character (e.g. the ']' below) gets consumed
        // as a literal by parse_concat before we ever get here.
        assert_eq!(
            parse("(a"),
            Err(ParseError {
                kind: ParseErrorKind::UnexpectedEof,
                pos: 2,
            })
        );
        assert_eq!(
            parse("(a]"),
            Err(ParseError {
                kind: ParseErrorKind::UnexpectedEof,
                pos: 3,
            })
        );
    }

    #[test]
    fn trailing_backslash_is_an_error() {
        assert_eq!(
            parse("\\"),
            Err(ParseError {
                kind: ParseErrorKind::UnexpectedEof,
                pos: 1,
            })
        );
    }
}
