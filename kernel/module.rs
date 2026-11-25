// SPDX-License-Identifier: GPL-2.0-only

//! Linux module for the Novatek NT36532.

use core::ffi::c_int;

use kernel::{error::to_result, prelude::*};
extern crate self as panel_touch;

#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-touch/src/controller/mod.rs"]
mod controller;
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-touch/src/firmware/mod.rs"]
mod firmware;
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-touch/src/profile/mod.rs"]
mod profile;
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-touch/src/report/mod.rs"]
mod report;
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-touch/src/spi/mod.rs"]
mod spi;

pub(crate) use controller::{
    InputError, InterruptAction, ResetHistory, ResetLine, TouchController, TouchRuntime,
};
pub(crate) use firmware::{BootError, BootIo, Bootloader, FirmwareError, FirmwareImage, Partition};
pub(crate) use profile::{ContactFormat, FirmwareProfile, TOUCH_PROFILES, TouchStrategy};
pub(crate) use report::{Contact, InputReport, InputSink, InputState, Pen, ReportError};
pub(crate) use spi::{PagedSpi, SpiBus, SpiError};

extern crate self as panel_domain;
extern crate self as panel_nt36532;

#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-nt36532/src/panel/mod.rs"]
mod display_panel;
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-nt36532/src/profile/mod.rs"]
mod display_profile;
extern crate self as panel_backlight;
extern crate self as panel_power;

#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-backlight/src/backlight.rs"]
mod backlight_port;
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-domain/src/lifecycle/mod.rs"]
mod display_lifecycle;
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/panel-power/src/panel_power.rs"]
mod power_port;

pub(crate) use backlight_port::Backlight;
pub(crate) use display_lifecycle::{Display, DisplayError, DisplayState, LiveRefresh, Panel};
pub(crate) use power_port::PanelPower;

pub(crate) use display_panel::{
    DsiCommand, DsiMode, DsiPacket, PanelIo, PanelMode, PanelProfile, ResetStep,
};
pub(crate) use display_profile::{
    DisplayMode, Ktz8866Profile, PanelLinkConfig, SELECTED_PROFILE, SelectedProfile,
};

#[allow(dead_code)]
mod dsi_bindings;
#[allow(dead_code)]
mod linux_drm_panel;
#[allow(dead_code)]
mod linux_panel_io;
#[allow(dead_code)]
mod linux_panel_links;
#[allow(dead_code)]
mod linux_panel_session;
#[allow(dead_code)]
mod linux_panel_session_error;
mod panel_modes;

#[cfg(nt36532_ktz8866)]
extern crate self as ktz8866_driver;
#[cfg(nt36532_ktz8866)]
include!(concat!(env!("OUT_DIR"), "/ktz_source.rs"));
#[cfg(nt36532_ktz8866)]
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/adapter-KTZ8866/src/adapter.rs"]
mod ktz_adapter;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/adapter-KTZ8866/src/adapter_error.rs"]
mod ktz_adapter_error;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/adapter-KTZ8866/src/hardware.rs"]
mod ktz_hardware;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code, unreachable_pub)]
#[path = "../crates/adapter-KTZ8866/src/pair/mod.rs"]
mod ktz_pair;
#[cfg(nt36532_ktz8866)]
pub(crate) use ktz_adapter_error::AdapterError;
#[cfg(nt36532_ktz8866)]
pub(crate) use ktz_hardware::Ktz8866Hardware;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code)]
mod linux_ktz_bus;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code)]
mod linux_ktz_delay;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code)]
mod linux_ktz_lines;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code)]
mod linux_ktz_pair;
#[cfg(nt36532_ktz8866)]
#[allow(dead_code)]
mod linux_ktz_power;

#[cfg(nt36532_ktz8866)]
mod panel_device;

mod linux_firmware_blob;
mod linux_touch_firmware;
mod linux_touch_input;
mod linux_touch_irq;
mod linux_touch_panel;
mod linux_touch_reset;
mod linux_touch_session;
mod linux_touch_spi;
mod touch_callbacks;
mod touch_device;
mod touch_irq_state;

unsafe extern "C" {
    #[cfg(nt36532_ktz8866)]
    fn nt36532_panel_driver_register() -> c_int;
    #[cfg(nt36532_ktz8866)]
    fn nt36532_panel_driver_unregister();
    fn nt36532_touch_driver_register() -> c_int;
    fn nt36532_touch_driver_unregister();
}

module! {
    type: Nt36532,
    name: "nt36532",
    description: "Novatek NT36532 display and touch controller",
    license: "GPL",
}

struct Nt36532;

impl kernel::Module for Nt36532 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        // SAFETY: The kernel invokes module initialization once, in process context.
        to_result(unsafe { nt36532_touch_driver_register() })?;
        #[cfg(nt36532_ktz8866)]
        if let Err(error) = to_result(unsafe { nt36532_panel_driver_register() }) {
            // SAFETY: The SPI registration succeeded above.
            unsafe { nt36532_touch_driver_unregister() };
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for Nt36532 {
    fn drop(&mut self) {
        // SAFETY: A live Nt36532 owns the registered SPI driver.
        unsafe { nt36532_touch_driver_unregister() };
        #[cfg(nt36532_ktz8866)]
        // SAFETY: Initialization completed both registrations.
        unsafe {
            nt36532_panel_driver_unregister()
        };
    }
}
