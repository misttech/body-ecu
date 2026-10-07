// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `SomeIpSystem`: the SOME/IP server. Methods and events are registered with it, a
//! request is dispatched to its handler and answered, an event goes to every client that
//! ever sent a request, and events raised while a request is dispatched go after its
//! response. With the `transport` feature the server listens on a UDP socket through the
//! shim, from a thread that polls it, as the OpenSOME/IP transport does on Zephyr;
//! without it (tests) the server records what it would send.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use body_ecu_ports::{MethodHandler, SomeIpMessage, SomeIpService, printk};
use openbsw_lifecycle::{ComponentBase, LifecycleComponent};

#[cfg(feature = "transport")]
use alloc::collections::BTreeSet;
#[cfg(feature = "transport")]
use body_ecu_someip::{Endpoint, Message};

/// Server or client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SomeIpRole {
    /// Serves requests.
    Server,
    /// Sends them.
    Client,
}

/// The system's configuration (`SomeIpConfig`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SomeIpConfig {
    /// The address to bind to (a server) or to send to (a client).
    pub host: [u8; 4],
    /// Its port.
    pub port: u16,
    /// Server or client.
    pub role: SomeIpRole,
    /// Whether service discovery runs; it never does under emulation.
    pub enable_sd: bool,
    /// The discovery multicast group.
    pub sd_multicast: [u8; 4],
    /// Its port.
    pub sd_port: u16,
    /// How often offers go out.
    pub sd_offer_interval_ms: u32,
}

impl Default for SomeIpConfig {
    fn default() -> Self {
        Self {
            host: [0, 0, 0, 0],
            port: 30490,
            role: SomeIpRole::Server,
            enable_sd: true,
            sd_multicast: [239, 255, 255, 251],
            sd_port: 30491,
            sd_offer_interval_ms: 5000,
        }
    }
}

type MethodKey = u32;

/// An event registration, kept as the C++ keeps it: discovery, which would offer it,
/// is not ported, so nothing reads it.
#[allow(dead_code)]
struct EventRegistration {
    service_id: u16,
    event_id: u16,
    eventgroup_id: u16,
}

/// The largest datagram the receive loop takes.
#[cfg(feature = "transport")]
const RECEIVE_BUFFER_SIZE: usize = 1500;

/// The SOME/IP server.
#[cfg_attr(not(feature = "transport"), allow(dead_code))]
pub struct SomeIpSystem {
    base: ComponentBase,
    config: SomeIpConfig,
    running: Cell<bool>,
    dispatching: Cell<bool>,
    methods: RefCell<BTreeMap<MethodKey, MethodHandler>>,
    events: RefCell<Vec<EventRegistration>>,
    sent_events: RefCell<Vec<SomeIpMessage>>,
    sent_responses: RefCell<Vec<SomeIpMessage>>,
    pending_events: RefCell<Vec<SomeIpMessage>>,
    #[cfg(feature = "transport")]
    socket: Cell<i32>,
    #[cfg(feature = "transport")]
    known_clients: RefCell<BTreeSet<Endpoint>>,
    #[cfg(feature = "transport")]
    server_endpoint: Cell<Endpoint>,
    #[cfg(feature = "transport")]
    is_server: Cell<bool>,
}

// SAFETY: on the MCU the dispatch path runs on the receive thread and the registrations
// happen during the lifecycle's init, before it; the C++ guards nothing more
// (`PlatformMutex` is a no-op there, as cooperative scheduling makes races impossible).
unsafe impl Sync for SomeIpSystem {}

impl SomeIpSystem {
    /// A server with `config`, not running.
    pub const fn new(config: SomeIpConfig) -> Self {
        Self {
            base: ComponentBase::new(),
            config,
            running: Cell::new(false),
            dispatching: Cell::new(false),
            methods: RefCell::new(BTreeMap::new()),
            events: RefCell::new(Vec::new()),
            sent_events: RefCell::new(Vec::new()),
            sent_responses: RefCell::new(Vec::new()),
            pending_events: RefCell::new(Vec::new()),
            #[cfg(feature = "transport")]
            socket: Cell::new(-1),
            #[cfg(feature = "transport")]
            known_clients: RefCell::new(BTreeSet::new()),
            #[cfg(feature = "transport")]
            server_endpoint: Cell::new(Endpoint::new([0; 4], 0)),
            #[cfg(feature = "transport")]
            is_server: Cell::new(false),
        }
    }

    /// Whether the server runs.
    pub fn is_running(&self) -> bool {
        self.running.get()
    }

    /// The events sent, without a transport.
    pub fn sent_events(&self) -> Vec<SomeIpMessage> {
        self.sent_events.borrow().clone()
    }

    /// The responses sent, without a transport.
    pub fn sent_responses(&self) -> Vec<SomeIpMessage> {
        self.sent_responses.borrow().clone()
    }

    const fn make_key(service_id: u16, method_id: u16) -> MethodKey {
        ((service_id as u32) << 16) | method_id as u32
    }

    /// The response to `request`: its handler's, or an error for a method nobody
    /// registered.
    pub fn dispatch(&self, request: &SomeIpMessage) -> SomeIpMessage {
        printk!(
            "[SOME/IP] dispatch: lookup 0x{:04X}/0x{:04X}\n",
            request.service_id,
            request.method_id
        );
        forkpoint::lifecycle::send_event(
            "someip.request",
            (u64::from(request.service_id) << 16) | u64::from(request.method_id),
        );
        let methods = self.methods.borrow();
        let handler = methods.get(&Self::make_key(request.service_id, request.method_id));
        forkpoint::assert_sometimes!(
            handler.is_none(),
            "someip: an unknown method is answered with an error"
        );
        let Some(handler) = handler else {
            printk!("[SOME/IP] dispatch: method NOT FOUND\n");
            let mut err = request.clone();
            err.message_type = 0x81; // Error
            err.return_code = 0x05; // E_NOT_OK
            return err;
        };
        printk!("[SOME/IP] dispatch: calling handler\n");
        let response = handler(request);
        printk!("[SOME/IP] dispatch: handler returned rc=0x{:02X}\n", response.return_code);
        forkpoint::assert_always!(
            response.service_id == request.service_id && response.method_id == request.method_id,
            "someip: a response names the request's service and method"
        );
        forkpoint::assert_sometimes!(
            response.return_code != 0,
            "someip: a handler refuses a request"
        );
        response
    }

    /// The configured host as `a.b.c.d`, as the C++ keeps it as a string.
    #[cfg(feature = "transport")]
    fn host_text(&self) -> alloc::string::String {
        let [a, b, c, d] = self.config.host;
        alloc::format!("{a}.{b}.{c}.{d}")
    }

    #[cfg(feature = "transport")]
    fn init_sd(&self) {
        if !self.config.enable_sd {
            printk!("[SOME/IP-SD] Disabled by configuration\n");
        }
        // Service discovery is never enabled under emulation (`enable_sd = real_hw`), and
        // the OpenSOME/IP discovery it would start is not ported.
    }

    #[cfg(feature = "transport")]
    fn send_to(&self, message: &Message, to: &Endpoint) {
        let fd = self.socket.get();
        if fd < 0 {
            return;
        }
        body_ecu_ffi::udp_send_to(fd, &message.serialize(), to.address, to.port);
    }

    #[cfg(feature = "transport")]
    fn to_someip(msg: &SomeIpMessage) -> Message {
        Message::new(
            msg.service_id,
            msg.method_id,
            msg.client_id,
            msg.session_id,
            msg.message_type,
            msg.return_code,
            msg.payload.clone(),
        )
    }

    #[cfg(feature = "transport")]
    fn from_someip(msg: &Message) -> SomeIpMessage {
        SomeIpMessage {
            service_id: msg.service_id,
            method_id: msg.method_id,
            client_id: msg.client_id,
            session_id: msg.session_id,
            message_type: msg.message_type,
            return_code: msg.return_code,
            payload: msg.payload.clone(),
        }
    }

    /// A message arrived from `sender` (the C++ `on_message_received`): a request is
    /// dispatched and answered, then the events it raised go out; anything else goes to
    /// its handler, if one is registered, without an answer.
    #[cfg(feature = "transport")]
    pub fn on_message_received(&self, message: &Message, sender: Endpoint) {
        self.known_clients.borrow_mut().insert(sender);

        let incoming = Self::from_someip(message);

        printk!(
            "[SOME/IP] Received service=0x{:04X} method=0x{:04X} type=0x{:02X} from {} payload=[",
            incoming.service_id,
            incoming.method_id,
            incoming.message_type,
            sender
        );
        for (i, byte) in incoming.payload.iter().enumerate() {
            printk!("{}0x{:02X}", if i != 0 { " " } else { "" }, byte);
        }
        printk!("]\n");

        if message.is_request() {
            printk!("[SOME/IP] >> dispatch\n");
            self.dispatching.set(true);
            let mut response = self.dispatch(&incoming);
            self.dispatching.set(false);
            let deferred: Vec<SomeIpMessage> =
                core::mem::take(&mut *self.pending_events.borrow_mut());
            printk!(
                "[SOME/IP] << dispatch rc=0x{:02X} pending={}\n",
                response.return_code,
                deferred.len()
            );
            response.message_type = 0x80; // RESPONSE
            let resp_msg = Self::to_someip(&response);
            printk!("[SOME/IP] >> send response\n");
            self.send_to(&resp_msg, &sender);
            printk!("[SOME/IP] << send response OK\n");

            let clients_snapshot: Vec<Endpoint> =
                self.known_clients.borrow().iter().copied().collect();
            for evt in &deferred {
                let evt_msg = Self::to_someip(evt);
                for client in &clients_snapshot {
                    self.send_to(&evt_msg, client);
                }
            }
        } else {
            let methods = self.methods.borrow();
            if let Some(handler) =
                methods.get(&Self::make_key(incoming.service_id, incoming.method_id))
            {
                handler(&incoming);
            }
        }
    }

    /// The receive thread's loop (the OpenSOME/IP `UdpTransport::receive_loop`): take
    /// every datagram waiting, and sleep 10 ms when none does, until the server stops.
    #[cfg(feature = "transport")]
    pub fn receive_loop(&self) {
        let mut buffer = alloc::vec![0_u8; RECEIVE_BUFFER_SIZE];
        while self.running.get() {
            let fd = self.socket.get();
            if fd < 0 {
                // Socket was closed, exit loop
                break;
            }
            match body_ecu_ffi::udp_recv_from(fd, &mut buffer) {
                Ok((received, address, port)) if received > 0 => {
                    if let Some(message) = Message::deserialize(&buffer[..received]) {
                        self.on_message_received(&message, Endpoint::new(address, port));
                    }
                }
                Ok(_) => {}
                Err(error) if error == -body_ecu_ffi::EAGAIN => {
                    // Timeout in non-blocking mode: just continue polling, with a small
                    // delay to prevent a tight polling loop.
                    body_ecu_ffi::msleep(10);
                }
                Err(error) => {
                    printk!("[SOME/IP] Transport error: {}\n", error);
                    body_ecu_ffi::msleep(10);
                }
            }
        }
    }
}

impl LifecycleComponent for SomeIpSystem {
    fn base(&self) -> &ComponentBase {
        &self.base
    }

    fn init(&'static self) {
        #[cfg(feature = "transport")]
        {
            self.is_server.set(self.config.role == SomeIpRole::Server);
            if !self.is_server.get() {
                self.server_endpoint.set(Endpoint::new(self.config.host, self.config.port));
            }
            printk!(
                "[SOME/IP] Initialized ({}) {}:{}\n",
                if self.is_server.get() { "server" } else { "client -> " },
                self.host_text(),
                self.config.port
            );
        }
        self.transition_done();
    }

    fn run(&'static self) {
        self.running.set(true);
        #[cfg(feature = "transport")]
        {
            // The transport's start: a socket bound to the configured endpoint (any port
            // for a client), and the receive thread.
            let port = if self.is_server.get() { self.config.port } else { 0 };
            let fd = body_ecu_ffi::udp_open(port);
            if fd < 0 {
                printk!("[SOME/IP] Failed to start transport (error {})\n", 1);
                return;
            }
            self.socket.set(fd);
            body_ecu_ffi::someip_thread_start();
            printk!("[SOME/IP] Transport running on udp://{}:{}\n", self.host_text(), port);
            self.init_sd();
        }
        self.transition_done();
    }

    fn shutdown(&'static self) {
        self.running.set(false);
        #[cfg(feature = "transport")]
        {
            let fd = self.socket.replace(-1);
            if fd >= 0 {
                body_ecu_ffi::udp_close(fd);
                printk!("[SOME/IP] Transport stopped\n");
            }
        }
        self.methods.borrow_mut().clear();
        self.events.borrow_mut().clear();
        self.pending_events.borrow_mut().clear();
        self.transition_done();
    }
}

impl SomeIpService for SomeIpSystem {
    fn register_method(&self, service_id: u16, method_id: u16, handler: MethodHandler) {
        self.methods.borrow_mut().insert(Self::make_key(service_id, method_id), handler);
    }

    fn register_event(&self, service_id: u16, event_id: u16, eventgroup_id: u16) {
        self.events.borrow_mut().push(EventRegistration { service_id, event_id, eventgroup_id });
    }

    fn send_event(&self, service_id: u16, event_id: u16, payload: &[u8]) {
        let msg = SomeIpMessage {
            service_id,
            method_id: event_id,
            client_id: 0,
            session_id: 0,
            message_type: 0x02, // Notification
            return_code: 0x00,
            payload: payload.to_vec(),
        };
        #[cfg(not(feature = "transport"))]
        self.sent_events.borrow_mut().push(msg);

        #[cfg(feature = "transport")]
        {
            if self.dispatching.get() {
                self.pending_events.borrow_mut().push(msg);
                return;
            }
            if self.socket.get() >= 0 && self.running.get() {
                let someip_msg = Self::to_someip(&msg);
                for client in self.known_clients.borrow().iter() {
                    self.send_to(&someip_msg, client);
                }
            }
        }
    }

    fn send_response(&self, response: &SomeIpMessage) {
        #[cfg(not(feature = "transport"))]
        self.sent_responses.borrow_mut().push(response.clone());

        #[cfg(feature = "transport")]
        {
            if self.socket.get() >= 0 && self.running.get() {
                let someip_msg = Self::to_someip(response);
                if self.is_server.get() {
                    for client in self.known_clients.borrow().iter() {
                        self.send_to(&someip_msg, client);
                    }
                } else {
                    self.send_to(&someip_msg, &self.server_endpoint.get());
                }
            }
        }
    }
}

#[cfg(all(test, not(feature = "transport")))]
mod tests {
    use alloc::boxed::Box;

    use body_ecu_ports::mock::leak;

    use super::*;

    fn system() -> &'static SomeIpSystem {
        leak(SomeIpSystem::new(SomeIpConfig {
            host: [127, 0, 0, 1],
            port: 30490,
            ..Default::default()
        }))
    }

    #[test]
    fn lifecycle_transitions() {
        let sys = system();
        assert!(!sys.is_running());
        sys.init();
        assert!(!sys.is_running());
        sys.run();
        assert!(sys.is_running());
        sys.shutdown();
        assert!(!sys.is_running());
    }

    #[test]
    fn method_registration_and_dispatch() {
        let sys = system();
        sys.init();
        let called = leak(Cell::new(false));
        let received = leak(RefCell::new(Vec::new()));
        sys.register_method(
            0x1000,
            0x0001,
            Box::new(move |req| {
                called.set(true);
                *received.borrow_mut() = req.payload.clone();
                let mut resp = req.clone();
                resp.message_type = 0x80;
                resp.return_code = 0x00;
                resp.payload = alloc::vec![0xAA];
                resp
            }),
        );
        let request = SomeIpMessage {
            service_id: 0x1000,
            method_id: 0x0001,
            client_id: 0x0042,
            session_id: 0x0001,
            payload: alloc::vec![0x01, 0x02],
            ..Default::default()
        };
        let response = sys.dispatch(&request);
        assert!(called.get());
        assert_eq!(*received.borrow(), [0x01, 0x02]);
        assert_eq!(response.message_type, 0x80);
        assert_eq!(response.return_code, 0x00);
        assert_eq!(response.service_id, 0x1000);
        assert_eq!(response.method_id, 0x0001);
        assert_eq!(response.client_id, 0x0042);
        assert_eq!(response.payload, [0xAA]);
    }

    #[test]
    fn unregistered_method_returns_error() {
        let sys = system();
        sys.init();
        let request = SomeIpMessage { service_id: 0x9999, method_id: 0x0001, ..Default::default() };
        let response = sys.dispatch(&request);
        assert_eq!(response.message_type, 0x81);
        assert_eq!(response.return_code, 0x05);
    }

    #[test]
    fn event_publish() {
        let sys = system();
        sys.init();
        sys.register_event(0x1000, 0x8001, 0x0001);
        sys.send_event(0x1000, 0x8001, &[0x01, 0x00, 0x01]);
        let events = sys.sent_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].service_id, 0x1000);
        assert_eq!(events[0].method_id, 0x8001);
        assert_eq!(events[0].message_type, 0x02);
        assert_eq!(events[0].return_code, 0x00);
        assert_eq!(events[0].payload, [0x01, 0x00, 0x01]);
    }

    #[test]
    fn response_building() {
        let sys = system();
        sys.init();
        let response = SomeIpMessage {
            service_id: 0x1000,
            method_id: 0x0001,
            client_id: 0x0042,
            session_id: 0x0007,
            message_type: 0x80,
            return_code: 0x00,
            payload: alloc::vec![0xDE, 0xAD],
        };
        sys.send_response(&response);
        let responses = sys.sent_responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0], response);
    }

    #[test]
    fn shutdown_clears_registrations() {
        let sys = system();
        sys.init();
        sys.register_method(0x1000, 0x0001, Box::new(|req| req.clone()));
        sys.shutdown();
        let request = SomeIpMessage { service_id: 0x1000, method_id: 0x0001, ..Default::default() };
        assert_eq!(sys.dispatch(&request).message_type, 0x81);
    }
}
