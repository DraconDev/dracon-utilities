//! Structured logging — human-readable to stderr.

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub(crate) enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    fn emoji(&self) -> &'static str {
        match self {
            Level::Error => "❌",
            Level::Warn => "⚠️",
            Level::Info => "ℹ️",
            Level::Debug => "🔍",
        }
    }
}

/// Emit a human-readable log line to stderr.
/// Automatically flushes to ensure journald captures each line immediately.
pub(crate) fn log(level: Level, msg: &str) {
    eprintln!("{} {}", level.emoji(), msg);
    use std::io::Write;
    let _ = std::io::stderr().flush();
}

/// Emit a human-readable log line with repo context.
/// Automatically flushes to ensure journald captures each line immediately.
#[allow(dead_code)]
pub(crate) fn log_repo(level: Level, repo: &str, msg: &str) {
    eprintln!("{} [{}] {}", level.emoji(), repo, msg);
    use std::io::Write;
    let _ = std::io::stderr().flush();
}

/// Emit a human-readable log line with module context.
/// Automatically flushes to ensure journald captures each line immediately.
#[allow(dead_code)]
pub(crate) fn log_module(level: Level, module: &str, msg: &str) {
    eprintln!("{} [{}] {}", level.emoji(), module, msg);
    use std::io::Write;
    let _ = std::io::stderr().flush();
}

/// Log an error-level message.
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Error, &format!($($arg)*));
    };
}

/// Log a warning-level message.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Warn, &format!($($arg)*));
    };
}

/// Log an info-level message.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Info, &format!($($arg)*));
    };
}

/// Log a debug-level message.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Debug, &format!($($arg)*));
    };
}

/// Log a warning with repo context.
#[macro_export]
macro_rules! log_repo_warn {
    ($repo:expr, $($arg:tt)*) => {
        $crate::log::log_repo($crate::log::Level::Warn, $repo, &format!($($arg)*));
    };
}

/// Log info with repo context.
#[macro_export]
macro_rules! log_repo_info {
    ($repo:expr, $($arg:tt)*) => {
        $crate::log::log_repo($crate::log::Level::Info, $repo, &format!($($arg)*));
    };
}

/// Log debug with repo context.
#[macro_export]
macro_rules! log_repo_debug {
    ($repo:expr, $($arg:tt)*) => {
        $crate::log::log_repo($crate::log::Level::Debug, $repo, &format!($($arg)*));
    };
}
