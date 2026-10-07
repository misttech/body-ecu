// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `ZephyrTimerService`: eight timer slots, each a `k_timer` whose expiry submits a work
//! item, so the callback runs on the system work queue. The shim owns the kernel objects
//! and calls `rust_timer_slot_fired(slot)`, which the application routes to
//! [`ZephyrTimerService::fired`].

use core::cell::{Cell, RefCell};

use body_ecu_ports::{INVALID_TIMER_ID, TimerCallback, TimerId, TimerService, printk};

/// How many timers run at once (`kMaxTimers`).
pub const MAX_TIMERS: usize = body_ecu_ffi::TIMER_SLOTS;

struct TimerSlot {
    callback: RefCell<Option<TimerCallback>>,
    active: Cell<bool>,
    periodic: Cell<bool>,
}

/// The timer service.
pub struct ZephyrTimerService {
    slots: [TimerSlot; MAX_TIMERS],
}

// SAFETY: slots are taken from the thread that starts a timer and fired from the system
// work queue, never at once, as the C++ service assumes with no lock.
unsafe impl Sync for ZephyrTimerService {}

impl ZephyrTimerService {
    /// A service with every slot free.
    pub const fn new() -> Self {
        Self {
            slots: [const {
                TimerSlot {
                    callback: RefCell::new(None),
                    active: Cell::new(false),
                    periodic: Cell::new(false),
                }
            }; MAX_TIMERS],
        }
    }

    fn start(&self, milliseconds: u32, periodic: bool, callback: TimerCallback) -> TimerId {
        for (i, slot) in self.slots.iter().enumerate() {
            if !slot.active.get() {
                slot.active.set(true);
                slot.periodic.set(periodic);
                *slot.callback.borrow_mut() = Some(callback);
                body_ecu_ffi::timer_slot_start(i as u32, milliseconds, periodic);
                return i as TimerId;
            }
        }
        printk!("[timer] ERROR: No free timer slots\n");
        INVALID_TIMER_ID
    }

    /// Timer `slot` fired: run its callback (the C++ `workHandler`), and free a one-shot.
    pub fn fired(&self, slot: u32) {
        let Some(slot) = self.slots.get(slot as usize) else { return };
        if !slot.active.get() {
            return;
        }
        if let Some(callback) = slot.callback.borrow().as_ref() {
            callback();
        }
        if !slot.periodic.get() {
            slot.active.set(false);
        }
    }
}

impl Default for ZephyrTimerService {
    fn default() -> Self {
        Self::new()
    }
}

impl TimerService for ZephyrTimerService {
    fn start_periodic(&self, interval_ms: u32, callback: TimerCallback) -> TimerId {
        self.start(interval_ms, true, callback)
    }

    fn start_one_shot(&self, delay_ms: u32, callback: TimerCallback) -> TimerId {
        self.start(delay_ms, false, callback)
    }

    fn cancel(&self, id: TimerId) {
        let idx = id as usize;
        if let Some(slot) = self.slots.get(idx)
            && slot.active.get()
        {
            body_ecu_ffi::timer_slot_stop(id);
            slot.active.set(false);
            *slot.callback.borrow_mut() = None;
        }
    }
}
