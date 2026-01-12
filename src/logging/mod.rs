mod default_logging;
mod error;
mod systemd_logging;

pub use default_logging::init_logging_async;
pub use systemd_logging::init_logging_async_systemd;
