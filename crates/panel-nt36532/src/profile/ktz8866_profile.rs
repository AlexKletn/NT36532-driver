#[derive(Clone, Copy)]
pub struct Ktz8866Profile {
    pub sink_mask: u8,
    pub led_config_1: u8,
    pub led_config_2: u8,
    pub dimming: u8,
    pub led_option_2: u8,
    pub pwm_ramp: u8,
    pub boost: u8,
    pub positive_bias: u8,
    pub negative_bias: u8,
    pub bias_step_ms: u16,
    pub settle_on_ms: u16,
    pub settle_off_ms: u16,
}
