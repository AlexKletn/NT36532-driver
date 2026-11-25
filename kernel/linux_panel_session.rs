// SPDX-License-Identifier: GPL-2.0-only

use core::ptr::NonNull;

use kernel::bindings;

use crate::{
    Backlight, Display, DisplayState, PanelPower, SELECTED_PROFILE, display_panel::Nt36532,
};

use super::{
    linux_panel_io::LinuxPanelIo, linux_panel_links::LinuxPanelLinks,
    linux_panel_session_error::LinuxPanelSessionError,
};

pub(super) struct LinuxPanelSession<B> {
    links: LinuxPanelLinks,
    display: Display<Nt36532<'static, LinuxPanelIo>, B>,
}

impl<B> LinuxPanelSession<B>
where
    B: Backlight + PanelPower<Error = <B as Backlight>::Error>,
{
    /// # Safety
    ///
    /// The GPIO and regulator must remain live for this session and be
    /// dedicated to the panel. Serialize access to all panel resources.
    pub(super) unsafe fn new(
        links: LinuxPanelLinks,
        reset: NonNull<bindings::gpio_desc>,
        vddio: NonNull<bindings::regulator>,
        illumination: B,
    ) -> Result<Self, crate::display_panel::PanelError<kernel::error::Error>> {
        // SAFETY: The session retains the managed links and the caller owns the GPIO and regulator.
        let io = unsafe { LinuxPanelIo::new(*links.channels(), reset, vddio) };
        Ok(Self {
            links,
            display: Display::new(io.into_controller()?, illumination),
        })
    }

    pub(super) fn prepare(
        &mut self,
        refresh_hz: u16,
    ) -> Result<(), LinuxPanelSessionError<<B as Backlight>::Error>> {
        let mode = SELECTED_PROFILE
            .as_ref()
            .and_then(|profile| profile.display_mode(refresh_hz))
            .ok_or(LinuxPanelSessionError::InvalidMode)?;
        let pps = self
            .links
            .compression_pps()
            .map_err(LinuxPanelSessionError::Links)?;
        self.display
            .prepare(&mode.panel_mode(Some(&pps)))
            .map_err(LinuxPanelSessionError::Display)
    }

    pub(super) fn set_live_refresh(
        &mut self,
        refresh_hz: u16,
    ) -> Result<(), LinuxPanelSessionError<<B as Backlight>::Error>> {
        let mode = SELECTED_PROFILE
            .as_ref()
            .and_then(|profile| profile.display_mode(refresh_hz))
            .ok_or(LinuxPanelSessionError::InvalidMode)?;
        self.display
            .set_live_refresh(&mode.panel_mode(None))
            .map_err(LinuxPanelSessionError::Display)
    }

    pub(super) fn unprepare(
        &mut self,
    ) -> Result<(), LinuxPanelSessionError<<B as Backlight>::Error>> {
        self.display
            .unprepare()
            .map_err(LinuxPanelSessionError::Display)
    }

    pub(super) fn max_brightness(&self) -> u16 {
        self.display.max_brightness()
    }

    pub(super) fn set_brightness(
        &mut self,
        level: u16,
    ) -> Result<(), LinuxPanelSessionError<<B as Backlight>::Error>> {
        if self.display.state() != DisplayState::Prepared {
            return Ok(());
        }
        self.display
            .set_brightness(level)
            .map_err(LinuxPanelSessionError::Display)
    }
}
