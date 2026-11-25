#![no_std]

#[cfg(all(feature = "std", unix))]
extern crate std;

mod controller;
mod firmware;
mod profile;
mod report;
mod spi;

pub use controller::{
    InputError, InterruptAction, ResetHistory, ResetLine, TouchController, TouchRuntime,
};
pub use firmware::{
    BootCrcStatus, BootError, BootIo, Bootloader, FirmwareError, FirmwareImage, Partition,
};
#[cfg(all(feature = "std", unix))]
pub use firmware::{read_firmware_fd, read_firmware_path};
pub use profile::{ContactFormat, FirmwareProfile, TOUCH_PROFILES, TouchStrategy};
pub use report::{Contact, InputReport, InputSink, InputState, Pen, ReportError};
pub use spi::{PagedSpi, SpiBus, SpiError};
