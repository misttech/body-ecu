// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The Body ECU's MCU firmware in Rust: the port of `app/src/main.cpp`.
//!
//! Every system the C++ `main` builds is a static here, wired at startup in the same
//! order, logged through Zephyr's log the same way, and registered with OpenBSW's
//! lifecycle manager at the same run levels; the SOME/IP server, the button, and the
//! timers reach Zephyr through `src/zephyr_shim.c`.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

mod allocator;
mod config;
mod log;
mod runtime_monitor;

use alloc::boxed::Box;
use core::cell::Cell;

use body_ecu_adapters::system::{
    DiagnosticsSystem, DoIpTransport, DoorLockSystem, IgnitionSystem, LightingSystem, SomeIpConfig,
    SomeIpSystem, VehicleInfoProvider, VehicleModeSystem,
};
use body_ecu_adapters::zephyr::{ButtonAdapter, GpioAdapter, LocalSignalBus, ZephyrTimerService};
use body_ecu_diagnostics::DtcStore;
use body_ecu_door_lock::{DoorLockConfig, LockState};
use body_ecu_ignition::IgnitionConfig;
use body_ecu_lighting::LightingConfig;
use body_ecu_ports::{
    DiagDataProvider, NullButtonInput, SomeIpMessage, SomeIpService, ids, printk,
};
use body_ecu_vehicle_mode::VehicleModeConfig;
use openbsw_async_zephyr::{LockType, TaskContext, ZephyrAdapter};
use openbsw_bsp_zephyr::system_timer::system_time_us32;
use openbsw_lifecycle::LifecycleManager;

use config::{TASK_COUNT, TASK_NAMES, TASK_SYSADMIN};
use runtime_monitor::RUNTIME_MONITOR;

const MAX_NUM_COMPONENTS: usize = 16;
const MAX_NUM_LEVELS: usize = 8;
const MAX_NUM_COMPONENTS_PER_LEVEL: usize = MAX_NUM_COMPONENTS;

static CONTEXTS: [TaskContext; TASK_COUNT] = [
    TaskContext::new(&CONTEXTS[0]),
    TaskContext::new(&CONTEXTS[1]),
    TaskContext::new(&CONTEXTS[2]),
    TaskContext::new(&CONTEXTS[3]),
    TaskContext::new(&CONTEXTS[4]),
    TaskContext::new(&CONTEXTS[5]),
];
static ASYNC_ADAPTER: ZephyrAdapter<TASK_COUNT> = ZephyrAdapter::new(&CONTEXTS, TASK_NAMES);

static LIFECYCLE_MANAGER: LifecycleManager<
    MAX_NUM_COMPONENTS,
    MAX_NUM_LEVELS,
    MAX_NUM_COMPONENTS_PER_LEVEL,
    LockType,
> = LifecycleManager::new(TASK_SYSADMIN, &system_time_us32);

/// Service discovery is off: the C++ enables it only on real silicon (`enable_sd =
/// real_hw`), and the Rust firmware runs on the emulator, which reports none.
static SOMEIP_SYSTEM: SomeIpSystem = SomeIpSystem::new(SomeIpConfig {
    host: [0, 0, 0, 0],
    port: 30490,
    role: body_ecu_adapters::system::SomeIpRole::Server,
    enable_sd: false,
    sd_multicast: [239, 255, 255, 251],
    sd_port: 30491,
    sd_offer_interval_ms: 5000,
});

static GPIO_ADAPTER: GpioAdapter = GpioAdapter::new();
static BUTTON_ADAPTER: ButtonAdapter = ButtonAdapter::new();
static SIGNAL_BUS: LocalSignalBus = LocalSignalBus::new();
static NULL_BUTTON: NullButtonInput = NullButtonInput;
static LIGHTING: LightingSystem =
    LightingSystem::new(&GPIO_ADAPTER, &SOMEIP_SYSTEM, LightingConfig::DEFAULT);
static DOOR_LOCK: DoorLockSystem = DoorLockSystem::new(
    &GPIO_ADAPTER,
    &NULL_BUTTON,
    &SOMEIP_SYSTEM,
    DoorLockConfig { lock_gpio_pin: 2, ..DoorLockConfig::DEFAULT },
    Some(&SIGNAL_BUS),
);
static TIMER_SERVICE: ZephyrTimerService = ZephyrTimerService::new();
static VEHICLE_MODE: VehicleModeSystem =
    VehicleModeSystem::new(&SOMEIP_SYSTEM, VehicleModeConfig::DEFAULT);
static IGNITION: IgnitionSystem = IgnitionSystem::new(
    &BUTTON_ADAPTER,
    VEHICLE_MODE.manager(),
    &TIMER_SERVICE,
    IgnitionConfig::DEFAULT,
    Some(&SIGNAL_BUS),
);
static VEHICLE_INFO: VehicleInfoProvider = VehicleInfoProvider::new();
static DTC_STORE: DtcStore = DtcStore::new();
static DIAGNOSTICS: DiagnosticsSystem = DiagnosticsSystem::new(&DTC_STORE);
static DOIP_TRANSPORT: DoIpTransport = DoIpTransport::new(DoIpTransport::DEFAULT_PORT);

/// The software-only door lock's state, for a board whose GPIO is not available.
static SW_STATE: SyncCell<LockState> = SyncCell(Cell::new(LockState::Unlocked));

/// A cell the software-only handlers share; they run one at a time on the SOME/IP
/// receive thread.
struct SyncCell<T>(Cell<T>);

// SAFETY: see above.
unsafe impl<T> Sync for SyncCell<T> {}

/// STM32H753 DBGMCU IDCODE register. Real silicon reports DEV_ID = 0x450; Renode
/// typically does not model this peripheral and returns 0.
fn is_real_hardware() -> bool {
    (body_ecu_ffi::dbgmcu_idcode() & 0xFFF) == 0x450
}

/// The response that acknowledges `request` with `payload`.
fn response(request: &SomeIpMessage, payload: alloc::vec::Vec<u8>) -> SomeIpMessage {
    let mut resp = request.clone();
    resp.message_type = 0x80;
    resp.return_code = 0x00;
    resp.payload = payload;
    resp
}

/// The application's entry point, called by `rust_main.c` from Zephyr's `main`.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() {
    body_ecu_ports::console::set_sink(body_ecu_ffi::printk);

    log_inf!("Body ECU starting (Zephyr + OpenBSW)");
    log_inf!("Platform: {}", log::Text(body_ecu_ffi::board_name()));

    openbsw_async::set_binding(&ASYNC_ADAPTER);
    ASYNC_ADAPTER.init();

    let real_hw = is_real_hardware();
    log_inf!(
        "Hardware detection: {} (DBGMCU IDCODE=0x{:08x})",
        if real_hw { "real silicon" } else { "emulated (Renode)" },
        body_ecu_ffi::dbgmcu_idcode()
    );

    let gpio_ok = GPIO_ADAPTER.configure();
    forkpoint::assert_always!(gpio_ok, "body_ecu: the LEDs' GPIO ports are ready");

    if gpio_ok {
        BUTTON_ADAPTER.configure();
    }
    if gpio_ok {
        let count = body_ecu_ffi::led_count();
        for _blink in 0..3 {
            for i in 0..count {
                let rc = body_ecu_ffi::led_set(i, true);
                let (port, pin) = body_ecu_ffi::led_port_and_pin(i);
                printk!("[blink] led{} ON rc={} (port=0x{:x} pin={})\n", i, rc, port, pin);
            }
            zephyr_ffi::msleep(200);
            for i in 0..count {
                body_ecu_ffi::led_set(i, false);
            }
            zephyr_ffi::msleep(200);
        }
        log_inf!("LED blink test done (3 LEDs x 3 blinks)");
    } else {
        log_wrn!("GPIO not available -- using software-only door-lock (emulation)");
    }

    log_inf!("CP1: after GPIO");

    log_inf!("CP2: creating LocalSignalBus");
    log_inf!("CP3: LocalSignalBus done");

    // The lighting and door lock systems exist only with GPIO (the C++ `emplace`s them);
    // here they are statics, used only when `gpio_ok`.

    log_inf!("CP4: creating VehicleModeSystem");
    log_inf!("CP5: VehicleModeSystem done");

    log_inf!("CP6: creating VehicleInfoProvider");
    VEHICLE_INFO.set_vin("WVW00000BODYECU01");
    VEHICLE_INFO.set_ecu_serial("BECU-ZEP-001");

    SOMEIP_SYSTEM.register_method(
        ids::vehicle_info::SERVICE_ID,
        ids::vehicle_info::METHOD_GET_VIN,
        Box::new(|req| {
            let payload = VEHICLE_INFO
                .read_data(VehicleInfoProvider::DID_VIN)
                .map(|vin| vin.data)
                .unwrap_or_default();
            response(req, payload)
        }),
    );
    SOMEIP_SYSTEM.register_event(
        ids::vehicle_info::SERVICE_ID,
        ids::vehicle_info::EVENT_VIN_AVAILABLE,
        ids::vehicle_info::EVENTGROUP_VEHICLE_INFO_EVENTS,
    );
    log_inf!("Registered vehicle_info SOME/IP service (VIN + event)");

    log_inf!("CP7: creating DiagnosticsSystem");
    log_inf!("CP8: creating DoIpTransport");
    DIAGNOSTICS.add_transport(&DOIP_TRANSPORT);
    DIAGNOSTICS.add_provider(&VEHICLE_INFO);
    log_inf!("CP9: diagnostics wired");

    if gpio_ok {
        DIAGNOSTICS.add_provider(LIGHTING.controller());
        DIAGNOSTICS.add_provider(DOOR_LOCK.controller());
        VEHICLE_MODE.manager().add_observer(LIGHTING.controller());
        VEHICLE_MODE.manager().add_observer(DOOR_LOCK.controller());
    }

    log_inf!("CP10: adding lifecycle components");
    LIFECYCLE_MANAGER.add_component(b"someip", &SOMEIP_SYSTEM, 1);
    if gpio_ok {
        LIFECYCLE_MANAGER.add_component(b"lighting", &LIGHTING, 2);
        LIFECYCLE_MANAGER.add_component(b"door_lock", &DOOR_LOCK, 2);
    }
    LIFECYCLE_MANAGER.add_component(b"vehicle_mode", &VEHICLE_MODE, 2);
    if gpio_ok {
        LIFECYCLE_MANAGER.add_component(b"ignition", &IGNITION, 2);
    }
    LIFECYCLE_MANAGER.add_component(b"diagnostics", &DIAGNOSTICS, 3);
    LIFECYCLE_MANAGER.add_component(b"doip", &DOIP_TRANSPORT, 3);

    log_inf!("CP11: lifecycle components added");
    if !gpio_ok {
        let cfg = DoorLockConfig::default();
        SOMEIP_SYSTEM.register_method(
            cfg.service_id,
            cfg.lock_method,
            Box::new(move |req| {
                let old = SW_STATE.0.get();
                SW_STATE.0.set(LockState::Locked);
                SOMEIP_SYSTEM.send_event(
                    cfg.service_id,
                    cfg.lock_state_changed_event,
                    &[old as u8, SW_STATE.0.get() as u8],
                );
                response(req, req.payload.clone())
            }),
        );
        SOMEIP_SYSTEM.register_method(
            cfg.service_id,
            cfg.unlock_method,
            Box::new(move |req| {
                let old = SW_STATE.0.get();
                SW_STATE.0.set(LockState::Unlocked);
                SOMEIP_SYSTEM.send_event(
                    cfg.service_id,
                    cfg.lock_state_changed_event,
                    &[old as u8, SW_STATE.0.get() as u8],
                );
                response(req, req.payload.clone())
            }),
        );
        SOMEIP_SYSTEM.register_method(
            cfg.service_id,
            cfg.get_status_method,
            Box::new(|req| response(req, alloc::vec![SW_STATE.0.get() as u8])),
        );
        SOMEIP_SYSTEM.register_event(
            cfg.service_id,
            cfg.lock_state_changed_event,
            cfg.eventgroup_id,
        );
        log_inf!("Registered software-only door-lock SOME/IP handlers");
    }

    log_inf!("Transitioning to run level 3...");
    LIFECYCLE_MANAGER.transition_to_level(MAX_NUM_LEVELS as u8);
    log_inf!("Body ECU ready - all systems running");
    forkpoint::assert_reachable!("body_ecu: all systems running");
    forkpoint::lifecycle::setup_complete(u64::from(LIFECYCLE_MANAGER.level_count()));

    if let Some(vin) = VEHICLE_INFO.read_data(VehicleInfoProvider::DID_VIN) {
        SOMEIP_SYSTEM.send_event(
            ids::vehicle_info::SERVICE_ID,
            ids::vehicle_info::EVENT_VIN_AVAILABLE,
            &vin.data,
        );
        log_inf!("Published VIN event via SOME/IP");
    }

    RUNTIME_MONITOR.start();
    ASYNC_ADAPTER.run();

    body_ecu_ffi::sleep_forever()
}

/// The button was pressed: from the system work queue, through the shim.
#[unsafe(no_mangle)]
pub extern "C" fn rust_button_pressed() {
    BUTTON_ADAPTER.pressed();
}

/// Timer `slot` fired: from the system work queue, through the shim.
#[unsafe(no_mangle)]
pub extern "C" fn rust_timer_slot_fired(slot: u32) {
    TIMER_SERVICE.fired(slot);
}

/// The SOME/IP receive thread's body.
#[unsafe(no_mangle)]
pub extern "C" fn rust_someip_thread() {
    SOMEIP_SYSTEM.receive_loop();
}

/// A panic is what an `estd` assertion failing is in the C++: it prints and stops the
/// kernel.
#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    unsafe extern "C" {
        fn rust_panic_wrap() -> !;
    }
    printk!("\n*** ASSERT FAILED: {}\n", info);
    forkpoint::assert_unreachable!("body_ecu: an estd assertion fails");
    // SAFETY: `rust_panic_wrap` is the C function in rust_main.c that calls k_panic().
    unsafe { rust_panic_wrap() }
}
