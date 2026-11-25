// SPDX-License-Identifier: GPL-2.0-only

use core::num::NonZeroU32;

use kernel::time::{Delta, delay::fsleep};
use ktz8866_driver::Delay;

pub(super) struct LinuxKtzDelay;

impl Delay for LinuxKtzDelay {
    fn delay_ms(&mut self, milliseconds: NonZeroU32) {
        fsleep(Delta::from_millis(i64::from(milliseconds.get())));
    }
}
