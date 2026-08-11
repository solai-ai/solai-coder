use crate::color::blend;
use crate::color::is_light;
use crate::terminal_palette::StdoutColorLevel;
use crate::terminal_palette::best_color;
use crate::terminal_palette::default_bg;
use crate::terminal_palette::default_fg;
use crate::terminal_palette::rgb_color;
use crate::terminal_palette::stdout_color_level;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;

const LIGHT_BG_ACCENT_RGB: (u8, u8, u8) = (0, 95, 135);
const SOLAI_LOGO_RGB: [(u8, u8, u8); 5] = [
    (0, 128, 255),
    (112, 64, 255),
    (255, 0, 255),
    (255, 36, 96),
    (255, 128, 0),
];
const SOLAI_LOGO_ANSI: [Color; 5] = [
    Color::Blue,
    Color::LightBlue,
    Color::Magenta,
    Color::LightRed,
    Color::Yellow,
];
// Decorative table rules should remain visible without competing with cell content.
const TABLE_SEPARATOR_FG_ALPHA: f32 = 0.20;

pub fn user_message_style() -> Style {
    user_message_style_for(default_bg())
}

pub fn proposed_plan_style() -> Style {
    proposed_plan_style_for(default_bg())
}

/// Returns a low-contrast rule style for separators within markdown tables.
pub(crate) fn table_separator_style() -> Style {
    table_separator_style_for(default_fg(), default_bg(), stdout_color_level())
}

/// Returns the shared accent style for active or selected TUI controls.
pub(crate) fn accent_style() -> Style {
    accent_style_for(default_bg())
}

/// Returns one color from the SOLAI logo accent sequence.
pub(crate) fn solai_logo_style(index: usize) -> Style {
    solai_logo_style_for(index, stdout_color_level())
}

/// Returns a SOLAI logo style interpolated across a horizontal text run.
pub(crate) fn solai_logo_gradient_style(position: usize, width: usize) -> Style {
    solai_logo_gradient_style_for(position, width, stdout_color_level())
}

/// Returns the style for a user-authored message using the provided terminal background.
pub fn user_message_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    match terminal_bg {
        Some(bg) => Style::default().bg(user_message_bg(bg)),
        None => Style::default(),
    }
}

pub fn proposed_plan_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    match terminal_bg {
        Some(bg) => Style::default().bg(proposed_plan_bg(bg)),
        None => Style::default(),
    }
}

/// Returns the shared accent style for the provided terminal background.
pub(crate) fn accent_style_for(terminal_bg: Option<(u8, u8, u8)>) -> Style {
    if terminal_bg.is_some_and(is_light) {
        Style::default().fg(best_color(LIGHT_BG_ACCENT_RGB)).bold()
    } else {
        Style::default().fg(Color::Cyan).bold()
    }
}

pub(crate) fn solai_logo_style_for(index: usize, color_level: StdoutColorLevel) -> Style {
    let color_index = index % SOLAI_LOGO_RGB.len();
    let color = match color_level {
        StdoutColorLevel::TrueColor => rgb_color(SOLAI_LOGO_RGB[color_index]),
        StdoutColorLevel::Ansi256 => best_color(SOLAI_LOGO_RGB[color_index]),
        StdoutColorLevel::Ansi16 | StdoutColorLevel::Unknown => SOLAI_LOGO_ANSI[color_index],
    };
    Style::default().fg(color)
}

pub(crate) fn solai_logo_gradient_style_for(
    position: usize,
    width: usize,
    color_level: StdoutColorLevel,
) -> Style {
    if width <= 1 {
        return solai_logo_style_for(/*index*/ 0, color_level);
    }

    let max_position = width.saturating_sub(1);
    let clamped = position.min(max_position);
    let segment_count = SOLAI_LOGO_RGB.len() - 1;
    let color = if clamped == max_position {
        SOLAI_LOGO_RGB[segment_count]
    } else {
        let scaled = clamped * segment_count;
        let segment = scaled / max_position;
        let offset = scaled % max_position;
        let alpha = offset as f32 / max_position as f32;
        interpolate_rgb(SOLAI_LOGO_RGB[segment], SOLAI_LOGO_RGB[segment + 1], alpha)
    };

    let fg = match color_level {
        StdoutColorLevel::TrueColor => rgb_color(color),
        StdoutColorLevel::Ansi256 => best_color(color),
        StdoutColorLevel::Ansi16 | StdoutColorLevel::Unknown => {
            let color_index = clamped * SOLAI_LOGO_ANSI.len() / width;
            SOLAI_LOGO_ANSI[color_index.min(SOLAI_LOGO_ANSI.len() - 1)]
        }
    };
    Style::default().fg(fg)
}

fn interpolate_rgb(from: (u8, u8, u8), to: (u8, u8, u8), alpha: f32) -> (u8, u8, u8) {
    let interpolate =
        |start: u8, end: u8| (start as f32 + (end as f32 - start as f32) * alpha).round() as u8;
    (
        interpolate(from.0, to.0),
        interpolate(from.1, to.1),
        interpolate(from.2, to.2),
    )
}

fn table_separator_style_for(
    terminal_fg: Option<(u8, u8, u8)>,
    terminal_bg: Option<(u8, u8, u8)>,
    color_level: StdoutColorLevel,
) -> Style {
    let (Some(fg), Some(bg)) = (terminal_fg, terminal_bg) else {
        return Style::default().dim();
    };
    let separator_rgb = blend(fg, bg, TABLE_SEPARATOR_FG_ALPHA);
    match color_level {
        StdoutColorLevel::TrueColor => Style::default().fg(rgb_color(separator_rgb)),
        StdoutColorLevel::Ansi256 => Style::default().fg(best_color(separator_rgb)),
        StdoutColorLevel::Ansi16 | StdoutColorLevel::Unknown => Style::default().dim(),
    }
}

#[allow(clippy::disallowed_methods)]
pub fn user_message_bg(terminal_bg: (u8, u8, u8)) -> Color {
    let (top, alpha) = if is_light(terminal_bg) {
        ((0, 0, 0), 0.04)
    } else {
        ((255, 255, 255), 0.12)
    };
    best_color(blend(top, terminal_bg, alpha))
}

#[allow(clippy::disallowed_methods)]
pub fn proposed_plan_bg(terminal_bg: (u8, u8, u8)) -> Color {
    user_message_bg(terminal_bg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use ratatui::style::Modifier;

    #[test]
    fn accent_style_uses_darker_cyan_on_light_backgrounds() {
        let style = accent_style_for(Some((255, 255, 255)));

        assert_eq!(style.fg, Some(best_color(LIGHT_BG_ACCENT_RGB)));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn accent_style_uses_cyan_on_dark_or_unknown_backgrounds() {
        let expected = Style::default().fg(Color::Cyan).bold();

        assert_eq!(accent_style_for(Some((0, 0, 0))), expected);
        assert_eq!(accent_style_for(/*terminal_bg*/ None), expected);
    }

    #[test]
    fn solai_logo_style_uses_logo_truecolor_sequence() {
        assert_eq!(
            solai_logo_style_for(/*index*/ 0, StdoutColorLevel::TrueColor).fg,
            Some(rgb_color((0, 128, 255)))
        );
        assert_eq!(
            solai_logo_style_for(/*index*/ 1, StdoutColorLevel::TrueColor).fg,
            Some(rgb_color((112, 64, 255)))
        );
        assert_eq!(
            solai_logo_style_for(/*index*/ 4, StdoutColorLevel::TrueColor).fg,
            Some(rgb_color((255, 128, 0)))
        );
        assert_eq!(
            solai_logo_style_for(/*index*/ 5, StdoutColorLevel::TrueColor).fg,
            Some(rgb_color((0, 128, 255)))
        );
    }

    #[test]
    fn solai_logo_style_falls_back_to_ansi_sequence() {
        assert_eq!(
            solai_logo_style_for(/*index*/ 0, StdoutColorLevel::Ansi16).fg,
            Some(Color::Blue)
        );
        assert_eq!(
            solai_logo_style_for(/*index*/ 1, StdoutColorLevel::Unknown).fg,
            Some(Color::LightBlue)
        );
        assert_eq!(
            solai_logo_style_for(/*index*/ 4, StdoutColorLevel::Ansi16).fg,
            Some(Color::Yellow)
        );
    }

    #[test]
    fn solai_logo_gradient_style_interpolates_across_width() {
        assert_eq!(
            solai_logo_gradient_style_for(0, 5, StdoutColorLevel::TrueColor).fg,
            Some(rgb_color((0, 128, 255)))
        );
        assert_eq!(
            solai_logo_gradient_style_for(4, 5, StdoutColorLevel::TrueColor).fg,
            Some(rgb_color((255, 128, 0)))
        );
        assert_eq!(
            solai_logo_gradient_style_for(2, 5, StdoutColorLevel::TrueColor).fg,
            Some(rgb_color((255, 0, 255)))
        );
    }

    #[test]
    fn table_separator_blends_toward_dark_background() {
        let style = table_separator_style_for(
            Some((255, 255, 255)),
            Some((0, 0, 0)),
            StdoutColorLevel::TrueColor,
        );

        assert_eq!(style.fg, Some(rgb_color((51, 51, 51))));
    }

    #[test]
    fn table_separator_blends_toward_light_background() {
        let style = table_separator_style_for(
            Some((0, 0, 0)),
            Some((255, 255, 255)),
            StdoutColorLevel::TrueColor,
        );

        assert_eq!(style.fg, Some(rgb_color((204, 204, 204))));
    }

    #[test]
    fn table_separator_dims_when_palette_aware_color_is_unavailable() {
        let expected = Style::default().dim();

        assert_eq!(
            table_separator_style_for(
                Some((255, 255, 255)),
                Some((0, 0, 0)),
                StdoutColorLevel::Ansi16,
            ),
            expected
        );
        assert_eq!(
            table_separator_style_for(
                /*terminal_fg*/ None,
                Some((0, 0, 0)),
                StdoutColorLevel::TrueColor,
            ),
            expected
        );
    }
}
