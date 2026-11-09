use std::fmt;
use std::ops::Add;

#[derive(Debug, Clone)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == self.y && self.z == other.z
    }
}

impl Eq for Position {}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dx = if self.dx>0.0 {
            "↑"
        } else if self.dx<0.0 {
            "↓"
        } else {
            " "
        };
        let dy = if self.dy>0.0 {
            "↑"
        } else if self.dy<0.0 {
            "↓"
        } else {
            " "
        };
        let dz = if self.dz>0.0 {
            "↑"
        } else if self.dz<0.0 {
            "↓"
        } else {
            " "
        };
        write!(f, "{}X: {:+07.2}     {}Y: {:+07.2}      {}Z: {:+07.2}", dx,  self.x, dy, self.y, dz, self.z)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            dx: 0.0,
            dy: 0.0,
            dz: 0.0,
        }
    }
}

impl Add for Position {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            dx: other.x -self.x,
            dy: other.y -self.y,
            dz: other.z -self.z,
        }
    }
}