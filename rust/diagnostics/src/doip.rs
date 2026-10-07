// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The DoIP (ISO 13400-2:2019) message codec (`DoIpProtocol.h`): the generic header and
//! the routing activation, diagnostic message, and acknowledgement payloads.

use alloc::vec::Vec;

/// The protocol version.
pub const PROTOCOL_VERSION: u8 = 0x03;
/// Its inverse, which follows it in every header.
pub const INVERSE_VERSION: u8 = !PROTOCOL_VERSION;
/// The TCP port.
pub const DEFAULT_PORT: u16 = 13400;
/// The generic header's length.
pub const HEADER_LEN: usize = 8;

/// A payload type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum PayloadType {
    /// Generic negative acknowledgement.
    GenericNack = 0x0000,
    /// Vehicle identification request.
    VehicleIdentRequest = 0x0001,
    /// Vehicle identification response.
    VehicleIdentResponse = 0x0004,
    /// Routing activation request.
    RoutingActivationRequest = 0x0005,
    /// Routing activation response.
    RoutingActivationResponse = 0x0006,
    /// Alive check request.
    AliveCheckRequest = 0x0007,
    /// Alive check response.
    AliveCheckResponse = 0x0008,
    /// Diagnostic message.
    DiagnosticMessage = 0x8001,
    /// Diagnostic message positive acknowledgement.
    DiagnosticPositiveAck = 0x8002,
    /// Diagnostic message negative acknowledgement.
    DiagnosticNegativeAck = 0x8003,
}

/// A routing activation response code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RoutingActivationCode {
    /// Unknown source address.
    UnknownSourceAddress = 0x00,
    /// Already active.
    AlreadyActive = 0x02,
    /// Success.
    Success = 0x10,
}

/// A generic header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// The protocol version.
    pub version: u8,
    /// Its inverse.
    pub inverse: u8,
    /// The payload type.
    pub payload_type: u16,
    /// The payload's length.
    pub payload_length: u32,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: PROTOCOL_VERSION,
            inverse: INVERSE_VERSION,
            payload_type: 0,
            payload_length: 0,
        }
    }
}

/// The header at the start of `data`.
pub fn parse_header(data: &[u8; HEADER_LEN]) -> Header {
    Header {
        version: data[0],
        inverse: data[1],
        payload_type: u16::from_be_bytes([data[2], data[3]]),
        payload_length: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
    }
}

/// A header for `payload_len` bytes of `payload_type`.
pub fn serialize_header(payload_type: PayloadType, payload_len: u32) -> Vec<u8> {
    let mut header = alloc::vec![PROTOCOL_VERSION, INVERSE_VERSION];
    header.extend_from_slice(&(payload_type as u16).to_be_bytes());
    header.extend_from_slice(&payload_len.to_be_bytes());
    header
}

/// A routing activation response: tester address, entity address, response code, and
/// four reserved bytes.
pub fn make_routing_activation_response(
    tester_addr: u16,
    entity_addr: u16,
    code: RoutingActivationCode,
) -> Vec<u8> {
    const PAYLOAD_LEN: u32 = 9;
    let mut msg = serialize_header(PayloadType::RoutingActivationResponse, PAYLOAD_LEN);
    msg.extend_from_slice(&tester_addr.to_be_bytes());
    msg.extend_from_slice(&entity_addr.to_be_bytes());
    msg.push(code as u8);
    msg.extend_from_slice(&[0x00; 4]); // reserved / OEM
    msg
}

/// A diagnostic message positive acknowledgement.
pub fn make_diagnostic_ack(source_addr: u16, target_addr: u16, ack_code: u8) -> Vec<u8> {
    const PAYLOAD_LEN: u32 = 5;
    let mut msg = serialize_header(PayloadType::DiagnosticPositiveAck, PAYLOAD_LEN);
    msg.extend_from_slice(&source_addr.to_be_bytes());
    msg.extend_from_slice(&target_addr.to_be_bytes());
    msg.push(ack_code);
    msg
}

/// A diagnostic message carrying `uds_payload`.
pub fn make_diagnostic_message(source_addr: u16, target_addr: u16, uds_payload: &[u8]) -> Vec<u8> {
    let payload_len = 4 + uds_payload.len() as u32;
    let mut msg = serialize_header(PayloadType::DiagnosticMessage, payload_len);
    msg.extend_from_slice(&source_addr.to_be_bytes());
    msg.extend_from_slice(&target_addr.to_be_bytes());
    msg.extend_from_slice(uds_payload);
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_of(msg: &[u8]) -> Header {
        parse_header(msg[..HEADER_LEN].try_into().unwrap())
    }

    #[test]
    fn serialize_and_parse_header() {
        let data = serialize_header(PayloadType::DiagnosticMessage, 42);
        assert_eq!(data.len(), HEADER_LEN);
        let hdr = header_of(&data);
        assert_eq!(hdr.version, PROTOCOL_VERSION);
        assert_eq!(hdr.inverse, INVERSE_VERSION);
        assert_eq!(hdr.payload_type, PayloadType::DiagnosticMessage as u16);
        assert_eq!(hdr.payload_length, 42);
    }

    #[test]
    fn routing_activation_response() {
        let msg = make_routing_activation_response(0x0E00, 0x0E80, RoutingActivationCode::Success);
        assert_eq!(msg.len(), HEADER_LEN + 9);
        let hdr = header_of(&msg);
        assert_eq!(hdr.payload_type, PayloadType::RoutingActivationResponse as u16);
        assert_eq!(hdr.payload_length, 9);
        assert_eq!(
            &msg[HEADER_LEN..HEADER_LEN + 5],
            &[0x0E, 0x00, 0x0E, 0x80, RoutingActivationCode::Success as u8]
        );
    }

    #[test]
    fn diagnostic_message() {
        let uds = [0x22, 0xF1, 0x90];
        let msg = make_diagnostic_message(0x0E80, 0x0E00, &uds);
        assert_eq!(msg.len(), HEADER_LEN + 4 + uds.len());
        let hdr = header_of(&msg);
        assert_eq!(hdr.payload_type, PayloadType::DiagnosticMessage as u16);
        assert_eq!(hdr.payload_length as usize, 4 + uds.len());
        assert_eq!(&msg[HEADER_LEN..], &[0x0E, 0x80, 0x0E, 0x00, 0x22, 0xF1, 0x90]);
    }

    #[test]
    fn diagnostic_ack() {
        let msg = make_diagnostic_ack(0x0E80, 0x0E00, 0x00);
        assert_eq!(msg.len(), HEADER_LEN + 5);
        let hdr = header_of(&msg);
        assert_eq!(hdr.payload_type, PayloadType::DiagnosticPositiveAck as u16);
        assert_eq!(hdr.payload_length, 5);
        assert_eq!(msg[HEADER_LEN + 4], 0x00);
    }

    #[test]
    fn header_version_inverse() {
        assert_eq!(PROTOCOL_VERSION ^ 0xFF, INVERSE_VERSION);
    }
}
