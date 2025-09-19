use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use futures::future;
use ratatui::{backend::Backend, style::Color, Terminal};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}},
};
use tokio::task;

use crate::zfs::{Pool, Dataset, Snapshot};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatasetSortOrder {
    TotalSizeDesc,
    TotalSizeAsc,
    DatasetSizeDesc,
    DatasetSizeAsc,
    SnapshotSizeDesc,
    SnapshotSizeAsc,
    NameDesc,
    NameAsc,
}

impl DatasetSortOrder {
    const VALUES: [Self; 8] = [
        Self::TotalSizeDesc, Self::TotalSizeAsc, Self::DatasetSizeDesc, Self::DatasetSizeAsc,
        Self::SnapshotSizeDesc, Self::SnapshotSizeAsc, Self::NameDesc, Self::NameAsc,
    ];

    pub const fn next(self) -> Self {
        let current_idx = match self {
            Self::TotalSizeDesc => 0,
            Self::TotalSizeAsc => 1,
            Self::DatasetSizeDesc => 2,
            Self::DatasetSizeAsc => 3,
            Self::SnapshotSizeDesc => 4,
            Self::SnapshotSizeAsc => 5,
            Self::NameDesc => 6,
            Self::NameAsc => 7,
        };
        Self::VALUES[(current_idx + 1) % Self::VALUES.len()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SnapshotSortOrder {
    UsedDesc,
    UsedAsc,
    ReferencedDesc,
    ReferencedAsc,
    NameDesc,
    NameAsc,
}

impl SnapshotSortOrder {
    const VALUES: [Self; 6] = [
        Self::UsedDesc, Self::UsedAsc, Self::ReferencedDesc,
        Self::ReferencedAsc, Self::NameDesc, Self::NameAsc,
    ];

    pub const fn next(self) -> Self {
        let current_idx = match self {
            Self::UsedDesc => 0,
            Self::UsedAsc => 1,
            Self::ReferencedDesc => 2,
            Self::ReferencedAsc => 3,
            Self::NameDesc => 4,
            Self::NameAsc => 5,
        };
        Self::VALUES[(current_idx + 1) % Self::VALUES.len()]
    }
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
    pub prefetch_total: Arc<std::sync::atomic::AtomicUsize>, // Total datasets to process
    pub prefetch_completed: Arc<std::sync::atomic::AtomicUsize>, // Completed datasets
    pub selected_pool_index: usize,
    pub selected_dataset_index: usize,
    pub selected_snapshot_index: usize,
    pub selected_theme_index: usize,
    pub dataset_sort_order: DatasetSortOrder,
    pub snapshot_sort_order: SnapshotSortOrder,
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
            prefetch_total: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            prefetch_completed: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            selected_pool_index: 0,
            selected_dataset_index: 0,
            selected_snapshot_index: 0,
            selected_theme_index: 0,
            dataset_sort_order: DatasetSortOrder::TotalSizeDesc,
            snapshot_sort_order: SnapshotSortOrder::UsedDesc,
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

            // Use timeout to allow periodic UI updates during background operations
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key_event(key.code, key.modifiers).await?;
                }
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
        match &self.current_view {
            AppView::PoolList => self.selected_pool_index = self.selected_pool_index.saturating_sub(1),
            AppView::DatasetView(_) => self.selected_dataset_index = self.selected_dataset_index.saturating_sub(1),
            AppView::SnapshotDetail(_, _) => self.selected_snapshot_index = self.selected_snapshot_index.saturating_sub(1),
            AppView::Help => {}
        }
    }

    fn next_item(&mut self) {
        match &self.current_view {
            AppView::PoolList => {
                self.selected_pool_index = (self.selected_pool_index + 1).min(self.pools.len().saturating_sub(1));
            }
            AppView::DatasetView(_) => {
                self.selected_dataset_index = (self.selected_dataset_index + 1).min(self.datasets.len().saturating_sub(1));
            }
            AppView::SnapshotDetail(_, _) => {
                self.selected_snapshot_index = (self.selected_snapshot_index + 1).min(self.snapshots.len().saturating_sub(1));
            }
            AppView::Help => {}
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
        let prefetch_total = Arc::clone(&self.prefetch_total);
        let prefetch_completed = Arc::clone(&self.prefetch_completed);

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

            // Set total count for progress tracking
            prefetch_total.store(all_datasets.len(), std::sync::atomic::Ordering::Relaxed);
            prefetch_completed.store(0, std::sync::atomic::Ordering::Relaxed);

            // Create semaphore to limit concurrent snapshot fetches
            // Default to 4x CPU count for optimal I/O concurrency
            let cpu_count = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            let max_concurrent = cpu_count * 8;
            let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

            // Prefetch snapshots for each dataset in parallel
            let tasks: Vec<_> = all_datasets
                .into_iter()
                .map(|dataset| {
                    let cache = Arc::clone(&cache);
                    let sem = Arc::clone(&semaphore);
                    let completed = Arc::clone(&prefetch_completed);

                    task::spawn(async move {
                        // Acquire semaphore permit to limit concurrency
                        let _permit = sem.acquire().await.ok()?;

                        let result = match crate::zfs::get_snapshots(&dataset.name).await {
                            Ok(snapshots) => {
                                if let Ok(mut cache_lock) = cache.lock() {
                                    cache_lock.insert(dataset.name.clone(), snapshots);
                                }
                                Some(())
                            }
                            Err(_) => {
                                // Continue with other datasets if one fails
                                None
                            }
                        };

                        // Increment completed count
                        completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        result
                    })
                })
                .collect();

            // Wait for all snapshot fetches to complete
            future::join_all(tasks).await;

            // Signal completion
            prefetch_complete.store(true, Ordering::Relaxed);
        });
    }


    async fn load_datasets(&mut self, pool_name: &str) -> Result<()> {
        self.datasets = crate::zfs::get_datasets(pool_name).await?;
        self.sort_datasets();
        self.reset_dataset_selection();
        Ok(())
    }

    async fn load_snapshots(&mut self, dataset_name: &str) -> Result<()> {
        self.snapshots = self.get_cached_snapshots(dataset_name).unwrap_or_default();

        if self.snapshots.is_empty() {
            self.snapshots = crate::zfs::get_snapshots(dataset_name).await?;
            self.cache_snapshots(dataset_name);
        }

        self.sort_snapshots();
        self.reset_snapshot_selection();
        Ok(())
    }

    fn get_cached_snapshots(&self, dataset_name: &str) -> Option<Vec<Snapshot>> {
        self.snapshot_cache
            .lock()
            .ok()?
            .get(dataset_name)
            .cloned()
    }

    fn cache_snapshots(&self, dataset_name: &str) {
        if let Ok(mut cache_lock) = self.snapshot_cache.lock() {
            cache_lock.insert(dataset_name.to_string(), self.snapshots.clone());
        }
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

    pub const fn get_theme_colors(&self) -> ThemeColors {
        match self.theme {
            Theme::Dark => ThemeColors {
                background: Color::Black,
                text: Color::White,
                accent: Color::Cyan,
                highlight: Color::Blue,
                border: Color::Gray,
                selected: Color::Yellow,
                warning: Color::Red,
            },
            Theme::Light => ThemeColors {
                background: Color::White,
                text: Color::Black,
                accent: Color::Blue,
                highlight: Color::LightBlue,
                border: Color::DarkGray,
                selected: Color::Magenta,
                warning: Color::Red,
            },
        }
    }

    fn toggle_sort(&mut self) {
        match &self.current_view {
            AppView::DatasetView(_) => {
                self.dataset_sort_order = self.dataset_sort_order.clone().next();
                self.sort_datasets();
                self.reset_dataset_selection();
            }
            AppView::SnapshotDetail(_, _) => {
                self.snapshot_sort_order = self.snapshot_sort_order.clone().next();
                self.sort_snapshots();
                self.reset_snapshot_selection();
            }
            _ => {}
        }
    }

    fn reset_dataset_selection(&mut self) {
        self.selected_dataset_index = 0;
        self.dataset_scroll_offset = 0;
    }

    fn reset_snapshot_selection(&mut self) {
        self.selected_snapshot_index = 0;
        self.snapshot_scroll_offset = 0;
    }

    fn sort_datasets(&mut self) {
        match self.dataset_sort_order {
            DatasetSortOrder::TotalSizeDesc => self.datasets.sort_by(|a, b| (b.referenced + b.snapshot_used).cmp(&(a.referenced + a.snapshot_used))),
            DatasetSortOrder::TotalSizeAsc => self.datasets.sort_by(|a, b| (a.referenced + a.snapshot_used).cmp(&(b.referenced + b.snapshot_used))),
            DatasetSortOrder::DatasetSizeDesc => self.datasets.sort_by(|a, b| b.referenced.cmp(&a.referenced)),
            DatasetSortOrder::DatasetSizeAsc => self.datasets.sort_by(|a, b| a.referenced.cmp(&b.referenced)),
            DatasetSortOrder::SnapshotSizeDesc => self.datasets.sort_by(|a, b| b.snapshot_used.cmp(&a.snapshot_used)),
            DatasetSortOrder::SnapshotSizeAsc => self.datasets.sort_by(|a, b| a.snapshot_used.cmp(&b.snapshot_used)),
            DatasetSortOrder::NameDesc => self.datasets.sort_by(|a, b| b.name.cmp(&a.name)),
            DatasetSortOrder::NameAsc => self.datasets.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }

    fn sort_snapshots(&mut self) {
        match self.snapshot_sort_order {
            SnapshotSortOrder::UsedDesc => self.snapshots.sort_by(|a, b| b.used.cmp(&a.used)),
            SnapshotSortOrder::UsedAsc => self.snapshots.sort_by(|a, b| a.used.cmp(&b.used)),
            SnapshotSortOrder::ReferencedDesc => self.snapshots.sort_by(|a, b| b.referenced.cmp(&a.referenced)),
            SnapshotSortOrder::ReferencedAsc => self.snapshots.sort_by(|a, b| a.referenced.cmp(&b.referenced)),
            SnapshotSortOrder::NameDesc => self.snapshots.sort_by(|a, b| b.name.cmp(&a.name)),
            SnapshotSortOrder::NameAsc => self.snapshots.sort_by(|a, b| a.name.cmp(&b.name)),
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
                let total_items = self.snapshots.len();
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

    pub fn is_prefetch_complete(&self) -> bool {
        self.prefetch_complete.load(Ordering::Relaxed)
    }

    pub fn get_prefetch_progress(&self) -> (usize, usize) {
        let total = self.prefetch_total.load(Ordering::Relaxed);
        let completed = self.prefetch_completed.load(Ordering::Relaxed);
        (completed, total)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub background: Color,
    pub text: Color,
    pub accent: Color,
    pub highlight: Color,
    pub border: Color,
    pub selected: Color,
    pub warning: Color,
}