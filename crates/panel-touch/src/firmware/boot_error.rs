use super::BootCrcStatus;

#[derive(Debug, PartialEq, Eq)]
pub enum BootError<E> {
    Transport(E),
    InvalidSramRange,
    UnexpectedChipId { trim: [u8; 6] },
    CascadeCopyTimeout,
    FirmwareReadyTimeout { crc: BootCrcStatus },
    PenCommandTimeout,
}

impl<E> From<E> for BootError<E> {
    fn from(error: E) -> Self {
        Self::Transport(error)
    }
}
