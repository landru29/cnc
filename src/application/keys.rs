use crossterm::event::KeyModifiers;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
};
use std::time::Duration;
use ratatui::{
    text::Text,
};

use crate::history::{Direction, History};


pub enum KeyEvent {
    Command(String),
    ProgramStop,
    ProgramRun,
    ProgramPause,
    ProgramModeToggle,
    ProgramStepOver,
    AppExit,
    DeviceReset,
    CursorAt(u16)
}


#[derive(Debug, Clone)]
pub struct EventManager {
    current_command: String,
    history: History,
    cursor_position: u16
}

impl EventManager {
    pub fn new() -> Self {
        Self{
            current_command:"".to_string(),
            history: History::new(),
            cursor_position: 0,
        }
    }

    pub fn iterate(&mut self) -> anyhow::Result<Option<KeyEvent>> {
        if !event::poll(Duration::from_millis(50)).map_err(|err| anyhow::anyhow!(err))? {
            return Ok(None);
        }

        let event = event::read().map_err(|err| anyhow::anyhow!(err))?;

        if let Event::Key(k) = event {
            if k.kind != KeyEventKind::Press {
                return Ok(None);
            }
            return match (k.code, k.modifiers) {
                (KeyCode::Enter, KeyModifiers::NONE) => {
                    let command = self.current_command.clone();
                    self.history.push(self.current_command.clone());
                    self.current_command = "".to_string();
                    self.cursor_position = 0;
                    Ok(Some(KeyEvent::Command(command)))
                },
                (KeyCode::F(4), KeyModifiers::NONE) => Ok(Some(KeyEvent::ProgramStop)),
                (KeyCode::F(5), KeyModifiers::NONE) => Ok(Some(KeyEvent::ProgramRun)),
                (KeyCode::F(6), KeyModifiers::NONE) => Ok(Some(KeyEvent::ProgramPause)),
                (KeyCode::F(7), KeyModifiers::NONE) => Ok(Some(KeyEvent::ProgramModeToggle)),
                (KeyCode::F(8), KeyModifiers::NONE) => Ok(Some(KeyEvent::ProgramStepOver)),
                (KeyCode::Up, KeyModifiers::NONE) => {
                    if let Some(line) = self.history.get(Direction::Previous) {
                        self.current_command = line.clone();
                        self.cursor_position = line.len() as u16;
                    }
                    
                    Ok(Some(KeyEvent::CursorAt(self.cursor_position)))
                },
                (KeyCode::Down, KeyModifiers::NONE) => {
                    if let Some(line) = self.history.get(Direction::Next) {
                        self.current_command = line.clone();
                        self.cursor_position = line.len() as u16;
                    }
                        
                    Ok(Some(KeyEvent::CursorAt(self.cursor_position)))
                },
                (KeyCode::Left, KeyModifiers::NONE) => {
                    if self.cursor_position>0 {
                        self.cursor_position -=1;
                    }

                    Ok(Some(KeyEvent::CursorAt(self.cursor_position)))
                },
                (KeyCode::Right, KeyModifiers::NONE) => {
                    if self.cursor_position < self.current_command.len() as u16 {
                        self.cursor_position +=1;
                    }

                    Ok(Some(KeyEvent::CursorAt(self.cursor_position)))
                }
                (KeyCode::Char('d'), KeyModifiers::CONTROL) => Ok(Some(KeyEvent::AppExit)),
                (KeyCode::Char('x'), KeyModifiers::CONTROL) => Ok(Some(KeyEvent::DeviceReset)),
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    if self.cursor_position>0 {
                        self.current_command.remove(self.cursor_position as usize -1);
                        self.cursor_position -=1;
                    }
                    Ok(Some(KeyEvent::CursorAt(self.cursor_position)))
                },
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    // self.current_command.push(c.to_ascii_uppercase());
                    self.current_command.insert(self.cursor_position as usize, c.to_ascii_uppercase());
                    self.cursor_position +=1;
                    Ok(Some(KeyEvent::CursorAt(self.cursor_position)))
                },
                _ => Ok(None),
            }
        }

        Ok(None)
    }

    pub fn command_size(&self) -> usize {
        self.current_command.len()
    }
}


impl<'a> Into<Text<'a>> for EventManager {
    fn into(self) -> Text<'a> {
        Text::from(format!("{}", self.current_command))
    }
}

impl<'a> From<&'a EventManager> for &'a str {
    fn from(em: &'a EventManager) -> &'a str {
        em.current_command.as_str()
    }
}