use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::{
    app::{App, AppView, DatasetSortOrder, SnapshotSortOrder, Theme},
    zfs::format_bytes,
};

const MIN_NAME_WIDTH: usize = 20;
const BAR_WIDTH: usize = 20;

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(f.area());

    let visible_height = chunks[0].height.saturating_sub(2) as usize;
    app.update_scroll(visible_height);

    match &app.current_view {
        AppView::PoolList => draw_pool_list(f, chunks[0], app),
        AppView::DatasetView(pool_name) => draw_dataset_view(f, chunks[0], app, pool_name),
        AppView::SnapshotDetail(pool_name, dataset_name) => {
            draw_snapshot_detail(f, chunks[0], app, pool_name, dataset_name)
        }
        AppView::Help => draw_help_screen(f, chunks[0], app),
    }

    draw_status_bar(f, chunks[1], app);
}

fn draw_pool_list(f: &mut Frame, area: Rect, app: &App) {
    let colors = app.get_theme_colors();

    let max_name_width = calculate_max_pool_name_width(&app.pools);

    let items: Vec<ListItem> = app
        .pools
        .iter()
        .enumerate()
        .map(|(i, pool)| {
            let usage_percent = if pool.size > 0 {
                pool.allocated as f64 / pool.size as f64 * 100.0
            } else {
                0.0
            };

            // Use actual percentage for bar scaling (0-100%)
            let bar_chars = (BAR_WIDTH as f64 * usage_percent / 100.0) as usize;
            let usage_bar = create_progress_bar(bar_chars, '█');

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<width$}", pool.name, width = max_name_width),
                    Style::default().fg(colors.selected),
                ),
                Span::raw(" "),
                Span::styled(
                    usage_bar,
                    Style::default().fg(colors.accent),
                ),
                Span::styled(
                    format!(
                        " {:>8} / {:>8} ({:>3.0}%) [{}]",
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

    let max_name_width = calculate_max_dataset_name_width(&app.datasets, pool_name);

    // Calculate visible area height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.datasets.len(), visible_height);

    // Find maximum values in the visible range for relative scaling
    let visible_datasets: Vec<_> = app.datasets.iter().skip(start).take(end - start).collect();
    let max_dataset_size = visible_datasets
        .iter()
        .map(|d| d.referenced)
        .max()
        .unwrap_or(1);
    let max_snapshot_size = visible_datasets
        .iter()
        .map(|d| d.snapshot_used)
        .max()
        .unwrap_or(1);

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

            // Calculate percentages relative to maximum values on screen
            let dataset_percent = if max_dataset_size > 0 {
                (dataset_only as f64 / max_dataset_size as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            let snapshot_percent = if max_snapshot_size > 0 {
                (snapshot_used as f64 / max_snapshot_size as f64 * 100.0).min(100.0)
            } else {
                0.0
            };

            let dataset_chars = (BAR_WIDTH as f64 * dataset_percent / 100.0) as usize;
            let snapshot_chars = (BAR_WIDTH as f64 * snapshot_percent / 100.0) as usize;

            let dataset_bar = create_progress_bar(dataset_chars, '█');
            let snapshot_bar = create_progress_bar(snapshot_chars, '▓');

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

    let sort_indicator = get_dataset_sort_indicator(&app.dataset_sort_order);

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

    // Calculate visible area height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.snapshots.len(), visible_height);

    // Find maximum values in the visible range for relative scaling
    let visible_snapshots: Vec<_> = app.snapshots.iter().skip(start).take(end - start).collect();
    let max_used_size = visible_snapshots
        .iter()
        .map(|s| s.used)
        .max()
        .unwrap_or(1);
    let max_referenced_size = visible_snapshots
        .iter()
        .map(|s| s.referenced)
        .max()
        .unwrap_or(1);

    // Calculate available width for names dynamically
    // Total width minus bars, labels, spacing, and data display
    // Format: "NAME U:[bar] R:[bar] USED_SIZE REF_SIZE CREATION"
    // Fixed parts: " U:" (3) + bar (22) + " R:" (3) + bar (22) + " " (1) + used (8) + " " (1) + ref (8) + " " (1) + creation (~19) = ~88 chars
    let fixed_width = 88;
    let available_width = area.width as usize;
    let name_width = if available_width > fixed_width {
        (available_width - fixed_width).max(MIN_NAME_WIDTH)
    } else {
        MIN_NAME_WIDTH
    };

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

            // Calculate percentages relative to maximum values on screen
            let used_percent = if max_used_size > 0 {
                (snapshot_used as f64 / max_used_size as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            let referenced_percent = if max_referenced_size > 0 {
                (snapshot_referenced as f64 / max_referenced_size as f64 * 100.0).min(100.0)
            } else {
                0.0
            };

            let used_chars = (BAR_WIDTH as f64 * used_percent / 100.0) as usize;
            let referenced_chars = (BAR_WIDTH as f64 * referenced_percent / 100.0) as usize;

            let used_bar = create_progress_bar(used_chars, '▓');
            let referenced_bar = create_progress_bar(referenced_chars, '█');

            // Extract just the snapshot name (after the @)
            let short_name = snapshot.name
                .split('@')
                .last()
                .unwrap_or(&snapshot.name);

            // Truncate name with ellipsis if needed
            let display_name = truncate_with_ellipsis(short_name, name_width);

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<width$}", display_name, width = name_width),
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

    let sort_indicator = get_snapshot_sort_indicator(&app.snapshot_sort_order);

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
        "".to_string()
    } else {
        let (completed, total) = app.get_prefetch_progress();
        if total > 0 {
            format!(" [Loading snapshots for dataset {} of {}...]", completed, total)
        } else {
            " [Loading snapshots...]".to_string()
        }
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

fn calculate_max_pool_name_width(pools: &[crate::zfs::Pool]) -> usize {
    pools
        .iter()
        .map(|p| p.name.len())
        .max()
        .unwrap_or(MIN_NAME_WIDTH)
        .max(MIN_NAME_WIDTH)
}

fn calculate_max_dataset_name_width(datasets: &[crate::zfs::Dataset], pool_name: &str) -> usize {
    datasets
        .iter()
        .map(|d| {
            let short_name = d.name
                .strip_prefix(pool_name)
                .unwrap_or(&d.name)
                .trim_start_matches('/');
            short_name.len()
        })
        .max()
        .unwrap_or(MIN_NAME_WIDTH)
        .max(MIN_NAME_WIDTH)
}


fn create_progress_bar(filled_chars: usize, fill_char: char) -> String {
    let mut bar = String::with_capacity(BAR_WIDTH + 2);
    bar.push('[');
    for i in 0..BAR_WIDTH {
        if i < filled_chars {
            bar.push(fill_char);
        } else {
            bar.push(' ');
        }
    }
    bar.push(']');
    bar
}

fn get_dataset_sort_indicator(sort_order: &DatasetSortOrder) -> &'static str {
    match sort_order {
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

fn get_snapshot_sort_indicator(sort_order: &SnapshotSortOrder) -> &'static str {
    match sort_order {
        SnapshotSortOrder::UsedDesc => "Used Size ↓",
        SnapshotSortOrder::UsedAsc => "Used Size ↑",
        SnapshotSortOrder::ReferencedDesc => "Referenced Size ↓",
        SnapshotSortOrder::ReferencedAsc => "Referenced Size ↑",
        SnapshotSortOrder::NameDesc => "Name ↓",
        SnapshotSortOrder::NameAsc => "Name ↑",
    }
}

fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        return text.to_string();
    }

    if max_width <= 3 {
        return "...".chars().take(max_width).collect();
    }

    let half = (max_width - 3) / 2;
    let start = &text[..half];
    let end_start = text.len() - (max_width - 3 - half);
    let end = &text[end_start..];

    format!("{}...{}", start, end)
}