use ratatui::{
    backend::CrosstermBackend,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout, Position},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
    Terminal,
};
use std::{io::Stdout, sync::mpsc::{Receiver, Sender, channel}, time::Duration};
use crossterm::event::KeyModifiers;
use crate::grbl::{machine::MachineConfig, program::Program, status::Status, configuration::Configuration};
use anyhow::Result as AnyhowResult;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use anyhow::Result;

pub struct App {
    state: Option<AppState>,
    running: Arc<AtomicBool>,
    pub handle: Option<std::thread::JoinHandle<()>>,
}

struct AppState {
    gcode: Program,
    log: Vec<String>,
    to_cnc: Vec<String>,
    input: String,
    machine_configuration: Configuration,
    pub rx: Receiver<String>,
    pub tx: Sender<String>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl AppState {
    fn new(
        gcode: Program,
        description: String,
    ) -> Self {
        let (tx, rx) = channel::<String>();
        Self {
            gcode,
            log: vec![],
            to_cnc: vec![],
            input: String::new(),
            machine_configuration: Configuration::new(description),
            rx: rx,
            tx: tx,
            terminal: ratatui::init(),
        }
    }

    fn send_command(&mut self, cmd: String) -> AnyhowResult<()> {
        if !cmd.is_empty() {
            if cmd.to_lowercase() == "exit" || cmd.to_lowercase() == "quit"  {
                anyhow::bail!("Exiting application")
            } else if cmd.to_lowercase() == "p" {
                if !self.gcode.end_of_program_reached && let Err(_) = self.gcode.process(1) {}
            } else {
                if cmd != "?".to_string() && cmd != "$G".to_string() {
                    self.to_cnc.push(format!("> {cmd}"));
                }
                self.tx.send(cmd).expect("Failed to send command to serial port");
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
        let to_cnc: Vec<Line> = to_cnc[to_cnc_start..].iter().map(|l| Line::from(l.as_str())).collect();
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
        let log: Vec<Line> = log[log_start..].iter().map(|l| Line::from(l.as_str())).collect();
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

}

impl App {
    pub fn new(
        gcode: Program,
        description: String,
    ) -> Self {
        Self {
            state: Some(AppState::new(gcode, description)),
            running: Arc::new(AtomicBool::new(true)),
            handle: None,
        }
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn start(&mut self) -> Result<()> {
        let mut app = self
            .state
            .take()
            .ok_or_else(|| anyhow::anyhow!("Application has already been started"))?;
        let running_thread = Arc::clone(&self.running);
        self.handle = Some(std::thread::spawn(move || {
            while running_thread.load(Ordering::Relaxed) {
                while let Ok(line) = app.rx.try_recv() {
                    if let Ok(Status::RealTime(status)) = line.parse::<Status>() {
                        app.machine_configuration = app.machine_configuration.clone() + status;
                    } else if let Ok(machine_config) = line.parse::<MachineConfig>() {
                        app.machine_configuration = app.machine_configuration.clone() + machine_config;
                    } else {
                        app.log.push(line);
                    }
                }

                let gcode = app.gcode.clone();
                let log = app.log.clone();
                let to_cnc = app.to_cnc.clone();
                let input = app.input.clone();
                let machine_configuration = app.machine_configuration.clone();

                if let Err(_) = app.terminal.draw(|f| {
                    if let Err(_) =
                        AppState::draw(f, &gcode, &log, &to_cnc, &input, &machine_configuration)
                    {}
                }) {
                    running_thread.store(false, Ordering::Relaxed);
                    continue;
                };

                match event::poll(Duration::from_millis(50)) {
                    Ok(value)=> {
                        if !value {
                            continue;
                        }
                    },
                    Err(_)=>{
                        running_thread.store(false, Ordering::Relaxed);
                        continue;
                    },
                };

                if let Ok(Event::Key(k)) = event::read() {
                    if k.kind != KeyEventKind::Press {
                        continue;
                    }
                    match (k.code, k.modifiers) {
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            let cmd = std::mem::take(&mut app.input);
                            if let Err(_) = app.send_command(cmd) {
                                break;
                            }
                        }
                        (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                            running_thread.store(false, Ordering::Relaxed);
                        }
                        (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                            app.input.clear();
                            let ctrl_x: u8 = 0x18;
                            let cmd = format!("{}", ctrl_x as char);
                            if let Err(_) = app.send_command(cmd) {
                                break;
                            }
                        }
                        (KeyCode::Backspace, KeyModifiers::NONE) => {
                            app.input.pop();
                        }
                        (KeyCode::Char(c), KeyModifiers::NONE) => {
                            app.input.push(c.to_ascii_uppercase())
                        }
                        (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                            app.input.push(c.to_ascii_uppercase())
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
