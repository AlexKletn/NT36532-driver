use super::*;
use crate::{Contact, Pen};

#[derive(Default)]
struct Sink {
    frames: usize,
    releases: usize,
    eraser: bool,
}

impl InputSink for Sink {
    fn contact(&mut self, _: &Contact) {}
    fn finish_contacts(&mut self) {
        self.frames += 1;
    }
    fn pen(&mut self, _: &Pen, eraser: bool) {
        self.eraser = eraser;
    }
    fn release_pen(&mut self) {
        self.releases += 1;
    }
}

#[test]
fn releases_pen_on_missing_frames_timeout_and_reset() {
    let mut state = InputState::default();
    let mut sink = Sink::default();
    let pen_report = InputReport::Input {
        contacts: [None; 10],
        pen: Some(Pen {
            x: 0,
            y: 0,
            pressure: 1,
            distance: 0,
            tilt_x: 0,
            tilt_y: 0,
            buttons: [false; 2],
        }),
    };
    let empty = InputReport::Input {
        contacts: [None; 10],
        pen: None,
    };
    state.publish(&pen_report, 10, &mut sink);
    for _ in 0..7 {
        state.publish(&empty, 20, &mut sink);
    }
    assert_eq!(sink.releases, 0);
    state.publish(&empty, 20, &mut sink);
    assert_eq!(sink.releases, 1);

    state.publish(&pen_report, 30, &mut sink);
    assert_eq!(state.check_pen_timeout(149, &mut sink), Some(1));
    assert_eq!(state.check_pen_timeout(150, &mut sink), None);
    assert_eq!(sink.releases, 2);

    state.publish(&pen_report, 160, &mut sink);
    let frames = sink.frames;
    state.publish(&InputReport::FirmwareReset, 161, &mut sink);
    assert_eq!(sink.frames, frames + 1);
    assert_eq!(sink.releases, 3);

    assert!(!state.toggle_pen_tool(&mut sink));
    state.publish(&pen_report, 170, &mut sink);
    assert!(!sink.eraser);
    assert!(state.toggle_pen_tool(&mut sink));
    assert_eq!(sink.releases, 4);
    state.publish(&pen_report, 171, &mut sink);
    assert!(sink.eraser);
}
