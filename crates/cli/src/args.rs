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
    pub only_matching: bool,
    pub fixed_strings: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgsError {
    pub kind: ArgsErrorKind,
}

impl ArgsError {
    fn missing_pattern() -> Self {
        Self {
            kind: ArgsErrorKind::MissingPattern,
        }
    }

    fn unexpected(arg: String) -> Self {
        Self {
            kind: ArgsErrorKind::UnexpectedArgument(arg),
        }
    }

    fn eoi() -> Self {
        Self {
            kind: ArgsErrorKind::UnexpectedEOI,
        }
    }
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
    let mut after_context = 0;
    let mut before_context = 0;
    let mut only_matching = false;
    let mut fixed_strings = false;

    while let Some(arg) = args.next() {
        if let Some(long) = arg.strip_prefix("--") {
            match long {
                "count" => output_mode = OutputMode::Count,
                "files-with-matches" => output_mode = OutputMode::Files,
                "ignore-case" => case_insensitive = true,
                "word-regexp" => whole_word = true,
                "invert-match" => invert_match = true,
                "only-matching" => only_matching = true,
                "fixed-strings" => fixed_strings = true,
                "after-context" => {
                    let next = next_arg(args)?;
                    after_context = parse_usize(next)?;
                }
                "before-context" => {
                    let next = next_arg(args)?;
                    before_context = parse_usize(next)?;
                }
                "context" => {
                    let next = next_arg(args)?;
                    after_context = parse_usize(next)?;
                    before_context = after_context;
                }
                _ => {
                    return Err(ArgsError::unexpected(arg));
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
                    'o' => only_matching = true,
                    'F' => fixed_strings = true,
                    'A' => {
                        let value = short_flag_value(short, i, c, args)?;
                        after_context = parse_usize(value)?;
                        break;
                    }
                    'B' => {
                        let value = short_flag_value(short, i, c, args)?;
                        before_context = parse_usize(value)?;
                        break;
                    }
                    'C' => {
                        let value = short_flag_value(short, i, c, args)?;
                        after_context = parse_usize(value)?;
                        before_context = after_context;
                        break;
                    }
                    _ => {
                        return Err(ArgsError::unexpected(arg));
                    }
                }
            }
        } else {
            positionals.push(arg);
        }
    }

    let mut positionals = positionals.into_iter();

    let pattern = positionals.next().ok_or(ArgsError::missing_pattern())?;
    let path = positionals.next().map(PathBuf::from);

    if let Some(arg) = positionals.next() {
        return Err(ArgsError::unexpected(arg));
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
        only_matching,
        fixed_strings,
    })
}

fn next_arg(args: &mut impl Iterator<Item = String>) -> Result<String, ArgsError> {
    args.next().ok_or(ArgsError::eoi())
}

fn parse_usize(value: String) -> Result<usize, ArgsError> {
    value.parse().map_err(|_| ArgsError::unexpected(value))
}

fn short_flag_value(
    short: &str,
    pos: usize,
    c: char,
    args: &mut impl Iterator<Item = String>,
) -> Result<String, ArgsError> {
    let rest = &short[pos + c.len_utf8()..];
    if rest.is_empty() {
        next_arg(args)
    } else {
        Ok(rest.to_string())
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

    #[test]
    fn default_context_is_zero() {
        let args = parse_vec(vec!["ab"]).unwrap();
        assert_eq!(args.before_context, 0);
        assert_eq!(args.after_context, 0);
    }

    #[test]
    fn long_after_context_flag_sets_after_context() {
        let args = parse_vec(vec!["--after-context", "2", "ab"]).unwrap();
        assert_eq!(args.after_context, 2);
        assert_eq!(args.before_context, 0);
    }

    #[test]
    fn long_before_context_flag_sets_before_context() {
        let args = parse_vec(vec!["--before-context", "3", "ab"]).unwrap();
        assert_eq!(args.before_context, 3);
        assert_eq!(args.after_context, 0);
    }

    #[test]
    fn long_context_flag_sets_both_before_and_after_context() {
        let args = parse_vec(vec!["--context", "2", "ab"]).unwrap();
        assert_eq!(args.before_context, 2);
        assert_eq!(args.after_context, 2);
    }

    #[test]
    fn long_after_context_flag_without_a_value_is_unexpected_eoi() {
        let err = parse_vec(vec!["--after-context"]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedEOI);
    }

    #[test]
    fn long_after_context_flag_with_a_non_numeric_value_is_unexpected() {
        let err = parse_vec(vec!["--after-context", "x", "ab"]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedArgument("x".into()));
    }

    #[test]
    fn short_after_context_flag_with_a_separate_value_sets_after_context() {
        let args = parse_vec(vec!["-A", "4", "ab"]).unwrap();
        assert_eq!(args.after_context, 4);
    }

    #[test]
    fn short_after_context_flag_with_an_attached_value_sets_after_context() {
        let args = parse_vec(vec!["-A4", "ab"]).unwrap();
        assert_eq!(args.after_context, 4);
    }

    #[test]
    fn short_before_context_flag_with_a_separate_value_sets_before_context() {
        let args = parse_vec(vec!["-B", "4", "ab"]).unwrap();
        assert_eq!(args.before_context, 4);
    }

    #[test]
    fn short_before_context_flag_with_an_attached_value_sets_before_context() {
        let args = parse_vec(vec!["-B4", "ab"]).unwrap();
        assert_eq!(args.before_context, 4);
    }

    #[test]
    fn short_context_flag_sets_both_before_and_after_context() {
        let args = parse_vec(vec!["-C2", "ab"]).unwrap();
        assert_eq!(args.before_context, 2);
        assert_eq!(args.after_context, 2);
    }

    #[test]
    fn short_after_context_flag_without_a_value_is_unexpected_eoi() {
        let err = parse_vec(vec!["-A"]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedEOI);
    }

    #[test]
    fn short_after_context_flag_with_a_non_numeric_attached_value_is_unexpected() {
        let err = parse_vec(vec!["-Ax", "ab"]).unwrap_err();
        assert_eq!(err.kind, ArgsErrorKind::UnexpectedArgument("x".into()));
    }

    #[test]
    fn short_context_flag_can_be_bundled_after_other_short_flags() {
        let args = parse_vec(vec!["-iA2", "ab"]).unwrap();
        assert!(args.case_insensitive);
        assert_eq!(args.after_context, 2);
    }
}
