use anyhow::{Result, bail};
use mpclipboard_generic_client::{
    Output, mpclipboard_config_read_from_xdg_config_dir, mpclipboard_poll, mpclipboard_send,
    mpclipboard_setup, mpclipboard_start_thread, mpclipboard_stop_thread,
};
use std::io::BufRead as _;

fn main() -> Result<()> {
    mpclipboard_setup();

    let config = mpclipboard_config_read_from_xdg_config_dir();
    if config.is_null() {
        bail!("config is NULL");
    }
    mpclipboard_start_thread(config);

    std::thread::spawn(|| {
        loop {
            let Output { text, connectivity } = mpclipboard_poll();
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
                mpclipboard_send(input.as_ptr().cast());
            }
            Err(err) => {
                log::error!("Error reading from console: {}", err);
                break;
            }
        }
    }

    mpclipboard_stop_thread();

    Ok(())
}
