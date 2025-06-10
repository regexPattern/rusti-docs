use std::fmt;
use std::sync::OnceLock;
use std::sync::mpsc::SendError;

use chrono::Local;

static DEBUG_ENABLED: OnceLock<bool> = OnceLock::new();

#[derive(Debug, PartialEq)]
pub struct Log {
    pub msg: String,
    pub level: LogLevel,
}

#[derive(Debug, PartialEq)]
pub enum LogLevel {
    ClusterDebug,
    ClusterInfo,
    Debug,
    Error,
    Info,
    Warn,
}

pub type Error = SendError<Log>;

impl fmt::Display for Log {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let debug_on = *DEBUG_ENABLED.get_or_init(|| {
            std::env::var("LOG_LEVEL")
                .map(|v| v.eq_ignore_ascii_case("DEBUG"))
                .unwrap_or(false)
        });

        if (self.level == LogLevel::Debug || self.level == LogLevel::ClusterDebug) && !debug_on {
            return Ok(());
        }

        let now = Local::now();
        let ts = now.format("%d-%m-%Y %H:%M:%S");

        let prefix = match self.level {
            LogLevel::ClusterDebug => concat!("\x1b[94m", "CDEBUG", "\x1b[0m"),
            LogLevel::ClusterInfo => concat!("\x1b[92m", "CINFO", "\x1b[0m"),
            LogLevel::Debug => concat!("\x1b[94m", "DEBUG", "\x1b[0m"),
            LogLevel::Error => concat!("\x1b[91m", "ERROR", "\x1b[0m"),
            LogLevel::Info => concat!("\x1b[92m", "INFO", "\x1b[0m"),
            LogLevel::Warn => concat!("\x1b[93m", "WARN", "\x1b[0m"),
        };

        writeln!(f, "{ts} {prefix} {}", self.msg)
    }
}

#[macro_export]
macro_rules! cdebug {
    ($($arg:tt)*) => ({
        $crate::Log { msg: format!($($arg)*), level: $crate::LogLevel::ClusterDebug }
    });
}

#[macro_export]
macro_rules! cinfo {
    ($($arg:tt)*) => ({
        $crate::Log { msg: format!($($arg)*), level: $crate::LogLevel::ClusterInfo }
    });
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        $crate::Log { msg: format!($($arg)*), level: $crate::LogLevel::Debug }
    });
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        $crate::Log { msg: format!($($arg)*), level: $crate::LogLevel::Info }
    });
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ({
        $crate::Log { msg: format!($($arg)*), level: $crate::LogLevel::Warn }
    });
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        $crate::Log { msg: format!($($arg)*), level: $crate::LogLevel::Error }
    });
}
