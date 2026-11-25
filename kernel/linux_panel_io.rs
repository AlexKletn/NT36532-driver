// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::c_void,
    ptr::{NonNull, from_ref},
};

use kernel::{
    bindings,
    error::{Error, code::EINVAL, to_result},
    time::{Delta, delay::fsleep},
};
use panel_nt36532::{DsiMode, DsiPacket, PanelIo};

use crate::SELECTED_PROFILE;
use crate::display_panel::{Nt36532, PanelError};

use super::dsi_bindings::{
    DCS_LONG_WRITE, DCS_SHORT_WRITE, nt36532_dsi_send_pps, nt36532_dsi_write,
    nt36532_panel_gpio_set, nt36532_panel_set_vddio,
};

/// Borrowed resources of one bound Linux panel. Operations may sleep.
pub(super) struct LinuxPanelIo {
    dsi: [NonNull<c_void>; 2],
    reset: NonNull<bindings::gpio_desc>,
    vddio: NonNull<bindings::regulator>,
    vddio_enabled: bool,
}

impl LinuxPanelIo {
    /// # Safety
    ///
    /// The caller must keep the DSI devices attached and the GPIO and regulator
    /// acquired for the lifetime of the adapter, and serialize panel access.
    pub(super) unsafe fn new(
        dsi: [NonNull<c_void>; 2],
        reset: NonNull<bindings::gpio_desc>,
        vddio: NonNull<bindings::regulator>,
    ) -> Self {
        Self {
            dsi,
            reset,
            vddio,
            vddio_enabled: false,
        }
    }
}

impl LinuxPanelIo {
    pub(super) fn into_controller(self) -> Result<Nt36532<'static, Self>, PanelError<Error>> {
        let profile = SELECTED_PROFILE.as_ref().ok_or(PanelError::Dsi(EINVAL))?;
        Nt36532::new(self, profile.panel)
    }
}

impl PanelIo for LinuxPanelIo {
    type Error = Error;

    fn enable_vddio(&mut self) -> Result<(), Self::Error> {
        if self.vddio_enabled {
            return Ok(());
        }
        // SAFETY: The owner holds the regulator for the lifetime of the adapter.
        to_result(unsafe { nt36532_panel_set_vddio(self.vddio.as_ptr(), true) })?;
        self.vddio_enabled = true;
        Ok(())
    }

    fn disable_vddio(&mut self) -> Result<(), Self::Error> {
        if !self.vddio_enabled {
            return Ok(());
        }
        // SAFETY: The owner holds the regulator for the lifetime of the adapter.
        to_result(unsafe { nt36532_panel_set_vddio(self.vddio.as_ptr(), false) })?;
        self.vddio_enabled = false;
        Ok(())
    }

    fn set_reset(&mut self, asserted: bool) -> Result<(), Self::Error> {
        // SAFETY: The GPIO descriptor remains live and the owner configured it as an output.
        to_result(unsafe { nt36532_panel_gpio_set(self.reset.as_ptr(), asserted) })
    }

    fn delay_ms(&mut self, milliseconds: u16) {
        fsleep(Delta::from_millis(milliseconds.into()));
    }

    fn write_dsi(&mut self, packet: DsiPacket<'_>, mode: DsiMode) -> Result<(), Self::Error> {
        let (packet_type, payload) = match &packet {
            DsiPacket::DcsShort(opcode) => (DCS_SHORT_WRITE, core::slice::from_ref(opcode)),
            DsiPacket::DcsLong(payload) => (DCS_LONG_WRITE, *payload),
        };
        for channel in self.dsi {
            // SAFETY: The channel is attached and the payload lives through the synchronous transfer.
            to_result(unsafe {
                nt36532_dsi_write(
                    channel.as_ptr(),
                    packet_type,
                    mode == DsiMode::LowPower,
                    payload.as_ptr(),
                    payload.len(),
                )
            })?;
        }
        Ok(())
    }

    fn send_dsc_pps(&mut self, pps: &[u8; 128]) -> Result<(), Self::Error> {
        for channel in self.dsi {
            // SAFETY: The channel is attached and the shim reads exactly 128 bytes from pps.
            to_result(unsafe { nt36532_dsi_send_pps(channel.as_ptr(), from_ref(pps).cast()) })?;
        }
        Ok(())
    }
}
