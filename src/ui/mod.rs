use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

use crate::{
    app::{App, AppView},
    zfs::format_bytes,
};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)].as_ref())
        .split(f.area());

    // Main content area
    match &app.current_view {
        AppView::PoolList => draw_pool_list(f, chunks[0], app),
        AppView::DatasetView(pool_name) => draw_dataset_view(f, chunks[0], app, pool_name),
        AppView::SnapshotDetail(pool_name, dataset_name) => {
            draw_snapshot_detail(f, chunks[0], app, pool_name, dataset_name)
        }
    }

    // Status bar
    draw_status_bar(f, chunks[1], app);
}

fn draw_pool_list(f: &mut Frame, area: Rect, app: &App) {
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
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!(
                    " {:>8} / {:>8} ({:>3}%) [{}]",
                    format_bytes(pool.allocated),
                    format_bytes(pool.size),
                    usage_percent,
                    pool.health
                )),
            ])];

            ListItem::new(content).style(if i == app.selected_pool_index {
                Style::default().bg(Color::Blue)
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
                .border_style(Style::default().fg(Color::White)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_widget(pools_list, area);
}

fn draw_dataset_view(f: &mut Frame, area: Rect, app: &App, pool_name: &str) {
    let items: Vec<ListItem> = app
        .datasets
        .iter()
        .enumerate()
        .map(|(i, dataset)| {
            let total_used = dataset.used;
            let dataset_only = dataset.referenced;
            let snapshot_used = dataset.snapshot_used;

            let dataset_percent = if total_used > 0 {
                (dataset_only as f64 / total_used as f64 * 100.0) as u16
            } else {
                100
            };

            let snapshot_percent = if total_used > 0 {
                (snapshot_used as f64 / total_used as f64 * 100.0) as u16
            } else {
                0
            };

            // Create usage bar representation
            let bar_width = 40;
            let dataset_chars = (bar_width as f64 * dataset_percent as f64 / 100.0) as usize;
            let snapshot_chars = (bar_width as f64 * snapshot_percent as f64 / 100.0) as usize;

            let mut bar = String::new();
            bar.push('[');
            for j in 0..bar_width {
                if j < dataset_chars {
                    bar.push('█'); // Dataset usage
                } else if j < dataset_chars + snapshot_chars {
                    bar.push('▓'); // Snapshot usage
                } else {
                    bar.push(' '); // Free space
                }
            }
            bar.push(']');

            let short_name = dataset.name.strip_prefix(pool_name)
                .unwrap_or(&dataset.name)
                .trim_start_matches('/');

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<30}", short_name),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    bar,
                    Style::default().fg(Color::Green),
                ),
                Span::raw(format!(
                    " {:>8} (D:{:>8} S:{:>8})",
                    format_bytes(total_used),
                    format_bytes(dataset_only),
                    format_bytes(snapshot_used),
                )),
            ])];

            ListItem::new(content).style(if i == app.selected_dataset_index {
                Style::default().bg(Color::Blue)
            } else {
                Style::default()
            })
        })
        .collect();

    let datasets_list = List::new(items)
        .block(
            Block::default()
                .title(format!("Datasets in Pool: {}", pool_name))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
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
    let items: Vec<ListItem> = app
        .snapshots
        .iter()
        .enumerate()
        .map(|(i, snapshot)| {
            // Extract just the snapshot name (after the @)
            let short_name = snapshot.name
                .split('@')
                .last()
                .unwrap_or(&snapshot.name);

            let content = vec![Line::from(vec![
                Span::styled(
                    format!("{:<40}", short_name),
                    Style::default().fg(Color::Magenta),
                ),
                Span::raw(format!(
                    " {:>8} {:>8} {}",
                    format_bytes(snapshot.used),
                    format_bytes(snapshot.referenced),
                    snapshot.creation
                )),
            ])];

            ListItem::new(content).style(if i == app.selected_snapshot_index {
                Style::default().bg(Color::Blue)
            } else {
                Style::default()
            })
        })
        .collect();

    let snapshots_list = List::new(items)
        .block(
            Block::default()
                .title(format!("Snapshots in Dataset: {}", dataset_name))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_widget(snapshots_list, area);
}

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let (status_text, help_text) = match &app.current_view {
        AppView::PoolList => (
            "Pool List".to_string(),
            "↑/↓: Navigate | →/Enter: View Datasets | q: Quit"
        ),
        AppView::DatasetView(pool_name) => (
            format!("Datasets in {}", pool_name),
            "↑/↓: Navigate | →/Enter: View Snapshots | ←/Esc: Back | q: Quit"
        ),
        AppView::SnapshotDetail(_, dataset_name) => (
            format!("Snapshots in {}", dataset_name),
            "↑/↓: Navigate | ←/Esc: Back | q: Quit"
        ),
    };

    let status = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(&status_text, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled(help_text, Style::default().fg(Color::Gray)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::White)),
    )
    .wrap(Wrap { trim: true });

    f.render_widget(status, area);
}