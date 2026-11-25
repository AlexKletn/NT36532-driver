#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Partition {
    pub file_offset: u32,
    pub sram_address: u32,
    pub byte_count: u32,
    pub crc: u32,
}
