#[derive(Debug, Clone)]
pub struct History {
    data: Vec<String>,
    cursor: i16,
}

pub enum Direction {
    Previous,
    Next,
}

const MAX_SIZE: usize = 50;

impl History {
    pub fn new() -> Self {
        Self{
            data: vec![],
            cursor: -1,
        }
    }

    pub fn push(&mut self, line: String) {
        self.cursor = -1;

        if let Some(first) = self.data.first() {
            if first.clone() == line.trim().to_uppercase() {
                return;
            }
        }
        self.data.insert(0, line.trim().to_uppercase());
        if self.data.len() > MAX_SIZE {
            self.data.truncate(MAX_SIZE);
        }
    }

    pub fn get(&mut self, direction: Direction) -> Option<String> {
        if self.cursor == -1 {
            self.cursor = 0;
        } else if let Direction::Previous = direction {
            self.cursor += 1;
        } else if self.cursor>0 {
            self.cursor -= 1;
        }

        self.data.get(self.cursor as usize).cloned()
    }
}