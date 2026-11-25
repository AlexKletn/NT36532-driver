use crate::{
    BootError, BootIo, FirmwareImage, InputError, InputReport, InputSink, InputState,
    InterruptAction, ResetHistory, ResetLine, TouchController,
};

pub struct TouchRuntime<I, R, S> {
    controller: TouchController<I, R>,
    input: S,
    state: InputState,
    contact_max: [u16; 2],
    pen_max: Option<[u16; 2]>,
}

impl<I, R, S> TouchRuntime<I, R, S>
where
    I: BootIo,
    R: ResetLine<Error = I::Error>,
    S: InputSink,
{
    pub fn new(
        controller: TouchController<I, R>,
        input: S,
        contact_max: [u16; 2],
        pen_max: Option<[u16; 2]>,
    ) -> Self {
        Self {
            controller,
            input,
            state: InputState::default(),
            contact_max,
            pen_max,
        }
    }

    pub fn start(
        &mut self,
        image: &FirmwareImage<'_>,
        pen_command: Option<(u8, u8)>,
    ) -> Result<Option<BootError<I::Error>>, BootError<I::Error>> {
        self.stop();
        self.controller.start(image)?;
        Ok(pen_command
            .filter(|_| self.pen_max.is_some())
            .and_then(|(support, kind)| self.controller.enable_pen(support, kind).err()))
    }

    pub fn stop(&mut self) {
        self.controller.stop();
        self.state.release_all(&mut self.input);
    }

    pub fn suspend_input(&mut self) -> bool {
        self.state.release_all(&mut self.input);
        self.controller.is_running()
    }

    pub fn handle_interrupt(
        &mut self,
        now_ms: u64,
    ) -> Result<InterruptAction, InputError<I::Error>> {
        self.handle_interrupt_with_history(now_ms, || false, |_| {})
    }

    pub fn handle_interrupt_with_history(
        &mut self,
        now_ms: u64,
        take_request: impl FnOnce() -> bool,
        publish: impl FnOnce(ResetHistory),
    ) -> Result<InterruptAction, InputError<I::Error>> {
        let report = self.controller.read_report_with_history(
            self.pen_max.is_some(),
            self.contact_max,
            self.pen_max.unwrap_or_default(),
            take_request,
            publish,
        )?;
        let pen_was_present = self.state.pen_active();
        self.state.publish(&report, now_ms, &mut self.input);
        if report == InputReport::FirmwareReset {
            return Ok(InterruptAction::ReloadFirmware);
        }
        if !pen_was_present && let Some(delay_ms) = self.pen_watchdog(now_ms) {
            return Ok(InterruptAction::ArmPenWatchdog { delay_ms });
        }
        Ok(InterruptAction::None)
    }

    pub fn firmware_history(&mut self) -> Result<ResetHistory, InputError<I::Error>> {
        self.controller.firmware_history()
    }

    pub fn eraser(&self) -> bool {
        self.state.eraser()
    }

    pub fn toggle_pen_tool(&mut self) -> bool {
        self.state.toggle_pen_tool(&mut self.input)
    }

    pub fn pen_watchdog(&mut self, now_ms: u64) -> Option<u64> {
        self.state.check_pen_timeout(now_ms, &mut self.input)
    }
}

#[cfg(test)]
#[path = "touch_runtime.tests.rs"]
mod tests;
