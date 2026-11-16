use std::fmt;
use std::str::FromStr;
use crate::grbl::{machine::MachineConfig, status::Status};

pub enum Feedback {
    Status(Status),
    Ok,
    MachineConfig(MachineConfig),
    Other(String),
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackError {}

impl fmt::Display for FeedbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

impl fmt::Display for Feedback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Feedback::Ok => write!(f, "Ok"),
            Feedback::Status(status) => write!(f, "Status: {:?}", status),
            Feedback::MachineConfig(config) => write!(f, "MachineConfig: {:?}", config),
            Feedback::Other(config) => write!(f, "Other: {:?}", config),
        }
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
        } else if line.to_lowercase().trim_end() == "ok".to_string() {
            Ok(Feedback::Ok)
        } else {
            Ok(Feedback::Other(line.to_string()))
        }
    }
}