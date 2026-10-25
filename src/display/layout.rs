use crate::grbl::{
    configuration::Configuration, machine::MachineConfig, program::Program, status::{Mode, State::Alarm, Status},
};
use crate::logging::ErrorLogger;
use anyhow::Result as AnyhowResult;
use anyhow::Result;
use crossterm::event::KeyModifiers;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout, Position, Alignment},
    style::{Color, Style, Modifier},
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
    program_running: bool,
    gcode: Option<Program>,
    helper: Vec<String>,
    to_cnc: Vec<String>,
    input: String,
    machine_configuration: Configuration,
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
    pub tx: Sender<String>,
    command_tx: Sender<String>,
    rx: Option<Receiver<String>>,
    command_rx: Option<Receiver<String>>,
    running: Arc<AtomicBool>,
    logger: Option<ErrorLogger>,
}

pub struct ThreadedFrontend {
    handle: Option<JoinHandle<()>>,
}

impl Frontend {
    pub fn new(gcode: Option<Program>, description: String, logger: Option<ErrorLogger>) -> Self {
        let (tx, rx) = channel::<String>();
        let (command_tx, command_rx) = channel::<String>();
        Self {
            program_running: false,
            gcode,
            helper: vec![],
            to_cnc: vec![],
            input: String::new(),
            machine_configuration: Configuration::new(description),
            terminal: Some(ratatui::init()),
            tx,
            command_tx,
            rx: Some(rx),
            command_rx: Some(command_rx),
            running: Arc::new(AtomicBool::new(true)),
            logger,
        }
    }

    fn log_error(&self, error: impl std::fmt::Display) {
        if let Some(logger) = &self.logger {
            if let Err(log_error) = logger.log_error(&error) {
                eprintln!("Failed to write frontend error to log file: {log_error}");
            }
        }
    }

    fn send_command(
        to_cnc: Option<&mut Vec<String>>,
        command_tx: &Sender<String>,
        logger: Option<&ErrorLogger>,
        cmd: String,
    ) -> AnyhowResult<()> {
        if !cmd.is_empty() {
            if cmd.to_lowercase() == "exit" || cmd.to_lowercase() == "quit" {
                anyhow::bail!("Exiting application")
            } else {
                if let Some(echo) = to_cnc {
                    echo.push(format!("> {cmd}"));
                }
                if let Err(error) = command_tx.send(cmd) {
                    if let Some(logger) = logger {
                        if let Err(log_error) = logger.log_error(&error) {
                            eprintln!("Failed to write frontend error to log file: {log_error}");
                        }
                    }
                    return Err(error.into());
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self) -> AnyhowResult<()> {
        let terminal = self.terminal
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Application terminal has already been taken"))?;

        terminal.draw(|f| {
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

            f.render_widget(Paragraph::new(self.machine_configuration.clone()), rows[0]);

            let cols = if let None = self.gcode {
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

            let to_cnc_visible = rows[1].height.saturating_sub(2) as usize;
            let to_cnc_start = self.to_cnc.len().saturating_sub(to_cnc_visible);
            let visible_commands: Vec<Line> = self.to_cnc[to_cnc_start..]
                .iter()
                .map(|l| Line::from(l.as_str()))
                .collect();
            f.render_widget(
                Paragraph::new(visible_commands).block(Block::default().borders(Borders::ALL)),
                cols[0],
            );

            if let Some(prog) = self.gcode.clone() {
                let viewport = prog.to_owned().viewport(rows[1].height as usize - 2);
                f.render_widget(
                    Paragraph::new(viewport).block(Block::default().borders(Borders::ALL)),
                    cols[1],
                );
            }

            let helper_lines: Vec<Line> = self.helper
                .iter()
                .map(|l| Line::from(l.as_str()))
                .collect();
            f.render_widget(
                Paragraph::new(helper_lines).block(Block::default().borders(Borders::ALL)),
                rows[2],
            );

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

            let mut keys = vec![
                ("↑/↓", "History"),
                ("Ctrl+D", "Quit"),
                ("Ctrl+X", "Reset"),
            ];

            if let Some(_) = self.gcode {
                keys.extend([
                    ("F4", "Stop"),
                    ("F5", "Run"),
                    (
                        "F7",
                        if self.machine_configuration.status.step_mode == Mode::Step {
                            "Prog mode"
                        } else {
                            "Step mode"
                        },
                    ),
                ]);

                if self.machine_configuration.status.step_mode == Mode::Step {
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

    pub fn take_command_receiver(&mut self) -> Result<Receiver<String>> {
        self.command_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("Application command receiver has already been taken"))
    }

    fn run_iteration(&mut self) -> bool {
        let lines: Vec<String> = self
            .rx
            .as_ref()
            .expect("Frontend receiver must be available while running")
            .try_iter()
            .collect();

        for line in lines {
            match line.parse::<Status>() {
                Ok(Status::RealTime(status)) => {
                    self.machine_configuration = self.machine_configuration.clone() + status;

                    if self.program_running
                        && let Some(prog) = self.gcode.as_mut()
                        && !prog.end_of_program_reached
                        && let Some(buffer) = self.machine_configuration.status.buffer.as_ref()
                        && let Some(state) = self.machine_configuration.status.state.clone()
                        && state != Alarm
                    {
                        self.to_cnc.push("------------".to_string());
                        match prog.process(buffer.lines as usize) {
                            Ok(data) => {
                                data.iter().for_each(|command| {
                                    self.to_cnc.push(command.clone());
                                    if Self::send_command(
                                        None,
                                        &self.command_tx,
                                        self.logger.as_ref(),
                                        command.clone(),
                                    )
                                    .is_err()
                                    {
                                        self.program_running = false;
                                    }
                                });
                            }
                            Err(error) => self.log_error(error),
                        }
                    }
                }
                Ok(Status::Error(code)) => {
                    self.log_error(format!("GRBL reported error:{code}"));
                    self.to_cnc.push(line);
                }
                Ok(Status::Alarm(code)) => {
                    self.log_error(format!("GRBL reported ALARM:{code}"));
                    self.to_cnc.push(line);
                }
                _ => {
                    if let Ok(machine_config) = line.parse::<MachineConfig>() {
                        self.machine_configuration = self.machine_configuration.clone() + machine_config;
                    } else if line.to_ascii_lowercase().trim() != "ok" {
                        self.to_cnc.push(line);
                    }
                }
            }
        }

        if let Err(error) = self.draw() {
            self.log_error(error);
            self.running.store(false, Ordering::Relaxed);
            return true;
        }

        match event::poll(Duration::from_millis(50)) {
            Ok(false) => return true,
            Ok(true) => {}
            Err(error) => {
                self.log_error(error);
                self.running.store(false, Ordering::Relaxed);
                return true;
            }
        }

        let event = match event::read() {
            Ok(event) => event,
            Err(error) => {
                self.log_error(error);
                self.running.store(false, Ordering::Relaxed);
                return true;
            }
        };

        if let Event::Key(k) = event {
            if k.kind != KeyEventKind::Press {
                return true;
            }
            match (k.code, k.modifiers) {
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    let cmd = std::mem::take(&mut self.input);
                    if Self::send_command(
                        Some(&mut self.to_cnc),
                        &self.command_tx,
                        self.logger.as_ref(),
                        cmd,
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                (KeyCode::F(4), KeyModifiers::NONE) => { // Stop
                    self.program_running = true;
                    for cmd in vec!["!".to_string(), Self::ctrl_x()].iter() {
                        if Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            self.logger.as_ref(),
                            cmd.clone(),
                        )
                        .is_err()
                        {
                            break;
                        }
                    }
                }
                (KeyCode::F(5), KeyModifiers::NONE) => { // Run
                    self.program_running = true;
                }
                (KeyCode::F(6), KeyModifiers::NONE) => { // Pause
                    if Self::send_command(
                        Some(&mut self.to_cnc),
                        &self.command_tx,
                        self.logger.as_ref(),
                        "!".to_string(),
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                (KeyCode::F(7), KeyModifiers::NONE) => { // Step mode / prog mode
                    if Self::send_command(
                        Some(&mut self.to_cnc),
                        &self.command_tx,
                        self.logger.as_ref(),
                        "$GB".to_string(),
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                (KeyCode::F(8), KeyModifiers::NONE) => { // Step Over
                    if Self::send_command(
                        Some(&mut self.to_cnc),
                        &self.command_tx,
                        self.logger.as_ref(),
                        "~".to_string(),
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                    self.running.store(false, Ordering::Relaxed);
                }
                (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                    self.input.clear();
                    self.program_running = false;
                    if Self::send_command(
                        None,
                        &self.command_tx,
                        self.logger.as_ref(),
                        Self::ctrl_x(),
                    )
                    .is_err()
                    {
                        return false;
                    }
                }
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    self.input.pop();
                }
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    self.input.push(c.to_ascii_uppercase())
                }
                _ => {}
            }
        }
        true
    }
}

impl ThreadedFrontend {
    pub fn new() -> Self {
        Self { handle: None }
    }

    pub fn start(&mut self, mut frontend: Frontend) -> Result<()> {
        if self.handle.is_some() {
            anyhow::bail!("Application UI has already been started");
        }
        let running = Arc::clone(&frontend.running);

        self.handle = Some(std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                if !frontend.run_iteration() {
                    break;
                }
            }
        }));
        Ok(())
    }
}

impl Frontend {
    fn ctrl_x() -> String {
        format!("{}", 0x18 as char)
    }
}

impl ThreadedFrontend {
    pub fn wait(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("UI thread panicked"))?;
        }
        Ok(())
    }
}
