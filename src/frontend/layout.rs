use anyhow::Result as AnyhowResult;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{
        Clear,
        ClearType,
        EnterAlternateScreen,
        LeaveAlternateScreen,
        disable_raw_mode,
        enable_raw_mode
    }
};

use std::io::{
    stdout,
    Stdout,
};
use tui_markdown;
use crate::grbl::{
    configuration::Configuration, helper::Helper, program::Program, status::{
        Mode, State,
    },
};
use crate::application::keys::EventManager;


use super::stylesheet::HelperStyle;

pub struct Frontend {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    first_run: bool,
}

impl Frontend {
    pub fn new() -> AnyhowResult<Self> {
        enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(
            stdout,
            EnterAlternateScreen,
            Clear(ClearType::All),
            MoveTo(0, 0)
        )?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        Ok(Self { terminal, first_run: true })
    }

    pub fn leave(&mut self) -> AnyhowResult<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;

        Ok(())
    }

    pub fn draw(
        &mut self,
        machine_configuration: &Configuration,
        to_cnc: &[String],
        gcode: Option<&mut Program>,
        event_manager: &EventManager,
        cursor_position: u16,
        command_helper: &Helper,
        program_running: &bool,
    ) -> AnyhowResult<()> {
        if self.first_run {
            let _ = self.terminal.clear();
            self.first_run = false;
        }

        self.terminal.draw(|f| {
            // Vertical: status / panels / helper / prompt
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // status bar
                    Constraint::Min(5),    // top panels
                    Constraint::Length(6), // helper area
                    Constraint::Length(1), // prompt
                    Constraint::Length(1), // command helper
                ])
                .split(f.area());

            // ************************************
            // **      Header: status bar        **
            // ************************************
            f.render_widget(Paragraph::new(machine_configuration.clone()), rows[0]);


            // ************************************
            // **          Double panel          **
            // ************************************
            let cols = if gcode.is_none() {
                    Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(rows[1])
            } else {
                Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rows[1])
            };

            // ************************************
            // **      CNC commands display      **
            // ************************************
            let to_cnc_visible = rows[1].height.saturating_sub(2) as usize;
            let to_cnc_start = to_cnc.len().saturating_sub(to_cnc_visible);
            let visible_commands: Vec<Line> = to_cnc[to_cnc_start..]
                .iter()
                .map(|l| Line::from(l.as_str()))
                .collect();
            f.render_widget(
                Paragraph::new(visible_commands).block(Block::default().borders(Borders::ALL)),
                cols[0],
            );

            // ************************************
            // **            Program             **
            // ************************************
            let has_gcode = gcode.is_some();
            if let Some(prog) = gcode {
                let buffer = machine_configuration.status.buffer.clone().unwrap_or_default();
                let viewport = prog
                    .viewport(rows[1].height as usize - 2, &buffer);
                f.render_widget(
                    Paragraph::new(viewport).block(Block::default().borders(Borders::ALL)),
                    cols[1],
                );
            }

            // ************************************
            // **         Command prompt         **
            // ************************************
            let prompt = "Enter command ";
            let line = Line::from(vec![
                Span::styled(prompt, Style::default().fg(Color::Yellow)),
                Span::raw(<&str>::from(event_manager)),
            ]);
            f.render_widget(Paragraph::new(line), rows[3]);
            f.set_cursor_position(Position::new(
                rows[3].x + prompt.len() as u16 + cursor_position,
                rows[3].y,
            ));

            // ************************************
            // **         Command helper         **
            // ************************************
            let helper_line = command_helper
                .get(event_manager.into())
                .map(|help| tui_markdown::from_str_with_options(help, &tui_markdown::Options::new(HelperStyle)))
                .unwrap_or_default();
            f.render_widget(
                Paragraph::new(helper_line).block(Block::default().borders(Borders::ALL)),
                rows[2],
            );

            // ************************************
            // **          Footer: keys          **
            // ************************************
            let mut keys = vec![
                ("↑/↓", "History"),
                ("Ctrl+D", "Quit"),
                ("Ctrl+X", "Reset"),
            ];

            let status = &machine_configuration.status.clone().state.map_or(State::Idle, |x| x);

            if status == &State::Alarm {
                keys.push(("F4", "Stop"));
            }

            if has_gcode {
                if !*program_running {
                    keys.push(("F5", "Run"));
                }

                keys.push((
                    "F7",
                    match machine_configuration.status.step_mode {
                        Mode::Step => "Prog mode",
                        _ => "Step mode"
                    },
                ));

                if machine_configuration.status.step_mode == Mode::Step {
                    keys.push(("F8", "Step over"));
                }
            }

            let key_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            let desc_style = Style::default().fg(Color::Gray);

            let mut spans = Vec::new();
            for (key, desc) in keys {
                spans.push(Span::styled(format!(" {key} "), key_style));
                spans.push(Span::styled(format!(" {desc}  "), desc_style));
            }

            f.render_widget(
                Paragraph::new(Line::from(spans)).alignment(Alignment::Right),
                rows[4],
            );
        })?;
        Ok(())
    }

}

