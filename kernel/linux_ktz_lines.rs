// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_char, c_void},
    ptr::NonNull,
};

use kernel::{
    bindings,
    error::{Error, code::EINVAL, from_err_ptr, to_result},
    prelude::CStr,
};

use super::dsi_bindings::nt36532_panel_gpio_set;

unsafe extern "C" {
    fn nt36532_ktz_gpio_get(device: *mut c_void, name: *const c_char) -> *mut bindings::gpio_desc;
}

pub(super) struct LinuxKtzLines {
    hwen: NonNull<bindings::gpio_desc>,
    enp: NonNull<bindings::gpio_desc>,
    enn: NonNull<bindings::gpio_desc>,
}

impl LinuxKtzLines {
    /// # Safety
    ///
    /// The panel device must keep its managed resources live while the lines
    /// are used. The caller must serialize GPIO access.
    pub(super) unsafe fn acquire(device: NonNull<bindings::device>) -> Result<Self, Error> {
        let get = |name: &CStr| {
            // SAFETY: The device owns the requested descriptor and name is NUL-terminated.
            let line = from_err_ptr(unsafe {
                nt36532_ktz_gpio_get(device.as_ptr().cast(), name.as_ptr().cast())
            })?;
            NonNull::new(line).ok_or(EINVAL)
        };
        Ok(Self {
            hwen: get(c"hwen")?,
            enp: get(c"enp")?,
            enn: get(c"enn")?,
        })
    }

    pub(super) fn enable(&mut self) -> Result<(), Error> {
        set(self.hwen, true)?;
        set(self.enp, true)?;
        set(self.enn, true)
    }

    pub(super) fn disable_bias(&mut self) -> Result<(), Error> {
        let negative = set(self.enn, false);
        let positive = set(self.enp, false);
        negative.and(positive)
    }
}

fn set(line: NonNull<bindings::gpio_desc>, enabled: bool) -> Result<(), Error> {
    // SAFETY: The owning device retains this output GPIO.
    to_result(unsafe { nt36532_panel_gpio_set(line.as_ptr(), enabled) })
}
