use crate::zfs::Pool;

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


pub fn create_progress_bar(filled_chars: usize, fill_char: char) -> String {
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