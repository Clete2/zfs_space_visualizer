use clap::{Parser, Subcommand};

#[derive(Parser, Debug, Clone)]
#[command(name = "zfs_space_visualizer")]
#[command(about = "A TUI application for visualizing ZFS space usage")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[derive(Default)]
pub struct Config {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Enable readonly mode (disables delete functionality)
    #[arg(long, help = "Enable readonly mode to disable delete functionality")]
    pub readonly: bool,

    /// Number of threads to use for dataset refresh operations
    #[arg(long, value_name = "NUM", help = "Number of threads for dataset operations (default: auto-detected)")]
    pub threads: Option<usize>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Update the application to the latest version
    Update,
}


impl Config {
    pub fn parse_args() -> Self {
        Config::parse()
    }

    /// Get the effective thread count, using auto-detection if not specified
    pub fn effective_thread_count(&self) -> usize {
        self.threads.unwrap_or_else(|| {
            let cpu_count = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4); // fallback to 4 if detection fails
            cpu_count * 8 // IO_CONCURRENCY_MULTIPLIER
        }).max(1) // ensure at least 1 thread
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if let Some(threads) = self.threads {
            if threads == 0 {
                return Err("Thread count must be at least 1".to_string());
            }
            if threads > 1000 {
                return Err("Thread count must not exceed 1000".to_string());
            }
        }
        Ok(())
    }
}

