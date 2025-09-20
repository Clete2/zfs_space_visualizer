use ratatui::style::Color;

#[derive(Debug, Clone, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Dark
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub background: Color,
    pub text: Color,
    pub accent: Color,
    pub highlight: Color,
    pub border: Color,
    pub selected: Color,
    pub warning: Color,
}

impl Theme {
    pub const fn get_colors(&self) -> ThemeColors {
        match self {
            Theme::Dark => ThemeColors {
                background: Color::Black,
                text: Color::White,
                accent: Color::Cyan,
                highlight: Color::Blue,
                border: Color::Gray,
                selected: Color::Yellow,
                warning: Color::Red,
            },
            Theme::Light => ThemeColors {
                background: Color::White,
                text: Color::Black,
                accent: Color::Blue,
                highlight: Color::LightBlue,
                border: Color::DarkGray,
                selected: Color::Magenta,
                warning: Color::Red,
            },
        }
    }
}

pub struct ThemeManager {
    pub current_theme: Theme,
    pub selected_theme_index: usize,
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self {
            current_theme: Theme::default(),
            selected_theme_index: 0,
        }
    }
}

impl ThemeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_colors(&self) -> ThemeColors {
        self.current_theme.get_colors()
    }

    pub fn previous_theme(&mut self) {
        if self.selected_theme_index > 0 {
            self.selected_theme_index -= 1;
        }
    }

    pub fn next_theme(&mut self) {
        if self.selected_theme_index < 1 { // We have 2 themes (0-1)
            self.selected_theme_index += 1;
        }
    }

    pub fn select_theme(&mut self) {
        self.current_theme = match self.selected_theme_index {
            0 => Theme::Dark,
            1 => Theme::Light,
            _ => Theme::Light,
        };
    }

    pub fn set_selected_index_from_theme(&mut self) {
        self.selected_theme_index = match self.current_theme {
            Theme::Dark => 0,
            Theme::Light => 1,
        };
    }
}