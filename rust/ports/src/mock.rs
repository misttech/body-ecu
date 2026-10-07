// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Recording doubles for every port, the counterparts of `libs/platform/ports/mock`. A
//! mock keeps what it was asked and hands the callbacks it was given back to the test,
//! which calls them as the platform would.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use crate::{
    AdcInput, ButtonCallback, ButtonInput, CanBus, CanFrame, CanRxCallback, DiagData,
    DiagDataProvider, GpioPort, MethodHandler, ModeObserver, SignalBus, SignalCallback,
    SignalValue, SomeIpMessage, SomeIpService, TimerCallback, TimerId, TimerService, VehicleMode,
};

/// `MockGpioPort`: remembers every write.
#[derive(Default)]
pub struct MockGpioPort {
    /// The writes, in order: `(pin, value)`.
    pub writes: RefCell<Vec<(u32, bool)>>,
    /// What `read` answers.
    pub levels: RefCell<BTreeMap<u32, bool>>,
}

impl MockGpioPort {
    /// A port with nothing written.
    pub const fn new() -> Self {
        Self { writes: RefCell::new(Vec::new()), levels: RefCell::new(BTreeMap::new()) }
    }

    /// The writes so far, cleared.
    pub fn take_writes(&self) -> Vec<(u32, bool)> {
        core::mem::take(&mut *self.writes.borrow_mut())
    }
}

impl GpioPort for MockGpioPort {
    fn write(&self, pin: u32, value: bool) {
        self.writes.borrow_mut().push((pin, value));
    }

    fn read(&self, pin: u32) -> bool {
        self.levels.borrow().get(&pin).copied().unwrap_or(false)
    }
}

/// `MockButtonInput`: keeps the last callback, which [`Self::press`] runs.
#[derive(Default)]
pub struct MockButtonInput {
    callback: RefCell<Option<ButtonCallback>>,
    /// How many times `on_press` was called.
    pub registrations: Cell<usize>,
}

impl MockButtonInput {
    /// A button nobody listens to.
    pub const fn new() -> Self {
        Self { callback: RefCell::new(None), registrations: Cell::new(0) }
    }

    /// Whether a callback is registered.
    pub fn has_callback(&self) -> bool {
        self.callback.borrow().is_some()
    }

    /// Press the button: run the callback.
    pub fn press(&self) {
        if let Some(callback) = self.callback.borrow().as_ref() {
            callback();
        }
    }
}

impl ButtonInput for MockButtonInput {
    fn on_press(&self, callback: ButtonCallback) {
        self.registrations.set(self.registrations.get() + 1);
        *self.callback.borrow_mut() = Some(callback);
    }
}

/// `MockAdcInput`: answers every read with what the test set, or with a function of
/// the call count.
pub struct MockAdcInput {
    /// What `read` answers.
    pub value: Cell<i32>,
    /// A reader that decides per call, when set; it sees the call count.
    #[allow(clippy::type_complexity)]
    pub reader: RefCell<Option<Box<dyn Fn(usize) -> i32>>>,
    /// How many reads happened.
    pub reads: Cell<usize>,
}

impl Default for MockAdcInput {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAdcInput {
    /// An ADC reading 0.
    pub const fn new() -> Self {
        Self { value: Cell::new(0), reader: RefCell::new(None), reads: Cell::new(0) }
    }
}

impl AdcInput for MockAdcInput {
    fn read(&self, _channel: u8) -> i32 {
        let count = self.reads.get();
        self.reads.set(count + 1);
        match self.reader.borrow().as_ref() {
            Some(reader) => reader(count),
            None => self.value.get(),
        }
    }
}

/// `MockSignalBus`: a store of values, the subscriptions, and every publication.
#[derive(Default)]
pub struct MockSignalBus {
    /// What `get` answers.
    pub values: RefCell<BTreeMap<String, SignalValue>>,
    /// Every publication, in order.
    pub published: RefCell<Vec<(String, SignalValue)>>,
    subscriptions: RefCell<Vec<(String, SignalCallback)>>,
}

impl MockSignalBus {
    /// An empty bus.
    pub const fn new() -> Self {
        Self {
            values: RefCell::new(BTreeMap::new()),
            published: RefCell::new(Vec::new()),
            subscriptions: RefCell::new(Vec::new()),
        }
    }

    /// Make `get(path)` answer `value`.
    pub fn set(&self, path: &str, value: SignalValue) {
        self.values.borrow_mut().insert(path.to_string(), value);
    }

    /// The paths subscribed to, in order.
    pub fn subscribed_paths(&self) -> Vec<String> {
        self.subscriptions.borrow().iter().map(|(path, _)| path.clone()).collect()
    }

    /// Deliver `value` at `path` to its subscribers, as a publisher elsewhere would.
    pub fn deliver(&self, path: &str, value: &SignalValue) {
        for (subscribed, callback) in self.subscriptions.borrow().iter() {
            if subscribed == path {
                callback(path, value);
            }
        }
    }

    /// How many times `path` was published.
    pub fn publications(&self, path: &str) -> usize {
        self.published.borrow().iter().filter(|(p, _)| p == path).count()
    }
}

impl SignalBus for MockSignalBus {
    fn publish(&self, path: &str, value: &SignalValue) -> bool {
        self.published.borrow_mut().push((path.to_string(), value.clone()));
        true
    }

    fn subscribe(&self, path: &str, callback: SignalCallback) {
        self.subscriptions.borrow_mut().push((path.to_string(), callback));
    }

    fn get(&self, path: &str) -> Option<SignalValue> {
        self.values.borrow().get(path).cloned()
    }
}

/// `MockSomeIpService`: keeps the registrations and what was sent.
#[derive(Default)]
pub struct MockSomeIpService {
    methods: RefCell<Vec<(u16, u16, MethodHandler)>>,
    /// The events registered: `(service, event, eventgroup)`.
    pub events: RefCell<Vec<(u16, u16, u16)>>,
    /// The events sent: `(service, event, payload)`.
    pub sent_events: RefCell<Vec<(u16, u16, Vec<u8>)>>,
    /// The responses sent.
    pub sent_responses: RefCell<Vec<SomeIpMessage>>,
}

impl MockSomeIpService {
    /// A service with nothing registered.
    pub const fn new() -> Self {
        Self {
            methods: RefCell::new(Vec::new()),
            events: RefCell::new(Vec::new()),
            sent_events: RefCell::new(Vec::new()),
            sent_responses: RefCell::new(Vec::new()),
        }
    }

    /// How many methods are registered.
    pub fn method_count(&self) -> usize {
        self.methods.borrow().len()
    }

    /// Call the handler of `method_id` of `service_id` with `request`.
    pub fn call(&self, service_id: u16, method_id: u16, request: &SomeIpMessage) -> SomeIpMessage {
        let methods = self.methods.borrow();
        let (_, _, handler) = methods
            .iter()
            .find(|(service, method, _)| *service == service_id && *method == method_id)
            .expect("the method is registered");
        handler(request)
    }

    /// The events sent so far, cleared.
    pub fn take_sent_events(&self) -> Vec<(u16, u16, Vec<u8>)> {
        core::mem::take(&mut *self.sent_events.borrow_mut())
    }
}

impl SomeIpService for MockSomeIpService {
    fn register_method(&self, service_id: u16, method_id: u16, handler: MethodHandler) {
        self.methods.borrow_mut().push((service_id, method_id, handler));
    }

    fn register_event(&self, service_id: u16, event_id: u16, eventgroup_id: u16) {
        self.events.borrow_mut().push((service_id, event_id, eventgroup_id));
    }

    fn send_event(&self, service_id: u16, event_id: u16, payload: &[u8]) {
        self.sent_events.borrow_mut().push((service_id, event_id, payload.to_vec()));
    }

    fn send_response(&self, response: &SomeIpMessage) {
        self.sent_responses.borrow_mut().push(response.clone());
    }
}

/// `MockDiagDataProvider`: answers from tables the test fills.
#[derive(Default)]
pub struct MockDiagDataProvider {
    /// What `read_data` answers per identifier.
    pub data: RefCell<BTreeMap<u16, Vec<u8>>>,
    /// The identifiers `io_control` accepts.
    pub controllable: RefCell<Vec<u16>>,
    /// Every `io_control` call: `(did, control_param)`.
    pub controls: RefCell<Vec<(u16, Vec<u8>)>>,
}

impl MockDiagDataProvider {
    /// A provider that serves nothing.
    pub const fn new() -> Self {
        Self {
            data: RefCell::new(BTreeMap::new()),
            controllable: RefCell::new(Vec::new()),
            controls: RefCell::new(Vec::new()),
        }
    }
}

impl DiagDataProvider for MockDiagDataProvider {
    fn read_data(&self, did: u16) -> Option<DiagData> {
        self.data.borrow().get(&did).map(|data| DiagData { did, data: data.clone() })
    }

    fn io_control(&self, did: u16, control_param: &[u8]) -> bool {
        self.controls.borrow_mut().push((did, control_param.to_vec()));
        self.controllable.borrow().contains(&did)
    }
}

/// `MockModeObserver`: remembers every change.
#[derive(Default)]
pub struct MockModeObserver {
    /// The changes, in order.
    pub changes: RefCell<Vec<(VehicleMode, VehicleMode)>>,
}

impl MockModeObserver {
    /// An observer that saw nothing.
    pub const fn new() -> Self {
        Self { changes: RefCell::new(Vec::new()) }
    }
}

impl ModeObserver for MockModeObserver {
    fn on_mode_changed(&self, old_mode: VehicleMode, new_mode: VehicleMode) {
        self.changes.borrow_mut().push((old_mode, new_mode));
    }
}

/// `MockTimerService`: keeps the callbacks, which the test fires.
pub struct MockTimerService {
    /// The one-shot timers started: `(delay_ms, callback)`.
    pub one_shots: RefCell<Vec<(u32, TimerCallback)>>,
    /// The periodic timers started: `(interval_ms, callback)`.
    pub periodics: RefCell<Vec<(u32, TimerCallback)>>,
    /// The ids cancelled.
    pub cancelled: RefCell<Vec<TimerId>>,
    /// The id the next start returns.
    pub next_id: Cell<TimerId>,
}

impl Default for MockTimerService {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTimerService {
    /// A service whose first timer is 42, as the C++ tests answer.
    pub const fn new() -> Self {
        Self {
            one_shots: RefCell::new(Vec::new()),
            periodics: RefCell::new(Vec::new()),
            cancelled: RefCell::new(Vec::new()),
            next_id: Cell::new(42),
        }
    }

    /// Fire the last one-shot timer started.
    pub fn fire_last_one_shot(&self) {
        let one_shots = self.one_shots.borrow();
        let (_, callback) = one_shots.last().expect("a one-shot timer was started");
        callback();
    }

    /// Fire the first periodic timer started.
    pub fn tick_first_periodic(&self) {
        let periodics = self.periodics.borrow();
        let (_, callback) = periodics.first().expect("a periodic timer was started");
        callback();
    }
}

impl TimerService for MockTimerService {
    fn start_periodic(&self, interval_ms: u32, callback: TimerCallback) -> TimerId {
        let id = self.next_id.get();
        if id == crate::INVALID_TIMER_ID {
            return id;
        }
        self.periodics.borrow_mut().push((interval_ms, callback));
        id
    }

    fn start_one_shot(&self, delay_ms: u32, callback: TimerCallback) -> TimerId {
        let id = self.next_id.get();
        if id == crate::INVALID_TIMER_ID {
            return id;
        }
        self.one_shots.borrow_mut().push((delay_ms, callback));
        id
    }

    fn cancel(&self, id: TimerId) {
        self.cancelled.borrow_mut().push(id);
    }
}

/// `MockCanBus`: keeps the frames sent and the receive callbacks.
#[derive(Default)]
pub struct MockCanBus {
    /// The frames sent.
    pub sent: RefCell<Vec<CanFrame>>,
    callbacks: RefCell<Vec<CanRxCallback>>,
    /// Whether `send` reports success.
    pub accept: Cell<bool>,
}

impl MockCanBus {
    /// A bus that accepts every frame.
    pub const fn new() -> Self {
        Self {
            sent: RefCell::new(Vec::new()),
            callbacks: RefCell::new(Vec::new()),
            accept: Cell::new(true),
        }
    }

    /// How many receive callbacks are registered.
    pub fn callback_count(&self) -> usize {
        self.callbacks.borrow().len()
    }

    /// Deliver `frame` to every receive callback, as the bus would.
    pub fn receive(&self, frame: &CanFrame) {
        for callback in self.callbacks.borrow().iter() {
            callback(frame);
        }
    }
}

impl CanBus for MockCanBus {
    fn send(&self, frame: &CanFrame) -> bool {
        self.sent.borrow_mut().push(*frame);
        self.accept.get()
    }

    fn add_rx_callback(&self, callback: CanRxCallback) {
        self.callbacks.borrow_mut().push(callback);
    }
}

/// A port as a `'static` reference, as the domain modules take their ports: the mock
/// lives for the rest of the test process.
pub fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}
