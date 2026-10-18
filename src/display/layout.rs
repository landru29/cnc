use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};
use std::{sync::mpsc::{Receiver, Sender}, time::Duration};
use crossterm::event::KeyModifiers;
use crate::grbl::{machine::MachineConfig, program::Program, status::{RealTime, Status}};
use anyhow::Result as AnyhowResult;

struct App<'a> {
    gcode: Program,
    log: Vec<String>,
    to_cnc: Vec<String>,
    input: String,
    status: RealTime,
    config: MachineConfig,
    end_of_program_reached: bool,
    rx: &'a Receiver<String>,
    tx: &'a Sender<String>,
    description: String,
}

impl<'a> App<'a> {
    fn new(serial_rx: &'a Receiver<String>, serial_tx: &'a Sender<String>, gcode: Program, description: String) -> Self {
        Self {
            gcode,
            log: vec![],
            to_cnc: vec![],
            input: String::new(),
            status: RealTime::default(),
            config: MachineConfig::default(),
            end_of_program_reached:false,
            rx: serial_rx,
            tx: serial_tx,
            description,
        }
    }

    fn send_command(&mut self, cmd: String) -> AnyhowResult<()> {
        if !cmd.is_empty() {
            if cmd.to_lowercase() == "exit" || cmd.to_lowercase() == "quit"  {
                anyhow::bail!("Exiting application")
            } else if cmd.to_lowercase() == "p" {
                if !self.end_of_program_reached && let Err(_) = self.gcode.process(1) {
                    self.end_of_program_reached = true;
                }
            } else {
                if cmd != "?".to_string() && cmd != "$G".to_string() {
                    self.to_cnc.push(format!("> {cmd}"));
                }
                self.tx.send(cmd).expect("Failed to send command to serial port");
            }
        }

        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        // Vertical: status / panels / log / prompt
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // status bar
                Constraint::Min(5),    // top panels
                Constraint::Length(6), // log area
                Constraint::Length(1), // prompt
            ])
            .split(f.area());

        // Status bar
        let status = Line::from(format!(
            "{}     {}     {}    {}    {}    {}",
            self.config.distance,
            self.status
                .state
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.status.position(),
            self.status
                .feed_speed
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.status.buffer
                .as_ref()
                .map(|b| b.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.description
        ));
        f.render_widget(Paragraph::new(status), rows[0]);

        // Two side-by-side panels
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        let to_cnc_visible = rows[1].height.saturating_sub(2) as usize;
        let to_cnc_start = self.to_cnc.len().saturating_sub(to_cnc_visible);
        let to_cnc: Vec<Line> = self.to_cnc[to_cnc_start..].iter().map(|l| Line::from(l.as_str())).collect();
        f.render_widget(
            Paragraph::new(to_cnc).block(Block::default().borders(Borders::ALL)),
            cols[0],
        );

        let (viewport, cursor_in_viewport) = self
            .gcode
            .clone()
            .viewport(rows[1].height as usize - 2);
        
        let lines: Vec<Line> = viewport
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let mut rendered_line = l.clone();
                if !self.end_of_program_reached && i == cursor_in_viewport {
                    rendered_line = format!("> {}", rendered_line);
                } else if i == cursor_in_viewport {
                    rendered_line = format!("█ {}", rendered_line);
                }
                let style = if i == cursor_in_viewport {
                    Style::default().fg(Color::LightBlue).add_modifier(ratatui::style::Modifier::BOLD)
                } else if l.starts_with('(') || l.starts_with('%') || l.starts_with(';') {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(rendered_line, style))
            })
            .collect();
        f.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL)),
            cols[1],
        );

        // Log area (shows the latest lines that fit)
        let log_visible = rows[2].height.saturating_sub(2) as usize;
        let log_start = self.log.len().saturating_sub(log_visible);
        let log: Vec<Line> = self.log[log_start..].iter().map(|l| Line::from(l.as_str())).collect();
        f.render_widget(
            Paragraph::new(log).block(Block::default().borders(Borders::ALL)),
            rows[2],
        );

        // Prompt
        let prompt = "Enter command ";
        let line = Line::from(vec![
            Span::styled(prompt, Style::default().fg(Color::Yellow)),
            Span::raw(self.input.as_str()),
        ]);
        f.render_widget(Paragraph::new(line), rows[3]);
        f.set_cursor_position(Position::new(
            rows[3].x + (prompt.len() + self.input.chars().count()) as u16,
            rows[3].y,
        ));
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        loop {
            while let Ok(line) = self.rx.try_recv() {
                if let Ok(Status::RealTime(status)) = line.parse::<Status>() {
                    self.status = self.status.clone() + status;
                } else if let Ok(machine_config) = line.parse::<MachineConfig>() {
                    self.config = machine_config;
                } else {
                    self.log.push(line);
                }
            }
            
            terminal.draw(|f| self.draw(f))?;
            if !event::poll(Duration::from_millis(50))? {
                continue;
            }
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                match (k.code, k.modifiers) {
                    (KeyCode::Enter, KeyModifiers::NONE) => {
                        let cmd = std::mem::take(&mut self.input);
                        if let Err(_) = self.send_command(cmd) {
                            break;
                        }
                    }
                    (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                        self.input.clear();
                        let ctrl_x: u8 = 0x18;
                        let cmd = format!("{}", ctrl_x as char);
                        if let Err(_) = self.send_command(cmd) {
                            break;
                        }
                    }
                    (KeyCode::Backspace, KeyModifiers::NONE) => {
                        self.input.pop();
                    }
                    (KeyCode::Char(c), KeyModifiers::NONE) => self.input.push(c.to_ascii_uppercase()),
                    (KeyCode::Char(c), KeyModifiers::SHIFT) => self.input.push(c.to_ascii_uppercase()),
                    _ => {}
                }
            }
        }

        Ok(())
    }
}

pub fn display(serial_rx: &Receiver<String>, serial_tx: &Sender<String>, gcode: Program, description: String) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    

    let mut application = App::new(serial_rx, serial_tx, gcode, description);


    let res = application.run(&mut terminal);
    ratatui::restore();
    res
}