#[derive(Debug, PartialEq, Eq)]
pub enum FirmwareError {
    InvalidPartitionTable,
    InvalidStrategy,
    StrategyUnavailable,
    AmbiguousStrategy,
    UnknownProfile { crc: [u32; 2] },
}
