use crate::{Contact, FirmwareImage, FirmwareProfile, Pen, ReportError};

const CONTACT_BYTES: usize = 6;
const MAX_CONTACTS: usize = 10;
pub(crate) const TOUCH_REPORT_BYTES: usize = 65;
pub(crate) const PEN_REPORT_BYTES: usize = 108;

#[derive(Debug, PartialEq, Eq)]
pub enum InputReport {
    Idle,
    FirmwareReset,
    Input {
        contacts: [Option<Contact>; MAX_CONTACTS],
        pen: Option<Pen>,
    },
}

impl InputReport {
    pub fn decode(
        bytes: &[u8],
        firmware: &FirmwareImage<'_>,
        contact_max: [u16; 2],
        pen_max: [u16; 2],
    ) -> Result<Self, ReportError> {
        Self::decode_with_profile(bytes, firmware.profile(), contact_max, pen_max)
    }

    pub fn decode_with_profile(
        bytes: &[u8],
        profile: &FirmwareProfile,
        contact_max: [u16; 2],
        pen_max: [u16; 2],
    ) -> Result<Self, ReportError> {
        if !matches!(bytes.len(), TOUCH_REPORT_BYTES | PEN_REPORT_BYTES) {
            return Err(ReportError::InvalidLength);
        }

        let marker = &bytes[..CONTACT_BYTES];
        if marker.iter().all(|&byte| byte == 0x77) {
            return Ok(Self::Idle);
        }
        if marker.iter().all(|&byte| matches!(byte, 0xfd | 0xfe)) {
            return Ok(Self::FirmwareReset);
        }
        let checksum = bytes[..TOUCH_REPORT_BYTES]
            .iter()
            .copied()
            .fold(0u8, u8::wrapping_add);
        if checksum != 0 {
            return Err(ReportError::InvalidChecksum);
        }

        let contacts = core::array::from_fn(|index| {
            let start = index * CONTACT_BYTES;
            let record = bytes[start..start + CONTACT_BYTES].try_into().ok()?;
            Contact::decode(record, profile, contact_max)
        });
        let pen = Pen::decode(&bytes[TOUCH_REPORT_BYTES..], profile, pen_max);
        Ok(Self::Input { contacts, pen })
    }
}

#[cfg(test)]
#[path = "input_report.tests.rs"]
mod tests;
