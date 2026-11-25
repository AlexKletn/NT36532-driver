extern crate std;

use std::vec::Vec;

use super::{DsiCommand, DsiPacket, Nt36532, Panel, PanelError, PanelIo, PanelMode, PanelProfile};
use crate::{DsiMode, ResetStep};

#[derive(Debug, PartialEq, Eq)]
enum Operation {
    Power(bool),
    Reset(bool),
    Wait(u16),
    Short(u8, DsiMode),
    Long(Vec<u8>, DsiMode),
    Pps(u8),
}

#[derive(Default)]
struct FakeIo {
    trace: Vec<Operation>,
    fail_shutdown: bool,
}

impl PanelIo for FakeIo {
    type Error = &'static str;

    fn enable_vddio(&mut self) -> Result<(), Self::Error> {
        self.trace.push(Operation::Power(true));
        Ok(())
    }

    fn disable_vddio(&mut self) -> Result<(), Self::Error> {
        self.trace.push(Operation::Power(false));
        Ok(())
    }

    fn set_reset(&mut self, asserted: bool) -> Result<(), Self::Error> {
        self.trace.push(Operation::Reset(asserted));
        Ok(())
    }

    fn delay_ms(&mut self, milliseconds: u16) {
        self.trace.push(Operation::Wait(milliseconds));
    }

    fn write_dsi(&mut self, packet: DsiPacket<'_>, mode: DsiMode) -> Result<(), Self::Error> {
        match packet {
            DsiPacket::DcsShort(command) => {
                self.trace.push(Operation::Short(command, mode));
                if command == 0x28 && self.fail_shutdown {
                    return Err("DSI write failed");
                }
            }
            DsiPacket::DcsLong(payload) => self.trace.push(Operation::Long(payload.into(), mode)),
        }
        Ok(())
    }

    fn send_dsc_pps(&mut self, pps: &[u8; 128]) -> Result<(), Self::Error> {
        self.trace.push(Operation::Pps(pps[0]));
        Ok(())
    }
}

#[test]
fn cold_start_sends_init_then_pps_then_mode_commands() {
    use Operation::*;

    let mut panel = Nt36532::new(FakeIo::default(), profile()).unwrap();
    let mode = PanelMode {
        compression_pps: Some([17; 128]),
        post_init: &[DsiCommand {
            packet: DsiPacket::DcsLong(&[0xb3, 0]),
            mode: DsiMode::LowPower,
            delay_after_ms: 0,
        }],
    };
    panel.enable_vddio().unwrap();
    panel.prepare(&mode).unwrap();

    assert_eq!(
        panel.io.trace,
        [
            Power(true),
            Wait(20),
            Reset(true),
            Wait(10),
            Reset(false),
            Wait(10),
            Long([0xff, 0x10].into(), DsiMode::LowPower),
            Short(0x11, DsiMode::LowPower),
            Wait(120),
            Pps(17),
            Long([0xb3, 0].into(), DsiMode::LowPower),
        ]
    );
}

#[test]
fn shutdown_asserts_reset_after_a_dsi_error_and_allows_cleanup_to_continue() {
    use Operation::*;

    let mut panel = Nt36532::new(
        FakeIo {
            fail_shutdown: true,
            ..Default::default()
        },
        profile(),
    )
    .unwrap();
    panel
        .prepare(&PanelMode {
            compression_pps: None,
            post_init: &[],
        })
        .unwrap();
    panel.io.trace.clear();

    assert_eq!(
        panel.unprepare(),
        Err(PanelError::Shutdown {
            commands: Some("DSI write failed"),
            reset: None
        })
    );
    panel.unprepare().unwrap();
    panel.disable_vddio().unwrap();

    assert_eq!(
        panel.io.trace,
        [Short(0x28, DsiMode::HighSpeed), Reset(true), Power(false)]
    );
}

#[test]
fn rejects_a_malformed_mode_packet_before_toggling_reset() {
    let mut panel = Nt36532::new(FakeIo::default(), profile()).unwrap();
    let mode = PanelMode {
        compression_pps: None,
        post_init: &[DsiCommand {
            packet: DsiPacket::DcsLong(&[]),
            mode: DsiMode::LowPower,
            delay_after_ms: 0,
        }],
    };
    assert_eq!(panel.prepare(&mode), Err(PanelError::InvalidLongPacket));
    assert!(panel.io.trace.is_empty());
}

fn profile() -> PanelProfile<'static> {
    PanelProfile {
        vddio_settle_ms: 20,
        reset: &[
            ResetStep {
                asserted: true,
                delay_after_ms: 10,
            },
            ResetStep {
                asserted: false,
                delay_after_ms: 10,
            },
        ],
        init: &[
            DsiCommand {
                packet: DsiPacket::DcsLong(&[0xff, 0x10]),
                mode: DsiMode::LowPower,
                delay_after_ms: 0,
            },
            DsiCommand {
                packet: DsiPacket::DcsShort(0x11),
                mode: DsiMode::LowPower,
                delay_after_ms: 120,
            },
        ],
        shutdown: &[DsiCommand {
            packet: DsiPacket::DcsShort(0x28),
            mode: DsiMode::HighSpeed,
            delay_after_ms: 20,
        }],
    }
}

#[test]
fn live_refresh_sends_only_mode_commands() {
    use panel_domain::LiveRefresh;

    let mut panel = Nt36532::new(FakeIo::default(), profile()).unwrap();
    panel
        .set_live_refresh(&PanelMode {
            compression_pps: Some([17; 128]),
            post_init: &[DsiCommand {
                packet: DsiPacket::DcsLong(&[0xb3, 0]),
                mode: DsiMode::LowPower,
                delay_after_ms: 0,
            }],
        })
        .unwrap();

    assert_eq!(
        panel.io.trace,
        [Operation::Long([0xb3, 0].into(), DsiMode::LowPower)]
    );
}
