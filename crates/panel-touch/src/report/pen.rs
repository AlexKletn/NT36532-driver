use crate::FirmwareProfile;

#[derive(Debug, PartialEq, Eq)]
pub struct Pen {
    pub x: u16,
    pub y: u16,
    pub pressure: u16,
    pub distance: u16,
    pub tilt_x: i8,
    pub tilt_y: i8,
    pub buttons: [bool; 2],
}

impl Pen {
    pub(super) fn decode(
        bytes: &[u8],
        profile: &FirmwareProfile,
        input_max: [u16; 2],
    ) -> Option<Self> {
        let [
            1,
            x_hi,
            x_lo,
            y_hi,
            y_lo,
            p_hi,
            p_lo,
            tilt_x,
            tilt_y,
            d_hi,
            d_lo,
            buttons,
            ..,
        ] = *bytes
        else {
            return None;
        };
        let mut x = u16::from_be_bytes([x_hi, x_lo]);
        let mut y = u16::from_be_bytes([y_hi, y_lo]);
        if let Some([wire_x, wire_y]) = profile.pen_wire_max {
            x = scale(x, wire_x, input_max[0])?;
            y = scale(y, wire_y, input_max[1])?;
        }
        Some(Self {
            x,
            y,
            pressure: u16::from_be_bytes([p_hi, p_lo]),
            distance: u16::from_be_bytes([d_hi, d_lo]),
            tilt_x: tilt_x as i8,
            tilt_y: tilt_y as i8,
            buttons: profile.pen_buttons(buttons),
        })
    }
}

fn scale(value: u16, wire_max: u16, input_max: u16) -> Option<u16> {
    let wire_max = u32::from(wire_max);
    let product = u32::from(value) * u32::from(input_max);
    let scaled = (product + wire_max / 2).checked_div(wire_max)?;
    Some(scaled.min(u32::from(input_max)) as u16)
}
