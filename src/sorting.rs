use crate::zfs::{Dataset, Snapshot};

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

impl Default for DatasetSortOrder {
    fn default() -> Self {
        Self::TotalSizeDesc
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

impl Default for SnapshotSortOrder {
    fn default() -> Self {
        Self::UsedDesc
    }
}

#[derive(Default)]
pub struct SortManager {
    pub dataset_sort_order: DatasetSortOrder,
    pub snapshot_sort_order: SnapshotSortOrder,
}


impl SortManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sort_datasets(&self, datasets: &mut [Dataset]) {
        match self.dataset_sort_order {
            DatasetSortOrder::TotalSizeDesc => datasets.sort_by(|a, b| (b.referenced + b.snapshot_used).cmp(&(a.referenced + a.snapshot_used))),
            DatasetSortOrder::TotalSizeAsc => datasets.sort_by(|a, b| (a.referenced + a.snapshot_used).cmp(&(b.referenced + b.snapshot_used))),
            DatasetSortOrder::DatasetSizeDesc => datasets.sort_by(|a, b| b.referenced.cmp(&a.referenced)),
            DatasetSortOrder::DatasetSizeAsc => datasets.sort_by(|a, b| a.referenced.cmp(&b.referenced)),
            DatasetSortOrder::SnapshotSizeDesc => datasets.sort_by(|a, b| b.snapshot_used.cmp(&a.snapshot_used)),
            DatasetSortOrder::SnapshotSizeAsc => datasets.sort_by(|a, b| a.snapshot_used.cmp(&b.snapshot_used)),
            DatasetSortOrder::NameDesc => datasets.sort_by(|a, b| b.name.cmp(&a.name)),
            DatasetSortOrder::NameAsc => datasets.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }

    pub fn sort_snapshots(&self, snapshots: &mut [Snapshot]) {
        match self.snapshot_sort_order {
            SnapshotSortOrder::UsedDesc => snapshots.sort_by(|a, b| b.used.cmp(&a.used)),
            SnapshotSortOrder::UsedAsc => snapshots.sort_by(|a, b| a.used.cmp(&b.used)),
            SnapshotSortOrder::ReferencedDesc => snapshots.sort_by(|a, b| b.referenced.cmp(&a.referenced)),
            SnapshotSortOrder::ReferencedAsc => snapshots.sort_by(|a, b| a.referenced.cmp(&b.referenced)),
            SnapshotSortOrder::NameDesc => snapshots.sort_by(|a, b| b.name.cmp(&a.name)),
            SnapshotSortOrder::NameAsc => snapshots.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }

    pub fn toggle_dataset_sort(&mut self) {
        self.dataset_sort_order = self.dataset_sort_order.next();
    }

    pub fn toggle_snapshot_sort(&mut self) {
        self.snapshot_sort_order = self.snapshot_sort_order.next();
    }

    pub fn get_dataset_sort_indicator(&self) -> &'static str {
        match self.dataset_sort_order {
            DatasetSortOrder::TotalSizeDesc => "Total Size ↓",
            DatasetSortOrder::TotalSizeAsc => "Total Size ↑",
            DatasetSortOrder::DatasetSizeDesc => "Dataset Size ↓",
            DatasetSortOrder::DatasetSizeAsc => "Dataset Size ↑",
            DatasetSortOrder::SnapshotSizeDesc => "Snapshots Size ↓",
            DatasetSortOrder::SnapshotSizeAsc => "Snapshots Size ↑",
            DatasetSortOrder::NameDesc => "Name ↓",
            DatasetSortOrder::NameAsc => "Name ↑",
        }
    }

    pub fn get_snapshot_sort_indicator(&self) -> &'static str {
        match self.snapshot_sort_order {
            SnapshotSortOrder::UsedDesc => "Used Size ↓",
            SnapshotSortOrder::UsedAsc => "Used Size ↑",
            SnapshotSortOrder::ReferencedDesc => "Referenced Size ↓",
            SnapshotSortOrder::ReferencedAsc => "Referenced Size ↑",
            SnapshotSortOrder::NameDesc => "Name ↓",
            SnapshotSortOrder::NameAsc => "Name ↑",
        }
    }
}