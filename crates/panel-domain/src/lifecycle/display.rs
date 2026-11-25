use panel_backlight::Backlight;
use panel_power::PanelPower;

use super::{DisplayError, DisplayState, Panel, ShutdownErrors, pending_shutdown::PendingShutdown};

pub struct Display<P, B> {
    panel: P,
    illumination: B,
    prepared: bool,
    pending: PendingShutdown,
}

impl<P: Panel, B: Backlight + PanelPower<Error = <B as Backlight>::Error>> Display<P, B> {
    pub fn new(panel: P, illumination: B) -> Self {
        Self {
            panel,
            illumination,
            prepared: false,
            pending: PendingShutdown::default(),
        }
    }

    pub fn state(&self) -> DisplayState {
        if self.prepared {
            DisplayState::Prepared
        } else if self.pending.is_pending() {
            DisplayState::RecoveryRequired
        } else {
            DisplayState::Off
        }
    }

    pub fn prepare(
        &mut self,
        mode: &P::Mode,
    ) -> Result<(), DisplayError<P::Error, <B as Backlight>::Error>> {
        let state = self.state();
        if state != DisplayState::Off {
            return Err(DisplayError::InvalidState(state));
        }

        self.pending.vddio = true;
        if let Err(source) = self.panel.enable_vddio() {
            return Err(DisplayError::EnableVddio {
                source,
                rollback: self.shutdown(),
            });
        }
        self.pending.bias = true;
        if let Err(source) = self.illumination.enable() {
            return Err(DisplayError::EnableBias {
                source,
                rollback: self.shutdown(),
            });
        }
        self.pending.dsi = true;
        if let Err(source) = self.panel.prepare(mode) {
            return Err(DisplayError::PreparePanel {
                source,
                rollback: self.shutdown(),
            });
        }

        self.prepared = true;
        Ok(())
    }

    pub fn max_brightness(&self) -> u16 {
        self.illumination.max_level()
    }

    pub fn set_brightness(
        &mut self,
        level: u16,
    ) -> Result<(), DisplayError<P::Error, <B as Backlight>::Error>> {
        if !self.prepared {
            return Err(DisplayError::InvalidState(self.state()));
        }
        self.illumination
            .set_level(level)
            .map_err(DisplayError::Backlight)
    }

    pub fn unprepare(&mut self) -> Result<(), DisplayError<P::Error, <B as Backlight>::Error>> {
        let errors = self.shutdown();
        if errors.has_errors() {
            Err(DisplayError::Shutdown(errors))
        } else {
            Ok(())
        }
    }

    fn shutdown(&mut self) -> ShutdownErrors<P::Error, <B as Backlight>::Error> {
        let backlight = if core::mem::take(&mut self.prepared) {
            self.illumination.set_level(0).err()
        } else {
            None
        };
        let panel = if self.pending.dsi {
            let result = self.panel.unprepare();
            self.pending.dsi = result.is_err();
            result.err()
        } else {
            None
        };
        let bias = if self.pending.bias {
            let result = self.illumination.disable();
            self.pending.bias = result.is_err();
            result.err()
        } else {
            None
        };
        // DSI cleanup needs the logic rail, and matrix bias must fall before it.
        let vddio = if self.pending.vddio && !self.pending.dsi && !self.pending.bias {
            let result = self.panel.disable_vddio();
            self.pending.vddio = result.is_err();
            result.err()
        } else {
            None
        };

        ShutdownErrors {
            backlight,
            panel,
            bias,
            vddio,
        }
    }
}

#[cfg(test)]
#[path = "display.tests.rs"]
mod tests;

impl<P, B> Display<P, B>
where
    P: super::LiveRefresh,
    B: Backlight + PanelPower<Error = <B as Backlight>::Error>,
{
    pub fn set_live_refresh(
        &mut self,
        mode: &P::Mode,
    ) -> Result<(), DisplayError<P::Error, <B as Backlight>::Error>> {
        if !self.prepared {
            return Err(DisplayError::InvalidState(self.state()));
        }
        self.panel
            .set_live_refresh(mode)
            .map_err(DisplayError::LiveRefresh)
    }
}
