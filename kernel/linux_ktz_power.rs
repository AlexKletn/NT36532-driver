// SPDX-License-Identifier: GPL-2.0-only

use core::{num::NonZeroU32, ptr::NonNull};

use kernel::{
    bindings,
    error::{Error, code::EIO},
    prelude::pr_err,
};

use crate::{Backlight, Delay, PanelPower, ktz_pair::PairConfig};

use super::{
    linux_ktz_delay::LinuxKtzDelay,
    linux_ktz_lines::LinuxKtzLines,
    linux_ktz_pair::{LinuxKtzPair, acquire_pair},
};

pub(super) struct LinuxKtzPower {
    pair: LinuxKtzPair,
    lines: LinuxKtzLines,
    settle_off: NonZeroU32,
    bias_may_be_on: bool,
}

impl LinuxKtzPower {
    /// # Safety
    ///
    /// The panel and dedicated I²C clients must remain bound while this
    /// adapter is live. The caller must serialize access.
    pub(super) unsafe fn acquire(
        device: NonNull<bindings::device>,
        config: PairConfig,
        sink_masks: [u8; 2],
        settle_off: NonZeroU32,
    ) -> Result<Self, Error> {
        // SAFETY: The panel keeps both sets of managed resources alive.
        let pair = unsafe { acquire_pair(device, config, sink_masks)? };
        let lines = unsafe { LinuxKtzLines::acquire(device)? };
        Ok(Self {
            pair,
            lines,
            settle_off,
            bias_may_be_on: false,
        })
    }
}

impl Backlight for LinuxKtzPower {
    type Error = Error;

    fn set_level(&mut self, level: u16) -> Result<(), Error> {
        self.pair.set_level(level).map_err(chip_error)
    }

    fn level(&self) -> Option<u16> {
        self.pair.level()
    }

    fn max_level(&self) -> u16 {
        self.pair.max_level()
    }
}

impl PanelPower for LinuxKtzPower {
    type Error = Error;

    fn enable(&mut self) -> Result<(), Error> {
        self.lines.enable()?;
        self.bias_may_be_on = true;
        self.pair.enable().map_err(chip_error)
    }

    fn disable(&mut self) -> Result<(), Error> {
        let mut bias_result = Ok(());
        if self.bias_may_be_on {
            if let Err(error) = self.pair.set_level(0) {
                chip_error(error);
            }
            bias_result = self.pair.disable().map_err(chip_error);
            self.bias_may_be_on = bias_result.is_err();
            LinuxKtzDelay.delay_ms(self.settle_off);
        }
        let lines_result = self.lines.disable_bias();
        bias_result.and(lines_result)
    }
}

fn chip_error(error: impl core::fmt::Debug) -> Error {
    pr_err!("NT36532 KTZ8866 operation failed: {:?}\n", error);
    EIO
}
