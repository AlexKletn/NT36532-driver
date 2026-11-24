pub trait Ktz8866Hardware {
    type Error;

    fn enable_bias(&mut self) -> Result<(), Self::Error>;
    fn disable_bias(&mut self) -> Result<(), Self::Error>;

    /// Zero turns off the LEDs while keeping the panel bias available.
    fn apply_level(&mut self, level: u16) -> Result<(), Self::Error>;
}
