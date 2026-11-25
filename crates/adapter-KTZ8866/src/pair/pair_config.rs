use core::num::NonZeroU32;

use ktz8866_driver::{BacklightConfig, BiasConfig};

pub struct PairConfig {
    pub led_only: BacklightConfig,
    pub led_and_bias: BacklightConfig,
    pub bias: BiasConfig,
    pub settle_after_enable_ms: Option<NonZeroU32>,
}
