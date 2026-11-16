use anyhow::Result;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span, Text},
};

use super::status::BufferStatus;

use crate::logging::Logger;
use crate::logging::Level as LogLevel;
use regex::Regex;

#[derive(Clone)]
pub struct Program {
    code: Vec<String>,
    logger: Option<Logger>,
    pub cursor: usize, // last line in machine buffer
    pub eop_reached_in_machine_buffer: bool, // End Of Program
    machine_buffer_size: Option<usize>,
    machine_buffer: Vec<usize>,
    pub running: bool,
}

impl Program {
    pub fn new(data: Vec<String>, logger: Option<Logger>) -> Self {
        Self{
            code: data,
            logger,
            cursor: 0,
            eop_reached_in_machine_buffer: false,
            machine_buffer_size: None,
            machine_buffer: vec![],
            running: false,
        }
    }

    fn log(&self, level: LogLevel, msg: impl std::fmt::Display) {
        if let Some(logger) = &self.logger {
            let message = format!("[PROGRAM] {}", msg);
            if let Err(log_error) = logger.log(level, &message) {
                eprintln!("Failed to write frontend error to log file: {log_error}");
            }
        }
    }

    pub fn set_initial_machine_buffer_size(&mut self, size: usize) {
        match self.machine_buffer_size {
            None => self.machine_buffer_size = Some(size),
            Some(_) => {}
        };
    }

    pub fn open(filename: String, logger: Option<Logger>) -> Result<Self> {
        let file_content = std::fs::read_to_string(filename)?;

        let gcode: Vec<String> = file_content.lines().map(str::to_owned).collect();

        Ok(Self::new(gcode, logger))
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

        let new_buffer_size = if max_buffer_size > buffer.lines {
            max_buffer_size - buffer.lines
        } else {
            0
        };

        let start_index = self.machine_buffer.len().saturating_sub(new_buffer_size);

        self.log(LogLevel::Info, format!("[BUFFER] cutting: {:?}, device buffer lines: {}, new buffer size: {}, cutting index: {}", self.machine_buffer, buffer.lines, new_buffer_size, start_index));

        self.machine_buffer = self.machine_buffer[start_index..].to_vec();
    }

    fn lines(&self, current: usize, count: usize, max_bytes: usize) -> Result<Vec<usize>> {
        self.log(LogLevel::Info, format!("collecting lines with current: {}, count: {}, max_bytes: {}", current, count, max_bytes));
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
                let mut line = self.code[current_cursor as usize].clone();

                if let Some(pos) = line.find(|c| c == ';' || c == '(') {
                    line.truncate(pos);
                }

                let line_length = line.len();
                if line_length > remaining_bytes {
                    break;
                }
                
                remaining_bytes -= line_length+1; // +1 for newline character
                output.push(current_cursor as usize);
            }
        }

        self.log(LogLevel::Info, format!("collected lines: {:?}", output));

        Ok(output)
    }

    pub fn process(&mut self, buffer: &BufferStatus) -> Result<Vec<String>> {
        self.log(LogLevel::Info, format!("[BUFFER] init: {:?}", self.machine_buffer));
        self.cut_buffer(&buffer);
        self.log(LogLevel::Info, format!("[BUFFER] cut: {:?} - {}", self.machine_buffer, buffer));
        
        let cursor = self.cursor;
        let line_map =  self
            .lines(cursor, buffer.lines,buffer.bytes)
            .map_err(|err| {
                self.eop_reached_in_machine_buffer = true;
                err
            })?;
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

        self.machine_buffer.extend(line_map);
        self.log(LogLevel::Info, format!("[BUFFER] extended: {:?}", self.machine_buffer));

        Ok(result)
    }

    pub fn is_code_line(line: &str) -> bool {
        !line.trim().starts_with(";")
            && !line.trim().starts_with("%")
            && !line.trim().is_empty()
    }

    pub fn viewport(&mut self, height: usize, buffer: &BufferStatus, current_line: Option<u32>) -> ProgramViewPort {
        self.cut_buffer(&buffer);

        let len = self.code.len();

        let start_offset =  height.saturating_sub(self.machine_buffer.len()) / 2;

        let program_cursor = *(self.machine_buffer.first().unwrap_or(&self.cursor));

        let start = if program_cursor > start_offset {
            program_cursor - start_offset
        } else {
            0
        };

        let end = (start + height).min(len);
        
        let buffer = if self.machine_buffer.len()>0 {
            self.log(LogLevel::Info, format!("$$$$$$ current command line : {}", self.machine_buffer.first().unwrap_or(&start)));
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

                let spans: Vec<Span> = if rendered_line.starts_with('(') || rendered_line.starts_with('%') || rendered_line.starts_with(';') {
                    vec![Span::styled(rendered_line, Style::default().fg(Color::Green))]
                } else if let Some(pos) = rendered_line.find(|c| c == ';' || c == '(') {
                    let (before, after) = rendered_line.split_at(pos);

                    vec![
                        Span::styled(before.to_owned(), code_style),
                        Span::styled(after.to_owned(), Style::default().fg(Color::Green))
                    ]
                } else {
                    vec![Span::styled(rendered_line, code_style)]
                };

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

