use std::{fmt::Display, io};

use regex_engine::ParseError;

use crate::args::ArgsError;

#[derive(Debug)]
pub enum AppError {
    Args(ArgsError),
    Pattern(ParseError),
    Io(io::Error),
}

impl From<ArgsError> for AppError {
    fn from(value: ArgsError) -> Self {
        Self::Args(value)
    }
}

impl From<ParseError> for AppError {
    fn from(value: ParseError) -> Self {
        Self::Pattern(value)
    }
}

impl From<io::Error> for AppError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Args(value) => write!(f, "{}", value),
            Self::Pattern(value) => write!(f, "{}", value),
            Self::Io(value) => write!(f, "{}", value),
        }
    }
}
