use super::{DisplayState, ShutdownErrors};

#[derive(Debug, PartialEq, Eq)]
pub enum DisplayError<P, B> {
    InvalidState(DisplayState),
    EnableVddio {
        source: P,
        rollback: ShutdownErrors<P, B>,
    },
    EnableBias {
        source: B,
        rollback: ShutdownErrors<P, B>,
    },
    PreparePanel {
        source: P,
        rollback: ShutdownErrors<P, B>,
    },
    Backlight(B),
    LiveRefresh(P),
    Shutdown(ShutdownErrors<P, B>),
}
