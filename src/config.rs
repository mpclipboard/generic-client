use anyhow::{Context as _, Result};
use http::Uri;
use serde::{Deserialize, Serialize};
use std::{ffi::c_char, str::FromStr};

use crate::ffi::cstring_to_string;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// Instruction for the `Config::read` function how to read the config.
pub enum ConfigReadOption {
    /// Read from "./config.toml", based on your current working directory
    FromLocalFile = 0,

    /// Read from XDG Config dir (i.e. from `~/.config/mpclipboard/config.toml`)
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

/// Representation of a runtime configuration
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Config {
    /// URI of the WebSocket server
    /// (e.g. `"ws://127.0.0.1:3000"` or `"wss://mpclipboard.me.dev"`)
    #[serde(with = "http_serde::uri")]
    pub uri: Uri,

    /// Token that is used for authentication
    pub token: String,

    /// Unique name of the client
    /// (e.g. `"macos-old-laptop"` or `"linux-dusty-minipc"`)
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
    /// Reads the config based on the given instruction
    /// (which is either "read from XDG dir" or "read from ./config.toml")
    pub fn read(option: ConfigReadOption) -> Result<Self> {
        let path = option.path();
        let content =
            std::fs::read_to_string(&path).with_context(|| format!("failed to read {path}"))?;
        toml::from_str(&content).context("invalid config format")
    }
}

#[unsafe(no_mangle)]
/// Reads the config based on the given instruction
/// (which is either "read from XDG dir" or "read from ./config.toml")
pub extern "C" fn mpclipboard_config_read(option: ConfigReadOption) -> *mut Config {
    let config = match Config::read(option) {
        Ok(config) => config,
        Err(err) => {
            log::error!("{err:?}");
            return std::ptr::null_mut();
        }
    };
    Box::leak(Box::new(config))
}

#[unsafe(no_mangle)]
/// Constructs the config in-place based on given parameters that match fields 1-to-1.
pub extern "C" fn mpclipboard_config_new(
    uri: *const c_char,
    token: *const c_char,
    name: *const c_char,
) -> *mut Config {
    let Ok(uri) = cstring_to_string(uri) else {
        log::error!("invalid uri");
        return std::ptr::null_mut();
    };
    let Ok(uri) = Uri::from_str(&uri) else {
        log::error!("uri is invalid");
        return std::ptr::null_mut();
    };
    let Ok(token) = cstring_to_string(token) else {
        log::error!("invalid token");
        return std::ptr::null_mut();
    };
    let Ok(name) = cstring_to_string(name) else {
        log::error!("invalid name");
        return std::ptr::null_mut();
    };

    Box::leak(Box::new(Config {
        uri,
        token: token.to_string(),
        name: name.to_string(),
    }))
}
