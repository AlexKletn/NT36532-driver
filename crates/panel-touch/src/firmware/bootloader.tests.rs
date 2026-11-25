use super::*;
use crate::TOUCH_PROFILES;

#[derive(Default)]
struct TestIo {
    writes: usize,
    crc_length: Option<[u8; 3]>,
    first_partition: Option<usize>,
    cascade_enabled: bool,
    ready: bool,
    force_timeout: bool,
}

impl BootIo for TestIo {
    type Error = ();

    fn write(&mut self, address: u32, data: &[u8]) -> Result<(), ()> {
        self.writes += 1;
        match address {
            0x1fb518 => self.crc_length = data.try_into().ok(),
            0x1000 => self.first_partition = Some(data.len()),
            CASCADE_COPY => self.cascade_enabled = data == [0x56],
            BOOT_READY => self.ready = data == [1],
            _ => {}
        }
        Ok(())
    }

    fn read(&mut self, address: u32, data: &mut [u8]) -> Result<(), ()> {
        match address {
            ILM_CRC_RESULT => {
                data.copy_from_slice(&0x1234_5678u32.to_le_bytes());
                return Ok(());
            }
            DLM_CRC_RESULT => return Err(()),
            CRC_ERROR_FLAGS => {
                data[0] = 2;
                return Ok(());
            }
            _ => {}
        }
        data[0] = if address == RESET_STATE && self.ready && !self.force_timeout {
            0xa0
        } else {
            0
        };
        Ok(())
    }

    fn delay_ms(&mut self, _: u16) {}
}

#[test]
fn uploads_cascade_image_and_programs_encoded_crc_length() {
    let Some(profile) = TOUCH_PROFILES.first() else {
        return;
    };
    let mut bytes = [0u8; 128];
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
    bytes[0x20] = 2;
    let image = FirmwareImage::parse(&bytes).unwrap();
    let mut bootloader = Bootloader::new(TestIo::default());
    bootloader.upload(&image).unwrap();
    let mut timeout = Bootloader::new(TestIo {
        force_timeout: true,
        ..TestIo::default()
    });
    assert_eq!(
        timeout.upload(&image),
        Err(BootError::FirmwareReadyTimeout {
            crc: BootCrcStatus {
                expected_ilm: profile.crc[0],
                expected_dlm: profile.crc[1],
                actual_ilm: Some(0x1234_5678),
                actual_dlm: None,
                flags: Some(2),
            }
        })
    );

    let io = bootloader.into_inner();
    assert!(io.cascade_enabled && io.ready);
    assert_eq!(io.crc_length, Some([7, 0, 0]));
    assert_eq!(io.first_partition, Some(8));

    bytes[4..8].copy_from_slice(&ADDRESS_SPACE_BYTES.to_le_bytes());
    let image = FirmwareImage::parse(&bytes).unwrap();
    let mut bootloader = Bootloader::new(TestIo::default());
    assert_eq!(bootloader.upload(&image), Err(BootError::InvalidSramRange));
    assert_eq!(bootloader.into_inner().writes, 0);
}
