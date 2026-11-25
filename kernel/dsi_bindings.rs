// SPDX-License-Identifier: GPL-2.0-only

use core::ffi::{c_int, c_void};
use kernel::bindings;

pub(super) const DCS_SHORT_WRITE: u8 = 0x05;
pub(super) const DCS_LONG_WRITE: u8 = 0x39;

unsafe extern "C" {
    pub(super) fn nt36532_panel_gpio_set(reset: *mut bindings::gpio_desc, asserted: bool) -> c_int;
    pub(super) fn nt36532_panel_set_vddio(vddio: *mut bindings::regulator, enabled: bool) -> c_int;
    pub(super) fn nt36532_dsi_write(
        dsi: *mut c_void,
        packet_type: u8,
        low_power: bool,
        payload: *const u8,
        length: usize,
    ) -> c_int;

    pub(super) fn nt36532_dsi_send_pps(dsi: *mut c_void, payload: *const u8) -> c_int;
}
