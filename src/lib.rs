#![allow(static_mut_refs)]

pub use crate::config::{
    Config, mpclipboard_config_new, mpclipboard_config_read_from_xdg_config_dir,
};
use crate::{event::Event, incoming_tx::IncomingTx, outcoming_rx::OutcomingRx, thread::Thread};
use anyhow::{Context, Result};
use mpclipboard_common::Clip;
use tokio::sync::mpsc::channel;

mod config;
mod event;
mod incoming_tx;
mod main_loop;
mod outcoming_rx;
mod runtime;
mod thread;
mod websocket;

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_setup() {
    #[cfg(target_os = "android")]
    {
        use android_logger::Config;
        use log::LevelFilter;

        android_logger::init_once(
            Config::default()
                .with_tag("RUST")
                .with_max_level(LevelFilter::Trace),
        );
    }

    #[cfg(not(target_os = "android"))]
    pretty_env_logger::init();

    log::info!("info example");
    log::error!("error example");

    if let Err(err) = crate::websocket::init_tls_connector() {
        log::error!("failed to init WS connector");
        log::error!("{err:?}");
        std::process::exit(1);
    }
    log::info!("TLS Connector has been configured");
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_setup_rustls_on_jvm(
    env: *mut jni::sys::JNIEnv,
    context: jni::sys::jobject,
) {
    let mut env = match unsafe { jni::JNIEnv::from_raw(env) } {
        Ok(env) => env,
        Err(err) => {
            log::error!("JNIEnv::from_raw failed: {:?}", err);
            return;
        }
    };
    let context = unsafe { jni::objects::JObject::from_raw(context) };

    if let Err(err) = rustls_platform_verifier::android::init_hosted(&mut env, context) {
        log::error!("Failed to instantiate rustls_platform_verifier: {err:?}");
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_start_thread(config: *mut Config) {
    let config = Config::from_ptr(config);

    let (incoming_tx, incoming_rx) = channel::<Clip>(256);
    let (outcoming_tx, outcoming_rx) = channel::<Event>(256);

    Thread::start(incoming_rx, outcoming_tx, config);

    IncomingTx::set(incoming_tx);
    OutcomingRx::set(outcoming_rx);
}
#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_stop_thread() -> bool {
    Thread::stop()
        .inspect_err(|err| log::error!("{err:?}"))
        .is_ok()
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_send(text: *const u8) {
    fn send(text: *const u8) -> Result<()> {
        let text = unsafe { std::ffi::CStr::from_ptr(text.cast()) }
            .to_str()
            .context("text passed to mpclipboard_clip_new must be NULL-terminated")?;
        let clip = Clip::new(text);
        IncomingTx::blocking_send(clip)?;
        Ok(())
    }

    if let Err(err) = send(text) {
        log::error!("{err:?}");
    }
}

#[repr(C)]
pub struct Output {
    pub text: *mut u8,
    pub connectivity: *mut bool,
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_poll() -> Output {
    fn poll() -> Result<Output> {
        let (clip, connectivity) = OutcomingRx::recv_squashed()?;

        let text = clip
            .map(|clip| clip.text)
            .map(string_to_bytes)
            .unwrap_or(std::ptr::null_mut());
        let connectivity = connectivity
            .map(Box::new)
            .map(|c| Box::leak(c) as *mut _)
            .unwrap_or(std::ptr::null_mut());

        Ok(Output { text, connectivity })
    }

    poll()
        .inspect_err(|err| log::error!("{err:?}"))
        .unwrap_or(Output {
            text: std::ptr::null_mut(),
            connectivity: std::ptr::null_mut(),
        })
}

fn string_to_bytes(s: String) -> *mut u8 {
    match std::ffi::CString::new(s) {
        Ok(text) => {
            let mut bytes = text.into_bytes_with_nul();
            let ptr = bytes.as_mut_ptr();
            std::mem::forget(bytes);
            ptr
        }
        Err(_) => {
            log::error!("clip text is NULL terminated");
            std::ptr::null_mut()
        }
    }
}
