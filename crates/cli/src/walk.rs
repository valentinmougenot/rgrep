use std::{
    fs::ReadDir,
    io,
    path::{Path, PathBuf},
};

pub fn walk(
    root: &Path,
    should_skip: impl Fn(&Path, bool) -> bool,
) -> impl Iterator<Item = io::Result<PathBuf>> {
    Files {
        root: Some(root.to_path_buf()),
        stack: Vec::new(),
        should_skip,
    }
}

struct Files<F: Fn(&Path, bool) -> bool> {
    root: Option<PathBuf>,
    stack: Vec<ReadDir>,
    should_skip: F,
}

impl<F: Fn(&Path, bool) -> bool> Iterator for Files<F> {
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
                    Ok(m) if m.is_dir() && !(self.should_skip)(&entry.path(), true) => {
                        match std::fs::read_dir(entry.path()) {
                            Ok(value) => self.stack.push(value),
                            Err(e) => return Some(Err(e)),
                        }
                    }
                    Ok(m) if m.is_file() && !(self.should_skip)(&entry.path(), false) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("rgrep_walk_test_{name}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn file_names(root: &Path) -> Vec<String> {
        let mut names: Vec<String> = walk(root, |_, _| false)
            .map(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn finds_files_recursively() {
        let dir = TempDir::new("recursive");
        fs::create_dir_all(dir.path().join("a/b")).unwrap();
        fs::create_dir_all(dir.path().join("c")).unwrap();
        fs::write(dir.path().join("root.txt"), "").unwrap();
        fs::write(dir.path().join("a/a1.txt"), "").unwrap();
        fs::write(dir.path().join("a/b/b1.txt"), "").unwrap();
        fs::write(dir.path().join("c/c1.txt"), "").unwrap();

        assert_eq!(
            file_names(dir.path()),
            vec!["a1.txt", "b1.txt", "c1.txt", "root.txt"]
        );
    }

    #[test]
    fn skips_symlinks() {
        let dir = TempDir::new("symlinks");
        fs::create_dir_all(dir.path().join("real_dir")).unwrap();
        fs::write(dir.path().join("real_file.txt"), "").unwrap();
        std::os::unix::fs::symlink(dir.path().join("real_dir"), dir.path().join("link_to_dir"))
            .unwrap();
        std::os::unix::fs::symlink(
            dir.path().join("real_file.txt"),
            dir.path().join("link_to_file.txt"),
        )
        .unwrap();

        assert_eq!(file_names(dir.path()), vec!["real_file.txt"]);
    }

    #[test]
    fn returns_empty_for_an_empty_directory() {
        let dir = TempDir::new("empty");
        assert!(file_names(dir.path()).is_empty());
    }

    #[test]
    fn errors_when_root_is_not_a_directory() {
        let dir = TempDir::new("not_a_dir");
        let file = dir.path().join("file.txt");
        fs::write(&file, "").unwrap();

        let results: Vec<_> = walk(&file, |_, _| false).collect();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[test]
    fn errors_when_root_does_not_exist() {
        let dir = TempDir::new("missing");
        let missing = dir.path().join("does_not_exist");

        let results: Vec<_> = walk(&missing, |_, _| false).collect();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }
}
