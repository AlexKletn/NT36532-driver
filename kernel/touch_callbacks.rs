// SPDX-License-Identifier: GPL-2.0-only

use core::{
    ffi::c_void,
    ptr::NonNull,
    sync::atomic::{AtomicPtr, Ordering},
};

use kernel::prelude::*;

use super::{
    linux_touch_firmware::LinuxTouchStartError, linux_touch_irq::LinuxTouchIrq,
    linux_touch_panel::LinuxTouchPanel, linux_touch_session::LinuxTouchSession,
};

unsafe extern "C" {
    fn nt36532_touch_take_preserve_request() -> bool;
}

pub(super) struct TouchCallbacks {
    pub(super) session: Pin<KBox<LinuxTouchSession<'static>>>,
    pub(super) irq: AtomicPtr<LinuxTouchIrq<'static>>,
    pub(super) follower: AtomicPtr<c_void>,
}

impl TouchCallbacks {
    unsafe fn context<'a>(context: *mut c_void) -> &'a Self {
        // SAFETY: The owning device keeps the context live through callback drain.
        unsafe { &*context.cast::<Self>() }
    }

    fn irq(&self) -> &LinuxTouchIrq<'static> {
        let irq = self.irq.load(Ordering::Acquire);
        // SAFETY: The IRQ is published before the follower is registered and
        // outlives the follower and all of its callbacks.
        unsafe { &*irq }
    }

    pub(super) unsafe extern "C" fn start(context: *mut c_void, follower: *mut c_void) {
        // SAFETY: Both arguments are provided by the live DRM follower.
        let callbacks = unsafe { Self::context(context) };
        callbacks.follower.store(follower, Ordering::Release);
        // SAFETY: The worker runs independently of the session IRQ callbacks.
        match unsafe { callbacks.session.start(callbacks.irq()) } {
            Ok(Some(error)) => pr_warn!("NT36532 pen initialization failed: {:?}\n", error),
            Err(LinuxTouchStartError::Request(error)) => {
                pr_err!("NT36532 firmware request failed: {:?}\n", error)
            }
            Err(LinuxTouchStartError::Image(error)) => {
                pr_err!("NT36532 firmware image rejected: {:?}\n", error)
            }
            Err(LinuxTouchStartError::Boot(error)) => {
                pr_err!("NT36532 touch boot failed: {:?}\n", error)
            }
            Ok(None) => {}
        }
    }

    pub(super) unsafe extern "C" fn stop(context: *mut c_void) {
        // SAFETY: DRM unregister drains its callbacks before freeing this context.
        let callbacks = unsafe { Self::context(context) };
        // SAFETY: This is the independent DRM callback and holds no session locks.
        unsafe {
            let preserve = nt36532_touch_take_preserve_request();
            callbacks.session.stop(callbacks.irq(), preserve);
        };
        callbacks
            .follower
            .store(core::ptr::null_mut(), Ordering::Release);
    }

    pub(super) unsafe extern "C" fn interrupt(context: *mut c_void) {
        // SAFETY: The registered interrupt owns a live callback context.
        let callbacks = unsafe { Self::context(context) };
        if matches!(callbacks.session.interrupt(callbacks.irq()), Ok(true))
            && let Some(follower) = NonNull::new(callbacks.follower.load(Ordering::Acquire))
        {
            // SAFETY: Follower shutdown drains this IRQ before freeing the handle.
            unsafe { LinuxTouchPanel::request_reload(follower) };
        }
    }

    pub(super) unsafe extern "C" fn watchdog(context: *mut c_void) {
        // SAFETY: The watchdog work is drained before the context is dropped.
        let callbacks = unsafe { Self::context(context) };
        callbacks.session.watchdog(callbacks.irq());
    }
}
