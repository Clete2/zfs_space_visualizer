use crate::{
    data::DataManager,
    sorting::SortManager,
    theme::ThemeManager,
};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum AppView {
    PoolList,
    DatasetView(String), // pool name
    SnapshotDetail(String, String), // pool name, dataset name
    Help,
}

pub struct AppState {
    pub should_quit: bool,
    pub current_view: AppView,
    pub previous_view: Option<AppView>,

    // Selection indices
    pub selected_pool_index: usize,
    pub selected_dataset_index: usize,
    pub selected_snapshot_index: usize,

    // Scroll offsets
    pub dataset_scroll_offset: usize,
    pub snapshot_scroll_offset: usize,

    // Component managers
    pub data_manager: DataManager,
    pub sort_manager: SortManager,
    pub theme_manager: ThemeManager,

    // Deletion confirmation state
    pub delete_confirmation_pending: bool,
    pub delete_confirmation_timestamp: Option<Instant>,

    // Error state
    pub error_message: Option<String>,

    // Cached status text
    pub status_help_text: String,
    pub status_help_color: ratatui::style::Color,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            should_quit: false,
            current_view: AppView::PoolList,
            previous_view: None,
            selected_pool_index: 0,
            selected_dataset_index: 0,
            selected_snapshot_index: 0,
            dataset_scroll_offset: 0,
            snapshot_scroll_offset: 0,
            data_manager: DataManager::new(),
            sort_manager: SortManager::new(),
            theme_manager: ThemeManager::new(),
            delete_confirmation_pending: false,
            delete_confirmation_timestamp: None,
            error_message: None,
            status_help_text: "↑/↓: Navigate | PgUp/PgDn: Page | d: Delete | s: Sort | ←/Esc: Back | h: Help | q: Quit".to_string(),
            status_help_color: ratatui::style::Color::Reset,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_visible_range(&self, total_items: usize, visible_height: usize) -> (usize, usize) {
        let scroll_offset = match &self.current_view {
            AppView::DatasetView(_) => self.dataset_scroll_offset,
            AppView::SnapshotDetail(_, _) => self.snapshot_scroll_offset,
            _ => 0,
        };

        let start = scroll_offset;
        let end = (start + visible_height).min(total_items);
        (start, end)
    }

    pub fn update_scroll(&mut self, visible_height: usize) {
        match &self.current_view {
            AppView::DatasetView(_) => {
                let total_items = self.data_manager.datasets.len();
                if total_items <= visible_height {
                    // All items fit on screen, no scrolling needed
                    self.dataset_scroll_offset = 0;
                } else {
                    // Calculate the maximum possible scroll offset
                    let max_scroll = total_items.saturating_sub(visible_height);

                    // Ensure selected item is visible
                    if self.selected_dataset_index < self.dataset_scroll_offset {
                        // Selected item is above visible area, scroll up
                        self.dataset_scroll_offset = self.selected_dataset_index;
                    } else if self.selected_dataset_index >= self.dataset_scroll_offset + visible_height {
                        // Selected item is below visible area, scroll down to show it
                        self.dataset_scroll_offset = (self.selected_dataset_index + 1).saturating_sub(visible_height);
                    }

                    // Ensure we don't scroll past the end
                    self.dataset_scroll_offset = self.dataset_scroll_offset.min(max_scroll);
                }
            }
            AppView::SnapshotDetail(_, _) => {
                let total_items = self.data_manager.snapshots.len();
                if total_items <= visible_height {
                    // All items fit on screen, no scrolling needed
                    self.snapshot_scroll_offset = 0;
                } else {
                    // Calculate the maximum possible scroll offset
                    let max_scroll = total_items.saturating_sub(visible_height);

                    // Ensure selected item is visible
                    if self.selected_snapshot_index < self.snapshot_scroll_offset {
                        // Selected item is above visible area, scroll up
                        self.snapshot_scroll_offset = self.selected_snapshot_index;
                    } else if self.selected_snapshot_index >= self.snapshot_scroll_offset + visible_height {
                        // Selected item is below visible area, scroll down to show it
                        self.snapshot_scroll_offset = (self.selected_snapshot_index + 1).saturating_sub(visible_height);
                    }

                    // Ensure we don't scroll past the end
                    self.snapshot_scroll_offset = self.snapshot_scroll_offset.min(max_scroll);
                }
            }
            _ => {}
        }
    }

    pub fn reset_dataset_selection(&mut self) {
        self.selected_dataset_index = 0;
        self.dataset_scroll_offset = 0;
    }

    pub fn reset_snapshot_selection(&mut self) {
        self.selected_snapshot_index = 0;
        self.snapshot_scroll_offset = 0;
    }

    pub fn start_delete_confirmation(&mut self) {
        self.delete_confirmation_pending = true;
        self.delete_confirmation_timestamp = Some(Instant::now());
        self.update_status_help_text();
    }

    pub fn clear_delete_confirmation(&mut self) {
        self.delete_confirmation_pending = false;
        self.delete_confirmation_timestamp = None;
        self.update_status_help_text();
    }

    pub fn is_delete_confirmation_expired(&self) -> bool {
        if let Some(timestamp) = self.delete_confirmation_timestamp {
            timestamp.elapsed().as_secs() >= crate::navigation::DELETE_CONFIRMATION_TIMEOUT_SECS
        } else {
            false
        }
    }

    pub fn set_error(&mut self, message: String) {
        self.error_message = Some(message);
        self.update_status_help_text();
    }

    pub fn clear_error(&mut self) {
        self.error_message = None;
        self.update_status_help_text();
    }

    pub fn update_status_help_text(&mut self) {
        // Check for error first
        if let Some(error) = &self.error_message {
            self.status_help_text = format!("ERROR: {} (Press any key to continue)", error);
            self.status_help_color = ratatui::style::Color::Red;
            return;
        }

        // Check for delete confirmation (only in snapshot view)
        if self.delete_confirmation_pending {
            if let crate::state::AppView::SnapshotDetail(_, _) = &self.current_view {
                if let Some(snapshot) = self.data_manager.snapshots.get(self.selected_snapshot_index) {
                    let short_name = snapshot.name.split('@').next_back().unwrap_or(&snapshot.name);
                    self.status_help_text = format!("⚠️  DELETE {}: Press 'd' again to CONFIRM", short_name);
                    self.status_help_color = ratatui::style::Color::Yellow;
                } else {
                    self.status_help_text = "⚠️  Press 'd' again to CONFIRM DELETION".to_string();
                    self.status_help_color = ratatui::style::Color::Yellow;
                }
            } else {
                self.status_help_text = "↑/↓: Navigate | PgUp/PgDn: Page | d: Delete | s: Sort | ←/Esc: Back | h: Help | q: Quit".to_string();
                self.status_help_color = ratatui::style::Color::Reset;
            }
        } else {
            self.status_help_text = "↑/↓: Navigate | PgUp/PgDn: Page | d: Delete | s: Sort | ←/Esc: Back | h: Help | q: Quit".to_string();
            self.status_help_color = ratatui::style::Color::Reset;
        }
    }
}