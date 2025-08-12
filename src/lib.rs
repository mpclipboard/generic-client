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
