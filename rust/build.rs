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

use std::io::Result;

use git_version::git_version;

fn main() -> Result<()> {
    println!(
        "cargo::rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%b %e %Y")
    );
    println!("cargo::rustc-env=GIT_REV={}", git_version!());

    protobuf_codegen::Codegen::new()
        .pure()
        .include("src")
        .inputs(["src/bindiff_config.proto", "src/binexport2.proto"])
        .cargo_out_dir("bindiff")
        .run_from_script();
    Ok(())
}