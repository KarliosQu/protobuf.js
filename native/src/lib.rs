#![deny(clippy::all)]

use napi_derive::napi;

mod util;
mod types;

// Re-export utility functions
pub use util::*;

/// Initialize the native module
#[napi]
pub fn init() -> napi::Result<String> {
    Ok("protobufjs native module initialized".to_string())
}

/// Get the version of the native module
#[napi]
pub fn get_native_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
