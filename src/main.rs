mod app;
mod zfs;
mod ui;
mod state;
mod navigation;
mod data;
mod sorting;
mod theme;
mod config;
mod update;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

use app::App;
use config::{Config, Commands};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let config = Config::parse_args();

    // Handle update command before validating config or starting TUI
    if let Some(Commands::Update) = &config.command {
        return update::check_and_update().await;
    }

    // Validate configuration
    if let Err(e) = config.validate() {
        eprintln!("Configuration error: {}", e);
        std::process::exit(1);
    }

    let _guard = TerminalGuard;
    let mut terminal = setup_terminal()?;

    let mut app = App::new(config);
    let result = app.run(&mut terminal).await;

    terminal.show_cursor()?;

    result
}
