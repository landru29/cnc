use anyhow::Result as AnyhowResult;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    sync::mpsc::{
        Receiver,
        Sender,
        channel
    },
};

use crate::{
    frontend::layout::Frontend, grbl::{
        configuration::Configuration, helper::Helper, program::Program, status::{
            State,
            Status,
        },
    }, serial::feedback::Feedback
};
use crate::logging::Logger;
use crate::logging::Level as LogLevel;

use super::{
    keys::{
        EventManager,
        KeyEvent,
    },
    machine::MachineData,
};

use log::{info, error};

pub struct Application {
    gcode: Option<Program>,
    to_cnc: Vec<MachineData>,
    event_manager: EventManager,
    machine_configuration: Configuration,
    pub tx: Sender<Feedback>,
    command_tx: Sender<String>,
    rx: Option<Receiver<Feedback>>,
    command_rx: Option<Receiver<String>>,
    running: Arc<AtomicBool>,
    command_helper: Helper,
    cursor_position: u16,
}



impl Application {
    pub fn new(gcode: Option<Program>, description: String, command_helper: Helper, running: Arc<AtomicBool>) -> Self {
        let (tx, rx) = channel::<Feedback>();
        let (command_tx, command_rx) = channel::<String>();

        info!("Starting frontend");

        Self {
            // program_running: false,
            gcode,
            to_cnc: vec![],
            event_manager: EventManager::new(),
            machine_configuration: Configuration::new(description),
            tx,
            command_tx,
            rx: Some(rx),
            command_rx: Some(command_rx),
            running,
            command_helper,
            cursor_position: 0,
        }
    }

    fn send_command(
        to_cnc: Option<&mut Vec<MachineData>>,
        command_tx: &Sender<String>,
        cmd: String,
    ) -> AnyhowResult<()> {
        let command = cmd.clone();
        if !cmd.is_empty() {
            if cmd.to_lowercase() == "exit" || cmd.to_lowercase() == "quit" {
                anyhow::bail!("Exiting application")
            } else {
                if let Some(echo) = to_cnc {
                    echo.push(MachineData::ToMachine(format!("{cmd}")));
                }
                if let Err(error) = command_tx.send(cmd) {
                    error!("could not send command {}: {}", command, error);

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
                            cmd,
                        )?;
                    },
                    KeyEvent::ProgramStop => {
                        let prog = self.gcode.as_mut().ok_or(anyhow::anyhow!("missing program"))?;
                        prog.running = false;
                        for cmd in vec!["!".to_string(), Self::ctrl_x()].iter() {
                            Self::send_command(
                                Some(&mut self.to_cnc),
                                &self.command_tx,
                                cmd.clone(),
                            )?;
                        }
                    },
                    KeyEvent::ProgramRun => {
                        if let Some(state) = &self.machine_configuration.status.state {
                            match state {
                                State::Idle => {
                                    let prog = self.gcode.as_mut().ok_or(anyhow::anyhow!("missing program"))?;
                                    prog.running = true;
                                    info!("================= Starting program execution =================");
                                }
                                _ => {},
                            };
                        };
                    },
                    KeyEvent::ProgramPause => {
                        let prog = self.gcode.as_mut().ok_or(anyhow::anyhow!("missing program"))?;
                        prog.running = false;
                        Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            "!".to_string(),
                        )?;
                    },
                    KeyEvent::ProgramModeToggle => {
                        Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            "$GB".to_string(),
                        )?;
                    },
                    KeyEvent::ProgramStepOver => {
                        Self::send_command(
                            Some(&mut self.to_cnc),
                            &self.command_tx,
                            "~".to_string(),
                        )?;
                    },
                    KeyEvent::AppExit => {
                        self.running.store(false, Ordering::Relaxed);
                    },
                    KeyEvent::DeviceReset => {
                        if let Some(prog) = self.gcode.as_mut() {
                            prog.running = false;
                        }
                        Self::send_command(
                            None,
                            &self.command_tx,
                            Self::ctrl_x(),
                        )?;
                    },
                    KeyEvent::DeviceReboot => {
                        if let Some(prog) = self.gcode.as_mut() {
                            prog.running = false;
                        }
                        Self::send_command(
                            None,
                            &self.command_tx,
                            "$BYE".to_string(),
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
        let program_running = self.gcode.as_ref().map(|prog| prog.running).unwrap_or(false);

        if !program_running {
            return Ok(())
        }

        let buffer = self.machine_configuration.status.buffer.as_ref().ok_or(anyhow::anyhow!("missing buffer information"))?;
        let state = self.machine_configuration.status.state.clone().ok_or(anyhow::anyhow!("missing machine state"))?;

        if state == State::Alarm {
            return Err(anyhow::anyhow!("machine is in alarm state"));
        }

        let buffer_lines = buffer.lines;
        let buffer_bytes = buffer.bytes;
        info!("Buffer: {} lines / {} bytes", buffer_lines, buffer_bytes);

        let commands = {
            let prog = self.gcode.as_mut().ok_or(anyhow::anyhow!("missing program"))?;
            if prog.eop_reached_in_machine_buffer 
                && let Some(state) = self.machine_configuration.status.state.as_ref()
                && *state != State::Run {
                prog.running = false;
                return Ok(());
            }

            prog.set_initial_machine_buffer_size(buffer_lines as usize);

            match prog.process(&buffer) {
                Ok(data) => data,
                Err(error) => {
                    error!("program processing: {}", error);
                    return Ok(());
                }
            }
        };

        for command in commands {
            let command_text = command.clone();
            self.to_cnc.push(MachineData::ToMachine(command_text.clone()));
            info!(" ({}) >> {}", command_text.len(), command_text);
            if Self::send_command(
                None,
                &self.command_tx,
                command_text.clone(),
            )
            .is_err()
            {
                if let Some(prog) = self.gcode.as_mut() {
                    prog.running = false;
                }
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn run_iteration(&mut self, frontend: &mut Frontend) -> AnyhowResult<()> {
        let program_running = self.gcode.as_ref().map(|prog| prog.running).unwrap_or(false);

        while let Ok(line) = self.rx.as_ref().unwrap().try_recv() {
            match line {
                Feedback::Status(status) => {
                    info!("Received status update from GRBL");
                    match status {
                        Status::RealTime(status) => {
                            self.machine_configuration = self.machine_configuration.clone() + status;

                            self.process_program()?;
                        }
                        Status::Error(code) => {
                            error!("GRBL reported error:{code}");
                            self.to_cnc.push(MachineData::FromMachine(format!("!ERROR! {code}. Type '$X'")));
                        }
                        Status::Alarm(code) => {
                            error!("GRBL reported ALARM:{code}");
                            self.to_cnc.push(MachineData::FromMachine(format!("!ALARM! {code}. Type '$X'")));
                        }
                        _ => {}
                    };
                },
                Feedback::MachineConfig(machine_config) => {
                    info!("Received machine configuration update from GRBL");
                    self.machine_configuration = self.machine_configuration.clone() + machine_config;
                },
                Feedback::Ok => {
                    //self.to_cnc.push(MachineData::FromMachine("ok".to_string()));
                },
                Feedback::Other(data) => {
                    self.to_cnc.push(MachineData::FromMachine(data));
                },
            };
        }

        frontend.draw(
            &self.machine_configuration,
            &self.to_cnc,
            self.gcode.as_mut(),
            &self.event_manager,
            self.cursor_position,
            &self.command_helper,
            &program_running,
        ).map_err(|e| {
            error!("frontend error: {}", &e);
            self.running.store(false, Ordering::Relaxed);

            anyhow::anyhow!("display error: {}", &e)
        })?;

        self.user_commande_listen()?;
        
        Ok(())
    }

    fn ctrl_x() -> String {
        format!("{}", 0x18 as char)
    }
}


