use std::io::Result;

use git_version::git_version;

extern crate prost_build;

fn main() -> Result<()> {
    println!(
        "cargo::rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%b %e %Y")
    );
    println!("cargo::rustc-env=GIT_REV={}", git_version!());

    let mut prost_build = prost_build::Config::new();
    prost_build
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .field_attribute("LogOptions.level", "#[serde(default)]")
        .field_attribute("LogOptions.to_file", "#[serde(default)]")
        .field_attribute("LogOptions.directory", "#[serde(default)]")
        .field_attribute("LogOptions.to_stderr", "#[serde(default)]")
        .field_attribute(
            "LogOptions.level",
            "#[serde(deserialize_with = \"log_options::LogLevel::from_str\")]",
        )
        .field_attribute(
            "GraphLayoutOptions.layout",
            "#[serde(deserialize_with = \"graph_layout_options::GraphLayout::from_str\")]",
        )
        
        .compile_protos(&["bindiff_config.proto"], &["."])?;

    Ok(())
}
