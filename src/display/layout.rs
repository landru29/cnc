use crate::grbl::{
    configuration::Configuration, machine::MachineConfig, program::Program, status::Status,
};
use anyhow::Result as AnyhowResult;
use anyhow::Result;
use crossterm::event::KeyModifiers;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread::JoinHandle;
use std::{
    io::Stdout,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration,
};

pub struct Frontend {
    gcode: Program,
    log: Vec<String>,
    to_cnc: Vec<String>,
    input: String,
    machine_configuration: Configuration,
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    pub tx: Sender<String>,
    command_tx: Sender<String>,
    running: Arc<AtomicBool>,
    rx: Option<Receiver<String>>,
    command_rx: Option<Receiver<String>>,
    handle: Option<JoinHandle<()>>,
}

impl Frontend {
    pub fn new(gcode: Program, description: String) -> Self {
        let (tx, rx) = channel::<String>();
        let (command_tx, command_rx) = channel::<String>();
        Self {
            gcode,
            log: vec![],
            to_cnc: vec![],
            input: String::new(),
            machine_configuration: Configuration::new(description),
            terminal: Some(ratatui::init()),
            tx,
            command_tx,
            running: Arc::new(AtomicBool::new(true)),
            rx: Some(rx),
            command_rx: Some(command_rx),
            handle: None,
        }
    }

    fn send_command(
        gcode: &mut Program,
        to_cnc: &mut Vec<String>,
        command_tx: &Sender<String>,
        cmd: String,
    ) -> AnyhowResult<()> {
        if !cmd.is_empty() {
            if cmd.to_lowercase() == "exit" || cmd.to_lowercase() == "quit" {
                anyhow::bail!("Exiting application")
            } else if cmd.to_lowercase() == "p" {
                if !gcode.end_of_program_reached
                    && let Err(_) = gcode.process(1)
                {}
            } else {
                if cmd != "?".to_string() && cmd != "$G".to_string() {
                    to_cnc.push(format!("> {cmd}"));
                }
                command_tx
                    .send(cmd)
                    .expect("Failed to send command to serial port");
            }
        }

        Ok(())
    }

    fn draw(
        f: &mut Frame,
        gcode: &Program,
        log: &[String],
        to_cnc: &[String],
        input: &str,
        machine_configuration: &Configuration,
    ) -> AnyhowResult<()> {
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
        f.render_widget(Paragraph::new(machine_configuration.clone()), rows[0]);

        // Two side-by-side panels
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);

        let to_cnc_visible = rows[1].height.saturating_sub(2) as usize;
        let to_cnc_start = to_cnc.len().saturating_sub(to_cnc_visible);
        let to_cnc: Vec<Line> = to_cnc[to_cnc_start..]
            .iter()
            .map(|l| Line::from(l.as_str()))
            .collect();
        f.render_widget(
            Paragraph::new(to_cnc).block(Block::default().borders(Borders::ALL)),
            cols[0],
        );

        let viewport = gcode.clone().viewport(rows[1].height as usize - 2);

        f.render_widget(
            Paragraph::new(viewport).block(Block::default().borders(Borders::ALL)),
            cols[1],
        );

        // Log area (shows the latest lines that fit)
        let log_visible = rows[2].height.saturating_sub(2) as usize;
        let log_start = log.len().saturating_sub(log_visible);
        let log: Vec<Line> = log[log_start..]
            .iter()
            .map(|l| Line::from(l.as_str()))
            .collect();
        f.render_widget(
            Paragraph::new(log).block(Block::default().borders(Borders::ALL)),
            rows[2],
        );

        // Prompt
        let prompt = "Enter command ";
        let line = Line::from(vec![
            Span::styled(prompt, Style::default().fg(Color::Yellow)),
            Span::raw(input),
        ]);
        f.render_widget(Paragraph::new(line), rows[3]);
        f.set_cursor_position(Position::new(
            rows[3].x + (prompt.len() + input.chars().count()) as u16,
            rows[3].y,
        ));

        Ok(())
    }

    pub fn take_command_receiver(&mut self) -> Result<Receiver<String>> {
        self.command_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("Application command receiver has already been taken"))
    }

    pub fn start(&mut self) -> Result<()> {
        if self.handle.is_some() {
            anyhow::bail!("Application UI has already been started");
        }
        let terminal = self
            .terminal
            .take()
            .ok_or_else(|| anyhow::anyhow!("Application terminal has already been taken"))?;
        let rx = self
            .rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("Application receiver has already been taken"))?;
        let running_thread = Arc::clone(&self.running);
        let mut gcode = self.gcode.clone();
        let log = self.log.clone();
        let mut to_cnc = self.to_cnc.clone();
        let mut input = self.input.clone();
        let mut machine_configuration = self.machine_configuration.clone();
        let command_tx = self.command_tx.clone();
        self.handle = Some(std::thread::spawn(move || {
            let mut terminal = terminal;
            while running_thread.load(Ordering::Relaxed) {
                while let Ok(line) = rx.try_recv() {
                    if let Ok(Status::RealTime(status)) = line.parse::<Status>() {
                        machine_configuration = machine_configuration.clone() + status;
                    } else if let Ok(machine_config) = line.parse::<MachineConfig>() {
                        machine_configuration = machine_configuration.clone() + machine_config;
                    } else if line != "ok" {
                        to_cnc.push(line);
                    }
                }

                if let Err(_) = terminal.draw(|f| {
                    if let Err(_) =
                        Self::draw(f, &gcode, &log, &to_cnc, &input, &machine_configuration)
                    {
                    }
                }) {
                    running_thread.store(false, Ordering::Relaxed);
                    continue;
                };

                match event::poll(Duration::from_millis(50)) {
                    Ok(value) => {
                        if !value {
                            continue;
                        }
                    }
                    Err(_) => {
                        running_thread.store(false, Ordering::Relaxed);
                        continue;
                    }
                };

                if let Ok(Event::Key(k)) = event::read() {
                    if k.kind != KeyEventKind::Press {
                        continue;
                    }
                    match (k.code, k.modifiers) {
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            let cmd = std::mem::take(&mut input);
                            if let Err(_) =
                                Self::send_command(&mut gcode, &mut to_cnc, &command_tx, cmd)
                            {
                                break;
                            }
                        }
                        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                            running_thread.store(false, Ordering::Relaxed);
                        }
                        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                            input.clear();
                            let ctrl_x: u8 = 0x18;
                            let cmd = format!("{}", ctrl_x as char);
                            if let Err(_) =
                                Self::send_command(&mut gcode, &mut to_cnc, &command_tx, cmd)
                            {
                                break;
                            }
                        }
                        (KeyCode::Backspace, KeyModifiers::NONE) => {
                            input.pop();
                        }
                        (KeyCode::Char(c), KeyModifiers::NONE) => {
                            input.push(c.to_ascii_uppercase())
                        }
                        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                            input.push(c.to_ascii_uppercase())
                        }
                        _ => {}
                    }
                }
            }
        }));
        Ok(())
    }

    pub fn wait(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("UI thread panicked"))?;
        }
        Ok(())
    }
}
