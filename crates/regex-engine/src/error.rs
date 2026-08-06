use std::fmt::Display;

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

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ParseErrorKind::UnexpectedEof => write!(f, "Unexpected EOF at position {}", self.pos),
            ParseErrorKind::UnclosedGroup => write!(f, "Unclosed group at position {}", self.pos),
            ParseErrorKind::UnclosedCharClass => {
                write!(f, "Unclosed char class at position {}", self.pos)
            }
            ParseErrorKind::EmptyCharClass => {
                write!(f, "Empty char class at position {}", self.pos)
            }
            ParseErrorKind::InvalidRange { start, end } => {
                write!(
                    f,
                    "Invalid range '{}-{}' at position {}",
                    start, end, self.pos
                )
            }
            ParseErrorKind::DanglingRepetitionOperator => {
                write!(f, "Dangling repetition operator at position {}", self.pos)
            }
        }
    }
}
