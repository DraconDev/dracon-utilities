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
/// Automatically flushes to ensure journald captures each line immediately.
pub(crate) fn log(level: Level, msg: &str) {
    eprintln!("{} {}", level.emoji(), msg);
    use std::io::Write;
    let _ = std::io::stderr().flush();
}

/// Log a warning-level message.
#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::log::log($crate::log::Level::Warn, &format!($($arg)*));
    };
}
