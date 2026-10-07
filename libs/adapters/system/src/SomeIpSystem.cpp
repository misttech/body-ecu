#include "SomeIpSystem.h"

#include <forkpoint/hostcall.h>

#ifdef __ZEPHYR__
#include <zephyr/sys/printk.h>
#define SOMEIP_LOG(...) printk(__VA_ARGS__)
#else
#include <cstdio>
#define SOMEIP_LOG(...) std::printf(__VA_ARGS__)
#endif

namespace body_ecu::adapters {

SomeIpSystem::SomeIpSystem(const SomeIpConfig& config) : config_(config) {}

SomeIpSystem::~SomeIpSystem() {
    if (running_) {
        shutdown();
    }
}

void SomeIpSystem::init() {
#ifdef HAS_OPENSOMEIP
    is_server_ = (config_.role == SomeIpRole::Server);
    if (is_server_) {
        ::someip::transport::Endpoint local(config_.host, config_.port);
        transport_ = std::make_shared<::someip::transport::UdpTransport>(local);
    } else {
        ::someip::transport::Endpoint local("0.0.0.0", 0);
        transport_ = std::make_shared<::someip::transport::UdpTransport>(local);
        server_endpoint_ = ::someip::transport::Endpoint(config_.host, config_.port);
    }
    transport_->set_listener(this);
    std::printf("[SOME/IP] Initialized (%s) %s:%u\n",
                is_server_ ? "server" : "client -> ",
                config_.host.c_str(), config_.port);
#endif
    transitionDone();
}

#ifdef HAS_OPENSOMEIP
void SomeIpSystem::initSd() {
    if (!config_.enable_sd) {
        std::printf("[SOME/IP-SD] Disabled by configuration\n");
        return;
    }
    ::someip::sd::SdConfig sd_cfg;
    sd_cfg.multicast_address = config_.sd_multicast;
    sd_cfg.multicast_port    = config_.sd_port;
    sd_cfg.unicast_address   = config_.host;
    sd_cfg.cyclic_offer      = std::chrono::milliseconds(config_.sd_offer_interval_ms);

    if (is_server_) {
        sd_server_ = std::make_unique<::someip::sd::SdServer>(sd_cfg);
        if (!sd_server_->initialize()) {
            std::printf("[SOME/IP-SD] Failed to initialize SD server\n");
            return;
        }

        ::someip::sd::ServiceInstance svc;
        svc.service_id = 0x1001;
        svc.instance_id = 0x0001;
        svc.major_version = 1;
        svc.ttl_seconds = 60;
        std::string endpoint = config_.host + ":" + std::to_string(config_.port);
        sd_server_->offer_service(svc, endpoint);

        for (const auto& ev : events_) {
            ::someip::sd::ServiceInstance ev_svc;
            ev_svc.service_id = ev.service_id;
            ev_svc.instance_id = 0x0001;
            ev_svc.major_version = 1;
            ev_svc.ttl_seconds = 60;
            std::string ep = config_.host + ":" + std::to_string(config_.port);
            sd_server_->offer_service(ev_svc, ep);
        }

        std::printf("[SOME/IP-SD] Server offering services on %s (multicast %s:%u)\n",
                    endpoint.c_str(), config_.sd_multicast.c_str(), config_.sd_port);
    } else {
        sd_client_ = std::make_unique<::someip::sd::SdClient>(sd_cfg);
        if (!sd_client_->initialize()) {
            std::printf("[SOME/IP-SD] Failed to initialize SD client\n");
            return;
        }

        std::set<uint16_t> service_ids;
        service_ids.insert(0x1001);
        {
            PlatformLockGuard lock(mutex_);
            for (const auto& ev : events_) {
                service_ids.insert(ev.service_id);
            }
        }

        for (uint16_t sid : service_ids) {
            sd_client_->find_service(sid,
                [this](const std::vector<::someip::sd::ServiceInstance>& svcs) {
                    onServiceFound(svcs);
                },
                std::chrono::milliseconds(30000));
            std::printf("[SOME/IP-SD] Searching for service 0x%04X\n", sid);
        }

        std::printf("[SOME/IP-SD] Client searching on multicast %s:%u\n",
                    config_.sd_multicast.c_str(), config_.sd_port);
    }
}

void SomeIpSystem::shutdownSd() {
    if (sd_server_) {
        sd_server_->shutdown();
        sd_server_.reset();
    }
    if (sd_client_) {
        sd_client_->shutdown();
        sd_client_.reset();
    }
}

void SomeIpSystem::onServiceFound(
    const std::vector<::someip::sd::ServiceInstance>& services) {
    for (const auto& svc : services) {
        if (!svc.ip_address.empty() && svc.port != 0) {
            std::printf("[SOME/IP-SD] Discovered service 0x%04X at %s:%u\n",
                        svc.service_id, svc.ip_address.c_str(), svc.port);
            server_endpoint_ = ::someip::transport::Endpoint(svc.ip_address, svc.port);

            if (sd_client_) {
                std::set<uint16_t> egs;
                {
                    PlatformLockGuard lock(mutex_);
                    for (const auto& ev : events_) {
                        if (ev.service_id == svc.service_id) {
                            egs.insert(ev.eventgroup_id);
                        }
                    }
                }
                if (egs.empty()) {
                    egs.insert(0x0001);
                }
                for (uint16_t eg : egs) {
                    sd_client_->subscribe_eventgroup(svc.service_id, svc.instance_id, eg);
                    std::printf("[SOME/IP-SD] Subscribed to eventgroup 0x%04X of service 0x%04X\n",
                                eg, svc.service_id);
                }
            }
        }
    }
}
#endif

void SomeIpSystem::run() {
    running_ = true;
#ifdef HAS_OPENSOMEIP
    auto result = transport_->start();
    if (result != ::someip::Result::SUCCESS) {
        std::printf("[SOME/IP] Failed to start transport (error %d)\n",
                    static_cast<int>(result));
        return;
    }
    std::printf("[SOME/IP] Transport running on %s\n",
                transport_->get_local_endpoint().to_string().c_str());
    initSd();
#endif
    transitionDone();
}

void SomeIpSystem::shutdown() {
    running_ = false;
#ifdef HAS_OPENSOMEIP
    shutdownSd();
    if (transport_) {
        transport_->set_listener(nullptr);
        transport_->stop();
        std::printf("[SOME/IP] Transport stopped\n");
    }
#endif
    {
        PlatformLockGuard lock(mutex_);
        methods_.clear();
        events_.clear();
        pending_events_.clear();
    }
    transitionDone();
}

void SomeIpSystem::registerMethod(uint16_t service_id, uint16_t method_id,
                                  ports::MethodHandler handler) {
    PlatformLockGuard lock(mutex_);
    methods_[makeKey(service_id, method_id)] = std::move(handler);
}

void SomeIpSystem::registerEvent(uint16_t service_id, uint16_t event_id,
                                 uint16_t eventgroup_id) {
    PlatformLockGuard lock(mutex_);
    events_.push_back({service_id, event_id, eventgroup_id});
}

void SomeIpSystem::sendEvent(uint16_t service_id, uint16_t event_id,
                             const std::vector<uint8_t>& payload) {
    ports::SomeIpMessage msg;
    msg.service_id = service_id;
    msg.method_id = event_id;
    msg.message_type = 0x02;  // Notification
    msg.return_code = 0x00;
    msg.payload = payload;
#ifndef HAS_OPENSOMEIP
    sent_events_.push_back(msg);
#endif

#ifdef HAS_OPENSOMEIP
    {
        PlatformLockGuard lock(mutex_);
        if (dispatching_) {
            pending_events_.push_back(msg);
            return;
        }
    }
    if (transport_ && running_) {
        auto someip_msg = toSomeIp(msg);
        PlatformLockGuard lock(mutex_);
        for (const auto& client : known_clients_) {
            (void)transport_->send_message(someip_msg, client);
        }
    }
#endif
}

void SomeIpSystem::sendResponse(const ports::SomeIpMessage& response) {
#ifndef HAS_OPENSOMEIP
    sent_responses_.push_back(response);
#endif

#ifdef HAS_OPENSOMEIP
    if (transport_ && running_) {
        auto someip_msg = toSomeIp(response);
        if (is_server_) {
            PlatformLockGuard lock(mutex_);
            for (const auto& client : known_clients_) {
                (void)transport_->send_message(someip_msg, client);
            }
        } else {
            (void)transport_->send_message(someip_msg, server_endpoint_);
        }
    }
#endif
}

ports::SomeIpMessage SomeIpSystem::dispatch(
    const ports::SomeIpMessage& request) {
    SOMEIP_LOG("[SOME/IP] dispatch: lookup 0x%04X/0x%04X\n",
               request.service_id, request.method_id);
    fpt_send_event("someip.request",
                   (static_cast<uint64_t>(request.service_id) << 16) | request.method_id);
    ports::MethodHandler handler;
    {
        PlatformLockGuard lock(mutex_);
        auto it = methods_.find(makeKey(request.service_id, request.method_id));
        FPT_SOMETIMES(it == methods_.end(), "someip: an unknown method is answered with an error");
        if (it == methods_.end()) {
            SOMEIP_LOG("[SOME/IP] dispatch: method NOT FOUND\n");
            ports::SomeIpMessage err = request;
            err.message_type = 0x81;  // Error
            err.return_code = 0x05;   // E_NOT_OK
            return err;
        }
        handler = it->second;
    }
    SOMEIP_LOG("[SOME/IP] dispatch: calling handler\n");
    auto response = handler(request);
    SOMEIP_LOG("[SOME/IP] dispatch: handler returned rc=0x%02X\n",
               response.return_code);
    FPT_ALWAYS(response.service_id == request.service_id && response.method_id == request.method_id,
               "someip: a response names the request's service and method");
    FPT_SOMETIMES(response.return_code != 0, "someip: a handler refuses a request");
    return response;
}

// --- opensomeip transport integration ---

#ifdef HAS_OPENSOMEIP

void SomeIpSystem::on_message_received(
    ::someip::MessagePtr message,
    const ::someip::transport::Endpoint& sender) {
    if (!message) return;

    {
        PlatformLockGuard lock(mutex_);
        known_clients_.insert(sender);
    }

    auto incoming = fromSomeIp(*message);

    SOMEIP_LOG("[SOME/IP] Received service=0x%04X method=0x%04X type=0x%02X from %s payload=[",
               incoming.service_id, incoming.method_id,
               incoming.message_type, sender.to_string().c_str());
    for (size_t i = 0; i < incoming.payload.size(); ++i) {
        SOMEIP_LOG("%s0x%02X", i ? " " : "", incoming.payload[i]);
    }
    SOMEIP_LOG("]\n");

    if (message->is_request()) {
        SOMEIP_LOG("[SOME/IP] >> dispatch\n");
        {
            PlatformLockGuard lock(mutex_);
            dispatching_ = true;
        }
        auto response = dispatch(incoming);
        std::vector<ports::SomeIpMessage> deferred;
        {
            PlatformLockGuard lock(mutex_);
            dispatching_ = false;
            deferred.swap(pending_events_);
        }
        SOMEIP_LOG("[SOME/IP] << dispatch rc=0x%02X pending=%zu\n",
                   response.return_code, deferred.size());
        response.message_type = 0x80;  // RESPONSE
        auto resp_msg = toSomeIp(response);
        SOMEIP_LOG("[SOME/IP] >> send response\n");
        (void)transport_->send_message(resp_msg, sender);
        SOMEIP_LOG("[SOME/IP] << send response OK\n");

        std::set<::someip::transport::Endpoint> clients_snapshot;
        {
            PlatformLockGuard lock(mutex_);
            clients_snapshot = known_clients_;
        }
        for (auto& evt : deferred) {
            auto evt_msg = toSomeIp(evt);
            for (const auto& client : clients_snapshot) {
                (void)transport_->send_message(evt_msg, client);
            }
        }
    } else {
        ports::MethodHandler handler;
        {
            PlatformLockGuard lock(mutex_);
            auto key = makeKey(incoming.service_id, incoming.method_id);
            auto it = methods_.find(key);
            if (it != methods_.end()) {
                handler = it->second;
            }
        }
        if (handler) {
            handler(incoming);
        }
    }
}

void SomeIpSystem::on_connection_established(
    const ::someip::transport::Endpoint& endpoint) {
    SOMEIP_LOG("[SOME/IP] Connection established: %s\n",
               endpoint.to_string().c_str());
}

void SomeIpSystem::on_connection_lost(
    const ::someip::transport::Endpoint& endpoint) {
    SOMEIP_LOG("[SOME/IP] Connection lost: %s\n",
               endpoint.to_string().c_str());
    PlatformLockGuard lock(mutex_);
    known_clients_.erase(endpoint);
}

void SomeIpSystem::on_error(::someip::Result error) {
    SOMEIP_LOG("[SOME/IP] Transport error: %d\n", static_cast<int>(error));
}

ports::SomeIpMessage SomeIpSystem::fromSomeIp(const ::someip::Message& msg) {
    ports::SomeIpMessage result;
    result.service_id = msg.get_service_id();
    result.method_id = msg.get_method_id();
    result.client_id = msg.get_client_id();
    result.session_id = msg.get_session_id();
    result.message_type = static_cast<uint8_t>(msg.get_message_type());
    result.return_code  = static_cast<uint8_t>(msg.get_return_code());
    result.payload = msg.get_payload();
    return result;
}

::someip::Message SomeIpSystem::toSomeIp(const ports::SomeIpMessage& msg) {
    ::someip::MessageId msg_id(msg.service_id, msg.method_id);
    ::someip::RequestId req_id(msg.client_id, msg.session_id);
    auto type = static_cast<::someip::MessageType>(msg.message_type);
    auto rc = static_cast<::someip::ReturnCode>(msg.return_code);
    ::someip::Message result(msg_id, req_id, type, rc);
    result.set_payload(msg.payload);
    return result;
}

#endif  // HAS_OPENSOMEIP

}  // namespace body_ecu::adapters
