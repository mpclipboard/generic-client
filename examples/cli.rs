use anyhow::Result;
use libc::free;
use mpclipboard_generic_client::{
    ConfigReadOption, Handle, Output, mpclipboard_clip_drop, mpclipboard_clip_get_text,
    mpclipboard_config_read, mpclipboard_handle_poll, mpclipboard_handle_send,
    mpclipboard_handle_stop, mpclipboard_logger_init, mpclipboard_thread_start,
    mpclipboard_tls_init,
};
use std::io::BufRead as _;

fn main() -> Result<()> {
    mpclipboard_logger_init();
    mpclipboard_tls_init();

    let config = mpclipboard_config_read(ConfigReadOption::FromLocalFile);
    let handle = unsafe { mpclipboard_thread_start(config) };

    let sync_handle = SyncHandle::new(handle);

    std::thread::spawn(move || {
        let handle = sync_handle.unwrap();
        loop {
            let Output { clip, connectivity } = unsafe { mpclipboard_handle_poll(handle) };
            if !clip.is_null() {
                {
                    let text = unsafe { mpclipboard_clip_get_text(clip) };
                    let text = unsafe { std::ffi::CString::from_raw(text.cast()) };
                    let text = text.to_str().unwrap().to_string();
                    log::info!("text = {text:?}");
                }
                unsafe { mpclipboard_clip_drop(clip) };
                unsafe { free(clip.cast()) }
            };
            if !connectivity.is_null() {
                {
                    let connectivity = unsafe { *connectivity };
                    log::info!("connectivity = {connectivity:?}");
                }
                unsafe { free(connectivity.cast()) }
            };
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(input) => {
                if input == "exit" {
                    break;
                }
                let input = std::ffi::CString::new(input).unwrap();
                unsafe { mpclipboard_handle_send(handle, input.as_ptr().cast()) };
            }
            Err(err) => {
                log::error!("Error reading from console: {}", err);
                break;
            }
        }
    }

    unsafe { mpclipboard_handle_stop(handle) };

    Ok(())
}

struct SyncHandle(usize);
impl SyncHandle {
    fn new(handle: *mut Handle) -> Self {
        Self(handle as usize)
    }
    fn unwrap(self) -> *mut Handle {
        self.0 as *mut Handle
    }
}
