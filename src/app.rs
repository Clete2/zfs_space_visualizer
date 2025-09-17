use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{
    backend::Backend,
    Terminal,
};

use crate::zfs::{Pool, Dataset, Snapshot};

#[derive(Debug, Clone)]
pub enum AppView {
    PoolList,
    DatasetView(String), // pool name
    SnapshotDetail(String, String), // pool name, dataset name
}

#[derive(Debug)]
pub struct App {
    pub should_quit: bool,
    pub current_view: AppView,
    pub pools: Vec<Pool>,
    pub datasets: Vec<Dataset>,
    pub snapshots: Vec<Snapshot>,
    pub selected_pool_index: usize,
    pub selected_dataset_index: usize,
    pub selected_snapshot_index: usize,
}

impl Default for App {
    fn default() -> Self {
        Self {
            should_quit: false,
            current_view: AppView::PoolList,
            pools: Vec::new(),
            datasets: Vec::new(),
            snapshots: Vec::new(),
            selected_pool_index: 0,
            selected_dataset_index: 0,
            selected_snapshot_index: 0,
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
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => self.should_quit = true,
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Left => self.go_back().await?,
            KeyCode::Enter | KeyCode::Right => self.go_forward().await?,
            KeyCode::Up => self.previous_item(),
            KeyCode::Down => self.next_item(),
            _ => {}
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
        }
        Ok(())
    }

    async fn load_pools(&mut self) -> Result<()> {
        self.pools = crate::zfs::get_pools().await?;
        Ok(())
    }

    async fn load_datasets(&mut self, pool_name: &str) -> Result<()> {
        self.datasets = crate::zfs::get_datasets(pool_name).await?;
        Ok(())
    }

    async fn load_snapshots(&mut self, dataset_name: &str) -> Result<()> {
        self.snapshots = crate::zfs::get_snapshots(dataset_name).await?;
        Ok(())
    }
}