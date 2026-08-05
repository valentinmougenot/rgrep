#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseErrorKind {
    UnexpectedEof,
    UnclosedGroup,
    UnclosedCharClass,
    EmptyCharClass,
    InvalidRange { start: char, end: char },
    DanglingRepetitionOperator,
}
