#[derive(Default)]
pub(super) struct PendingShutdown {
    pub(super) dsi: bool,
    pub(super) bias: bool,
    pub(super) vddio: bool,
}

impl PendingShutdown {
    pub(super) fn is_pending(&self) -> bool {
        self.dsi || self.bias || self.vddio
    }
}
