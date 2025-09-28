use anyhow::Result;
use futures::future;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}},
};
use tokio::task;

use crate::zfs::{Pool, Dataset, Snapshot};

pub struct DataManager {
    pub pools: Vec<Pool>,
    pub datasets: Vec<Dataset>,
    pub snapshots: Vec<Snapshot>,
    pub snapshot_cache: Arc<Mutex<HashMap<String, Vec<Snapshot>>>>,
    pub prefetch_complete: Arc<AtomicBool>,
    pub prefetch_total: Arc<AtomicUsize>,
    pub prefetch_completed: Arc<AtomicUsize>,
    pub thread_count: usize,
}

impl DataManager {
    pub fn new(thread_count: usize) -> Self {
        Self {
            pools: Vec::new(),
            datasets: Vec::new(),
            snapshots: Vec::new(),
            snapshot_cache: Arc::new(Mutex::new(HashMap::new())),
            prefetch_complete: Arc::new(AtomicBool::new(false)),
            prefetch_total: Arc::new(AtomicUsize::new(0)),
            prefetch_completed: Arc::new(AtomicUsize::new(0)),
            thread_count,
        }
    }

    pub async fn load_pools(&mut self) -> Result<()> {
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
        let thread_count = self.thread_count;

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
            prefetch_total.store(all_datasets.len(), Ordering::Relaxed);
            prefetch_completed.store(0, Ordering::Relaxed);

            // Create semaphore to limit concurrent snapshot fetches
            // Use configured thread count
            let max_concurrent = thread_count;
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
                        completed.fetch_add(1, Ordering::Relaxed);
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

    pub async fn load_datasets(&mut self, pool_name: &str) -> Result<()> {
        self.datasets = crate::zfs::get_datasets(pool_name).await?;
        Ok(())
    }

    pub async fn load_snapshots(&mut self, dataset_name: &str) -> Result<()> {
        self.snapshots = self.get_cached_snapshots(dataset_name).unwrap_or_default();

        if self.snapshots.is_empty() {
            self.snapshots = crate::zfs::get_snapshots(dataset_name).await?;
            self.cache_snapshots(dataset_name);
        }

        Ok(())
    }

    pub async fn reload_snapshots(&mut self, dataset_name: &str) -> Result<()> {
        // Force reload from ZFS, bypassing cache
        self.snapshots = crate::zfs::get_snapshots(dataset_name).await?;
        self.cache_snapshots(dataset_name);
        Ok(())
    }

    pub fn get_cached_snapshots(&self, dataset_name: &str) -> Option<Vec<Snapshot>> {
        self.snapshot_cache
            .lock()
            .ok()?
            .get(dataset_name)
            .cloned()
    }

    pub fn cache_snapshots(&self, dataset_name: &str) {
        if let Ok(mut cache_lock) = self.snapshot_cache.lock() {
            cache_lock.insert(dataset_name.to_string(), self.snapshots.clone());
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