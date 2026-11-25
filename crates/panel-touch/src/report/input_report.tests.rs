use super::*;
use crate::{FirmwareError, TOUCH_PROFILES, TouchStrategy};

fn image() -> [u8; 128] {
    let mut bytes = [0; 128];
    for (offset, word) in [
        (0, 0x40u32),
        (4, 0x1000),
        (8, 7),
        (12, 0x50),
        (16, 0x2000),
        (20, 7),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

#[test]
fn uses_the_explicit_strategy_for_an_unknown_image() {
    let bytes = image();
    assert!(matches!(
        FirmwareImage::parse(&bytes),
        Err(FirmwareError::UnknownProfile { .. })
    ));
    if !TOUCH_PROFILES
        .iter()
        .any(|profile| profile.firmware_name == "novatek/novatek_ts_tm_fw.bin")
    {
        return;
    }
    let firmware = FirmwareImage::parse_with_strategy(&bytes, TouchStrategy::Packed12).unwrap();
    let mut report = [0; PEN_REPORT_BYTES];
    report[..6].copy_from_slice(&[9, 0x10, 0x20, 0x35, 3, 0]);
    report[64] = 0u8.wrapping_sub(report[..64].iter().copied().fold(0, u8::wrapping_add));
    report[65..77].copy_from_slice(&[1, 0, 12, 0, 32, 0, 7, 0xff, 1, 0, 2, 2]);
    let InputReport::Input { contacts, pen } =
        InputReport::decode(&report, &firmware, [2943, 1839], [2943, 1839]).unwrap()
    else {
        panic!("expected contacts and pen");
    };
    assert_eq!(
        contacts[0],
        Some(Contact {
            id: 1,
            x: 0x103,
            y: 0x205,
            width: 3
        })
    );
    assert_eq!(pen.unwrap().buttons, [false, true]);
    report[64] ^= 1;
    assert_eq!(
        InputReport::decode(&report, &firmware, [2943, 1839], [2943, 1839]),
        Err(ReportError::InvalidChecksum)
    );
    report[..6].fill(0xfd);
    assert_eq!(
        InputReport::decode(&report, &firmware, [2943, 1839], [2943, 1839]),
        Ok(InputReport::FirmwareReset)
    );
}
