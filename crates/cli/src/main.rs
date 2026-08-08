use crate::app::App;

mod app;
mod args;
mod colorizer;
mod error;
mod gitignore;
mod output;
mod walk;

fn main() {
    let app = match App::new() {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    match app.run() {
        Ok(false) => {}
        Ok(true) => std::process::exit(2),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
