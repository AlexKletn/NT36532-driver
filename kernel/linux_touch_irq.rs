// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::{c_int, c_uint, c_void},
    marker::PhantomData,
    ptr::NonNull,
};

use kernel::{
    bindings,
    error::{Error, code::EINVAL, from_err_ptr},
};
use panel_touch::InterruptAction;

unsafe extern "C" {
    fn nt36532_touch_irq_create(
        device: *mut c_void,
        irq: c_int,
        context: *mut c_void,
        interrupt: unsafe extern "C" fn(*mut c_void),
        watchdog: unsafe extern "C" fn(*mut c_void),
    ) -> *mut c_void;
    fn nt36532_touch_irq_enable(source: *mut c_void);
    fn nt36532_touch_irq_disable(source: *mut c_void);
    fn nt36532_touch_irq_arm_watchdog(source: *mut c_void, delay_ms: c_uint);
    fn nt36532_touch_irq_destroy(source: *mut c_void);
}

pub(super) struct LinuxTouchIrq<'a> {
    source: NonNull<c_void>,
    device_lifetime: PhantomData<&'a bindings::device>,
}

impl LinuxTouchIrq<'_> {
    /// # Safety
    ///
    /// The device and callback context must remain valid until this handle is
    /// dropped. Drop from a sleepable context outside the callbacks without
    /// holding their locks. The callbacks must serialize access to shared state.
    pub(super) unsafe fn register(
        device: NonNull<bindings::device>,
        irq: c_int,
        context: NonNull<c_void>,
        interrupt: unsafe extern "C" fn(*mut c_void),
        watchdog: unsafe extern "C" fn(*mut c_void),
    ) -> Result<Self, Error> {
        // SAFETY: The caller keeps the device and callback context alive.
        let source = from_err_ptr(unsafe {
            nt36532_touch_irq_create(
                device.as_ptr().cast(),
                irq,
                context.as_ptr(),
                interrupt,
                watchdog,
            )
        })?;
        Ok(Self {
            source: NonNull::new(source).ok_or(EINVAL)?,
            device_lifetime: PhantomData,
        })
    }

    /// # Safety
    ///
    /// Call once after registration or a matching disable. Serialize with
    /// teardown and publish the callback state before enabling the IRQ.
    pub(super) unsafe fn enable(&self) {
        // SAFETY: The source is live and its IRQ is disabled by the caller.
        unsafe { nt36532_touch_irq_enable(self.source.as_ptr()) };
    }

    /// # Safety
    ///
    /// Call once after enable. Do not call from a callback or while holding a
    /// lock needed by either callback.
    pub(super) unsafe fn disable_and_drain(&self) {
        // SAFETY: The caller ensures the callbacks can finish.
        unsafe { nt36532_touch_irq_disable(self.source.as_ptr()) };
    }

    pub(super) fn watchdog_result(&self, next_delay_ms: Option<u64>) {
        if let Some(delay_ms) = next_delay_ms {
            let delay_ms = u32::try_from(delay_ms).unwrap_or(u32::MAX);
            // SAFETY: The source is live. Disabled work refuses to queue.
            unsafe { nt36532_touch_irq_arm_watchdog(self.source.as_ptr(), delay_ms) };
        }
    }

    #[must_use]
    pub(super) fn interrupt_requests_reload(&self, action: InterruptAction) -> bool {
        match action {
            InterruptAction::ArmPenWatchdog { delay_ms } => {
                self.watchdog_result(Some(delay_ms));
                false
            }
            InterruptAction::ReloadFirmware => true,
            InterruptAction::None => false,
        }
    }
}

impl Drop for LinuxTouchIrq<'_> {
    fn drop(&mut self) {
        // SAFETY: This handle owns the source and keeps its callbacks live.
        unsafe { nt36532_touch_irq_destroy(self.source.as_ptr()) };
    }
}
