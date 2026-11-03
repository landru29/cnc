use ratatui::style::{Color, Modifier, Style};
use tui_markdown::StyleSheet;

#[derive(Clone, Debug)]
pub struct HelperStyle;

impl StyleSheet for HelperStyle {
    fn heading(&self, level: u8) -> Style {
        match level {
            1 => Style::new()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            2 => Style::new().fg(Color::Cyan).bold(),
            3 => Style::new().fg(Color::Green).italic(),
            _ => Style::new().fg(Color::Gray).bold(),
        }
    }

    fn code(&self) -> Style {
        Style::new().white().on_black().add_modifier(Modifier::ITALIC)
    }

    fn link(&self) -> Style {
        Style::new().blue().underlined()
    }

    fn blockquote(&self) -> Style {
        Style::new().green()
    }

    fn heading_meta(&self) -> Style {
        Style::new().dim()
    }

    fn metadata_block(&self) -> Style {
        Style::new().light_yellow()
    }
}