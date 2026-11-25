use super::{DsiMode, DsiPacket};

pub struct DsiCommand<'a> {
    pub packet: DsiPacket<'a>,
    pub mode: DsiMode,
    pub delay_after_ms: u16,
}
