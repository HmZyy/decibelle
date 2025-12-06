use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ThemeName {
    TokyoNight,
    #[default]
    CatppuccinMocha,
    Gruvbox,
    Kanagawa,
    Hackerman,
}

static CURRENT_THEME: OnceLock<Theme> = OnceLock::new();

pub fn init_theme(name: ThemeName) {
    let theme = match name {
        ThemeName::TokyoNight => Theme::tokyo_night(),
        ThemeName::CatppuccinMocha => Theme::catppuccin_mocha(),
        ThemeName::Gruvbox => Theme::gruvbox(),
        ThemeName::Kanagawa => Theme::kanagawa(),
        ThemeName::Hackerman => Theme::hackerman(),
    };
    let _ = CURRENT_THEME.set(theme);
}

pub fn get_theme() -> &'static Theme {
    CURRENT_THEME.get_or_init(Theme::catppuccin_mocha)
}

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

    pub const fn gruvbox() -> Self {
        Self {
            // Base
            bg: Color::Rgb(60, 56, 54),           // #3c3836
            bg_highlight: Color::Rgb(80, 73, 69), // derived
            fg: Color::Rgb(235, 219, 178),        // #ebdbb2
            fg_dim: Color::Rgb(146, 131, 116),    // #928374

            // UI elements
            border: Color::Rgb(69, 133, 136),          // #458588
            border_focused: Color::Rgb(142, 192, 124), // #8ec07c
            selection_bg: Color::Rgb(80, 73, 69),      // derived
            current_bg: Color::Rgb(69, 133, 136),      // #458588

            // Accents
            accent: Color::Rgb(142, 192, 124),    // #8ec07c
            accent_alt: Color::Rgb(215, 153, 33), // #d79921

            // Semantic
            playing: Color::Rgb(142, 192, 124), // #8ec07c
            paused: Color::Rgb(215, 153, 33),   // #d79921
            info: Color::Rgb(69, 133, 136),     // #458588
            title: Color::Rgb(204, 36, 29),     // #cc241d
        }
    }

    pub const fn kanagawa() -> Self {
        Self {
            // Base
            bg: Color::Rgb(46, 50, 87),            // #2e3257
            bg_highlight: Color::Rgb(66, 70, 107), // derived
            fg: Color::Rgb(255, 254, 247),         // #fffef7
            fg_dim: Color::Rgb(186, 187, 189),     // #babbbd

            // UI elements
            border: Color::Rgb(98, 125, 154),          // #627d9a
            border_focused: Color::Rgb(223, 197, 164), // #dfc5a4
            selection_bg: Color::Rgb(66, 70, 107),     // derived
            current_bg: Color::Rgb(98, 125, 154),      // #627d9a

            // Accents
            accent: Color::Rgb(223, 197, 164),    // #dfc5a4
            accent_alt: Color::Rgb(98, 125, 154), // #627d9a

            // Semantic
            playing: Color::Rgb(223, 197, 164), // #dfc5a4
            paused: Color::Rgb(186, 187, 189),  // #babbbd
            info: Color::Rgb(98, 125, 154),     // #627d9a
            title: Color::Rgb(255, 254, 247),   // #fffef7
        }
    }

    pub const fn hackerman() -> Self {
        Self {
            // Base
            bg: Color::Rgb(0, 0, 0),              // #000000
            bg_highlight: Color::Rgb(10, 25, 10), // #0a190a
            fg: Color::Rgb(0, 255, 65),           // #00ff41
            fg_dim: Color::Rgb(0, 128, 32),       // #008020

            // UI elements
            border: Color::Rgb(0, 180, 45),          // #00b42d
            border_focused: Color::Rgb(57, 255, 20), // #39ff14
            selection_bg: Color::Rgb(0, 50, 12),     // #00320c
            current_bg: Color::Rgb(0, 200, 50),      // #00c832

            // Accents
            accent: Color::Rgb(57, 255, 20),     // #39ff14
            accent_alt: Color::Rgb(0, 255, 159), // #00ff9f

            // Semantic
            playing: Color::Rgb(0, 255, 65), // #00ff41
            paused: Color::Rgb(180, 255, 0), // #b4ff00
            info: Color::Rgb(0, 255, 159),   // #00ff9f
            title: Color::Rgb(57, 255, 20),  // #39ff14
        }
    }

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
