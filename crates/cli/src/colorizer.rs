use std::{
    io::{self, IsTerminal},
    path::Path,
};

pub struct Colorizer {
    enabled: bool,
}

impl Colorizer {
    pub fn from_stdout() -> Self {
        Self {
            enabled: io::stdout().is_terminal(),
        }
    }

    pub fn path(&self, text: &Path) -> String {
        self.wrap(&text.display().to_string(), "35")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_is_wrapped_in_magenta_when_enabled() {
        let colorizer = Colorizer { enabled: true };
        assert_eq!(
            colorizer.path(Path::new("file.txt")),
            "\x1b[35mfile.txt\x1b[0m"
        );
    }

    #[test]
    fn path_is_unchanged_when_disabled() {
        let colorizer = Colorizer { enabled: false };
        assert_eq!(colorizer.path(Path::new("file.txt")), "file.txt");
    }

    #[test]
    fn line_number_is_wrapped_in_green_when_enabled() {
        let colorizer = Colorizer { enabled: true };
        assert_eq!(colorizer.line_number(42), "\x1b[32m42\x1b[0m");
    }

    #[test]
    fn line_number_is_unchanged_when_disabled() {
        let colorizer = Colorizer { enabled: false };
        assert_eq!(colorizer.line_number(42), "42");
    }

    #[test]
    fn matched_is_wrapped_in_bold_red_when_enabled() {
        let colorizer = Colorizer { enabled: true };
        assert_eq!(colorizer.matched("hello"), "\x1b[1;31mhello\x1b[0m");
    }

    #[test]
    fn matched_is_unchanged_when_disabled() {
        let colorizer = Colorizer { enabled: false };
        assert_eq!(colorizer.matched("hello"), "hello");
    }
}
