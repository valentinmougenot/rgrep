use std::{iter::Peekable, str::CharIndices};

pub(crate) struct Scanner<'a> {
    chars: Peekable<CharIndices<'a>>,
    offset: usize,
}

impl<'a> Scanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.char_indices().peekable(),
            offset: 0,
        }
    }

    pub fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|c| c.1)
    }

    pub fn peek2(&self) -> Option<char> {
        let mut chars = self.chars.clone();
        chars.next();
        chars.peek().map(|c| c.1)
    }

    pub fn bump(&mut self) -> Option<char> {
        let (idx, c) = self.chars.next()?;
        self.offset = idx + c.len_utf8();
        Some(c)
    }

    pub fn position(&self) -> usize {
        self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peek_does_not_consume() {
        let mut s = Scanner::new("ab");
        assert_eq!(s.peek(), Some('a'));
        assert_eq!(s.peek(), Some('a'));
        assert_eq!(s.bump(), Some('a'));
        assert_eq!(s.peek(), Some('b'));
    }

    #[test]
    fn peek2_looks_one_char_further_than_peek() {
        let mut s = Scanner::new("abc");
        assert_eq!(s.peek(), Some('a'));
        assert_eq!(s.peek2(), Some('b'));
        assert_eq!(s.bump(), Some('a'));
        assert_eq!(s.peek(), Some('b'));
        assert_eq!(s.peek2(), Some('c'));
    }

    #[test]
    fn peek2_is_none_near_end_of_input() {
        let mut s = Scanner::new("a");
        assert_eq!(s.peek2(), None);
        assert_eq!(s.bump(), Some('a'));
        assert_eq!(s.peek2(), None);
    }

    #[test]
    fn bump_returns_none_at_eof() {
        let mut s = Scanner::new("");
        assert_eq!(s.peek(), None);
        assert_eq!(s.bump(), None);
    }

    #[test]
    fn position_starts_at_zero() {
        let s = Scanner::new("abc");
        assert_eq!(s.position(), 0);
    }

    #[test]
    fn position_tracks_byte_offset_not_char_count() {
        // 'é' is 2 bytes in UTF-8: position must advance by len_utf8(),
        // not by 1 per char, or it stops matching valid byte offsets
        // into the original &str as soon as non-ASCII input appears.
        let mut s = Scanner::new("éa");
        assert_eq!(s.bump(), Some('é'));
        assert_eq!(s.position(), 2);
        assert_eq!(s.bump(), Some('a'));
        assert_eq!(s.position(), 3);
    }
}
