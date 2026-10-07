// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Port of `libs/platform/diagnostics`: the UDS services the ECU answers
//! (DiagnosticSessionControl, ReadDataByIdentifier, InputOutputControlByIdentifier,
//! ReadDTCInformation), the diagnostic trouble code store, the transport layer a request
//! arrives on, and the DoIP message codec.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod doip;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

use body_ecu_ports::DiagDataProvider;

/// A diagnostic request: the service and its parameters.
pub type DiagRequest = Vec<u8>;
/// A diagnostic response.
pub type DiagResponse = Vec<u8>;
/// What answers a request.
pub type DiagRequestHandler = Box<dyn Fn(&[u8]) -> DiagResponse>;

/// A transport requests arrive on (`ITransportLayer.h`).
pub trait TransportLayer {
    /// Answer every request with `handler`.
    fn set_request_handler(&self, handler: DiagRequestHandler);
    /// Send `response` to the tester.
    fn send_response(&self, response: &[u8]);
}

/// A diagnostic trouble code (`DtcStore.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dtc {
    /// The code.
    pub code: u32,
    /// Its status.
    pub status_mask: u8,
}

/// The trouble codes stored (`DtcStore`).
#[derive(Default)]
pub struct DtcStore {
    dtcs: RefCell<Vec<Dtc>>,
}

// SAFETY: the store is read and written from the diagnostic transports, one request at
// a time, as the C++ store is with no lock.
unsafe impl Sync for DtcStore {}

impl DtcStore {
    /// An empty store.
    pub const fn new() -> Self {
        Self { dtcs: RefCell::new(Vec::new()) }
    }

    /// Store `code` with `status_mask`, replacing its status when it is stored already.
    pub fn store(&self, code: u32, status_mask: u8) {
        let mut dtcs = self.dtcs.borrow_mut();
        for dtc in dtcs.iter_mut() {
            if dtc.code == code {
                dtc.status_mask = status_mask;
                return;
            }
        }
        dtcs.push(Dtc { code, status_mask });
    }

    /// Forget every code.
    pub fn clear(&self) {
        self.dtcs.borrow_mut().clear();
    }

    /// Every code, in the order stored.
    pub fn get_all(&self) -> Vec<Dtc> {
        self.dtcs.borrow().clone()
    }
}

/// The diagnostic session (`UdsServiceHandler.h`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DiagSession {
    /// The default session.
    Default = 0x01,
    /// The extended session, which allows I/O control.
    Extended = 0x03,
}

const SID_DIAG_SESSION_CONTROL: u8 = 0x10;
const SID_READ_DATA_BY_ID: u8 = 0x22;
const SID_IO_CONTROL: u8 = 0x2F;
const SID_READ_DTC: u8 = 0x19;

const NRC_SERVICE_NOT_SUPPORTED: u8 = 0x11;
const NRC_SUB_FUNCTION_NOT_SUPPORTED: u8 = 0x12;
const NRC_REQUEST_OUT_OF_RANGE: u8 = 0x31;
const NRC_CONDITIONS_NOT_CORRECT: u8 = 0x22;
const POSITIVE_RESPONSE_OFFSET: u8 = 0x40;

/// The UDS service handler implementing 0x10, 0x22, 0x2F, 0x19 (`UdsServiceHandler`).
pub struct UdsServiceHandler {
    dtc_store: &'static DtcStore,
    providers: RefCell<Vec<&'static dyn DiagDataProvider>>,
    session: Cell<DiagSession>,
}

impl UdsServiceHandler {
    /// A handler reporting the codes in `dtc_store`, in the default session.
    pub const fn new(dtc_store: &'static DtcStore) -> Self {
        Self {
            dtc_store,
            providers: RefCell::new(Vec::new()),
            session: Cell::new(DiagSession::Default),
        }
    }

    /// Ask `provider` for data identifiers too.
    pub fn add_provider(&self, provider: &'static dyn DiagDataProvider) {
        self.providers.borrow_mut().push(provider);
    }

    /// The response to `request`.
    pub fn handle_request(&self, request: &[u8]) -> DiagResponse {
        let Some(&sid) = request.first() else {
            return Self::negative_response(0x00, NRC_SERVICE_NOT_SUPPORTED);
        };
        match sid {
            SID_DIAG_SESSION_CONTROL => self.handle_diag_session_control(request),
            SID_READ_DATA_BY_ID => self.handle_read_data_by_id(request),
            SID_IO_CONTROL => self.handle_io_control(request),
            SID_READ_DTC => self.handle_read_dtc(request),
            _ => Self::negative_response(sid, NRC_SERVICE_NOT_SUPPORTED),
        }
    }

    /// The session.
    pub fn current_session(&self) -> DiagSession {
        self.session.get()
    }

    fn handle_diag_session_control(&self, request: &[u8]) -> DiagResponse {
        if request.len() < 2 {
            return Self::negative_response(
                SID_DIAG_SESSION_CONTROL,
                NRC_SUB_FUNCTION_NOT_SUPPORTED,
            );
        }

        let sub = request[1];
        if sub == 0x01 {
            self.session.set(DiagSession::Default);
        } else if sub == 0x03 {
            self.session.set(DiagSession::Extended);
        } else {
            return Self::negative_response(
                SID_DIAG_SESSION_CONTROL,
                NRC_SUB_FUNCTION_NOT_SUPPORTED,
            );
        }

        alloc::vec![SID_DIAG_SESSION_CONTROL + POSITIVE_RESPONSE_OFFSET, sub]
    }

    fn handle_read_data_by_id(&self, request: &[u8]) -> DiagResponse {
        if request.len() < 3 {
            return Self::negative_response(SID_READ_DATA_BY_ID, NRC_REQUEST_OUT_OF_RANGE);
        }

        let did = u16::from_be_bytes([request[1], request[2]]);

        for provider in self.providers.borrow().iter() {
            if let Some(data) = provider.read_data(did) {
                let mut resp = alloc::vec![
                    SID_READ_DATA_BY_ID + POSITIVE_RESPONSE_OFFSET,
                    request[1],
                    request[2]
                ];
                resp.extend_from_slice(&data.data);
                return resp;
            }
        }

        Self::negative_response(SID_READ_DATA_BY_ID, NRC_REQUEST_OUT_OF_RANGE)
    }

    fn handle_io_control(&self, request: &[u8]) -> DiagResponse {
        if self.session.get() != DiagSession::Extended {
            return Self::negative_response(SID_IO_CONTROL, NRC_CONDITIONS_NOT_CORRECT);
        }

        if request.len() < 4 {
            return Self::negative_response(SID_IO_CONTROL, NRC_REQUEST_OUT_OF_RANGE);
        }

        let did = u16::from_be_bytes([request[1], request[2]]);
        let control_param = &request[3..];

        for provider in self.providers.borrow().iter() {
            if provider.io_control(did, control_param) {
                return alloc::vec![
                    SID_IO_CONTROL + POSITIVE_RESPONSE_OFFSET,
                    request[1],
                    request[2]
                ];
            }
        }

        Self::negative_response(SID_IO_CONTROL, NRC_REQUEST_OUT_OF_RANGE)
    }

    fn handle_read_dtc(&self, request: &[u8]) -> DiagResponse {
        if request.len() < 2 {
            return Self::negative_response(SID_READ_DTC, NRC_SUB_FUNCTION_NOT_SUPPORTED);
        }

        let mut resp = alloc::vec![SID_READ_DTC + POSITIVE_RESPONSE_OFFSET, request[1]];

        for dtc in self.dtc_store.get_all() {
            resp.push(((dtc.code >> 16) & 0xFF) as u8);
            resp.push(((dtc.code >> 8) & 0xFF) as u8);
            resp.push((dtc.code & 0xFF) as u8);
            resp.push(dtc.status_mask);
        }

        resp
    }

    fn negative_response(sid: u8, nrc: u8) -> DiagResponse {
        alloc::vec![0x7F, sid, nrc]
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_ports::mock::{MockDiagDataProvider, leak};

    use super::*;

    struct Fixture {
        provider: &'static MockDiagDataProvider,
        dtc_store: &'static DtcStore,
        handler: &'static UdsServiceHandler,
    }

    fn fixture() -> Fixture {
        let provider = leak(MockDiagDataProvider::new());
        let dtc_store = leak(DtcStore::new());
        let handler = leak(UdsServiceHandler::new(dtc_store));
        handler.add_provider(provider);
        Fixture { provider, dtc_store, handler }
    }

    #[test]
    fn read_data_by_id_lighting() {
        let f = fixture();
        f.provider.data.borrow_mut().insert(0xF100, alloc::vec![1, 0, 1]);
        let resp = f.handler.handle_request(&[0x22, 0xF1, 0x00]);
        assert_eq!(resp, [0x62, 0xF1, 0x00, 1, 0, 1]);
    }

    #[test]
    fn read_data_by_id_door() {
        let f = fixture();
        f.provider.data.borrow_mut().insert(0xF101, alloc::vec![0x01]);
        let resp = f.handler.handle_request(&[0x22, 0xF1, 0x01]);
        assert_eq!(resp[0], 0x62);
        assert_eq!(resp[3], 0x01);
    }

    #[test]
    fn read_data_by_id_mode() {
        let f = fixture();
        f.provider.data.borrow_mut().insert(0xF102, alloc::vec![0x02]);
        let resp = f.handler.handle_request(&[0x22, 0xF1, 0x02]);
        assert_eq!(resp[0], 0x62);
        assert_eq!(resp[3], 0x02);
    }

    #[test]
    fn unsupported_did() {
        let f = fixture();
        assert_eq!(f.handler.handle_request(&[0x22, 0xFF, 0xFF]), [0x7F, 0x22, 0x31]);
    }

    #[test]
    fn io_control_in_extended_session() {
        let f = fixture();
        f.handler.handle_request(&[0x10, 0x03]);
        f.provider.controllable.borrow_mut().push(0xF100);
        let resp = f.handler.handle_request(&[0x2F, 0xF1, 0x00, 0x00, 0x01]);
        assert_eq!(resp, [0x6F, 0xF1, 0x00]);
        assert_eq!(f.provider.controls.borrow().as_slice(), [(0xF100, alloc::vec![0x00, 0x01])]);
    }

    #[test]
    fn io_control_rejected_in_default_session() {
        let f = fixture();
        assert_eq!(f.handler.current_session(), DiagSession::Default);
        assert_eq!(f.handler.handle_request(&[0x2F, 0xF1, 0x00, 0x00, 0x01]), [0x7F, 0x2F, 0x22]);
    }

    #[test]
    fn read_dtc_with_stored_dtcs() {
        let f = fixture();
        f.dtc_store.store(0x010203, 0x09);
        f.dtc_store.store(0x040506, 0x27);
        let resp = f.handler.handle_request(&[0x19, 0x02]);
        assert_eq!(resp, [0x59, 0x02, 0x01, 0x02, 0x03, 0x09, 0x04, 0x05, 0x06, 0x27]);
    }

    #[test]
    fn read_dtc_no_dtcs() {
        let f = fixture();
        assert_eq!(f.handler.handle_request(&[0x19, 0x02]), [0x59, 0x02]);
    }

    #[test]
    fn session_control() {
        let f = fixture();
        assert_eq!(f.handler.current_session(), DiagSession::Default);
        assert_eq!(f.handler.handle_request(&[0x10, 0x03])[0], 0x50);
        assert_eq!(f.handler.current_session(), DiagSession::Extended);
        assert_eq!(f.handler.handle_request(&[0x10, 0x01])[0], 0x50);
        assert_eq!(f.handler.current_session(), DiagSession::Default);
    }

    #[test]
    fn unsupported_service() {
        let f = fixture();
        assert_eq!(f.handler.handle_request(&[0xFF]), [0x7F, 0xFF, 0x11]);
    }

    #[test]
    fn a_stored_code_keeps_its_place_and_takes_the_new_status() {
        let store = DtcStore::new();
        store.store(1, 1);
        store.store(2, 2);
        store.store(1, 9);
        assert_eq!(
            store.get_all(),
            [Dtc { code: 1, status_mask: 9 }, Dtc { code: 2, status_mask: 2 }]
        );
        store.clear();
        assert!(store.get_all().is_empty());
    }
}
