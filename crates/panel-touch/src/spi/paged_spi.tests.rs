use super::*;

#[derive(Default)]
struct TestBus {
    writes: usize,
}

impl SpiBus for TestBus {
    type Error = ();

    fn write(&mut self, tx: &[u8]) -> Result<(), ()> {
        self.writes += 1;
        match self.writes {
            1 | 3 => assert_eq!(tx, [0xff, 0x24, 0xb0]),
            2 => assert_eq!(tx, [0xd0, 0x7b, 1, 0]),
            _ => panic!("unexpected write"),
        }
        Ok(())
    }

    fn exchange(&mut self, tx: &[u8], rx: &mut [u8]) -> Result<(), ()> {
        assert_eq!(self.writes, 3);
        assert_eq!(tx, [0x50, 0, 0, 0]);
        rx.copy_from_slice(&[0xff, 0xff, 0x12, 0x34]);
        Ok(())
    }

    fn delay_ms(&mut self, _: u16) {}
}

#[test]
fn encodes_page_offsets_and_discards_read_prefix() {
    let mut spi = PagedSpi::new(TestBus::default());
    let address = 0x125850;
    spi.write(address, &[0x7b, 1, 0]).unwrap();
    let mut data = [0; 2];
    spi.read(address, &mut data).unwrap();
    assert_eq!(data, [0x12, 0x34]);
    assert_eq!(spi.write(1 << 23, &[1]), Err(SpiError::InvalidAddress));
    assert_eq!(
        spi.write(address, &[0; 1025]),
        Err(SpiError::TransferTooLong)
    );
    assert_eq!(spi.bus.writes, 3);
}
