use super::{AdapterError, Backlight, Ktz8866Adapter, Ktz8866Hardware, PanelPower};

#[derive(Default)]
struct FakeHardware {
    bias_enabled: bool,
    last_level: Option<u16>,
    fail_write: bool,
}

impl Ktz8866Hardware for FakeHardware {
    type Error = &'static str;

    fn enable_bias(&mut self) -> Result<(), Self::Error> {
        self.bias_enabled = true;
        Ok(())
    }

    fn disable_bias(&mut self) -> Result<(), Self::Error> {
        self.bias_enabled = false;
        Ok(())
    }

    fn apply_level(&mut self, level: u16) -> Result<(), Self::Error> {
        if self.fail_write {
            return Err("I2C write failed");
        }
        self.last_level = Some(level);
        Ok(())
    }
}

#[test]
fn blanking_preserves_bias_and_power_off_invalidates_level() {
    let mut adapter = Ktz8866Adapter::new(FakeHardware::default());
    adapter.enable().unwrap();
    adapter.set_level(900).unwrap();
    adapter.set_level(0).unwrap();

    assert!(adapter.hardware.bias_enabled);
    assert_eq!(adapter.hardware.last_level, Some(0));
    assert_eq!(adapter.level(), Some(0));

    adapter.disable().unwrap();
    assert!(!adapter.hardware.bias_enabled);
    assert_eq!(adapter.level(), None);
}

#[test]
fn rejects_invalid_levels_and_forgets_uncertain_hardware_state() {
    let mut adapter = Ktz8866Adapter::new(FakeHardware::default());
    adapter.enable().unwrap();
    adapter.set_level(900).unwrap();

    assert_eq!(
        adapter.set_level(2048),
        Err(AdapterError::LevelOutOfRange {
            level: 2048,
            max_level: 2047
        })
    );
    assert_eq!(adapter.hardware.last_level, Some(900));
    assert_eq!(adapter.level(), Some(900));

    adapter.hardware.fail_write = true;
    assert_eq!(
        adapter.set_level(1000),
        Err(AdapterError::Hardware("I2C write failed"))
    );
    assert_eq!(adapter.level(), None);
}
