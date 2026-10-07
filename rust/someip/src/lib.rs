// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The part of SOME/IP the Body ECU's server uses, as the OpenSOME/IP stack provides it
//! to the C++ firmware: the 16-byte header and its validation (`someip/message.h`), and
//! UDP endpoints (`transport/endpoint.h`). Service discovery, TP segmentation, E2E
//! protection, TCP, and the RPC and event layers are not used by the firmware under
//! emulation (`enable_sd` is off there) and are not ported.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt;

/// The protocol version every message carries.
pub const PROTOCOL_VERSION: u8 = 0x01;
/// The interface version every message carries.
pub const INTERFACE_VERSION: u8 = 0x01;
/// The header's length.
pub const HEADER_SIZE: usize = 16;
/// The header's length field counts the bytes after it: 8 of header, then the payload.
const LENGTH_AFTER_FIELD: u32 = 8;

/// Message types (`someip/types.h`).
pub mod message_type {
    /// A request expecting a response.
    pub const REQUEST: u8 = 0x00;
    /// A request expecting none.
    pub const REQUEST_NO_RETURN: u8 = 0x01;
    /// An event.
    pub const NOTIFICATION: u8 = 0x02;
    /// A response.
    pub const RESPONSE: u8 = 0x80;
    /// An error.
    pub const ERROR: u8 = 0x81;
    /// A segmented request.
    pub const TP_REQUEST: u8 = 0x20;
    /// A segmented request expecting no response.
    pub const TP_REQUEST_NO_RETURN: u8 = 0x21;
    /// A segmented event.
    pub const TP_NOTIFICATION: u8 = 0x22;
    /// A segmented response.
    pub const TP_RESPONSE: u8 = 0xA0;
    /// A segmented error.
    pub const TP_ERROR: u8 = 0xA1;

    /// Whether `value` names a message type.
    pub const fn is_valid(value: u8) -> bool {
        matches!(
            value,
            REQUEST
                | REQUEST_NO_RETURN
                | NOTIFICATION
                | RESPONSE
                | ERROR
                | TP_REQUEST
                | TP_REQUEST_NO_RETURN
                | TP_NOTIFICATION
                | TP_RESPONSE
                | TP_ERROR
        )
    }

    /// Whether `value` is a request (`someip::is_request`).
    pub const fn is_request(value: u8) -> bool {
        matches!(value, REQUEST | REQUEST_NO_RETURN | TP_REQUEST | TP_REQUEST_NO_RETURN)
    }
}

/// Return codes (`someip/types.h`).
pub mod return_code {
    /// No error.
    pub const E_OK: u8 = 0x00;
    /// An unspecified error.
    pub const E_NOT_OK: u8 = 0x01;
    /// The service is unknown.
    pub const E_UNKNOWN_SERVICE: u8 = 0x02;
    /// The method is unknown.
    pub const E_UNKNOWN_METHOD: u8 = 0x03;
    /// The service is not ready.
    pub const E_NOT_READY: u8 = 0x04;
    /// The service is not reachable.
    pub const E_NOT_REACHABLE: u8 = 0x05;
    /// The request timed out.
    pub const E_TIMEOUT: u8 = 0x06;
    /// The protocol version is wrong.
    pub const E_WRONG_PROTOCOL_VERSION: u8 = 0x07;
    /// The interface version is wrong.
    pub const E_WRONG_INTERFACE_VERSION: u8 = 0x08;
    /// The payload is malformed.
    pub const E_MALFORMED_MESSAGE: u8 = 0x09;
    /// The message type is wrong.
    pub const E_WRONG_MESSAGE_TYPE: u8 = 0x0A;
}

/// A SOME/IP message on the wire (`someip::Message`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The service.
    pub service_id: u16,
    /// The method or event.
    pub method_id: u16,
    /// The client.
    pub client_id: u16,
    /// The session.
    pub session_id: u16,
    /// The protocol version, [`PROTOCOL_VERSION`] unless deserialized otherwise.
    pub protocol_version: u8,
    /// The interface version, [`INTERFACE_VERSION`] unless deserialized otherwise.
    pub interface_version: u8,
    /// The message type.
    pub message_type: u8,
    /// The return code.
    pub return_code: u8,
    /// The payload.
    pub payload: Vec<u8>,
}

impl Message {
    /// A message of `message_type` with `return_code` and `payload`.
    pub fn new(
        service_id: u16,
        method_id: u16,
        client_id: u16,
        session_id: u16,
        message_type: u8,
        return_code: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            service_id,
            method_id,
            client_id,
            session_id,
            protocol_version: PROTOCOL_VERSION,
            interface_version: INTERFACE_VERSION,
            message_type,
            return_code,
            payload,
        }
    }

    /// Whether this is a request.
    pub const fn is_request(&self) -> bool {
        message_type::is_request(self.message_type)
    }

    /// The message's bytes: the header in network byte order, then the payload.
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(HEADER_SIZE + self.payload.len());
        data.extend_from_slice(&self.service_id.to_be_bytes());
        data.extend_from_slice(&self.method_id.to_be_bytes());
        let length = LENGTH_AFTER_FIELD + self.payload.len() as u32;
        data.extend_from_slice(&length.to_be_bytes());
        data.extend_from_slice(&self.client_id.to_be_bytes());
        data.extend_from_slice(&self.session_id.to_be_bytes());
        data.push(self.protocol_version);
        data.push(self.interface_version);
        data.push(self.message_type);
        data.push(self.return_code);
        data.extend_from_slice(&self.payload);
        data
    }

    /// The message `data` holds, or `None` when it is malformed: too short, an unknown
    /// message type, another protocol or interface version, a length that does not
    /// match the bytes, or a notification with an error code (the checks
    /// `Message::deserialize` and `is_valid` make).
    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < HEADER_SIZE {
            return None;
        }
        let u16_at = |at: usize| u16::from_be_bytes([data[at], data[at + 1]]);
        let length = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let message = Self {
            service_id: u16_at(0),
            method_id: u16_at(2),
            client_id: u16_at(8),
            session_id: u16_at(10),
            protocol_version: data[12],
            interface_version: data[13],
            message_type: data[14],
            return_code: data[15],
            payload: data[HEADER_SIZE..].to_vec(),
        };
        if !message_type::is_valid(message.message_type)
            || message.protocol_version != PROTOCOL_VERSION
            || message.interface_version != INTERFACE_VERSION
            || length != LENGTH_AFTER_FIELD + message.payload.len() as u32
        {
            return None;
        }
        if matches!(
            message.message_type,
            message_type::NOTIFICATION | message_type::TP_NOTIFICATION
        ) && message.return_code != return_code::E_OK
        {
            return None;
        }
        Some(message)
    }
}

/// A UDP endpoint (`transport::Endpoint`): an IPv4 address and a port.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Endpoint {
    /// The address.
    pub address: [u8; 4],
    /// The port.
    pub port: u16,
}

impl Endpoint {
    /// `address:port`.
    pub const fn new(address: [u8; 4], port: u16) -> Self {
        Self { address, port }
    }

    /// Every address, as a server binds to.
    pub const ANY: [u8; 4] = [0, 0, 0, 0];
}

impl fmt::Display for Endpoint {
    /// `udp://a.b.c.d:port`, as `Endpoint::to_string` prints a UDP endpoint.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a, b, c, d] = self.address;
        write!(f, "udp://{a}.{b}.{c}.{d}:{}", self.port)
    }
}

/// A UDP socket the server receives on and sends from.
pub trait UdpSocket {
    /// Send `data` to `to`; returns whether every byte went.
    fn send_to(&self, data: &[u8], to: &Endpoint) -> bool;
    /// The next datagram waiting, into `buf`: its length and sender, or `None` when
    /// nothing waits.
    fn recv_from(&self, buf: &mut [u8]) -> Option<(usize, Endpoint)>;
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::string::ToString;

    use super::*;

    #[test]
    fn a_request_round_trips_through_the_wire_format() {
        let request =
            Message::new(0x1004, 0x0001, 0x0001, 0x0001, message_type::REQUEST, 0, alloc::vec![]);
        let bytes = request.serialize();
        assert_eq!(bytes, [0x10, 0x04, 0x00, 0x01, 0, 0, 0, 8, 0, 1, 0, 1, 1, 1, 0, 0]);
        assert_eq!(Message::deserialize(&bytes), Some(request));
        let with_payload =
            Message::new(0x1000, 0x0001, 0, 2, message_type::REQUEST, 0, alloc::vec![0, 1]);
        assert_eq!(with_payload.serialize()[4..8], [0, 0, 0, 10]);
        assert_eq!(Message::deserialize(&with_payload.serialize()), Some(with_payload));
    }

    #[test]
    fn malformed_messages_are_refused() {
        let good = Message::new(1, 2, 3, 4, message_type::REQUEST, 0, alloc::vec![9]).serialize();
        assert!(Message::deserialize(&good[..15]).is_none(), "too short");
        let mut bad = good.clone();
        bad[14] = 0x55;
        assert!(Message::deserialize(&bad).is_none(), "unknown type");
        let mut bad = good.clone();
        bad[12] = 2;
        assert!(Message::deserialize(&bad).is_none(), "protocol version");
        let mut bad = good.clone();
        bad[13] = 2;
        assert!(Message::deserialize(&bad).is_none(), "interface version");
        let mut bad = good.clone();
        bad[7] = 20;
        assert!(Message::deserialize(&bad).is_none(), "length");
        let mut bad = good.clone();
        bad[14] = message_type::NOTIFICATION;
        bad[15] = 1;
        assert!(Message::deserialize(&bad).is_none(), "a notification with an error");
    }

    #[test]
    fn request_types_and_endpoints() {
        assert!(message_type::is_request(message_type::REQUEST));
        assert!(message_type::is_request(message_type::REQUEST_NO_RETURN));
        assert!(!message_type::is_request(message_type::RESPONSE));
        assert!(!message_type::is_request(message_type::NOTIFICATION));
        let endpoint = Endpoint::new([192, 168, 100, 1], 40000);
        assert_eq!(endpoint.to_string(), "udp://192.168.100.1:40000");
        assert!(Endpoint::new([10, 0, 0, 1], 1) < Endpoint::new([10, 0, 0, 2], 0));
    }
}
