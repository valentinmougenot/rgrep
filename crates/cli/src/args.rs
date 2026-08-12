use std::{fmt::Display, path::PathBuf};

use crate::output::OutputMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub pattern: String,
    pub path: Option<PathBuf>,
    pub output_mode: OutputMode,
    pub case_insensitive: bool,
    pub whole_word: bool,
    pub invert_match: bool,
    pub before_context: usize,
    pub after_context: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgsError {
    pub kind: ArgsErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsErrorKind {
    MissingPattern,
    UnexpectedArgument(String),
    UnexpectedEOI,
}

impl Display for ArgsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            ArgsErrorKind::MissingPattern => write!(f, "Missing pattern"),
            ArgsErrorKind::UnexpectedArgument(arg) => write!(f, "UnexpectedArgument '{}'", arg),
            ArgsErrorKind::UnexpectedEOI => write!(f, "Unexpected end of input"),
        }
    }
}

pub fn parse(args: &mut impl Iterator<Item = String>) -> Result<Args, ArgsError> {
    let mut positionals = Vec::new();
    let mut output_mode = OutputMode::Matches;
    let mut case_insensitive = false;
    let mut whole_word = false;
    let mut invert_match = false;
    let mut after_context: usize = 0;
    let mut before_context: usize = 0;

    while let Some(arg) = args.next() {
        if let Some(long) = arg.strip_prefix("--") {
            match long {
                "count" => output_mode = OutputMode::Count,
                "files-with-matches" => output_mode = OutputMode::Files,
                "ignore-case" => case_insensitive = true,
                "word-regexp" => whole_word = true,
                "invert-match" => invert_match = true,
                "after-context" => {
                    let next = args.next().ok_or(ArgsError {
                        kind: ArgsErrorKind::UnexpectedEOI,
                    })?;
                    after_context = next.parse().map_err(|_| ArgsError {
                        kind: ArgsErrorKind::UnexpectedArgument(next),
                    })?;
                }
                "before-context" => {
                    let next = args.next().ok_or(ArgsError {
                        kind: ArgsErrorKind::UnexpectedEOI,
                    })?;
                    before_context = next.parse().map_err(|_| ArgsError {
                        kind: ArgsErrorKind::UnexpectedArgument(next),
                    })?;
                }
                "context" => {
                    let next = args.next().ok_or(ArgsError {
                        kind: ArgsErrorKind::UnexpectedEOI,
                    })?;
                    let context = next.parse().map_err(|_| ArgsError {
                        kind: ArgsErrorKind::UnexpectedArgument(next),
                    })?;
                    after_context = context;
                    before_context = context;
                }
                _ => {
                    return Err(ArgsError {
                        kind: ArgsErrorKind::UnexpectedArgument(arg),
                    });
                }
            }
        } else if let Some(short) = arg.strip_prefix('-') {
            for (i, c) in short.char_indices() {
                match c {
                    'c' => output_mode = OutputMode::Count,
                    'l' => output_mode = OutputMode::Files,
                    'i' => case_insensitive = true,
                    'w' => whole_word = true,
                    'v' => invert_match = true,
                    'A' => {
                        let rest = &short[i + c.len_utf8()..];
                        let value = if rest.is_empty() {
                            args.next().ok_or(ArgsError {
                                kind: ArgsErrorKind::UnexpectedEOI,
                            })?
                        } else {
                            rest.to_string()
                        };
                        after_context = value.parse().map_err(|_| ArgsError {
                            kind: ArgsErrorKind::UnexpectedArgument(value),
                        })?;
                        break;
                    }
                    'B' => {
                        let rest = &short[i + c.len_utf8()..];
                        let value = if rest.is_empty() {
                            args.next().ok_or(ArgsError {
                                kind: ArgsErrorKind::UnexpectedEOI,
                            })?
                        } else {
                            rest.to_string()
                        };
                        before_context = value.parse().map_err(|_| ArgsError {
                            kind: ArgsErrorKind::UnexpectedArgument(value),
                        })?;
                        break;
                    }
                    'C' => {
                        let rest = &short[i + c.len_utf8()..];
                        let value = if rest.is_empty() {
                            args.next().ok_or(ArgsError {
                                kind: ArgsErrorKind::UnexpectedEOI,
                            })?
                        } else {
                            rest.to_string()
                        };
                        after_context = value.parse().map_err(|_| ArgsError {
                            kind: ArgsErrorKind::UnexpectedArgument(value),
                        })?;
                        before_context = after_context;
                        break;
                    }
                    _ => {
                        return Err(ArgsError {
                            kind: ArgsErrorKind::UnexpectedArgument(arg.clone()),
                        });
                    }
                }
            }
        } else {
            positionals.push(arg);
        }
    }

    let mut positionals = positionals.into_iter();

    let pattern = positionals.next().ok_or(ArgsError {
        kind: ArgsErrorKind::MissingPattern,
    })?;
    let path = positionals.next().map(PathBuf::from);

    if let Some(arg) = positionals.next() {
        return Err(ArgsError {
            kind: ArgsErrorKind::UnexpectedArgument(arg),
        });
    }

    Ok(Args {
        pattern,
        path,
        output_mode,
        case_insensitive,
        whole_word,
        invert_match,
        before_context,
        after_context,
    })
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
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedArgument("extra".into()));
    }

    #[test]
    fn default_output_mode_is_matches() {
        let args = parse_vec(vec!["ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Matches);
    }

    #[test]
    fn short_count_flag_sets_count_mode() {
        let args = parse_vec(vec!["-c", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Count);
    }

    #[test]
    fn short_files_flag_sets_files_mode() {
        let args = parse_vec(vec!["-l", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Files);
    }

    #[test]
    fn long_count_flag_sets_count_mode() {
        let args = parse_vec(vec!["--count", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Count);
    }

    #[test]
    fn long_files_flag_sets_files_mode() {
        let args = parse_vec(vec!["--files-with-matches", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Files);
    }

    #[test]
    fn bundled_short_flags_the_last_character_wins() {
        let args = parse_vec(vec!["-lc", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Count);

        let args = parse_vec(vec!["-cl", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Files);
    }

    #[test]
    fn later_flag_wins_over_an_earlier_one() {
        let args = parse_vec(vec!["-c", "-l", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Files);

        let args = parse_vec(vec!["-l", "-c", "ab"]).unwrap();
        assert_eq!(args.output_mode, OutputMode::Count);
    }

    #[test]
    fn unknown_short_flag_in_a_bundle_is_unexpected() {
        let err = parse_vec(vec!["-lx", "ab"]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedArgument("-lx".into()));
    }

    #[test]
    fn unknown_long_flag_is_unexpected() {
        let err = parse_vec(vec!["--bogus", "ab"]).unwrap_err();
        assert_eq!(
            err.kind,
            ArgsErrorKind::UnexpectedArgument("--bogus".into())
        );
    }

    #[test]
    fn default_case_insensitive_is_false() {
        let args = parse_vec(vec!["ab"]).unwrap();
        assert!(!args.case_insensitive);
    }

    #[test]
    fn short_ignore_case_flag_sets_case_insensitive() {
        let args = parse_vec(vec!["-i", "ab"]).unwrap();
        assert!(args.case_insensitive);
    }

    #[test]
    fn long_ignore_case_flag_sets_case_insensitive() {
        let args = parse_vec(vec!["--ignore-case", "ab"]).unwrap();
        assert!(args.case_insensitive);
    }

    #[test]
    fn ignore_case_can_be_bundled_with_other_short_flags() {
        let args = parse_vec(vec!["-ic", "ab"]).unwrap();
        assert!(args.case_insensitive);
        assert_eq!(args.output_mode, OutputMode::Count);
    }

    #[test]
    fn default_whole_word_is_false() {
        let args = parse_vec(vec!["ab"]).unwrap();
        assert!(!args.whole_word);
    }

    #[test]
    fn short_word_regexp_flag_sets_whole_word() {
        let args = parse_vec(vec!["-w", "ab"]).unwrap();
        assert!(args.whole_word);
    }

    #[test]
    fn long_word_regexp_flag_sets_whole_word() {
        let args = parse_vec(vec!["--word-regexp", "ab"]).unwrap();
        assert!(args.whole_word);
    }

    #[test]
    fn word_regexp_can_be_bundled_with_other_short_flags() {
        let args = parse_vec(vec!["-wi", "ab"]).unwrap();
        assert!(args.whole_word);
        assert!(args.case_insensitive);
    }

    #[test]
    fn default_invert_match_is_false() {
        let args = parse_vec(vec!["ab"]).unwrap();
        assert!(!args.invert_match);
    }

    #[test]
    fn short_invert_match_flag_sets_invert_match() {
        let args = parse_vec(vec!["-v", "ab"]).unwrap();
        assert!(args.invert_match);
    }

    #[test]
    fn long_invert_match_flag_sets_invert_match() {
        let args = parse_vec(vec!["--invert-match", "ab"]).unwrap();
        assert!(args.invert_match);
    }

    #[test]
    fn invert_match_can_be_bundled_with_other_short_flags() {
        let args = parse_vec(vec!["-vi", "ab"]).unwrap();
        assert!(args.invert_match);
        assert!(args.case_insensitive);
    }
}
