#[derive(Debug, PartialEq, Eq)]
pub enum SpiError<E> {
    Transport(E),
    InvalidAddress,
    TransferTooLong,
}
