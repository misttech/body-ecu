// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The tests of `libs/adapters/system/tests`: the lifecycle wrappers, the transports, and
//! CAN receive multicast.

use body_ecu_door_lock::{DoorLockConfig, LockState};
use body_ecu_lighting::{LightId, LightingConfig};
use body_ecu_ports::VehicleMode;
use body_ecu_ports::mock::{MockButtonInput, MockGpioPort, leak};
use body_ecu_vehicle_mode::VehicleModeConfig;
use openbsw_lifecycle::LifecycleComponent;

use super::*;

fn someip() -> &'static SomeIpSystem {
    leak(SomeIpSystem::new(SomeIpConfig::default()))
}

#[test]
fn lighting_init_registers_methods_and_events() {
    let gpio = leak(MockGpioPort::new());
    let sys = leak(LightingSystem::new(gpio, someip(), LightingConfig::default()));
    sys.init();
    assert!(!sys.controller().get_light_status()[0]);
}

#[test]
fn lighting_shutdown_turns_off_all_lights() {
    let gpio = leak(MockGpioPort::new());
    let sys = leak(LightingSystem::new(gpio, someip(), LightingConfig::default()));
    sys.init();
    sys.controller().set_light_state(LightId::Headlight, true);
    sys.shutdown();
    assert_eq!(gpio.take_writes(), [(0, true), (0, false), (1, false), (2, false)]);
}

#[test]
fn door_lock_init_and_lifecycle() {
    let gpio = leak(MockGpioPort::new());
    let button = leak(MockButtonInput::new());
    let sys = leak(DoorLockSystem::new(gpio, button, someip(), DoorLockConfig::default(), None));
    sys.init();
    assert_eq!(button.registrations.get(), 1);
    assert_eq!(sys.controller().get_state(), LockState::Unlocked);
    sys.run();
    sys.shutdown();
    assert_eq!(sys.controller().get_state(), LockState::Unlocked);
}

#[test]
fn door_lock_lock_and_shutdown_unlocks() {
    let gpio = leak(MockGpioPort::new());
    let button = leak(MockButtonInput::new());
    let sys = leak(DoorLockSystem::new(gpio, button, someip(), DoorLockConfig::default(), None));
    sys.init();
    sys.controller().lock();
    assert_eq!(sys.controller().get_state(), LockState::Locked);
    sys.shutdown();
    assert_eq!(sys.controller().get_state(), LockState::Unlocked);
    assert!(!gpio.take_writes().is_empty());
}

#[test]
fn vehicle_mode_init_and_lifecycle() {
    let sys = leak(VehicleModeSystem::new(someip(), VehicleModeConfig::default()));
    sys.init();
    assert_eq!(sys.manager().get_mode(), VehicleMode::Off);
    sys.run();
    sys.manager().set_mode(VehicleMode::Accessory);
    assert_eq!(sys.manager().get_mode(), VehicleMode::Accessory);
    sys.shutdown();
    assert_eq!(sys.manager().get_mode(), VehicleMode::Off);
}

#[test]
fn diagnostics_init_and_handle_request() {
    let store = leak(body_ecu_diagnostics::DtcStore::new());
    let sys = leak(DiagnosticsSystem::new(store));
    sys.init();
    // No providers: 0x22 earns a negative response, NRC 0x31 (requestOutOfRange).
    assert_eq!(sys.handler().handle_request(&[0x22, 0xF1, 0x00]), [0x7F, 0x22, 0x31]);
}

#[test]
fn diagnostics_hands_its_handler_to_every_transport() {
    let store = leak(body_ecu_diagnostics::DtcStore::new());
    let sys = leak(DiagnosticsSystem::new(store));
    let doip = leak(DoIpTransport::new(DoIpTransport::DEFAULT_PORT));
    sys.add_transport(doip);
    sys.init();
    doip.init();
    assert!(!doip.is_listening());
    assert!(!doip.is_connected());
    assert_eq!(doip.port(), 13400);
    doip.shutdown();
}

#[cfg(feature = "can")]
mod can {
    use alloc::boxed::Box;
    use core::cell::Cell;

    use body_ecu_can_gateway::{GatewayDirection, ServiceMapping};
    use body_ecu_diagnostics::TransportLayer;
    use body_ecu_ports::CanFrame;
    use body_ecu_ports::mock::{MockCanBus, MockSomeIpService};

    use super::*;

    #[test]
    fn can_gateway_lifecycle_starts_and_stops_gateway() {
        let can = leak(MockCanBus::new());
        let sys = leak(CanGatewaySystem::new(can, someip()));
        sys.add_mapping(ServiceMapping {
            name: "test".into(),
            direction: GatewayDirection::SomeIpToCan,
            someip_service_id: 0x1000,
            someip_method_id: 0x0001,
            can_id: 0x200,
            can_dlc: 4,
            ..Default::default()
        });
        sys.init();
        sys.run();
        assert!(sys.gateway().is_running());
        sys.shutdown();
        assert!(!sys.gateway().is_running());
    }

    #[test]
    fn docan_single_frame_request_dispatch() {
        let can = leak(MockCanBus::new());
        let transport = leak(DoCanTransport::new(can));
        let called = leak(Cell::new(false));
        transport.set_request_handler(Box::new(move |req| {
            called.set(true);
            assert_eq!(req, [0x22, 0xF1, 0x00]);
            alloc::vec![0x62, 0xF1, 0x00, 0x01]
        }));
        transport.init();
        let mut frame =
            CanFrame { id: DoCanTransport::DIAG_RX_CAN_ID, dlc: 4, ..Default::default() };
        frame.data[..4].copy_from_slice(&[0x03, 0x22, 0xF1, 0x00]);
        can.receive(&frame);
        assert!(called.get());
        let sent = can.sent.borrow();
        assert_eq!(sent[0].id, DoCanTransport::DIAG_TX_CAN_ID);
        assert_eq!(sent[0].data[0], 0x04);
        assert_eq!(sent[0].data[1], 0x62);
    }

    #[test]
    fn docan_ignores_non_diag_can_ids() {
        let can = leak(MockCanBus::new());
        let transport = leak(DoCanTransport::new(can));
        let called = leak(Cell::new(false));
        transport.set_request_handler(Box::new(move |_| {
            called.set(true);
            alloc::vec![]
        }));
        transport.init();
        let mut frame = CanFrame { id: 0x999, dlc: 4, ..Default::default() };
        frame.data[..4].copy_from_slice(&[0x03, 0x22, 0xF1, 0x00]);
        can.receive(&frame);
        assert!(!called.get());
    }

    /// Both DoCAN and the CAN gateway must receive frames from the same bus after the
    /// full lifecycle (init + run): the regression test of the CAN receive ownership bug.
    #[test]
    fn both_receive_frames_after_lifecycle_init() {
        let can = leak(MockCanBus::new());
        let someip = leak(MockSomeIpService::new());
        let docan = leak(DoCanTransport::new(can));
        let gateway = leak(body_ecu_can_gateway::CanGateway::new(can, someip));
        gateway.add_mapping(ServiceMapping {
            name: "door_status".into(),
            direction: GatewayDirection::CanToSomeIp,
            someip_service_id: 0x1001,
            someip_event_id: 0x8001,
            someip_eventgroup_id: 0x0001,
            can_id: 0x300,
            can_dlc: 2,
            ..Default::default()
        });
        let diag_handler_called = leak(Cell::new(false));
        docan.set_request_handler(Box::new(move |_| {
            diag_handler_called.set(true);
            alloc::vec![0x62, 0xF1, 0x00, 0x42]
        }));
        docan.init();
        gateway.start();
        assert!(
            can.callback_count() >= 2,
            "both DoCAN and the CAN gateway register receive callbacks"
        );

        let mut diag_frame = CanFrame { id: 0x600, dlc: 4, ..Default::default() };
        diag_frame.data[..4].copy_from_slice(&[0x03, 0x22, 0xF1, 0x00]);
        let mut gw_frame = CanFrame { id: 0x300, dlc: 2, ..Default::default() };
        gw_frame.data[..2].copy_from_slice(&[0xAA, 0xBB]);
        can.receive(&diag_frame);
        can.receive(&gw_frame);

        assert!(
            diag_handler_called.get(),
            "DoCAN's handler receives frames after the gateway starts"
        );
        assert_eq!(someip.take_sent_events(), [(0x1001, 0x8001, alloc::vec![0xAA, 0xBB])]);
    }
}
