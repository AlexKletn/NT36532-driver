#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
    Off,
    Prepared,
    RecoveryRequired,
}
