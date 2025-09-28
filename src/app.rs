use anyhow::Result;
use crossterm::event::{self, Event};
use ratatui::{backend::Backend, Terminal};

use crate::{navigation::Navigator, config::Config};

pub struct App {
    state: crate::state::AppState,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            state: crate::state::AppState::new(config),
        }
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Load initial data
        self.state.data_manager.load_pools().await?;

        loop {
            // Check for timeout expiration
            if self.state.delete_confirmation_pending && self.state.is_delete_confirmation_expired() {
                self.state.clear_delete_confirmation();
            }

            // Draw UI first to ensure error messages are visible
            terminal.draw(|f| crate::ui::draw(f, &mut self.state))?;

            // Use timeout to allow periodic UI updates during background operations
            if event::poll(std::time::Duration::from_millis(100))?
                && let Event::Key(key) = event::read()? {
                    Navigator::handle_key_event(&mut self.state, key.code, key.modifiers).await?;
                }

            if self.state.should_quit {
                break;
            }
        }
        Ok(())
    }
}
