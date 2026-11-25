use crate::{InputReport, InputSink};

const PEN_GONE_REPORTS: u8 = 8;
const PEN_RELEASE_MS: u64 = 120;

#[derive(Default)]
pub struct InputState {
    pen_last_ms: Option<u64>,
    pen_missing_reports: u8,
    eraser: bool,
}

impl InputState {
    pub(crate) fn pen_active(&self) -> bool {
        self.pen_last_ms.is_some()
    }

    pub fn publish(&mut self, report: &InputReport, now_ms: u64, sink: &mut impl InputSink) {
        match report {
            InputReport::Idle => {}
            InputReport::FirmwareReset => self.release_all(sink),
            InputReport::Input { contacts, pen } => {
                for contact in contacts.iter().flatten() {
                    sink.contact(contact);
                }
                sink.finish_contacts();

                if let Some(pen) = pen {
                    self.pen_last_ms = Some(now_ms);
                    self.pen_missing_reports = 0;
                    sink.pen(pen, self.eraser);
                } else if self.pen_last_ms.is_some() {
                    self.pen_missing_reports += 1;
                    if self.pen_missing_reports >= PEN_GONE_REPORTS {
                        self.release_pen(sink);
                    }
                }
            }
        }
    }

    pub fn check_pen_timeout(&mut self, now_ms: u64, sink: &mut impl InputSink) -> Option<u64> {
        let last_ms = self.pen_last_ms?;
        let remaining = PEN_RELEASE_MS.saturating_sub(now_ms.saturating_sub(last_ms));
        if remaining == 0 {
            self.release_pen(sink);
            return None;
        }
        Some(remaining)
    }

    pub fn eraser(&self) -> bool {
        self.eraser
    }

    pub fn toggle_pen_tool(&mut self, sink: &mut impl InputSink) -> bool {
        if !self.pen_active() {
            return false;
        }
        self.release_pen(sink);
        self.eraser = !self.eraser;
        true
    }

    pub fn release_all(&mut self, sink: &mut impl InputSink) {
        sink.finish_contacts();
        self.release_pen(sink);
    }

    fn release_pen(&mut self, sink: &mut impl InputSink) {
        if self.pen_last_ms.take().is_some() {
            self.pen_missing_reports = 0;
            sink.release_pen();
        }
    }
}

#[cfg(test)]
#[path = "input_state.tests.rs"]
mod tests;
