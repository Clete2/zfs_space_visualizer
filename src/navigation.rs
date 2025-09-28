use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::state::{AppState, AppView};

const PAGE_SIZE: usize = 10;
pub const DELETE_CONFIRMATION_TIMEOUT_SECS: u64 = 3;

pub struct Navigator;

impl Navigator {
    pub async fn handle_key_event(
        state: &mut AppState,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<()> {
        // Clear any error message on any key press (but not during delete operations)
        let clearing_error = state.error_message.is_some();
        if clearing_error {
            state.clear_error();
            // If we're just clearing an error, don't process other key actions
            return Ok(());
        }
        match &state.current_view {
            AppView::Help => {
                match key {
                    KeyCode::Char('q') => state.should_quit = true,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => state.should_quit = true,
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Left => Self::go_back(state).await?,
                    KeyCode::Up => state.theme_manager.previous_theme(),
                    KeyCode::Down => state.theme_manager.next_theme(),
                    KeyCode::Enter | KeyCode::Right => state.theme_manager.select_theme(),
                    _ => {}
                }
            }
            _ => {
                match key {
                    KeyCode::Char('q') => state.should_quit = true,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => state.should_quit = true,
                    KeyCode::Char('h') => Self::show_help(state),
                    KeyCode::Char('s') => Self::toggle_sort(state),
                    KeyCode::Char('d') if !state.config.readonly => Self::handle_delete_key(state).await?,
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Left => Self::go_back(state).await?,
                    KeyCode::Enter | KeyCode::Right => Self::go_forward(state).await?,
                    KeyCode::Up => Self::previous_item(state),
                    KeyCode::Down => Self::next_item(state),
                    KeyCode::PageUp => Self::page_up(state),
                    KeyCode::PageDown => Self::page_down(state),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn previous_item(state: &mut AppState) {
        match &state.current_view {
            AppView::PoolList => state.selected_pool_index = state.selected_pool_index.saturating_sub(1),
            AppView::DatasetView(_) => state.selected_dataset_index = state.selected_dataset_index.saturating_sub(1),
            AppView::SnapshotDetail(_, _) => state.selected_snapshot_index = state.selected_snapshot_index.saturating_sub(1),
            AppView::Help => {}
        }
    }

    fn next_item(state: &mut AppState) {
        match &state.current_view {
            AppView::PoolList => {
                state.selected_pool_index = (state.selected_pool_index + 1).min(state.data_manager.pools.len().saturating_sub(1));
            }
            AppView::DatasetView(_) => {
                state.selected_dataset_index = (state.selected_dataset_index + 1).min(state.data_manager.datasets.len().saturating_sub(1));
            }
            AppView::SnapshotDetail(_, _) => {
                state.selected_snapshot_index = (state.selected_snapshot_index + 1).min(state.data_manager.snapshots.len().saturating_sub(1));
            }
            AppView::Help => {}
        }
    }

    fn page_up(state: &mut AppState) {
        match &state.current_view {
            AppView::PoolList => {
                state.selected_pool_index = state.selected_pool_index.saturating_sub(PAGE_SIZE);
            }
            AppView::DatasetView(_) => {
                state.selected_dataset_index = state.selected_dataset_index.saturating_sub(PAGE_SIZE);
            }
            AppView::SnapshotDetail(_, _) => {
                state.selected_snapshot_index = state.selected_snapshot_index.saturating_sub(PAGE_SIZE);
            }
            AppView::Help => {}
        }
    }

    fn page_down(state: &mut AppState) {
        match &state.current_view {
            AppView::PoolList => {
                state.selected_pool_index = (state.selected_pool_index + PAGE_SIZE).min(state.data_manager.pools.len().saturating_sub(1));
            }
            AppView::DatasetView(_) => {
                state.selected_dataset_index = (state.selected_dataset_index + PAGE_SIZE).min(state.data_manager.datasets.len().saturating_sub(1));
            }
            AppView::SnapshotDetail(_, _) => {
                state.selected_snapshot_index = (state.selected_snapshot_index + PAGE_SIZE).min(state.data_manager.snapshots.len().saturating_sub(1));
            }
            AppView::Help => {}
        }
    }

    async fn go_forward(state: &mut AppState) -> Result<()> {
        match &state.current_view {
            AppView::PoolList => {
                if let Some(pool_name) = state.data_manager.pools.get(state.selected_pool_index).map(|p| p.name.clone()) {
                    state.current_view = AppView::DatasetView(pool_name.clone());
                    state.selected_dataset_index = 0;
                    state.data_manager.load_datasets(&pool_name).await?;
                    state.sort_manager.sort_datasets(&mut state.data_manager.datasets);
                    state.reset_dataset_selection();
                }
            }
            AppView::DatasetView(pool_name) => {
                if let Some(dataset_name) = state.data_manager.datasets.get(state.selected_dataset_index).map(|d| d.name.clone()) {
                    state.current_view = AppView::SnapshotDetail(pool_name.clone(), dataset_name.clone());
                    state.selected_snapshot_index = 0;
                    state.data_manager.load_snapshots(&dataset_name).await?;
                    state.sort_manager.sort_snapshots(&mut state.data_manager.snapshots);
                    state.reset_snapshot_selection();
                }
            }
            AppView::SnapshotDetail(_, _) => {
                // No further navigation
            }
            AppView::Help => {
                // No forward navigation from help
            }
        }
        Ok(())
    }

    async fn go_back(state: &mut AppState) -> Result<()> {
        match &state.current_view {
            AppView::PoolList => {
                // Can't go back further
            }
            AppView::DatasetView(_) => {
                state.current_view = AppView::PoolList;
            }
            AppView::SnapshotDetail(pool_name, _) => {
                state.current_view = AppView::DatasetView(pool_name.clone());
            }
            AppView::Help => {
                if let Some(prev_view) = state.previous_view.take() {
                    state.current_view = prev_view;
                } else {
                    state.current_view = AppView::PoolList;
                }
            }
        }
        Ok(())
    }

    fn show_help(state: &mut AppState) {
        state.previous_view = Some(state.current_view.clone());
        state.current_view = AppView::Help;
        state.theme_manager.set_selected_index_from_theme();
    }

    fn toggle_sort(state: &mut AppState) {
        match &state.current_view {
            AppView::DatasetView(_) => {
                state.sort_manager.toggle_dataset_sort();
                state.sort_manager.sort_datasets(&mut state.data_manager.datasets);
                state.reset_dataset_selection();
            }
            AppView::SnapshotDetail(_, _) => {
                state.sort_manager.toggle_snapshot_sort();
                state.sort_manager.sort_snapshots(&mut state.data_manager.snapshots);
                state.reset_snapshot_selection();
            }
            _ => {}
        }
    }

    async fn handle_delete_key(state: &mut AppState) -> Result<()> {
        // Only allow deletion in snapshot view
        let AppView::SnapshotDetail(_pool_name, dataset_name) = &state.current_view else {
            return Ok(());
        };

        // If no snapshots exist, do nothing
        if state.data_manager.snapshots.is_empty() {
            return Ok(());
        }

        if !state.delete_confirmation_pending {
            // First 'd' press - start confirmation
            state.start_delete_confirmation();
            return Ok(());
        }

        // Second 'd' press - execute deletion
        let Some(snapshot) = state.data_manager.snapshots.get(state.selected_snapshot_index) else {
            state.clear_delete_confirmation();
            return Ok(());
        };
        match crate::zfs::delete_snapshot(&snapshot.name).await {
            Ok(()) => {
                // Force reload snapshots from ZFS after deletion
                state.data_manager.reload_snapshots(dataset_name).await?;
                state.sort_manager.sort_snapshots(&mut state.data_manager.snapshots);

                // Adjust selection if we deleted the last item
                if state.selected_snapshot_index >= state.data_manager.snapshots.len() {
                    state.selected_snapshot_index = state.data_manager.snapshots.len().saturating_sub(1);
                }
            }
            Err(e) => {
                // Extract a user-friendly error message
                let error_msg = if e.to_string().contains("permission denied") {
                    "Permission denied. Try running with elevated privileges (sudo).".to_string()
                } else if e.to_string().contains("dataset does not exist") {
                    "Snapshot no longer exists.".to_string()
                } else if e.to_string().contains("dataset is busy") {
                    "Snapshot is currently in use and cannot be deleted.".to_string()
                } else {
                    format!("Failed to delete snapshot: {}", e)
                };
                state.set_error(error_msg);
            }
        }

        state.clear_delete_confirmation();
        Ok(())
    }
}