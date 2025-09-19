use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::Backend,
    Terminal,
};

use crate::zfs::{Pool, Dataset, Snapshot};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use tokio::task;

#[derive(Debug, Clone)]
pub enum AppView {
    PoolList,
    DatasetView(String), // pool name
    SnapshotDetail(String, String), // pool name, dataset name
    Help,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SortOrder {
    TotalSizeDesc,
    TotalSizeAsc,
    DatasetSizeDesc,
    DatasetSizeAsc,
    SnapshotSizeDesc,
    SnapshotSizeAsc,
    NameDesc,
    NameAsc,
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub current_view: AppView,
    pub previous_view: Option<AppView>,
    pub theme: Theme,
    pub pools: Vec<Pool>,
    pub datasets: Vec<Dataset>,
    pub snapshots: Vec<Snapshot>,
    pub snapshot_cache: Arc<Mutex<HashMap<String, Vec<Snapshot>>>>, // Cache snapshots by dataset name
    pub prefetch_complete: Arc<AtomicBool>, // Track if background prefetch is done
    pub selected_pool_index: usize,
    pub selected_dataset_index: usize,
    pub selected_snapshot_index: usize,
    pub selected_theme_index: usize,
    pub dataset_sort_order: SortOrder,
    pub snapshot_sort_order: SortOrder,
    pub dataset_scroll_offset: usize,
    pub snapshot_scroll_offset: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            current_view: AppView::PoolList,
            previous_view: None,
            theme: Theme::Dark, // Default to dark theme
            pools: Vec::new(),
            datasets: Vec::new(),
            snapshots: Vec::new(),
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
            prefetch_complete: Arc::new(AtomicBool::new(false)),
            selected_pool_index: 0,
            selected_dataset_index: 0,
            selected_snapshot_index: 0,
            selected_theme_index: 0,
            dataset_sort_order: SortOrder::TotalSizeDesc,
            snapshot_sort_order: SortOrder::TotalSizeDesc,
            dataset_scroll_offset: 0,
            snapshot_scroll_offset: 0,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Load initial data
        self.load_pools().await?;

        loop {
            terminal.draw(|f| crate::ui::draw(f, self))?;

            if let Event::Key(key) = event::read()? {
                self.handle_key_event(key.code, key.modifiers).await?;
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    async fn handle_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        match &self.current_view {
            AppView::Help => {
                match key {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => self.should_quit = true,
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Left => self.go_back().await?,
                    KeyCode::Up => self.previous_theme(),
                    KeyCode::Down => self.next_theme(),
                    KeyCode::Enter | KeyCode::Right => self.select_theme(),
                    _ => {}
                }
            }
            _ => {
                match key {
                    KeyCode::Char('q') => self.should_quit = true,
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => self.should_quit = true,
                    KeyCode::Char('h') => self.show_help(),
                    KeyCode::Char('s') => self.toggle_sort(),
                    KeyCode::Esc | KeyCode::Backspace | KeyCode::Left => self.go_back().await?,
                    KeyCode::Enter | KeyCode::Right => self.go_forward().await?,
                    KeyCode::Up => self.previous_item(),
                    KeyCode::Down => self.next_item(),
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn previous_item(&mut self) {
        match self.current_view {
            AppView::PoolList => {
                if self.selected_pool_index > 0 {
                    self.selected_pool_index -= 1;
                }
            }
            AppView::DatasetView(_) => {
                if self.selected_dataset_index > 0 {
                    self.selected_dataset_index -= 1;
                }
            }
            AppView::SnapshotDetail(_, _) => {
                if self.selected_snapshot_index > 0 {
                    self.selected_snapshot_index -= 1;
                }
            }
            AppView::Help => {
                // Navigation handled by theme methods
            }
        }
    }

    fn next_item(&mut self) {
        match self.current_view {
            AppView::PoolList => {
                if self.selected_pool_index < self.pools.len().saturating_sub(1) {
                    self.selected_pool_index += 1;
                }
            }
            AppView::DatasetView(_) => {
                if self.selected_dataset_index < self.datasets.len().saturating_sub(1) {
                    self.selected_dataset_index += 1;
                }
            }
            AppView::SnapshotDetail(_, _) => {
                if self.selected_snapshot_index < self.snapshots.len().saturating_sub(1) {
                    self.selected_snapshot_index += 1;
                }
            }
            AppView::Help => {
                // Navigation handled by theme methods
            }
        }
    }

    async fn go_forward(&mut self) -> Result<()> {
        match &self.current_view {
            AppView::PoolList => {
                if let Some(pool) = self.pools.get(self.selected_pool_index) {
                    let pool_name = pool.name.clone();
                    self.current_view = AppView::DatasetView(pool_name.clone());
                    self.selected_dataset_index = 0;
                    self.load_datasets(&pool_name).await?;
                }
            }
            AppView::DatasetView(pool_name) => {
                if let Some(dataset) = self.datasets.get(self.selected_dataset_index) {
                    let dataset_name = dataset.name.clone();
                    self.current_view = AppView::SnapshotDetail(pool_name.clone(), dataset_name.clone());
                    self.selected_snapshot_index = 0;
                    self.load_snapshots(&dataset_name).await?;
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

    async fn go_back(&mut self) -> Result<()> {
        match &self.current_view {
            AppView::PoolList => {
                // Can't go back further
            }
            AppView::DatasetView(_) => {
                self.current_view = AppView::PoolList;
            }
            AppView::SnapshotDetail(pool_name, _) => {
                self.current_view = AppView::DatasetView(pool_name.clone());
            }
            AppView::Help => {
                if let Some(prev_view) = self.previous_view.take() {
                    self.current_view = prev_view;
                } else {
                    self.current_view = AppView::PoolList;
                }
            }
        }
        Ok(())
    }

    async fn load_pools(&mut self) -> Result<()> {
        self.pools = crate::zfs::get_pools().await?;

        // Start background prefetch of all snapshots (non-blocking)
        self.start_background_prefetch();

        Ok(())
    }

    fn start_background_prefetch(&mut self) {
        let pools = self.pools.clone();
        let cache = Arc::clone(&self.snapshot_cache);
        let prefetch_complete = Arc::clone(&self.prefetch_complete);

        task::spawn(async move {
            // Get all datasets from all pools
            let mut all_datasets = Vec::new();

            for pool in &pools {
                match crate::zfs::get_datasets(&pool.name).await {
                    Ok(datasets) => {
                        all_datasets.extend(datasets);
                    }
                    Err(_) => {
                        // Continue with other pools if one fails
                        continue;
                    }
                }
            }

            // Prefetch snapshots for each dataset
            for dataset in &all_datasets {
                match crate::zfs::get_snapshots(&dataset.name).await {
                    Ok(snapshots) => {
                        if let Ok(mut cache_lock) = cache.lock() {
                            cache_lock.insert(dataset.name.clone(), snapshots);
                        }
                    }
                    Err(_) => {
                        // Continue with other datasets if one fails
                        continue;
                    }
                }
            }

            // Signal completion
            prefetch_complete.store(true, Ordering::Relaxed);
        });
    }


    async fn load_datasets(&mut self, pool_name: &str) -> Result<()> {
        self.datasets = crate::zfs::get_datasets(pool_name).await?;
        self.sort_datasets();
        self.selected_dataset_index = 0;
        self.dataset_scroll_offset = 0;
        Ok(())
    }

    async fn load_snapshots(&mut self, dataset_name: &str) -> Result<()> {
        // Try to get snapshots from cache first
        let cached_snapshots = {
            if let Ok(cache_lock) = self.snapshot_cache.lock() {
                cache_lock.get(dataset_name).cloned()
            } else {
                None
            }
        };

        if let Some(snapshots) = cached_snapshots {
            self.snapshots = snapshots;
        } else {
            // Fall back to fetching if not in cache
            self.snapshots = crate::zfs::get_snapshots(dataset_name).await?;
            // Cache the result for future use
            if let Ok(mut cache_lock) = self.snapshot_cache.lock() {
                cache_lock.insert(dataset_name.to_string(), self.snapshots.clone());
            }
        }

        self.sort_snapshots();
        self.selected_snapshot_index = 0;
        self.snapshot_scroll_offset = 0;
        Ok(())
    }

    fn show_help(&mut self) {
        self.previous_view = Some(self.current_view.clone());
        self.current_view = AppView::Help;
        self.selected_theme_index = match self.theme {
            Theme::Dark => 0,
            Theme::Light => 1,
        };
    }

    fn previous_theme(&mut self) {
        if self.selected_theme_index > 0 {
            self.selected_theme_index -= 1;
        }
    }

    fn next_theme(&mut self) {
        if self.selected_theme_index < 1 { // We have 2 themes (0-1)
            self.selected_theme_index += 1;
        }
    }

    fn select_theme(&mut self) {
        self.theme = match self.selected_theme_index {
            0 => Theme::Dark,
            1 => Theme::Light,
            _ => Theme::Light,
        };
    }

    pub fn get_theme_colors(&self) -> ThemeColors {
        match self.theme {
            Theme::Dark => ThemeColors {
                background: ratatui::style::Color::Black,
                text: ratatui::style::Color::White,
                accent: ratatui::style::Color::Cyan,
                highlight: ratatui::style::Color::Blue,
                border: ratatui::style::Color::Gray,
                selected: ratatui::style::Color::Yellow,
                warning: ratatui::style::Color::Red,
            },
            Theme::Light => ThemeColors {
                background: ratatui::style::Color::White,
                text: ratatui::style::Color::Black,
                accent: ratatui::style::Color::Blue,
                highlight: ratatui::style::Color::LightBlue,
                border: ratatui::style::Color::DarkGray,
                selected: ratatui::style::Color::Magenta,
                warning: ratatui::style::Color::Red,
            },
        }
    }

    fn toggle_sort(&mut self) {
        match &self.current_view {
            AppView::DatasetView(_) => {
                self.dataset_sort_order = match self.dataset_sort_order {
                    SortOrder::TotalSizeDesc => SortOrder::TotalSizeAsc,
                    SortOrder::TotalSizeAsc => SortOrder::DatasetSizeDesc,
                    SortOrder::DatasetSizeDesc => SortOrder::DatasetSizeAsc,
                    SortOrder::DatasetSizeAsc => SortOrder::SnapshotSizeDesc,
                    SortOrder::SnapshotSizeDesc => SortOrder::SnapshotSizeAsc,
                    SortOrder::SnapshotSizeAsc => SortOrder::NameDesc,
                    SortOrder::NameDesc => SortOrder::NameAsc,
                    SortOrder::NameAsc => SortOrder::TotalSizeDesc,
                };
                self.sort_datasets();
                self.selected_dataset_index = 0;
                self.dataset_scroll_offset = 0;
            }
            AppView::SnapshotDetail(_, _) => {
                self.snapshot_sort_order = match self.snapshot_sort_order {
                    SortOrder::TotalSizeDesc => SortOrder::TotalSizeAsc,
                    SortOrder::TotalSizeAsc => SortOrder::DatasetSizeDesc,
                    SortOrder::DatasetSizeDesc => SortOrder::DatasetSizeAsc,
                    SortOrder::DatasetSizeAsc => SortOrder::SnapshotSizeDesc,
                    SortOrder::SnapshotSizeDesc => SortOrder::SnapshotSizeAsc,
                    SortOrder::SnapshotSizeAsc => SortOrder::NameDesc,
                    SortOrder::NameDesc => SortOrder::NameAsc,
                    SortOrder::NameAsc => SortOrder::TotalSizeDesc,
                };
                self.sort_snapshots();
                self.selected_snapshot_index = 0;
                self.snapshot_scroll_offset = 0;
            }
            _ => {} // No sorting for pool list or help
        }
    }

    fn sort_datasets(&mut self) {
        match self.dataset_sort_order {
            SortOrder::TotalSizeDesc => self.datasets.sort_by(|a, b| (b.referenced + b.snapshot_used).cmp(&(a.referenced + a.snapshot_used))),
            SortOrder::TotalSizeAsc => self.datasets.sort_by(|a, b| (a.referenced + a.snapshot_used).cmp(&(b.referenced + b.snapshot_used))),
            SortOrder::DatasetSizeDesc => self.datasets.sort_by(|a, b| b.referenced.cmp(&a.referenced)),
            SortOrder::DatasetSizeAsc => self.datasets.sort_by(|a, b| a.referenced.cmp(&b.referenced)),
            SortOrder::SnapshotSizeDesc => self.datasets.sort_by(|a, b| b.snapshot_used.cmp(&a.snapshot_used)),
            SortOrder::SnapshotSizeAsc => self.datasets.sort_by(|a, b| a.snapshot_used.cmp(&b.snapshot_used)),
            SortOrder::NameDesc => self.datasets.sort_by(|a, b| b.name.cmp(&a.name)),
            SortOrder::NameAsc => self.datasets.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }

    fn sort_snapshots(&mut self) {
        match self.snapshot_sort_order {
            SortOrder::TotalSizeDesc => self.snapshots.sort_by(|a, b| b.used.cmp(&a.used)),
            SortOrder::TotalSizeAsc => self.snapshots.sort_by(|a, b| a.used.cmp(&b.used)),
            SortOrder::DatasetSizeDesc => self.snapshots.sort_by(|a, b| b.referenced.cmp(&a.referenced)),
            SortOrder::DatasetSizeAsc => self.snapshots.sort_by(|a, b| a.referenced.cmp(&b.referenced)),
            SortOrder::SnapshotSizeDesc => self.snapshots.sort_by(|a, b| b.used.cmp(&a.used)), // For snapshots, used is the snapshot size
            SortOrder::SnapshotSizeAsc => self.snapshots.sort_by(|a, b| a.used.cmp(&b.used)),
            SortOrder::NameDesc => self.snapshots.sort_by(|a, b| b.name.cmp(&a.name)),
            SortOrder::NameAsc => self.snapshots.sort_by(|a, b| a.name.cmp(&b.name)),
        }
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
                let total_items = self.datasets.len();
                if self.selected_dataset_index >= self.dataset_scroll_offset + visible_height {
                    self.dataset_scroll_offset = self.selected_dataset_index.saturating_sub(visible_height - 1);
                } else if self.selected_dataset_index < self.dataset_scroll_offset {
                    self.dataset_scroll_offset = self.selected_dataset_index;
                }
                if self.dataset_scroll_offset + visible_height > total_items && total_items > visible_height {
                    self.dataset_scroll_offset = total_items.saturating_sub(visible_height);
                }
            }
            AppView::SnapshotDetail(_, _) => {
                let total_items = self.snapshots.len();
                if self.selected_snapshot_index >= self.snapshot_scroll_offset + visible_height {
                    self.snapshot_scroll_offset = self.selected_snapshot_index.saturating_sub(visible_height - 1);
                } else if self.selected_snapshot_index < self.snapshot_scroll_offset {
                    self.snapshot_scroll_offset = self.selected_snapshot_index;
                }
                if self.snapshot_scroll_offset + visible_height > total_items && total_items > visible_height {
                    self.snapshot_scroll_offset = total_items.saturating_sub(visible_height);
                }
            }
            _ => {}
        }
    }

    pub fn is_prefetch_complete(&self) -> bool {
        self.prefetch_complete.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub background: ratatui::style::Color,
    pub text: ratatui::style::Color,
    pub accent: ratatui::style::Color,
    pub highlight: ratatui::style::Color,
    pub border: ratatui::style::Color,
    pub selected: ratatui::style::Color,
    pub warning: ratatui::style::Color,
}