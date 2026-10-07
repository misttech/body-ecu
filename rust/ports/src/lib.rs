// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/platform/ports`: the abstract interfaces the Body ECU's domain logic
//! depends on, and nothing else. An adapter implements each one for a platform; the
//! domain modules take them as `&'static dyn` references, as the C++ takes them by
//! reference, and the application wires them at startup.
//!
//! Callbacks (`std::function` in the C++) are boxed closures. The interfaces take `&self`:
//! an implementation keeps its state in cells, as more than one domain module holds a
//! reference to the same port (the GPIO port is shared by lighting and the door lock, the
//! button by the door lock and the ignition).
//!
//! With the `mock` feature, [`mock`] holds recording doubles for every port, the
//! counterparts of `libs/platform/ports/mock`.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod console;
pub mod ids;
#[cfg(feature = "mock")]
pub mod mock;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// A SOME/IP message as the services see it (`ISomeIpService.h`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SomeIpMessage {
    /// The service.
    pub service_id: u16,
    /// The method, or the event.
    pub method_id: u16,
    /// The client the request came from.
    pub client_id: u16,
    /// The client's session.
    pub session_id: u16,
    /// Request (0x00), notification (0x02), response (0x80), or error (0x81).
    pub message_type: u8,
    /// E_OK (0x00) or an error.
    pub return_code: u8,
    /// The payload.
    pub payload: Vec<u8>,
}

/// A method handler: the response to a request.
pub type MethodHandler = Box<dyn Fn(&SomeIpMessage) -> SomeIpMessage>;

/// The SOME/IP service a domain module registers its methods and events with.
pub trait SomeIpService {
    /// Register `handler` for `method_id` of `service_id`.
    fn register_method(&self, service_id: u16, method_id: u16, handler: MethodHandler);
    /// Register `event_id` of `service_id` in `eventgroup_id`.
    fn register_event(&self, service_id: u16, event_id: u16, eventgroup_id: u16);
    /// Send `event_id` of `service_id` with `payload` to every subscriber.
    fn send_event(&self, service_id: u16, event_id: u16, payload: &[u8]);
    /// Send `response`.
    fn send_response(&self, response: &SomeIpMessage);
}

/// A GPIO port (`IGpioPort.h`): output pins the domain logic drives.
pub trait GpioPort {
    /// Drive `pin` to `value`.
    fn write(&self, pin: u32, value: bool);
    /// The level of `pin`.
    fn read(&self, pin: u32) -> bool;
}

/// What runs when the button is pressed.
pub type ButtonCallback = Box<dyn Fn()>;

/// A button (`IButtonInput.h`).
pub trait ButtonInput {
    /// Run `callback` on every press.
    fn on_press(&self, callback: ButtonCallback);
}

/// A button nobody presses (`NullButtonInput.h`).
pub struct NullButtonInput;

impl ButtonInput for NullButtonInput {
    fn on_press(&self, _callback: ButtonCallback) {}
}

/// An ADC input (`IAdcInput.h`).
pub trait AdcInput {
    /// The raw sample of `channel`, or a negative value when the read failed.
    fn read(&self, channel: u8) -> i32;
}

/// A value on the signal bus (`ISignalBus.h`'s `SignalValue` variant).
#[derive(Clone, Debug, PartialEq)]
pub enum SignalValue {
    /// A boolean.
    Bool(bool),
    /// A signed integer.
    Int(i32),
    /// A float.
    Float(f32),
    /// A string.
    String(String),
    /// Bytes.
    Bytes(Vec<u8>),
}

/// What runs when a subscribed signal is published: the path and the value.
pub type SignalCallback = Box<dyn Fn(&str, &SignalValue)>;

/// The vehicle signal bus (`ISignalBus.h`): VSS paths to values.
pub trait SignalBus {
    /// Publish `value` at `path`; returns whether it was accepted.
    fn publish(&self, path: &str, value: &SignalValue) -> bool;
    /// Run `callback` whenever `path` is published.
    fn subscribe(&self, path: &str, callback: SignalCallback);
    /// The last value published at `path`.
    fn get(&self, path: &str) -> Option<SignalValue>;
}

/// Diagnostic data for a data identifier (`IDiagDataProvider.h`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagData {
    /// The data identifier.
    pub did: u16,
    /// Its data.
    pub data: Vec<u8>,
}

/// A module that answers diagnostic requests (`IDiagDataProvider.h`).
pub trait DiagDataProvider {
    /// The data of `did`, if this provider serves it.
    fn read_data(&self, did: u16) -> Option<DiagData>;
    /// Apply `control_param` to `did`; returns whether this provider did.
    fn io_control(&self, did: u16, control_param: &[u8]) -> bool;
}

/// The vehicle's mode (`IModeObserver.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum VehicleMode {
    /// Off.
    Off = 0,
    /// Accessory.
    Accessory = 1,
    /// Run.
    Run = 2,
    /// Crank.
    Crank = 3,
}

impl VehicleMode {
    /// The mode `value` names, as a C++ `static_cast<VehicleMode>` would read it; a value
    /// no mode has is `None`.
    pub const fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Off),
            1 => Some(Self::Accessory),
            2 => Some(Self::Run),
            3 => Some(Self::Crank),
            _ => None,
        }
    }
}

/// Hears about mode changes (`IModeObserver.h`).
pub trait ModeObserver {
    /// The mode changed from `old_mode` to `new_mode`.
    fn on_mode_changed(&self, old_mode: VehicleMode, new_mode: VehicleMode);
}

/// A timer (`ITimerService.h`).
pub type TimerId = u32;
/// No timer: what a start returns when every slot is taken.
pub const INVALID_TIMER_ID: TimerId = u32::MAX;
/// What runs when a timer fires.
pub type TimerCallback = Box<dyn Fn()>;

/// Timers (`ITimerService.h`).
pub trait TimerService {
    /// Run `callback` every `interval_ms`.
    fn start_periodic(&self, interval_ms: u32, callback: TimerCallback) -> TimerId;
    /// Run `callback` once after `delay_ms`.
    fn start_one_shot(&self, delay_ms: u32, callback: TimerCallback) -> TimerId;
    /// Stop timer `id`.
    fn cancel(&self, id: TimerId);
}

/// The most data bytes a classic CAN frame carries.
pub const CAN_MAX_CLASSIC_DLC: u8 = 8;
/// The most data bytes a CAN FD frame carries.
pub const CAN_MAX_FD_DLC: u8 = 64;

/// A CAN frame (`ICanBus.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CanFrame {
    /// The identifier.
    pub id: u32,
    /// The data length.
    pub dlc: u8,
    /// The data; `dlc` bytes of it are meaningful.
    pub data: [u8; CAN_MAX_FD_DLC as usize],
}

impl Default for CanFrame {
    fn default() -> Self {
        Self { id: 0, dlc: 0, data: [0; CAN_MAX_FD_DLC as usize] }
    }
}

/// What runs for each received frame.
pub type CanRxCallback = Box<dyn Fn(&CanFrame)>;

/// A CAN bus (`ICanBus.h`).
pub trait CanBus {
    /// Send `frame`; returns whether it was queued.
    fn send(&self, frame: &CanFrame) -> bool;
    /// Register an additional receive listener. Multiple listeners are supported; each
    /// receives every incoming frame.
    fn add_rx_callback(&self, callback: CanRxCallback);
    /// Deprecated in the C++: forwards to [`Self::add_rx_callback`], kept for parity.
    fn set_rx_callback(&self, callback: CanRxCallback) {
        self.add_rx_callback(callback);
    }
}
