#![no_std]

mod adapter;
mod adapter_error;
mod hardware;

pub use adapter::Ktz8866Adapter;
pub use adapter_error::AdapterError;
pub use hardware::Ktz8866Hardware;
