#[derive(Debug, PartialEq, Eq)]
pub enum InterruptAction {
    None,
    ArmPenWatchdog { delay_ms: u64 },
    ReloadFirmware,
}
