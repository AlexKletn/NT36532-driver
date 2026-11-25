use crate::ContactFormat;

pub struct FirmwareProfile {
    pub firmware_name: &'static str,
    pub crc: [u32; 2],
    pub contact_format: ContactFormat,
    pub contact_wire_max: Option<[u16; 2]>,
    pub pen_wire_max: Option<[u16; 2]>,
    pub pen_button_masks: [u8; 2],
}

impl FirmwareProfile {
    pub fn pen_buttons(&self, flags: u8) -> [bool; 2] {
        self.pen_button_masks.map(|mask| flags & mask != 0)
    }
}
