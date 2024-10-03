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

        // Until prost can serialize/deserialize enums correctly, let's get the
        // implementation from this crate's proc-macro.
        .type_attribute("LogLevel", "#[derive(bindiff::FromStrImpl)]")
        .type_attribute("GraphLayout", "#[derive(bindiff::FromStrImpl)]")
        .type_attribute("HierarchicalLayoutStyle", "#[derive(bindiff::FromStrImpl)]")
        .type_attribute("OrthogonalLayoutStyle", "#[derive(bindiff::FromStrImpl)]")
        .type_attribute("CircularLayoutStyle", "#[derive(bindiff::FromStrImpl)]")
        .type_attribute("LayoutOrientation", "#[derive(bindiff::FromStrImpl)]")
        .type_attribute("MouseWheelAction", "#[derive(bindiff::FromStrImpl)]")

        // Ignore missing fields when deserializing
        // TODO(cblichmann): Find a way to configure Serde to do this by default
        .field_attribute("LogOptions.level", "#[serde(default)]")
        .field_attribute("LogOptions.to_file", "#[serde(default)]")
        .field_attribute("LogOptions.directory", "#[serde(default)]")
        .field_attribute("LogOptions.to_stderr", "#[serde(default)]")

        // Annotate fields with the enum type to deserialize from
        .field_attribute(
            "LogOptions.level",
            "#[serde(deserialize_with = \"crate::config::config::log_options::LogLevel::from_str\")]",
        )
        .field_attribute(
            "GraphLayoutOptions.layout",
            "#[serde(deserialize_with = \"crate::config::config::ui_preferences::graph_layout_options::GraphLayout::from_str\")]",
        )
        .field_attribute(
            "GraphLayoutOptions.wheel_action",
            "#[serde(deserialize_with = \"crate::config::config::ui_preferences::graph_layout_options::MouseWheelAction::from_str\")]",
        )
        .field_attribute(
            "HierarchicalLayoutOptions.style",
            "#[serde(deserialize_with = \"crate::config::config::ui_preferences::graph_layout_options::hierarchical_layout_options::HierarchicalLayoutStyle::from_str\")]",
        )
        .field_attribute(
            "HierarchicalLayoutOptions.orientation",
            "#[serde(deserialize_with = \"crate::config::config::ui_preferences::graph_layout_options::LayoutOrientation::from_str\")]",
        )
        .field_attribute(
            "OrthogonalLayoutOptions.style",
            "#[serde(deserialize_with = \"crate::config::config::ui_preferences::graph_layout_options::orthogonal_layout_options::OrthogonalLayoutStyle::from_str\")]",
        )
        .field_attribute(
            "OrthogonalLayoutOptions.orientation",
            "#[serde(deserialize_with = \"crate::config::config::ui_preferences::graph_layout_options::LayoutOrientation::from_str\")]",
        )
        .field_attribute(
            "CircularLayoutOptions.style",
            "#[serde(deserialize_with = \"crate::config::config::ui_preferences::graph_layout_options::circular_layout_options::CircularLayoutStyle::from_str\")]",
        )

        .compile_protos(&["bindiff_config.proto"], &["."])?;

    Ok(())
}
