// SPDX-License-Identifier: GPL-2.0-only

use core::{ffi::c_void, marker::PhantomData, ptr::NonNull};

use kernel::{
    bindings,
    error::{Error, code::EINVAL, from_err_ptr},
};
use panel_touch::{Contact, InputSink, Pen};

unsafe extern "C" {
    fn nt36532_input_create(
        device: *mut c_void,
        with_pen: bool,
        double_pen_resolution: bool,
        limits: *mut u16,
    ) -> *mut c_void;
    fn nt36532_input_contact(input: *mut c_void, slot: u8, x: u16, y: u16, width: u8);
    fn nt36532_input_finish_contacts(input: *mut c_void);
    fn nt36532_input_pen_position(input: *mut c_void, x: u16, y: u16, tilt_x: i8, tilt_y: i8);
    fn nt36532_input_pen_state(
        input: *mut c_void,
        pressure: u16,
        distance: u16,
        stylus: bool,
        stylus2: bool,
        eraser: bool,
    );
}

pub(super) struct LinuxTouchInput<'a> {
    input: NonNull<c_void>,
    contact_max: [u16; 2],
    pen_max: Option<[u16; 2]>,
    device_lifetime: PhantomData<&'a bindings::device>,
}

impl LinuxTouchInput<'_> {
    /// # Safety
    ///
    /// Call from probe in a sleepable context. The device's managed resources
    /// must remain live for the adapter's lifetime. Serialize event publishing.
    pub(super) unsafe fn register(
        device: NonNull<bindings::device>,
        with_pen: bool,
        double_pen_resolution: bool,
    ) -> Result<Self, Error> {
        let mut limits = [0; 4];
        // SAFETY: The caller provides a live device and limits has four entries.
        let input = from_err_ptr(unsafe {
            nt36532_input_create(
                device.as_ptr().cast(),
                with_pen,
                double_pen_resolution,
                limits.as_mut_ptr(),
            )
        })?;
        let [touch_x, touch_y, pen_x, pen_y] = limits;
        Ok(Self {
            input: NonNull::new(input).ok_or(EINVAL)?,
            contact_max: [touch_x, touch_y],
            pen_max: with_pen.then_some([pen_x, pen_y]),
            device_lifetime: PhantomData,
        })
    }

    pub(super) fn contact_max(&self) -> [u16; 2] {
        self.contact_max
    }

    pub(super) fn pen_max(&self) -> Option<[u16; 2]> {
        self.pen_max
    }
}

impl InputSink for LinuxTouchInput<'_> {
    fn contact(&mut self, contact: &Contact) {
        let Some(slot) = contact.id.checked_sub(1).filter(|&slot| slot < 10) else {
            return;
        };
        let [max_x, max_y] = self.contact_max;
        // SAFETY: The registered input device is live and the slot is in range.
        unsafe {
            nt36532_input_contact(
                self.input.as_ptr(),
                slot,
                contact.x.min(max_x),
                contact.y.min(max_y),
                contact.width,
            );
        }
    }

    fn finish_contacts(&mut self) {
        // SAFETY: The registered input device is live.
        unsafe { nt36532_input_finish_contacts(self.input.as_ptr()) };
    }

    fn pen(&mut self, pen: &Pen, eraser: bool) {
        let [stylus, stylus2] = pen.buttons;
        let Some([max_x, max_y]) = self.pen_max else {
            return;
        };
        // SAFETY: The input handle is live. The shim handles absent pen devices.
        unsafe {
            nt36532_input_pen_position(
                self.input.as_ptr(),
                pen.x.min(max_x),
                pen.y.min(max_y),
                pen.tilt_x,
                pen.tilt_y,
            );
            nt36532_input_pen_state(
                self.input.as_ptr(),
                pen.pressure,
                pen.distance,
                stylus,
                stylus2,
                eraser,
            );
        }
    }

    fn release_pen(&mut self) {
        // SAFETY: The input handle is live. The shim handles absent pen devices.
        unsafe { nt36532_input_pen_state(self.input.as_ptr(), 0, 0, false, false, false) };
    }
}
