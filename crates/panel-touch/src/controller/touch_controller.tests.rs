use super::*;
use crate::TOUCH_PROFILES;

#[derive(Default)]
struct TestIo {
    identified: bool,
    valid_chip: bool,
    reset_event: bool,
    writes: usize,
}

impl BootIo for TestIo {
    type Error = ();

    fn write(&mut self, _: u32, _: &[u8]) -> Result<(), ()> {
        assert!(self.identified);
        self.writes += 1;
        Ok(())
    }

    fn read(&mut self, address: u32, data: &mut [u8]) -> Result<(), ()> {
        match address {
            CHIP_TRIM_ADDRESS => {
                self.identified = true;
                if self.valid_chip {
                    data[3..].copy_from_slice(&CHIP_ID);
                }
            }
            0x125860 => data[0] = 0xa0,
            EVENT_BUFFER => {
                assert_eq!(data.len(), TOUCH_REPORT_BYTES);
                if self.reset_event {
                    data[..6].fill(0xfd);
                }
            }
            FIRMWARE_INFO => data.fill(0xab),
            _ => panic!("unexpected register {address:#x}"),
        }
        Ok(())
    }

    fn delay_ms(&mut self, _: u16) {}
}

#[derive(Default)]
struct TestReset {
    transitions: usize,
}

impl ResetLine for TestReset {
    type Error = ();

    fn set_asserted(&mut self, asserted: bool) -> Result<(), ()> {
        assert_eq!(asserted, self.transitions == 0);
        self.transitions += 1;
        Ok(())
    }
}

#[test]
fn rejects_unrecognized_hardware_before_upload() {
    let Some(bytes) = image() else {
        return;
    };
    let firmware = FirmwareImage::parse(&bytes).unwrap();
    let mut controller = TouchController::new(TestIo::default(), TestReset::default());
    assert_eq!(
        controller.start(&firmware),
        Err(BootError::UnexpectedChipId { trim: [0; 6] })
    );
    assert_eq!(controller.reset.transitions, 2);
    assert_eq!(controller.bootloader.io_mut().writes, 0);
}

#[test]
fn reads_firmware_information_from_event_buffer() {
    let mut controller = TouchController::new(TestIo::default(), TestReset::default());
    assert_eq!(controller.firmware_info(), Ok([0xab; 12]));
}

fn image() -> Option<[u8; 128]> {
    let profile = TOUCH_PROFILES.first()?;
    let mut bytes = [0; 128];
    for (offset, word) in [
        (0, 0x40u32),
        (4, 0x1000),
        (8, 7),
        (12, 0x50),
        (16, 0x2000),
        (20, 7),
        (24, profile.crc[0]),
        (28, profile.crc[1]),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&word.to_le_bytes());
    }
    Some(bytes)
}

#[test]
fn reads_after_firmware_is_released_and_stops_on_reset() {
    let Some(bytes) = image() else {
        return;
    };
    let mut controller = TouchController::new(
        TestIo {
            valid_chip: true,
            ..TestIo::default()
        },
        TestReset::default(),
    );
    assert_eq!(
        controller.read_report(false, [0; 2], [0; 2]),
        Err(InputError::NotRunning)
    );
    {
        let image = FirmwareImage::parse(&bytes).unwrap();
        controller.start(&image).unwrap();
    }
    assert!(matches!(
        controller.read_report(false, [0; 2], [0; 2]),
        Ok(InputReport::Input { .. })
    ));
    controller.bootloader.io_mut().reset_event = true;
    for _ in 0..10 {
        assert_eq!(
            controller.read_report(false, [0; 2], [0; 2]),
            Ok(InputReport::Idle)
        );
    }

    assert_eq!(
        controller.read_report(false, [0; 2], [0; 2]),
        Ok(InputReport::FirmwareReset)
    );
    assert_eq!(
        controller.read_report(false, [0; 2], [0; 2]),
        Err(InputError::NotRunning)
    );
}
