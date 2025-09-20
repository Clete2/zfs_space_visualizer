mod utils;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::{
    state::{AppState, AppView},
    zfs::format_bytes,
};

use utils::*;

pub fn draw(f: &mut Frame, app: &mut AppState) {
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

fn draw_pool_list(f: &mut Frame, area: Rect, app: &AppState) {
    let colors = app.theme_manager.get_colors();

    let max_name_width = calculate_max_pool_name_width(&app.data_manager.pools);

    let items: Vec<ListItem> = app
        .data_manager
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
                    Style::default().fg(colors.text),
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
                Style::default().fg(colors.selected).add_modifier(Modifier::BOLD).add_modifier(Modifier::REVERSED)
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

fn draw_dataset_view(f: &mut Frame, area: Rect, app: &AppState, pool_name: &str) {
    let colors = app.theme_manager.get_colors();

    let max_name_width = calculate_max_dataset_name_width(&app.data_manager.datasets, pool_name);

    // Calculate visible area height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.data_manager.datasets.len(), visible_height);

    // Find maximum values from all datasets for consistent scaling
    let max_dataset_size = app.data_manager.datasets
        .iter()
        .map(|d| d.referenced)
        .max()
        .unwrap_or(1);
    let max_snapshot_size = app.data_manager.datasets
        .iter()
        .map(|d| d.snapshot_used)
        .max()
        .unwrap_or(1);

    let items: Vec<ListItem> = app
        .data_manager
        .datasets
        .iter()
        .skip(start)
        .take(end - start)
        .map(|dataset| {
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
                    Style::default().fg(colors.text),
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

            ListItem::new(content)
        })
        .collect();

    let sort_indicator = app.sort_manager.get_dataset_sort_indicator();

    let title = format!("Datasets in Pool: {} (Sort: {})", pool_name, sort_indicator);

    let datasets_list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().fg(colors.selected).add_modifier(Modifier::BOLD).add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    // Create list state and set the selected index relative to visible items
    let mut list_state = ListState::default();
    if app.selected_dataset_index >= start && app.selected_dataset_index < end {
        list_state.select(Some(app.selected_dataset_index - start));
    }

    f.render_stateful_widget(datasets_list, area, &mut list_state);
}

fn draw_snapshot_detail(
    f: &mut Frame,
    area: Rect,
    app: &AppState,
    _pool_name: &str,
    dataset_name: &str,
) {
    let colors = app.theme_manager.get_colors();

    // Calculate visible area height (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.data_manager.snapshots.len(), visible_height);

    // Find maximum values from all snapshots for consistent scaling
    let max_used_size = app.data_manager.snapshots
        .iter()
        .map(|s| s.used)
        .max()
        .unwrap_or(1);
    let max_referenced_size = app.data_manager.snapshots
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
        .data_manager
        .snapshots
        .iter()
        .skip(start)
        .take(end - start)
        .map(|snapshot| {
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
                .next_back()
                .unwrap_or(&snapshot.name);

            // Truncate name with ellipsis if needed
            let display_name = truncate_with_ellipsis(short_name, name_width);

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<width$}", display_name, width = name_width),
                    Style::default().fg(colors.text),
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

            ListItem::new(content)
        })
        .collect();

    let sort_indicator = app.sort_manager.get_snapshot_sort_indicator();

    let title = format!("Snapshots in Dataset: {} (Sort: {})", dataset_name, sort_indicator);

    let snapshots_list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().fg(colors.selected).add_modifier(Modifier::BOLD).add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    // Create list state and set the selected index relative to visible items
    let mut list_state = ListState::default();
    if app.selected_snapshot_index >= start && app.selected_snapshot_index < end {
        list_state.select(Some(app.selected_snapshot_index - start));
    }

    f.render_stateful_widget(snapshots_list, area, &mut list_state);
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &AppState) {
    let colors = app.theme_manager.get_colors();
    let prefetch_status = if app.data_manager.is_prefetch_complete() {
        "".to_string()
    } else {
        let (completed, total) = app.data_manager.get_prefetch_progress();
        if total > 0 {
            format!(" [Loading snapshots for dataset {} of {}...]", completed, total)
        } else {
            " [Loading snapshots...]".to_string()
        }
    };

    let (status_text, help_text) = match &app.current_view {
        AppView::PoolList => {
            let total = app.data_manager.pools.len();
            let current = if total > 0 { app.selected_pool_index + 1 } else { 0 };
            (
                format!("Pool List ({}/{}){}",  current, total, prefetch_status),
                "↑/↓: Navigate | PgUp/PgDn: Page | →/Enter: View Datasets | h: Help | q: Quit"
            )
        },
        AppView::DatasetView(pool_name) => {
            let total = app.data_manager.datasets.len();
            let current = if total > 0 { app.selected_dataset_index + 1 } else { 0 };
            (
                format!("Datasets in {} ({}/{}){}",  pool_name, current, total, prefetch_status),
                "↑/↓: Navigate | PgUp/PgDn: Page | →/Enter: View Snapshots | s: Sort | ←/Esc: Back | h: Help | q: Quit"
            )
        },
        AppView::SnapshotDetail(_, dataset_name) => {
            let total = app.data_manager.snapshots.len();
            let current = if total > 0 { app.selected_snapshot_index + 1 } else { 0 };
            (
                format!("Snapshots in {} ({}/{}){}",  dataset_name, current, total, prefetch_status),
                "↑/↓: Navigate | PgUp/PgDn: Page | s: Sort | ←/Esc: Back | h: Help | q: Quit"
            )
        },
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

fn draw_help_screen(f: &mut Frame, area: Rect, app: &AppState) {
    let colors = app.theme_manager.get_colors();

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
    let themes = ["Dark", "Light"];
    let theme_items: Vec<ListItem> = themes
        .iter()
        .enumerate()
        .map(|(i, theme_name)| {
            let content = vec![Line::from(vec![
                Span::styled(
                    format!("  {}", theme_name),
                    if i == app.theme_manager.selected_theme_index {
                        Style::default().fg(colors.selected).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.text)
                    },
                ),
                if i == app.theme_manager.selected_theme_index {
                    Span::styled(" ◀", Style::default().fg(colors.accent))
                } else {
                    Span::raw("")
                },
            ])];

            ListItem::new(content).style(if i == app.theme_manager.selected_theme_index {
                Style::default().fg(colors.selected).add_modifier(Modifier::BOLD).add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            })
        })
        .collect();

    let theme_list = List::new(theme_items)
        .block(
            Block::default()
                .title(format!("Theme (Current: {})", match app.theme_manager.current_theme {
                    crate::theme::Theme::Dark => "Dark",
                    crate::theme::Theme::Light => "Light",
                }))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(theme_list, chunks[1]);
}

