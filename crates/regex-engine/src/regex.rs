use crate::{
    ParseError,
    compiler::Compiler,
    nfa::Nfa,
    parser::Parser,
    vm::{find, is_match},
};

pub struct Regex {
    nfa: Nfa,
    case_insensitive: bool,
}

impl Regex {
    pub fn new(pattern: &str) -> Result<Self, ParseError> {
        let mut parser = Parser::new(pattern);
        let ast = parser.parse()?;
        let compiler = Compiler::new();
        let nfa = compiler.build(&ast);

        Ok(Self {
            nfa,
            case_insensitive: false,
        })
    }

    pub fn is_match(&self, text: &str) -> bool {
        is_match(&self.nfa, text, self.case_insensitive)
    }

    pub fn find<'t>(&self, text: &'t str) -> Option<Match<'t>> {
        let (start, end) = find(&self.nfa, text, self.case_insensitive)?;
        Some(Match { text, start, end })
    }
}

pub struct Match<'t> {
    text: &'t str,
    start: usize,
    end: usize,
}

impl<'t> Match<'t> {
    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }

    pub fn as_str(&self) -> &str {
        &self.text[self.start..self.end]
    }
}

pub struct RegexBuilder {
    pattern: String,
    case_insensitive: bool,
}

impl RegexBuilder {
    pub fn new(pattern: String) -> Self {
        Self {
            pattern,
            case_insensitive: false,
        }
    }

    pub fn case_insensitive(mut self, value: bool) -> Self {
        self.case_insensitive = value;
        self
    }

    pub fn build(self) -> Result<Regex, ParseError> {
        let mut parser = Parser::new(&self.pattern);
        let ast = parser.parse()?;
        let compiler = Compiler::new();
        let nfa = compiler.build(&ast);

        Ok(Regex {
            nfa,
            case_insensitive: self.case_insensitive,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_returns_error_for_invalid_pattern() {
        assert!(Regex::new("(").is_err());
    }

    #[test]
    fn new_returns_ok_for_valid_pattern() {
        assert!(Regex::new("abc").is_ok());
    }

    #[test]
    fn is_match_delegates_to_the_compiled_nfa() {
        let re = Regex::new("^ab$").unwrap();
        assert!(re.is_match("ab"));
        assert!(!re.is_match("xab"));
        assert!(!re.is_match("abx"));
    }

    #[test]
    fn is_match_finds_unanchored_substring() {
        let re = Regex::new("ab").unwrap();
        assert!(re.is_match("xxabxx"));
        assert!(!re.is_match("xyz"));
    }

    #[test]
    fn find_returns_none_when_there_is_no_match() {
        let re = Regex::new("abc").unwrap();
        assert!(re.find("xyz").is_none());
    }

    #[test]
    fn find_returns_the_start_and_end_of_the_match() {
        let re = Regex::new("ab").unwrap();
        let m = re.find("xxabxx").unwrap();
        assert_eq!(m.start(), 2);
        assert_eq!(m.end(), 4);
    }

    #[test]
    fn find_as_str_returns_the_matched_substring() {
        let re = Regex::new("ab").unwrap();
        let m = re.find("xxabxx").unwrap();
        assert_eq!(m.as_str(), "ab");
    }

    #[test]
    fn find_respects_anchors() {
        let re = Regex::new("^ab$").unwrap();
        assert!(re.find("ab").is_some());
        assert!(re.find("xab").is_none());
        assert!(re.find("abx").is_none());
    }
}
