use std::ops::Add;

use super::machine::MachineConfig;
use super::status::RealTime;
use ratatui::{
    text::Text,
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
        Text::from(format!("{}    {}    {}    {}    {}    {}",
            self.config.distance,
            self.status
                .state
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            self.status.position(),
            self.status
                .feed_speed
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.status.buffer
                .as_ref()
                .map(|b| b.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.description,
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