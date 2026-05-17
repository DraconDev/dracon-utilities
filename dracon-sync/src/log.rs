//! Structured logging — human-readable to stderr.

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
pub(crate) fn log(level: Level, msg: &str) {
    eprintln!("{} {}", level.emoji(), msg);
}

/// Emit a human-readable log line with repo context.
#[allow(dead_code)]
pub(crate) fn log_repo(level: Level, repo: &str, msg: &str) {
    eprintln!("{} [{}] {}", level.emoji(), repo, msg);
}

/// Emit a human-readable log line with module context.
#[allow(dead_code)]
pub(crate) fn log_module(level: Level, module: &str, msg: &str) {
    eprintln!("{} [{}] {}", level.emoji(), module, msg);
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
