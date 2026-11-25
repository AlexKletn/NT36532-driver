// SPDX-License-Identifier: GPL-2.0-only

use core::{ffi::c_int, marker::PhantomData, ptr::NonNull};

use kernel::{
    bindings,
    error::{Error, to_result},
};
use panel_touch::{ResetLine, SpiError};

unsafe extern "C" {
    fn nt36532_touch_set_reset(gpio: *mut bindings::gpio_desc, asserted: bool) -> c_int;
}

pub(super) struct LinuxTouchReset<'a> {
    gpio: NonNull<bindings::gpio_desc>,
    lifetime: PhantomData<&'a mut bindings::gpio_desc>,
}

impl LinuxTouchReset<'_> {
    /// # Safety
    ///
    /// The GPIO must be acquired as an output and remain live for the adapter's
    /// lifetime. The caller must serialize access and use a sleepable context.
    pub(super) unsafe fn new(gpio: NonNull<bindings::gpio_desc>) -> Self {
        Self {
            gpio,
            lifetime: PhantomData,
        }
    }
}

impl ResetLine for LinuxTouchReset<'_> {
    type Error = SpiError<Error>;

    fn set_asserted(&mut self, asserted: bool) -> Result<(), Self::Error> {
        // SAFETY: The adapter's caller owns the live output GPIO.
        to_result(unsafe { nt36532_touch_set_reset(self.gpio.as_ptr(), asserted) })
            .map_err(SpiError::Transport)
    }
}
