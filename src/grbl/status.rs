pub enum Status {
    Ok,
    Error(u8),
    Alarm(u8),
    Message(String),
    Feed(u32),
    RealTime(Vec<RealTime>),
}

pub enum RealTime {
    State(State),
    MachinePosition(i32, i32, i32),
    WorkPosition(i32, i32, i32),
    WorkCoordinateOffset(i32, i32, i32),
    LineNumber(u32),
    Buffer(u32, u32),
}

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

impl Status {
    pub fn new(data: &str) -> Option<Status> {
        let data = data.trim();
        if data.starts_with("ok") {
            Some(Status::Ok)
        } else if data.starts_with("error:") {
            let code = data[6..].trim().parse::<u8>().ok()?;
            Some(Status::Error(code))
        } else if data.starts_with("ALARM:") {
            let code = data[6..].trim().parse::<u8>().ok()?;
            Some(Status::Alarm(code))
        } else if data.starts_with("MSG:") {
            let message = data[4..].trim().to_string();
            Some(Status::Message(message))
        } else if data.starts_with("F:") {
            let feed = data[2..].trim().parse::<u32>().ok()?;
            Some(Status::Feed(feed))
        } else if data.starts_with("<") && data.ends_with(">") {
            Some(Status::RealTime(RealTime::new(data)))
        } else {
            None
        }
    }
}

impl RealTime {
    pub fn new(data: &str) -> Vec<RealTime> {
        let data = data.trim();
        let mut result = Vec::new();

        if !(data.starts_with("<") && data.ends_with(">")) {
            return result;
        }

        let content = &data[1..data.len() - 1];
        let parts: Vec<&str> = content.split('|').collect();

        for part in parts {
            if part.starts_with("MPos:") {
                let coords: Vec<&str> = part[5..].split(',').collect();
                if coords.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        coords[0].parse::<i32>(),
                        coords[1].parse::<i32>(),
                        coords[2].parse::<i32>(),
                    ) {
                        result.push(RealTime::MachinePosition(x, y, z));
                    }
                }
            } else if part.starts_with("WPos:") {
                let coords: Vec<&str> = part[5..].split(',').collect();
                if coords.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        coords[0].parse::<i32>(),
                        coords[1].parse::<i32>(),
                        coords[2].parse::<i32>(),
                    ) {
                        result.push(RealTime::WorkPosition(x, y, z));
                    }
                }
            } else if part.starts_with("WCO:") {
                let coords: Vec<&str> = part[4..].split(',').collect();
                if coords.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        coords[0].parse::<i32>(),
                        coords[1].parse::<i32>(),
                        coords[2].parse::<i32>(),
                    ) {
                        result.push(RealTime::WorkCoordinateOffset(x, y, z));
                    }
                }
            } else if part.starts_with("Ln:") {
                if let Ok(line_number) = part[3..].parse::<u32>() {
                    result.push(RealTime::LineNumber(line_number));
                }
            } else if part.starts_with("Buf:") {
                let values: Vec<&str> = part[4..].split(',').collect();
                if values.len() == 2 {
                    if let (Ok(available), Ok(used)) = (
                        values[0].parse::<u32>(),
                        values[1].parse::<u32>(),
                    ) {
                        result.push(RealTime::Buffer(available, used));
                    }
                }
            } else if part == "Idle"
                || part == "Run"
                || part == "Hold"
                || part == "Jog"
                || part == "Alarm"
                || part == "Door"
                || part == "Check"
                || part == "Home"
                || part == "Sleep"
            {
                let state = match part {
                    "Idle" => State::Idle,
                    "Run" => State::Run,
                    "Hold" => State::Hold,
                    "Jog" => State::Jog,
                    "Alarm" => State::Alarm,
                    "Door" => State::Door,
                    "Check" => State::Check,
                    "Home" => State::Home,
                    "Sleep" => State::Sleep,
                    _ => unreachable!(),
                };
                result.push(RealTime::State(state));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_time_status() {
        let status = RealTime::new(
            "<Idle|MPos:10,20,30|WPos:1,2,3|WCO:4,5,6|Ln:42|Buf:7,8>",
        );

        assert!(matches!(
            status.as_slice(),
            [
                RealTime::State(State::Idle),
                RealTime::MachinePosition(10, 20, 30),
                RealTime::WorkPosition(1, 2, 3),
                RealTime::WorkCoordinateOffset(4, 5, 6),
                RealTime::LineNumber(42),
                RealTime::Buffer(7, 8),
            ]
        ));
    }

    #[test]
    fn ignores_invalid_real_time_fields() {
        let status = RealTime::new("<Idle|MPos:bad|Buf:bad,2>");

        assert!(matches!(status.as_slice(), [RealTime::State(State::Idle)]));
    }

    #[test]
    fn parses_status_variants() {
        assert!(matches!(Status::new("ok"), Some(Status::Ok)));
        assert!(matches!(Status::new("error:12"), Some(Status::Error(12))));
        assert!(matches!(Status::new("ALARM:4"), Some(Status::Alarm(4))));
        assert!(matches!(
            Status::new("MSG:hello world"),
            Some(Status::Message(message)) if message == "hello world"
        ));
        assert!(matches!(Status::new("F:123"), Some(Status::Feed(123))));
        assert!(matches!(
            Status::new("<Idle|MPos:10,20,30>"),
            Some(Status::RealTime(items)) if matches!(items.as_slice(), [
                RealTime::State(State::Idle),
                RealTime::MachinePosition(10, 20, 30),
            ])
        ));
    }

    #[test]
    fn ignores_invalid_status_inputs() {
        assert!(Status::new("").is_none());
        assert!(Status::new("error:abc").is_none());
        assert!(Status::new("ALARM:999").is_none());
        assert!(matches!(
            Status::new("MSG:"),
            Some(Status::Message(message)) if message.is_empty()
        ));
        assert!(Status::new("F:abc").is_none());
        assert!(matches!(
            Status::new("<Idle|MPos:bad>"),
            Some(Status::RealTime(items)) if matches!(items.as_slice(), [
                RealTime::State(State::Idle),
            ])
        ));
    }
}

