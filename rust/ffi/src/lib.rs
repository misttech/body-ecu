// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

//! Rust's view of what the Body ECU asks of Zephyr beyond the kernel objects the Rust
//! OpenBSW demo's `zephyr_ffi` covers: the LEDs and the button of the board, the timer
//! slots behind the timer service, UDP sockets for SOME/IP, the SOME/IP receive thread,
//! the debug identity code, the console and the log, and the heap.
//!
//! As in `zephyr_ffi`, nothing here depends on Zephyr's headers: the application's
//! `zephyr_shim.c` owns the devices and kernel objects and exports `cpp_`-prefixed
//! functions that take and return plain scalars, declared here and wrapped safely. On the
//! host (tests), every function is a neutral stub: no LED, no button, no socket.
//!
//! The shim calls back into Rust through `rust_button_pressed`, `rust_timer_slot_fired`,
//! and `rust_someip_thread`, which the application defines.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

#[cfg(not(target_os = "none"))]
mod host;
#[cfg(target_os = "none")]
mod target;

#[cfg(not(target_os = "none"))]
use host as imp;
#[cfg(target_os = "none")]
use target as imp;

/// How many timer slots the shim has (`ZephyrTimerService::kMaxTimers`).
pub const TIMER_SLOTS: usize = 8;

/// Zephyr's `EAGAIN`: nothing waits on the socket.
pub const EAGAIN: i32 = 11;

/// Print `line` on the console at once (`printk`).
pub fn printk(line: &[u8]) {
    imp::printk(line);
}

/// Log `line` at INFO on the `body_ecu` module (`LOG_INF`).
pub fn log_inf(line: &[u8]) {
    imp::log_inf(line);
}

/// Log `line` at WARNING on the `body_ecu` module (`LOG_WRN`).
pub fn log_wrn(line: &[u8]) {
    imp::log_wrn(line);
}

/// The board's name (`CONFIG_BOARD`).
pub fn board_name() -> &'static [u8] {
    imp::board_name()
}

/// `size` bytes aligned to `align` from the kernel heap (`k_aligned_alloc`), or null.
pub fn malloc(size: usize, align: usize) -> *mut u8 {
    imp::malloc(size, align)
}

/// Return `ptr` to the kernel heap (`k_free`).
///
/// # Safety
///
/// `ptr` came from [`malloc`] and is not used afterwards.
pub unsafe fn free(ptr: *mut u8) {
    // SAFETY: the caller's contract.
    unsafe { imp::free(ptr) }
}

/// The debug support block's identity code (`DBGMCU->IDCODE`).
pub fn dbgmcu_idcode() -> u32 {
    imp::dbgmcu_idcode()
}

/// How many LEDs the board has (the `led0`, `led1`, and `led2` aliases).
pub fn led_count() -> u32 {
    imp::led_count()
}

/// Configure every LED as an inactive output; `Err(index)` names the first that failed,
/// with the driver's error.
pub fn led_configure() -> Result<(), (u32, i32)> {
    imp::led_configure()
}

/// Drive LED `index`; returns the driver's result.
pub fn led_set(index: u32, value: bool) -> i32 {
    imp::led_set(index, value)
}

/// The level of LED `index`.
pub fn led_get(index: u32) -> bool {
    imp::led_get(index)
}

/// The address of LED `index`'s port device and its pin, as the C++ blink test prints
/// them.
pub fn led_port_and_pin(index: u32) -> (usize, u8) {
    imp::led_port_and_pin(index)
}

/// Configure the user button as an input interrupting on its active edge; the shim then
/// calls `rust_button_pressed` from the system work queue on each press. `Err` names
/// what failed: `0` the port, `1` the pin, `2` the interrupt, with the driver's error.
pub fn button_configure() -> Result<(), (u32, i32)> {
    imp::button_configure()
}

/// The button's pin and Device Tree flags.
pub fn button_pin_and_flags() -> (u8, u16) {
    imp::button_pin_and_flags()
}

/// Start timer `slot`: after `milliseconds`, and every `milliseconds` when `periodic`,
/// the shim calls `rust_timer_slot_fired(slot)` from the system work queue.
pub fn timer_slot_start(slot: u32, milliseconds: u32, periodic: bool) {
    imp::timer_slot_start(slot, milliseconds, periodic);
}

/// Stop timer `slot`.
pub fn timer_slot_stop(slot: u32) {
    imp::timer_slot_stop(slot);
}

/// Open a non-blocking UDP socket bound to every address at `port`; the descriptor, or
/// a negative errno.
pub fn udp_open(port: u16) -> i32 {
    imp::udp_open(port)
}

/// The next datagram on `fd` into `buf`: its length and sender, `Err(EAGAIN)` when none
/// waits, or another negative errno.
pub fn udp_recv_from(fd: i32, buf: &mut [u8]) -> Result<(usize, [u8; 4], u16), i32> {
    imp::udp_recv_from(fd, buf)
}

/// Send `data` from `fd` to `address:port`; the bytes sent, or a negative errno.
pub fn udp_send_to(fd: i32, data: &[u8], address: [u8; 4], port: u16) -> i32 {
    imp::udp_send_to(fd, data, address, port)
}

/// Close `fd`.
pub fn udp_close(fd: i32) {
    imp::udp_close(fd);
}

/// Start the SOME/IP receive thread, which runs `rust_someip_thread`.
pub fn someip_thread_start() {
    imp::someip_thread_start();
}

/// Sleep for `milliseconds`.
pub fn msleep(milliseconds: i32) {
    imp::msleep(milliseconds);
}

/// Never return: the end of `main`.
pub fn sleep_forever() -> ! {
    imp::sleep_forever()
}

/// Lock the signal bus's mutex.
pub fn signal_bus_lock() {
    imp::signal_bus_lock();
}

/// Unlock the signal bus's mutex.
pub fn signal_bus_unlock() {
    imp::signal_bus_unlock();
}

/// Configure ADC channel `channel` at `resolution` bits; the driver's result.
pub fn adc_configure(channel: u8, resolution: u8) -> i32 {
    imp::adc_configure(channel, resolution)
}

/// Read ADC channel `channel`: the sample, or a negative errno.
pub fn adc_read(channel: u8) -> i32 {
    imp::adc_read(channel)
}
