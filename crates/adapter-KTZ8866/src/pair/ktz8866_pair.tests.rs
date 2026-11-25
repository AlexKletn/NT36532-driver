extern crate std;

use core::{cell::RefCell, num::NonZeroU32};
use std::{rc::Rc, vec::Vec};

use ktz8866_driver::{BacklightConfig, BiasConfig, DriverError};

use super::{Delay, Ktz8866, Ktz8866Hardware, Ktz8866Pair, PairConfig, PairError, RegisterBus};

#[derive(Debug, PartialEq, Eq)]
enum Operation {
    Write(char, u8, u8),
    Delay(u32),
}

type Trace = Rc<RefCell<Vec<Operation>>>;

struct Bus {
    chip: char,
    trace: Trace,
    fail_register: Option<u8>,
}

impl RegisterBus for Bus {
    type Error = &'static str;

    fn write(&mut self, register: u8, value: u8) -> Result<(), Self::Error> {
        self.trace
            .borrow_mut()
            .push(Operation::Write(self.chip, register, value));
        if self.fail_register == Some(register) {
            return Err("write failed");
        }
        Ok(())
    }
}

struct RecordingDelay(Trace);

impl Delay for RecordingDelay {
    fn delay_ms(&mut self, milliseconds: NonZeroU32) {
        self.0
            .borrow_mut()
            .push(Operation::Delay(milliseconds.get()));
    }
}

#[test]
fn programs_bias_chip_first_and_stages_brightness_before_lighting_either_half() {
    use Operation::{Delay, Write};

    let (mut pair, trace) = pair(None);
    pair.enable_bias().unwrap();
    pair.apply_level(0x103).unwrap();

    assert_eq!(
        trace.borrow().as_slice(),
        [
            Write('B', 0x02, 0xdb),
            Write('B', 0x0c, 0x2e),
            Write('B', 0x0d, 0x24),
            Write('B', 0x0e, 0x24),
            Write('B', 0x09, 0x9c),
            Delay(5),
            Write('B', 0x09, 0x9e),
            Write('A', 0x02, 0xdb),
            Delay(10),
            Write('A', 0x04, 3),
            Write('A', 0x05, 32),
            Write('B', 0x04, 3),
            Write('B', 0x05, 32),
            Write('A', 0x08, 0x4f),
            Write('B', 0x08, 0x4f),
        ]
    );
}

#[test]
fn rolls_back_bias_if_the_led_only_chip_fails_to_configure() {
    use Operation::{Delay, Write};

    let (mut pair, trace) = pair(Some(0x02));

    assert_eq!(
        pair.enable_bias(),
        Err(PairError::LedOnlyConfiguration {
            source: DriverError::Bus("write failed"),
            rollback: None
        })
    );
    assert!(trace.borrow().ends_with(&[
        Write('A', 0x02, 0xdb),
        Write('B', 0x09, 0x9c),
        Delay(5),
        Write('B', 0x09, 0x98),
    ]));
}

fn pair(fail_register: Option<u8>) -> (Ktz8866Pair<Bus, Bus, RecordingDelay>, Trace) {
    let trace = Trace::default();
    let led_only = Ktz8866::new(
        Bus {
            chip: 'A',
            trace: trace.clone(),
            fail_register,
        },
        0x0f,
    )
    .unwrap();
    let led_and_bias = Ktz8866::new(
        Bus {
            chip: 'B',
            trace: trace.clone(),
            fail_register: None,
        },
        0x0f,
    )
    .unwrap();
    let backlight = || BacklightConfig {
        led_config_1: Some(0xdb),
        ..Default::default()
    };
    let config = PairConfig {
        led_only: backlight(),
        led_and_bias: backlight(),
        bias: BiasConfig {
            boost_register: 0x2e,
            positive_output_register: 0x24,
            negative_output_register: 0x24,
            step_delay_ms: NonZeroU32::new(5).unwrap(),
        },
        settle_after_enable_ms: NonZeroU32::new(10),
    };

    (
        Ktz8866Pair::new(
            led_only,
            led_and_bias,
            RecordingDelay(trace.clone()),
            config,
        ),
        trace,
    )
}
