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

//! Defines the messaging pattern used in a [`Service`](crate::service::Service)-based
//! communication.
//!
//! ## Messaging Patterns
//!
//! ### Publish-Subscribe
//!
//! See the
//! [Wikipedia Article: Publish-subscribe pattern](https://en.wikipedia.org/wiki/Publish%E2%80%93subscribe_pattern).
//! It uses uni-directional communication where `n`
//! [`Publisher`](crate::port::publisher::Publisher)s continuously send data to `m`
//! [`Subscriber`](crate::port::subscriber::Subscriber)s.
//!
//! ### Event
//!
//! Enable processes to notify and wakeup other processes by sending events that are uniquely
//! identified by a [`crate::port::event_id::EventId`]. Hereby, `n`
//! [`Notifier`](crate::port::notifier::Notifier)s can notify `m`
//! [`Listener`](crate::port::listener::Listener)s.
//!
//! **Note:** This does **not** send or receive POSIX signals nor is it based on them.

/// Identifies the kind of messaging pattern the [`Service`](crate::service::Service) will use.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[repr(u32)]
pub enum MessagingPattern {
    /// Unidirectional communication pattern where the
    /// [`Publisher`](crate::port::publisher::Publisher) sends arbitrary data to the
    /// [`Subscriber`](crate::port::subscriber::Subscriber)
    PublishSubscribe = 0,

    /// Unidirectional communication pattern where the [`Notifier`](crate::port::notifier::Notifier)
    /// sends signals/events to the [`Listener`](crate::port::listener::Listener) which has the
    /// ability to sleep until a signal/event arrives.
    /// Building block to realize push-notifications.
    Event,

    /// Biidirectional communication pattern where the
    /// [`Client`](crate::port::client::Client) sends arbitrary data in form of requests to the
    /// [`Server`](crate::port::server::Server) and receives a stream of responses.
    RequestResponse,
}
