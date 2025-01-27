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

#[cfg(all(
    target_family = "unix",
    not(target_os = "macos"),
    not(target_os = "ios")
))]
use std::env;
use std::path::PathBuf;

use anyhow::Result;
use app_dirs2::AppDataType::SharedData;
use app_dirs2::AppDataType::UserData;

/// Returns the platform-specific application data directory, which is a
/// per-user, writable path. If the directory does not exists, the function tries
/// to create it.
/// Returns one of these paths when called with "BinDiff":
/// | OS      | Typical value                                     |
/// |---------|---------------------------------------------------|
/// | Windows | C:\Users\<User>\AppData\Roaming\BinDiff           |
/// |         |  %AppData%\BinDiff                                |
/// | Linux   | /home/<User>/.bindiff                             |
/// | macOS   | /Users/<User>/Library/Application Support/BinDiff |
pub fn get_or_create_appdata_directory(product_name: &str) -> Result<PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let path = app_dirs2::get_data_root(UserData)?.join(product_name);
    #[cfg(all(
        target_family = "unix",
        not(target_os = "macos"),
        not(target_os = "ios")
    ))]
    let path = std::env::home_dir()?
        .as_path()
        .join(".".to_owned() + product_name.to_ascii_lowercase());
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

/// Returns the platform-specific per-machine application data directory. This is
/// usually a non-writable path.
/// Returns one of these paths when called with "BinDiff":
/// | OS      | Typical value                        |
/// |---------|--------------------------------------|
/// | Windows | C:\ProgramData\BinDiff               |
/// |         | %ProgramData%\BinDiff                |
/// | Linux   | /etc/opt/bindiff                     |
/// | macOS   | /Library/Application Support/BinDiff |
pub fn get_common_appdata_directory(product_name: &str) -> Result<PathBuf> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let path = app_dirs2::get_data_root(SharedData)?.join(product_name);
    #[cfg(all(
        target_family = "unix",
        not(target_os = "macos"),
        not(target_os = "ios")
    ))]
    let path = Path::new(env).join(product_name.to_ascii_lowercase());
    Ok(path)
}
