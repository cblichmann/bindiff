use std::fs;
use std::ops::Deref;
use std::path::Path;

use anyhow::Result;
use lazy_static::lazy_static;

include!(concat!(env!("OUT_DIR"), "/security.bindiff.rs"));

pub const CONFIG_NAME: &str = "bindiff.json";

lazy_static! {
    /// Returns the default configuration.
    static ref DEFAULT_CONFIG: Config =
        serde_json::from_str(include_str!("../bindiff.json")).unwrap();
}

/// Returns the current application global configuration.
///
/// On first call, initializes from well-known locations in the filesystem, or
/// if no configuration is found, with default values.
pub fn proto() -> Config {
    DEFAULT_CONFIG.deref().clone()
}

/// Loads configuration from a JSON string.
pub fn load_from_json(data: &str) -> Result<Config> {
    serde_json::from_str(data).map_err(anyhow::Error::from)
}

pub fn load_from_file<P: AsRef<Path>>(filename: P) -> Result<Config> {
    load_from_json(&fs::read_to_string(filename)?)
}
