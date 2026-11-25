use super::DsiCommand;

pub struct PanelMode<'a> {
    pub compression_pps: Option<[u8; 128]>,
    pub post_init: &'a [DsiCommand<'a>],
}
