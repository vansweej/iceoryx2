// Copyright (c) 2024 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

#include "iox2/service.hpp"
#include "iox/assertions_addendum.hpp"
#include "iox2/iceoryx2.h"
#include "iox2/internal/iceoryx2.hpp"
#include "iox2/service_details.hpp"
#include "iox2/service_type.hpp"
#include "iox2/static_config.hpp"

namespace iox2 {
template <ServiceType S>
auto Service<S>::does_exist(const ServiceName& service_name,
                            const ConfigView config,
                            const MessagingPattern messaging_pattern) -> iox::expected<bool, ServiceDetailsError> {
    bool does_exist_result = false;
    auto result = iox2_service_does_exist(iox::into<iox2_service_type_e>(S),
                                          service_name.as_view().m_ptr,
                                          config.m_ptr,
                                          iox::into<iox2_messaging_pattern_e>(messaging_pattern),
                                          &does_exist_result);

    if (result == IOX2_OK) {
        return iox::ok(does_exist_result);
    }

    return iox::err(iox::into<ServiceDetailsError>(result));
}

template <ServiceType S>
auto Service<S>::details([[maybe_unused]] const ServiceName& service_name,
                         [[maybe_unused]] const ConfigView config,
                         [[maybe_unused]] const MessagingPattern messaging_pattern)
    -> iox::expected<iox::optional<ServiceDetails<S>>, ServiceDetailsError> {
    IOX_TODO();
}

template <ServiceType S>
auto list_callback(const iox2_static_config_t* const static_config, void* ctx) -> iox2_callback_progression_e {
    auto callback = static_cast<iox::function<CallbackProgression(ServiceDetails<S>)>*>(ctx);
    auto result = (*callback)(ServiceDetails<S> { StaticConfig(*static_config) });
    return iox::into<iox2_callback_progression_e>(result);
}

template <ServiceType S>
auto Service<S>::list(const ConfigView config, const iox::function<CallbackProgression(ServiceDetails<S>)>& callback)
    -> iox::expected<void, ServiceListError> {
    auto mutable_callback = callback;
    auto result =
        iox2_service_list(iox::into<iox2_service_type_e>(S), config.m_ptr, list_callback<S>, &mutable_callback);

    if (result == IOX2_OK) {
        return iox::ok();
    }

    return iox::err(iox::into<ServiceListError>(result));
}

template class Service<ServiceType::Ipc>;
template class Service<ServiceType::Local>;
} // namespace iox2
