#![no_std]

mod adapter;
mod adapter_error;
mod hardware;
mod pair;

pub use adapter::Ktz8866Adapter;
pub use adapter_error::AdapterError;
pub use hardware::Ktz8866Hardware;
pub use pair::{Ktz8866Pair, PairConfig, PairError};
