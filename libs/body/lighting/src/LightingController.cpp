#include "lighting/LightingController.h"

#ifdef __ZEPHYR__
#include <zephyr/sys/printk.h>
#define LIGHT_LOG(...) printk(__VA_ARGS__)
#else
#include <cstdio>
#define LIGHT_LOG(...) std::printf(__VA_ARGS__)
#endif

#include <forkpoint/hostcall.h>

namespace body_ecu::body {

LightingController::LightingController(ports::IGpioPort& gpio,
                                       ports::ISomeIpService& someip,
                                       const LightingConfig& config)
    : gpio_(gpio), someip_(someip), config_(config) {}

void LightingController::init() {
    someip_.registerMethod(
        config_.service_id, config_.set_light_state_method,
        [this](const ports::SomeIpMessage& req) {
            return handleSetLightState(req);
        });
    someip_.registerMethod(
        config_.service_id, config_.get_light_status_method,
        [this](const ports::SomeIpMessage& req) {
            return handleGetLightStatus(req);
        });
    someip_.registerEvent(config_.service_id,
                          config_.light_status_changed_event,
                          config_.eventgroup_id);
}

bool LightingController::setLightState(LightId id, bool on) {
    auto idx = static_cast<size_t>(id);
    if (idx >= kLightCount) return false;

    bool old_state = states_[idx];
    states_[idx] = on;
    gpio_.write(config_.gpio_pins[idx], on);
    FPT_ALWAYS(states_[idx] == on, "lighting: a light's state follows the request");
    FPT_SOMETIMES(on, "lighting: a light turns on");
    FPT_SOMETIMES(!on, "lighting: a light turns off");
    fpt_send_event("lighting.light", (static_cast<uint64_t>(idx) << 8) | (on ? 1u : 0u));

    if (old_state != on) {
        publishLightStatusChanged();
    }
    return true;
}

std::array<bool, kLightCount> LightingController::getLightStatus() const {
    return states_;
}

void LightingController::onModeChanged(ports::VehicleMode /*old_mode*/,
                                       ports::VehicleMode new_mode) {
    if (new_mode == ports::VehicleMode::Off) {
        for (size_t i = 0; i < kLightCount; ++i) {
            setLightState(static_cast<LightId>(i), false);
        }
    }
}

std::optional<ports::DiagData> LightingController::readData(
    uint16_t did) const {
    if (did != config_.diag_did) return std::nullopt;
    ports::DiagData data;
    data.did = did;
    for (bool s : states_) {
        data.data.push_back(s ? 1 : 0);
    }
    return data;
}

bool LightingController::ioControl(
    uint16_t did, const std::vector<uint8_t>& control_param) {
    if (did != config_.diag_did) return false;
    if (control_param.size() < 2) return false;
    auto light_id = static_cast<LightId>(control_param[0]);
    bool state = control_param[1] != 0;
    return setLightState(light_id, state);
}

ports::SomeIpMessage LightingController::handleSetLightState(
    const ports::SomeIpMessage& request) {
    ports::SomeIpMessage response = request;
    response.message_type = 0x80;

    if (request.payload.size() < 2) {
        response.return_code = 0x01;
        return response;
    }

    auto light_id = static_cast<LightId>(request.payload[0]);
    bool state = request.payload[1] != 0;

    LIGHT_LOG("[light] set id=%d state=%d\n",
              static_cast<int>(light_id), state ? 1 : 0);

    response.return_code = setLightState(light_id, state) ? 0x00 : 0x01;

    LIGHT_LOG("[light] set done rc=%d\n", response.return_code);

    return response;
}

ports::SomeIpMessage LightingController::handleGetLightStatus(
    const ports::SomeIpMessage& request) {
    ports::SomeIpMessage response = request;
    response.message_type = 0x80;
    response.return_code = 0x00;
    response.payload.clear();
    for (bool s : states_) {
        response.payload.push_back(s ? 1 : 0);
    }
    return response;
}

void LightingController::publishLightStatusChanged() {
    std::vector<uint8_t> payload;
    for (bool s : states_) {
        payload.push_back(s ? 1 : 0);
    }
    someip_.sendEvent(config_.service_id, config_.light_status_changed_event,
                      payload);
}

}  // namespace body_ecu::body
