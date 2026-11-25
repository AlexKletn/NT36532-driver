#[derive(Debug, PartialEq, Eq)]
pub struct ResetHistory {
    pub primary: Option<[u8; 64]>,
    pub secondary: Option<[u8; 64]>,
}
