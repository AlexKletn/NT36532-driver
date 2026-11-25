use core::str::FromStr;

use crate::{ContactFormat, FirmwareError};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TouchStrategy {
    #[default]
    Auto,
    Packed12,
    BigEndian16,
}

impl FromStr for TouchStrategy {
    type Err = FirmwareError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "packed12-v1" => Ok(Self::Packed12),
            "big-endian16-v1" => Ok(Self::BigEndian16),
            _ => Err(FirmwareError::InvalidStrategy),
        }
    }
}

impl TouchStrategy {
    pub(crate) fn contact_format(self) -> Option<ContactFormat> {
        match self {
            Self::Auto => None,
            Self::Packed12 => Some(ContactFormat::Packed12),
            Self::BigEndian16 => Some(ContactFormat::BigEndian16),
        }
    }
}
