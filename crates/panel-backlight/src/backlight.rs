pub trait Backlight {
    type Error;

    fn set_level(&mut self, level: u16) -> Result<(), Self::Error>;
    fn level(&self) -> Option<u16>;
    fn max_level(&self) -> u16;
}
