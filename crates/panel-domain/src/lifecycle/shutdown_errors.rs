#[derive(Debug, PartialEq, Eq)]
pub struct ShutdownErrors<P, B> {
    pub backlight: Option<B>,
    pub panel: Option<P>,
    pub bias: Option<B>,
    pub vddio: Option<P>,
}

impl<P, B> ShutdownErrors<P, B> {
    pub(super) fn has_errors(&self) -> bool {
        self.backlight.is_some()
            || self.panel.is_some()
            || self.bias.is_some()
            || self.vddio.is_some()
    }
}
