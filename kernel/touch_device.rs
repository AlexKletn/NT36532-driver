// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_char, c_int, c_void},
    ptr::{self, NonNull},
    sync::atomic::{AtomicPtr, Ordering},
};
use kernel::{
    alloc::flags::GFP_KERNEL, bindings, device::Device, error::code::EINVAL, prelude::*,
    str::CStrExt,
};

use super::{
    linux_touch_firmware::LinuxTouchFirmware, linux_touch_input::LinuxTouchInput,
    linux_touch_irq::LinuxTouchIrq, linux_touch_panel::LinuxTouchPanel,
    linux_touch_reset::LinuxTouchReset, linux_touch_session::LinuxTouchSession,
    linux_touch_spi::LinuxTouchSpi, touch_callbacks::TouchCallbacks,
};

struct TouchDevice {
    follower: LinuxTouchPanel<'static>,
    irq: KBox<LinuxTouchIrq<'static>>,
    callbacks: KBox<TouchCallbacks>,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_touch_probe(
    spi: *mut c_void,
    device: *mut c_void,
    reset: *mut bindings::gpio_desc,
    irq: c_int,
    firmware: *const c_char,
    strategy: *const c_char,
    with_pen: bool,
    double_pen_resolution: bool,
    output: *mut *mut c_void,
) -> c_int {
    let probe = || -> Result {
        let device: NonNull<bindings::device> = NonNull::new(device.cast()).ok_or(EINVAL)?;
        // SAFETY: The C driver calls probe with managed resources and read-only
        // module parameters, which stay live through removal.
        let (firmware, spi, reset, input) = unsafe {
            let firmware = LinuxTouchFirmware::new(
                device.cast(),
                CStr::from_char_ptr(firmware.cast()),
                CStr::from_char_ptr(strategy.cast()),
            )?;
            (
                firmware,
                LinuxTouchSpi::new(NonNull::new(spi).ok_or(EINVAL)?),
                LinuxTouchReset::new(NonNull::new(reset).ok_or(EINVAL)?),
                LinuxTouchInput::register(device, with_pen, double_pen_resolution)?,
            )
        };
        let session = KBox::pin_init(
            LinuxTouchSession::new(firmware, spi, reset, input, with_pen.then_some((1, 0))),
            GFP_KERNEL,
        )?;
        let callbacks = KBox::new(
            TouchCallbacks {
                session,
                irq: AtomicPtr::new(ptr::null_mut()),
                follower: AtomicPtr::new(ptr::null_mut()),
            },
            GFP_KERNEL,
        )?;
        let context = NonNull::from(&*callbacks).cast();
        // SAFETY: The context is heap-allocated and IRQ starts disabled.
        let irq = unsafe {
            LinuxTouchIrq::register(
                device,
                irq,
                context,
                TouchCallbacks::interrupt,
                TouchCallbacks::watchdog,
            )
        }?;
        let irq = KBox::new(irq, GFP_KERNEL)?;
        callbacks
            .irq
            .store(ptr::from_ref(&*irq).cast_mut(), Ordering::Release);
        // SAFETY: The IRQ and its callback context have been fully initialized.
        let follower = unsafe {
            LinuxTouchPanel::register(device, context, TouchCallbacks::start, TouchCallbacks::stop)
        }?;
        let driver = KBox::new(
            TouchDevice {
                follower,
                irq,
                callbacks,
            },
            GFP_KERNEL,
        )?;
        // SAFETY: Probe is given a writable output slot by the SPI driver.
        unsafe { output.write(KBox::into_raw(driver).cast()) };
        Ok(())
    };
    probe().map_or_else(Error::to_errno, |()| 0)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_touch_remove(context: *mut c_void) {
    // SAFETY: The SPI driver retrieves exactly the owning pointer from probe.
    // Field order unregisters DRM and drains the IRQ before dropping callbacks.
    drop(unsafe { KBox::from_raw(context.cast::<TouchDevice>()) });
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_touch_is_eraser(context: *const c_void) -> bool {
    // SAFETY: Device sysfs callbacks are drained before SPI remove.
    let driver = unsafe { &*context.cast::<TouchDevice>() };
    driver.callbacks.session.eraser()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_touch_toggle_pen_tool(context: *const c_void) -> c_int {
    // SAFETY: Device sysfs callbacks are drained before SPI remove.
    let driver = unsafe { &*context.cast::<TouchDevice>() };
    driver
        .callbacks
        .session
        .toggle_pen_tool()
        .map_or_else(Error::to_errno, |()| 0)
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_touch_state(context: *const c_void) -> *const c_char {
    // SAFETY: Device sysfs callbacks are drained before SPI remove.
    let driver = unsafe { &*context.cast::<TouchDevice>() };
    driver.callbacks.session.state_name().as_ptr().cast()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_touch_set_paused(context: *const c_void, paused: bool) {
    // SAFETY: Device sysfs callbacks are drained before SPI remove.
    let driver = unsafe { &*context.cast::<TouchDevice>() };
    if paused {
        // SAFETY: This session owns the dedicated IRQ and no locks are held.
        unsafe { driver.callbacks.session.pause(&driver.irq) };
    } else {
        driver.callbacks.session.resume(&driver.follower);
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_touch_read_history(
    context: *const c_void,
    primary_out: *mut [u8; 64],
    secondary_out: *mut [u8; 64],
) -> c_int {
    // SAFETY: Device sysfs callbacks are drained before SPI remove.
    let driver = unsafe { &*context.cast::<TouchDevice>() };
    match driver.callbacks.session.firmware_history() {
        Ok(history) => {
            let mut available = 0;
            if let Some(primary) = history.primary {
                // SAFETY: The caller provided a writable 64-byte buffer.
                unsafe { primary_out.write(primary) };
                available |= 1;
            }
            if let Some(secondary) = history.secondary {
                // SAFETY: The caller provided a writable 64-byte buffer.
                unsafe { secondary_out.write(secondary) };
                available |= 2;
            }
            available
        }
        Err(error) => error.to_errno(),
    }
}
