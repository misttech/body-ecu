// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! `VehicleInfoProvider`: the VIN and the ECU serial number as diagnostic data.

use alloc::vec::Vec;
use core::cell::RefCell;

use body_ecu_ports::{DiagData, DiagDataProvider};

/// The vehicle information provider.
pub struct VehicleInfoProvider {
    vin: RefCell<[u8; Self::VIN_LENGTH]>,
    ecu_serial: RefCell<Vec<u8>>,
}

// SAFETY: set during startup, read from the diagnostic transports afterwards.
unsafe impl Sync for VehicleInfoProvider {}

impl VehicleInfoProvider {
    /// The VIN's data identifier.
    pub const DID_VIN: u16 = 0xF190;
    /// The ECU serial number's data identifier.
    pub const DID_ECU_SERIAL: u16 = 0xF18C;
    /// A VIN's length.
    pub const VIN_LENGTH: usize = 17;

    /// A provider with a VIN of zeros and no serial number.
    pub const fn new() -> Self {
        Self { vin: RefCell::new([b'0'; Self::VIN_LENGTH]), ecu_serial: RefCell::new(Vec::new()) }
    }

    /// Set the VIN: `vin`'s first 17 bytes, NUL padded when shorter.
    pub fn set_vin(&self, vin: &str) {
        let mut stored = [0_u8; Self::VIN_LENGTH];
        let len = vin.len().min(Self::VIN_LENGTH);
        stored[..len].copy_from_slice(&vin.as_bytes()[..len]);
        *self.vin.borrow_mut() = stored;
    }

    /// Set the serial number.
    pub fn set_ecu_serial(&self, serial: &str) {
        *self.ecu_serial.borrow_mut() = serial.as_bytes().to_vec();
    }
}

impl Default for VehicleInfoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagDataProvider for VehicleInfoProvider {
    fn read_data(&self, did: u16) -> Option<DiagData> {
        if did == Self::DID_VIN {
            return Some(DiagData { did, data: self.vin.borrow().to_vec() });
        }
        let serial = self.ecu_serial.borrow();
        if did == Self::DID_ECU_SERIAL && !serial.is_empty() {
            return Some(DiagData { did, data: serial.clone() });
        }
        None
    }

    fn io_control(&self, _did: u16, _control_param: &[u8]) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use body_ecu_diagnostics::{DtcStore, UdsServiceHandler};
    use body_ecu_ports::mock::leak;

    use super::*;

    #[test]
    fn read_vin_default() {
        let provider = VehicleInfoProvider::new();
        let data = provider.read_data(VehicleInfoProvider::DID_VIN).unwrap();
        assert_eq!(data.did, 0xF190);
        assert_eq!(data.data, [b'0'; 17]);
    }

    #[test]
    fn read_vin_custom() {
        let provider = VehicleInfoProvider::new();
        provider.set_vin("WVWZZZ3CZWE123456");
        let data = provider.read_data(VehicleInfoProvider::DID_VIN).unwrap();
        assert_eq!(data.data, b"WVWZZZ3CZWE123456");
    }

    #[test]
    fn read_ecu_serial() {
        let provider = VehicleInfoProvider::new();
        provider.set_ecu_serial("BECU-001");
        assert_eq!(
            provider.read_data(VehicleInfoProvider::DID_ECU_SERIAL).unwrap().data,
            b"BECU-001"
        );
    }

    #[test]
    fn unknown_did() {
        assert!(VehicleInfoProvider::new().read_data(0xFFFF).is_none());
    }

    #[test]
    fn io_control_always_false() {
        assert!(!VehicleInfoProvider::new().io_control(0xF190, &[]));
    }

    #[test]
    fn read_vin_via_uds_handler() {
        let provider = leak(VehicleInfoProvider::new());
        provider.set_vin("WVWZZZ3CZWE123456");
        let dtc_store = leak(DtcStore::new());
        let handler = UdsServiceHandler::new(dtc_store);
        handler.add_provider(provider);
        let resp = handler.handle_request(&[0x22, 0xF1, 0x90]);
        assert_eq!(&resp[..3], &[0x62, 0xF1, 0x90]);
        assert_eq!(&resp[3..20], b"WVWZZZ3CZWE123456");
    }
}
