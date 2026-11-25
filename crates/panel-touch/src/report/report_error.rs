#[derive(Debug, PartialEq, Eq)]
pub enum ReportError {
    InvalidLength,
    InvalidChecksum,
}
