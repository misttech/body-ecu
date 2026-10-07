// Copyright 2026 Mist Tecnologia LTDA. All rights reserved.

// The C shim between the Rust Body ECU and Zephyr.
//
// Rust does not depend on Zephyr's headers or on generated bindings: this file owns every
// kernel object and device the firmware needs and exports `cpp_`-prefixed functions that
// take and return plain scalars. Each function is argument marshalling plus one Zephyr
// call; the state machines live in Rust. Rust exports the `rust_`-prefixed callbacks;
// weak fallbacks here let an application that does not define them link.
//
// The first part is the Rust OpenBSW demo's shim (the async contexts, timers, kernel,
// time, console, and CAN); the second is what the Body ECU adds: the log, the LEDs, the
// button, the timer slots, UDP sockets, the SOME/IP receive thread, the heap, and the
// signal bus mutex.

#include <errno.h>
#include <string.h>

#include <zephyr/device.h>
#include <zephyr/drivers/gpio.h>
#include <zephyr/drivers/uart.h>
#include <zephyr/init.h>
#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <zephyr/net/socket.h>
#include <zephyr/sys/fdtable.h>
#include <zephyr/sys/reboot.h>
#include <cmsis_core.h>

#ifdef CONFIG_CAN
#include <zephyr/drivers/can.h>
#endif
#ifdef CONFIG_ADC
#include <zephyr/drivers/adc.h>
#endif

LOG_MODULE_REGISTER(body_ecu, LOG_LEVEL_INF);

// The async contexts of async/Config.h, in priority order, and their stacks as
// app/src/main.cpp defines them. Context 0 is unused in the C++ (its enum starts at 1);
// the Rust adapter creates a thread per context, so it gets a thread that only waits.
#define TASK_COUNT 6

K_THREAD_STACK_DEFINE(unused_stack, 512);
K_THREAD_STACK_DEFINE(background_stack, 2 * 1024);
K_THREAD_STACK_DEFINE(body_stack, 4 * 1024);
K_THREAD_STACK_DEFINE(someip_stack, 8 * 1024);
K_THREAD_STACK_DEFINE(diag_stack, 4 * 1024);
K_THREAD_STACK_DEFINE(sysadmin_stack, 2 * 1024);

static k_thread_stack_t *const stacks[TASK_COUNT] = {
	unused_stack, background_stack, body_stack, someip_stack, diag_stack, sysadmin_stack,
};
static const size_t stack_sizes[TASK_COUNT] = {
	K_THREAD_STACK_SIZEOF(unused_stack),     K_THREAD_STACK_SIZEOF(background_stack),
	K_THREAD_STACK_SIZEOF(body_stack),       K_THREAD_STACK_SIZEOF(someip_stack),
	K_THREAD_STACK_SIZEOF(diag_stack),       K_THREAD_STACK_SIZEOF(sysadmin_stack),
};
static struct k_thread threads[TASK_COUNT];
static k_tid_t thread_ids[TASK_COUNT];
static struct k_event events[TASK_COUNT];
static struct k_timer timers[TASK_COUNT];
static bool application_initialized;

typedef void (*rust_task_entry_t)(void *arg);

// Callbacks into Rust.
extern void rust_task_timer_expired(uint32_t context);
extern void rust_async_enter_task(int32_t context);
extern void rust_async_leave_task(int32_t context);
extern void rust_async_enter_isr_group(uint32_t group);
extern void rust_async_leave_isr_group(uint32_t group);
extern void rust_button_pressed(void);
extern void rust_timer_slot_fired(uint32_t slot);
extern void rust_someip_thread(void);
#ifdef CONFIG_CAN
extern void rust_can_rx(uint32_t id, bool extended, uint8_t length, const uint8_t *data, void *user);
extern void rust_can_tx_done(int error, void *user);
#endif

__weak void rust_task_timer_expired(uint32_t context) { ARG_UNUSED(context); }
__weak void rust_async_enter_task(int32_t context) { ARG_UNUSED(context); }
__weak void rust_async_leave_task(int32_t context) { ARG_UNUSED(context); }
__weak void rust_async_enter_isr_group(uint32_t group) { ARG_UNUSED(group); }
__weak void rust_async_leave_isr_group(uint32_t group) { ARG_UNUSED(group); }
__weak void rust_button_pressed(void) {}
__weak void rust_timer_slot_fired(uint32_t slot) { ARG_UNUSED(slot); }
__weak void rust_someip_thread(void) {}
#ifdef CONFIG_CAN
__weak void rust_can_rx(uint32_t id, bool extended, uint8_t length, const uint8_t *data, void *user)
{
	ARG_UNUSED(id);
	ARG_UNUSED(extended);
	ARG_UNUSED(length);
	ARG_UNUSED(data);
	ARG_UNUSED(user);
}
__weak void rust_can_tx_done(int error, void *user)
{
	ARG_UNUSED(error);
	ARG_UNUSED(user);
}
#endif

static int init_objects(void)
{
	for (uint32_t i = 0; i < TASK_COUNT; ++i) {
		k_event_init(&events[i]);
	}
	application_initialized = true;
	return 0;
}
SYS_INIT(init_objects, APPLICATION, CONFIG_KERNEL_INIT_PRIORITY_DEFAULT);

// Threads and events

static void task_entry(void *p1, void *p2, void *p3)
{
	ARG_UNUSED(p3);
	((rust_task_entry_t)p1)(p2);
}

void cpp_task_create(uint32_t context, const char *name, int32_t priority, rust_task_entry_t entry,
		     void *arg)
{
	thread_ids[context] = k_thread_create(&threads[context], stacks[context], stack_sizes[context],
					      task_entry, (void *)entry, arg, NULL, priority, 0,
					      K_FOREVER);
	k_thread_name_set(thread_ids[context], name);
}

void cpp_task_start(uint32_t context)
{
	if (thread_ids[context] != NULL) {
		k_thread_start(thread_ids[context]);
	}
}

const char *cpp_task_name(uint32_t context)
{
	if (thread_ids[context] != NULL) {
		return k_thread_name_get(thread_ids[context]);
	}
	return "<undefined>";
}

uint32_t cpp_task_stack_size(uint32_t context)
{
	return thread_ids[context] != NULL ? thread_ids[context]->stack_info.size : 0;
}

int32_t cpp_task_stack_unused(uint32_t context, uint32_t *unused)
{
	size_t value = 0;
	int result = k_thread_stack_space_get(thread_ids[context], &value);
	*unused = (uint32_t)value;
	return result;
}

void cpp_event_post(uint32_t context, uint32_t mask) { k_event_post(&events[context], mask); }

uint32_t cpp_event_wait(uint32_t context, uint32_t mask)
{
	return k_event_wait(&events[context], mask, false, K_FOREVER);
}

void cpp_event_clear(uint32_t context, uint32_t mask) { k_event_clear(&events[context], mask); }

// Timers

static void timer_expired(struct k_timer *timer)
{
	rust_task_timer_expired((uint32_t)(uintptr_t)k_timer_user_data_get(timer));
}

static int init_timers(void)
{
	for (uint32_t i = 0; i < TASK_COUNT; ++i) {
		k_timer_init(&timers[i], timer_expired, NULL);
		k_timer_user_data_set(&timers[i], (void *)(uintptr_t)i);
	}
	return 0;
}
SYS_INIT(init_timers, APPLICATION, CONFIG_KERNEL_INIT_PRIORITY_DEFAULT);

void cpp_timer_start_us(uint32_t context, uint32_t microseconds)
{
	k_timer_start(&timers[context], K_USEC(microseconds), K_NO_WAIT);
}

void cpp_timer_stop(uint32_t context) { k_timer_stop(&timers[context]); }

// Kernel

uint32_t cpp_irq_lock(void) { return irq_lock(); }
void cpp_irq_unlock(uint32_t key) { irq_unlock(key); }
bool cpp_is_in_isr(void) { return k_is_in_isr(); }
int32_t cpp_current_priority(void) { return k_thread_priority_get(k_current_get()); }
// k_current_get() may read a stale thread pointer inside the switch hooks.
int32_t cpp_sched_current_priority(void) { return k_thread_priority_get(k_sched_current_thread_query()); }
int32_t cpp_msleep(int32_t milliseconds) { return k_msleep(milliseconds); }

void cpp_reboot_cold(void)
{
	sys_reboot(SYS_REBOOT_COLD);
	for (;;) {
	}
}

void cpp_sleep_forever(void)
{
	k_sleep(K_FOREVER);
	for (;;) {
	}
}

// Time

uint64_t cpp_cycle_get_64(void) { return k_cycle_get_64(); }
uint32_t cpp_cyc_to_us_floor32(uint64_t cycles) { return k_cyc_to_us_floor32(cycles); }
uint64_t cpp_cyc_to_us_floor64(uint64_t cycles) { return k_cyc_to_us_floor64(cycles); }
uint32_t cpp_cyc_to_ms_floor32(uint64_t cycles) { return k_cyc_to_ms_floor32(cycles); }
uint64_t cpp_cyc_to_ns_floor64(uint64_t cycles) { return k_cyc_to_ns_floor64(cycles); }
uint32_t cpp_cycles_per_sec(void) { return sys_clock_hw_cycles_per_sec(); }

// Console

static const struct device *const console_dev = DEVICE_DT_GET(DT_CHOSEN(zephyr_console));

int32_t cpp_uart_poll_in(void)
{
	unsigned char c = -1;
	uart_poll_in(console_dev, &c);
	return c;
}

void cpp_uart_poll_out(uint8_t byte) { uart_poll_out(console_dev, byte); }

// The log and the console lines

void cpp_printk_n(const uint8_t *line, size_t len) { printk("%.*s", (int)len, (const char *)line); }

void cpp_log_inf_n(const uint8_t *line, size_t len) { LOG_INF("%.*s", (int)len, (const char *)line); }

void cpp_log_wrn_n(const uint8_t *line, size_t len) { LOG_WRN("%.*s", (int)len, (const char *)line); }

const char *cpp_board_name(void) { return CONFIG_BOARD; }

// STM32H753 DBGMCU IDCODE register, read as main.cpp reads it.
uint32_t cpp_dbgmcu_idcode(void) { return *(volatile uint32_t *)0x5C001000; }

// The heap

void *cpp_malloc(size_t size, size_t align) { return k_aligned_alloc(align, size); }
void cpp_free(void *ptr) { k_free(ptr); }

// LEDs: the led0, led1, and led2 aliases, as main.cpp lists them.

#ifdef CONFIG_GPIO
static const struct gpio_dt_spec leds[] = {
	GPIO_DT_SPEC_GET_OR(DT_ALIAS(led0), gpios, {0}),
	GPIO_DT_SPEC_GET_OR(DT_ALIAS(led1), gpios, {0}),
	GPIO_DT_SPEC_GET_OR(DT_ALIAS(led2), gpios, {0}),
};

uint32_t cpp_led_count(void) { return ARRAY_SIZE(leds); }

int32_t cpp_led_configure(uint32_t *failed)
{
	for (uint32_t i = 0; i < ARRAY_SIZE(leds); i++) {
		if (!gpio_is_ready_dt(&leds[i])) {
			*failed = i;
			return -ENODEV;
		}
		int ret = gpio_pin_configure_dt(&leds[i], GPIO_OUTPUT_INACTIVE);
		if (ret < 0) {
			*failed = i;
			return ret;
		}
	}
	return 0;
}

int32_t cpp_led_set(uint32_t index, bool value)
{
	if (index >= ARRAY_SIZE(leds) || !gpio_is_ready_dt(&leds[index])) {
		return -EINVAL;
	}
	return gpio_pin_set_dt(&leds[index], value ? 1 : 0);
}

bool cpp_led_get(uint32_t index)
{
	if (index >= ARRAY_SIZE(leds) || !gpio_is_ready_dt(&leds[index])) {
		return false;
	}
	return gpio_pin_get_dt(&leds[index]) != 0;
}

const void *cpp_led_port(uint32_t index) { return index < ARRAY_SIZE(leds) ? leds[index].port : NULL; }
uint8_t cpp_led_pin(uint32_t index) { return index < ARRAY_SIZE(leds) ? leds[index].pin : 0; }

// The user button: its interrupt submits a work item, which calls into Rust.

static const struct gpio_dt_spec user_btn = GPIO_DT_SPEC_GET_OR(DT_ALIAS(sw0), gpios, {0});
static struct gpio_callback button_cb_data;
static struct k_work button_work;

static void button_work_handler(struct k_work *work)
{
	ARG_UNUSED(work);
	rust_button_pressed();
}

static void button_isr(const struct device *dev, struct gpio_callback *cb, uint32_t pins)
{
	ARG_UNUSED(dev);
	ARG_UNUSED(cb);
	ARG_UNUSED(pins);
	k_work_submit(&button_work);
}

int32_t cpp_button_configure(uint32_t *failed)
{
	if (!gpio_is_ready_dt(&user_btn)) {
		*failed = 0;
		return -ENODEV;
	}
	k_work_init(&button_work, button_work_handler);
	int ret = gpio_pin_configure_dt(&user_btn, GPIO_INPUT);
	if (ret < 0) {
		*failed = 1;
		return ret;
	}
	ret = gpio_pin_interrupt_configure_dt(&user_btn, GPIO_INT_EDGE_TO_ACTIVE);
	if (ret < 0) {
		*failed = 2;
		return ret;
	}
	gpio_init_callback(&button_cb_data, button_isr, BIT(user_btn.pin));
	gpio_add_callback(user_btn.port, &button_cb_data);
	return 0;
}

uint8_t cpp_button_pin(void) { return user_btn.pin; }
uint16_t cpp_button_flags(void) { return user_btn.dt_flags; }
#endif

// Timer slots: each a k_timer whose expiry submits a work item, so the callback runs on
// the system work queue, as ZephyrTimerService does.

#define TIMER_SLOTS 8

static struct k_timer slot_timers[TIMER_SLOTS];
static struct k_work slot_works[TIMER_SLOTS];

static void slot_work_handler(struct k_work *work)
{
	rust_timer_slot_fired((uint32_t)(work - slot_works));
}

static void slot_timer_expired(struct k_timer *timer)
{
	k_work_submit(&slot_works[timer - slot_timers]);
}

static int init_slots(void)
{
	for (uint32_t i = 0; i < TIMER_SLOTS; ++i) {
		k_timer_init(&slot_timers[i], slot_timer_expired, NULL);
		k_work_init(&slot_works[i], slot_work_handler);
	}
	return 0;
}
SYS_INIT(init_slots, APPLICATION, CONFIG_KERNEL_INIT_PRIORITY_DEFAULT);

void cpp_timer_slot_start(uint32_t slot, uint32_t milliseconds, bool periodic)
{
	if (slot >= TIMER_SLOTS) {
		return;
	}
	k_timer_start(&slot_timers[slot], K_MSEC(milliseconds), periodic ? K_MSEC(milliseconds) : K_NO_WAIT);
}

void cpp_timer_slot_stop(uint32_t slot)
{
	if (slot < TIMER_SLOTS) {
		k_timer_stop(&slot_timers[slot]);
	}
}

// UDP sockets for SOME/IP, as OpenSOME/IP's Zephyr backend opens them: non-blocking.

int32_t cpp_udp_open(uint16_t port)
{
	int fd = zsock_socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
	if (fd < 0) {
		return -errno;
	}
	int flags = zsock_fcntl(fd, ZVFS_F_GETFL, 0);
	if (flags < 0 || zsock_fcntl(fd, ZVFS_F_SETFL, flags | ZVFS_O_NONBLOCK) < 0) {
		zsock_close(fd);
		return -errno;
	}
	struct sockaddr_in addr = {0};
	addr.sin_family = AF_INET;
	addr.sin_addr.s_addr = htonl(INADDR_ANY);
	addr.sin_port = htons(port);
	if (zsock_bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
		int err = errno;
		zsock_close(fd);
		return -err;
	}
	return fd;
}

int32_t cpp_udp_recv_from(int32_t fd, uint8_t *buf, size_t len, uint8_t *address, uint16_t *port)
{
	struct sockaddr_in src = {0};
	socklen_t src_len = sizeof(src);
	ssize_t received = zsock_recvfrom(fd, buf, len, 0, (struct sockaddr *)&src, &src_len);
	if (received < 0) {
		int err = errno;
		return (err == EAGAIN || err == EWOULDBLOCK) ? -EAGAIN : -err;
	}
	memcpy(address, &src.sin_addr.s_addr, 4);
	*port = ntohs(src.sin_port);
	return (int32_t)received;
}

int32_t cpp_udp_send_to(int32_t fd, const uint8_t *data, size_t len, const uint8_t *address, uint16_t port)
{
	struct sockaddr_in dst = {0};
	dst.sin_family = AF_INET;
	memcpy(&dst.sin_addr.s_addr, address, 4);
	dst.sin_port = htons(port);
	ssize_t sent = zsock_sendto(fd, data, len, 0, (const struct sockaddr *)&dst, sizeof(dst));
	return sent < 0 ? -errno : (int32_t)sent;
}

void cpp_udp_close(int32_t fd) { zsock_close(fd); }

// The SOME/IP receive thread, as OpenSOME/IP's Zephyr thread backend creates it.

#define SOMEIP_RX_STACK_SIZE 8192
K_THREAD_STACK_DEFINE(someip_rx_stack, SOMEIP_RX_STACK_SIZE);
static struct k_thread someip_rx_thread;

static void someip_rx_entry(void *p1, void *p2, void *p3)
{
	ARG_UNUSED(p1);
	ARG_UNUSED(p2);
	ARG_UNUSED(p3);
	rust_someip_thread();
}

void cpp_someip_thread_start(void)
{
	k_thread_create(&someip_rx_thread, someip_rx_stack, K_THREAD_STACK_SIZEOF(someip_rx_stack),
			someip_rx_entry, NULL, NULL, NULL, K_PRIO_PREEMPT(7), 0, K_NO_WAIT);
}

// The signal bus's mutex (LocalSignalBus's k_mutex).

static struct k_mutex signal_bus_mutex;

static int init_signal_bus_mutex(void)
{
	k_mutex_init(&signal_bus_mutex);
	return 0;
}
SYS_INIT(init_signal_bus_mutex, APPLICATION, CONFIG_KERNEL_INIT_PRIORITY_DEFAULT);

void cpp_signal_bus_lock(void) { k_mutex_lock(&signal_bus_mutex, K_FOREVER); }
void cpp_signal_bus_unlock(void) { k_mutex_unlock(&signal_bus_mutex); }

// The ADC, as AdcAdapter configures it.

#ifdef CONFIG_ADC
static const struct device *const adc_dev = DEVICE_DT_GET(DT_NODELABEL(adc1));
static struct adc_channel_cfg adc_channel_cfg;
static uint8_t adc_resolution = 12;

int32_t cpp_adc_configure(uint8_t channel, uint8_t resolution)
{
	if (!device_is_ready(adc_dev)) {
		return -ENODEV;
	}
	adc_channel_cfg.gain = ADC_GAIN_1;
	adc_channel_cfg.reference = ADC_REF_INTERNAL;
	adc_channel_cfg.acquisition_time = ADC_ACQ_TIME(ADC_ACQ_TIME_TICKS, 811);
	adc_channel_cfg.channel_id = channel;
	adc_resolution = resolution;
	return adc_channel_setup(adc_dev, &adc_channel_cfg);
}

int32_t cpp_adc_read(uint8_t channel)
{
	int16_t sample = 0;
	struct adc_sequence sequence = {
		.options = NULL,
		.channels = BIT(channel),
		.buffer = &sample,
		.buffer_size = sizeof(sample),
		.resolution = adc_resolution,
		.oversampling = 0,
		.calibrate = false,
	};
	int ret = adc_read(adc_dev, &sequence);
	return ret < 0 ? ret : (int32_t)sample;
}
#else
int32_t cpp_adc_configure(uint8_t channel, uint8_t resolution)
{
	ARG_UNUSED(channel);
	ARG_UNUSED(resolution);
	return -ENODEV;
}
int32_t cpp_adc_read(uint8_t channel)
{
	ARG_UNUSED(channel);
	return -ENODEV;
}
#endif

// CAN, as the Rust OpenBSW demo's shim has it; the Forkpoint image leaves CAN out.

#ifdef CONFIG_CAN
static const struct device *const can_dev = DEVICE_DT_GET(DT_CHOSEN(zephyr_canbus));
static const struct can_filter match_all_filter = {.id = 0, .mask = 0, .flags = 0};

bool cpp_can_device_is_ready(void) { return device_is_ready(can_dev); }
int32_t cpp_can_set_mode_normal(void) { return can_set_mode(can_dev, CAN_MODE_NORMAL); }
int32_t cpp_can_start(void) { return can_start(can_dev); }
int32_t cpp_can_stop(void) { return can_stop(can_dev); }

static void rx_callback(const struct device *dev, struct can_frame *frame, void *user)
{
	ARG_UNUSED(dev);
	rust_can_rx(frame->id, (frame->flags & CAN_FRAME_IDE) != 0, frame->dlc, frame->data, user);
}

int32_t cpp_can_add_rx_filter_all(void *user)
{
	return can_add_rx_filter(can_dev, rx_callback, user, &match_all_filter);
}

void cpp_can_remove_rx_filter(int32_t filter_id) { can_remove_rx_filter(can_dev, filter_id); }

static void tx_callback(const struct device *dev, int error, void *user)
{
	ARG_UNUSED(dev);
	rust_can_tx_done(error, user);
}

int32_t cpp_can_send(uint32_t id, bool extended, uint8_t dlc, const uint8_t *data, void *user)
{
	struct can_frame frame;
	if (dlc > sizeof(frame.data)) {
		return -EINVAL;
	}
	memset(&frame, 0, sizeof(frame));
	frame.id = id;
	frame.flags = extended ? CAN_FRAME_IDE : 0;
	frame.dlc = dlc;
	memcpy(frame.data, data, dlc);
	return can_send(can_dev, &frame, K_NO_WAIT, tx_callback, user);
}

uint32_t cpp_can_bitrate(void)
{
	return DT_PROP_OR(DT_CHOSEN(zephyr_canbus), bitrate,
			  DT_PROP_OR(DT_CHOSEN(zephyr_canbus), bus_speed, CONFIG_CAN_DEFAULT_BITRATE));
}

int32_t cpp_can_get_state(int32_t *state, uint8_t *tx_error_count, uint8_t *rx_error_count)
{
	enum can_state can_state_value;
	struct can_bus_err_cnt err_cnt;
	int result = can_get_state(can_dev, &can_state_value, &err_cnt);
	if (result == 0) {
		*state = (int32_t)can_state_value;
		*tx_error_count = err_cnt.tx_err_cnt;
		*rx_error_count = err_cnt.rx_err_cnt;
	}
	return result;
}
#endif

// rivet as the C library (CONFIG_EXTERNAL_LIBC), with the kernel hooks of Forkpoint's
// nucleo-g474re-zephyr-rivet example: errno is the running thread's, as Zephyr's own is, so
// the socket errors the shim reads are the ones Zephyr set; a mutex keeps two threads'
// printf output apart; write() takes rivet's output to the console UART by polling, as
// printk does; and _exit() masks interrupts and spins. The firmware's own heap is Zephyr's
// (allocator.rs), so rivet's heap hooks stay unset.

#ifdef CONFIG_EXTERNAL_LIBC
#include <rivet_errno.h>
#include <rivet_stdio.h>
#include <zephyr/sys/errno_private.h>

static bool in_thread(void) { return !k_is_pre_kernel() && !k_is_in_isr(); }

static int *thread_errno(void) { return k_is_pre_kernel() ? NULL : z_errno(); }

static const struct rivet_errno_hooks errno_hooks = {
	.current = thread_errno,
};

K_MUTEX_DEFINE(stdout_mutex);

static void lock_stdout(void)
{
	if (in_thread()) {
		(void)k_mutex_lock(&stdout_mutex, K_FOREVER);
	}
}

static void unlock_stdout(void)
{
	if (in_thread()) {
		(void)k_mutex_unlock(&stdout_mutex);
	}
}

static const struct rivet_stdio_hooks stdio_hooks = {
	.lock = lock_stdout,
	.unlock = unlock_stdout,
};

static int init_rivet_hooks(void)
{
	rivet_errno_set_hooks(&errno_hooks);
	rivet_stdio_set_hooks(&stdio_hooks);
	return 0;
}
SYS_INIT(init_rivet_hooks, PRE_KERNEL_1, 0);

ssize_t write(int fd, const void *buf, size_t count)
{
	ARG_UNUSED(fd);
	const unsigned char *bytes = buf;
	for (size_t i = 0; i < count; i++) {
		uart_poll_out(console_dev, bytes[i]);
	}
	return (ssize_t)count;
}

_Noreturn void _exit(int status)
{
	ARG_UNUSED(status);
	(void)arch_irq_lock();
	for (;;) {
	}
}
#endif

// The PWM LED of the OpenBSW demo, which this board's application does not drive;
// declared for the zephyr_ffi crate.
int32_t cpp_pwm_set_led0(uint32_t period_ns, uint32_t pulse_ns)
{
	ARG_UNUSED(period_ns);
	ARG_UNUSED(pulse_ns);
	return -ENOTSUP;
}

// Tracing hooks, which forward to Rust as the OpenBSW demo's shim does. The C++ Body ECU
// is built without Zephyr's tracing, and so is this image, so they are not compiled in.

#ifdef CONFIG_TRACING_USER
#include <tracing_user.h>

void sys_trace_thread_switched_in_user(void)
{
	if (!application_initialized) {
		return;
	}
	unsigned int key = irq_lock();
	rust_async_enter_task(cpp_sched_current_priority() - 1);
	irq_unlock(key);
}

void sys_trace_thread_switched_out_user(void)
{
	if (!application_initialized) {
		return;
	}
	unsigned int key = irq_lock();
	rust_async_leave_task(cpp_sched_current_priority() - 1);
	irq_unlock(key);
}

void sys_trace_isr_enter_user(void)
{
	if (!application_initialized) {
		return;
	}
	rust_async_enter_isr_group(0);
}

void sys_trace_isr_exit_user(void)
{
	if (!application_initialized) {
		return;
	}
	rust_async_leave_isr_group(0);
}
#endif
