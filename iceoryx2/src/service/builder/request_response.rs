// Copyright (c) 2025 Contributors to the Eclipse Foundation
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

use core::fmt::Debug;
use core::marker::PhantomData;

use crate::prelude::{AttributeSpecifier, AttributeVerifier};
use crate::service::builder::OpenDynamicStorageFailure;
use crate::service::dynamic_config::request_response::DynamicConfigSettings;
use crate::service::port_factory::request_response;
use crate::service::static_config::messaging_pattern::MessagingPattern;
use crate::service::{self, header, static_config};
use crate::service::{builder, dynamic_config, Service};
use iceoryx2_bb_elementary::alignment::Alignment;
use iceoryx2_bb_log::{fail, fatal_panic, warn};
use iceoryx2_cal::dynamic_storage::{DynamicStorageCreateError, DynamicStorageOpenError};
use iceoryx2_cal::serialize::Serialize;
use iceoryx2_cal::static_storage::{StaticStorage, StaticStorageCreateError, StaticStorageLocked};

use super::message_type_details::{MessageTypeDetails, TypeVariant};
use super::{ServiceState, RETRY_LIMIT};

/// Errors that can occur when an existing [`MessagingPattern::RequestResponse`] [`Service`] shall
/// be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestResponseOpenError {
    /// Service could not be openen since it does not exist
    DoesNotExist,
    /// The [`Service`] has a lower maximum amount of active responses than requested.
    DoesNotSupportRequestedAmountOfActiveResponses,
    /// The [`Service`] has a lower maximum amount of active requests than requested.
    DoesNotSupportRequestedAmountOfActiveRequests,
    /// The [`Service`] has a lower maximum amount of borrowed responses than requested.
    DoesNotSupportRequestedAmountOfBorrowedResponses,
    /// The [`Service`] has a lower maximum amount of borrowed requests than requested.
    DoesNotSupportRequestedAmountOfBorrowedRequests,
    /// The [`Service`] has a lower maximum response buffer size than requested.
    DoesNotSupportRequestedResponseBufferSize,
    /// The [`Service`] has a lower maximum request buffer size than requested.
    DoesNotSupportRequestedRequestBufferSize,
    /// The [`Service`] has a lower maximum number of servers than requested.
    DoesNotSupportRequestedAmountOfServers,
    /// The [`Service`] has a lower maximum number of clients than requested.
    DoesNotSupportRequestedAmountOfClients,
    /// The [`Service`] has a lower maximum number of nodes than requested.
    DoesNotSupportRequestedAmountOfNodes,
    /// The maximum number of [`Node`](crate::node::Node)s have already opened the [`Service`].
    ExceedsMaxNumberOfNodes,
    /// The [`Service`]s creation timeout has passed and it is still not initialized. Can be caused
    /// by a process that crashed during [`Service`] creation.
    HangsInCreation,
    /// The [`Service`] has the wrong request payload type, request header type or type alignment.
    IncompatibleRequestType,
    /// The [`Service`] has the wrong response payload type, response header type or type alignment.
    IncompatibleResponseType,
    /// The [`AttributeVerifier`] required attributes that the [`Service`] does not satisfy.
    IncompatibleAttributes,
    /// The [`Service`] has the wrong messaging pattern.
    IncompatibleMessagingPattern,
    /// The [`Service`] required overflow behavior for requests is not compatible.
    IncompatibleOverflowBehaviorForRequests,
    /// The [`Service`] required overflow behavior for responses is not compatible.
    IncompatibleOverflowBehaviorForResponses,
    /// The process has not enough permissions to open the [`Service`].
    InsufficientPermissions,
    /// Errors that indicate either an implementation issue or a wrongly configured system.
    InternalFailure,
    /// The [`Service`] is marked for destruction and currently cleaning up since no one is using it anymore.
    /// When the call creation call is repeated with a little delay the [`Service`] should be
    /// recreatable.
    IsMarkedForDestruction,
    /// Some underlying resources of the [`Service`] are either missing, corrupted or unaccessible.
    ServiceInCorruptedState,
}

impl core::fmt::Display for RequestResponseOpenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "RequestResponseOpenError::{:?}", self)
    }
}

impl core::error::Error for RequestResponseOpenError {}

impl From<ServiceAvailabilityState> for RequestResponseOpenError {
    fn from(value: ServiceAvailabilityState) -> Self {
        match value {
            ServiceAvailabilityState::IncompatibleRequestType => {
                RequestResponseOpenError::IncompatibleRequestType
            }
            ServiceAvailabilityState::IncompatibleResponseType => {
                RequestResponseOpenError::IncompatibleResponseType
            }
            ServiceAvailabilityState::ServiceState(ServiceState::IncompatibleMessagingPattern) => {
                RequestResponseOpenError::IncompatibleMessagingPattern
            }
            ServiceAvailabilityState::ServiceState(ServiceState::InsufficientPermissions) => {
                RequestResponseOpenError::InsufficientPermissions
            }
            ServiceAvailabilityState::ServiceState(ServiceState::HangsInCreation) => {
                RequestResponseOpenError::HangsInCreation
            }
            ServiceAvailabilityState::ServiceState(ServiceState::Corrupted) => {
                RequestResponseOpenError::ServiceInCorruptedState
            }
        }
    }
}

/// Errors that can occur when a new [`MessagingPattern::RequestResponse`] [`Service`] shall be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RequestResponseCreateError {
    /// The [`Service`] already exists.
    AlreadyExists,
    /// Errors that indicate either an implementation issue or a wrongly configured system.
    InternalFailure,
    /// Multiple processes are trying to create the same [`Service`].
    IsBeingCreatedByAnotherInstance,
    /// The process has insufficient permissions to create the [`Service`].
    InsufficientPermissions,
    /// The [`Service`]s creation timeout has passed and it is still not initialized. Can be caused
    /// by a process that crashed during [`Service`] creation.
    HangsInCreation,
    /// Some underlying resources of the [`Service`] are either missing, corrupted or unaccessible.
    ServiceInCorruptedState,
}

impl core::fmt::Display for RequestResponseCreateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "RequestResponseCreateError::{:?}", self)
    }
}

impl core::error::Error for RequestResponseCreateError {}

impl From<ServiceAvailabilityState> for RequestResponseCreateError {
    fn from(value: ServiceAvailabilityState) -> Self {
        match value {
            ServiceAvailabilityState::IncompatibleRequestType
            | ServiceAvailabilityState::IncompatibleResponseType
            | ServiceAvailabilityState::ServiceState(ServiceState::IncompatibleMessagingPattern) => {
                RequestResponseCreateError::AlreadyExists
            }
            ServiceAvailabilityState::ServiceState(ServiceState::InsufficientPermissions) => {
                RequestResponseCreateError::InsufficientPermissions
            }
            ServiceAvailabilityState::ServiceState(ServiceState::HangsInCreation) => {
                RequestResponseCreateError::HangsInCreation
            }
            ServiceAvailabilityState::ServiceState(ServiceState::Corrupted) => {
                RequestResponseCreateError::ServiceInCorruptedState
            }
        }
    }
}

/// Errors that can occur when a [`MessagingPattern::RequestResponse`] [`Service`] shall be
/// created or opened.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RequestResponseOpenOrCreateError {
    /// Failures that can occur when an existing [`Service`] could not be opened.
    RequestResponseOpenError(RequestResponseOpenError),
    /// Failures that can occur when a [`Service`] could not be created.
    RequestResponseCreateError(RequestResponseCreateError),
    /// Can occur when another process creates and removes the same [`Service`] repeatedly with a
    /// high frequency.
    SystemInFlux,
}

impl From<ServiceAvailabilityState> for RequestResponseOpenOrCreateError {
    fn from(value: ServiceAvailabilityState) -> Self {
        RequestResponseOpenOrCreateError::RequestResponseOpenError(value.into())
    }
}

impl From<RequestResponseOpenError> for RequestResponseOpenOrCreateError {
    fn from(value: RequestResponseOpenError) -> Self {
        RequestResponseOpenOrCreateError::RequestResponseOpenError(value)
    }
}

impl From<RequestResponseCreateError> for RequestResponseOpenOrCreateError {
    fn from(value: RequestResponseCreateError) -> Self {
        RequestResponseOpenOrCreateError::RequestResponseCreateError(value)
    }
}

impl core::fmt::Display for RequestResponseOpenOrCreateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "RequestResponseOpenOrCreateError::{:?}", self)
    }
}

impl core::error::Error for RequestResponseOpenOrCreateError {}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
enum ServiceAvailabilityState {
    ServiceState(ServiceState),
    IncompatibleRequestType,
    IncompatibleResponseType,
}

/// Builder to create new [`MessagingPattern::RequestResponse`] based [`Service`]s
///
/// # Example
///
/// See [`crate::service`]
#[derive(Debug)]
pub struct Builder<
    RequestPayload: Debug,
    RequestHeader: Debug,
    ResponsePayload: Debug,
    ResponseHeader: Debug,
    ServiceType: Service,
> {
    base: builder::BuilderWithServiceType<ServiceType>,
    override_request_alignment: Option<usize>,
    override_response_alignment: Option<usize>,
    verify_enable_safe_overflow_for_requests: bool,
    verify_enable_safe_overflow_for_responses: bool,
    verify_max_active_responses: bool,
    verify_max_active_requests: bool,
    verify_max_borrowed_responses: bool,
    verify_max_borrowed_requests: bool,
    verify_max_response_buffer_size: bool,
    verify_max_request_buffer_size: bool,
    verify_max_servers: bool,
    verify_max_clients: bool,
    verify_max_nodes: bool,

    _request_payload: PhantomData<RequestPayload>,
    _request_header: PhantomData<RequestHeader>,
    _response_payload: PhantomData<ResponsePayload>,
    _response_header: PhantomData<ResponseHeader>,
}

impl<
        RequestPayload: Debug,
        RequestHeader: Debug,
        ResponsePayload: Debug,
        ResponseHeader: Debug,
        ServiceType: Service,
    > Builder<RequestPayload, RequestHeader, ResponsePayload, ResponseHeader, ServiceType>
{
    pub(crate) fn new(base: builder::BuilderWithServiceType<ServiceType>) -> Self {
        Self {
            base,
            override_request_alignment: None,
            override_response_alignment: None,
            verify_enable_safe_overflow_for_requests: false,
            verify_enable_safe_overflow_for_responses: false,
            verify_max_active_responses: false,
            verify_max_active_requests: false,
            verify_max_borrowed_responses: false,
            verify_max_borrowed_requests: false,
            verify_max_response_buffer_size: false,
            verify_max_request_buffer_size: false,
            verify_max_servers: false,
            verify_max_clients: false,
            verify_max_nodes: false,
            _request_payload: PhantomData,
            _request_header: PhantomData,
            _response_payload: PhantomData,
            _response_header: PhantomData,
        }
    }

    fn config_details_mut(&mut self) -> &mut static_config::request_response::StaticConfig {
        match self.base.service_config.messaging_pattern {
            static_config::messaging_pattern::MessagingPattern::RequestResponse(ref mut v) => v,
            _ => {
                fatal_panic!(from self, "This should never happen! Accessing wrong messaging pattern in RequestResponse builder!");
            }
        }
    }

    fn config_details(&self) -> &static_config::request_response::StaticConfig {
        match self.base.service_config.messaging_pattern {
            static_config::messaging_pattern::MessagingPattern::RequestResponse(ref v) => v,
            _ => {
                fatal_panic!(from self, "This should never happen! Accessing wrong messaging pattern in RequestResponse builder!");
            }
        }
    }

    /// Sets the request user header type of the [`Service`].
    pub fn request_user_header<M: Debug>(
        self,
    ) -> Builder<RequestPayload, M, ResponsePayload, ResponseHeader, ServiceType> {
        unsafe {
            core::mem::transmute::<
                Self,
                Builder<RequestPayload, M, ResponsePayload, ResponseHeader, ServiceType>,
            >(self)
        }
    }

    /// Sets the response user header type of the [`Service`].
    pub fn response_user_header<M: Debug>(
        self,
    ) -> Builder<RequestPayload, RequestHeader, ResponsePayload, M, ServiceType> {
        unsafe {
            core::mem::transmute::<
                Self,
                Builder<RequestPayload, RequestHeader, ResponsePayload, M, ServiceType>,
            >(self)
        }
    }

    /// If the [`Service`] is created, it defines the request [`Alignment`] of the payload for the
    /// service. If an existing [`Service`] is opened it requires the service to have at least the
    /// defined [`Alignment`]. If the Payload [`Alignment`] is greater than the provided
    /// [`Alignment`] then the Payload [`Alignment`] is used.
    pub fn request_payload_alignment(mut self, alignment: Alignment) -> Self {
        self.override_request_alignment = Some(alignment.value());
        self
    }

    /// If the [`Service`] is created, it defines the response [`Alignment`] of the payload for the
    /// service. If an existing [`Service`] is opened it requires the service to have at least the
    /// defined [`Alignment`]. If the Payload [`Alignment`] is greater than the provided
    /// [`Alignment`] then the Payload [`Alignment`] is used.
    pub fn response_payload_alignment(mut self, alignment: Alignment) -> Self {
        self.override_response_alignment = Some(alignment.value());
        self
    }

    /// If the [`Service`] is created, defines the overflow behavior of the service for requests.
    /// If an existing [`Service`] is opened it requires the service to have the defined overflow
    /// behavior.
    pub fn enable_safe_overflow_for_requests(mut self, value: bool) -> Self {
        self.config_details_mut().enable_safe_overflow_for_requests = value;
        self.verify_enable_safe_overflow_for_requests = true;
        self
    }

    /// If the [`Service`] is created, defines the overflow behavior of the service for responses.
    /// If an existing [`Service`] is opened it requires the service to have the defined overflow
    /// behavior.
    pub fn enable_safe_overflow_for_responses(mut self, value: bool) -> Self {
        self.config_details_mut().enable_safe_overflow_for_responses = value;
        self.verify_enable_safe_overflow_for_responses = true;
        self
    }

    /// Defines how many active responses a [`Client`](crate::port::client::Client) can hold in
    /// parallel. The objects are used to receive the samples to a request that was sent earlier
    /// to a [`Server`](crate::port::server::Server)
    pub fn max_active_responses(mut self, value: usize) -> Self {
        self.config_details_mut().max_active_responses = value;
        self.verify_max_active_responses = true;
        self
    }

    /// Defines how many active requests a [`Server`](crate::port::server::Server) can hold in
    /// parallel. The objects are used to send answers to a request that was received earlier
    /// from a [`Client`](crate::port::client::Client)
    pub fn max_active_requests(mut self, value: usize) -> Self {
        self.config_details_mut().max_active_requests = value;
        self.verify_max_active_requests = true;
        self
    }

    /// If the [`Service`] is created it defines how many samples a
    /// response can borrow at most in parallel. If an existing
    /// [`Service`] is opened it defines the minimum required.
    pub fn max_borrowed_responses(mut self, value: usize) -> Self {
        self.config_details_mut().max_borrowed_responses = value;
        self.verify_max_borrowed_responses = true;
        self
    }

    /// If the [`Service`] is created it defines how many samples a
    /// request can borrow at most in parallel. If an existing
    /// [`Service`] is opened it defines the minimum required.
    pub fn max_borrowed_requests(mut self, value: usize) -> Self {
        self.config_details_mut().max_borrowed_requests = value;
        self.verify_max_borrowed_requests = true;
        self
    }

    /// If the [`Service`] is created it defines how many responses fit in the
    /// [`Clients`](crate::port::client::Client)s buffer. If an existing
    /// [`Service`] is opened it defines the minimum required.
    pub fn max_response_buffer_size(mut self, value: usize) -> Self {
        self.config_details_mut().max_response_buffer_size = value;
        self.verify_max_response_buffer_size = true;
        self
    }

    /// If the [`Service`] is created it defines how many requests fit in the
    /// [`Server`](crate::port::server::Server)s buffer. If an existing
    /// [`Service`] is opened it defines the minimum required.
    pub fn max_request_buffer_size(mut self, value: usize) -> Self {
        self.config_details_mut().max_request_buffer_size = value;
        self.verify_max_request_buffer_size = true;
        self
    }

    /// If the [`Service`] is created it defines how many [`crate::port::server::Server`]s shall
    /// be supported at most. If an existing [`Service`] is opened it defines how many
    /// [`crate::port::server::Server`]s must be at least supported.
    pub fn max_servers(mut self, value: usize) -> Self {
        self.config_details_mut().max_servers = value;
        self.verify_max_servers = true;
        self
    }

    /// If the [`Service`] is created it defines how many [`crate::port::client::Client`]s shall
    /// be supported at most. If an existing [`Service`] is opened it defines how many
    /// [`crate::port::client::Client`]s must be at least supported.
    pub fn max_clients(mut self, value: usize) -> Self {
        self.config_details_mut().max_clients = value;
        self.verify_max_clients = true;
        self
    }

    /// If the [`Service`] is created it defines how many [`Node`](crate::node::Node)s shall
    /// be able to open it in parallel. If an existing [`Service`] is opened it defines how many
    /// [`Node`](crate::node::Node)s must be at least supported.
    pub fn max_nodes(mut self, value: usize) -> Self {
        self.config_details_mut().max_nodes = value;
        self.verify_max_nodes = true;
        self
    }

    fn adjust_configuration_to_meaningful_values(&mut self) {
        let origin = format!("{:?}", self);
        let settings = self.base.service_config.request_response_mut();

        if settings.max_request_buffer_size == 0 {
            warn!(from origin,
                "Setting the maximum size of the request buffer to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_request_buffer_size = 1;
        }

        if settings.max_response_buffer_size == 0 {
            warn!(from origin,
                "Setting the maximum size of the response buffer to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_response_buffer_size = 1;
        }

        if settings.max_active_requests == 0 {
            warn!(from origin,
                "Setting the maximum number of active requests to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_active_requests = 1;
        }

        if settings.max_active_responses == 0 {
            warn!(from origin,
                "Setting the maximum number of active responses to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_active_responses = 1;
        }

        if settings.max_borrowed_responses == 0 {
            warn!(from origin,
                "Setting the maximum number of borrowed responses to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_borrowed_responses = 1;
        }

        if settings.max_borrowed_requests == 0 {
            warn!(from origin,
                "Setting the maximum number of borrowed requests to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_borrowed_requests = 1;
        }

        if settings.max_servers == 0 {
            warn!(from origin,
                "Setting the maximum number of servers to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_servers = 1;
        }

        if settings.max_clients == 0 {
            warn!(from origin,
                "Setting the maximum number of clients to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_clients = 1;
        }

        if settings.max_nodes == 0 {
            warn!(from origin,
                "Setting the maximum number of nodes to 0 is not supported. Adjust it to 1, the smallest supported value.");
            settings.max_nodes = 1;
        }
    }

    fn verify_service_configuration(
        &self,
        existing_settings: &static_config::StaticConfig,
        required_attributes: &AttributeVerifier,
    ) -> Result<static_config::request_response::StaticConfig, RequestResponseOpenError> {
        let msg = "Unable to open request response service";

        let existing_attributes = existing_settings.attributes();
        if let Err(incompatible_key) = required_attributes.verify_requirements(existing_attributes)
        {
            fail!(from self, with RequestResponseOpenError::IncompatibleAttributes,
                "{} due to incompatible service attribute key \"{}\". The following attributes {:?} are required but the service has the attributes {:?}.",
                msg, incompatible_key, required_attributes, existing_attributes);
        }

        let required_configuration = self.base.service_config.request_response();
        let existing_configuration = match &existing_settings.messaging_pattern {
            MessagingPattern::RequestResponse(ref v) => v,
            p => {
                fail!(from self, with RequestResponseOpenError::IncompatibleMessagingPattern,
                    "{} since a service with the messaging pattern {:?} exists but MessagingPattern::RequestResponse is required.",
                    msg, p);
            }
        };

        if self.verify_enable_safe_overflow_for_requests
            && existing_configuration.enable_safe_overflow_for_requests
                != required_configuration.enable_safe_overflow_for_requests
        {
            fail!(from self, with RequestResponseOpenError::IncompatibleOverflowBehaviorForRequests,
                "{} since the service has an incompatible safe overflow behavior for requests.",
                msg);
        }

        if self.verify_enable_safe_overflow_for_responses
            && existing_configuration.enable_safe_overflow_for_responses
                != required_configuration.enable_safe_overflow_for_responses
        {
            fail!(from self, with RequestResponseOpenError::IncompatibleOverflowBehaviorForResponses,
                "{} since the service has an incompatible safe overflow behavior for responses.",
                msg);
        }

        if self.verify_max_active_responses
            && existing_configuration.max_active_responses
                < required_configuration.max_active_responses
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedAmountOfActiveResponses,
                "{} since the service supports only {} active responses but {} are required.",
                msg, existing_configuration.max_active_responses, required_configuration.max_active_responses);
        }

        if self.verify_max_active_requests
            && existing_configuration.max_active_requests
                < required_configuration.max_active_requests
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedAmountOfActiveRequests,
                "{} since the service supports only {} active requests but {} are required.",
                msg, existing_configuration.max_active_requests, required_configuration.max_active_requests);
        }

        if self.verify_max_borrowed_responses
            && existing_configuration.max_borrowed_responses
                < required_configuration.max_borrowed_responses
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedAmountOfBorrowedResponses,
                "{} since the service supports only {} borrowed responses but {} are required.",
                msg, existing_configuration.max_borrowed_responses, required_configuration.max_borrowed_responses);
        }

        if self.verify_max_borrowed_requests
            && existing_configuration.max_borrowed_requests
                < required_configuration.max_borrowed_requests
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedAmountOfBorrowedRequests,
                "{} since the service supports only {} borrowed requests but {} are required.",
                msg, existing_configuration.max_borrowed_requests, required_configuration.max_borrowed_requests);
        }

        if self.verify_max_response_buffer_size
            && existing_configuration.max_response_buffer_size
                < required_configuration.max_response_buffer_size
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedResponseBufferSize,
                "{} since the service supports a maximum response buffer size of {} but a size of {} is required.",
                msg, existing_configuration.max_response_buffer_size, required_configuration.max_response_buffer_size);
        }

        if self.verify_max_request_buffer_size
            && existing_configuration.max_request_buffer_size
                < required_configuration.max_request_buffer_size
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedRequestBufferSize,
                "{} since the service supports a maximum request buffer size of {} but a size of {} is required.",
                msg, existing_configuration.max_request_buffer_size, required_configuration.max_request_buffer_size);
        }

        if self.verify_max_servers
            && existing_configuration.max_servers < required_configuration.max_servers
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedAmountOfServers,
                "{} since the service supports at most {} servers but {} are required.",
                msg, existing_configuration.max_servers, required_configuration.max_servers);
        }

        if self.verify_max_clients
            && existing_configuration.max_clients < required_configuration.max_clients
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedAmountOfClients,
                "{} since the service supports at most {} clients but {} are required.",
                msg, existing_configuration.max_clients, required_configuration.max_clients);
        }

        if self.verify_max_nodes
            && existing_configuration.max_nodes < required_configuration.max_nodes
        {
            fail!(from self, with RequestResponseOpenError::DoesNotSupportRequestedAmountOfNodes,
                "{} since the service supports at most {} nodes but {} are required.",
                msg, existing_configuration.max_nodes, required_configuration.max_nodes);
        }

        Ok(existing_configuration.clone())
    }

    fn is_service_available(
        &mut self,
        error_msg: &str,
    ) -> Result<
        Option<(static_config::StaticConfig, ServiceType::StaticStorage)>,
        ServiceAvailabilityState,
    > {
        match self.base.is_service_available(error_msg) {
            Ok(Some((config, storage))) => {
                if !self
                    .config_details()
                    .request_message_type_details
                    .is_compatible_to(&config.request_response().request_message_type_details)
                {
                    fail!(from self, with ServiceAvailabilityState::IncompatibleRequestType,
                        "{} since the services uses the request type \"{:?}\" which is not compatible to the requested type \"{:?}\".",
                        error_msg, &config.request_response().request_message_type_details,
                        self.config_details().request_message_type_details);
                }

                if !self
                    .config_details()
                    .response_message_type_details
                    .is_compatible_to(&config.request_response().response_message_type_details)
                {
                    fail!(from self, with ServiceAvailabilityState::IncompatibleResponseType,
                        "{} since the services uses the response type \"{:?}\" which is not compatible to the requested type \"{:?}\".",
                        error_msg, &config.request_response().response_message_type_details,
                        self.config_details().response_message_type_details);
                }

                Ok(Some((config, storage)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ServiceAvailabilityState::ServiceState(e)),
        }
    }

    fn create_impl(
        &mut self,
        attributes: &AttributeSpecifier,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseCreateError> {
        let msg = "Unable to create request response service";
        self.adjust_configuration_to_meaningful_values();

        match self.is_service_available(msg)? {
            Some(_) => {
                fail!(from self, with RequestResponseCreateError::AlreadyExists,
                    "{} since the service already exists.",
                    msg);
            }
            None => {
                let service_tag = self
                    .base
                    .create_node_service_tag(msg, RequestResponseCreateError::InternalFailure)?;

                let static_config = match self.base.create_static_config_storage() {
                    Ok(static_config) => static_config,
                    Err(StaticStorageCreateError::AlreadyExists) => {
                        fail!(from self, with RequestResponseCreateError::AlreadyExists,
                            "{} since the service already exists.", msg);
                    }
                    Err(StaticStorageCreateError::Creation) => {
                        fail!(from self, with RequestResponseCreateError::IsBeingCreatedByAnotherInstance,
                            "{} since the service is being created by another instance.", msg);
                    }
                    Err(StaticStorageCreateError::InsufficientPermissions) => {
                        fail!(from self, with RequestResponseCreateError::InsufficientPermissions,
                            "{} since the static service information could not be created due to insufficient permissions.",
                            msg);
                    }
                    Err(e) => {
                        fail!(from self, with RequestResponseCreateError::InternalFailure,
                            "{} since the static service information could not be created due to an internal failure ({:?}).",
                            msg, e);
                    }
                };

                let request_response_config = self.base.service_config.request_response();
                let dynamic_config_setting = DynamicConfigSettings {
                    number_of_servers: request_response_config.max_servers,
                    number_of_clients: request_response_config.max_clients,
                };

                let dynamic_config = match self.base.create_dynamic_config_storage(
                    dynamic_config::MessagingPattern::RequestResponse(
                        dynamic_config::request_response::DynamicConfig::new(
                            &dynamic_config_setting,
                        ),
                    ),
                    dynamic_config::request_response::DynamicConfig::memory_size(
                        &dynamic_config_setting,
                    ),
                    request_response_config.max_nodes,
                ) {
                    Ok(dynamic_config) => dynamic_config,
                    Err(DynamicStorageCreateError::AlreadyExists) => {
                        fail!(from self, with RequestResponseCreateError::ServiceInCorruptedState,
                            "{} since the dynamic config of a previous instance of the service still exists.",
                            msg);
                    }
                    Err(e) => {
                        fail!(from self, with RequestResponseCreateError::InternalFailure,
                            "{} since the dynamic service segment could not be created ({:?}).",
                            msg, e);
                    }
                };

                self.base.service_config.attributes = attributes.0.clone();
                let serialized_service_config = fail!(from self,
                          when ServiceType::ConfigSerializer::serialize(&self.base.service_config),
                          with RequestResponseCreateError::ServiceInCorruptedState,
                          "{} since the configuration could not be serialized.",
                          msg);

                let mut unlocked_static_details = fail!(from self,
                        when static_config.unlock(serialized_service_config.as_slice()),
                        with RequestResponseCreateError::ServiceInCorruptedState,
                        "{} since the configuration could not be written into the static storage.",
                        msg);

                unlocked_static_details.release_ownership();
                if let Some(mut service_tag) = service_tag {
                    service_tag.release_ownership();
                }

                Ok(request_response::PortFactory::new(
                    ServiceType::__internal_from_state(service::ServiceState::new(
                        self.base.service_config.clone(),
                        self.base.shared_node.clone(),
                        dynamic_config,
                        unlocked_static_details,
                    )),
                ))
            }
        }
    }

    fn open_impl(
        &mut self,
        attributes: &AttributeVerifier,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseOpenError> {
        const OPEN_RETRY_LIMIT: usize = 5;
        let msg = "Unable to open request response service";

        let mut service_open_retry_count = 0;
        loop {
            match self.is_service_available(msg)? {
                None => {
                    fail!(from self, with RequestResponseOpenError::DoesNotExist,
                        "{} since the service does not exist.",
                        msg);
                }
                Some((static_config, static_storage)) => {
                    let request_response_static_config =
                        self.verify_service_configuration(&static_config, attributes)?;

                    let service_tag = self
                        .base
                        .create_node_service_tag(msg, RequestResponseOpenError::InternalFailure)?;

                    let dynamic_config = match self.base.open_dynamic_config_storage() {
                        Ok(v) => v,
                        Err(OpenDynamicStorageFailure::IsMarkedForDestruction) => {
                            fail!(from self, with RequestResponseOpenError::IsMarkedForDestruction,
                                "{} since the service is marked for destruction.",
                                msg);
                        }
                        Err(OpenDynamicStorageFailure::ExceedsMaxNumberOfNodes) => {
                            fail!(from self, with RequestResponseOpenError::ExceedsMaxNumberOfNodes,
                                "{} since it would exceed the maximum number of supported nodes.",
                                msg);
                        }
                        Err(OpenDynamicStorageFailure::DynamicStorageOpenError(
                            DynamicStorageOpenError::DoesNotExist,
                        )) => {
                            fail!(from self, with RequestResponseOpenError::ServiceInCorruptedState,
                                "{} since the dynamic segment of the service is missing.",
                                msg);
                        }
                        Err(e) => {
                            if self.is_service_available(msg)?.is_none() {
                                fail!(from self, with RequestResponseOpenError::DoesNotExist,
                                    "{} since the service does not exist.", msg);
                            }

                            service_open_retry_count += 1;

                            if OPEN_RETRY_LIMIT < service_open_retry_count {
                                fail!(from self, with RequestResponseOpenError::ServiceInCorruptedState,
                                    "{} since the dynamic service information could not be opened ({:?}).",
                                    msg, e);
                            }

                            continue;
                        }
                    };

                    self.base.service_config.messaging_pattern =
                        MessagingPattern::RequestResponse(request_response_static_config.clone());

                    if let Some(mut service_tag) = service_tag {
                        service_tag.release_ownership();
                    }

                    return Ok(request_response::PortFactory::new(
                        ServiceType::__internal_from_state(service::ServiceState::new(
                            static_config,
                            self.base.shared_node.clone(),
                            dynamic_config,
                            static_storage,
                        )),
                    ));
                }
            }
        }
    }

    fn open_or_create_impl(
        mut self,
        attributes: &AttributeVerifier,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseOpenOrCreateError> {
        let msg = "Unable to open or create request response service";

        let mut retry_count = 0;
        loop {
            if RETRY_LIMIT < retry_count {
                fail!(from self,
                      with RequestResponseOpenOrCreateError::SystemInFlux,
                      "{} since an instance is creating and removing the same service repeatedly.",
                      msg);
            }
            retry_count += 1;

            match self.is_service_available(msg)? {
                Some(_) => match self.open_impl(attributes) {
                    Ok(factory) => return Ok(factory),
                    Err(RequestResponseOpenError::DoesNotExist) => continue,
                    Err(e) => return Err(e.into()),
                },
                None => {
                    match self.create_impl(&AttributeSpecifier(attributes.attributes().clone())) {
                        Ok(factory) => return Ok(factory),
                        Err(RequestResponseCreateError::AlreadyExists)
                        | Err(RequestResponseCreateError::IsBeingCreatedByAnotherInstance) => {
                            continue;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }
    }

    fn prepare_message_type_details(&mut self) {
        self.config_details_mut().request_message_type_details = MessageTypeDetails::from::<
            header::request_response::RequestHeader,
            RequestHeader,
            RequestPayload,
        >(TypeVariant::FixedSize);

        self.config_details_mut().response_message_type_details = MessageTypeDetails::from::<
            header::request_response::ResponseHeader,
            ResponseHeader,
            ResponsePayload,
        >(TypeVariant::FixedSize);

        if let Some(alignment) = self.override_request_alignment {
            self.config_details_mut()
                .request_message_type_details
                .payload
                .alignment = self
                .config_details()
                .request_message_type_details
                .payload
                .alignment
                .max(alignment);
        }

        if let Some(alignment) = self.override_response_alignment {
            self.config_details_mut()
                .response_message_type_details
                .payload
                .alignment = self
                .config_details()
                .response_message_type_details
                .payload
                .alignment
                .max(alignment);
        }
    }

    /// If the [`Service`] exists, it will be opened otherwise a new [`Service`] will be
    /// created.
    pub fn open_or_create(
        self,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseOpenOrCreateError> {
        self.open_or_create_with_attributes(&AttributeVerifier::new())
    }

    /// If the [`Service`] exists, it will be opened otherwise a new [`Service`] will be
    /// created. It defines a set of attributes.
    ///
    /// If the [`Service`] already exists all attribute requirements must be satisfied,
    /// and service payload type must be the same, otherwise the open process will fail.
    /// If the [`Service`] does not exist the required attributes will be defined in the [`Service`].
    pub fn open_or_create_with_attributes(
        mut self,
        required_attributes: &AttributeVerifier,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseOpenOrCreateError> {
        self.prepare_message_type_details();
        self.open_or_create_impl(required_attributes)
    }

    /// Opens an existing [`Service`].
    pub fn open(
        self,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseOpenError> {
        self.open_with_attributes(&AttributeVerifier::new())
    }

    /// Opens an existing [`Service`] with attribute requirements. If the defined attribute
    /// requirements are not satisfied the open process will fail.
    pub fn open_with_attributes(
        mut self,
        required_attributes: &AttributeVerifier,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseOpenError> {
        self.prepare_message_type_details();
        self.open_impl(required_attributes)
    }

    /// Creates a new [`Service`].
    pub fn create(
        self,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseCreateError> {
        self.create_with_attributes(&AttributeSpecifier::new())
    }

    /// Creates a new [`Service`] with a set of attributes.
    pub fn create_with_attributes(
        mut self,
        attributes: &AttributeSpecifier,
    ) -> Result<request_response::PortFactory<ServiceType>, RequestResponseCreateError> {
        self.prepare_message_type_details();
        self.create_impl(attributes)
    }
}
