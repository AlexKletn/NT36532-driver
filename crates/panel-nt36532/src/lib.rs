#![no_std]

mod panel;
mod profile;

pub use panel::{
    DsiCommand, DsiMode, DsiPacket, Nt36532, PanelError, PanelIo, PanelMode, PanelProfile,
    ResetStep,
};
pub use profile::{
    DisplayMode, Ktz8866Profile, PanelLinkConfig, SELECTED_PROFILE, SelectedProfile,
};
