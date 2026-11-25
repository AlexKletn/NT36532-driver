// SPDX-License-Identifier: GPL-2.0-only

use crate::{DsiCommand, PanelMode};

#[derive(Clone, Copy)]
pub struct DisplayMode {
    pub refresh_hz: u16,
    pub hdisplay: u16,
    pub hfront: u16,
    pub hsync: u16,
    pub hback: u16,
    pub vdisplay: u16,
    pub vfront: u16,
    pub vsync: u16,
    pub vback: u16,
    pub(crate) post_init: &'static [DsiCommand<'static>],
}

impl DisplayMode {
    pub fn panel_mode(&self, pps: Option<&[u8; 128]>) -> PanelMode<'static> {
        PanelMode {
            compression_pps: pps.copied(),
            post_init: self.post_init,
        }
    }
}
