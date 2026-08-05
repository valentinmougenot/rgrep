#[derive(Debug, PartialEq, Eq)]
pub enum Ast {
    Alternation(Vec<Self>),
    Concat(Vec<Self>),
    Repetition {
        kind: RepetitionKind,
        inner: Box<Self>,
    },
    Group(Box<Self>),
    CharClass {
        negative: bool,
        items: Vec<ClassItem>,
    },
    Literal(char),
    Dot,
    StartAnchor,
    EndAnchor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepetitionKind {
    ZeroOrMore,
    OneOrMore,
    ZeroOrOne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassItem {
    Char(char),
    Range(char, char),
}
