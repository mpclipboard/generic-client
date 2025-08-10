use anyhow::{Result, bail};
use mpclipboard_generic_client::{
    ConfigReadOption, Handle, Output, mpclipboard_config_read, mpclipboard_init, mpclipboard_poll,
    mpclipboard_send, mpclipboard_start_thread, mpclipboard_stop_thread,
};
use std::io::BufRead as _;

fn main() -> Result<()> {
    mpclipboard_init();

    let config = mpclipboard_config_read(ConfigReadOption::FromLocalFile);
    if config.is_null() {
        bail!("config is NULL");
    }
    let handle = mpclipboard_start_thread(config);

    let sync_handle = SyncHandle::new(handle);

    std::thread::spawn(move || {
        let handle = sync_handle.unwrap();
        loop {
            let Output { text, connectivity } = mpclipboard_poll(handle);
            if !text.is_null() {
                let text = unsafe { std::ffi::CString::from_raw(text.cast()) };
                let text = text.to_str().unwrap().to_string();
                log::info!("text = {text:?}");
            };
            if !connectivity.is_null() {
                let connectivity = unsafe { Some(*Box::from_raw(connectivity)) };
                log::info!("connectivity = {connectivity:?}");
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
                mpclipboard_send(handle, input.as_ptr().cast());
            }
            Err(err) => {
                log::error!("Error reading from console: {}", err);
                break;
            }
        }
    }

    mpclipboard_stop_thread(handle);

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
