use crate::zfs::Pool;
use ratatui::{
    style::{Color, Style},
    text::Span,
};

pub const MIN_NAME_WIDTH: usize = 20;
pub const BAR_WIDTH: usize = 20;

pub fn calculate_max_pool_name_width(pools: &[Pool]) -> usize {
    pools
        .iter()
        .map(|p| p.name.len())
        .max()
        .unwrap_or(MIN_NAME_WIDTH)
        .max(MIN_NAME_WIDTH)
}

pub fn create_progress_bar_with_text(
    filled_chars: usize,
    fill_char: char,
    text: String,
    filled_bg_color: Color,
    text_color: Color
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();

    // Add opening bracket
    spans.push(Span::raw("["));

    // Right-justify the text within the bar
    let text_len = text.len().min(BAR_WIDTH);
    let start_pos = if text_len < BAR_WIDTH {
        BAR_WIDTH - text_len  // Right-justify
    } else {
        0
    };

    let truncated_text = if text.len() > BAR_WIDTH {
        text[..BAR_WIDTH].to_string()
    } else {
        text
    };

    for i in 0..BAR_WIDTH {
        if i >= start_pos && i < start_pos + text_len {
            // Show text character overlaying the bar
            let text_char = truncated_text.chars().nth(i - start_pos).unwrap_or(' ');
            if i < filled_chars {
                // Text on filled portion - use background color normally
                // When highlighted, this will lose the background but text will be different color
                spans.push(Span::styled(
                    text_char.to_string(),
                    Style::default()
                        .fg(text_color)
                        .bg(filled_bg_color)
                ));
            } else {
                // Text on empty portion - use dimmer color
                spans.push(Span::styled(
                    text_char.to_string(),
                    Style::default().fg(filled_bg_color)
                ));
            }
        } else {
            // Show the bar character
            if i < filled_chars {
                spans.push(Span::styled(
                    fill_char.to_string(),
                    Style::default().fg(filled_bg_color)
                ));
            } else {
                spans.push(Span::raw(" "));
            }
        }
    }

    // Add closing bracket
    spans.push(Span::raw("]"));

    spans
}


pub fn truncate_with_ellipsis(text: &str, max_width: usize) -> String {
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