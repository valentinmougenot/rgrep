mod line_match;
mod matcher;
mod searcher;

pub use line_match::LineMatch;
pub use matcher::{LiteralMatcher, Matcher};
pub use searcher::search;
