pub use crate::config::{
    Config, ConfigReadOption, mpclipboard_config_new, mpclipboard_config_read,
};
use crate::{clip::Clip, event::Event, thread::Thread};
pub use handle::{Handle, Output, mpclipboard_poll, mpclipboard_send};
use tls::TLS;
use tokio::sync::mpsc::channel;

mod clip;
mod config;
mod connection;
mod event;
mod handle;
mod logger;
mod main_loop;
mod store;
mod thread;
mod tls;

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_init() -> bool {
    logger::init();
    TLS::init()
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_start_thread(config: *mut Config) -> *mut Handle {
    let Some(config) = Config::from_ptr(config) else {
        log::error!("no config provided");
        return std::ptr::null_mut();
    };

    let (ctx, crx) = channel::<Clip>(256);
    let (etx, erx) = channel::<Event>(256);

    let thread = Thread::spawn(crx, etx, config);

    Box::leak(Box::new(Handle { ctx, erx, thread }))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_stop_thread(handle: *mut Handle) -> bool {
    let Some(handle) = Handle::owned_from_ptr(handle) else {
        log::error!("NULL handle");
        return false;
    };

    if let Err(err) = handle.thread.stop() {
        log::error!("{err:?}");
        false
    } else {
        true
    }
}
