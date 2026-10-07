// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The SOME/IP service, method, event, and eventgroup ids of `config/services.yaml`, as
//! `scripts/generate_someip_config.py` writes them into `someip_service_ids.h` for the
//! C++ build. Written by hand here, in the same order; keep them in step with the YAML.

/// `lighting`.
pub mod lighting {
    /// The service.
    pub const SERVICE_ID: u16 = 0x1000;
    /// The instance.
    pub const INSTANCE_ID: u16 = 0x0001;
    /// `set_light_state`.
    pub const METHOD_SET_LIGHT_STATE: u16 = 0x0001;
    /// `get_light_status`.
    pub const METHOD_GET_LIGHT_STATUS: u16 = 0x0002;
    /// `light_status_changed`.
    pub const EVENT_LIGHT_STATUS_CHANGED: u16 = 0x8001;
    /// `lighting_events`.
    pub const EVENTGROUP_LIGHTING_EVENTS: u16 = 0x0001;
}

/// `door_lock`.
pub mod door_lock {
    /// The service.
    pub const SERVICE_ID: u16 = 0x1001;
    /// The instance.
    pub const INSTANCE_ID: u16 = 0x0001;
    /// `lock`.
    pub const METHOD_LOCK: u16 = 0x0001;
    /// `unlock`.
    pub const METHOD_UNLOCK: u16 = 0x0002;
    /// `get_status`.
    pub const METHOD_GET_STATUS: u16 = 0x0003;
    /// `lock_state_changed`.
    pub const EVENT_LOCK_STATE_CHANGED: u16 = 0x8001;
    /// `door_events`.
    pub const EVENTGROUP_DOOR_EVENTS: u16 = 0x0001;
}

/// `vehicle_mode`.
pub mod vehicle_mode {
    /// The service.
    pub const SERVICE_ID: u16 = 0x1002;
    /// The instance.
    pub const INSTANCE_ID: u16 = 0x0001;
    /// The `mode` field's getter.
    pub const FIELD_MODE_GETTER: u16 = 0x0001;
    /// The `mode` field's setter.
    pub const FIELD_MODE_SETTER: u16 = 0x0002;
    /// The `mode` field's notifier.
    pub const FIELD_MODE_NOTIFIER: u16 = 0x8001;
    /// `mode_events`.
    pub const EVENTGROUP_MODE_EVENTS: u16 = 0x0001;
}

/// `vehicle_info`.
pub mod vehicle_info {
    /// The service.
    pub const SERVICE_ID: u16 = 0x1004;
    /// The instance.
    pub const INSTANCE_ID: u16 = 0x0001;
    /// `get_vin`.
    pub const METHOD_GET_VIN: u16 = 0x0001;
    /// `vin_available`.
    pub const EVENT_VIN_AVAILABLE: u16 = 0x8001;
    /// `vehicle_info_events`.
    pub const EVENTGROUP_VEHICLE_INFO_EVENTS: u16 = 0x0001;
}

/// `speed_sensor`.
pub mod speed_sensor {
    /// The service.
    pub const SERVICE_ID: u16 = 0x1003;
    /// The instance.
    pub const INSTANCE_ID: u16 = 0x0001;
    /// `get_speed`.
    pub const METHOD_GET_SPEED: u16 = 0x0001;
    /// `set_speed`: override the speed (-1.0 to resume the ADC).
    pub const METHOD_SET_SPEED: u16 = 0x0002;
    /// `speed_changed`.
    pub const EVENT_SPEED_CHANGED: u16 = 0x8001;
    /// `speed_events`.
    pub const EVENTGROUP_SPEED_EVENTS: u16 = 0x0001;
}
