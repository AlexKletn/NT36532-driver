use ktz8866_driver::{Delay, Ktz8866, RegisterBus};

use crate::Ktz8866Hardware;

use super::{PairConfig, PairError};

pub struct Ktz8866Pair<L, B, D> {
    led_only: Ktz8866<L>,
    led_and_bias: Ktz8866<B>,
    delay: D,
    config: PairConfig,
}

impl<L, B, D> Ktz8866Pair<L, B, D> {
    pub fn new(
        led_only: Ktz8866<L>,
        led_and_bias: Ktz8866<B>,
        delay: D,
        config: PairConfig,
    ) -> Self {
        Self {
            led_only,
            led_and_bias,
            delay,
            config,
        }
    }
}

impl<L: RegisterBus, B: RegisterBus, D: Delay> Ktz8866Hardware for Ktz8866Pair<L, B, D> {
    type Error = PairError<L::Error, B::Error>;

    fn enable_bias(&mut self) -> Result<(), Self::Error> {
        self.led_and_bias
            .configure_backlight(&self.config.led_and_bias)
            .map_err(PairError::LedAndBias)?;

        if let Err(source) = self
            .led_and_bias
            .enable_bias(&self.config.bias, &mut self.delay)
        {
            let rollback = self
                .led_and_bias
                .disable_bias(&self.config.bias, &mut self.delay)
                .err();
            return Err(PairError::BiasEnable { source, rollback });
        }

        if let Err(source) = self.led_only.configure_backlight(&self.config.led_only) {
            let rollback = self
                .led_and_bias
                .disable_bias(&self.config.bias, &mut self.delay)
                .err();
            return Err(PairError::LedOnlyConfiguration { source, rollback });
        }

        if let Some(settle_ms) = self.config.settle_after_enable_ms {
            self.delay.delay_ms(settle_ms);
        }
        Ok(())
    }

    fn disable_bias(&mut self) -> Result<(), Self::Error> {
        self.led_and_bias
            .disable_bias(&self.config.bias, &mut self.delay)
            .map_err(PairError::LedAndBias)
    }

    fn apply_level(&mut self, level: u16) -> Result<(), Self::Error> {
        // Stage both levels first so the two halves do not light at different brightnesses.
        self.led_only
            .write_brightness(level)
            .map_err(PairError::LedOnly)?;
        self.led_and_bias
            .write_brightness(level)
            .map_err(PairError::LedAndBias)?;
        self.led_only
            .set_leds_enabled(level != 0)
            .map_err(PairError::LedOnly)?;
        self.led_and_bias
            .set_leds_enabled(level != 0)
            .map_err(PairError::LedAndBias)
    }
}

#[cfg(test)]
#[path = "ktz8866_pair.tests.rs"]
mod tests;
