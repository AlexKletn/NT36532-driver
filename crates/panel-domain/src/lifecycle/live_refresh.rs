use super::Panel;

pub trait LiveRefresh: Panel {
    fn set_live_refresh(&mut self, mode: &Self::Mode) -> Result<(), Self::Error>;
}
