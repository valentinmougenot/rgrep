use std::{
    fs::File,
    io::BufReader,
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
};

use search::{LineMatch, Matcher, search};

use crate::error::{AppError, AppResult};

pub struct FileResult {
    pub path: PathBuf,
    pub matches: AppResult<Vec<LineMatch>>,
}

pub fn search_files(
    paths: mpsc::Receiver<PathBuf>,
    matcher: Arc<dyn Matcher>,
    invert_match: bool,
    before_context: usize,
    after_context: usize,
    threads_count: Option<usize>,
) -> (mpsc::Receiver<FileResult>, Vec<JoinHandle<()>>) {
    let paths = Arc::new(Mutex::new(paths));

    let threads_available = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let threads_count = threads_count
        .unwrap_or(threads_available)
        .min(threads_available)
        .max(1);

    let (result_tx, result_rx) = mpsc::channel::<FileResult>();

    let mut handles = Vec::new();
    for _ in 0..threads_count {
        let paths = Arc::clone(&paths);
        let matcher = Arc::clone(&matcher);
        let result_tx = result_tx.clone();

        let handle = thread::spawn(move || {
            loop {
                let path = {
                    let guard = paths.lock().unwrap();
                    guard.recv()
                };
                let Ok(path) = path else { break };

                let matches = File::open(&path)
                    .map(BufReader::new)
                    .map_err(AppError::from)
                    .map(|reader| -> _ {
                        search(
                            matcher.as_ref(),
                            reader,
                            invert_match,
                            before_context,
                            after_context,
                        )
                        .collect()
                    });

                if result_tx.send(FileResult { path, matches }).is_err() {
                    break;
                }
            }
        });
        handles.push(handle);
    }

    return (result_rx, handles);
}

#[cfg(test)]
mod tests {
    use super::*;
    use search::LiteralMatcher;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("rgrep_pool_test_{name}_{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, name: &str, contents: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, contents).unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn dummy_matcher() -> Arc<dyn Matcher> {
        Arc::new(LiteralMatcher::new("x", false, false))
    }

    #[test]
    fn search_files_returns_a_result_for_every_input_path_regardless_of_completion_order() {
        let dir = TempDir::new("multi_file");
        let a = dir.write("a.txt", "no match here\n");
        let b = dir.write("b.txt", "a cat sat\n");
        let c = dir.write("c.txt", "another cat\n");

        let matcher: Arc<dyn Matcher> = Arc::new(LiteralMatcher::new("cat", false, false));
        let (tx, rx) = mpsc::channel();
        for path in [&a, &b, &c] {
            tx.send(path.clone()).unwrap();
        }
        drop(tx);

        let (results, handles) = search_files(rx, matcher, false, 0, 0, Some(2));

        let mut results: Vec<FileResult> = results.into_iter().collect();
        results.sort_by(|x, y| x.path.cmp(&y.path));

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].path, a);
        assert!(results[0].matches.as_ref().unwrap().is_empty());
        assert_eq!(results[1].path, b);
        assert_eq!(results[1].matches.as_ref().unwrap().len(), 1);
        assert_eq!(results[2].path, c);
        assert_eq!(results[2].matches.as_ref().unwrap().len(), 1);

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn search_files_reports_an_io_error_for_a_path_that_cannot_be_opened() {
        let dir = TempDir::new("missing_file");
        let missing = dir.0.join("does_not_exist.txt");

        let matcher: Arc<dyn Matcher> = Arc::new(LiteralMatcher::new("x", false, false));
        let (tx, rx) = mpsc::channel();
        tx.send(missing.clone()).unwrap();
        drop(tx);

        let (results, handles) = search_files(rx, matcher, false, 0, 0, Some(1));
        let results: Vec<FileResult> = results.into_iter().collect();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, missing);
        assert!(results[0].matches.is_err());

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn search_files_falls_back_to_available_parallelism_when_thread_count_is_none() {
        let expected = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let (tx, rx) = mpsc::channel();
        drop(tx);
        let (_results, handles) = search_files(rx, dummy_matcher(), false, 0, 0, None);

        assert_eq!(handles.len(), expected);
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn search_files_uses_the_explicit_thread_count_when_it_fits_within_availability() {
        let (tx, rx) = mpsc::channel();
        drop(tx);
        let (_results, handles) = search_files(rx, dummy_matcher(), false, 0, 0, Some(1));

        assert_eq!(handles.len(), 1);
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn search_files_clamps_an_explicit_thread_count_of_zero_up_to_one() {
        let (tx, rx) = mpsc::channel();
        drop(tx);
        let (_results, handles) = search_files(rx, dummy_matcher(), false, 0, 0, Some(0));

        assert_eq!(handles.len(), 1);
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn search_files_clamps_an_explicit_thread_count_above_availability() {
        let available = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        let (tx, rx) = mpsc::channel();
        drop(tx);
        let (_results, handles) = search_files(rx, dummy_matcher(), false, 0, 0, Some(available + 1000));

        assert_eq!(handles.len(), available);
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
