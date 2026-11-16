use std::ops::Add;

use super::machine::MachineConfig;
use super::status::RealTime;
use ratatui::{
    style::{Color, Style}, text::{Line, Span, Text},
};


#[derive(Debug, Clone)]
pub struct Configuration{
    pub status: RealTime,
    pub config: MachineConfig,
    pub description: String,
}


impl Configuration {
    pub fn new(description: String) -> Self {
        Self {
            status: RealTime::default(),
            config: MachineConfig::default(),
            description
        }
    }
}

impl<'a> Into<Text<'a>> for Configuration {
    fn into(self) -> Text<'a> {
        let status = self.status.state.clone()
            .map(|state| state.into())
            .unwrap_or_else(|| Span::styled("",  Style::default()));

        let feed_speed = self.status.clone().feed_speed
            .map(|fs| fs.into())
            .unwrap_or_else(|| Line::from(Span::styled("", Style::default())));

        let buffer = self.status.clone().buffer
            .map(|b| b.into())
            .unwrap_or_else(|| Line::from(Span::styled("", Style::default())));

        let line_num = self.status.line_number
            .map(|b| Line::from(vec![
                Span::styled("Ln: ", Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{}", b), Style::default().fg(Color::White)),
            ]))
            .unwrap_or_else(|| Line::from(Span::styled("", Style::default())));

        let distance: Span<'a> = self.config.distance.into();
        let step_mode: Span<'a> = self.status.clone().step_mode.into();

        let lines = vec![
            Line::from(distance),
            Line::from(Span::styled("    ", Style::default())),
            Line::from(status),
            Line::from(Span::styled("    ", Style::default())),
            self.status.position().into(),
            Line::from(Span::styled("    ", Style::default())),
            feed_speed,
            Line::from(Span::styled("    ", Style::default())),
            buffer,
            Line::from(Span::styled("    ", Style::default())),
            Line::from(step_mode),
            Line::from(Span::styled("    ", Style::default())),
            Line::from(Span::styled(self.description, Style::default())),
            Line::from(Span::styled("    ", Style::default())),
            line_num,
        ];

        Text::from(Line::from(
            lines.into_iter()
                .flat_map(|line| line.spans)
                .collect::<Vec<_>>()
        ))
    }
}

impl Add<RealTime> for Configuration {
    type Output = Self;

    fn add(self, rt: RealTime) -> Self {
        Self {
            status: self.status + rt,
            config: self.config,
            description: self.description,
        }
    }
}

impl Add<MachineConfig> for Configuration {
    type Output = Self;

    fn add(self, mc: MachineConfig) -> Self {
        Self {
            status: self.status,
            config: mc,
            description: self.description
        }
    }
}