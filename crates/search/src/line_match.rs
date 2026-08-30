use std::ops::Range;

pub struct LineMatch {
    pub line_number: usize,
    pub line: String,
    pub match_spans: Vec<Range<usize>>,
    pub is_context: bool,
}
