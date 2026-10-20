use anyhow::Result;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

#[derive(Clone)]
pub struct Program {
    pub code: Vec<String>,
    pub cursor: usize,
    pub end_of_program_reached: bool,
}

impl Program {
    pub fn new(data: Vec<String>) -> Self {
        Self{
            code: data,
            cursor: 0,
            end_of_program_reached: false,
        }
    }

    pub fn process(&mut self, command_offset: usize) -> Result<usize> {
        let mut current_cursor = self.cursor;

        for _ in 0..command_offset {
            current_cursor +=1;
            
            if current_cursor>= self.code.len() {
                break;
            }

            while current_cursor<self.code.len() && !Self::is_code_line(&self.code[current_cursor]) {
                current_cursor += 1;
            }
        }

        if current_cursor < self.code.len()  && Self::is_code_line(&self.code[current_cursor]) {
            self.cursor = current_cursor;

            Ok(current_cursor)
        } else {
            self.end_of_program_reached = true;
            anyhow::bail!("end of program")
        }
    }

    pub fn load(self, lines: usize, buffer_size: usize) -> Vec<String> {
        let mut current_cursor = self.cursor;
        let mut output: Vec<String> = vec![];
        let mut remaining_buffer = buffer_size;

        for _ in 0..lines {
            current_cursor +=1;
            
            if current_cursor>= self.code.len() {
                break;
            }

            while current_cursor<self.code.len() && !Self::is_code_line(&self.code[current_cursor]) {
                current_cursor += 1;
            }

            let current_line = Self::clean_code_line(self.code[current_cursor].clone());
            if remaining_buffer>current_line.len() {
                remaining_buffer -= current_line.len();
                output.push(current_line);
            }
        }

        output
    }

    pub fn clean_code_line(line: String) -> String {
        let comment_start = line
            .find(';')
            .or_else(|| line.find('%'))
            .unwrap_or(line.len());
        line[..comment_start].trim().to_string()
    }

    pub fn is_code_line(line: &str) -> bool {
        !line.trim().starts_with(";")
            && !line.trim().starts_with("%")
            && !line.trim().is_empty()
    }

    pub fn viewport(self, height: usize, ) -> ProgramViewPort {
        let half_size = height / 2 + (height % 2);
        let len = self.code.len();

        if self.cursor >= len {
            let start = len.saturating_sub(height);
            return ProgramViewPort{
                program_lines: self.code[start..].to_vec(),
                cursor: height,
                end_of_program_reached: self.end_of_program_reached,
            };
        }

        let start = if self.cursor > half_size {
            (self.cursor - half_size).min(len.saturating_sub(height))
        } else {
            0
        };
        let end = (start + height).min(len);

        ProgramViewPort {
            program_lines: self.code[start..end].to_vec(),
            cursor: self.cursor - start,
            end_of_program_reached: self.end_of_program_reached,
        }
    }
}


pub struct ProgramViewPort{
    pub program_lines: Vec<String>, 
    pub cursor: usize,
    pub end_of_program_reached: bool,
}

impl<'a> Into<Text<'a>> for ProgramViewPort {
    fn into(self) -> Text<'a> {
        let lines: Vec<Line> = self.program_lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let mut rendered_line = l.clone();
                if i == self.cursor && self.end_of_program_reached{
                    rendered_line = format!("█ {}", rendered_line);
                } else if i == self.cursor {
                    rendered_line = format!("> {}", rendered_line);
                } else {
                    rendered_line = format!("  {}", rendered_line);
                }
                let style = if i == self.cursor {
                    Style::default().fg(Color::LightBlue).add_modifier(ratatui::style::Modifier::BOLD)
                } else if l.starts_with('(') || l.starts_with('%') || l.starts_with(';') {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                Line::from(Span::styled(rendered_line, style))
            })
            .collect();

        Text::from(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program_at(lines: &[&str], cursor: usize) -> Program {
        let mut p = Program::new(lines.iter().map(|s| s.to_string()).collect());
        p.cursor = cursor;
        p
    }

    const CODE: [&str; 10] = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

    #[test]
    fn window_empty_program() {
        let viewport = program_at(&[], 0).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert!(window.is_empty());
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_cursor_at_start() {
        let viewport = program_at(&CODE, 0).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(effective, 0);
    }

    #[test]
    fn window_cursor_near_start() {
        let viewport = program_at(&CODE, 2).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(effective, 2);
    }

    #[test]
    fn window_centers_cursor() {
        let viewport = program_at(&CODE, 5).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["c", "d", "e", "f", "g"]);
        assert_eq!(effective, 3);
    }

    #[test]
    fn window_cursor_near_end() {
        let viewport = program_at(&CODE, 8).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 3);
    }

    #[test]
    fn window_cursor_at_last_line() {
        let viewport = program_at(&CODE, 9).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 4);
    }

    #[test]
    fn window_cursor_past_end() {
        let viewport = program_at(&CODE, 10).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_code_shorter_than_height() {
        let viewport = program_at(&["a", "b", "c"], 1).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c"]);
        assert_eq!(effective, 1);
    }

    #[test]
    fn window_cursor_past_end_short_code() {
        let viewport = program_at(&["a", "b", "c"], 3).viewport(5);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c"]);
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_even_height() {
        let viewport = program_at(&CODE, 5).viewport(4);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["d", "e", "f", "g"]);
        assert_eq!(effective, 2);
    }

    #[test]
    fn window_even_height_cursor_at_last_line() {
        let viewport = program_at(&CODE, 9).viewport(4);
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["g", "h", "i", "j"]);
        assert_eq!(effective, 3);
    }

    #[test]
    fn clean_line_without_comment() {
        assert_eq!(Program::clean_code_line("G0 X10 Y20".to_string()), "G0 X10 Y20");
    }

    #[test]
    fn clean_line_strips_semicolon_comment() {
        assert_eq!(
            Program::clean_code_line("G0 X10 ; move to origin".to_string()),
            "G0 X10"
        );
    }

    #[test]
    fn clean_line_strips_percent_comment() {
        assert_eq!(
            Program::clean_code_line("G0 X10 % program start".to_string()),
            "G0 X10"
        );
    }

    #[test]
    fn clean_line_comment_only() {
        assert_eq!(Program::clean_code_line("; full comment".to_string()), "");
        assert_eq!(Program::clean_code_line("% full comment".to_string()), "");
    }

    #[test]
    fn clean_line_trims_whitespace() {
        assert_eq!(Program::clean_code_line("  G0 X10  ".to_string()), "G0 X10");
    }
}


