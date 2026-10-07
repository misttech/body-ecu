// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/body/vehicle-mode`: the vehicle mode as a SOME/IP field with a getter, a
//! setter, and a notifier, with the transitions the state machine allows, and observers
//! told of every change.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use body_ecu_ports::{ModeObserver, SomeIpMessage, SomeIpService, VehicleMode, ids};

/// The manager's configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VehicleModeConfig {
    /// The SOME/IP service.
    pub service_id: u16,
    /// The field's getter.
    pub getter_method: u16,
    /// The field's setter.
    pub setter_method: u16,
    /// The field's notifier.
    pub notifier_event: u16,
    /// The notifier's group.
    pub eventgroup_id: u16,
}

impl VehicleModeConfig {
    /// The defaults, as a constant for a static.
    pub const DEFAULT: Self = Self {
        service_id: ids::vehicle_mode::SERVICE_ID,
        getter_method: ids::vehicle_mode::FIELD_MODE_GETTER,
        setter_method: ids::vehicle_mode::FIELD_MODE_SETTER,
        notifier_event: ids::vehicle_mode::FIELD_MODE_NOTIFIER,
        eventgroup_id: ids::vehicle_mode::EVENTGROUP_MODE_EVENTS,
    };
}

impl Default for VehicleModeConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The vehicle mode manager (`VehicleModeManager`).
///
/// Thread-safety contract: [`Self::add_observer`] must only be called during
/// initialization (before the lifecycle transitions to run level). [`Self::set_mode`] and
/// [`Self::get_mode`] are single-threaded (main loop or timer callback context). Do not
/// call from multiple threads concurrently.
pub struct VehicleModeManager {
    someip: &'static dyn SomeIpService,
    config: VehicleModeConfig,
    mode: Cell<VehicleMode>,
    observers: RefCell<Vec<&'static dyn ModeObserver>>,
}

impl VehicleModeManager {
    /// A manager serving over `someip` with `config`, in mode Off.
    pub const fn new(someip: &'static dyn SomeIpService, config: VehicleModeConfig) -> Self {
        Self {
            someip,
            config,
            mode: Cell::new(VehicleMode::Off),
            observers: RefCell::new(Vec::new()),
        }
    }

    /// Register the getter, the setter, and the notifier.
    pub fn init(&'static self) {
        self.someip.register_method(
            self.config.service_id,
            self.config.getter_method,
            Box::new(move |request| self.handle_get(request)),
        );
        self.someip.register_method(
            self.config.service_id,
            self.config.setter_method,
            Box::new(move |request| self.handle_set(request)),
        );
        self.someip.register_event(
            self.config.service_id,
            self.config.notifier_event,
            self.config.eventgroup_id,
        );
    }

    /// The mode.
    pub fn get_mode(&self) -> VehicleMode {
        self.mode.get()
    }

    /// Change to `mode`; returns whether the transition was allowed.
    pub fn set_mode(&self, mode: VehicleMode) -> bool {
        let valid = self.is_valid_transition(self.mode.get(), mode);
        forkpoint::assert_sometimes!(!valid, "vehicle_mode: an invalid transition is refused");
        if !valid {
            return false;
        }
        if mode == self.mode.get() {
            return true;
        }

        let old = self.mode.get();
        self.mode.set(mode);
        forkpoint::assert_always!(
            self.is_valid_transition(old, self.mode.get()),
            "vehicle_mode: only valid transitions happen"
        );
        forkpoint::assert_sometimes!(
            self.mode.get() == VehicleMode::Run,
            "vehicle_mode: the vehicle runs"
        );
        forkpoint::lifecycle::send_event("vehicle_mode.changed", self.mode.get() as u64);
        self.publish_mode_changed();
        self.notify_observers(old, mode);
        true
    }

    /// Register an observer. Must be called during init, before the lifecycle
    /// transitions to run level.
    pub fn add_observer(&self, observer: &'static dyn ModeObserver) {
        self.observers.borrow_mut().push(observer);
    }

    fn is_valid_transition(&self, from: VehicleMode, to: VehicleMode) -> bool {
        if from == to {
            return true;
        }
        use VehicleMode as M;
        match from {
            M::Off => to == M::Accessory,
            M::Accessory => to == M::Off || to == M::Run || to == M::Crank,
            M::Run => to == M::Accessory || to == M::Crank,
            M::Crank => to == M::Run || to == M::Accessory,
        }
    }

    fn handle_get(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;
        response.return_code = 0x00;
        response.payload = alloc::vec![self.mode.get() as u8];
        response
    }

    fn handle_set(&self, request: &SomeIpMessage) -> SomeIpMessage {
        let mut response = request.clone();
        response.message_type = 0x80;

        if request.payload.is_empty() {
            response.return_code = 0x01;
            return response;
        }

        // A byte that names no mode is a transition the state machine never allows, as
        // the C++ `switch` refuses a `VehicleMode` cast from it.
        let accepted =
            VehicleMode::from_u8(request.payload[0]).is_some_and(|target| self.set_mode(target));
        response.return_code = if accepted { 0x00 } else { 0x01 };
        response
    }

    fn publish_mode_changed(&self) {
        self.someip.send_event(
            self.config.service_id,
            self.config.notifier_event,
            &[self.mode.get() as u8],
        );
    }

    fn notify_observers(&self, old_mode: VehicleMode, new_mode: VehicleMode) {
        for observer in self.observers.borrow().iter() {
            observer.on_mode_changed(old_mode, new_mode);
        }
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_ports::mock::{MockModeObserver, MockSomeIpService, leak};

    use super::*;

    struct Fixture {
        someip: &'static MockSomeIpService,
        config: VehicleModeConfig,
        mgr: &'static VehicleModeManager,
    }

    fn fixture() -> Fixture {
        let someip = leak(MockSomeIpService::new());
        let config = VehicleModeConfig::default();
        let mgr = leak(VehicleModeManager::new(someip, config.clone()));
        mgr.init();
        assert_eq!(someip.method_count(), 2);
        assert_eq!(someip.events.borrow().len(), 1);
        Fixture { someip, config, mgr }
    }

    #[test]
    fn initial_mode_is_off() {
        let f = fixture();
        assert_eq!(f.mgr.get_mode(), VehicleMode::Off);
    }

    #[test]
    fn mode_getter() {
        let f = fixture();
        f.mgr.set_mode(VehicleMode::Accessory);
        assert_eq!(f.someip.take_sent_events().len(), 1);
        assert_eq!(f.mgr.get_mode(), VehicleMode::Accessory);
    }

    #[test]
    fn mode_setter() {
        let f = fixture();
        assert!(f.mgr.set_mode(VehicleMode::Accessory));
        let events = f.someip.take_sent_events();
        assert_eq!(events.len(), 1);
        assert_eq!((events[0].0, events[0].1), (f.config.service_id, f.config.notifier_event));
        assert_eq!(f.mgr.get_mode(), VehicleMode::Accessory);
    }

    #[test]
    fn invalid_transition_rejected() {
        let f = fixture();
        assert!(!f.mgr.set_mode(VehicleMode::Crank));
        assert!(f.someip.take_sent_events().is_empty());
        assert_eq!(f.mgr.get_mode(), VehicleMode::Off);
    }

    #[test]
    fn valid_transition_chain() {
        let f = fixture();
        assert!(f.mgr.set_mode(VehicleMode::Accessory));
        assert!(f.mgr.set_mode(VehicleMode::Run));
        assert!(f.mgr.set_mode(VehicleMode::Crank));
        assert_eq!(f.someip.take_sent_events().len(), 3);
        assert_eq!(f.mgr.get_mode(), VehicleMode::Crank);
    }

    #[test]
    fn subscriber_notification() {
        let f = fixture();
        let observer = leak(MockModeObserver::new());
        f.mgr.add_observer(observer);
        f.mgr.set_mode(VehicleMode::Accessory);
        assert_eq!(
            observer.changes.borrow().as_slice(),
            [(VehicleMode::Off, VehicleMode::Accessory)]
        );
        assert_eq!(f.someip.take_sent_events().len(), 1);
    }

    #[test]
    fn multiple_subscribers() {
        let f = fixture();
        let observers = [
            leak(MockModeObserver::new()),
            leak(MockModeObserver::new()),
            leak(MockModeObserver::new()),
        ];
        for observer in observers {
            f.mgr.add_observer(observer);
        }
        f.mgr.set_mode(VehicleMode::Accessory);
        for observer in observers {
            assert_eq!(observer.changes.borrow().len(), 1);
        }
        assert_eq!(f.someip.take_sent_events().len(), 1);
    }

    #[test]
    fn same_mode_sets_no_op() {
        let f = fixture();
        assert!(f.mgr.set_mode(VehicleMode::Off));
        assert!(f.someip.take_sent_events().is_empty());
    }

    #[test]
    fn the_setter_refuses_an_empty_payload_and_an_unknown_mode() {
        let f = fixture();
        let request = SomeIpMessage {
            service_id: f.config.service_id,
            method_id: f.config.setter_method,
            ..Default::default()
        };
        assert_eq!(
            f.someip.call(f.config.service_id, f.config.setter_method, &request).return_code,
            0x01
        );
        let request = SomeIpMessage { payload: alloc::vec![7], ..request };
        assert_eq!(
            f.someip.call(f.config.service_id, f.config.setter_method, &request).return_code,
            0x01
        );
        let request = SomeIpMessage { payload: alloc::vec![1], ..request };
        assert_eq!(
            f.someip.call(f.config.service_id, f.config.setter_method, &request).return_code,
            0x00
        );
        let getter = SomeIpMessage { method_id: f.config.getter_method, ..request };
        assert_eq!(
            f.someip.call(f.config.service_id, f.config.getter_method, &getter).payload,
            [1]
        );
    }
}
