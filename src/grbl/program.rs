use anyhow::Result;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use super::status::BufferStatus;

use log::info;
use regex::Regex;

#[derive(Clone)]
pub struct Program {
    code: Vec<String>,
    pub cursor: usize, // last line in machine buffer
    pub eop_reached_in_machine_buffer: bool, // End Of Program
    machine_buffer_size: Option<usize>,
    data_sent: Vec<usize>,
    pub running: bool,
}

pub fn commented_line(line: String) -> (String, String) {
    if line.starts_with('(') || line.starts_with('%') || line.starts_with(';') {
        (String::default(), line.clone())
    } else if let Some(pos) = line.find(|c| c == ';' || c == '(') {
        let (before, after) = line.split_at(pos);

        (before.to_owned(), after.to_owned())
    } else {
        (line.clone(), String::default())
    }
}

impl Program {
    pub fn new(data: Vec<String>) -> Self {
        Self{
            code: data,
            cursor: 0,
            eop_reached_in_machine_buffer: false,
            machine_buffer_size: None,
            data_sent: vec![],
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

    fn lines(&self, current: usize, count: usize, max_bytes: usize) -> Result<Vec<usize>> {
        info!("collecting lines with current: {}, count: {}, max_bytes: {}", current, count, max_bytes);
        let mut output: Vec<usize> = vec![];
        if current >= self.code.len() {
            return Err(anyhow::anyhow!("end of program reached"));
        }

        if count == 0 || max_bytes == 0 {
            return Ok(output);
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
                let code_comment = commented_line(self.code[current_cursor as usize].clone());

                let line_length = code_comment.0.len();
                if line_length > remaining_bytes {
                    break;
                }
                
                remaining_bytes -= line_length+1; // +1 for newline character
                output.push(current_cursor as usize);
            }
        }

        info!("collected lines: {:?}", output);

        Ok(output)
    }

    pub fn process(&mut self, buffer: &BufferStatus) -> Result<Vec<String>> {
        let cursor = self.cursor;
        let line_map =  self
            .lines(cursor, buffer.lines,buffer.bytes)
            .map_err(|err| {
                self.eop_reached_in_machine_buffer = true;
                err
            })?;

        if line_map.is_empty() {
            return Ok(vec![]);
        }

        let result = line_map
            .iter()
            .map(|index| {
                let mut line = self.code[*index].clone();

                if let Some(pos) = line.find(|c| c == ';' || c == '(') {
                    line.truncate(pos);
                }
                
                line
            })
            .collect::<Vec<String>>();

        if let Some(last) = line_map.last() {
            self.cursor = *last;
        }

        self.data_sent.extend(line_map);

        Ok(result)
    }

    pub fn is_code_line(line: &str) -> bool {
        !line.trim().starts_with(";")
            && !line.trim().starts_with("%")
            && !line.trim().is_empty()
    }

    pub fn viewport(&mut self, height: usize, buffer: &BufferStatus, current_line: Option<u32>) -> ProgramViewPort {
        let len = self.code.len();

        let machine_buffer_length = if let Some(max_size) = self.machine_buffer_size {max_size - buffer.lines} else {0};

        let start_offset =  height.saturating_sub(machine_buffer_length) / 2;

        let start_index = self.data_sent.len().saturating_sub(machine_buffer_length);
        let machine_buffer = self.data_sent[start_index..].to_vec();

        let program_cursor = *(machine_buffer.first().unwrap_or(&self.cursor));

        let start = if program_cursor > start_offset {
            program_cursor - start_offset
        } else {
            0
        };

        let end = (start + height).min(len);
        
        let buffer = if machine_buffer_length>0 {
            Some((
                *(machine_buffer.first().unwrap_or(&start)),
                *(machine_buffer.last().unwrap_or(&start))
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
            stop: self.eop_reached_in_machine_buffer,
            current_line: current_line.map(|v| v as usize),
        }
    }
}


pub struct ProgramViewPort{
    pub program_lines: Vec<(usize, String)>, 
    pub buffer: Option<(usize,usize)>,
    pub stop: bool,
    current_line: Option<usize>
}

impl<'a> Into<Text<'a>> for ProgramViewPort {
    fn into(self) -> Text<'a> {
        let line_number_regex = Regex::new(r"(?i)N(\d+)").unwrap();

        let size = self.program_lines.len();
        let lines: Vec<Line> = self.program_lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let index = line.0;

                let prefix = if self.stop && i == size - 1 && self.buffer.is_none() {
                        "█"
                    } else if let Some(buffer) = self.buffer {
                    if index == buffer.0 && index == buffer.1 {
                        "╠"
                    } else if index > buffer.0 && index < buffer.1 {
                        "║"
                    } else if index == buffer.0 {
                        "╔"
                    } else if index == buffer.1 {
                        "╚"
                    } else {
                        " "
                    }
                } else {
                    " "
                };

                let rendered_line = line.1.clone();

                let code_style = if let Some(line) = self.current_line 
                   && let Some(caps) = line_number_regex.captures(rendered_line.as_str())
                   && caps[1].parse::<usize>().unwrap() == line {
                    Style::default().fg(Color::Blue)
                } else {
                    Style::default()
                };

                let code_comment = commented_line(rendered_line);
                let spans = vec![
                    Span::styled(code_comment.0, code_style),
                    Span::styled(code_comment.1, Style::default().fg(Color::Green))
                ];

                let mut output = vec![
                    Span::styled(prefix, Style::default().fg(Color::Blue)),
                ];

                output.extend(spans);

                Line::from(output)
            })
            .collect();

        Text::from(lines)
    }
}

