use crate::{BootIo, SpiBus, SpiError};

const PAGE_SELECT: u8 = 0xff;
const WRITE_BIT: u8 = 0x80;
const OFFSET_MASK: u8 = 0x7f;
const READ_PREFIX_BYTES: usize = 2;
const MAX_PAYLOAD_BYTES: usize = 1024;
const ADDRESS_SPACE_BYTES: u32 = 1 << 23;

pub struct PagedSpi<B> {
    bus: B,
    tx: [u8; MAX_PAYLOAD_BYTES + READ_PREFIX_BYTES],
    rx: [u8; MAX_PAYLOAD_BYTES + READ_PREFIX_BYTES],
}

impl<B: SpiBus> PagedSpi<B> {
    pub fn new(bus: B) -> Self {
        Self {
            bus,
            tx: [0; MAX_PAYLOAD_BYTES + READ_PREFIX_BYTES],
            rx: [0; MAX_PAYLOAD_BYTES + READ_PREFIX_BYTES],
        }
    }

    fn select_page(&mut self, address: u32, count: usize) -> Result<u8, SpiError<B::Error>> {
        if count > MAX_PAYLOAD_BYTES {
            return Err(SpiError::TransferTooLong);
        }
        if address >= ADDRESS_SPACE_BYTES || count as u32 > ADDRESS_SPACE_BYTES - address {
            return Err(SpiError::InvalidAddress);
        }
        self.bus
            .write(&[PAGE_SELECT, (address >> 15) as u8, (address >> 7) as u8])
            .map_err(SpiError::Transport)?;
        Ok(address as u8 & OFFSET_MASK)
    }
}

impl<B: SpiBus> BootIo for PagedSpi<B> {
    type Error = SpiError<B::Error>;

    fn write(&mut self, address: u32, data: &[u8]) -> Result<(), Self::Error> {
        let offset = self.select_page(address, data.len())?;
        let end = data.len() + 1;
        self.tx[0] = offset | WRITE_BIT;
        self.tx[1..end].copy_from_slice(data);
        self.bus.write(&self.tx[..end]).map_err(SpiError::Transport)
    }

    fn read(&mut self, address: u32, data: &mut [u8]) -> Result<(), Self::Error> {
        let offset = self.select_page(address, data.len())?;
        let end = data.len() + READ_PREFIX_BYTES;
        self.tx[..end].fill(0);
        self.tx[0] = offset;
        self.bus
            .exchange(&self.tx[..end], &mut self.rx[..end])
            .map_err(SpiError::Transport)?;
        data.copy_from_slice(&self.rx[READ_PREFIX_BYTES..end]);
        Ok(())
    }

    fn delay_ms(&mut self, milliseconds: u16) {
        self.bus.delay_ms(milliseconds);
    }
}

#[cfg(test)]
#[path = "paged_spi.tests.rs"]
mod tests;
