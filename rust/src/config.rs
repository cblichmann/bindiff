use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;
use serde::Deserializer;

pub const CONFIG_NAME: &str = "bindiff.json";

include!(concat!(env!("OUT_DIR"), "/security.bindiff.rs"));

use config::log_options::LogLevel;
impl LogLevel {
    pub fn from_str<'de, D>(deserializer: D) -> Result<i32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Self::from_str_name(s)
            .map(|v| v as i32)
            .ok_or(serde::de::Error::unknown_variant(s, &["values"]))
    }
}

use config::ui_preferences::graph_layout_options::GraphLayout;
impl GraphLayout {
    pub fn from_str<'de, D>(deserializer: D) -> Result<i32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        Self::from_str_name(s)
            .map(|v| v as i32)
            .ok_or(serde::de::Error::unknown_variant(s, &["values"]))
    }
}

/// Returns the current application global configuration.
///
/// On first call, initializes from well-known locations in the filesystem, or
/// if no configuration is found, with default values.
pub fn proto() -> Config {
    defaults()
}

/// Returns the default configuration.
pub fn defaults() -> Config {
    let l = config::log_options::LogLevel::from_str_name("INFO").unwrap();

    let s = include_str!("../bindiff.json");
    let c: Config = serde_json::from_str(s).unwrap();
    c
}

/// Loads configuration from a JSON string.
pub fn load_from_json(data: &str) -> Result<Config> {
    Err(anyhow::anyhow!("not implemented!"))
}

pub fn load_from_file<P: AsRef<Path>>(filename: P) -> Result<Config> {
    load_from_json(&fs::read_to_string(filename)?)
}
