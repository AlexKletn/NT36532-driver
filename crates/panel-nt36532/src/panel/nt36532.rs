use panel_domain::{LiveRefresh, Panel};

use super::{DsiCommand, DsiPacket, PanelError, PanelIo, PanelMode, PanelProfile};

pub struct Nt36532<'a, I> {
    io: I,
    profile: PanelProfile<'a>,
    needs_shutdown: bool,
}

impl<'a, I: PanelIo> Nt36532<'a, I> {
    pub fn new(io: I, profile: PanelProfile<'a>) -> Result<Self, PanelError<I::Error>> {
        validate_commands(profile.init)?;
        validate_commands(profile.shutdown)?;
        Ok(Self {
            io,
            profile,
            needs_shutdown: false,
        })
    }
}

impl<'a, I: PanelIo> Panel for Nt36532<'a, I> {
    type Error = PanelError<I::Error>;
    type Mode = PanelMode<'a>;

    fn enable_vddio(&mut self) -> Result<(), Self::Error> {
        self.io.enable_vddio().map_err(PanelError::Vddio)?;
        wait(&mut self.io, self.profile.vddio_settle_ms);
        Ok(())
    }

    fn prepare(&mut self, mode: &Self::Mode) -> Result<(), Self::Error> {
        validate_commands(mode.post_init)?;
        self.needs_shutdown = true;
        for step in self.profile.reset {
            self.io
                .set_reset(step.asserted)
                .map_err(PanelError::Reset)?;
            wait(&mut self.io, step.delay_after_ms);
        }
        run_commands(&mut self.io, self.profile.init).map_err(PanelError::Dsi)?;
        if let Some(pps) = &mode.compression_pps {
            self.io.send_dsc_pps(pps).map_err(PanelError::Compression)?;
        }
        run_commands(&mut self.io, mode.post_init).map_err(PanelError::Dsi)
    }

    fn unprepare(&mut self) -> Result<(), Self::Error> {
        if !self.needs_shutdown {
            return Ok(());
        }
        let commands = run_commands(&mut self.io, self.profile.shutdown).err();
        let reset = self.io.set_reset(true).err();
        // Once reset is asserted a retry can move on to removing the power rails.
        self.needs_shutdown = reset.is_some();
        if commands.is_some() || reset.is_some() {
            Err(PanelError::Shutdown { commands, reset })
        } else {
            Ok(())
        }
    }

    fn disable_vddio(&mut self) -> Result<(), Self::Error> {
        self.io.disable_vddio().map_err(PanelError::Vddio)
    }
}

impl<'a, I: PanelIo> LiveRefresh for Nt36532<'a, I> {
    fn set_live_refresh(&mut self, mode: &Self::Mode) -> Result<(), Self::Error> {
        validate_commands(mode.post_init)?;
        run_commands(&mut self.io, mode.post_init).map_err(PanelError::Dsi)
    }
}

fn validate_commands<E>(commands: &[DsiCommand<'_>]) -> Result<(), PanelError<E>> {
    if commands
        .iter()
        .any(|command| matches!(command.packet, DsiPacket::DcsLong(data) if data.len() < 2))
    {
        return Err(PanelError::InvalidLongPacket);
    }
    Ok(())
}

fn run_commands<I: PanelIo>(io: &mut I, commands: &[DsiCommand<'_>]) -> Result<(), I::Error> {
    for command in commands {
        io.write_dsi(command.packet, command.mode)?;
        wait(io, command.delay_after_ms);
    }
    Ok(())
}

fn wait(io: &mut impl PanelIo, milliseconds: u16) {
    if milliseconds != 0 {
        io.delay_ms(milliseconds);
    }
}

#[cfg(test)]
#[path = "nt36532.tests.rs"]
mod tests;
