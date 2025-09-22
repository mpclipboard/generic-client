#![warn(missing_docs)]
#![warn(trivial_casts, trivial_numeric_casts)]
#![warn(unused_qualifications)]
#![warn(deprecated_in_future)]
#![warn(unused_lifetimes)]
#![allow(clippy::boxed_local)]
#![doc = include_str!("../README.md")]

pub use config::{Config, ConfigReadOption, mpclipboard_config_new, mpclipboard_config_read};
pub use handle::{
    Handle, mpclipboard_handle_poll, mpclipboard_handle_send, mpclipboard_handle_stop,
    mpclipboard_handle_take_fd,
};
pub use logger::{Logger, mpclipboard_logger_test};
pub use output::Output;
pub use thread::{Thread, mpclipboard_thread_start};
pub use tls::TLS;

mod clip;
mod config;
mod connection;
mod event;
mod ffi;
mod handle;
mod logger;
mod main_loop;
mod output;
mod store;
mod thread;
mod tls;

/// Initializes MPClipboard's Logger and TLS connector.
///
/// This is the first thing that you must do before calling any
/// MPClipboard functions.
///
/// Returns `false` if TLS connector can't be initialized.
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_init() -> bool {
    Logger::init();

    if let Err(err) = TLS::init() {
        log::error!("failed to init WS connector: {err:?}");
        return false;
    }
    log::info!("TLS Connector has been configured");
    true
}
