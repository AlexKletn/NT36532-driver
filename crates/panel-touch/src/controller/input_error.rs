use crate::ReportError;

#[derive(Debug, PartialEq, Eq)]
pub enum InputError<E> {
    NotRunning,
    Transport(E),
    InvalidReport(ReportError),
}
