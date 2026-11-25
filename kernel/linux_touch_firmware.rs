// SPDX-License-Identifier: GPL-2.0-only

use core::{ffi::c_void, ptr::NonNull};

use kernel::{
    error::{Error, code::EINVAL},
    str::CStr,
};
use panel_touch::{
    BootError, BootIo, FirmwareError, FirmwareImage, InputSink, ResetLine, TouchRuntime,
    TouchStrategy,
};

use super::linux_firmware_blob::LinuxFirmwareBlob;

#[derive(Debug)]
pub(super) enum LinuxTouchStartError<E> {
    Request(Error),
    Image(FirmwareError),
    Boot(BootError<E>),
}

pub(super) struct LinuxTouchFirmware<'a> {
    device: NonNull<c_void>,
    name: &'a CStr,
    strategy: TouchStrategy,
}

impl<'a> LinuxTouchFirmware<'a> {
    /// # Safety
    ///
    /// The device must stay bound for the lifetime of this adapter.
    pub(super) unsafe fn new(
        device: NonNull<c_void>,
        name: &'a CStr,
        strategy: &CStr,
    ) -> Result<Self, Error> {
        let strategy = core::str::from_utf8(strategy.to_bytes())
            .map_err(|_| EINVAL)?
            .parse()
            .map_err(|_| EINVAL)?;
        Ok(Self {
            device,
            name,
            strategy,
        })
    }

    pub(super) fn start<I, R, S>(
        &self,
        runtime: &mut TouchRuntime<I, R, S>,
        pen_command: Option<(u8, u8)>,
    ) -> Result<Option<BootError<I::Error>>, LinuxTouchStartError<I::Error>>
    where
        I: BootIo,
        R: ResetLine<Error = I::Error>,
        S: InputSink,
    {
        // SAFETY: The adapter retains a live device and a NUL-terminated name.
        let firmware =
            unsafe { LinuxFirmwareBlob::request(self.name.as_ptr().cast(), self.device) }
                .map_err(LinuxTouchStartError::Request)?;
        let image = FirmwareImage::parse_with_strategy(firmware.data(), self.strategy)
            .map_err(LinuxTouchStartError::Image)?;
        runtime
            .start(&image, pen_command)
            .map_err(LinuxTouchStartError::Boot)
    }
}
