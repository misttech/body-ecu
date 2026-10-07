// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `LocalSignalBus`: a lightweight in-process signal bus for the MCU, a store of values
//! and subscribers by path, guarded by the shim's `k_mutex`.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use body_ecu_ports::{SignalBus, SignalCallback, SignalValue};

/// The in-process signal bus.
pub struct LocalSignalBus {
    store: RefCell<BTreeMap<String, SignalValue>>,
    subscribers: RefCell<BTreeMap<String, Vec<SignalCallback>>>,
}

// SAFETY: every access to the maps happens under the shim's mutex, as the C++ guards
// them with its `k_mutex`; the callbacks run outside it, as the C++ runs its copies.
unsafe impl Sync for LocalSignalBus {}

impl LocalSignalBus {
    /// An empty bus.
    pub const fn new() -> Self {
        Self { store: RefCell::new(BTreeMap::new()), subscribers: RefCell::new(BTreeMap::new()) }
    }
}

impl Default for LocalSignalBus {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalBus for LocalSignalBus {
    fn publish(&self, path: &str, value: &SignalValue) -> bool {
        body_ecu_ffi::signal_bus_lock();
        self.store.borrow_mut().insert(path.to_string(), value.clone());
        body_ecu_ffi::signal_bus_unlock();
        // A callback may publish in turn: the subscribers are borrowed, not held exclusively.
        let subscribers = self.subscribers.borrow();
        if let Some(callbacks) = subscribers.get(path) {
            for callback in callbacks {
                callback(path, value);
            }
        }
        true
    }

    fn subscribe(&self, path: &str, callback: SignalCallback) {
        body_ecu_ffi::signal_bus_lock();
        self.subscribers.borrow_mut().entry(path.to_string()).or_default().push(callback);
        body_ecu_ffi::signal_bus_unlock();
    }

    fn get(&self, path: &str) -> Option<SignalValue> {
        body_ecu_ffi::signal_bus_lock();
        let value = self.store.borrow().get(path).cloned();
        body_ecu_ffi::signal_bus_unlock();
        value
    }
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use core::cell::Cell;

    use body_ecu_ports::mock::leak;

    use super::*;

    #[test]
    fn a_published_value_is_stored_and_delivered() {
        let bus = leak(LocalSignalBus::new());
        let seen = leak(Cell::new(0));
        bus.subscribe(
            "Vehicle.Speed",
            Box::new(move |path, value| {
                assert_eq!(path, "Vehicle.Speed");
                assert_eq!(*value, SignalValue::Float(3.0));
                seen.set(seen.get() + 1);
            }),
        );
        assert!(bus.publish("Vehicle.Speed", &SignalValue::Float(3.0)));
        assert_eq!(seen.get(), 1);
        assert_eq!(bus.get("Vehicle.Speed"), Some(SignalValue::Float(3.0)));
        assert_eq!(bus.get("Other"), None);
    }
}
