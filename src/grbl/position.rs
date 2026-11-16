use std::fmt;
use std::ops::Add;

use ratatui::{
    style::{Color, Style}, text::{Line, Span},
};

#[derive(Debug, Clone)]
pub struct Delta(f64);

#[derive(Debug, Clone)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dx: Delta,
    pub dy: Delta,
    pub dz: Delta,
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == self.y && self.z == other.z
    }
}

impl Eq for Position {}

impl<'a> Into<Line<'a>> for Position {
    fn into(self) -> Line<'a> {
        let spans = vec![
            self.dx.into(),
            Span::styled("X: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:+07.2}", self.x), Style::default().fg(Color::White)),

            Span::styled("     ", Style::default()),
            self.dy.into(),
            Span::styled("Y: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:+07.2}", self.y), Style::default().fg(Color::White)),

            Span::styled("     ", Style::default()),
            self.dz.into(),
            Span::styled("Z: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:+07.2}", self.z), Style::default().fg(Color::White)),
        ];

        Line::from(spans)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}X: {:+07.2}     {}Y: {:+07.2}      {}Z: {:+07.2}", self.dx,  self.x, self.dy, self.y, self.dz, self.z)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            dx: Delta::default(),
            dy: Delta::default(),
            dz: Delta::default(),
        }
    }
}

impl Default for Delta {
    fn default() -> Self {
        Self(0.0)
    }
}

impl Delta {
    pub fn new(this: f64, other: f64) -> Self {
        Self(other - this)
    }

    pub fn movement(&self) -> bool {
        return self.0 != 0.0
    }
}

impl fmt::Display for Delta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", if self.0>0.0 {
            "↑"
        } else if self.0<0.0 {
            "↓"
        } else {
            " "
        })
    }
}

impl<'a> Into<Span<'a>> for Delta {
    fn into(self) -> Span<'a> {
        Span::styled(
            format!("{}", self), 
            if self.0>0.0 {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::Magenta)
            },
        )
    }
}

impl Add for Position {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
            dx: Delta::new(self.x, other.x),
            dy: Delta::new(self.y, other.y),
            dz: Delta::new(self.z, other.z),
        }
    }
}