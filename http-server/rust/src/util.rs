pub use tracing::error;
pub use tracing::info;

#[macro_export]
macro_rules! log_info {
    ($w:ident, $m:literal) => (info!(concat!("[{}] ", $m), $w.name()));
    ($w:ident, $m:literal, $($arg:expr),+) => (info!(concat!("[{}] ", $m), $w.name(), $($arg),+));
}

#[macro_export]
macro_rules! log_error {
    ($w:ident, $m:literal) => (error!(concat!("[{}] ", $m), $w.name()));
    ($w:ident, $m:literal, $($arg:expr),+) => (error!(concat!("[{}] ", $m), $w.name(), $($arg),+));
}

