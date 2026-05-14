use std::time::{SystemTime, UNIX_EPOCH};

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    fn as_str(&self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        }
    }
}

/// Structured log event — emitted as JSONL for machine parsing.
#[derive(Debug, serde::Serialize)]
pub(crate) struct Event<'a> {
    ts: u64,
    level: &'a str,
    msg: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<&'a str>,
}

fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Emit a structured log line to stderr as JSON.
pub(crate) fn log(level: Level, msg: &str) {
    let event = Event {
        ts: timestamp_secs(),
        level: level.as_str(),
        msg,
        repo: None,
        module: None,
    };
    if let Ok(line) = serde_json::to_string(&event) {
        eprintln!("{}", line);
    }
}

/// Emit a structured log line with optional repo path.
pub(crate) fn log_repo(level: Level, repo: &str, msg: &str) {
    let event = Event {
        ts: timestamp_secs(),
        level: level.as_str(),
        msg,
        repo: Some(repo),
        module: None,
    };
    if let Ok(line) = serde_json::to_string(&event) {
        eprintln!("{}", line);
    }
}

/// Emit a structured log line with optional module name.
pub(crate) fn log_module(level: Level, module: &str, msg: &str) {
    let event = Event {
        ts: timestamp_secs(),
        level: level.as_str(),
        msg,
        repo: None,
        module: Some(module),
    };
    if let Ok(line) = serde_json::to_string(&event) {
        eprintln!("{}", line);
    }
}

/// Convenience macros for each level.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Error, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Warn, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Info, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Debug, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_repo_warn {
    ($repo:expr, $($arg:tt)*) => {
        $crate::log::log_repo($crate::log::Level::Warn, $repo, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_repo_info {
    ($repo:expr, $($arg:tt)*) => {
        $crate::log::log_repo($crate::log::Level::Info, $repo, &format!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_repo_debug {
    ($repo:expr, $($arg:tt)*) => {
        $crate::log::log_repo($crate::log::Level::Debug, $repo, &format!($($arg)*));
    };
}
