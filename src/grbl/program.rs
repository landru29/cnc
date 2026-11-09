use anyhow::Result;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use super::status::BufferStatus;

#[derive(Clone)]
pub struct Program {
    code: Vec<String>,
    cursor: usize, // last line in machine buffer
    pub eop_reached_in_machine_buffer: bool, // End Of Program
    machine_buffer_size: Option<usize>,
    machine_buffer: Vec<usize>,
}

impl Program {
    pub fn new(data: Vec<String>) -> Self {
        Self{
            code: data,
            cursor: 0,
            eop_reached_in_machine_buffer: false,
            machine_buffer_size: None,
            machine_buffer: vec![],
        }
    }

    pub fn set_initial_machine_buffer_size(&mut self, size: usize) {
        match self.machine_buffer_size {
            None => self.machine_buffer_size = Some(size),
            Some(_) => {}
        };
    }

    pub fn open(filename: String) -> Result<Self> {
        let file_content = std::fs::read_to_string(filename)?;

        let gcode: Vec<String> = file_content.lines().map(str::to_owned).collect();

        Ok(Self::new(gcode))
    }

    fn cut_buffer(&mut self, buffer: &BufferStatus) {
        if self.machine_buffer_size.is_none() {
            self.machine_buffer_size = Some(buffer.lines);

            return
        }

        let max_buffer_size = self.machine_buffer_size.unwrap();
        if buffer.lines> max_buffer_size {
            self.machine_buffer_size = Some(buffer.lines);

            return;
        }

        let start_index = self.machine_buffer.len().saturating_sub(max_buffer_size - buffer.lines);
        self.machine_buffer = self.machine_buffer[start_index..].to_vec();
    }

    fn lines(&self, current: usize, count: usize, max_bytes: usize) -> Result<Vec<usize>> {
        let mut output: Vec<usize> = vec![];
        if current > self.code.len() {
            return Err(anyhow::anyhow!("end of program reached"));
        }

        let mut current_cursor = current as i64;
        let mut remaining_bytes = max_bytes;

        for _ in 0..count {
            current_cursor += 1;
            if current_cursor < 0 || current_cursor >= self.code.len() as i64 {
                break;
            }

            while current_cursor < self.code.len() as i64
                && !Self::is_code_line(&self.code[current_cursor as usize])
            {
                current_cursor += 1;
            }


            if current_cursor < self.code.len() as i64
                && Self::is_code_line(&self.code[current_cursor as usize])
            {
                let line_length = self.code[current_cursor as usize].len();
                if line_length > remaining_bytes {
                    break;
                }
                
                remaining_bytes -= line_length+1; // +1 for newline character
                output.push(current_cursor as usize);
            }
        }

        Ok(output)
    }

    pub fn process(&mut self, buffer: &BufferStatus) -> Result<Vec<String>> {
        self.cut_buffer(&buffer);
        
        let cursor = self.cursor;
        let line_map =  self
            .lines(cursor, buffer.lines,buffer.bytes)
            .map_err(|err| {
                self.eop_reached_in_machine_buffer = true;
                err
            })?;
        let result = line_map
            .iter()
            .map(|index| self.code[*index].clone())
            .collect::<Vec<String>>();

        if let Some(last) = line_map.last() {
            self.cursor = *last;
        }

        self.machine_buffer.extend(line_map);

        Ok(result)
    }

    pub fn is_code_line(line: &str) -> bool {
        !line.trim().starts_with(";")
            && !line.trim().starts_with("%")
            && !line.trim().is_empty()
    }

    pub fn viewport(&self, height: usize, buffer: &BufferStatus) -> ProgramViewPort {
        let half_size = height / 2 + (height % 2);
        let len = self.code.len();
        

        let mut program_cursor = *(self.machine_buffer.first().unwrap_or(&self.cursor));

        if let Some(buffer_size) = self.machine_buffer_size {
            let offset = buffer_size - buffer.lines;
            if offset>0 {
                program_cursor += offset;
            }
        }

        if program_cursor >= len {
            let start = len.saturating_sub(height);
            return ProgramViewPort{
                program_lines: self.code[start..].to_vec(),
                cursor: height,
                end_of_program_reached: self.eop_reached_in_machine_buffer,
                buffer: None,
            };
        }

        let start = if program_cursor > half_size {
            (program_cursor - half_size).min(len.saturating_sub(height))
        } else {
            0
        };
        let end = (start + height).min(len);
        
        let buffer = if self.machine_buffer.len()>0 {
            Some((
                *(self.machine_buffer.first().unwrap_or(&start)),
                *(self.machine_buffer.last().unwrap_or(&start))
            ))
        } else {
            None
        };

        ProgramViewPort {
            program_lines: self.code[start..end].to_vec(),
            cursor: program_cursor - start,
            end_of_program_reached: self.eop_reached_in_machine_buffer,
            buffer,
        }
    }
}


pub struct ProgramViewPort{
    pub program_lines: Vec<String>, 
    pub cursor: usize,
    pub buffer: Option<(usize,usize)>,
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
                    rendered_line = format!("█  {}", rendered_line);
                } else if i == self.cursor {
                    rendered_line = format!(">  {}", rendered_line);
                } else {
                    rendered_line = format!("   {}", rendered_line);
                }

                let mut style: Style = Style::default();

                if let Some(buffer) = self.buffer {
                    if i >= buffer.0 && i <= buffer.1 {
                        style = style.fg(Color::LightBlue).add_modifier(ratatui::style::Modifier::BOLD)
                    }
                }

                if l.starts_with('(') || l.starts_with('%') || l.starts_with(';') {
                    style = style.fg(Color::Green);
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

    fn default_buffer() -> BufferStatus {
        BufferStatus::default()
    }

    const CODE: [&str; 10] = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

    #[test]
    fn window_empty_program() {
        let viewport = program_at(&[], 0).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert!(window.is_empty());
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_cursor_at_start() {
        let viewport = program_at(&CODE, 0).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(effective, 0);
    }

    #[test]
    fn window_cursor_near_start() {
        let viewport = program_at(&CODE, 2).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(effective, 2);
    }

    #[test]
    fn window_centers_cursor() {
        let viewport = program_at(&CODE, 5).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["c", "d", "e", "f", "g"]);
        assert_eq!(effective, 3);
    }

    #[test]
    fn window_cursor_near_end() {
        let viewport = program_at(&CODE, 8).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 3);
    }

    #[test]
    fn window_cursor_at_last_line() {
        let viewport = program_at(&CODE, 9).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 4);
    }

    #[test]
    fn window_cursor_past_end() {
        let viewport = program_at(&CODE, 10).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_code_shorter_than_height() {
        let viewport = program_at(&["a", "b", "c"], 1).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c"]);
        assert_eq!(effective, 1);
    }

    #[test]
    fn window_cursor_past_end_short_code() {
        let viewport = program_at(&["a", "b", "c"], 3).viewport(5, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["a", "b", "c"]);
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_even_height() {
        let viewport = program_at(&CODE, 5).viewport(4, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["d", "e", "f", "g"]);
        assert_eq!(effective, 2);
    }

    #[test]
    fn window_even_height_cursor_at_last_line() {
        let viewport = program_at(&CODE, 9).viewport(4, &default_buffer());
        let (window, effective) = (viewport.program_lines, viewport.cursor);
        assert_eq!(window, vec!["g", "h", "i", "j"]);
        assert_eq!(effective, 3);
    }
}
