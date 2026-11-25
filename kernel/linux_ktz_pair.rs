// SPDX-License-Identifier: GPL-2.0-only

use core::ptr::NonNull;

use kernel::{
    bindings,
    error::{Error, code::EINVAL},
};

use crate::{
    Ktz8866,
    ktz_adapter::Ktz8866Adapter,
    ktz_pair::{Ktz8866Pair, PairConfig},
};

use super::{linux_ktz_bus::LinuxKtzBus, linux_ktz_delay::LinuxKtzDelay};

pub(super) type LinuxKtzPair = Ktz8866Adapter<Ktz8866Pair<LinuxKtzBus, LinuxKtzBus, LinuxKtzDelay>>;

/// # Safety
///
/// The panel device and its dedicated I²C clients must remain bound while
/// the adapter is used. Serialize access with the panel lifecycle.
pub(super) unsafe fn acquire_pair(
    device: NonNull<bindings::device>,
    config: PairConfig,
    sink_masks: [u8; 2],
) -> Result<LinuxKtzPair, Error> {
    // SAFETY: The caller owns both managed I²C clients for the panel.
    let (led_only, led_and_bias) = unsafe {
        (
            LinuxKtzBus::acquire(device, c"spinel,backlight-a")?,
            LinuxKtzBus::acquire(device, c"spinel,backlight-b")?,
        )
    };
    let led_only = Ktz8866::new(led_only, sink_masks[0]).map_err(|_| EINVAL)?;
    let led_and_bias = Ktz8866::new(led_and_bias, sink_masks[1]).map_err(|_| EINVAL)?;
    Ok(Ktz8866Adapter::new(Ktz8866Pair::new(
        led_only,
        led_and_bias,
        LinuxKtzDelay,
        config,
    )))
}
