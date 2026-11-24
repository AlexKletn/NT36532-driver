#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError<E> {
    LevelOutOfRange { level: u16, max_level: u16 },
    Hardware(E),
}
