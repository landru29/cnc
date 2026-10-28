use anyhow::{Context, Result};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    fmt,
};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone)]
pub struct ErrorLogger {
    file: Arc<Mutex<File>>,
}

pub enum Level {
    Error,
    Info,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "ERR"),
            Self::Info => write!(f, "INFO"),
        }
    }
}

impl ErrorLogger {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())
            .with_context(|| format!("Failed to open log file {}", path.as_ref().display()))?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub fn log(&self, level: Level, message: impl std::fmt::Display) -> Result<()> {
        let timestamp = OffsetDateTime::now_utc().format(&Rfc3339)?;
        let mut file = self
            .file
            .lock()
            .map_err(|_| anyhow::anyhow!("Log file lock is poisoned"))?;

        writeln!(file, "[{timestamp}] {level}: {message}")?;
        file.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorLogger;
    use super::Level;
    use std::{fs, time::SystemTime};
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};

    #[test]
    fn appends_errors_to_the_log_file() {
        let timestamp = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("cnc-errors-{}-{timestamp}.log", std::process::id()));

        let logger = ErrorLogger::new(&path).unwrap();
        logger.log(Level::Error,"first error").unwrap();
        logger.clone().log(Level::Error,"second error").unwrap();
        drop(logger);

        let contents = fs::read_to_string(&path).unwrap();
        fs::remove_file(path).unwrap();
        let lines: Vec<_> = contents.lines().collect();

        assert_eq!(lines.len(), 2);
        for line in &lines {
            let timestamp = line
                .split_once("] ERROR:")
                .unwrap()
                .0
                .strip_prefix('[')
                .unwrap();
            assert!(OffsetDateTime::parse(timestamp, &Rfc3339).is_ok());
        }
        assert!(lines[0].ends_with("ERROR: first error"));
        assert!(lines[1].ends_with("ERROR: second error"));
    }
}
