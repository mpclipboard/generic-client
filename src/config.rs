use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub url: String,
    pub token: String,
    pub name: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("url", &self.url)
            .field("token", &"******")
            .field("name", &self.name)
            .finish()
    }
}

impl Config {
    pub fn read_from_xdg_config_dir() -> Result<Self> {
        let home = std::env::var("HOME").context("no $HOME")?;
        let path = format!("{home}/.config/shared-clipboard/config.toml");
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("failed to read {path}"))?;
        let config = toml::from_str(&content).context("invalid config format")?;
        Ok(config)
    }

    pub(crate) fn from_ptr(ptr: *mut Config) -> Self {
        unsafe { *Box::from_raw(ptr) }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn shared_clipboard_config_read_from_xdg_config_dir() -> *mut Config {
    let config = match Config::read_from_xdg_config_dir() {
        Ok(config) => config,
        Err(err) => {
            log::error!("{err:?}");
            return std::ptr::null_mut();
        }
    };
    Box::leak(Box::new(config))
}

#[unsafe(no_mangle)]
pub extern "C" fn shared_clipboard_config_new(
    url: *const u8,
    token: *const u8,
    name: *const u8,
) -> *mut Config {
    macro_rules! ptr_to_string_or_return_null {
        ($ptr:expr) => {
            match unsafe { std::ffi::CStr::from_ptr($ptr.cast()) }.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => {
                    log::error!("{} must be a NULL-temrinated string", stringify!($ptr));
                    return std::ptr::null_mut();
                }
            }
        };
    }
    let url = ptr_to_string_or_return_null!(url);
    let token = ptr_to_string_or_return_null!(token);
    let name = ptr_to_string_or_return_null!(name);
    Box::leak(Box::new(Config { url, token, name }))
}
