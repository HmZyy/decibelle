use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy)]
pub struct Theme {
    // Base colors
    pub bg: Color,
    pub bg_highlight: Color,
    pub fg: Color,
    pub fg_dim: Color,

    // UI elements
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub current_bg: Color,

    // Accents
    pub accent: Color,
    pub accent_alt: Color,

    // Semantic colors
    pub playing: Color,
    pub paused: Color,
    pub info: Color,
    pub title: Color,
}

impl Theme {
    pub const fn tokyo_night() -> Self {
        Self {
            // Base
            bg: Color::Rgb(26, 27, 38),           // #1a1b26
            bg_highlight: Color::Rgb(41, 46, 66), // #292e42
            fg: Color::Rgb(192, 202, 245),        // #c0caf5
            fg_dim: Color::Rgb(86, 95, 137),      // #565f89

            // UI elements
            border: Color::Rgb(61, 89, 161),           // #3d59a1
            border_focused: Color::Rgb(187, 154, 247), // #bb9af7
            selection_bg: Color::Rgb(41, 46, 66),      // #292e42
            current_bg: Color::Rgb(61, 89, 161),       // #3d59a1

            // Accents
            accent: Color::Rgb(125, 207, 255),     // #7dcfff
            accent_alt: Color::Rgb(255, 158, 100), // #ff9e64

            // Semantic
            playing: Color::Rgb(158, 206, 106), // #9ece6a
            paused: Color::Rgb(224, 175, 104),  // #e0af68
            info: Color::Rgb(187, 154, 247),    // #bb9af7
            title: Color::Rgb(122, 162, 247),   // #7aa2f7
        }
    }

    pub const fn catppuccin_mocha() -> Self {
        Self {
            // Base
            bg: Color::Rgb(30, 30, 46),           // #1e1e2e
            bg_highlight: Color::Rgb(49, 50, 68), // #313244
            fg: Color::Rgb(205, 214, 244),        // #cdd6f4
            fg_dim: Color::Rgb(108, 112, 134),    // #6c7086

            // UI elements
            border: Color::Rgb(137, 180, 250),         // #89b4fa
            border_focused: Color::Rgb(203, 166, 247), // #cba6f7
            selection_bg: Color::Rgb(69, 71, 90),      // #45475a
            current_bg: Color::Rgb(137, 180, 250),     // #89b4fa

            // Accents
            accent: Color::Rgb(148, 226, 213),     // #94e2d5
            accent_alt: Color::Rgb(250, 179, 135), // #fab387

            // Semantic
            playing: Color::Rgb(166, 227, 161), // #a6e3a1
            paused: Color::Rgb(249, 226, 175),  // #f9e2af
            info: Color::Rgb(203, 166, 247),    // #cba6f7
            title: Color::Rgb(137, 180, 250),   // #89b4fa
        }
    }

    // Computed styles
    pub fn border_style(&self, focused: bool) -> Style {
        Style::new().fg(if focused {
            self.border_focused
        } else {
            self.fg_dim
        })
    }

    pub fn selection_style(&self) -> Style {
        Style::new()
            .bg(self.selection_bg)
            .fg(self.fg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn current_style(&self) -> Style {
        Style::new()
            .bg(self.current_bg)
            .fg(self.bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn title_style(&self) -> Style {
        Style::new().fg(self.title).add_modifier(Modifier::BOLD)
    }

    pub fn label_style(&self) -> Style {
        Style::new().fg(self.fg_dim)
    }

    pub fn value_style(&self) -> Style {
        Style::new().fg(self.fg)
    }

    pub fn header_style(&self) -> Style {
        Style::new().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn slider_color(&self, playing: bool) -> Color {
        if playing { self.playing } else { self.paused }
    }
}

pub const THEME: Theme = Theme::catppuccin_mocha();
