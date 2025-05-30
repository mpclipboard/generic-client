use anyhow::{Result, bail};
use shared_clipboard_client_generic::{
    Output, shared_clipboard_config_read_from_xdg_cofig_dir, shared_clipboard_poll,
    shared_clipboard_send, shared_clipboard_setup, shared_clipboard_start_thread,
    shared_clipboard_stop_thread,
};
use std::io::BufRead as _;

fn main() -> Result<()> {
    shared_clipboard_setup();

    let config = shared_clipboard_config_read_from_xdg_cofig_dir();
    if config.is_null() {
        bail!("config is NULL");
    }
    shared_clipboard_start_thread(config);

    std::thread::spawn(|| {
        loop {
            let Output { text, connectivity } = shared_clipboard_poll();
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
                shared_clipboard_send(input.as_ptr().cast());
            }
            Err(err) => {
                log::error!("Error reading from console: {}", err);
                break;
            }
        }
    }

    shared_clipboard_stop_thread();

    Ok(())
}
