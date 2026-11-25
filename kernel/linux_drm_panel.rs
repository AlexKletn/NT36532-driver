// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_int, c_void},
    ptr::NonNull,
};

use kernel::{
    bindings,
    error::{code::EINVAL, from_err_ptr},
    prelude::*,
};

use crate::{Backlight, PanelPower, SELECTED_PROFILE};

use super::linux_panel_session::LinuxPanelSession;

unsafe extern "C" {
    fn nt36532_drm_panel_register(
        device: *mut c_void,
        context: *mut c_void,
        prepare: unsafe extern "C" fn(*mut c_void, u16) -> c_int,
        unprepare: unsafe extern "C" fn(*mut c_void) -> c_int,
        set_brightness: unsafe extern "C" fn(*mut c_void, u16) -> c_int,
        max_brightness: u16,
        bits_per_component: u8,
        set_live_refresh: unsafe extern "C" fn(*mut c_void, u16) -> c_int,
    ) -> *mut c_void;
    fn nt36532_drm_panel_unregister(panel: *mut c_void);
}

pub(super) struct LinuxDrmPanel<B> {
    panel: NonNull<c_void>,
    _session: KBox<LinuxPanelSession<B>>,
}

impl<B> LinuxDrmPanel<B>
where
    B: Backlight + PanelPower<Error = <B as Backlight>::Error>,
    <B as Backlight>::Error: core::fmt::Debug,
{
    /// # Safety
    ///
    /// This handle must be dropped in a sleepable context before the panel
    /// device releases its managed resources.
    pub(super) unsafe fn register(
        device: NonNull<bindings::device>,
        session: LinuxPanelSession<B>,
    ) -> Result<Self> {
        let link = SELECTED_PROFILE
            .as_ref()
            .and_then(|profile| profile.link.as_ref())
            .ok_or(EINVAL)?;
        let mut session = KBox::new(session, GFP_KERNEL)?;
        let max_brightness = session.max_brightness();
        let context = NonNull::from(&mut *session).cast::<c_void>();
        // SAFETY: The session has a stable address and the shim serializes callbacks.
        let panel = from_err_ptr(unsafe {
            nt36532_drm_panel_register(
                device.as_ptr().cast(),
                context.as_ptr(),
                Self::prepare,
                Self::unprepare,
                Self::set_brightness,
                max_brightness,
                link.bits_per_component,
                Self::set_live_refresh,
            )
        })?;
        Ok(Self {
            panel: NonNull::new(panel).ok_or(EINVAL)?,
            _session: session,
        })
    }

    unsafe extern "C" fn prepare(context: *mut c_void, refresh_hz: u16) -> c_int {
        let Some(profile) = &SELECTED_PROFILE else {
            return EINVAL.to_errno();
        };
        // SAFETY: The shim serializes callbacks and the session is still live.
        let session = unsafe { &mut *context.cast::<LinuxPanelSession<B>>() };
        let refresh_hz = profile
            .display_mode(refresh_hz)
            .map_or(profile.preferred_refresh_hz, |mode| mode.refresh_hz);
        match session.prepare(refresh_hz) {
            Ok(()) => 0,
            Err(error) => {
                report_error("prepare", error);
                EIO.to_errno()
            }
        }
    }

    unsafe extern "C" fn unprepare(context: *mut c_void) -> c_int {
        // SAFETY: The shim serializes callbacks and the session is still live.
        let session = unsafe { &mut *context.cast::<LinuxPanelSession<B>>() };
        match session.unprepare() {
            Ok(()) => 0,
            Err(error) => {
                report_error("unprepare", error);
                EIO.to_errno()
            }
        }
    }

    unsafe extern "C" fn set_live_refresh(context: *mut c_void, refresh_hz: u16) -> c_int {
        // SAFETY: The DRM shim serializes access to the live session.
        let session = unsafe { &mut *context.cast::<LinuxPanelSession<B>>() };
        match session.set_live_refresh(refresh_hz) {
            Ok(()) => 0,
            Err(error) => {
                report_error("live refresh", error);
                EIO.to_errno()
            }
        }
    }

    unsafe extern "C" fn set_brightness(context: *mut c_void, level: u16) -> c_int {
        // SAFETY: The shim serializes brightness and lifecycle callbacks.
        let session = unsafe { &mut *context.cast::<LinuxPanelSession<B>>() };
        match session.set_brightness(level) {
            Ok(()) => 0,
            Err(error) => {
                report_error("brightness", error);
                EIO.to_errno()
            }
        }
    }
}

impl<B> Drop for LinuxDrmPanel<B> {
    fn drop(&mut self) {
        // SAFETY: Unregister drains callbacks before the owned session drops.
        unsafe { nt36532_drm_panel_unregister(self.panel.as_ptr()) };
    }
}

fn report_error<B: core::fmt::Debug>(
    operation: &str,
    error: super::linux_panel_session_error::LinuxPanelSessionError<B>,
) {
    use super::linux_panel_session_error::LinuxPanelSessionError;

    match error {
        LinuxPanelSessionError::InvalidMode => pr_err!("NT36532 {}: invalid mode\n", operation),
        LinuxPanelSessionError::Links(error) => {
            pr_err!("NT36532 {}: DSI error {:?}\n", operation, error)
        }
        LinuxPanelSessionError::Display(error) => pr_err!("NT36532 {}: {:?}\n", operation, error),
    }
}
