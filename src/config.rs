use anyhow::{Context as _, Result};
use http::Uri;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[repr(C)]
#[derive(Clone, Copy)]
pub enum ConfigReadOption {
    FromLocalFile = 0,
    FromXdgConfigDir = 1,
}

impl ConfigReadOption {
    fn path(self) -> String {
        match self {
            ConfigReadOption::FromLocalFile => "config.toml".to_string(),
            ConfigReadOption::FromXdgConfigDir => {
                let home = std::env::var("HOME").expect("no $HOME");
                format!("{home}/.config/mpclipboard/config.toml")
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(with = "http_serde::uri")]
    pub uri: Uri,
    pub token: String,
    pub name: String,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("uri", &self.uri)
            .field("token", &"******")
            .field("name", &self.name)
            .finish()
    }
}

impl Config {
    pub fn read(option: ConfigReadOption) -> Result<Self> {
        let path = option.path();
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("failed to read {path}"))?;
        toml::from_str(&content).context("invalid config format")
    }

    pub(crate) fn from_ptr(ptr: *mut Config) -> Option<&'static Config> {
        unsafe { ptr.as_ref() }
    }
}

macro_rules! value_or_return_null {
    ($value:expr) => {
        match $value {
            Ok(value) => value,
            Err(err) => {
                log::error!("{err:?}");
                return std::ptr::null_mut();
            }
        }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_config_read(option: ConfigReadOption) -> *mut Config {
    let config = value_or_return_null!(Config::read(option));
    Box::leak(Box::new(config))
}

#[unsafe(no_mangle)]
pub extern "C" fn mpclipboard_config_new(
    uri: *const u8,
    token: *const u8,
    name: *const u8,
) -> *mut Config {
    let uri = value_or_return_null!(unsafe { std::ffi::CStr::from_ptr(uri.cast()) }.to_str());
    let token = value_or_return_null!(unsafe { std::ffi::CStr::from_ptr(token.cast()) }.to_str());
    let name = value_or_return_null!(unsafe { std::ffi::CStr::from_ptr(name.cast()) }.to_str());
    let uri = value_or_return_null!(Uri::from_str(uri));
    Box::leak(Box::new(Config {
        uri,
        token: token.to_string(),
        name: name.to_string(),
    }))
}
