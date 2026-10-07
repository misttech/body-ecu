// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! The bare-metal implementation: calls into `zephyr_shim.c`.

use core::ffi::c_void;

unsafe extern "C" {
    fn cpp_printk_n(line: *const u8, len: usize);
    fn cpp_log_inf_n(line: *const u8, len: usize);
    fn cpp_log_wrn_n(line: *const u8, len: usize);
    fn cpp_board_name() -> *const core::ffi::c_char;
    fn cpp_malloc(size: usize, align: usize) -> *mut c_void;
    fn cpp_free(ptr: *mut c_void);
    fn cpp_dbgmcu_idcode() -> u32;
    fn cpp_led_count() -> u32;
    fn cpp_led_configure(failed: *mut u32) -> i32;
    fn cpp_led_set(index: u32, value: bool) -> i32;
    fn cpp_led_get(index: u32) -> bool;
    fn cpp_led_port(index: u32) -> *const c_void;
    fn cpp_led_pin(index: u32) -> u8;
    fn cpp_button_configure(failed: *mut u32) -> i32;
    fn cpp_button_pin() -> u8;
    fn cpp_button_flags() -> u16;
    fn cpp_timer_slot_start(slot: u32, milliseconds: u32, periodic: bool);
    fn cpp_timer_slot_stop(slot: u32);
    fn cpp_udp_open(port: u16) -> i32;
    fn cpp_udp_recv_from(
        fd: i32,
        buf: *mut u8,
        len: usize,
        address: *mut u8,
        port: *mut u16,
    ) -> i32;
    fn cpp_udp_send_to(fd: i32, data: *const u8, len: usize, address: *const u8, port: u16) -> i32;
    fn cpp_udp_close(fd: i32);
    fn cpp_someip_thread_start();
    fn cpp_msleep(milliseconds: i32) -> i32;
    fn cpp_sleep_forever() -> !;
    fn cpp_signal_bus_lock();
    fn cpp_signal_bus_unlock();
    fn cpp_adc_configure(channel: u8, resolution: u8) -> i32;
    fn cpp_adc_read(channel: u8) -> i32;
}

pub fn printk(line: &[u8]) {
    // SAFETY: the shim reads `len` bytes at `line`, which the slice holds.
    unsafe { cpp_printk_n(line.as_ptr(), line.len()) }
}

pub fn log_inf(line: &[u8]) {
    // SAFETY: as `printk`.
    unsafe { cpp_log_inf_n(line.as_ptr(), line.len()) }
}

pub fn log_wrn(line: &[u8]) {
    // SAFETY: as `printk`.
    unsafe { cpp_log_wrn_n(line.as_ptr(), line.len()) }
}

pub fn board_name() -> &'static [u8] {
    // SAFETY: the shim returns a NUL-terminated string literal that lives forever.
    unsafe { core::ffi::CStr::from_ptr(cpp_board_name()) }.to_bytes()
}

pub fn malloc(size: usize, align: usize) -> *mut u8 {
    // SAFETY: a plain call.
    unsafe { cpp_malloc(size, align) }.cast()
}

pub unsafe fn free(ptr: *mut u8) {
    // SAFETY: the caller's contract.
    unsafe { cpp_free(ptr.cast()) }
}

pub fn dbgmcu_idcode() -> u32 {
    // SAFETY: a plain call.
    unsafe { cpp_dbgmcu_idcode() }
}

pub fn led_count() -> u32 {
    // SAFETY: a plain call.
    unsafe { cpp_led_count() }
}

pub fn led_configure() -> Result<(), (u32, i32)> {
    let mut failed = 0;
    // SAFETY: the shim writes the failing index through a valid pointer.
    let result = unsafe { cpp_led_configure(&mut failed) };
    if result == 0 { Ok(()) } else { Err((failed, result)) }
}

pub fn led_set(index: u32, value: bool) -> i32 {
    // SAFETY: a plain call; the shim bounds-checks the index.
    unsafe { cpp_led_set(index, value) }
}

pub fn led_get(index: u32) -> bool {
    // SAFETY: as `led_set`.
    unsafe { cpp_led_get(index) }
}

pub fn led_port_and_pin(index: u32) -> (usize, u8) {
    // SAFETY: as `led_set`; the pointer is only printed, never dereferenced.
    unsafe { (cpp_led_port(index) as usize, cpp_led_pin(index)) }
}

pub fn button_configure() -> Result<(), (u32, i32)> {
    let mut failed = 0;
    // SAFETY: the shim writes the failing step through a valid pointer.
    let result = unsafe { cpp_button_configure(&mut failed) };
    if result == 0 { Ok(()) } else { Err((failed, result)) }
}

pub fn button_pin_and_flags() -> (u8, u16) {
    // SAFETY: plain calls.
    unsafe { (cpp_button_pin(), cpp_button_flags()) }
}

pub fn timer_slot_start(slot: u32, milliseconds: u32, periodic: bool) {
    // SAFETY: a plain call; the shim bounds-checks the slot.
    unsafe { cpp_timer_slot_start(slot, milliseconds, periodic) }
}

pub fn timer_slot_stop(slot: u32) {
    // SAFETY: as `timer_slot_start`.
    unsafe { cpp_timer_slot_stop(slot) }
}

pub fn udp_open(port: u16) -> i32 {
    // SAFETY: a plain call.
    unsafe { cpp_udp_open(port) }
}

pub fn udp_recv_from(fd: i32, buf: &mut [u8]) -> Result<(usize, [u8; 4], u16), i32> {
    let mut address = [0_u8; 4];
    let mut port = 0_u16;
    // SAFETY: the shim writes at most `buf.len()` bytes into `buf`, four into `address`,
    // and the port through valid pointers.
    let received = unsafe {
        cpp_udp_recv_from(fd, buf.as_mut_ptr(), buf.len(), address.as_mut_ptr(), &mut port)
    };
    if received < 0 { Err(received) } else { Ok((received as usize, address, port)) }
}

pub fn udp_send_to(fd: i32, data: &[u8], address: [u8; 4], port: u16) -> i32 {
    // SAFETY: the shim reads `data.len()` bytes at `data` and four at `address`.
    unsafe { cpp_udp_send_to(fd, data.as_ptr(), data.len(), address.as_ptr(), port) }
}

pub fn udp_close(fd: i32) {
    // SAFETY: a plain call.
    unsafe { cpp_udp_close(fd) }
}

pub fn someip_thread_start() {
    // SAFETY: a plain call.
    unsafe { cpp_someip_thread_start() }
}

pub fn msleep(milliseconds: i32) {
    // SAFETY: a plain call.
    unsafe {
        cpp_msleep(milliseconds);
    }
}

pub fn sleep_forever() -> ! {
    // SAFETY: a plain call that never returns.
    unsafe { cpp_sleep_forever() }
}

pub fn signal_bus_lock() {
    // SAFETY: a plain call.
    unsafe { cpp_signal_bus_lock() }
}

pub fn signal_bus_unlock() {
    // SAFETY: a plain call.
    unsafe { cpp_signal_bus_unlock() }
}

pub fn adc_configure(channel: u8, resolution: u8) -> i32 {
    // SAFETY: a plain call.
    unsafe { cpp_adc_configure(channel, resolution) }
}

pub fn adc_read(channel: u8) -> i32 {
    // SAFETY: a plain call.
    unsafe { cpp_adc_read(channel) }
}
