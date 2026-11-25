use crate::{DisplayMode, Ktz8866Profile, PanelLinkConfig, PanelProfile};

pub struct SelectedProfile {
    pub panel: PanelProfile<'static>,
    pub link: Option<PanelLinkConfig>,
    pub ktz8866: Option<Ktz8866Profile>,
    pub modes: &'static [DisplayMode],
    pub preferred_refresh_hz: u16,
}

impl SelectedProfile {
    pub fn display_mode(&self, refresh_hz: u16) -> Option<&DisplayMode> {
        self.modes.iter().find(|mode| mode.refresh_hz == refresh_hz)
    }
}
