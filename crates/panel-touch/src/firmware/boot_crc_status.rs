#[derive(Debug, PartialEq, Eq)]
pub struct BootCrcStatus {
    pub expected_ilm: u32,
    pub expected_dlm: u32,
    pub actual_ilm: Option<u32>,
    pub actual_dlm: Option<u32>,
    pub flags: Option<u8>,
}
