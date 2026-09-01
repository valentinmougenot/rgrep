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
