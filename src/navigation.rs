use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::state::{AppState, AppView};

pub struct Navigator;

impl Navigator {
    pub async fn handle_key_event(
        state: &mut AppState,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<()> {
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
        const PAGE_SIZE: usize = 10;
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
        const PAGE_SIZE: usize = 10;
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
                if let Some(pool) = state.data_manager.pools.get(state.selected_pool_index) {
                    let pool_name = pool.name.clone();
                    state.current_view = AppView::DatasetView(pool_name.clone());
                    state.selected_dataset_index = 0;
                    state.data_manager.load_datasets(&pool_name).await?;
                    state.sort_manager.sort_datasets(&mut state.data_manager.datasets);
                    state.reset_dataset_selection();
                }
            }
            AppView::DatasetView(pool_name) => {
                if let Some(dataset) = state.data_manager.datasets.get(state.selected_dataset_index) {
                    let dataset_name = dataset.name.clone();
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
}