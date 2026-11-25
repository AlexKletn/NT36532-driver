pub trait Panel {
    type Error;
    type Mode;

    fn enable_vddio(&mut self) -> Result<(), Self::Error>;
    fn prepare(&mut self, mode: &Self::Mode) -> Result<(), Self::Error>;

    /// Cleanup operations can be retried after partially completed initialization.
    fn unprepare(&mut self) -> Result<(), Self::Error>;
    fn disable_vddio(&mut self) -> Result<(), Self::Error>;
}
