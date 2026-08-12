pub struct LineMatch {
    pub line_number: usize,
    pub line: String,
    pub match_span: Option<std::ops::Range<usize>>,
    pub is_context: bool,
}
