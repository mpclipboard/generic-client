use anyhow::Result;
use mpclipboard_generic_client::{
    ConfigReadOption, Handle, Output, mpclipboard_config_read, mpclipboard_handle_poll,
    mpclipboard_handle_send, mpclipboard_handle_stop, mpclipboard_init, mpclipboard_thread_start,
};
use std::io::BufRead as _;

fn main() -> Result<()> {
    mpclipboard_init();

    let config = mpclipboard_config_read(ConfigReadOption::FromLocalFile);
    let handle = unsafe { mpclipboard_thread_start(config) };

    let sync_handle = SyncHandle::new(handle);

    std::thread::spawn(move || {
        let handle = sync_handle.unwrap();
        loop {
            let Output { text, connectivity } = unsafe { mpclipboard_handle_poll(handle) };
            if !text.is_null() {
                log::info!(
                    "text = {:?}",
                    unsafe { std::ffi::CStr::from_ptr(text) }.to_str()
                );
                unsafe { free(text.cast()) }
            };
            if !connectivity.is_null() {
                log::info!("connectivity = {:?}", unsafe { *connectivity });
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

unsafe extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}
