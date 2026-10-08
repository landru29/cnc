// such state is requested with commang "$G"


use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Motion {
    #[default]
    Seek, // G0
    Linear,   // G1
    ArcCw,    // G2
    ArcCcw,   // G3
    ProbeToward,        // G38.2
    ProbeTowardNoError, // G38.3
    ProbeAway,          // G38.4
    ProbeAwayNoError,   // G38.5
    None,     // G80
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Plane {
    #[default]
    XY, // G17
    ZX, // G18
    YZ, // G19
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistanceMode {
    #[default]
    Absolute, // G90
    Relative, // G91
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArcDistanceMode {
    Absolute, // G90.1
    #[default]
    Relative, // G91.1 (GRBL default, sometimes absent from the response)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FeedMode {
    InverseTime, // G93
    #[default]
    UnitsPerMinute, // G94
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Units {
    Inches, // G20
    #[default]
    Millimeters, // G21
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolLengthOffset {
    #[default]
    Cancel, // G49
    Dynamic, // G43.1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgramFlow {
    #[default]
    Running,
    Pause,         // M0
    OptionalPause, // M1
    End,           // M2
    EndAndReset,   // M30
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Spindle {
    Clockwise, // M3
    CounterClockwise, // M4
    #[default]
    Off, // M5
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Coolant {
    pub mist: bool,  // M7
    pub flood: bool, // M8
}

#[derive(Debug, Clone, PartialEq)]
pub struct MachineConfig {
    pub motion: Motion,
    /// 54..=59 for G54..G59
    pub work_coordinate_system: u8,
    pub plane: Plane,
    pub units: Units,
    pub distance: DistanceMode,
    pub arc_distance: ArcDistanceMode,
    pub feed_mode: FeedMode,
    pub tool_length_offset: ToolLengthOffset,
    pub program_flow: ProgramFlow,
    pub spindle: Spindle,
    pub coolant: Coolant,
    pub tool: u32,
    pub feed_rate: f64,
    pub spindle_speed: f64,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            motion: Motion::default(),
            work_coordinate_system: 54,
            plane: Plane::default(),
            units: Units::default(),
            distance: DistanceMode::default(),
            arc_distance: ArcDistanceMode::default(),
            feed_mode: FeedMode::default(),
            tool_length_offset: ToolLengthOffset::default(),
            program_flow: ProgramFlow::default(),
            spindle: Spindle::default(),
            coolant: Coolant::default(),
            tool: 0,
            feed_rate: 0.0,
            spindle_speed: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineConfigError {
    /// The line does not look like `[GC:...]`.
    NotAMachineConfig,
    /// Unknown or malformed G/M/T/F/S word.
    UnknownWord(String),
}

impl fmt::Display for MachineConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MachineConfigError::NotAMachineConfig => write!(f, "unrecognized line (expected `[GC:...]`)"),
            MachineConfigError::UnknownWord(w) => write!(f, "unknown or invalid word: `{w}`"),
        }
    }
}

impl std::error::Error for MachineConfigError {}

impl FromStr for MachineConfig {
    type Err = MachineConfigError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let inner = line
            .trim()
            .strip_prefix("[GC:")
            .and_then(|s| s.strip_suffix(']'))
            .ok_or(MachineConfigError::NotAMachineConfig)?;

        let mut st = MachineConfig::default();
        // M7/M8 mutations are cumulative: start from scratch.
        st.coolant = Coolant::default();

        for word in inner.split_whitespace() {
            let bad = || MachineConfigError::UnknownWord(word.to_string());
            let (letter, value) = word.split_at(1);

            match letter {
                "G" => match value {
                    "0" | "00" => st.motion = Motion::Seek,
                    "1" | "01" => st.motion = Motion::Linear,
                    "2" | "02" => st.motion = Motion::ArcCw,
                    "3" | "03" => st.motion = Motion::ArcCcw,
                    "38.2" => st.motion = Motion::ProbeToward,
                    "38.3" => st.motion = Motion::ProbeTowardNoError,
                    "38.4" => st.motion = Motion::ProbeAway,
                    "38.5" => st.motion = Motion::ProbeAwayNoError,
                    "80" => st.motion = Motion::None,
                    "54" => st.work_coordinate_system = 54,
                    "55" => st.work_coordinate_system = 55,
                    "56" => st.work_coordinate_system = 56,
                    "57" => st.work_coordinate_system = 57,
                    "58" => st.work_coordinate_system = 58,
                    "59" => st.work_coordinate_system = 59,
                    "17" => st.plane = Plane::XY,
                    "18" => st.plane = Plane::ZX,
                    "19" => st.plane = Plane::YZ,
                    "20" => st.units = Units::Inches,
                    "21" => st.units = Units::Millimeters,
                    "90" => st.distance = DistanceMode::Absolute,
                    "91" => st.distance = DistanceMode::Relative,
                    "90.1" => st.arc_distance = ArcDistanceMode::Absolute,
                    "91.1" => st.arc_distance = ArcDistanceMode::Relative,
                    "93" => st.feed_mode = FeedMode::InverseTime,
                    "94" => st.feed_mode = FeedMode::UnitsPerMinute,
                    "43.1" => st.tool_length_offset = ToolLengthOffset::Dynamic,
                    "49" => st.tool_length_offset = ToolLengthOffset::Cancel,
                    "40" => {} // tool compensation disabled: only supported value
                    _ => return Err(bad()),
                },
                "M" => match value {
                    "0" | "00" => st.program_flow = ProgramFlow::Pause,
                    "1" | "01" => st.program_flow = ProgramFlow::OptionalPause,
                    "2" | "02" => st.program_flow = ProgramFlow::End,
                    "30" => st.program_flow = ProgramFlow::EndAndReset,
                    "3" | "03" => st.spindle = Spindle::Clockwise,
                    "4" | "04" => st.spindle = Spindle::CounterClockwise,
                    "5" | "05" => st.spindle = Spindle::Off,
                    "7" | "07" => st.coolant.mist = true,
                    "8" | "08" => st.coolant.flood = true,
                    "9" | "09" => st.coolant = Coolant::default(),
                    _ => return Err(bad()),
                },
                "T" => st.tool = value.parse().map_err(|_| bad())?,
                "F" => st.feed_rate = value.parse().map_err(|_| bad())?,
                "S" => st.spindle_speed = value.parse().map_err(|_| bad())?,
                _ => return Err(bad()),
            }
        }
        Ok(st)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_reset_state() {
        let st: MachineConfig = "[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]".parse().unwrap();
        assert_eq!(st.distance, DistanceMode::Absolute);
        assert_eq!(st.work_coordinate_system, 54);
        assert_eq!(st.units, Units::Millimeters);
        assert_eq!(st.spindle, Spindle::Off);
        assert_eq!(st, MachineConfig::default());
    }

    #[test]
    fn relative_mode_with_values() {
        let st: MachineConfig = "[GC:G1 G55 G18 G20 G91 G91.1 G93 M3 M8 T2 F1200.5 S10000]"
            .parse()
            .unwrap();
        assert_eq!(st.motion, Motion::Linear);
        assert_eq!(st.work_coordinate_system, 55);
        assert_eq!(st.plane, Plane::ZX);
        assert_eq!(st.units, Units::Inches);
        assert_eq!(st.distance, DistanceMode::Relative);
        assert_eq!(st.feed_mode, FeedMode::InverseTime);
        assert_eq!(st.spindle, Spindle::Clockwise);
        assert!(st.coolant.flood && !st.coolant.mist);
        assert_eq!(st.tool, 2);
        assert_eq!(st.feed_rate, 1200.5);
        assert_eq!(st.spindle_speed, 10000.0);
    }

    #[test]
    fn mist_and_flood_together() {
        let st: MachineConfig = "[GC:G0 G54 G17 G21 G90 G94 M5 M7 M8 T0 F0 S0]".parse().unwrap();
        assert!(st.coolant.mist && st.coolant.flood);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!("ok".parse::<MachineConfig>(), Err(MachineConfigError::NotAMachineConfig));
        assert!(matches!(
            "[GC:G999]".parse::<MachineConfig>(),
            Err(MachineConfigError::UnknownWord(_))
        ));
    }
}