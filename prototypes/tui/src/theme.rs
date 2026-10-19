use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
    HighContrast,
    Monochrome,
}

impl ThemeMode {
    pub fn next(&self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::HighContrast,
            Self::HighContrast => Self::Monochrome,
            Self::Monochrome => Self::Dark,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Dark => "Dark (Default)",
            Self::Light => "Light",
            Self::HighContrast => "High Contrast",
            Self::Monochrome => "Monochrome / No Color",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    pub bg: Color,
    pub fg: Color,
    pub fg_muted: Color,
    pub accent: Color,
    pub border: Color,
    pub border_focus: Color,
    pub header_bg: Color,
    pub header_fg: Color,
    pub selected_bg: Color,
    pub selected_fg: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub fixture_badge_bg: Color,
    pub fixture_badge_fg: Color,
}

impl Theme {
    pub fn new(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                mode,
                bg: Color::Reset,
                fg: Color::Rgb(220, 224, 232),
                fg_muted: Color::Rgb(140, 145, 160),
                accent: Color::Rgb(114, 135, 253),
                border: Color::Rgb(69, 71, 90),
                border_focus: Color::Rgb(137, 180, 250),
                header_bg: Color::Rgb(30, 30, 46),
                header_fg: Color::Rgb(205, 214, 244),
                selected_bg: Color::Rgb(49, 50, 68),
                selected_fg: Color::Rgb(245, 224, 220),
                success: Color::Rgb(166, 227, 161),
                warning: Color::Rgb(249, 226, 175),
                error: Color::Rgb(243, 139, 168),
                info: Color::Rgb(137, 220, 235),
                fixture_badge_bg: Color::Rgb(250, 179, 135),
                fixture_badge_fg: Color::Rgb(17, 17, 27),
            },
            ThemeMode::Light => Self {
                mode,
                bg: Color::Reset,
                fg: Color::Rgb(76, 79, 105),
                fg_muted: Color::Rgb(140, 143, 161),
                accent: Color::Rgb(30, 102, 245),
                border: Color::Rgb(172, 176, 190),
                border_focus: Color::Rgb(30, 102, 245),
                header_bg: Color::Rgb(230, 233, 239),
                header_fg: Color::Rgb(76, 79, 105),
                selected_bg: Color::Rgb(204, 208, 218),
                selected_fg: Color::Rgb(76, 79, 105),
                success: Color::Rgb(64, 160, 43),
                warning: Color::Rgb(223, 142, 29),
                error: Color::Rgb(210, 15, 57),
                info: Color::Rgb(32, 159, 181),
                fixture_badge_bg: Color::Rgb(254, 100, 11),
                fixture_badge_fg: Color::Rgb(255, 255, 255),
            },
            ThemeMode::HighContrast => Self {
                mode,
                bg: Color::Black,
                fg: Color::White,
                fg_muted: Color::Gray,
                accent: Color::Cyan,
                border: Color::White,
                border_focus: Color::Yellow,
                header_bg: Color::Black,
                header_fg: Color::White,
                selected_bg: Color::White,
                selected_fg: Color::Black,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                fixture_badge_bg: Color::Yellow,
                fixture_badge_fg: Color::Black,
            },
            ThemeMode::Monochrome => Self {
                mode,
                bg: Color::Reset,
                fg: Color::Reset,
                fg_muted: Color::Reset,
                accent: Color::Reset,
                border: Color::Reset,
                border_focus: Color::Reset,
                header_bg: Color::Reset,
                header_fg: Color::Reset,
                selected_bg: Color::Reset,
                selected_fg: Color::Reset,
                success: Color::Reset,
                warning: Color::Reset,
                error: Color::Reset,
                info: Color::Reset,
                fixture_badge_bg: Color::Reset,
                fixture_badge_fg: Color::Reset,
            },
        }
    }

    pub fn title_style(&self) -> Style {
        if self.mode == ThemeMode::Monochrome {
            Style::default().add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.accent)
                .add_modifier(Modifier::BOLD)
        }
    }

    pub fn block_border_style(&self, focused: bool) -> Style {
        let color = if focused {
            self.border_focus
        } else {
            self.border
        };
        Style::default().fg(color)
    }

    pub fn fixture_badge_style(&self) -> Style {
        if self.mode == ThemeMode::Monochrome {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
                .bg(self.fixture_badge_bg)
                .fg(self.fixture_badge_fg)
                .add_modifier(Modifier::BOLD)
        }
    }

    pub fn selected_row_style(&self) -> Style {
        if self.mode == ThemeMode::Monochrome {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
                .bg(self.selected_bg)
                .fg(self.selected_fg)
                .add_modifier(Modifier::BOLD)
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(ThemeMode::Dark)
    }
}
