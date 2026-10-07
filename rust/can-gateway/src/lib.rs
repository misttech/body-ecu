// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/platform/can-gateway`: a table of mappings between SOME/IP methods or
//! events and CAN identifiers, and the gateway that translates both ways.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use body_ecu_ports::{CanBus, CanFrame, SomeIpMessage, SomeIpService};

/// Which way a mapping translates (`ServiceMapping.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GatewayDirection {
    /// A SOME/IP method call becomes a CAN frame.
    #[default]
    SomeIpToCan,
    /// A CAN frame becomes a SOME/IP event.
    CanToSomeIp,
}

/// One mapping (`ServiceMapping`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServiceMapping {
    /// Its name, for configuration.
    pub name: String,
    /// Its direction.
    pub direction: GatewayDirection,
    /// The SOME/IP service.
    pub someip_service_id: u16,
    /// The method, SOME/IP to CAN.
    pub someip_method_id: u16,
    /// The event, CAN to SOME/IP.
    pub someip_event_id: u16,
    /// The event's group.
    pub someip_eventgroup_id: u16,
    /// The CAN identifier.
    pub can_id: u32,
    /// The CAN data length.
    pub can_dlc: u8,
}

/// The translations (`MessageTranslator`).
pub struct MessageTranslator;

impl MessageTranslator {
    /// A frame of `can_id` carrying `someip_payload`, `dlc` bytes of it at most.
    pub fn someip_to_can_frame(can_id: u32, dlc: u8, someip_payload: &[u8]) -> CanFrame {
        let mut frame = CanFrame { id: can_id, ..CanFrame::default() };
        frame.dlc = dlc.min(frame.data.len() as u8);
        let copy_len = usize::from(frame.dlc).min(someip_payload.len());
        frame.data[..copy_len].copy_from_slice(&someip_payload[..copy_len]);
        frame
    }

    /// The payload a frame carries: its `dlc` data bytes.
    pub fn can_frame_to_someip_payload(frame: &CanFrame) -> Vec<u8> {
        let len = usize::from(frame.dlc).min(frame.data.len());
        frame.data[..len].to_vec()
    }
}

/// The gateway (`CanGateway`).
pub struct CanGateway {
    can: &'static dyn CanBus,
    someip: &'static dyn SomeIpService,
    mappings: RefCell<Vec<ServiceMapping>>,
    someip_to_can_index: RefCell<BTreeMap<u32, usize>>,
    can_to_someip_index: RefCell<BTreeMap<u32, usize>>,
    running: Cell<bool>,
    rx_registered: Cell<bool>,
}

impl CanGateway {
    /// A gateway between `can` and `someip` with no mappings.
    pub const fn new(can: &'static dyn CanBus, someip: &'static dyn SomeIpService) -> Self {
        Self {
            can,
            someip,
            mappings: RefCell::new(Vec::new()),
            someip_to_can_index: RefCell::new(BTreeMap::new()),
            can_to_someip_index: RefCell::new(BTreeMap::new()),
            running: Cell::new(false),
            rx_registered: Cell::new(false),
        }
    }

    /// Add `mapping`.
    pub fn add_mapping(&self, mapping: ServiceMapping) {
        self.mappings.borrow_mut().push(mapping);
    }

    /// Register the mappings' methods and events and start translating.
    pub fn start(&'static self) {
        self.build_indices();

        for m in self.mappings.borrow().iter() {
            if m.direction == GatewayDirection::SomeIpToCan {
                self.someip.register_method(
                    m.someip_service_id,
                    m.someip_method_id,
                    Box::new(move |msg| {
                        self.on_someip_message(msg);
                        let mut resp = msg.clone();
                        resp.message_type = 0x80;
                        resp.return_code = 0x00;
                        resp
                    }),
                );
            }
            if m.direction == GatewayDirection::CanToSomeIp {
                self.someip.register_event(
                    m.someip_service_id,
                    m.someip_event_id,
                    m.someip_eventgroup_id,
                );
            }
        }

        if !self.rx_registered.get() {
            self.can.add_rx_callback(Box::new(move |frame| self.on_can_frame(frame)));
            self.rx_registered.set(true);
        }

        self.running.set(true);
    }

    /// Stop translating.
    pub fn stop(&self) {
        self.running.set(false);
    }

    /// Whether the gateway translates.
    pub fn is_running(&self) -> bool {
        self.running.get()
    }

    /// A SOME/IP method call to translate to a frame.
    pub fn on_someip_message(&self, msg: &SomeIpMessage) {
        let key = Self::someip_key(msg.service_id, msg.method_id);
        let index = self.someip_to_can_index.borrow();
        let Some(&at) = index.get(&key) else { return };
        let mappings = self.mappings.borrow();
        let mapping = &mappings[at];
        let frame =
            MessageTranslator::someip_to_can_frame(mapping.can_id, mapping.can_dlc, &msg.payload);
        self.can.send(&frame);
    }

    /// A frame to translate to a SOME/IP event.
    pub fn on_can_frame(&self, frame: &CanFrame) {
        let index = self.can_to_someip_index.borrow();
        let Some(&at) = index.get(&frame.id) else { return };
        let mappings = self.mappings.borrow();
        let mapping = &mappings[at];
        let payload = MessageTranslator::can_frame_to_someip_payload(frame);
        self.someip.send_event(mapping.someip_service_id, mapping.someip_event_id, &payload);
    }

    fn build_indices(&self) {
        let mut someip_to_can = self.someip_to_can_index.borrow_mut();
        let mut can_to_someip = self.can_to_someip_index.borrow_mut();
        someip_to_can.clear();
        can_to_someip.clear();
        for (at, m) in self.mappings.borrow().iter().enumerate() {
            if m.direction == GatewayDirection::SomeIpToCan {
                someip_to_can.insert(Self::someip_key(m.someip_service_id, m.someip_method_id), at);
            } else {
                can_to_someip.insert(m.can_id, at);
            }
        }
    }

    const fn someip_key(service_id: u16, method_id: u16) -> u32 {
        ((service_id as u32) << 16) | method_id as u32
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_ports::mock::{MockCanBus, MockSomeIpService, leak};

    use super::*;

    fn mapping(
        name: &str,
        direction: GatewayDirection,
        service: u16,
        method_or_event: u16,
        can_id: u32,
        dlc: u8,
    ) -> ServiceMapping {
        let mut m = ServiceMapping {
            name: name.into(),
            direction,
            someip_service_id: service,
            can_id,
            can_dlc: dlc,
            ..Default::default()
        };
        match direction {
            GatewayDirection::SomeIpToCan => m.someip_method_id = method_or_event,
            GatewayDirection::CanToSomeIp => {
                m.someip_event_id = method_or_event;
                m.someip_eventgroup_id = 0x0001;
            }
        }
        m
    }

    struct Fixture {
        can: &'static MockCanBus,
        someip: &'static MockSomeIpService,
        gw: &'static CanGateway,
    }

    fn fixture() -> Fixture {
        let can = leak(MockCanBus::new());
        let someip = leak(MockSomeIpService::new());
        let gw = leak(CanGateway::new(can, someip));
        gw.add_mapping(mapping(
            "light_command",
            GatewayDirection::SomeIpToCan,
            0x1000,
            0x0001,
            0x200,
            4,
        ));
        gw.add_mapping(mapping(
            "door_status",
            GatewayDirection::CanToSomeIp,
            0x1001,
            0x8001,
            0x300,
            2,
        ));
        Fixture { can, someip, gw }
    }

    #[test]
    fn someip_to_can_translation() {
        let f = fixture();
        f.gw.start();
        assert!(f.someip.method_count() >= 1);
        assert!(!f.someip.events.borrow().is_empty());
        assert_eq!(f.can.callback_count(), 1);
        let msg = SomeIpMessage {
            service_id: 0x1000,
            method_id: 0x0001,
            payload: alloc::vec![1, 2, 3, 4],
            ..Default::default()
        };
        f.gw.on_someip_message(&msg);
        let sent = f.can.sent.borrow();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].id, 0x200);
        assert_eq!(sent[0].dlc, 4);
        assert_eq!(&sent[0].data[..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn can_to_someip_translation() {
        let f = fixture();
        f.gw.start();
        let mut frame = CanFrame { id: 0x300, dlc: 2, ..Default::default() };
        frame.data[0] = 0xAA;
        frame.data[1] = 0xBB;
        f.gw.on_can_frame(&frame);
        assert_eq!(f.someip.take_sent_events(), [(0x1001, 0x8001, alloc::vec![0xAA, 0xBB])]);
    }

    #[test]
    fn unmapped_message_drop() {
        let f = fixture();
        f.gw.start();
        let msg = SomeIpMessage {
            service_id: 0x9999,
            method_id: 0x0001,
            payload: alloc::vec![0xFF],
            ..Default::default()
        };
        f.gw.on_someip_message(&msg);
        assert!(f.can.sent.borrow().is_empty());
    }

    #[test]
    fn payload_serialization() {
        let frame = MessageTranslator::someip_to_can_frame(0x200, 4, &[0x11, 0x22, 0x33]);
        assert_eq!(frame.id, 0x200);
        assert_eq!(frame.dlc, 4);
        assert_eq!(&frame.data[..3], &[0x11, 0x22, 0x33]);
        let roundtrip = MessageTranslator::can_frame_to_someip_payload(&frame);
        assert_eq!(roundtrip, [0x11, 0x22, 0x33, 0x00]);
    }

    #[test]
    fn gateway_lifecycle() {
        let f = fixture();
        assert!(!f.gw.is_running());
        f.gw.start();
        assert!(f.gw.is_running());
        f.gw.stop();
        assert!(!f.gw.is_running());
    }

    #[test]
    fn bidirectional_mapping() {
        let can = leak(MockCanBus::new());
        let someip = leak(MockSomeIpService::new());
        let gw = leak(CanGateway::new(can, someip));
        gw.add_mapping(mapping(
            "sensor_relay",
            GatewayDirection::SomeIpToCan,
            0x2000,
            0x0010,
            0x400,
            3,
        ));
        let mut back =
            mapping("sensor_relay_return", GatewayDirection::CanToSomeIp, 0x2000, 0x8010, 0x401, 3);
        back.someip_eventgroup_id = 0x0002;
        gw.add_mapping(back);
        gw.start();
        assert_eq!(can.callback_count(), 1);

        let msg = SomeIpMessage {
            service_id: 0x2000,
            method_id: 0x0010,
            payload: alloc::vec![0xAA, 0xBB, 0xCC],
            ..Default::default()
        };
        gw.on_someip_message(&msg);
        assert_eq!(can.sent.borrow()[0].id, 0x400);
        assert_eq!(can.sent.borrow()[0].data[0], 0xAA);

        let mut frame = CanFrame { id: 0x401, dlc: 3, ..Default::default() };
        frame.data[..3].copy_from_slice(&[0x11, 0x22, 0x33]);
        gw.on_can_frame(&frame);
        assert_eq!(someip.take_sent_events(), [(0x2000, 0x8010, alloc::vec![0x11, 0x22, 0x33])]);
    }

    #[test]
    fn max_dlc_clamped_in_translation() {
        let frame = MessageTranslator::someip_to_can_frame(0x100, 255, &[0x01, 0x02]);
        assert!(usize::from(frame.dlc) <= frame.data.len());
        let payload = MessageTranslator::can_frame_to_someip_payload(&frame);
        assert!(payload.len() <= frame.data.len());
    }

    #[test]
    fn zero_dlc_is_valid() {
        let frame = MessageTranslator::someip_to_can_frame(0x100, 0, &[]);
        assert_eq!(frame.dlc, 0);
        assert!(MessageTranslator::can_frame_to_someip_payload(&frame).is_empty());
    }

    #[test]
    fn classic_can_max_dlc() {
        let frame = MessageTranslator::someip_to_can_frame(0x100, 8, &[0xAA; 8]);
        assert_eq!(frame.dlc, 8);
    }

    #[test]
    fn mapping_load_from_config() {
        let can = leak(MockCanBus::new());
        let someip = leak(MockSomeIpService::new());
        let gw = leak(CanGateway::new(can, someip));
        gw.add_mapping(mapping(
            "cfg_test_a",
            GatewayDirection::SomeIpToCan,
            0x3000,
            0x0001,
            0x500,
            8,
        ));
        gw.add_mapping(mapping(
            "cfg_test_b",
            GatewayDirection::CanToSomeIp,
            0x3001,
            0x8001,
            0x501,
            4,
        ));
        gw.start();
        assert_eq!(someip.method_count(), 1);
        assert_eq!(someip.events.borrow().as_slice(), [(0x3001, 0x8001, 0x0001)]);
        assert_eq!(can.callback_count(), 1);
        assert!(gw.is_running());

        let msg = SomeIpMessage {
            service_id: 0x3000,
            method_id: 0x0001,
            payload: alloc::vec![0xDE, 0xAD, 0xBE, 0xEF, 1, 2, 3, 4],
            ..Default::default()
        };
        gw.on_someip_message(&msg);
        let sent = can.sent.borrow();
        assert_eq!(sent[0].id, 0x500);
        assert_eq!(sent[0].dlc, 8);
        assert_eq!(sent[0].data[0], 0xDE);
        assert_eq!(sent[0].data[3], 0xEF);
    }
}
