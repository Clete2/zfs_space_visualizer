# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Development Commands

- `cargo run` - Run the application locally (requires ZFS)
- `cargo check` - Check for compilation errors
- `./build.sh [target]` - Cross-compile for specified target (defaults to x86_64-apple-darwin)
- `./build.sh x86_64-apple-darwin` - Build for local macOS target
- `./build.sh x86_64-unknown-linux-gnu` - Build for Linux (requires cross tool and Docker)

# Behaviors
- Run `cargo clippy --fix --allow-dirty` before commits that include Rust code changes

## Architecture Overview

This is a TUI (Terminal User Interface) application for visualizing ZFS space usage with three main components:

### Core Modules
- **main.rs** - Entry point, terminal setup/teardown, and async runtime initialization
- **app.rs** - Application state management and event handling with three-view navigation system
- **zfs.rs** - ZFS command execution and data parsing (pools, datasets, snapshots)
- **ui/mod.rs** - Ratatui-based rendering for all three views

### Navigation Flow
The application follows a hierarchical navigation pattern:
1. **Pool List** → shows all ZFS pools with usage stats
2. **Dataset View** → shows datasets in selected pool with visual usage bars
3. **Snapshot Detail** → shows individual snapshots in selected dataset

### Key Design Patterns
- **State-driven UI**: App struct holds current view state and selected indices
- **Async ZFS operations**: All ZFS commands use tokio::process::Command to prevent UI blocking
- **Error propagation**: Uses anyhow::Result throughout for consistent error handling
- **View-specific rendering**: Each view has dedicated draw functions in ui/mod.rs

### Data Structures
- `Pool`: ZFS pool with size, allocated, free space, and health status
- `Dataset`: Dataset with used space breakdown (dataset vs snapshot usage)
- `Snapshot`: Individual snapshot with usage and creation date

### ZFS Integration
- Executes `zpool list -H -p` for pool information
- Executes `zfs list -H -p -r -o name,used,avail,refer,usedbysnapshots <pool>` for datasets
- Executes `zfs list -H -p -t snap -r -o name,used,refer,creation <dataset>` for snapshots
- All commands use machine-readable output formats (-H -p flags)

## Development Preferences

- After every iteration: Build and deploy to cleteserver.home:/home/clete2/
- Use semantic commits for all changes
- Create feature branches for major new features only
- Never auto-deploy - always run scp command manually