use std::io::{self, IsTerminal};

pub struct Colorizer {
    enabled: bool,
}

impl Colorizer {
    pub fn from_stdout() -> Self {
        Self {
            enabled: io::stdout().is_terminal(),
        }
    }

    pub fn path(&self, text: &str) -> String {
        self.wrap(text, "35")
    }

    pub fn line_number(&self, line: usize) -> String {
        self.wrap(&line.to_string(), "32")
    }

    pub fn matched(&self, text: &str) -> String {
        self.wrap(text, "1;31")
    }

    fn wrap(&self, text: &str, code: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}
