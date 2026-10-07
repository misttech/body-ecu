#include "ignition/IgnitionController.h"

#ifdef __ZEPHYR__
#include <zephyr/sys/printk.h>
#else
#include <cstdio>
#define printk(...) std::printf(__VA_ARGS__)
#endif

#include <forkpoint/hostcall.h>

namespace body_ecu::body {

IgnitionController::IgnitionController(ports::IButtonInput& button,
                                       VehicleModeManager& mode_manager,
                                       ports::ITimerService& timer,
                                       const IgnitionConfig& config,
                                       ports::ISignalBus* signal_bus)
    : button_(button),
      mode_manager_(mode_manager),
      timer_(timer),
      config_(config),
      signal_bus_(signal_bus) {}

void IgnitionController::init() {
    button_.onPress([this]() { onButtonPress(); });

    if (mode_manager_.getMode() == ports::VehicleMode::Off) {
        printk("[ignition] Init: Off -> Accessory\n");
        mode_manager_.setMode(ports::VehicleMode::Accessory);
    }
}

void IgnitionController::onButtonPress() {
    auto mode = mode_manager_.getMode();
    FPT_ALWAYS(!cranking_ || mode == ports::VehicleMode::Crank,
               "ignition: cranking only while the mode is Crank");
    FPT_SOMETIMES(mode == ports::VehicleMode::Run, "ignition: the button is pressed in Run");
    fpt_send_event("ignition.button", static_cast<uint64_t>(mode));

    if (cranking_) {
        printk("[ignition] Button ignored (cranking)\n");
        return;
    }

    switch (mode) {
        case ports::VehicleMode::Accessory: {
            printk("[ignition] Accessory -> Crank\n");
            if (mode_manager_.setMode(ports::VehicleMode::Crank)) {
                cranking_ = true;
                auto tid = timer_.startOneShot(config_.crank_duration_ms, [this]() {
                    printk("[ignition] Crank -> Run\n");
                    mode_manager_.setMode(ports::VehicleMode::Run);
                    cranking_ = false;
                });
                if (tid == ports::kInvalidTimerId) {
                    printk("[ignition] FAULT: crank timer allocation failed, "
                           "reverting to Accessory\n");
                    cranking_ = false;
                    mode_manager_.setMode(ports::VehicleMode::Accessory);
                }
            }
            break;
        }
        case ports::VehicleMode::Run: {
            float speed = getCurrentSpeed();
            if (speed > 0.0f) {
                printk("[ignition] Cannot turn off: speed=%d km/h\n",
                       static_cast<int>(speed));
            } else {
                printk("[ignition] Run -> Accessory\n");
                mode_manager_.setMode(ports::VehicleMode::Accessory);
            }
            break;
        }
        default:
            printk("[ignition] Button ignored in mode %d\n",
                   static_cast<int>(mode));
            break;
    }
}

float IgnitionController::getCurrentSpeed() const {
    if (!signal_bus_) return 0.0f;

    auto speed = signal_bus_->get(config_.signal_speed);
    if (speed) {
        if (auto* f = std::get_if<float>(&*speed)) {
            return *f;
        }
    }
    return 0.0f;
}

}  // namespace body_ecu::body
