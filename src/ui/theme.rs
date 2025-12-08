use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::ui::notifications::NotificationLevel;

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
    #[allow(dead_code)]
    pub bg_highlight: Color,
    pub fg: Color,
    pub fg_dim: Color,

    // UI elements
    #[allow(dead_code)]
    pub border: Color,
    pub border_focused: Color,
    pub selection_bg: Color,
    pub current_bg: Color,

    // Accents
    pub accent: Color,
    #[allow(dead_code)]
    pub accent_alt: Color,

    // Semantic colors
    pub playing: Color,
    pub paused: Color,
    pub info: Color,
    pub title: Color,

    // Notification colors
    pub notif_debug: Color,
    pub notif_info: Color,
    pub notif_warning: Color,
    pub notif_error: Color,
}

impl Theme {
    pub const fn tokyo_night() -> Self {
        Self {
            bg: Color::Rgb(26, 27, 38),
            bg_highlight: Color::Rgb(41, 46, 66),
            fg: Color::Rgb(192, 202, 245),
            fg_dim: Color::Rgb(86, 95, 137),
            border: Color::Rgb(61, 89, 161),
            border_focused: Color::Rgb(187, 154, 247),
            selection_bg: Color::Rgb(41, 46, 66),
            current_bg: Color::Rgb(61, 89, 161),
            accent: Color::Rgb(125, 207, 255),
            accent_alt: Color::Rgb(255, 158, 100),
            playing: Color::Rgb(158, 206, 106),
            paused: Color::Rgb(224, 175, 104),
            info: Color::Rgb(187, 154, 247),
            title: Color::Rgb(122, 162, 247),
            notif_debug: Color::Rgb(192, 202, 245),
            notif_info: Color::Rgb(122, 162, 247),
            notif_warning: Color::Rgb(255, 158, 100),
            notif_error: Color::Rgb(247, 118, 142),
        }
    }

    pub const fn catppuccin_mocha() -> Self {
        Self {
            bg: Color::Rgb(30, 30, 46),
            bg_highlight: Color::Rgb(49, 50, 68),
            fg: Color::Rgb(205, 214, 244),
            fg_dim: Color::Rgb(108, 112, 134),
            border: Color::Rgb(137, 180, 250),
            border_focused: Color::Rgb(203, 166, 247),
            selection_bg: Color::Rgb(69, 71, 90),
            current_bg: Color::Rgb(137, 180, 250),
            accent: Color::Rgb(148, 226, 213),
            accent_alt: Color::Rgb(250, 179, 135),
            playing: Color::Rgb(166, 227, 161),
            paused: Color::Rgb(249, 226, 175),
            info: Color::Rgb(203, 166, 247),
            title: Color::Rgb(137, 180, 250),
            notif_debug: Color::Rgb(205, 214, 244),
            notif_info: Color::Rgb(137, 180, 250),
            notif_warning: Color::Rgb(250, 179, 135),
            notif_error: Color::Rgb(243, 139, 168),
        }
    }

    pub const fn gruvbox() -> Self {
        Self {
            bg: Color::Rgb(60, 56, 54),
            bg_highlight: Color::Rgb(80, 73, 69),
            fg: Color::Rgb(235, 219, 178),
            fg_dim: Color::Rgb(146, 131, 116),
            border: Color::Rgb(69, 133, 136),
            border_focused: Color::Rgb(142, 192, 124),
            selection_bg: Color::Rgb(80, 73, 69),
            current_bg: Color::Rgb(69, 133, 136),
            accent: Color::Rgb(142, 192, 124),
            accent_alt: Color::Rgb(215, 153, 33),
            playing: Color::Rgb(142, 192, 124),
            paused: Color::Rgb(215, 153, 33),
            info: Color::Rgb(69, 133, 136),
            title: Color::Rgb(204, 36, 29),
            notif_debug: Color::Rgb(235, 219, 178),
            notif_info: Color::Rgb(69, 133, 136),
            notif_warning: Color::Rgb(254, 128, 25),
            notif_error: Color::Rgb(204, 36, 29),
        }
    }

    pub const fn kanagawa() -> Self {
        Self {
            bg: Color::Rgb(46, 50, 87),
            bg_highlight: Color::Rgb(66, 70, 107),
            fg: Color::Rgb(255, 254, 247),
            fg_dim: Color::Rgb(186, 187, 189),
            border: Color::Rgb(98, 125, 154),
            border_focused: Color::Rgb(223, 197, 164),
            selection_bg: Color::Rgb(66, 70, 107),
            current_bg: Color::Rgb(98, 125, 154),
            accent: Color::Rgb(223, 197, 164),
            accent_alt: Color::Rgb(98, 125, 154),
            playing: Color::Rgb(223, 197, 164),
            paused: Color::Rgb(186, 187, 189),
            info: Color::Rgb(98, 125, 154),
            title: Color::Rgb(255, 254, 247),
            notif_debug: Color::Rgb(255, 254, 247),
            notif_info: Color::Rgb(126, 156, 216),
            notif_warning: Color::Rgb(255, 160, 102),
            notif_error: Color::Rgb(195, 64, 67),
        }
    }

    pub const fn hackerman() -> Self {
        Self {
            bg: Color::Rgb(0, 0, 0),
            bg_highlight: Color::Rgb(10, 25, 10),
            fg: Color::Rgb(0, 255, 65),
            fg_dim: Color::Rgb(0, 128, 32),
            border: Color::Rgb(0, 180, 45),
            border_focused: Color::Rgb(57, 255, 20),
            selection_bg: Color::Rgb(0, 50, 12),
            current_bg: Color::Rgb(0, 200, 50),
            accent: Color::Rgb(57, 255, 20),
            accent_alt: Color::Rgb(0, 255, 159),
            playing: Color::Rgb(0, 255, 65),
            paused: Color::Rgb(180, 255, 0),
            info: Color::Rgb(0, 255, 159),
            title: Color::Rgb(57, 255, 20),
            notif_debug: Color::Rgb(200, 200, 200),
            notif_info: Color::Rgb(0, 150, 255),
            notif_warning: Color::Rgb(255, 165, 0),
            notif_error: Color::Rgb(255, 50, 50),
        }
    }

    pub fn notification_color(&self, level: NotificationLevel) -> Color {
        match level {
            NotificationLevel::Debug => self.notif_debug,
            NotificationLevel::Info => self.notif_info,
            NotificationLevel::Warning => self.notif_warning,
            NotificationLevel::Error => self.notif_error,
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
