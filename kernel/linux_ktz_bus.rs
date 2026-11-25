// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_char, c_int, c_void},
    ptr::NonNull,
};

use kernel::{
    bindings,
    error::{Error, code::EINVAL, from_err_ptr, to_result},
    prelude::CStr,
};
use ktz8866_driver::RegisterBus;

unsafe extern "C" {
    fn nt36532_ktz_i2c_get(device: *mut c_void, property: *const c_char) -> *mut c_void;
    fn nt36532_ktz_i2c_write(client: *mut c_void, register: u8, value: u8) -> c_int;
}

pub(super) struct LinuxKtzBus(NonNull<c_void>);

impl LinuxKtzBus {
    /// # Safety
    ///
    /// The panel device must remain bound while the bus is used. The I²C
    /// client must be dedicated to this adapter and access must be serialized.
    pub(super) unsafe fn acquire(
        device: NonNull<bindings::device>,
        property: &CStr,
    ) -> Result<Self, Error> {
        // SAFETY: The caller supplies a live device. The shim retains the client.
        let client = from_err_ptr(unsafe {
            nt36532_ktz_i2c_get(device.as_ptr().cast(), property.as_ptr().cast())
        })?;
        Ok(Self(NonNull::new(client).ok_or(EINVAL)?))
    }
}

impl RegisterBus for LinuxKtzBus {
    type Error = Error;

    fn write(&mut self, register: u8, value: u8) -> Result<(), Error> {
        // SAFETY: The managed reference keeps the client live during transfer.
        to_result(unsafe { nt36532_ktz_i2c_write(self.0.as_ptr(), register, value) })
    }
}
