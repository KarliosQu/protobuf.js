#![deny(clippy::all)]

extern crate napi_derive;

pub mod writer;
pub mod reader;
pub mod pool;

pub use writer::Writer;
pub use reader::Reader;
