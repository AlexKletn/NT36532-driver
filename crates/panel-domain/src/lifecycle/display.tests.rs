extern crate std;

use core::cell::{Cell, RefCell};
use std::{rc::Rc, vec::Vec};

use super::{Backlight, Display, DisplayError, DisplayState, Panel, PanelPower};

type Trace = Rc<RefCell<Vec<Operation>>>;

#[derive(Debug, PartialEq, Eq)]
enum Operation {
    VddioOn,
    BiasOn,
    Prepare(u16),
    Brightness(u16),
    Unprepare,
    BiasOff,
    VddioOff,
}

struct TestPanel {
    trace: Trace,
    fail_prepare: bool,
}

impl Panel for TestPanel {
    type Error = &'static str;
    type Mode = u16;

    fn enable_vddio(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(Operation::VddioOn);
        Ok(())
    }

    fn prepare(&mut self, mode: &Self::Mode) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(Operation::Prepare(*mode));
        if self.fail_prepare {
            return Err("DSI command failed");
        }
        Ok(())
    }

    fn unprepare(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(Operation::Unprepare);
        Ok(())
    }

    fn disable_vddio(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(Operation::VddioOff);
        Ok(())
    }
}

struct TestBacklight {
    trace: Trace,
    fail_bias_off: Rc<Cell<bool>>,
}

impl Backlight for TestBacklight {
    type Error = &'static str;

    fn set_level(&mut self, level: u16) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(Operation::Brightness(level));
        Ok(())
    }

    fn level(&self) -> Option<u16> {
        None
    }

    fn max_level(&self) -> u16 {
        2047
    }
}

impl PanelPower for TestBacklight {
    type Error = &'static str;

    fn enable(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(Operation::BiasOn);
        Ok(())
    }

    fn disable(&mut self) -> Result<(), Self::Error> {
        self.trace.borrow_mut().push(Operation::BiasOff);
        if self.fail_bias_off.get() {
            return Err("bias switch failed");
        }
        Ok(())
    }
}

#[test]
fn display_lifecycle_keeps_logic_power_up_until_after_bias_is_removed() {
    use Operation::*;
    let (mut display, trace, _) = display(false);
    assert_eq!(
        display.set_brightness(1),
        Err(DisplayError::InvalidState(DisplayState::Off))
    );

    display.prepare(&120).unwrap();
    display.set_brightness(900).unwrap();
    display.set_brightness(0).unwrap();
    assert_eq!(display.state(), DisplayState::Prepared);
    assert_eq!(trace.borrow().last(), Some(&Brightness(0)));
    display.unprepare().unwrap();

    assert_eq!(display.state(), DisplayState::Off);
    assert_eq!(
        trace.borrow().as_slice(),
        [
            VddioOn,
            BiasOn,
            Prepare(120),
            Brightness(900),
            Brightness(0),
            Brightness(0),
            Unprepare,
            BiasOff,
            VddioOff
        ]
    );
}

#[test]
fn failed_prepare_rolls_back_and_returns_the_original_error() {
    use Operation::*;
    let (mut display, trace, _) = display(true);

    let error = display.prepare(&60).unwrap_err();
    assert!(
        matches!(error, DisplayError::PreparePanel { source: "DSI command failed", rollback }
        if !rollback.has_errors())
    );
    assert_eq!(display.state(), DisplayState::Off);
    assert_eq!(
        trace.borrow().as_slice(),
        [VddioOn, BiasOn, Prepare(60), Unprepare, BiasOff, VddioOff]
    );
}

#[test]
fn failed_bias_shutdown_keeps_vddio_and_retries_only_pending_operations() {
    use Operation::*;
    let (mut display, trace, fail_bias_off) = display(false);
    display.prepare(&120).unwrap();
    fail_bias_off.set(true);

    let error = display.unprepare().unwrap_err();
    assert!(
        matches!(error, DisplayError::Shutdown(errors) if errors.bias == Some("bias switch failed"))
    );
    assert_eq!(display.state(), DisplayState::RecoveryRequired);
    assert_eq!(trace.borrow().last(), Some(&BiasOff));
    assert_eq!(
        display.prepare(&120),
        Err(DisplayError::InvalidState(DisplayState::RecoveryRequired))
    );

    fail_bias_off.set(false);
    trace.borrow_mut().clear();
    display.unprepare().unwrap();
    assert_eq!(display.state(), DisplayState::Off);
    assert_eq!(trace.borrow().as_slice(), [BiasOff, VddioOff]);
}

fn display(fail_prepare: bool) -> (Display<TestPanel, TestBacklight>, Trace, Rc<Cell<bool>>) {
    let trace = Trace::default();
    let fail_bias_off = Rc::new(Cell::new(false));
    let panel = TestPanel {
        trace: trace.clone(),
        fail_prepare,
    };
    let backlight = TestBacklight {
        trace: trace.clone(),
        fail_bias_off: fail_bias_off.clone(),
    };
    (Display::new(panel, backlight), trace, fail_bias_off)
}
