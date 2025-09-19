use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

use crate::{
    app::{App, AppView, Theme},
    zfs::format_bytes,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    // Update scrolling for current view
    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    app.update_scroll(visible_height);

    // Main content area
    match &app.current_view.clone() {
        AppView::PoolList => draw_pool_list(f, chunks[0], app),
        AppView::DatasetView(pool_name) => draw_dataset_view(f, chunks[0], app, &pool_name.clone()),
        AppView::SnapshotDetail(pool_name, dataset_name) => {
            draw_snapshot_detail(f, chunks[0], app, &pool_name.clone(), &dataset_name.clone())
        }
        AppView::Help => draw_help_screen(f, chunks[0], app),
    }

    // Status bar
    draw_status_bar(f, chunks[1], app);
}

fn draw_pool_list(f: &mut Frame, area: Rect, app: &App) {
    let colors = app.get_theme_colors();
    let items: Vec<ListItem> = app
        .pools
        .iter()
        .enumerate()
        .map(|(i, pool)| {
            let usage_percent = if pool.size > 0 {
                (pool.allocated as f64 / pool.size as f64 * 100.0) as u64
            } else {
                0
            };

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<20}", pool.name),
                    Style::default().fg(colors.accent),
                ),
                Span::styled(
                    format!(
                        " {:>8} / {:>8} ({:>3}%) [{}]",
                        format_bytes(pool.allocated),
                        format_bytes(pool.size),
                        usage_percent,
                        pool.health
                    ),
                    Style::default().fg(colors.text),
                ),
            ])];

            ListItem::new(content).style(if i == app.selected_pool_index {
                Style::default().bg(colors.highlight)
            } else {
                Style::default()
            })
        })
        .collect();

    let pools_list = List::new(items)
        .block(
            Block::default()
                .title("ZFS Pools")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_widget(pools_list, area);
}

fn draw_dataset_view(f: &mut Frame, area: Rect, app: &App, pool_name: &str) {
    let colors = app.get_theme_colors();

    // Find the current pool to get its usable size for normalization
    let pool_usable_size = app.pools
        .iter()
        .find(|p| p.name == pool_name)
        .map(|p| p.usable_size)
        .unwrap_or(1); // Default to 1 to avoid division by zero

    // Calculate fixed width for dataset names
    let max_name_width = app.datasets
        .iter()
        .map(|d| {
            let short_name = d.name.strip_prefix(pool_name)
                .unwrap_or(&d.name)
                .trim_start_matches('/');
            short_name.len()
        })
        .max()
        .unwrap_or(20)
        .max(20); // Minimum width of 20

    // Calculate visible area height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.datasets.len(), visible_height);

    let items: Vec<ListItem> = app
        .datasets
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(i, dataset)| {
            let actual_index = start + i;
            let dataset_only = dataset.referenced;
            let snapshot_used = dataset.snapshot_used;

            // Calculate percentages relative to pool usable size for normalization
            let dataset_percent = (dataset_only as f64 / pool_usable_size as f64 * 100.0).min(100.0);
            let snapshot_percent = (snapshot_used as f64 / pool_usable_size as f64 * 100.0).min(100.0);

            // Create separate bars
            let bar_width = 20; // Smaller bars since we have two
            let dataset_chars = (bar_width as f64 * dataset_percent / 100.0) as usize;
            let snapshot_chars = (bar_width as f64 * snapshot_percent / 100.0) as usize;

            // Dataset bar
            let mut dataset_bar = String::new();
            dataset_bar.push('[');
            for j in 0..bar_width {
                if j < dataset_chars {
                    dataset_bar.push('█');
                } else {
                    dataset_bar.push(' ');
                }
            }
            dataset_bar.push(']');

            // Snapshot bar
            let mut snapshot_bar = String::new();
            snapshot_bar.push('[');
            for j in 0..bar_width {
                if j < snapshot_chars {
                    snapshot_bar.push('▓');
                } else {
                    snapshot_bar.push(' ');
                }
            }
            snapshot_bar.push(']');

            let short_name = dataset.name.strip_prefix(pool_name)
                .unwrap_or(&dataset.name)
                .trim_start_matches('/');

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<width$}", short_name, width = max_name_width),
                    Style::default().fg(colors.selected),
                ),
                Span::raw(" D:"),
                Span::styled(
                    dataset_bar,
                    Style::default().fg(colors.accent),
                ),
                Span::raw(" S:"),
                Span::styled(
                    snapshot_bar,
                    Style::default().fg(colors.text),
                ),
                Span::styled(format!(
                    " {:>8} (D:{:>8} S:{:>8})",
                    format_bytes(dataset_only + snapshot_used),
                    format_bytes(dataset_only),
                    format_bytes(snapshot_used),
                ), Style::default().fg(colors.text)),
            ])];

            ListItem::new(content).style(if actual_index == app.selected_dataset_index {
                Style::default().bg(colors.highlight)
            } else {
                Style::default()
            })
        })
        .collect();

    // Sort indicator
    let sort_indicator = match app.dataset_sort_order {
        crate::app::SortOrder::TotalSizeDesc => "Total Size ↓",
        crate::app::SortOrder::TotalSizeAsc => "Total Size ↑",
        crate::app::SortOrder::DatasetSizeDesc => "Dataset Size ↓",
        crate::app::SortOrder::DatasetSizeAsc => "Dataset Size ↑",
        crate::app::SortOrder::SnapshotSizeDesc => "Snapshots Size ↓",
        crate::app::SortOrder::SnapshotSizeAsc => "Snapshots Size ↑",
        crate::app::SortOrder::NameDesc => "Name ↓",
        crate::app::SortOrder::NameAsc => "Name ↑",
    };

    let title = format!("Datasets in Pool: {} (Sort: {})", pool_name, sort_indicator);

    let datasets_list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_widget(datasets_list, area);
}

fn draw_snapshot_detail(
    f: &mut Frame,
    area: Rect,
    app: &App,
    _pool_name: &str,
    dataset_name: &str,
) {
    let colors = app.get_theme_colors();

    // Find the current dataset to get its total size for normalization
    let dataset_total_size = app.datasets
        .iter()
        .find(|d| d.name == dataset_name)
        .map(|d| d.referenced + d.snapshot_used)
        .unwrap_or_else(|| {
            // If not found in current datasets, calculate from snapshots
            app.snapshots.iter().map(|s| s.used).sum::<u64>().max(1)
        });

    // Calculate fixed width for snapshot names
    let max_name_width = app.snapshots
        .iter()
        .map(|s| {
            let short_name = s.name
                .split('@')
                .last()
                .unwrap_or(&s.name);
            short_name.len()
        })
        .max()
        .unwrap_or(20)
        .max(20); // Minimum width of 20

    // Calculate visible area height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.snapshots.len(), visible_height);

    let items: Vec<ListItem> = app
        .snapshots
        .iter()
        .enumerate()
        .skip(start)
        .take(end - start)
        .map(|(i, snapshot)| {
            let actual_index = start + i;
            let snapshot_used = snapshot.used;
            let snapshot_referenced = snapshot.referenced;

            // Calculate percentages relative to dataset total size for normalization
            let used_percent = (snapshot_used as f64 / dataset_total_size as f64 * 100.0).min(100.0);
            let referenced_percent = (snapshot_referenced as f64 / dataset_total_size as f64 * 100.0).min(100.0);

            // Create separate bars
            let bar_width = 20; // Smaller bars since we have two
            let used_chars = (bar_width as f64 * used_percent / 100.0) as usize;
            let referenced_chars = (bar_width as f64 * referenced_percent / 100.0) as usize;

            // Used space bar (snapshot size)
            let mut used_bar = String::new();
            used_bar.push('[');
            for j in 0..bar_width {
                if j < used_chars {
                    used_bar.push('▓');
                } else {
                    used_bar.push(' ');
                }
            }
            used_bar.push(']');

            // Referenced space bar (actual data size)
            let mut referenced_bar = String::new();
            referenced_bar.push('[');
            for j in 0..bar_width {
                if j < referenced_chars {
                    referenced_bar.push('█');
                } else {
                    referenced_bar.push(' ');
                }
            }
            referenced_bar.push(']');

            // Extract just the snapshot name (after the @)
            let short_name = snapshot.name
                .split('@')
                .last()
                .unwrap_or(&snapshot.name);

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<width$}", short_name, width = max_name_width),
                    Style::default().fg(colors.selected),
                ),
                Span::raw(" U:"),
                Span::styled(
                    used_bar,
                    Style::default().fg(colors.text),
                ),
                Span::raw(" R:"),
                Span::styled(
                    referenced_bar,
                    Style::default().fg(colors.accent),
                ),
                Span::styled(format!(
                    " {:>8} {:>8} {}",
                    format_bytes(snapshot_used),
                    format_bytes(snapshot_referenced),
                    snapshot.creation
                ), Style::default().fg(colors.text)),
            ])];

            ListItem::new(content).style(if actual_index == app.selected_snapshot_index {
                Style::default().bg(colors.highlight)
            } else {
                Style::default()
            })
        })
        .collect();

    // Sort indicator
    let sort_indicator = match app.snapshot_sort_order {
        crate::app::SortOrder::TotalSizeDesc => "Total Size ↓",
        crate::app::SortOrder::TotalSizeAsc => "Total Size ↑",
        crate::app::SortOrder::DatasetSizeDesc => "Dataset Size ↓",
        crate::app::SortOrder::DatasetSizeAsc => "Dataset Size ↑",
        crate::app::SortOrder::SnapshotSizeDesc => "Snapshots Size ↓",
        crate::app::SortOrder::SnapshotSizeAsc => "Snapshots Size ↑",
        crate::app::SortOrder::NameDesc => "Name ↓",
        crate::app::SortOrder::NameAsc => "Name ↑",
    };

    let title = format!("Snapshots in Dataset: {} (Sort: {})", dataset_name, sort_indicator);

    let snapshots_list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_widget(snapshots_list, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let colors = app.get_theme_colors();
    let prefetch_status = if app.is_prefetch_complete() {
        ""
    } else {
        " [Loading snapshots...]"
    };

    let (status_text, help_text) = match &app.current_view {
        AppView::PoolList => (
            format!("Pool List{}", prefetch_status),
            "↑/↓: Navigate | →/Enter: View Datasets | h: Help | q: Quit"
        ),
        AppView::DatasetView(pool_name) => (
            format!("Datasets in {}{}", pool_name, prefetch_status),
            "↑/↓: Navigate | →/Enter: View Snapshots | s: Sort | ←/Esc: Back | h: Help | q: Quit"
        ),
        AppView::SnapshotDetail(_, dataset_name) => (
            format!("Snapshots in {}{}", dataset_name, prefetch_status),
            "↑/↓: Navigate | s: Sort | ←/Esc: Back | h: Help | q: Quit"
        ),
        AppView::Help => (
            format!("Help & Settings{}", prefetch_status),
            "↑/↓: Select Theme | Enter: Apply Theme | ←/Esc: Back | q: Quit"
        ),
    };

    let status = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(&status_text, Style::default().fg(colors.accent)),
        ]),
        Line::from(vec![
            Span::styled(help_text, Style::default().fg(colors.text)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(colors.border)),
    )
    .wrap(Wrap { trim: true });

    f.render_widget(status, area);
}

fn draw_help_screen(f: &mut Frame, area: Rect, app: &App) {
    let colors = app.get_theme_colors();

    // Split area into help content and theme selection
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
        .split(area);

    // Help content
    let help_text = vec![
        Line::from(vec![Span::styled("ZFS Space Visualizer", Style::default().fg(colors.accent).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from("NAVIGATION:"),
        Line::from("  ↑/↓ or j/k     Navigate up/down"),
        Line::from("  →/Enter        Go forward/select"),
        Line::from("  ←/Esc/Backspace Go back"),
        Line::from("  h              Show this help"),
        Line::from("  q or Ctrl+C    Quit application"),
        Line::from(""),
        Line::from("VIEWS:"),
        Line::from("  Pool List      Shows all ZFS pools with usage"),
        Line::from("  Dataset View   Shows datasets in selected pool"),
        Line::from("  Snapshot View  Shows snapshots in selected dataset"),
        Line::from(""),
        Line::from("LEGEND:"),
        Line::from("  Dataset View:"),
        Line::from("    D: █ Dataset data    S: ▓ Snapshot data"),
        Line::from("  Snapshot View:"),
        Line::from("    U: ▓ Used space     R: █ Referenced data"),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .title("Help")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .style(Style::default().fg(colors.text))
        .wrap(Wrap { trim: true });

    f.render_widget(help_paragraph, chunks[0]);

    // Theme selection
    let themes = vec!["Dark", "Light"];
    let theme_items: Vec<ListItem> = themes
        .iter()
        .enumerate()
        .map(|(i, theme_name)| {
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("  {}", theme_name),
                    if i == app.selected_theme_index {
                        Style::default().fg(colors.selected).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.text)
                    },
                ),
                if i == app.selected_theme_index {
                    Span::styled(" ◀", Style::default().fg(colors.accent))
                } else {
                    Span::raw("")
                },
            ])];

            ListItem::new(content).style(if i == app.selected_theme_index {
                Style::default().bg(colors.highlight)
            } else {
                Style::default()
            })
        })
        .collect();

    let theme_list = List::new(theme_items)
        .block(
            Block::default()
                .title(format!("Theme (Current: {})", if app.theme == Theme::Dark { "Dark" } else { "Light" }))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(theme_list, chunks[1]);
}