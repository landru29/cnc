use anyhow::Result as AnyhowResult;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    sync::mpsc::{
        Receiver,
        Sender,
        channel
    },
};

use crate::{
    frontend::layout::Frontend, grbl::{
        configuration::Configuration, 
        helper::Helper, 
        program::Program, 
        status::{
            State, 
            Status,
        },
    }, serial::feedback::Feedback
};
use crate::logging::ErrorLogger;
use crate::logging::Level as LogLevel;

use super::keys::{
    EventManager,
    KeyEvent,
};


pub struct Application {
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
    cursor_position: u16,
}

pub struct ThreadedApp {
    handle: Option<JoinHandle<Frontend>>,
    terminal: Option<Frontend>,
}

impl Application {
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

    pub fn take_command_receiver(&mut self) -> AnyhowResult<Receiver<String>> {
        self.command_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("Application command receiver has already been taken"))
    }

    fn user_commande_listen(&mut self) -> AnyhowResult<()> {
        if let Ok(opt_event) = self.event_manager.iterate() {
            if let Some(event) = opt_event {
                match event {
                    KeyEvent::Command(cmd) => {
                        self.cursor_position = 0;
                        Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            self.logger.as_ref(),
                            cmd,
                        )?;
                    },
                    KeyEvent::ProgramStop => {
                        self.program_running = true;
                        for cmd in vec!["!".to_string(), Self::ctrl_x()].iter() {
                            Self::send_command(
                                Some(&mut self.to_cnc),
                                &self.command_tx,
                                self.logger.as_ref(),
                                cmd.clone(),
                            )?;
                        }
                    },
                    KeyEvent::ProgramRun => {
                        if let Some(state) = &self.machine_configuration.status.state {
                            match state {
                                State::Idle => {
                                    self.program_running = true;
                                }
                                _ => {},
                            };
                        };
                    },
                    KeyEvent::ProgramPause => {
                        Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            self.logger.as_ref(),
                            "!".to_string(),
                        )?;
                    },
                    KeyEvent::ProgramModeToggle => {
                        Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            self.logger.as_ref(),
                            "$GB".to_string(),
                        )?;
                    },
                    KeyEvent::ProgramStepOver => {
                        Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            self.logger.as_ref(),
                            "~".to_string(),
                        )?;
                    },
                    KeyEvent::AppExit => {
                        self.running.store(false, Ordering::Relaxed);
                    },
                    KeyEvent::DeviceReset => {
                        self.program_running = false;
                        Self::send_command(
                            None,
                            &self.command_tx,
                            self.logger.as_ref(),
                            Self::ctrl_x(),
                        )?;
                    },
                    KeyEvent::CursorAt(position) => {
                        self.cursor_position = position;
                    }
                };
            }
        }

        Ok(())
    }

    fn process_program(&mut self) -> AnyhowResult<()>{
        if self.gcode.is_none() {
            self.program_running = false;
        }

        if !self.program_running {
            return Ok(())
        }

        let prog = self.gcode.as_mut().ok_or(anyhow::anyhow!("missing program"))?;

        if prog.eop_reached_in_machine_buffer {
            self.program_running = false;
            return Ok(());
        }

        let buffer = self.machine_configuration.status.buffer.as_ref().ok_or(anyhow::anyhow!("missing buffer information"))?;
        let state = self.machine_configuration.status.state.clone().ok_or(anyhow::anyhow!("missing machine state"))?;

        if state == State::Alarm {
            return Err(anyhow::anyhow!("machine is in alarm state"));
        }

        prog.set_initial_machine_buffer_size(buffer.lines as usize);

        self.to_cnc.push("------------".to_string());
        match prog.process(&buffer) {
            Ok(data) => {
                data.iter().for_each(|command| {
                    self.to_cnc.push(command.clone());
                    self.log(LogLevel::Info, format!(">> {}", command.clone()));
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

        Ok(())
    }

    fn run_iteration(&mut self, frontend: &mut Frontend/*terminal: &mut Terminal<CrosstermBackend<Stdout>>*/) -> AnyhowResult<()> {
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

                            self.process_program()?;
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

        frontend.draw(
            &self.machine_configuration,
            &self.to_cnc,
            self.gcode.as_ref(),
            &self.event_manager,
            self.cursor_position,
            &self.command_helper,
            &self.program_running,
        ).map_err(|e| {
            self.log(LogLevel::Error, &e);
            self.running.store(false, Ordering::Relaxed);

            anyhow::anyhow!("display error: {}", &e)
        })?;

        self.user_commande_listen()?;
        
        Ok(())
    }
}

impl ThreadedApp {
    pub fn new() -> Self {
        Self { handle: None, terminal: None }
    }

    pub fn start(&mut self, mut app: Application) -> AnyhowResult<()> {
        if self.handle.is_some() {
            anyhow::bail!("Application UI has already been started");
        }
        let running = Arc::clone(&app.running);

        let mut frontend = Frontend::new()?;

        self.handle = Some(std::thread::spawn(move || {
            while running.load(Ordering::Relaxed) {
                if app.run_iteration(&mut frontend).is_err() {
                    break;
                }
            }
            frontend
        }));

        Ok(())
    }
}

impl Application {
    fn ctrl_x() -> String {
        format!("{}", 0x18 as char)
    }
}

impl ThreadedApp {
    pub fn wait(&mut self) -> AnyhowResult<()> {
        if let Some(handle) = self.handle.take() {
            self.terminal = Some(handle
                .join()
                .map_err(|_| anyhow::anyhow!("UI thread panicked"))?);
        }

        self.terminal.as_mut().unwrap().leave()?;

        Ok(())
    }
}
