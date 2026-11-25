// SPDX-License-Identifier: GPL-2.0-only

use core::ffi::{c_int, c_void};

use kernel::error::code::EINVAL;

use crate::{DisplayMode, SELECTED_PROFILE};

unsafe extern "C" {
    fn nt36532_drm_add_mode(
        connector: *mut c_void,
        clock_khz: c_int,
        horizontal: *const u16,
        vertical: *const u16,
        preferred: bool,
    ) -> c_int;
}

#[unsafe(no_mangle)]
unsafe extern "C" fn nt36532_panel_add_modes(connector: *mut c_void) -> c_int {
    let Some(profile) = &SELECTED_PROFILE else {
        return 0;
    };
    let mut count = 0;
    let preferred = profile.preferred_refresh_hz;
    for mode in profile
        .modes
        .iter()
        .filter(|mode| mode.refresh_hz == preferred)
        .chain(
            profile
                .modes
                .iter()
                .filter(|mode| mode.refresh_hz != preferred),
        )
    {
        let Some((clock_khz, horizontal, vertical)) = mode_timings(mode) else {
            return if count == 0 { EINVAL.to_errno() } else { count };
        };
        // SAFETY: DRM supplies the live connector and the C helper copies the timings.
        let result = unsafe {
            nt36532_drm_add_mode(
                connector,
                clock_khz,
                horizontal.as_ptr(),
                vertical.as_ptr(),
                mode.refresh_hz == preferred,
            )
        };
        if result < 0 {
            return if count == 0 { result } else { count };
        }
        count += result;
    }
    count
}

fn axis(active: u16, front: u16, sync: u16, back: u16) -> Option<[u16; 4]> {
    let start = active.checked_add(front)?;
    let end = start.checked_add(sync)?;
    Some([active, start, end, end.checked_add(back)?])
}

fn mode_timings(mode: &DisplayMode) -> Option<(c_int, [u16; 4], [u16; 4])> {
    let horizontal = axis(mode.hdisplay, mode.hfront, mode.hsync, mode.hback)?;
    let vertical = axis(mode.vdisplay, mode.vfront, mode.vsync, mode.vback)?;
    let clock = u64::from(horizontal[3]) * u64::from(vertical[3]) * u64::from(mode.refresh_hz);
    let clock_khz = c_int::try_from(clock / 1000).ok()?;
    (clock_khz > 0).then_some((clock_khz, horizontal, vertical))
}
