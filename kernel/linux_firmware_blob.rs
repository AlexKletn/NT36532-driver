// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_char, c_void},
    ptr::{self, NonNull},
    slice,
};

use kernel::error::{Error, code::EINVAL, from_err_ptr};

unsafe extern "C" {
    fn nt36532_request_firmware(
        name: *const c_char,
        device: *mut c_void,
        data: *mut *const u8,
        size: *mut usize,
    ) -> *mut c_void;
    fn release_firmware(firmware: *const c_void);
}

pub(super) struct LinuxFirmwareBlob {
    handle: NonNull<c_void>,
    data: *const u8,
    size: usize,
}

impl LinuxFirmwareBlob {
    /// # Safety
    ///
    /// The name must be NUL-terminated and the device live for the request.
    pub(super) unsafe fn request(
        name: *const c_char,
        device: NonNull<c_void>,
    ) -> Result<Self, Error> {
        let mut data = ptr::null();
        let mut size = 0;
        // SAFETY: The caller supplies valid arguments and the out-pointers are writable.
        let handle = from_err_ptr(unsafe {
            nt36532_request_firmware(name, device.as_ptr(), &mut data, &mut size)
        })?;
        let blob = Self {
            handle: NonNull::new(handle).ok_or(EINVAL)?,
            data,
            size,
        };
        if size > isize::MAX as usize || (size != 0 && data.is_null()) {
            return Err(EINVAL);
        }
        Ok(blob)
    }

    pub(super) fn data(&self) -> &[u8] {
        if self.size == 0 {
            return &[];
        }
        // SAFETY: The firmware handle owns a buffer of this size until drop.
        unsafe { slice::from_raw_parts(self.data, self.size) }
    }
}

impl Drop for LinuxFirmwareBlob {
    fn drop(&mut self) {
        // SAFETY: The handle owns exactly one request_firmware reference.
        unsafe { release_firmware(self.handle.as_ptr()) };
    }
}
