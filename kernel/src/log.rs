//! Kernel logging: leveled macros over a serial sink.
//!
//! A minimal structured logger for M1. Records at or above the current level
//! are formatted as `[LEVEL] message` and written to COM1, which is guarded by
//! a [`SpinLock`] so logging is sound once concurrency exists. A second sink
//! (the framebuffer console, #14) and richer formatting come later.
//!
//! The logger is deliberately **not** used by the panic handler: a panic can
//! occur while the serial lock is held, so the handler writes to its own fresh
//! serial port instead of risking a deadlock here.

use core::sync::atomic::{AtomicU8, Ordering};

use hal::serial::SerialPort;
use hal::sync::SpinLock;

/// Log severity, ordered from most to least severe.
///
/// Not every variant is constructed yet — the full set is used across M1 as
/// subsystems start logging at their natural levels.
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// A fault or failure.
    Error = 1,
    /// Something unexpected that is not fatal.
    Warn = 2,
    /// Normal, noteworthy progress.
    Info = 3,
    /// Detailed information for debugging.
    Debug = 4,
    /// Very verbose tracing.
    Trace = 5,
}

impl Level {
    /// The level's short uppercase name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        }
    }
}

static MAX_LEVEL: AtomicU8 = AtomicU8::new(Level::Info as u8);
static SERIAL: SpinLock<SerialPort> = SpinLock::new(SerialPort::com1());

/// Initialize the logging backend (the serial sink). Call once, early in
/// `kmain`, before using the logging macros.
pub fn init() {
    SERIAL.lock().init();
}

/// Set the maximum severity that will be emitted; records above it are dropped.
pub fn set_max_level(level: Level) {
    MAX_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// Whether a record at `level` would currently be emitted.
#[must_use]
pub fn enabled(level: Level) -> bool {
    (level as u8) <= MAX_LEVEL.load(Ordering::Relaxed)
}

/// Emit one formatted record. Prefer the [`info!`](crate::info) family of
/// macros over calling this directly.
pub fn write(level: Level, args: core::fmt::Arguments) {
    use core::fmt::Write;
    if !enabled(level) {
        return;
    }
    let mut serial = SERIAL.lock();
    let _ = writeln!(serial, "[{}] {}", level.as_str(), args);
}

/// Emit a log record at an explicit [`Level`]. The level-specific macros
/// ([`info!`](crate::info), etc.) are usually clearer.
#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        $crate::log::write($level, core::format_args!($($arg)*))
    };
}

/// Log at [`Level::Error`](crate::log::Level::Error).
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Error, core::format_args!($($arg)*))
    };
}

/// Log at [`Level::Warn`](crate::log::Level::Warn).
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Warn, core::format_args!($($arg)*))
    };
}

/// Log at [`Level::Info`](crate::log::Level::Info).
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Info, core::format_args!($($arg)*))
    };
}

/// Log at [`Level::Debug`](crate::log::Level::Debug).
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Debug, core::format_args!($($arg)*))
    };
}

/// Log at [`Level::Trace`](crate::log::Level::Trace).
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::log::write($crate::log::Level::Trace, core::format_args!($($arg)*))
    };
}
