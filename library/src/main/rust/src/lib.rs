#![deny(clippy::all)]

extern crate napi_derive;

pub mod writer;
pub mod reader;
pub mod native_message;
pub mod native_type;
pub mod managed_message;
pub mod fast_encoder;
pub mod pool;

pub use writer::Writer;
pub use native_type::NativeType;
pub use reader::Reader;
pub use native_message::NativeMessage;
pub use managed_message::ManagedMessage;
