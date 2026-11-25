// SPDX-License-Identifier: GPL-2.0-only

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PanelLinkConfig {
    pub slice_width: u16,
    pub slice_height: u16,
    pub bits_per_pixel: u16,
    pub slices_per_channel: u8,
    pub bits_per_component: u8,
    pub lanes: u8,
    pub version_major: u8,
    pub version_minor: u8,
    pub block_prediction: bool,
    pub burst_mode: bool,
    pub noncontinuous_clock: bool,
}
