// SPDX-License-Identifier: GPL-2.0-only

use kernel::{
    new_mutex,
    prelude::*,
    sync::Mutex,
    time::{Instant, Monotonic},
};
use panel_touch::{
    BootError, InputError, PagedSpi, ResetHistory, SpiError, TouchController, TouchRuntime,
};

use super::{
    linux_touch_firmware::{LinuxTouchFirmware, LinuxTouchStartError},
    linux_touch_input::LinuxTouchInput,
    linux_touch_irq::LinuxTouchIrq,
    linux_touch_panel::LinuxTouchPanel,
    linux_touch_reset::LinuxTouchReset,
    linux_touch_spi::LinuxTouchSpi,
    touch_irq_state::TouchIrqState,
};

unsafe extern "C" {
    fn nt36532_touch_take_history_request() -> bool;
}

type TransportError = SpiError<Error>;
type Runtime<'a> =
    TouchRuntime<PagedSpi<LinuxTouchSpi<'a>>, LinuxTouchReset<'a>, LinuxTouchInput<'a>>;

#[pin_data]
pub(super) struct LinuxTouchSession<'a> {
    firmware: LinuxTouchFirmware<'a>,
    pen_command: Option<(u8, u8)>,
    clock_start: Instant<Monotonic>,
    #[pin]
    irq_state: Mutex<TouchIrqState>,
    #[pin]
    runtime: Mutex<Runtime<'a>>,
}

impl<'a> LinuxTouchSession<'a> {
    pub(super) fn new(
        firmware: LinuxTouchFirmware<'a>,
        spi: LinuxTouchSpi<'a>,
        reset: LinuxTouchReset<'a>,
        input: LinuxTouchInput<'a>,
        pen_command: Option<(u8, u8)>,
    ) -> impl PinInit<Self> + 'a {
        let contact_max = input.contact_max();
        let pen_max = input.pen_max();
        pin_init!(Self {
            firmware,
            pen_command,
            clock_start: Instant::now(),
            irq_state <- new_mutex!(TouchIrqState::Offline),
            runtime <- new_mutex!(TouchRuntime::new(
                TouchController::new(PagedSpi::new(spi), reset),
                input,
                contact_max,
                pen_max,
            )),
        })
    }

    /// # Safety
    ///
    /// The IRQ source must be dedicated to this session. Its callbacks must
    /// already be installed. Call from a worker independent of those callbacks.
    pub(super) unsafe fn start(
        &self,
        irq: &LinuxTouchIrq<'_>,
    ) -> Result<Option<BootError<TransportError>>, LinuxTouchStartError<TransportError>> {
        let mut state = self.irq_state.lock();
        if *state == TouchIrqState::Paused {
            return Ok(None);
        }
        let previous = self.drain_irq(irq, &mut state);
        let pen_warning = if previous == TouchIrqState::FirmwareRetained {
            None
        } else {
            let mut runtime = self.runtime.lock();
            runtime.stop();
            match self.firmware.start(&mut runtime, self.pen_command) {
                Ok(warning) => warning,
                Err(error) => {
                    *state = TouchIrqState::Failed;
                    return Err(error);
                }
            }
        };
        // SAFETY: The IRQ is drained, and the runtime is ready and unlocked.
        unsafe { irq.enable() };
        *state = TouchIrqState::Enabled;
        Ok(pen_warning)
    }

    /// # Safety
    ///
    /// The IRQ source must be dedicated to this session. Invoke outside the IRQ
    /// and watchdog callbacks, without holding either session lock.
    pub(super) unsafe fn stop(&self, irq: &LinuxTouchIrq<'_>, preserve: bool) {
        let mut state = self.irq_state.lock();
        if *state == TouchIrqState::Paused {
            return;
        }
        let previous = self.drain_irq(irq, &mut state);
        let mut runtime = self.runtime.lock();
        if preserve && previous == TouchIrqState::Enabled && runtime.suspend_input() {
            *state = TouchIrqState::FirmwareRetained;
        } else {
            runtime.stop();
        }
    }

    pub(super) fn state_name(&self) -> &'static CStr {
        match self.irq_state.try_lock().as_deref() {
            Some(TouchIrqState::Offline | TouchIrqState::FirmwareRetained) => c"offline",
            Some(TouchIrqState::Paused) => c"paused",
            Some(TouchIrqState::Enabled) => c"running",
            Some(TouchIrqState::Failed) => c"failed",
            None => c"recovering",
        }
    }

    /// # Safety
    ///
    /// The IRQ source must be dedicated to this session, and the caller must
    /// hold neither session lock.
    pub(super) unsafe fn pause(&self, irq: &LinuxTouchIrq<'_>) {
        let mut state = self.irq_state.lock();
        self.drain_irq(irq, &mut state);
        self.runtime.lock().stop();
        *state = TouchIrqState::Paused;
    }

    pub(super) fn resume(&self, follower: &LinuxTouchPanel<'_>) {
        let mut state = self.irq_state.lock();
        if matches!(*state, TouchIrqState::Paused | TouchIrqState::Failed) {
            *state = TouchIrqState::Offline;
            follower.queue_start();
        }
    }

    pub(super) fn interrupt(
        &self,
        irq: &LinuxTouchIrq<'_>,
    ) -> Result<bool, InputError<TransportError>> {
        let mut runtime = self.runtime.lock();
        let action = runtime.handle_interrupt_with_history(
            self.elapsed_ms(),
            // SAFETY: The shim serializes reads with sysfs parameter writes.
            || unsafe { nt36532_touch_take_history_request() },
            |ResetHistory { primary, secondary }| {
                pr_info!(
                    "NT36532 reset history: primary={:02x?} secondary={:02x?}\n",
                    primary,
                    secondary
                );
            },
        )?;
        Ok(irq.interrupt_requests_reload(action))
    }

    pub(super) fn watchdog(&self, irq: &LinuxTouchIrq<'_>) {
        let mut runtime = self.runtime.lock();
        let delay = runtime.pen_watchdog(self.elapsed_ms());
        irq.watchdog_result(delay);
    }

    pub(super) fn firmware_history(&self) -> Result<ResetHistory> {
        let state = self.irq_state.lock();
        if *state != TouchIrqState::Enabled {
            return Err(EAGAIN);
        }
        let history = self.runtime.lock().firmware_history().map_err(|_| EAGAIN)?;
        if history.primary.is_none() && history.secondary.is_none() {
            return Err(EIO);
        }
        Ok(history)
    }

    pub(super) fn eraser(&self) -> bool {
        self.runtime.lock().eraser()
    }

    pub(super) fn toggle_pen_tool(&self) -> Result {
        if self.runtime.lock().toggle_pen_tool() {
            Ok(())
        } else {
            Err(EAGAIN)
        }
    }

    fn elapsed_ms(&self) -> u64 {
        self.clock_start.elapsed().as_millis() as u64
    }

    fn drain_irq(&self, irq: &LinuxTouchIrq<'_>, state: &mut TouchIrqState) -> TouchIrqState {
        let previous = core::mem::replace(state, TouchIrqState::Offline);
        if previous == TouchIrqState::Enabled {
            // SAFETY: The lifecycle lock serializes transitions. Neither callback
            // needs that lock, and this method is called with runtime unlocked.
            unsafe { irq.disable_and_drain() };
        }
        previous
    }
}
