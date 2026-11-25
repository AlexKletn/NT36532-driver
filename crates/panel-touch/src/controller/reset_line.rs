pub trait ResetLine {
    type Error;

    fn set_asserted(&mut self, asserted: bool) -> Result<(), Self::Error>;
}
