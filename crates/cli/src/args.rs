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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_vec(v: Vec<&str>) -> Result<Args, ArgsError> {
        parse(&mut v.into_iter().map(String::from))
    }

    #[test]
    fn pattern_only() {
        let args = parse_vec(vec!["ab"]).unwrap();
        assert_eq!(args.pattern, "ab");
        assert_eq!(args.path, None);
    }

    #[test]
    fn pattern_and_path() {
        let args = parse_vec(vec!["ab", "file.txt"]).unwrap();
        assert_eq!(args.pattern, "ab");
        assert_eq!(args.path, Some(PathBuf::from("file.txt")));
    }

    #[test]
    fn no_args_is_missing_pattern() {
        let err = parse_vec(vec![]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::MissingPattern);
    }

    #[test]
    fn pattern_starting_with_dash_is_unexpected() {
        let err = parse_vec(vec!["-ab"]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedArgument("-ab".into()));
    }

    #[test]
    fn path_starting_with_dash_is_unexpected() {
        let err = parse_vec(vec!["ab", "-x"]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedArgument("-x".into()));
    }

    #[test]
    fn extra_positional_argument_is_unexpected() {
        let err = parse_vec(vec!["ab", "file.txt", "extra"]).unwrap_err();
        assert_eq!(
            err.kind,
            ArgsErrorKind::UnexpectedArgument("extra".into())
        );
    }
}
