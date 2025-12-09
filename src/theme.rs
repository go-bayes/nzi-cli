//! catppuccin mocha theme implementation for ratatui
//! provides a cohesive colour palette for the entire application

use ratatui::style::{Color, Modifier, Style};

/// catppuccin mocha colour palette
/// see: https://github.com/catppuccin/catppuccin
pub mod catppuccin {
    use ratatui::style::Color;

    // base colours
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236);
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    pub const LAVENDER: Color = Color::Rgb(180, 190, 254);

    // surface colours
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT1: Color = Color::Rgb(186, 194, 222);
    pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200);
    pub const OVERLAY1: Color = Color::Rgb(127, 132, 156);
    pub const OVERLAY0: Color = Color::Rgb(108, 112, 134);
    pub const SURFACE2: Color = Color::Rgb(88, 91, 112);
    pub const SURFACE1: Color = Color::Rgb(69, 71, 90);
    pub const BASE: Color = Color::Rgb(30, 30, 46);
}

/// themed styles for the application
pub struct Theme;

impl Theme {
    /// style for block titles
    pub fn block_title() -> Style {
        Style::default()
            .fg(catppuccin::MAUVE)
            .add_modifier(Modifier::BOLD)
    }

    /// default text style
    pub fn text() -> Style {
        Style::default().fg(catppuccin::TEXT)
    }

    /// dimmed text style
    pub fn text_dim() -> Style {
        Style::default().fg(catppuccin::SUBTEXT0)
    }

    /// muted text style
    pub fn text_muted() -> Style {
        Style::default().fg(catppuccin::OVERLAY1)
    }

    /// highlight text style
    pub fn text_highlight() -> Style {
        Style::default()
            .fg(catppuccin::PEACH)
            .add_modifier(Modifier::BOLD)
    }

    /// rainbow colour array for animations
    pub fn rainbow_colors() -> [Color; 7] {
        [
            catppuccin::RED,
            catppuccin::PEACH,
            catppuccin::YELLOW,
            catppuccin::GREEN,
            catppuccin::SAPPHIRE,
            catppuccin::BLUE,
            catppuccin::MAUVE,
        ]
    }

    /// get a colour from the rainbow palette based on index
    pub fn rainbow(index: usize) -> Color {
        Self::rainbow_colors()[index % 7]
    }
}
