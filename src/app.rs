use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::Backend, Terminal};

use crate::navigation::Navigator;

pub struct App {
    state: crate::state::AppState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            state: crate::state::AppState::new(),
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Load initial data
        self.state.data_manager.load_pools().await?;

        loop {
            terminal.draw(|f| crate::ui::draw(f, &mut self.state))?;

            // Use timeout to allow periodic UI updates during background operations
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    Navigator::handle_key_event(&mut self.state, key.code, key.modifiers).await?;
                }
            }

            if self.state.should_quit {
                break;
            }
        }
        Ok(())
    }
}
