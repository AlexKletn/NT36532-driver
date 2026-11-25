// SPDX-License-Identifier: GPL-2.0-only

use kernel::error::Error;

use crate::{DisplayError, display_panel::PanelError};

#[derive(Debug)]
pub(super) enum LinuxPanelSessionError<B> {
    InvalidMode,
    Links(Error),
    Display(DisplayError<PanelError<Error>, B>),
}
