// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The host stubs: no board, no socket, no sleep. Tests of the glue crates drive the
//! adapters through their callbacks instead of through hardware.

pub fn printk(_line: &[u8]) {}

pub fn log_inf(_line: &[u8]) {}

pub fn log_wrn(_line: &[u8]) {}

pub fn board_name() -> &'static [u8] {
    b"host"
}

pub fn malloc(_size: usize, _align: usize) -> *mut u8 {
    core::ptr::null_mut()
}

pub unsafe fn free(_ptr: *mut u8) {}

pub fn dbgmcu_idcode() -> u32 {
    0
}

pub fn led_count() -> u32 {
    3
}

pub fn led_configure() -> Result<(), (u32, i32)> {
    Ok(())
}

pub fn led_set(_index: u32, _value: bool) -> i32 {
    0
}

pub fn led_get(_index: u32) -> bool {
    false
}

pub fn led_port_and_pin(_index: u32) -> (usize, u8) {
    (0, 0)
}

pub fn button_configure() -> Result<(), (u32, i32)> {
    Ok(())
}

pub fn button_pin_and_flags() -> (u8, u16) {
    (13, 0)
}

pub fn timer_slot_start(_slot: u32, _milliseconds: u32, _periodic: bool) {}

pub fn timer_slot_stop(_slot: u32) {}

pub fn udp_open(_port: u16) -> i32 {
    -1
}

pub fn udp_recv_from(_fd: i32, _buf: &mut [u8]) -> Result<(usize, [u8; 4], u16), i32> {
    Err(-crate::EAGAIN)
}

pub fn udp_send_to(_fd: i32, data: &[u8], _address: [u8; 4], _port: u16) -> i32 {
    data.len() as i32
}

pub fn udp_close(_fd: i32) {}

pub fn someip_thread_start() {}

pub fn msleep(_milliseconds: i32) {}

pub fn sleep_forever() -> ! {
    panic!("the host never sleeps forever")
}

pub fn signal_bus_lock() {}

pub fn signal_bus_unlock() {}

pub fn adc_configure(_channel: u8, _resolution: u8) -> i32 {
    0
}

pub fn adc_read(_channel: u8) -> i32 {
    0
}
