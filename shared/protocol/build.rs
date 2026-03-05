//! Build script for generating Rust code from Protocol Buffer definitions.

use std::io::Result;

fn main() -> Result<()> {
    // Tell Cargo to rerun this if proto files change
    println!("cargo:rerun-if-changed=proto/");

    // Configure prost-build
    let mut config = prost_build::Config::new();

    // Note: Serde derives are disabled because prost_types::Timestamp and Any
    // don't implement Serialize/Deserialize. If JSON serialization is needed,
    // use prost-wkt-types or implement custom serialization.

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
