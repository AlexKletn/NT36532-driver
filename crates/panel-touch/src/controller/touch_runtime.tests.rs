use super::*;
use crate::{Contact, Pen, TOUCH_PROFILES};

#[derive(Default)]
struct Transport {
    reset_report: bool,
}

impl BootIo for &mut Transport {
    type Error = ();

    fn write(&mut self, _: u32, _: &[u8]) -> Result<(), ()> {
        Ok(())
    }

    fn read(&mut self, address: u32, data: &mut [u8]) -> Result<(), ()> {
        match address {
            0x1fb104 => data[3..].copy_from_slice(&[0x32, 0x65, 0x03]),
            0x125860 => data[0] = 0xa0,
            0x125800 if self.reset_report => data[..6].fill(0xfd),
            0x125800 => data[65] = 1,
            0x121afc => data.fill(0xaa),
            0x121b3c => return Err(()),
            _ => panic!("unexpected read"),
        }
        Ok(())
    }

    fn delay_ms(&mut self, _: u16) {}
}

struct Reset;

impl ResetLine for Reset {
    type Error = ();

    fn set_asserted(&mut self, _: bool) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
struct Sink {
    frames: usize,
    releases: usize,
}

impl InputSink for &mut Sink {
    fn contact(&mut self, _: &Contact) {}

    fn finish_contacts(&mut self) {
        self.frames += 1;
    }

    fn pen(&mut self, _: &Pen, _: bool) {}

    fn release_pen(&mut self) {
        self.releases += 1;
    }
}

#[test]
fn requests_reload_only_after_releasing_input() {
    let Some(profile) = TOUCH_PROFILES.first() else {
        return;
    };
    let mut bytes = [0u8; 128];
    for (offset, value) in [
        (0, 0x40u32),
        (4, 0x1000),
        (8, 7),
        (12, 0x50),
        (16, 0x2000),
        (20, 7),
        (24, profile.crc[0]),
        (28, profile.crc[1]),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let image = FirmwareImage::parse(&bytes).unwrap();
    let mut bus = Transport::default();
    let mut sink = Sink::default();
    let controller = TouchController::new(&mut bus, Reset);
    let mut runtime = TouchRuntime::new(controller, &mut sink, [0; 2], Some([0; 2]));
    assert_eq!(runtime.start(&image, None), Ok(None));
    assert_eq!(
        runtime.handle_interrupt(10),
        Ok(InterruptAction::ArmPenWatchdog { delay_ms: 120 })
    );
    assert_eq!(runtime.handle_interrupt(20), Ok(InterruptAction::None));
    assert_eq!(runtime.pen_watchdog(130), Some(10));
    assert_eq!(runtime.pen_watchdog(140), None);
    assert_eq!(runtime.input.releases, 1);

    assert!(runtime.suspend_input());
    assert_eq!(
        runtime.handle_interrupt(145),
        Ok(InterruptAction::ArmPenWatchdog { delay_ms: 120 })
    );
    runtime.stop();
    assert!(!runtime.suspend_input());
    assert_eq!(runtime.handle_interrupt(150), Err(InputError::NotRunning));

    let mut bus = Transport { reset_report: true };
    let controller = TouchController::new(&mut bus, Reset);
    let mut runtime = TouchRuntime::new(controller, &mut sink, [0; 2], Some([0; 2]));
    assert_eq!(runtime.start(&image, None), Ok(None));
    assert_eq!(
        runtime.firmware_history(),
        Ok(ResetHistory {
            primary: Some([0xaa; 64]),
            secondary: None
        })
    );
    let frames = runtime.input.frames;
    let mut history = None;
    assert_eq!(
        runtime.handle_interrupt_with_history(160, || true, |value| history = Some(value)),
        Ok(InterruptAction::None)
    );
    assert_eq!(
        history,
        Some(ResetHistory {
            primary: Some([0xaa; 64]),
            secondary: None
        })
    );
    for _ in 0..9 {
        assert_eq!(runtime.handle_interrupt(160), Ok(InterruptAction::None));
    }
    assert_eq!(runtime.input.frames, frames);
    assert_eq!(
        runtime.handle_interrupt(160),
        Ok(InterruptAction::ReloadFirmware)
    );
    assert_eq!(runtime.input.frames, frames + 1);
    assert_eq!(runtime.handle_interrupt(160), Err(InputError::NotRunning));
    assert_eq!(runtime.firmware_history(), Err(InputError::NotRunning));
}
