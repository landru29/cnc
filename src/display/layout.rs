use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

struct App {
    gcode: Vec<String>,
    current_line: usize,
    log: Vec<String>,
    to_cnc: Vec<String>,
    input: String,
    pos: [f32; 3],
    state: &'static str,
}

impl App {
    fn new(gcode: Vec<String>) -> Self {
        Self {
            gcode,
            current_line: 3,
            log: vec![],
            to_cnc: vec![],
            input: String::new(),
            pos: [0.0; 3],
            state: "WAITING",
        }
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
            "ABS     {}     X: {:+08.2}      Y: {:+08.2}      Z: {:+08.2}",
            self.state, self.pos[0], self.pos[1], self.pos[2]
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

        let lines: Vec<Line> = self
            .gcode
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let mut rendered_line = l.clone();
                if i == self.current_line {
                    if rendered_line.trim().is_empty() || rendered_line.trim().starts_with(';') || rendered_line.trim().starts_with('%') || rendered_line.trim().starts_with('(') {
                        self.current_line = self.current_line + 1;
                    } else {
                        rendered_line = format!("> {}", rendered_line);
                    }
                }
                let style = if i == self.current_line {
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
            terminal.draw(|f| self.draw(f))?;
            if let Event::Key(k) = event::read()? {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                match k.code {
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Enter => {
                        let cmd = std::mem::take(&mut self.input);
                        if !cmd.is_empty() {
                            if cmd.to_lowercase() == "exit" {
                                return Ok(());
                            } else if cmd.to_lowercase() == "p" {
                                self.current_line = self.current_line + 1;
                            } else {
                                self.to_cnc.push(format!("> {cmd}"));
                            }
                        }
                    }
                    KeyCode::Backspace => {
                        self.input.pop();
                    }
                    KeyCode::Char(c) => self.input.push(c),
                    _ => {}
                }
            }
        }
    }
}

pub fn display(gcode: Vec<String>) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    

    let mut application = App::new(gcode);


    let res = application.run(&mut terminal);
    ratatui::restore();
    res
}