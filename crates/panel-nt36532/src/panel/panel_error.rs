#[derive(Debug, PartialEq, Eq)]
pub enum PanelError<E> {
    InvalidLongPacket,
    Vddio(E),
    Reset(E),
    Dsi(E),
    Compression(E),
    Shutdown {
        commands: Option<E>,
        reset: Option<E>,
    },
}
