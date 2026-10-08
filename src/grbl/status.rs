use std::fmt;
use std::str::FromStr;
use std::ops::Add;
use super::position::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Ok,
    Error(u8),
    Alarm(u8),
    Message(String),
    Feed(u32),
    RealTime(RealTime),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusError {
    /// The line does not look like `<...|...>`.
    NotAStatus,
    /// Unknown or malformed parts.
    UnknownWord(String),
    /// A known field has an invalid value.
    Malformed(String),
}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAStatus => write!(f, "unrecognized line (expected `<...|...>`)"),
            Self::UnknownWord(w) => write!(f, "unknown or invalid word: `{w}`"),
            Self::Malformed(w) => write!(f, "malformed value: `{w}`"),
        }
    }
}

impl std::error::Error for StatusError {}


impl FromStr for Status {
    type Err = StatusError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let data = line.trim();
        if data == "ok" {
            Ok(Status::Ok)
        } else if data.starts_with("error:") {
            let code = data[6..]
                .trim()
                .parse::<u8>()
                .map_err(|_| StatusError::Malformed(data.to_string()))?;
            Ok(Status::Error(code))
        } else if data.starts_with("ALARM:") {
            let code = data[6..]
                .trim()
                .parse::<u8>()
                .map_err(|_| StatusError::Malformed(data.to_string()))?;
            Ok(Status::Alarm(code))
        } else if data.starts_with("MSG:") {
            let message = data[4..].trim().to_string();
            Ok(Status::Message(message))
        } else if data.starts_with("F:") {
            let feed = data[2..]
                .trim()
                .parse::<u32>()
                .map_err(|_| StatusError::Malformed(data.to_string()))?;
            Ok(Status::Feed(feed))
        } else if data.starts_with('<') && data.ends_with('>') {
            Ok(Status::RealTime(data.parse()?))
        } else {
            Err(StatusError::NotAStatus)
        }
    }
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealTime {
    pub state: Option<State>,
    pub machine_position: Option<Position>,
    pub work_position: Option<Position>,
    pub work_coordinate_offset: Option<Position>,
    pub line_number: Option<u32>,
    pub buffer: Option<(u32, u32)>,
}

impl RealTime {
    pub fn position(&self) -> Position {
        if let Some(pos) = &self.machine_position {
            return pos.clone();
        }

        if let Some(wpos) = &self.work_position && let Some(offset) = &self.work_coordinate_offset {
            return wpos.clone() + offset.clone();
        }

        Position::default()
    }
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    Run,
    Hold,
    Jog,
    Alarm,
    Door,
    Check,
    Home,
    Sleep,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Run => write!(f, "Run"),
            Self::Hold => write!(f, "Hold"),
            Self::Jog => write!(f, "Jog"),
            Self::Alarm => write!(f, "Alarm"),
            Self::Door => write!(f, "Door"),
            Self::Check => write!(f, "Check"),
            Self::Home => write!(f, "Home"),
            Self::Sleep => write!(f, "Sleep"),
        }
    }
}


impl Add for RealTime {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            state: if other.state == None {self.state} else {other.state},
            machine_position: if other.machine_position == None {self.machine_position} else {other.machine_position},
            work_position: if other.work_position == None {self.work_position} else {other.work_position},
            work_coordinate_offset: if other.work_coordinate_offset == None {self.work_coordinate_offset} else {other.work_coordinate_offset},
            line_number: if other.line_number == None {self.line_number} else {other.line_number},
            buffer: if other.buffer == None {self.buffer} else {other.buffer},
        }
    }
}


impl Default for RealTime {
    fn default() -> Self {
        Self {
            state: None,
            machine_position: None,
            work_position: None,
            work_coordinate_offset: None,
            line_number: None,
            buffer: None,
        }
    }
}

impl FromStr for RealTime {
    type Err = StatusError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let data = line.trim();
        if !(data.starts_with('<') && data.ends_with('>')) {
            return Err(StatusError::NotAStatus);
        }

        let content = &data[1..data.len() - 1];
        let mut rt = RealTime::default();

        for part in content.split('|') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some(coords) = part.strip_prefix("MPos:") {
                rt.machine_position = Some(parse_coords(coords)?);
            } else if let Some(coords) = part.strip_prefix("WPos:") {
                rt.work_position = Some(parse_coords(coords)?);
            } else if let Some(coords) = part.strip_prefix("WCO:") {
                rt.work_coordinate_offset = Some(parse_coords(coords)?);
            } else if let Some(value) = part.strip_prefix("Ln:") {
                rt.line_number = Some(
                    value
                        .trim()
                        .parse::<u32>()
                        .map_err(|_| StatusError::Malformed(part.to_string()))?,
                );
            } else if let Some(value) = part.strip_prefix("Buf:") {
                let values: Vec<&str> = value.split(',').collect();
                if values.len() != 2 {
                    return Err(StatusError::Malformed(part.to_string()));
                }
                let available = values[0]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| StatusError::Malformed(part.to_string()))?;
                let used = values[1]
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| StatusError::Malformed(part.to_string()))?;
                rt.buffer = Some((available, used));
            } else if let Some(state) = parse_state(part) {
                rt.state = Some(state);
            } else {
                return Err(StatusError::UnknownWord(part.to_string()));
            }
        }

        Ok(rt)
    }
}

fn parse_coords(s: &str) -> Result<Position, StatusError> {
    let coords: Vec<&str> = s.split(',').collect();
    if coords.len() != 3 {
        return Err(StatusError::Malformed(s.to_string()));
    }
    let x = coords[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| StatusError::Malformed(s.to_string()))?;
    let y = coords[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| StatusError::Malformed(s.to_string()))?;
    let z = coords[2]
        .trim()
        .parse::<f64>()
        .map_err(|_| StatusError::Malformed(s.to_string()))?;
    Ok(Position{x, y, z})
}

fn parse_state(part: &str) -> Option<State> {
    match part {
        "Idle" => Some(State::Idle),
        "Run" => Some(State::Run),
        "Hold" => Some(State::Hold),
        "Jog" => Some(State::Jog),
        "Alarm" => Some(State::Alarm),
        "Door" => Some(State::Door),
        "Check" => Some(State::Check),
        "Home" => Some(State::Home),
        "Sleep" => Some(State::Sleep),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_time_status() {
        let status: RealTime =
            "<Idle|MPos:10,20,30|WPos:1,2,3|WCO:4,5,6|Ln:42|Buf:7,8>".parse().unwrap();

        assert_eq!(
            status,
            RealTime {
                state: Some(State::Idle),
                machine_position: Some(Position{x:10.0, y:20.0, z:30.0}),
                work_position: Some(Position{x:1.0, y:2.0, z:3.0}),
                work_coordinate_offset: Some(Position{x:4.0, y:5.0, z:6.0}),
                line_number: Some(42),
                buffer: Some((7, 8)),
            }
        );
    }

    #[test]
    fn rejects_invalid_real_time_fields() {
        assert!(matches!(
            "<Idle|MPos:bad|Buf:bad,2>".parse::<RealTime>(),
            Err(StatusError::Malformed(_))
        ));
        assert!(matches!(
            "<Idle|Bogus:1>".parse::<RealTime>(),
            Err(StatusError::UnknownWord(_))
        ));
        assert!(matches!(
            "not a status".parse::<RealTime>(),
            Err(StatusError::NotAStatus)
        ));
    }

    #[test]
    fn parses_status_variants() {
        assert!(matches!("ok".parse(), Ok(Status::Ok)));
        assert!(matches!("error:12".parse(), Ok(Status::Error(12))));
        assert!(matches!("ALARM:4".parse(), Ok(Status::Alarm(4))));
        assert!(matches!(
            "MSG:hello world".parse(),
            Ok(Status::Message(message)) if message == "hello world"
        ));
        assert!(matches!("F:123".parse(), Ok(Status::Feed(123))));
        assert!(matches!(
            "<Idle|MPos:10,20,30>".parse(),
            Ok(Status::RealTime(rt)) if rt == RealTime {
                state: Some(State::Idle),
                machine_position: Some(Position{x:10.0, y:20.0, z:30.0}),
                work_position: None,
                work_coordinate_offset: None,
                line_number: None,
                buffer: None,
            }
        ));
    }

    #[test]
    fn rejects_invalid_status_inputs() {
        assert!(matches!("".parse::<Status>(), Err(StatusError::NotAStatus)));
        assert!(matches!(
            "error:abc".parse::<Status>(),
            Err(StatusError::Malformed(_))
        ));
        assert!(matches!(
            "ALARM:999".parse::<Status>(),
            Err(StatusError::Malformed(_))
        ));
        assert!(matches!(
            "MSG:".parse::<Status>(),
            Ok(Status::Message(message)) if message.is_empty()
        ));
        assert!(matches!(
            "F:abc".parse::<Status>(),
            Err(StatusError::Malformed(_))
        ));
        assert!(matches!(
            "<Idle|MPos:bad>".parse::<Status>(),
            Err(StatusError::Malformed(_))
        ));
    }

    #[test]
    fn parses_statuses_from_str() {
        let status: Status = "<Idle|MPos:1,2,3>".parse().unwrap();
        assert!(matches!(
            status,
            Status::RealTime(rt) if rt == RealTime {
                state: Some(State::Idle),
                machine_position: Some(Position{x:1.0, y:2.0, z:3.0}),
                work_position: None,
                work_coordinate_offset: None,
                line_number: None,
                buffer: None,
            }
        ));

        assert!(matches!(
            "garbage".parse::<Status>(),
            Err(StatusError::NotAStatus)
        ));
    }
}

