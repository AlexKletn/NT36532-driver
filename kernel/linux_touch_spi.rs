// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_int, c_void},
    marker::PhantomData,
    ptr::{NonNull, null_mut},
};

use kernel::{
    error::{Error, code::EINVAL, to_result},
    time::{Delta, delay::fsleep},
};
use panel_touch::SpiBus;

unsafe extern "C" {
    fn nt36532_spi_transfer(device: *mut c_void, tx: *const u8, rx: *mut u8, len: usize) -> c_int;
}

pub(super) struct LinuxTouchSpi<'a> {
    device: NonNull<c_void>,
    lifetime: PhantomData<&'a mut c_void>,
}

impl LinuxTouchSpi<'_> {
    /// # Safety
    ///
    /// The device must point to a live Linux spi_device for the adapter's
    /// lifetime. The caller must serialize access to the device.
    pub(super) unsafe fn new(device: NonNull<c_void>) -> Self {
        Self {
            device,
            lifetime: PhantomData,
        }
    }
}

impl SpiBus for LinuxTouchSpi<'_> {
    type Error = Error;

    fn write(&mut self, tx: &[u8]) -> Result<(), Error> {
        // SAFETY: The device is live and the shim reads tx synchronously.
        to_result(unsafe {
            nt36532_spi_transfer(self.device.as_ptr(), tx.as_ptr(), null_mut(), tx.len())
        })
    }

    fn exchange(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), Error> {
        if tx.len() != rx.len() {
            return Err(EINVAL);
        }
        // SAFETY: Both slices cover len bytes and the transfer is synchronous.
        to_result(unsafe {
            nt36532_spi_transfer(self.device.as_ptr(), tx.as_ptr(), rx.as_mut_ptr(), tx.len())
        })
    }

    fn delay_ms(&mut self, milliseconds: u16) {
        fsleep(Delta::from_millis(i64::from(milliseconds)));
    }
}
