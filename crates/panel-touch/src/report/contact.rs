use crate::{ContactFormat, FirmwareProfile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contact {
    pub id: u8,
    pub x: u16,
    pub y: u16,
    pub width: u8,
}

impl Contact {
    pub fn decode(data: [u8; 6], profile: &FirmwareProfile, input_max: [u16; 2]) -> Option<Self> {
        let [header, first, second, third, fourth, fifth] = data;
        let id = header >> 3;
        if !(1..=10).contains(&id) || !matches!(header & 7, 1 | 2) {
            return None;
        }
        let (x, y, width) = match profile.contact_format {
            ContactFormat::Packed12 => (
                (u16::from(first) << 4) | u16::from(third >> 4),
                (u16::from(second) << 4) | u16::from(third & 0xf),
                fourth,
            ),
            ContactFormat::BigEndian16 => (
                u16::from_be_bytes([first, second]),
                u16::from_be_bytes([third, fourth]),
                fifth,
            ),
        };
        let [x, y] = if let Some(wire_max) = profile.contact_wire_max {
            [
                scale(x, wire_max[0], input_max[0])?,
                scale(y, wire_max[1], input_max[1])?,
            ]
        } else {
            [x, y]
        };
        Some(Self {
            id,
            x,
            y,
            width: width.max(1),
        })
    }
}

fn scale(value: u16, wire_max: u16, input_max: u16) -> Option<u16> {
    if wire_max == 0 || value > wire_max {
        return None;
    }
    Some((u32::from(value) * u32::from(input_max) / u32::from(wire_max)) as u16)
}
