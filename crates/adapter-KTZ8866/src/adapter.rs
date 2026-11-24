use panel_backlight::Backlight;
use panel_power::PanelPower;

use crate::{AdapterError, Ktz8866Hardware};

pub struct Ktz8866Adapter<H> {
    hardware: H,
    level: Option<u16>,
}

impl<H> Ktz8866Adapter<H> {
    const MAX_LEVEL: u16 = 0x7ff;

    pub fn new(hardware: H) -> Self {
        Self {
            hardware,
            level: None,
        }
    }
}

impl<H: Ktz8866Hardware> Backlight for Ktz8866Adapter<H> {
    type Error = AdapterError<H::Error>;

    fn set_level(&mut self, level: u16) -> Result<(), Self::Error> {
        if level > Self::MAX_LEVEL {
            return Err(AdapterError::LevelOutOfRange {
                level,
                max_level: Self::MAX_LEVEL,
            });
        }

        // A partial write can leave the two LED banks at different levels.
        self.level = None;
        self.hardware
            .apply_level(level)
            .map_err(AdapterError::Hardware)?;
        self.level = Some(level);
        Ok(())
    }

    fn level(&self) -> Option<u16> {
        self.level
    }

    fn max_level(&self) -> u16 {
        Self::MAX_LEVEL
    }
}

impl<H: Ktz8866Hardware> PanelPower for Ktz8866Adapter<H> {
    type Error = AdapterError<H::Error>;

    fn enable(&mut self) -> Result<(), Self::Error> {
        self.level = None;
        self.hardware.enable_bias().map_err(AdapterError::Hardware)
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        self.level = None;
        self.hardware.disable_bias().map_err(AdapterError::Hardware)
    }
}

#[cfg(test)]
#[path = "adapter.tests.rs"]
mod tests;
