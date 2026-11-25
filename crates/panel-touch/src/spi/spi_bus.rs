pub trait SpiBus {
    type Error;

    fn write(&mut self, tx: &[u8]) -> Result<(), Self::Error>;
    fn exchange(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), Self::Error>;
    fn delay_ms(&mut self, milliseconds: u16);
}
