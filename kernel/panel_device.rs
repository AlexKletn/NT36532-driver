// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_int, c_void},
    num::NonZeroU32,
    ptr::NonNull,
};

use kernel::{bindings, prelude::*};

use crate::{BacklightConfig, BiasConfig, SELECTED_PROFILE, ktz_pair::PairConfig};

use super::{
    linux_drm_panel::LinuxDrmPanel, linux_ktz_power::LinuxKtzPower,
    linux_panel_links::LinuxPanelLinks, linux_panel_session::LinuxPanelSession,
};

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_panel_probe(
    dsi: *mut c_void,
    device: *mut c_void,
    reset: *mut bindings::gpio_desc,
    vddio: *mut bindings::regulator,
    output: *mut *mut c_void,
) -> c_int {
    let probe = || -> Result {
        let dsi = NonNull::new(dsi).ok_or(EINVAL)?;
        let device = NonNull::new(device.cast::<bindings::device>()).ok_or(EINVAL)?;
        let reset = NonNull::new(reset).ok_or(EINVAL)?;
        let vddio = NonNull::new(vddio).ok_or(EINVAL)?;
        let profile = SELECTED_PROFILE
            .as_ref()
            .and_then(|profile| profile.ktz8866)
            .ok_or(EINVAL)?;
        let registers = || BacklightConfig {
            led_config_1: Some(profile.led_config_1),
            led_config_2: Some(profile.led_config_2),
            dimming: Some(profile.dimming),
            led_option_2: Some(profile.led_option_2),
            pwm_ramp: Some(profile.pwm_ramp),
        };
        let config = PairConfig {
            led_only: registers(),
            led_and_bias: registers(),
            bias: BiasConfig {
                boost_register: profile.boost,
                positive_output_register: profile.positive_bias,
                negative_output_register: profile.negative_bias,
                step_delay_ms: NonZeroU32::new(profile.bias_step_ms.into()).ok_or(EINVAL)?,
            },
            settle_after_enable_ms: NonZeroU32::new(profile.settle_on_ms.into()),
        };
        let settle_off = NonZeroU32::new(profile.settle_off_ms.into()).ok_or(EINVAL)?;

        // SAFETY: The C probe owns the managed devices and lines through remove.
        let links = unsafe { LinuxPanelLinks::create(dsi)? };
        let illumination =
            unsafe { LinuxKtzPower::acquire(device, config, [profile.sink_mask; 2], settle_off)? };
        let session = unsafe { LinuxPanelSession::new(links, reset, vddio, illumination) }
            .map_err(|_| EINVAL)?;
        // Publish DRM before attach so the MSM host can discover the panel.
        let panel = unsafe { LinuxDrmPanel::register(device, session)? };
        let panel = KBox::new(panel, GFP_KERNEL)?;
        links.attach()?;
        // SAFETY: The caller supplies a writable output slot.
        unsafe { output.write(KBox::into_raw(panel).cast()) };
        Ok(())
    };
    probe().map_or_else(Error::to_errno, |()| 0)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_panel_remove(context: *mut c_void) {
    // SAFETY: The DSI driver passes the owning pointer returned by probe.
    drop(unsafe { KBox::from_raw(context.cast::<LinuxDrmPanel<LinuxKtzPower>>()) });
}
