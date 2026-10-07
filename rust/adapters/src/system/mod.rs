// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The systems (`libs/adapters/system`): the domain modules as OpenBSW lifecycle
//! components, the SOME/IP server, the diagnostic transports, and the vehicle
//! information provider.

mod diagnostics;
mod doip;
mod door_lock;
mod ignition;
mod lighting;
mod someip;
mod vehicle_info;
mod vehicle_mode;

#[cfg(feature = "can")]
mod can_gateway;
#[cfg(feature = "can")]
mod docan;
#[cfg(feature = "adc")]
mod speed_simulator;
#[cfg(all(test, not(feature = "transport")))]
mod tests;

#[cfg(feature = "can")]
pub use can_gateway::CanGatewaySystem;
pub use diagnostics::DiagnosticsSystem;
#[cfg(feature = "can")]
pub use docan::DoCanTransport;
pub use doip::DoIpTransport;
pub use door_lock::DoorLockSystem;
pub use ignition::IgnitionSystem;
pub use lighting::LightingSystem;
pub use someip::{SomeIpConfig, SomeIpRole, SomeIpSystem};
#[cfg(feature = "adc")]
pub use speed_simulator::SpeedSimulatorSystem;
pub use vehicle_info::VehicleInfoProvider;
pub use vehicle_mode::VehicleModeSystem;
