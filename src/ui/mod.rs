mod utils;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::{
    state::{AppState, AppView},
    zfs::format_bytes,
};

use utils::*;

const DATASET_VIEW_FIXED_WIDTH: usize = 79;
const SNAPSHOT_VIEW_FIXED_WIDTH: usize = 54;
const STATUS_BAR_HEIGHT: u16 = 3;
const HELP_CONTENT_PERCENTAGE: u16 = 70;
const THEME_SELECTION_PERCENTAGE: u16 = 30;

pub fn draw(f: &mut Frame, app: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(STATUS_BAR_HEIGHT)])
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
        .map(|pool| {
            let usage_percent = if pool.size > 0 {
                pool.allocated as f64 / pool.size as f64 * 100.0
            } else {
                0.0
            };

            // Use actual percentage for bar scaling (0-100%)
            let bar_chars = (BAR_WIDTH as f64 * usage_percent / 100.0) as usize;

            // Create text to overlay on the bar
            let bar_text = format!("{}/{}", format_bytes(pool.allocated), format_bytes(pool.size));
            let usage_bar_spans = create_progress_bar_with_text(
                bar_chars,
                '█',
                bar_text,
                colors.accent,  // Background color for filled portion
                Color::White    // Text color
            );

            let mut content_spans = vec![
                Span::styled(
                    format!("{:<width$}", pool.name, width = max_name_width),
                    Style::default().fg(colors.text),
                ),
                Span::raw(" "),
            ];

            // Add the progress bar with text overlay
            content_spans.extend(usage_bar_spans);

            // Add remaining info after the bar
            content_spans.push(Span::styled(
                format!(" ({:>3.0}%) [{}]", usage_percent, pool.health),
                Style::default().fg(colors.text),
            ));

            let content = vec![Line::from(content_spans)];

            ListItem::new(content)
        })
        .collect();

    let pools_list = List::new(items)
        .block(
            Block::default()
                .title("ZFS Pools")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().bg(colors.highlight).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    // Create list state and set the selected index
    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_pool_index));

    f.render_stateful_widget(pools_list, area, &mut list_state);
}

fn draw_dataset_view(f: &mut Frame, area: Rect, app: &AppState, pool_name: &str) {
    let colors = app.theme_manager.get_colors();
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.data_manager.datasets.len(), visible_height);
    let scaling_values = calculate_dataset_scaling(&app.data_manager.datasets);
    let name_width = calculate_dataset_name_width(area.width as usize);

    let items = create_dataset_list_items(
        &app.data_manager.datasets[start..end],
        pool_name,
        &scaling_values,
        name_width,
        &colors
    );

    let sort_indicator = app.sort_manager.get_dataset_sort_indicator();

    let title = format!("Datasets in Pool: {} (Sort: {})", pool_name, sort_indicator);

    let datasets_list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().bg(colors.highlight).fg(Color::White).add_modifier(Modifier::BOLD))
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
    let visible_height = area.height.saturating_sub(2) as usize;
    let (start, end) = app.get_visible_range(app.data_manager.snapshots.len(), visible_height);
    let scaling_values = calculate_snapshot_scaling(&app.data_manager.snapshots);
    let name_width = calculate_snapshot_name_width(area.width as usize);

    let items = create_snapshot_list_items(
        &app.data_manager.snapshots[start..end],
        &scaling_values,
        name_width,
        &colors
    );

    let sort_indicator = app.sort_manager.get_snapshot_sort_indicator();

    let title = format!("Snapshots in Dataset: {} (Sort: {})", dataset_name, sort_indicator);

    let snapshots_list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(colors.border)),
        )
        .highlight_style(Style::default().bg(colors.highlight).fg(Color::White).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    // Create list state and set the selected index relative to visible items
    let mut list_state = ListState::default();
    if app.selected_snapshot_index >= start && app.selected_snapshot_index < end {
        list_state.select(Some(app.selected_snapshot_index - start));
    }

    f.render_stateful_widget(snapshots_list, area, &mut list_state);
}

// Helper function to get delete confirmation text
fn get_delete_help_text(app: &AppState) -> (String, Color) {
    // Check for error first
    if let Some(error) = &app.error_message {
        return (format!("ERROR: {} (Press any key to continue)", error), Color::Red);
    }

    // Check for delete confirmation
    if app.delete_confirmation_pending {
        if let Some(snapshot) = app.data_manager.snapshots.get(app.selected_snapshot_index) {
            let short_name = snapshot.name.split('@').next_back().unwrap_or(&snapshot.name);
            (format!("⚠️  DELETE {}: Press 'd' again to CONFIRM or wait 3s to cancel", short_name), Color::Yellow)
        } else {
            ("⚠️  Press 'd' again to CONFIRM DELETION or wait 3 seconds to cancel".to_string(), Color::Yellow)
        }
    } else {
        ("↑/↓: Navigate | PgUp/PgDn: Page | d: Delete | s: Sort | ←/Esc: Back | h: Help | q: Quit".to_string(), Color::Reset)
    }
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

    let (status_text, help_text, help_color) = match &app.current_view {
        AppView::PoolList => {
            let total = app.data_manager.pools.len();
            let current = if total > 0 { app.selected_pool_index + 1 } else { 0 };
            (
                format!("Pool List ({}/{}){}",  current, total, prefetch_status),
                "↑/↓: Navigate | PgUp/PgDn: Page | →/Enter: View Datasets | h: Help | q: Quit".to_string(),
                Color::Reset
            )
        },
        AppView::DatasetView(pool_name) => {
            let total = app.data_manager.datasets.len();
            let current = if total > 0 { app.selected_dataset_index + 1 } else { 0 };
            (
                format!("Datasets in {} ({}/{}){}",  pool_name, current, total, prefetch_status),
                "↑/↓: Navigate | PgUp/PgDn: Page | →/Enter: View Snapshots | s: Sort | ←/Esc: Back | h: Help | q: Quit".to_string(),
                Color::Reset
            )
        },
        AppView::SnapshotDetail(_, dataset_name) => {
            let total = app.data_manager.snapshots.len();
            let current = if total > 0 { app.selected_snapshot_index + 1 } else { 0 };
            let (help_text, help_color) = get_delete_help_text(app);
            (
                format!("Snapshots in {} ({}/{}){}",  dataset_name, current, total, prefetch_status),
                help_text,
                help_color
            )
        },
        AppView::Help => (
            format!("Help & Settings{}", prefetch_status),
            "↑/↓: Select Theme | Enter: Apply Theme | ←/Esc: Back | q: Quit".to_string(),
            Color::Reset
        ),
    };

    let status = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(&status_text, Style::default().fg(colors.accent)),
        ]),
        Line::from(vec![
            Span::styled(&help_text, Style::default().fg(help_color)),
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
        .constraints([Constraint::Percentage(HELP_CONTENT_PERCENTAGE), Constraint::Percentage(THEME_SELECTION_PERCENTAGE)].as_ref())
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
        Line::from("    D: █ Dataset data    S: █ Snapshot data"),
        Line::from("  Snapshot View:"),
        Line::from("    U: █ Used space     R: █ Referenced data"),
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
                Style::default().bg(colors.highlight).fg(Color::White)
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
        .highlight_style(Style::default().bg(colors.highlight).fg(Color::White).add_modifier(Modifier::BOLD));

    f.render_widget(theme_list, chunks[1]);
}

struct DatasetScalingValues {
    max_dataset_size: u64,
    max_snapshot_size: u64,
    max_total_size: u64,
}

fn calculate_dataset_scaling(datasets: &[crate::zfs::Dataset]) -> DatasetScalingValues {
    DatasetScalingValues {
        max_dataset_size: datasets.iter().map(|d| d.referenced).max().unwrap_or(1),
        max_snapshot_size: datasets.iter().map(|d| d.snapshot_used).max().unwrap_or(1),
        max_total_size: datasets.iter().map(|d| d.referenced + d.snapshot_used).max().unwrap_or(1),
    }
}

fn calculate_dataset_name_width(area_width: usize) -> usize {
    if area_width > DATASET_VIEW_FIXED_WIDTH {
        area_width - DATASET_VIEW_FIXED_WIDTH
    } else {
        MIN_NAME_WIDTH
    }
}

fn create_dataset_list_items<'a>(
    datasets: &'a [crate::zfs::Dataset],
    pool_name: &'a str,
    scaling: &'a DatasetScalingValues,
    name_width: usize,
    colors: &'a crate::theme::ThemeColors,
) -> Vec<ListItem<'a>> {
    datasets.iter().map(|dataset| {
        let dataset_only = dataset.referenced;
        let snapshot_used = dataset.snapshot_used;
        let total_used = dataset_only + snapshot_used;

        let dataset_percent = if scaling.max_dataset_size > 0 {
            (dataset_only as f64 / scaling.max_dataset_size as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let snapshot_percent = if scaling.max_snapshot_size > 0 {
            (snapshot_used as f64 / scaling.max_snapshot_size as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let total_percent = if scaling.max_total_size > 0 {
            (total_used as f64 / scaling.max_total_size as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        let dataset_chars = (BAR_WIDTH as f64 * dataset_percent / 100.0) as usize;
        let snapshot_chars = (BAR_WIDTH as f64 * snapshot_percent / 100.0) as usize;
        let total_chars = (BAR_WIDTH as f64 * total_percent / 100.0) as usize;

        let dataset_text = format_bytes(dataset_only);
        let snapshot_text = format_bytes(snapshot_used);
        let total_text = format_bytes(total_used);

        let dataset_bar_spans = create_progress_bar_with_text(
            dataset_chars, '█', dataset_text, colors.accent, Color::White
        );
        let snapshot_bar_spans = create_progress_bar_with_text(
            snapshot_chars, '█', snapshot_text, colors.accent, Color::White
        );
        let total_bar_spans = create_progress_bar_with_text(
            total_chars, '█', total_text, colors.accent, Color::White
        );

        let short_name = dataset.name.strip_prefix(pool_name)
            .unwrap_or(&dataset.name)
            .trim_start_matches('/');

        let display_name = if short_name.is_empty() || short_name == pool_name {
            "(root dataset)".to_string()
        } else {
            truncate_with_ellipsis(short_name, name_width)
        };

        let mut content_spans = vec![
            Span::styled(
                format!("{:<width$}", display_name, width = name_width),
                Style::default().fg(colors.text),
            ),
            Span::raw(" D:"),
        ];

        content_spans.extend(dataset_bar_spans);
        content_spans.push(Span::raw(" S:"));
        content_spans.extend(snapshot_bar_spans);
        content_spans.push(Span::raw(" T:"));
        content_spans.extend(total_bar_spans);

        ListItem::new(vec![Line::from(content_spans)])
    }).collect()
}

struct SnapshotScalingValues {
    max_used_size: u64,
    max_referenced_size: u64,
}

fn calculate_snapshot_scaling(snapshots: &[crate::zfs::Snapshot]) -> SnapshotScalingValues {
    SnapshotScalingValues {
        max_used_size: snapshots.iter().map(|s| s.used).max().unwrap_or(1),
        max_referenced_size: snapshots.iter().map(|s| s.referenced).max().unwrap_or(1),
    }
}

fn calculate_snapshot_name_width(area_width: usize) -> usize {
    if area_width > SNAPSHOT_VIEW_FIXED_WIDTH {
        (area_width - SNAPSHOT_VIEW_FIXED_WIDTH).max(MIN_NAME_WIDTH)
    } else {
        MIN_NAME_WIDTH
    }
}

fn create_snapshot_list_items<'a>(
    snapshots: &'a [crate::zfs::Snapshot],
    scaling: &'a SnapshotScalingValues,
    name_width: usize,
    colors: &'a crate::theme::ThemeColors,
) -> Vec<ListItem<'a>> {
    snapshots.iter().map(|snapshot| {
        let snapshot_used = snapshot.used;
        let snapshot_referenced = snapshot.referenced;

        let used_percent = if scaling.max_used_size > 0 {
            (snapshot_used as f64 / scaling.max_used_size as f64 * 100.0).min(100.0)
        } else {
            0.0
        };
        let referenced_percent = if scaling.max_referenced_size > 0 {
            (snapshot_referenced as f64 / scaling.max_referenced_size as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        let used_chars = (BAR_WIDTH as f64 * used_percent / 100.0) as usize;
        let referenced_chars = (BAR_WIDTH as f64 * referenced_percent / 100.0) as usize;

        let used_text = format_bytes(snapshot_used);
        let referenced_text = format_bytes(snapshot_referenced);

        let used_bar_spans = create_progress_bar_with_text(
            used_chars, '█', used_text, colors.accent, Color::White
        );
        let referenced_bar_spans = create_progress_bar_with_text(
            referenced_chars, '█', referenced_text, colors.accent, Color::White
        );

        let short_name = snapshot.name.split('@').next_back().unwrap_or(&snapshot.name);
        let display_name = truncate_with_ellipsis(short_name, name_width);

        let mut content_spans = vec![
            Span::styled(
                format!("{:<width$}", display_name, width = name_width),
                Style::default().fg(colors.text),
            ),
            Span::raw(" U:"),
        ];

        content_spans.extend(used_bar_spans);
        content_spans.push(Span::raw(" R:"));
        content_spans.extend(referenced_bar_spans);

        ListItem::new(vec![Line::from(content_spans)])
    }).collect()
}

