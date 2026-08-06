use std::{fmt::Display, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub pattern: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgsError {
    pub kind: ArgsErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsErrorKind {
    MissingPattern,
    UnexpectedArgument(String),
}

impl Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ArgsErrorKind::MissingPattern => write!(f, "Missing pattern"),
            ArgsErrorKind::UnexpectedArgument(arg) => write!(f, "UnexpectedArgument '{}'", arg),
        }
    }
}

pub fn parse(args: &mut impl Iterator<Item = String>) -> Result<Args, ArgsError> {
    let pattern = match args.next() {
        Some(arg) if !arg.starts_with("-") => arg,
        Some(arg) => {
            return Err(ArgsError {
                kind: ArgsErrorKind::UnexpectedArgument(arg),
            });
        }
        None => {
            return Err(ArgsError {
                kind: ArgsErrorKind::MissingPattern,
            });
        }
    };

    let path = match args.next() {
        Some(arg) if arg.starts_with("-") => {
            return Err(ArgsError {
                kind: ArgsErrorKind::UnexpectedArgument(arg),
            });
        }
        arg => arg.map(PathBuf::from),
    };

    if let Some(arg) = args.next() {
        Err(ArgsError {
            kind: ArgsErrorKind::UnexpectedArgument(arg),
        })
    } else {
        Ok(Args { pattern, path })
    }
}
