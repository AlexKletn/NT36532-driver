use super::{BootCrcStatus, BootError, BootIo, FirmwareImage, Partition};

const SRAM_CHUNK_BYTES: usize = 1024;
const ADDRESS_SPACE_BYTES: u32 = 1 << 23;
const SOFTWARE_RESET: u32 = 0x1fb43e;
const CASCADE_COPY: u32 = 0x1fc925;
const BOOT_READY: u32 = 0x1fb50d;
const HOST_COMMAND: u32 = 0x125850;
const RESET_STATE: u32 = 0x125860;
const ILM_CRC_REGISTERS: [u32; 3] = [0x1fb528, 0x1fb518, 0x1fb500];
const DLM_CRC_REGISTERS: [u32; 3] = [0x1fb52c, 0x1fb530, 0x1fb504];

const ILM_CRC_RESULT: u32 = 0x1fb520;
const DLM_CRC_RESULT: u32 = 0x1fb524;
const CRC_ERROR_FLAGS: u32 = 0x1fb535;

pub struct Bootloader<I> {
    io: I,
}

impl<I: BootIo> Bootloader<I> {
    pub fn new(io: I) -> Self {
        Self { io }
    }

    pub(crate) fn io_mut(&mut self) -> &mut I {
        &mut self.io
    }

    pub fn into_inner(self) -> I {
        self.io
    }

    pub fn upload(&mut self, image: &FirmwareImage<'_>) -> Result<(), BootError<I::Error>> {
        let partitions = image.partitions();
        if partitions
            .iter()
            .any(|partition| partition.sram_address + partition.byte_count > ADDRESS_SPACE_BYTES)
        {
            return Err(BootError::InvalidSramRange);
        }

        self.io.write(SOFTWARE_RESET, &[0x69])?;
        self.io.delay_ms(5);
        self.write_crc_bank(ILM_CRC_REGISTERS, &partitions[0])?;
        self.write_crc_bank(DLM_CRC_REGISTERS, &partitions[1])?;

        if image.is_cascade() {
            self.io.write(CASCADE_COPY, &[0x56])?;
        }

        for (index, partition) in partitions.iter().enumerate() {
            let mut address = partition.sram_address;
            let data = image
                .partition_data(index)
                .expect("validated partition range");
            for chunk in data.chunks(SRAM_CHUNK_BYTES) {
                self.io.write(address, chunk)?;
                address += chunk.len() as u32;
            }
        }

        if image.is_cascade() && !self.poll(CASCADE_COPY, 200, 1, |state| state == 0)? {
            return Err(BootError::CascadeCopyTimeout);
        }

        self.io.write(RESET_STATE, &[0; 6])?;
        self.io.write(HOST_COMMAND, &[0xae, 0])?;
        self.io.write(BOOT_READY, &[1])?;
        self.io.delay_ms(5);
        if !self.poll(RESET_STATE, 50, 10, |state| (0xa0..=0xaf).contains(&state))? {
            return Err(BootError::FirmwareReadyTimeout {
                crc: self.crc_status(partitions),
            });
        }
        Ok(())
    }

    pub fn enable_pen(&mut self, support: u8, pen_type: u8) -> Result<(), BootError<I::Error>> {
        self.io.write(HOST_COMMAND, &[0x7b, support, pen_type])?;
        for _ in 0..20 {
            self.io.delay_ms(20);
            let mut status = [0];
            self.io.read(HOST_COMMAND, &mut status)?;
            if status[0] == 0 {
                return Ok(());
            }
        }
        Err(BootError::PenCommandTimeout)
    }

    fn crc_status(&mut self, partitions: &[Partition]) -> BootCrcStatus {
        let actual_ilm = self.read_diagnostic(ILM_CRC_RESULT).map(u32::from_le_bytes);
        let actual_dlm = self.read_diagnostic(DLM_CRC_RESULT).map(u32::from_le_bytes);
        let flags = self
            .read_diagnostic::<2>(CRC_ERROR_FLAGS)
            .map(|bytes| bytes[0]);
        BootCrcStatus {
            expected_ilm: partitions[0].crc,
            expected_dlm: partitions[1].crc,
            actual_ilm,
            actual_dlm,
            flags,
        }
    }

    fn read_diagnostic<const N: usize>(&mut self, address: u32) -> Option<[u8; N]> {
        let mut bytes = [0; N];
        self.io.read(address, &mut bytes).ok()?;
        Some(bytes)
    }

    fn write_crc_bank(
        &mut self,
        [destination, length, checksum]: [u32; 3],
        partition: &Partition,
    ) -> Result<(), I::Error> {
        self.io
            .write(destination, &partition.sram_address.to_le_bytes()[..3])?;
        self.io
            .write(length, &(partition.byte_count - 1).to_le_bytes()[..3])?;
        self.io.write(checksum, &partition.crc.to_le_bytes())
    }

    fn poll(
        &mut self,
        address: u32,
        attempts: u16,
        delay_ms: u16,
        ready: impl Fn(u8) -> bool,
    ) -> Result<bool, I::Error> {
        for _ in 0..attempts {
            let mut status = [0; 2];
            self.io.read(address, &mut status)?;
            if ready(status[0]) {
                return Ok(true);
            }
            self.io.delay_ms(delay_ms);
        }
        Ok(false)
    }
}

#[cfg(test)]
#[path = "bootloader.tests.rs"]
mod tests;
