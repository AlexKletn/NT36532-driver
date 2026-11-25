use super::{DsiMode, DsiPacket};

pub trait PanelIo {
    type Error;

    fn enable_vddio(&mut self) -> Result<(), Self::Error>;
    fn disable_vddio(&mut self) -> Result<(), Self::Error>;
    fn set_reset(&mut self, asserted: bool) -> Result<(), Self::Error>;
    fn delay_ms(&mut self, milliseconds: u16);

    /// Dual-DSI implementations must send each command to both receivers.
    fn write_dsi(&mut self, packet: DsiPacket<'_>, mode: DsiMode) -> Result<(), Self::Error>;
    fn send_dsc_pps(&mut self, pps: &[u8; 128]) -> Result<(), Self::Error>;
}
