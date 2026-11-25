pub trait BootIo {
    type Error;

    fn write(&mut self, address: u32, data: &[u8]) -> Result<(), Self::Error>;
    fn read(&mut self, address: u32, data: &mut [u8]) -> Result<(), Self::Error>;
    fn delay_ms(&mut self, milliseconds: u16);
}
