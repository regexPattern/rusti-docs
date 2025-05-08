use std::sync::mpsc::SendError;

use chrono::Local;

#[derive(Debug, PartialEq)]
pub struct LogMsg {
    pub msg: String,
    pub level: LogLevel,
}

#[derive(Debug, PartialEq)]
pub enum LogLevel {
    Info,
    Debug,
    Error,
}

pub type Error = SendError<LogMsg>;

impl std::fmt::Display for LogMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let now = Local::now();
        let ts = now.format("%d-%m-%Y %H:%M:%S");

        let prefix = match self.level {
            LogLevel::Info => concat!("\x1b[92m", "INFO", "\x1b[0m"),
            LogLevel::Debug => concat!("\x1b[94m", "DEBUG", "\x1b[0m"),
            LogLevel::Error => concat!("\x1b[91m", "ERROR", "\x1b[0m"),
        };

        write!(f, "{ts} {prefix} {}", self.msg)
    }
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        $crate::LogMsg { msg: format!($($arg)*), level: $crate::LogLevel::Info }
    });
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        $crate::LogMsg { msg: format!($($arg)*), level: $crate::LogLevel::Debug }
    });
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        $crate::LogMsg { msg: format!($($arg)*), level: $crate::LogLevel::Error }
    });
}
