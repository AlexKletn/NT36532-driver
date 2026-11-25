// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_int, c_void},
    ptr::{self, NonNull},
};

use kernel::error::{Error, code::EINVAL, from_err_ptr, to_result};

use crate::{PanelLinkConfig, SELECTED_PROFILE};

unsafe extern "C" {
    fn nt36532_panel_links_create(
        primary: *mut c_void,
        config: *const PanelLinkConfig,
        channels: *mut *mut c_void,
    ) -> *mut c_void;
    fn nt36532_panel_links_attach(links: *mut c_void) -> c_int;
    fn nt36532_panel_links_pack_pps(links: *mut c_void, pps: *mut u8) -> c_int;
}

#[derive(Clone, Copy)]
pub(super) struct LinuxPanelLinks {
    handle: NonNull<c_void>,
    channels: [NonNull<c_void>; 2],
}

impl LinuxPanelLinks {
    /// # Safety
    ///
    /// Call during probe with a live primary DSI device. The handle and channel
    /// pointers must stay within the lifetime of the device's managed resources.
    pub(super) unsafe fn create(primary: NonNull<c_void>) -> Result<Self, Error> {
        let config = SELECTED_PROFILE
            .as_ref()
            .and_then(|profile| profile.link.as_ref())
            .ok_or(EINVAL)?;
        let mut channels = [ptr::null_mut(); 2];
        // SAFETY: The primary is live and the output holds two channel pointers.
        let handle = from_err_ptr(unsafe {
            nt36532_panel_links_create(primary.as_ptr(), config, channels.as_mut_ptr())
        })?;
        let [primary, secondary] = channels;
        Ok(Self {
            handle: NonNull::new(handle).ok_or(EINVAL)?,
            channels: [
                NonNull::new(primary).ok_or(EINVAL)?,
                NonNull::new(secondary).ok_or(EINVAL)?,
            ],
        })
    }

    pub(super) fn attach(&self) -> Result<(), Error> {
        // SAFETY: The handle remains live through the managed device lifetime.
        to_result(unsafe { nt36532_panel_links_attach(self.handle.as_ptr()) })
    }

    pub(super) fn compression_pps(&self) -> Result<[u8; 128], Error> {
        let mut payload = [0; 128];
        // SAFETY: The managed links are live and the output holds one PPS.
        to_result(unsafe {
            nt36532_panel_links_pack_pps(self.handle.as_ptr(), payload.as_mut_ptr())
        })?;
        Ok(payload)
    }

    pub(super) fn channels(&self) -> &[NonNull<c_void>; 2] {
        &self.channels
    }
}
