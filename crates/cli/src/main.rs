use crate::app::{App, RunOutcome};

mod app;
mod args;
mod colorizer;
mod error;
mod gitignore;
mod output;
mod pool;
mod walk;

fn main() {
    let mut app = match App::new() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    match app.run() {
        Ok(RunOutcome::Matched) => {}
        Ok(RunOutcome::NoMatches) => std::process::exit(1),
        Ok(RunOutcome::HadError) => std::process::exit(2),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    }
}
