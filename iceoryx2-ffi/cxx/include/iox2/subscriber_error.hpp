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

#ifndef IOX2_SUBSCRIBER_ERROR_HPP
#define IOX2_SUBSCRIBER_ERROR_HPP

#include <cstdint>

namespace iox2 {
/// Defines the failure that can occur when receiving data with [`Subscriber::receive()`].
enum class SubscriberReceiveError : uint8_t {
    /// The maximum amount of [`Sample`]s a user can borrow with [`Subscriber::receive()`] is
    /// defined in [`Config`]. When this is exceeded [`Subscriber::receive()`]
    /// fails.
    ExceedsMaxBorrowedSamples,

    /// Failures when creating the connection
    FailedToEstablishConnection,

    /// Failures when mapping the corresponding data segment
    UnableToMapPublishersDataSegment
};

/// Describes the failures when a new [`Subscriber`] is created via the
/// [`PortFactorySubscriber`].
enum class SubscriberCreateError : uint8_t {
    /// The maximum amount of [`Subscriber`]s that can connect to a
    /// [`Service`] is defined in [`Config`]. When this is exceeded no more
    /// [`Subscriber`]s
    /// can be created for a specific [`Service`].
    ExceedsMaxSupportedSubscribers,

    /// When the [`Subscriber`] requires a larger buffer size than the
    /// [`Service`] offers the creation will fail.
    BufferSizeExceedsMaxSupportedBufferSizeOfService,
};

} // namespace iox2

#endif
