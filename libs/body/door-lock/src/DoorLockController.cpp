#include "door_lock/DoorLockController.h"

#ifdef __ZEPHYR__
#include <zephyr/sys/printk.h>
#else
#include <cstdio>
#define printk(...) std::printf(__VA_ARGS__)
#endif

#include <forkpoint/hostcall.h>

namespace body_ecu::body {

DoorLockController::DoorLockController(ports::IGpioPort& gpio,
                                       ports::IButtonInput& button,
                                       ports::ISomeIpService& someip,
                                       const DoorLockConfig& config,
                                       ports::ISignalBus* signal_bus)
    : gpio_(gpio),
      button_(button),
      someip_(someip),
      config_(config),
      signal_bus_(signal_bus) {}

void DoorLockController::init() {
    someip_.registerMethod(config_.service_id, config_.lock_method,
                           [this](const ports::SomeIpMessage& req) {
                               return handleLock(req);
                           });
    someip_.registerMethod(config_.service_id, config_.unlock_method,
                           [this](const ports::SomeIpMessage& req) {
                               return handleUnlock(req);
                           });
    someip_.registerMethod(config_.service_id, config_.get_status_method,
                           [this](const ports::SomeIpMessage& req) {
                               return handleGetStatus(req);
                           });
    someip_.registerEvent(config_.service_id,
                          config_.lock_state_changed_event,
                          config_.eventgroup_id);

    button_.onPress([this]() { onButtonPress(); });

    if (signal_bus_) {
        signal_bus_->subscribe(
            config_.signal_command_lock,
            [this](const std::string& path, const ports::SignalValue& value) {
                onCommandSignal(path, value);
            });
    }
}

bool DoorLockController::checkSafetyConstraints() const {
    if (!signal_bus_) return true;

    auto speed = signal_bus_->get(config_.signal_speed);
    if (speed) {
        if (auto* f = std::get_if<float>(&*speed)) {
            if (*f > 5.0f) return false;
        }
    }

    auto door_open = signal_bus_->get(config_.signal_door_open);
    if (door_open) {
        if (auto* b = std::get_if<bool>(&*door_open)) {
            if (*b) return false;
        }
    }

    return true;
}

bool DoorLockController::lock() {
    if (state_ == LockState::Error) return false;
    if (state_ == LockState::Locked) return true;
    if (!checkSafetyConstraints()) return false;

    LockState old = state_;
    state_ = LockState::Locked;
    gpio_.write(config_.lock_gpio_pin, true);
    publishStateChanged(old, state_);
    return true;
}

bool DoorLockController::unlock() {
    if (state_ == LockState::Error) return false;
    if (state_ == LockState::Unlocked) return true;
    if (!checkSafetyConstraints()) return false;

    LockState old = state_;
    state_ = LockState::Unlocked;
    gpio_.write(config_.lock_gpio_pin, false);
    publishStateChanged(old, state_);
    return true;
}

void DoorLockController::setError() {
    LockState old = state_;
    state_ = LockState::Error;
    publishStateChanged(old, state_);
}

void DoorLockController::onModeChanged(ports::VehicleMode /*old_mode*/,
                                       ports::VehicleMode new_mode) {
    if (new_mode == ports::VehicleMode::Run) {
        bool locked = lock();
        FPT_SOMETIMES(locked, "door_lock: entering Run locks the door");
    }
}

std::optional<ports::DiagData> DoorLockController::readData(
    uint16_t did) const {
    if (did != config_.diag_did) return std::nullopt;
    ports::DiagData data;
    data.did = did;
    data.data.push_back(static_cast<uint8_t>(state_));
    return data;
}

bool DoorLockController::ioControl(
    uint16_t did, const std::vector<uint8_t>& control_param) {
    if (did != config_.diag_did) return false;
    if (control_param.empty()) return false;
    return control_param[0] != 0 ? lock() : unlock();
}

void DoorLockController::onButtonPress() {
    if (state_ == LockState::Locked) {
        printk("[door_lock] Button pressed -- unlocking\n");
        unlock();
    } else if (state_ == LockState::Unlocked) {
        printk("[door_lock] Button pressed -- locking\n");
        lock();
    }
}

void DoorLockController::onCommandSignal(const std::string& /*path*/,
                                         const ports::SignalValue& value) {
    auto* cmd = std::get_if<bool>(&value);
    if (!cmd) return;

    bool success = *cmd ? lock() : unlock();

    if (signal_bus_) {
        uint8_t response_code = success ? 0x00 : 0x01;
        signal_bus_->publish(config_.signal_command_response,
                             ports::SignalValue{static_cast<int32_t>(response_code)});
    }
}

ports::SomeIpMessage DoorLockController::handleLock(
    const ports::SomeIpMessage& request) {
    ports::SomeIpMessage response = request;
    response.message_type = 0x80;
    response.return_code = lock() ? 0x00 : 0x01;
    return response;
}

ports::SomeIpMessage DoorLockController::handleUnlock(
    const ports::SomeIpMessage& request) {
    ports::SomeIpMessage response = request;
    response.message_type = 0x80;
    response.return_code = unlock() ? 0x00 : 0x01;
    return response;
}

ports::SomeIpMessage DoorLockController::handleGetStatus(
    const ports::SomeIpMessage& request) {
    ports::SomeIpMessage response = request;
    response.message_type = 0x80;
    response.return_code = 0x00;
    response.payload = {static_cast<uint8_t>(state_)};
    return response;
}

void DoorLockController::publishStateChanged(LockState old_state,
                                             LockState new_state) {
    FPT_ALWAYS(old_state != new_state, "door_lock: a state change changes the state");
    FPT_SOMETIMES(new_state == LockState::Locked, "door_lock: the door locks");
    FPT_SOMETIMES(new_state == LockState::Unlocked, "door_lock: the door unlocks");
    fpt_send_event("door_lock.state", static_cast<uint64_t>(new_state));
    std::vector<uint8_t> payload = {static_cast<uint8_t>(old_state),
                                    static_cast<uint8_t>(new_state)};
    someip_.sendEvent(config_.service_id, config_.lock_state_changed_event,
                      payload);

    if (signal_bus_) {
        signal_bus_->publish(config_.signal_is_locked,
                             ports::SignalValue{new_state == LockState::Locked});
    }
}

}  // namespace body_ecu::body
