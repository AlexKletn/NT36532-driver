// SPDX-License-Identifier: GPL-2.0-only

use core::{ffi::c_void, marker::PhantomData, ptr::NonNull};

use kernel::{
    bindings,
    error::{Error, code::EINVAL, from_err_ptr},
};

unsafe extern "C" {
    fn nt36532_touch_panel_register(
        device: *mut c_void,
        context: *mut c_void,
        start: unsafe extern "C" fn(*mut c_void, *mut c_void),
        stop: unsafe extern "C" fn(*mut c_void),
    ) -> *mut c_void;

    fn nt36532_touch_panel_request_reload(follower: *mut c_void) -> bool;
    fn nt36532_touch_panel_unregister(follower: *mut c_void);
}

pub(super) struct LinuxTouchPanel<'a> {
    follower: NonNull<c_void>,
    lifetime: PhantomData<&'a bindings::device>,
}

impl LinuxTouchPanel<'_> {
    /// # Safety
    ///
    /// The caller must keep the device and callback context live until this
    /// handle is dropped. Startup may run before registration returns. Call
    /// register and drop in a sleepable context without holding callback locks.
    /// The callbacks must not reenter the DRM panel or follower API.
    pub(super) unsafe fn register(
        device: NonNull<bindings::device>,
        context: NonNull<c_void>,
        start: unsafe extern "C" fn(*mut c_void, *mut c_void),
        stop: unsafe extern "C" fn(*mut c_void),
    ) -> Result<Self, Error> {
        // SAFETY: The caller supplies a live device and callback context.
        let follower = from_err_ptr(unsafe {
            nt36532_touch_panel_register(device.as_ptr().cast(), context.as_ptr(), start, stop)
        })?;
        Ok(Self {
            follower: NonNull::new(follower).ok_or(EINVAL)?,
            lifetime: PhantomData,
        })
    }

    pub(super) fn queue_start(&self) {
        // SAFETY: This handle owns the registered follower.
        unsafe { Self::request_reload(self.follower) };
    }

    /// # Safety
    ///
    /// The follower must remain live throughout the call.
    pub(super) unsafe fn request_reload(follower: NonNull<c_void>) {
        // SAFETY: The caller keeps the follower alive.
        unsafe { nt36532_touch_panel_request_reload(follower.as_ptr()) };
    }
}

impl Drop for LinuxTouchPanel<'_> {
    fn drop(&mut self) {
        // SAFETY: This handle owns the live follower and its callbacks can finish.
        unsafe { nt36532_touch_panel_unregister(self.follower.as_ptr()) };
    }
}
