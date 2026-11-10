use anyhow::Result;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use super::status::BufferStatus;

#[derive(Clone)]
pub struct Program {
    code: Vec<String>,
    pub cursor: usize, // last line in machine buffer
    pub eop_reached_in_machine_buffer: bool, // End Of Program
    machine_buffer_size: Option<usize>,
    machine_buffer: Vec<usize>,
    pub running: bool,
}

impl Program {
    pub fn new(data: Vec<String>) -> Self {
        Self{
            code: data,
            cursor: 0,
            eop_reached_in_machine_buffer: false,
            machine_buffer_size: None,
            machine_buffer: vec![],
            running: false,
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
        if current >= self.code.len() {
            return Err(anyhow::anyhow!("end of program reached"));
        }

        let mut current_cursor = current as i64;
        let mut remaining_bytes = max_bytes;

        for _ in 0..count {
            current_cursor += 1;
            if current_cursor < 0 || current_cursor >= self.code.len() as i64 {
                if output.is_empty() {
                    return Err(anyhow::anyhow!("end of program reached"));
                }

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

    pub fn viewport(&mut self, height: usize, buffer: &BufferStatus) -> ProgramViewPort {
        self.cut_buffer(&buffer);

        let half_size = height / 2 + (height % 2);
        let len = self.code.len();
        
        let program_cursor = *(self.machine_buffer.first().unwrap_or(&self.cursor));

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
            program_lines: self.code
                .iter()
                .enumerate()
                .map(|(num, line)| (num, line.clone()))
                .collect::<Vec<(usize, String)>>()[start..end].to_vec(),
            buffer,
        }
    }
}


pub struct ProgramViewPort{
    pub program_lines: Vec<(usize, String)>, 
    pub buffer: Option<(usize,usize)>,
}

impl<'a> Into<Text<'a>> for ProgramViewPort {
    fn into(self) -> Text<'a> {
        let size = self.program_lines.len();
        let lines: Vec<Line> = self.program_lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let mut rendered_line = line.1.clone();
                let index = line.0;

                let mut style: Style = Style::default();

                if let Some(buffer) = self.buffer {
                    if index >= buffer.0 && index <= buffer.1 {
                        style = style.fg(Color::LightBlue).add_modifier(ratatui::style::Modifier::BOLD)
                    }
                }

                if i == size - 1 && (self.buffer.is_none() || self.buffer.unwrap().1 < index) {
                    style = style.fg(Color::Red);
                    rendered_line.insert_str(0, "#");
                } else if index == self.buffer.map(|b| b.1).unwrap_or(0) {
                    rendered_line.insert_str(0, ">");
                } else {
                    rendered_line.insert_str(0, " ");
                }

                if rendered_line.starts_with('(') || rendered_line.starts_with('%') || rendered_line.starts_with(';') {
                    style = style.fg(Color::Green);
                };

                Line::from(Span::styled(rendered_line, style))
            })
            .collect();

        Text::from(lines)
    }
}

