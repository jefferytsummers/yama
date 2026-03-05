//! Build script for generating Rust code from Protocol Buffer definitions.

use std::io::Result;

fn main() -> Result<()> {
    // Tell Cargo to rerun this if proto files change
    println!("cargo:rerun-if-changed=proto/");

    // Configure prost-build
    let mut config = prost_build::Config::new();

    // Add serde derives for JSON serialization
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");
    config.type_attribute(".", "#[serde(rename_all = \"camelCase\")]");

    // Generate code for all proto files
    config.compile_protos(
        &[
            "proto/common.proto",
            "proto/agent.proto",
            "proto/system.proto",
        ],
        &["proto/"],
    )?;

    Ok(())
}
