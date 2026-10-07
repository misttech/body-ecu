#include <zephyr/kernel.h>
#include <zephyr/logging/log.h>
#include <optional>

LOG_MODULE_REGISTER(body_ecu, LOG_LEVEL_INF);

#ifdef CONFIG_GPIO
#include <zephyr/drivers/gpio.h>
#endif

#ifdef CONFIG_CAN
#include <zephyr/drivers/can.h>
#endif

#ifdef CONFIG_ADC
#include <zephyr/drivers/adc.h>
#endif

#include <forkpoint/hostcall.h>

#include <async/AsyncBinding.h>
#include <lifecycle/LifecycleManager.h>
#include <bsp/timer/SystemTimer.h>
#include <util/estd/assert.h>

#include "CanGatewaySystem.h"
#include "DiagnosticsSystem.h"
#include "DoCanTransport.h"
#include "DoIpTransport.h"
#include "DoorLockSystem.h"
#include "VehicleInfoProvider.h"
#include "LightingSystem.h"
#include "SomeIpSystem.h"
#include "IgnitionSystem.h"
#include "SpeedSimulatorSystem.h"
#include "VehicleModeSystem.h"

#include "someip_service_ids.h"
#include "ports/NullButtonInput.h"

#ifdef CONFIG_GPIO
#include "zephyr_adapters/ButtonAdapter.h"
#include "zephyr_adapters/CanAdapter.h"
#include "zephyr_adapters/GpioAdapter.h"
#endif

#ifdef CONFIG_ADC
#include "zephyr_adapters/AdcAdapter.h"
#endif

#include "zephyr_adapters/LocalSignalBus.h"
#include "zephyr_adapters/ZephyrTimerService.h"

namespace {

/// STM32H753 DBGMCU IDCODE register.  Real silicon reports DEV_ID = 0x450;
/// Renode typically does not model this peripheral and returns 0.
bool is_real_hardware()
{
    volatile uint32_t idcode = *reinterpret_cast<volatile uint32_t*>(0x5C001000);
    return (idcode & 0xFFF) == 0x450;
}

}  // namespace

using namespace body_ecu;

using AsyncAdapter        = ::async::AsyncBinding::AdapterType;
using AsyncRuntimeMonitor = ::async::AsyncBinding::RuntimeMonitorType;
using AsyncContextHook    = ::async::AsyncBinding::ContextHookType;

static constexpr size_t MaxNumComponents         = 16;
static constexpr size_t MaxNumLevels             = 8;
static constexpr size_t MaxNumComponentsPerLevel = MaxNumComponents;

using LifecycleManager = ::lifecycle::declare::
    LifecycleManager<MaxNumComponents, MaxNumLevels, MaxNumComponentsPerLevel>;

static char const* const isrGroupNames[ISR_GROUP_COUNT] = {"test"};

static AsyncRuntimeMonitor runtimeMonitor{
    AsyncContextHook::InstanceType::GetNameType::create<&AsyncAdapter::getTaskName>(),
    isrGroupNames};

static LifecycleManager lifecycleManager{
    TASK_SYSADMIN,
    ::lifecycle::LifecycleManager::GetTimestampType::create<&getSystemTimeUs32Bit>()};

K_THREAD_STACK_DEFINE(sysadminStack, 2 * 1024);
using SysadminTask = AsyncAdapter::Task<TASK_SYSADMIN, K_THREAD_STACK_SIZEOF(sysadminStack)>;
SysadminTask sysadminTask{"sysadmin", sysadminStack};

K_THREAD_STACK_DEFINE(bodyStack, 4 * 1024);
using BodyTask = AsyncAdapter::Task<TASK_BODY, K_THREAD_STACK_SIZEOF(bodyStack)>;
BodyTask bodyTask{"body", bodyStack};

K_THREAD_STACK_DEFINE(someipStack, 8 * 1024);
using SomeIpTask = AsyncAdapter::Task<TASK_SOMEIP, K_THREAD_STACK_SIZEOF(someipStack)>;
SomeIpTask someipTask{"someip", someipStack};

K_THREAD_STACK_DEFINE(diagStack, 4 * 1024);
using DiagTask = AsyncAdapter::Task<TASK_DIAG, K_THREAD_STACK_SIZEOF(diagStack)>;
DiagTask diagTask{"diag", diagStack};

K_THREAD_STACK_DEFINE(backgroundStack, 2 * 1024);
using BackgroundTask = AsyncAdapter::Task<TASK_BACKGROUND, K_THREAD_STACK_SIZEOF(backgroundStack)>;
BackgroundTask backgroundTask{"background", backgroundStack};

AsyncContextHook contextHook{runtimeMonitor};

int main(void)
{
    ::estd::set_assert_handler(
        [](char const* file, int line, char const* expr) {
            printk("\n*** ASSERT FAILED: file=%s line=%d expr=%s\n",
                   file ? file : "(null)", line, expr ? expr : "(null)");
            FPT_UNREACHABLE("body_ecu: an estd assertion fails");
            k_panic();
        });

    LOG_INF("Body ECU starting (Zephyr + OpenBSW)");
    LOG_INF("Platform: %s", CONFIG_BOARD);

    AsyncAdapter::init();

    bool real_hw = is_real_hardware();
    LOG_INF("Hardware detection: %s (DBGMCU IDCODE=0x%08x)",
            real_hw ? "real silicon" : "emulated (Renode)",
            *reinterpret_cast<volatile uint32_t*>(0x5C001000));

    adapters::SomeIpConfig someip_cfg{
        .host = "0.0.0.0", .port = 30490, .enable_sd = real_hw};
    static adapters::SomeIpSystem someip_system(someip_cfg);

#ifdef CONFIG_GPIO
    static const struct gpio_dt_spec leds[] = {
        GPIO_DT_SPEC_GET_OR(DT_ALIAS(led0), gpios, {0}),
        GPIO_DT_SPEC_GET_OR(DT_ALIAS(led1), gpios, {0}),
        GPIO_DT_SPEC_GET_OR(DT_ALIAS(led2), gpios, {0}),
    };

    std::vector<struct gpio_dt_spec> led_specs(std::begin(leds), std::end(leds));
    static adapters::GpioAdapter gpio_adapter(led_specs);
    bool gpio_ok = gpio_adapter.configure();
    FPT_ALWAYS(gpio_ok, "body_ecu: the LEDs' GPIO ports are ready");

    static const struct gpio_dt_spec user_btn =
        GPIO_DT_SPEC_GET_OR(DT_ALIAS(sw0), gpios, {0});
    static adapters::ButtonAdapter button_adapter(user_btn);
    if (gpio_ok) {
        button_adapter.configure();
    }
    if (gpio_ok) {
        for (int blink = 0; blink < 3; ++blink) {
            for (size_t i = 0; i < ARRAY_SIZE(leds); ++i) {
                int rc = gpio_pin_set_dt(&leds[i], 1);
                printk("[blink] led%zu ON rc=%d (port=%p pin=%d)\n",
                       i, rc, (void*)leds[i].port, leds[i].pin);
            }
            k_msleep(200);
            for (size_t i = 0; i < ARRAY_SIZE(leds); ++i) {
                gpio_pin_set_dt(&leds[i], 0);
            }
            k_msleep(200);
        }
        LOG_INF("LED blink test done (3 LEDs x 3 blinks)");
    } else {
        LOG_WRN("GPIO not available -- using software-only door-lock (emulation)");
    }
#endif

    LOG_INF("CP1: after GPIO");

#ifdef CONFIG_CAN
    const struct device* can_dev = DEVICE_DT_GET(DT_CHOSEN(zephyr_canbus));
    static adapters::CanAdapter can_adapter(can_dev);
    can_adapter.configure();
#endif

    LOG_INF("CP2: creating LocalSignalBus");
    static adapters::LocalSignalBus signal_bus;
    LOG_INF("CP3: LocalSignalBus done");

#ifdef CONFIG_GPIO
    static ports::NullButtonInput null_button;
    std::optional<adapters::LightingSystem> lighting;
    std::optional<adapters::DoorLockSystem> door_lock;
    if (gpio_ok) {
        lighting.emplace(gpio_adapter, someip_system);
        body::DoorLockConfig door_cfg;
        door_cfg.lock_gpio_pin = 2;
        door_lock.emplace(gpio_adapter, null_button, someip_system,
                          door_cfg, &signal_bus);
    }
#endif
    // --- Speed Simulator (ADC potentiometer) ---
    static adapters::ZephyrTimerService timer_service;
#ifdef CONFIG_ADC
    const struct device* adc_dev = DEVICE_DT_GET(DT_NODELABEL(adc1));
    static adapters::AdcAdapter adc_adapter(adc_dev);
    bool adc_ok = adc_adapter.configure(15, 12);
    std::optional<adapters::SpeedSimulatorSystem> speed_sim;
    if (real_hw && adc_ok) {
        body::SpeedSimulatorConfig speed_cfg;
        speed_cfg.adc_channel = 15;
        speed_sim.emplace(adc_adapter, someip_system, timer_service,
                          speed_cfg, &signal_bus);
    }
#endif

    LOG_INF("CP4: creating VehicleModeSystem");
    static adapters::VehicleModeSystem vehicle_mode(someip_system);
    LOG_INF("CP5: VehicleModeSystem done");

#ifdef CONFIG_GPIO
    std::optional<adapters::IgnitionSystem> ignition;
    if (gpio_ok) {
        ignition.emplace(button_adapter, vehicle_mode, timer_service,
                         body::IgnitionConfig{}, &signal_bus);
    }
#endif

#ifdef CONFIG_CAN
    static adapters::CanGatewaySystem can_gateway(can_adapter, someip_system);

    platform::ServiceMapping door_gw;
    door_gw.name = "door_status";
    door_gw.direction = platform::GatewayDirection::CanToSomeIp;
    door_gw.someip_service_id = 0x1001;
    door_gw.someip_event_id = 0x8001;
    door_gw.someip_eventgroup_id = 0x0001;
    door_gw.can_id = 0x300;
    door_gw.can_dlc = 2;
    can_gateway.addMapping(door_gw);
#endif

    LOG_INF("CP6: creating VehicleInfoProvider");
    static adapters::VehicleInfoProvider vehicle_info;
    vehicle_info.setVin("WVW00000BODYECU01");
    vehicle_info.setEcuSerial("BECU-ZEP-001");

    someip_system.registerMethod(
        body_ecu::someip::vehicle_info::kServiceId,
        body_ecu::someip::vehicle_info::method::kGetVin,
        [&vehicle_info](const ports::SomeIpMessage& req) {
            ports::SomeIpMessage resp = req;
            resp.message_type = 0x80;
            resp.return_code = 0x00;
            auto vin_data = vehicle_info.readData(adapters::VehicleInfoProvider::kDidVin);
            if (vin_data) {
                resp.payload = vin_data->data;
            }
            return resp;
        });
    someip_system.registerEvent(
        body_ecu::someip::vehicle_info::kServiceId,
        body_ecu::someip::vehicle_info::event::kVinAvailable,
        body_ecu::someip::vehicle_info::eventgroup::kVehicleInfoEvents);
    LOG_INF("Registered vehicle_info SOME/IP service (VIN + event)");

    LOG_INF("CP7: creating DiagnosticsSystem");
    static adapters::DiagnosticsSystem diagnostics;
    LOG_INF("CP8: creating DoIpTransport");
    static adapters::DoIpTransport doip_transport;
#ifdef CONFIG_CAN
    static adapters::DoCanTransport docan_transport(can_adapter);
    diagnostics.addTransport(&docan_transport);
#endif
    diagnostics.addTransport(&doip_transport);
    diagnostics.addProvider(&vehicle_info);
    LOG_INF("CP9: diagnostics wired");

#ifdef CONFIG_GPIO
    if (lighting && door_lock) {
        diagnostics.addProvider(&lighting->controller());
        diagnostics.addProvider(&door_lock->controller());
        vehicle_mode.manager().addObserver(&lighting->controller());
        vehicle_mode.manager().addObserver(&door_lock->controller());
    }
#endif
#ifdef CONFIG_ADC
    if (speed_sim) {
        vehicle_mode.manager().addObserver(&speed_sim->simulator());
    }
#endif

    LOG_INF("CP10: adding lifecycle components");
    lifecycleManager.addComponent("someip", someip_system, 1);
#ifdef CONFIG_GPIO
    if (lighting) lifecycleManager.addComponent("lighting", *lighting, 2);
    if (door_lock) lifecycleManager.addComponent("door_lock", *door_lock, 2);
#endif
    lifecycleManager.addComponent("vehicle_mode", vehicle_mode, 2);
#ifdef CONFIG_GPIO
    if (ignition) lifecycleManager.addComponent("ignition", *ignition, 2);
#endif
#ifdef CONFIG_ADC
    if (speed_sim) lifecycleManager.addComponent("speed_sim", *speed_sim, 2);
#endif
#ifdef CONFIG_CAN
    lifecycleManager.addComponent("can_gateway", can_gateway, 3);
    lifecycleManager.addComponent("docan", docan_transport, 3);
#endif
    lifecycleManager.addComponent("diagnostics", diagnostics, 3);
    lifecycleManager.addComponent("doip", doip_transport, 3);

    LOG_INF("CP11: lifecycle components added");
#ifdef CONFIG_GPIO
    if (!gpio_ok) {
        static body::LockState sw_state = body::LockState::Unlocked;
        body::DoorLockConfig cfg;
        someip_system.registerMethod(cfg.service_id, cfg.lock_method,
            [&](const ports::SomeIpMessage& req) {
                auto old = sw_state;
                sw_state = body::LockState::Locked;
                someip_system.sendEvent(cfg.service_id,
                    cfg.lock_state_changed_event,
                    {static_cast<uint8_t>(old), static_cast<uint8_t>(sw_state)});
                ports::SomeIpMessage resp = req;
                resp.message_type = 0x80;
                resp.return_code = 0x00;
                return resp;
            });
        someip_system.registerMethod(cfg.service_id, cfg.unlock_method,
            [&](const ports::SomeIpMessage& req) {
                auto old = sw_state;
                sw_state = body::LockState::Unlocked;
                someip_system.sendEvent(cfg.service_id,
                    cfg.lock_state_changed_event,
                    {static_cast<uint8_t>(old), static_cast<uint8_t>(sw_state)});
                ports::SomeIpMessage resp = req;
                resp.message_type = 0x80;
                resp.return_code = 0x00;
                return resp;
            });
        someip_system.registerMethod(cfg.service_id, cfg.get_status_method,
            [](const ports::SomeIpMessage& req) {
                ports::SomeIpMessage resp = req;
                resp.message_type = 0x80;
                resp.return_code = 0x00;
                resp.payload = {static_cast<uint8_t>(sw_state)};
                return resp;
            });
        someip_system.registerEvent(cfg.service_id,
            cfg.lock_state_changed_event, cfg.eventgroup_id);
        LOG_INF("Registered software-only door-lock SOME/IP handlers");
    }
#endif

#ifdef CONFIG_ADC
    if (!speed_sim) {
        body::SpeedSimulatorConfig scfg;
        someip_system.registerMethod(scfg.service_id, scfg.get_speed_method,
            [](const ports::SomeIpMessage& req) {
                ports::SomeIpMessage resp = req;
                resp.message_type = 0x80;
                resp.return_code = 0x00;
                resp.payload = {0, 0, 0, 0};
                return resp;
            });
        someip_system.registerMethod(scfg.service_id, scfg.set_speed_method,
            [](const ports::SomeIpMessage& req) {
                ports::SomeIpMessage resp = req;
                resp.message_type = 0x80;
                resp.return_code = 0x00;
                return resp;
            });
        someip_system.registerEvent(scfg.service_id,
            scfg.speed_changed_event, scfg.eventgroup_id);
        LOG_INF("Registered software-only speed SOME/IP handlers (emulation)");
    }
#endif

    LOG_INF("Transitioning to run level 3...");
    lifecycleManager.transitionToLevel(MaxNumLevels);
#ifdef CONFIG_CAN
    can_adapter.startReceiving();
#endif
    LOG_INF("Body ECU ready - all systems running");
    FPT_REACHABLE("body_ecu: all systems running");
    fpt_setup_complete(lifecycleManager.getLevelCount());

    {
        auto vin_data = vehicle_info.readData(adapters::VehicleInfoProvider::kDidVin);
        if (vin_data) {
            someip_system.sendEvent(
                body_ecu::someip::vehicle_info::kServiceId,
                body_ecu::someip::vehicle_info::event::kVinAvailable,
                vin_data->data);
            LOG_INF("Published VIN event via SOME/IP");
        }
    }

    runtimeMonitor.start();
    AsyncAdapter::run();

    k_sleep(K_FOREVER);
    return 0;
}
