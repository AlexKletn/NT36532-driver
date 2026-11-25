use crate::{
    BootError, BootIo, Bootloader, FirmwareImage, FirmwareProfile, InputError, InputReport,
    ResetHistory, ResetLine,
};

use crate::report::{PEN_REPORT_BYTES, TOUCH_REPORT_BYTES};

const CHIP_TRIM_ADDRESS: u32 = 0x1fb104;
const CHIP_ID: [u8; 3] = [0x32, 0x65, 0x03];
const EVENT_BUFFER: u32 = 0x125800;
const RESET_REPORTS_REQUIRED: u8 = 11;
const PRIMARY_HISTORY: u32 = 0x121afc;
const SECONDARY_HISTORY: u32 = 0x121b3c;
const FIRMWARE_INFO: u32 = EVENT_BUFFER + 0x78;

pub struct TouchController<I, R> {
    bootloader: Bootloader<I>,
    reset: R,
    profile: Option<&'static FirmwareProfile>,
    reset_reports: u8,
}

impl<I, R> TouchController<I, R>
where
    I: BootIo,
    R: ResetLine<Error = I::Error>,
{
    pub fn new(io: I, reset: R) -> Self {
        Self {
            bootloader: Bootloader::new(io),
            reset,
            profile: None,
            reset_reports: 0,
        }
    }

    pub fn start(&mut self, image: &FirmwareImage<'_>) -> Result<(), BootError<I::Error>> {
        self.profile = None;
        self.reset_reports = 0;
        self.reset.set_asserted(true)?;
        self.bootloader.io_mut().delay_ms(1);
        self.reset.set_asserted(false)?;
        self.bootloader.io_mut().delay_ms(10);

        let mut trim = [0; 6];
        self.bootloader
            .io_mut()
            .read(CHIP_TRIM_ADDRESS, &mut trim)?;
        if trim[3..] != CHIP_ID {
            return Err(BootError::UnexpectedChipId { trim });
        }
        self.bootloader.upload(image)?;
        self.profile = Some(image.profile());
        Ok(())
    }

    pub(crate) fn is_running(&self) -> bool {
        self.profile.is_some()
    }

    pub(crate) fn stop(&mut self) {
        self.profile = None;
        self.reset_reports = 0;
    }

    pub fn read_report(
        &mut self,
        with_pen: bool,
        contact_max: [u16; 2],
        pen_max: [u16; 2],
    ) -> Result<InputReport, InputError<I::Error>> {
        self.read_report_with_history(with_pen, contact_max, pen_max, || false, |_| {})
    }

    pub(crate) fn read_report_with_history(
        &mut self,
        with_pen: bool,
        contact_max: [u16; 2],
        pen_max: [u16; 2],
        take_request: impl FnOnce() -> bool,
        publish: impl FnOnce(ResetHistory),
    ) -> Result<InputReport, InputError<I::Error>> {
        let profile = self.profile.ok_or(InputError::NotRunning)?;
        let mut buffer = [0; PEN_REPORT_BYTES];
        let bytes = if with_pen {
            &mut buffer[..]
        } else {
            &mut buffer[..TOUCH_REPORT_BYTES]
        };
        self.bootloader
            .io_mut()
            .read(EVENT_BUFFER, bytes)
            .map_err(InputError::Transport)?;
        let report = InputReport::decode_with_profile(bytes, profile, contact_max, pen_max);
        match report {
            Ok(InputReport::Idle) => {}
            Ok(InputReport::FirmwareReset) => {
                if bytes[0] == 0xfd && take_request() {
                    publish(self.read_history());
                }
                self.reset_reports += 1;
                if self.reset_reports < RESET_REPORTS_REQUIRED {
                    return Ok(InputReport::Idle);
                }
                self.profile = None;
            }
            _ => self.reset_reports = 0,
        }
        report.map_err(InputError::InvalidReport)
    }

    pub fn firmware_history(&mut self) -> Result<ResetHistory, InputError<I::Error>> {
        if !self.is_running() {
            return Err(InputError::NotRunning);
        }
        Ok(self.read_history())
    }

    fn read_history(&mut self) -> ResetHistory {
        ResetHistory {
            primary: self.read_history_block(PRIMARY_HISTORY),
            secondary: self.read_history_block(SECONDARY_HISTORY),
        }
    }

    fn read_history_block(&mut self, address: u32) -> Option<[u8; 64]> {
        let mut bytes = [0; 64];
        self.bootloader.io_mut().read(address, &mut bytes).ok()?;
        Some(bytes)
    }

    pub fn firmware_info(&mut self) -> Result<[u8; 12], I::Error> {
        let mut info = [0; 12];
        self.bootloader.io_mut().read(FIRMWARE_INFO, &mut info)?;
        Ok(info)
    }

    pub fn enable_pen(&mut self, support: u8, pen_type: u8) -> Result<(), BootError<I::Error>> {
        self.bootloader.enable_pen(support, pen_type)
    }
}

#[cfg(test)]
#[path = "touch_controller.tests.rs"]
mod tests;
