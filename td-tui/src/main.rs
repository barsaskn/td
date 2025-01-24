#![warn(
    clippy::semicolon_if_nothing_returned,
    clippy::use_self,
    clippy::cloned_instead_of_copied
)]

mod keybinds;
mod ui;
mod utils;

use std::{error::Error, path::PathBuf, fs};

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use ui::AppState;

fn main() {
    let path: PathBuf;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        let name = std::env::args()
            .next()
            .expect("There should always be 1 item");
        let home_dir = dirs::home_dir().expect("Failed to find home directory");    
        path = home_dir.join(".td.json");
    }
    else {
        path = PathBuf::from(args[0].clone());
    }
    let app = match AppState::create(path) {
        Ok(app) => app,
        Err(e) => {
            println!("Error while loading database: {e}");
            return;
        }
    };

    if let Err(e) = run_app(app) {
        println!("Error while running app: {e}");
    }
}

fn run_app(mut app: AppState) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();

    // NOTE: could enable mouse capture here
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    app.run_loop(&mut terminal)?;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
