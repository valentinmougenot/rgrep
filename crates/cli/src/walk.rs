use std::{
    fs::ReadDir,
    io,
    path::{Path, PathBuf},
};

pub fn walk(root: &Path) -> impl Iterator<Item = io::Result<PathBuf>> {
    Files {
        root: Some(root.to_path_buf()),
        stack: Vec::new(),
    }
}

struct Files {
    root: Option<PathBuf>,
    stack: Vec<ReadDir>,
}

impl Iterator for Files {
    type Item = io::Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(root) = self.root.take() {
            match std::fs::read_dir(root) {
                Ok(value) => self.stack = vec![value],
                Err(e) => return Some(Err(e)),
            }
        }

        loop {
            let current = self.stack.last_mut()?;
            match current.next() {
                Some(Ok(entry)) => match entry.metadata() {
                    Ok(m) if m.is_dir() => match std::fs::read_dir(entry.path()) {
                        Ok(value) => self.stack.push(value),
                        Err(e) => return Some(Err(e)),
                    },
                    Ok(m) if m.is_file() => {
                        return Some(Ok(entry.path()));
                    }
                    Ok(_) => {}
                    Err(e) => return Some(Err(e)),
                },
                Some(Err(e)) => return Some(Err(e)),
                None => {
                    self.stack.pop();
                }
            }
        }
    }
}
