#[derive(Clone, Copy)]
pub enum DsiPacket<'a> {
    DcsShort(u8),
    DcsLong(&'a [u8]),
}
