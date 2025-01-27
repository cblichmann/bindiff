// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use lazy_static::lazy_static;
use protobuf::Message;
use protobuf_json_mapping::parse_from_str;
use protobuf_json_mapping::print_to_string_with_options;
use protobuf_json_mapping::PrintOptions;

include!(concat!(env!("OUT_DIR"), "/bindiff/mod.rs"));
use bindiff_config::config::MatchingStep;
use bindiff_config::Config;

use crate::util;
use crate::version::BINDIFF_NAME;
use crate::version::CONFIG_NAME;

lazy_static! {
    /// Returns the default configuration.
    static ref DEFAULT_CONFIG: Config =
        parse_from_str(include_str!("../bindiff.json")).unwrap();
}

/// Returns the current application global configuration.
///
/// On first call, initializes from well-known locations in the filesystem, or
/// if no configuration is found, with default values.
pub fn proto() -> Config {
    lazy_static! {
        static ref CONFIG: Config = {
            let mut config = DEFAULT_CONFIG.clone();

            if let Ok(common_path) =
                util::get_common_appdata_directory(BINDIFF_NAME)
            {
                if let Ok(common_config) =
                    load_from_file(common_path.join(CONFIG_NAME))
                {
                    merge_into(&common_config, &mut config).unwrap();
                }
            }

            if let Ok(user_path) =
                util::get_or_create_appdata_directory(BINDIFF_NAME)
            {
                if let Ok(user_config) =
                    load_from_file(user_path.join(CONFIG_NAME))
                {
                    merge_into(&user_config, &mut config).unwrap();
                }
            }
            config
        };
    };
    CONFIG.clone()
}

/// Loads configuration from a JSON string.
pub fn load_from_json(data: &str) -> Result<Config> {
    Ok(parse_from_str(data)?)
}

pub fn load_from_file<P: AsRef<Path>>(filename: P) -> Result<Config> {
    load_from_json(&fs::read_to_string(filename)?)
}

/// Serializes the specified configuration to JSON.
pub fn as_json_string(config: &Config) -> Result<String> {
    let print_options = PrintOptions {
        proto_field_name: true,
        always_output_default_values: true,
        ..Default::default()
    };
    let json = serde_json::from_str::<serde_json::Value>(
        &print_to_string_with_options(config, &print_options)?,
    )?;
    // Use serde to re-format, as protobuf_json can't be configured. This relies
    // on the preserve_order feature of serde_json to keep the order of the
    // emitted fields stable.
    Ok(serde_json::to_string_pretty(&json)?)
}

/// Saves configuration to the per-user configuration directory.
pub fn save_user_config(config: &Config) -> Result<()> {
    let filename =
        util::get_or_create_appdata_directory(BINDIFF_NAME)?.join(CONFIG_NAME);
    let data = as_json_string(config)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(filename)?;
    file.write(data.as_bytes())?;

    Ok(())
}

/// Merges configuration settings. Similar to protobuf's merge_from(), but
/// intelligently merges the matching algorithms, where order and uniqueness
/// matter.
pub fn merge_into(from: &Config, config: &mut Config) -> Result<()> {
    // Keep this code in sync with the implementation in `Config.java`.

    // Move away problematic fields
    let function_matching = config.function_matching.clone();
    config.function_matching.clear();
    let basic_block_matching: Vec<MatchingStep> =
        config.basic_block_matching.clone();
    config.basic_block_matching.clear();

    // Let protobuf handle the actual merge
    let config_bytes = from.write_to_bytes()?;
    config.merge_from(&mut protobuf::CodedInputStream::from_bytes(
        config_bytes.as_slice(),
    ))?;

    let names = HashSet::<&str>::from_iter(
        config.function_matching.iter().map(|s| s.name.as_str()),
    );
    if names.is_empty() || names.len() != config.function_matching.len() {
        // Duplicate or no algorithms, restore original list
        config.function_matching = function_matching;
    }

    let names = HashSet::<&str>::from_iter(
        config.basic_block_matching.iter().map(|s| s.name.as_str()),
    );
    if names.is_empty() || names.len() != config.basic_block_matching.len() {
        config.basic_block_matching = basic_block_matching;
    }

    Ok(())
}
