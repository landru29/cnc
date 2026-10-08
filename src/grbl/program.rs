use anyhow::Result;

#[derive(Clone)]
pub struct Program {
    pub code: Vec<String>,
    pub cursor: usize,
}

impl Program {
    pub fn new(data: Vec<String>) -> Self {
        Self{
            code: data,
            cursor: 0,
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

    pub fn viewport(self, height: usize) -> (Vec<String>, usize) {
        let half_size = height / 2 + (height % 2);
        let len = self.code.len();

        if self.cursor >= len {
            let start = len.saturating_sub(height);
            return (self.code[start..].to_vec(), height);
        }

        let start = if self.cursor > half_size {
            (self.cursor - half_size).min(len.saturating_sub(height))
        } else {
            0
        };
        let end = (start + height).min(len);

        (self.code[start..end].to_vec(), self.cursor - start)
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
        let (window, effective) = program_at(&[], 0).viewport(5);
        assert!(window.is_empty());
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_cursor_at_start() {
        let (window, effective) = program_at(&CODE, 0).viewport(5);
        assert_eq!(window, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(effective, 0);
    }

    #[test]
    fn window_cursor_near_start() {
        let (window, effective) = program_at(&CODE, 2).viewport(5);
        assert_eq!(window, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(effective, 2);
    }

    #[test]
    fn window_centers_cursor() {
        let (window, effective) = program_at(&CODE, 5).viewport(5);
        assert_eq!(window, vec!["c", "d", "e", "f", "g"]);
        assert_eq!(effective, 3);
    }

    #[test]
    fn window_cursor_near_end() {
        let (window, effective) = program_at(&CODE, 8).viewport(5);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 3);
    }

    #[test]
    fn window_cursor_at_last_line() {
        let (window, effective) = program_at(&CODE, 9).viewport(5);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 4);
    }

    #[test]
    fn window_cursor_past_end() {
        let (window, effective) = program_at(&CODE, 10).viewport(5);
        assert_eq!(window, vec!["f", "g", "h", "i", "j"]);
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_code_shorter_than_height() {
        let (window, effective) = program_at(&["a", "b", "c"], 1).viewport(5);
        assert_eq!(window, vec!["a", "b", "c"]);
        assert_eq!(effective, 1);
    }

    #[test]
    fn window_cursor_past_end_short_code() {
        let (window, effective) = program_at(&["a", "b", "c"], 3).viewport(5);
        assert_eq!(window, vec!["a", "b", "c"]);
        assert_eq!(effective, 5);
    }

    #[test]
    fn window_even_height() {
        let (window, effective) = program_at(&CODE, 5).viewport(4);
        assert_eq!(window, vec!["d", "e", "f", "g"]);
        assert_eq!(effective, 2);
    }

    #[test]
    fn window_even_height_cursor_at_last_line() {
        let (window, effective) = program_at(&CODE, 9).viewport(4);
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


