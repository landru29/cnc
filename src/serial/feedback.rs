use std::fmt;
use std::str::FromStr;
use crate::grbl::{machine::MachineConfig, status::Status};

pub enum Feedback {
    Message(String),
    Status(Status),
    MachineConfig(MachineConfig)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackError {}

impl fmt::Display for FeedbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}


impl std::error::Error for FeedbackError {}

impl FromStr for Feedback {
    type Err = FeedbackError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        if let Ok(status) = line.parse::<Status>() {
            Ok(Feedback::Status(status))
        } else if let Ok(machine_config) = line.parse::<MachineConfig>() {
            Ok(Feedback::MachineConfig(machine_config))
        } else {
            Ok(Feedback::Message(line.to_string()))
        }
    }
}