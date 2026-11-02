use crate::{grbl::{
    configuration::Configuration, helper::Helper, program::Program, status::{
        Mode, State::Alarm, Status,
    },
}, serial::feedback::Feedback};
use crate::logging::ErrorLogger;
use anyhow::Result as AnyhowResult;
use anyhow::Result;
use crossterm::{cursor::MoveTo, execute, terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode}};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Position, Alignment},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{io::stdout, sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
}};
use std::thread::JoinHandle;
use std::{
    io::Stdout,
    sync::mpsc::{Receiver, Sender, channel},
};
use crate::logging::Level as LogLevel;
use super::keys::{EventManager, KeyEvent};

pub struct Frontend {
    program_running: bool,
    gcode: Option<Program>,
    to_cnc: Vec<String>,
    event_manager: EventManager,
    machine_configuration: Configuration,
    pub tx: Sender<Feedback>,
    command_tx: Sender<String>,
    rx: Option<Receiver<Feedback>>,
    command_rx: Option<Receiver<String>>,
    running: Arc<AtomicBool>,
    logger: Option<ErrorLogger>,
    command_helper: Helper,
    first_run: bool,
    cursor_position: u16,
}

pub struct ThreadedFrontend {
    handle: Option<JoinHandle<Terminal<CrosstermBackend<Stdout>>>>,
    terminal: Option<Terminal<CrosstermBackend<Stdout>>>,
}

impl Frontend {
    pub fn new(gcode: Option<Program>, description: String, logger: Option<ErrorLogger>, command_helper: Helper) -> Self {
        let (tx, rx) = channel::<Feedback>();
        let (command_tx, command_rx) = channel::<String>();

        if let Some(log) = &logger {
            if let Err(log_error) = log.log(LogLevel::Info, "Starting frontend") {
                eprintln!("Failed to write frontend error to log file: {log_error}");
            }
        }

        Self {
            program_running: false,
            gcode,
            to_cnc: vec![],
            event_manager: EventManager::new(),
            machine_configuration: Configuration::new(description),
            tx,
            command_tx,
            rx: Some(rx),
            command_rx: Some(command_rx),
            running: Arc::new(AtomicBool::new(true)),
            logger,
            command_helper,
            first_run: true,
            cursor_position: 0,
        }
    }

    fn log(&self, level: LogLevel, error: impl std::fmt::Display) {
        if let Some(logger) = &self.logger {
            if let Err(log_error) = logger.log(level, &error) {
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
                        if let Err(log_error) = logger.log(LogLevel::Error, &error) {
                            eprintln!("Failed to write frontend error to log file: {log_error}");
                        }
                    }
                    return Err(error.into());
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> AnyhowResult<()> {
        if self.first_run {
            terminal.clear()?;
            self.first_run = false;
        }

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

            

            let prompt = "Enter command ";
            let line = Line::from(vec![
                Span::styled(prompt, Style::default().fg(Color::Yellow)),
                Span::raw(<&str>::from(&self.event_manager)),
            ]);
            f.render_widget(Paragraph::new(line), rows[3]);
            f.set_cursor_position(Position::new(
                rows[3].x + prompt.len() as u16 + self.cursor_position,
                rows[3].y,
            ));

            let helper_line = self
                .command_helper
                .get((&self.event_manager).into())
                .map(|help| tui_markdown::from_str(help))
                .unwrap_or_default();
            f.render_widget(
                Paragraph::new(helper_line).block(Block::default().borders(Borders::ALL)),
                rows[2],
            );

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

    fn run_iteration(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> bool {
        let lines: Vec<Feedback> = self
            .rx
            .as_ref()
            .expect("Frontend receiver must be available while running")
            .try_iter()
            .collect();

        for line in lines {
            match line {
                Feedback::Status(status) => {
                    match status {
                        Status::RealTime(status) => {
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
                                    Err(error) => self.log(LogLevel::Error, error),
                                }
                            }
                        }
                        Status::Error(code) => {
                            self.log(LogLevel::Error, format!("GRBL reported error:{code}"));
                            self.to_cnc.push(format!("!ERROR! {code}"));
                        }
                        Status::Alarm(code) => {
                            self.log(LogLevel::Error, format!("GRBL reported ALARM:{code}"));
                            self.to_cnc.push(format!("!ALARM! {code}"));
                        }
                        _ => {}
                    };
                },
                Feedback::MachineConfig(machine_config) => {
                    self.machine_configuration = self.machine_configuration.clone() + machine_config;
                },
                Feedback::Message(msg) => {
                    self.to_cnc.push(msg);
                },
            };
        }

        if let Err(error) = self.draw(terminal) {
            self.log(LogLevel::Error, error);
            self.running.store(false, Ordering::Relaxed);
            return true;
        }

        if let Ok(opt_event) = self.event_manager.iterate() {
            if let Some(event) = opt_event {
                match event {
                    KeyEvent::Command(cmd) => {
                        self.cursor_position = 0;
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
                    },
                    KeyEvent::ProgramStop => {
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
                    },
                    KeyEvent::ProgramRun => {
                        self.program_running = true;
                    },
                    KeyEvent::ProgramPause => {
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
                    },
                    KeyEvent::ProgramModeToggle => {
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
                    },
                    KeyEvent::ProgramStepOver => {
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
                    },
                    KeyEvent::AppExit => {
                        self.running.store(false, Ordering::Relaxed);
                    },
                    KeyEvent::DeviceReset => {
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
                    },
                    KeyEvent::CursorAt(position) => {
                        self.cursor_position = position;
                    }
                };
            }
        }
        true
    }
}

impl ThreadedFrontend {
    pub fn new() -> Self {
        Self { handle: None, terminal: None }
    }

    pub fn start(&mut self, mut frontend: Frontend) -> Result<()> {
        if self.handle.is_some() {
            anyhow::bail!("Application UI has already been started");
        }
        let running = Arc::clone(&frontend.running);

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

        self.handle = Some(std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                if !frontend.run_iteration(&mut terminal) {
                    break;
                }
            }
            terminal
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
            self.terminal = Some(handle
                .join()
                .map_err(|_| anyhow::anyhow!("UI thread panicked"))?);
        }

        disable_raw_mode()?;
        execute!(self.terminal.as_mut().unwrap().backend_mut(), LeaveAlternateScreen)?;
        self.terminal.as_mut().unwrap().show_cursor()?;

        Ok(())
    }
}
