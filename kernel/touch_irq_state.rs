// SPDX-License-Identifier: GPL-2.0-only

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TouchIrqState {
    Offline,
    Enabled,
    FirmwareRetained,
    Paused,
    Failed,
}
