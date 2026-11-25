use super::{DsiCommand, ResetStep};

#[derive(Clone, Copy)]
pub struct PanelProfile<'a> {
    pub vddio_settle_ms: u16,
    pub reset: &'a [ResetStep],
    pub init: &'a [DsiCommand<'a>],
    pub shutdown: &'a [DsiCommand<'a>],
}
