use crossterm::event::KeyModifiers;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
};
use std::time::Duration;
use ratatui::{
    style::{Color, Style, Modifier},
    text::{Line, Span},
};
use std::sync::LazyLock;
use regex::Regex;

use crate::{grbl::program::commented_line, history::{Direction, History}};


pub enum KeyEvent {
    Command(String),
    ProgramStop,
    ProgramRun,
    ProgramPause,
    ProgramModeToggle,
    ProgramStepOver,
    AppExit,
    DeviceReset,
    DeviceReboot,
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
                (KeyCode::Char('r'), KeyModifiers::CONTROL) => Ok(Some(KeyEvent::DeviceReboot)),
                (KeyCode::Backspace, KeyModifiers::NONE) => {
                    if self.cursor_position>0 {
                        self.current_command.remove(self.cursor_position as usize -1);
                        self.cursor_position -=1;
                    }
                    Ok(Some(KeyEvent::CursorAt(self.cursor_position)))
                },
                (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                    let mut char = c.to_ascii_uppercase();
                    if char == 'O' 
                        && self.cursor_position > 0
                        && self.current_command.chars().nth(self.cursor_position as usize -1).unwrap() == 'G' {
                        char = '0';
                    }
                    self.current_command.insert(self.cursor_position as usize, char);
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


static WORD: LazyLock<Regex> = LazyLock::new(|| {
    // Regex::new(r"(?P<letter>[A-Za-z])(?P<space>\s*)(?P<value>[-+]?(?:\d+\.?\d*|\.\d+))").unwrap()
    Regex::new(r"(?P<letter>[A-Za-z])(?:(?P<space>\s*)(?P<value>[-+]?(?:\d+\.?\d*|\.\d+)))?").unwrap()
});

fn letter_style(letter: char) -> Style {
    let base = Style::new().add_modifier(Modifier::BOLD);
    match letter.to_ascii_uppercase() {
        'G' => base.fg(Color::Cyan),
        'M' => base.fg(Color::Magenta),
        'X' | 'Y' | 'Z' | 'A' | 'B' | 'C' => base.fg(Color::Yellow),
        'I' | 'J' | 'K' | 'R' => base.fg(Color::LightYellow),
        'F' | 'S' => base.fg(Color::Green),
        'T' | 'N' | 'P' => base.fg(Color::Blue),
        _ => base.fg(Color::White),
    }
}

fn value_style(letter: char) -> Style {
    match letter.to_ascii_uppercase() {
        'G' => Style::new().fg(Color::Cyan),
        'M' => Style::new().fg(Color::Magenta),
        'F' | 'S' => Style::new().fg(Color::LightGreen),
        _ => Style::new().fg(Color::White),
    }
}

impl<'a> Into<Line<'a>> for EventManager {
    fn into(self) -> Line<'a> {
        let input = self.current_command.trim_start().to_string();
        if input.len() == 0 {
            return Line::default()
        }

        let code_comment = commented_line(input);

        if code_comment.0.starts_with('$') {
            return Line::from(vec![
                Span::styled(code_comment.0, Style::new().fg(Color::LightBlue)),
                Span::styled(code_comment.1, Style::new().fg(Color::Green)),
            ]);
        }

        let mut spans: Vec<Span> = vec![];
        let mut last = 0;

        let code = code_comment.0.as_str();

        for caps in WORD.captures_iter(code) {
            let whole = caps.get(0).unwrap();

            if whole.start() > last {
                spans.push(Span::raw(code[last..whole.start()].to_string()));
            }

            let letter = &caps["letter"];
            let ch = letter.chars().next().unwrap();

            spans.push(Span::styled(letter.to_string(), letter_style(ch)));

            if let Some(sp) = caps.name("space").filter(|s| !s.as_str().is_empty()) {
                spans.push(Span::raw(sp.as_str().to_string()));
            }
            if let Some(v) = caps.name("value") {
                spans.push(Span::styled(v.as_str().to_string(), value_style(ch)));
            }

            last = whole.end();
        }

        spans.push(Span::styled(code_comment.1, Style::new().fg(Color::Green)));

        Line::from(spans)
    }
}

impl<'a> From<&'a EventManager> for &'a str {
    fn from(em: &'a EventManager) -> &'a str {
        em.current_command.as_str()
    }
}