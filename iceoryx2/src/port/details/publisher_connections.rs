// Copyright (c) 2023 Contributors to the Eclipse Foundation
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

use core::cell::UnsafeCell;

extern crate alloc;
use alloc::sync::Arc;

use crate::{
    port::port_identifiers::{UniquePublisherId, UniqueSubscriberId},
    service::{
        self, config_scheme::connection_config,
        dynamic_config::publish_subscribe::PublisherDetails, naming_scheme::connection_name,
        static_config::publish_subscribe::StaticConfig, ServiceState,
    },
};

use crate::port::update_connections::ConnectionFailure;
use iceoryx2_bb_log::fail;
use iceoryx2_cal::named_concept::NamedConceptBuilder;
use iceoryx2_cal::zero_copy_connection::*;

use super::data_segment::DataSegmentView;

#[derive(Debug)]
pub(crate) struct Connection<Service: service::Service> {
    pub(crate) receiver: <Service::Connection as ZeroCopyConnection>::Receiver,
    pub(crate) data_segment: DataSegmentView<Service>,
    pub(crate) publisher_id: UniquePublisherId,
}

impl<Service: service::Service> Connection<Service> {
    fn new(
        this: &PublisherConnections<Service>,
        details: &PublisherDetails,
    ) -> Result<Self, ConnectionFailure> {
        let msg = format!(
            "Unable to establish connection to publisher {:?} from subscriber {:?}.",
            details.publisher_id, this.subscriber_id
        );

        let global_config = this.service_state.shared_node.config();
        let receiver = fail!(from this,
                        when <Service::Connection as ZeroCopyConnection>::
                            Builder::new( &connection_name(details.publisher_id, this.subscriber_id))
                                    .config(&connection_config::<Service>(global_config))
                                    .buffer_size(this.buffer_size)
                                    .receiver_max_borrowed_samples(this.static_config.subscriber_max_borrowed_samples)
                                    .enable_safe_overflow(this.static_config.enable_safe_overflow)
                                    .number_of_samples_per_segment(details.number_of_samples)
                                    .max_supported_shared_memory_segments(details.max_number_of_segments)
                                    .timeout(global_config.global.service.creation_timeout)
                                    .create_receiver(),
                        "{} since the zero copy connection could not be established.", msg);

        let data_segment = fail!(from this,
                            when DataSegmentView::open(details, global_config),
                            "{} since the publishers data segment could not be opened.", msg);

        Ok(Self {
            receiver,
            data_segment,
            publisher_id: details.publisher_id,
        })
    }
}
#[derive(Debug)]
pub(crate) struct PublisherConnections<Service: service::Service> {
    connections: Vec<UnsafeCell<Option<Arc<Connection<Service>>>>>,
    subscriber_id: UniqueSubscriberId,
    pub(crate) service_state: Arc<ServiceState<Service>>,
    pub(crate) static_config: StaticConfig,
    pub(crate) buffer_size: usize,
}

impl<Service: service::Service> PublisherConnections<Service> {
    pub(crate) fn new(
        capacity: usize,
        subscriber_id: UniqueSubscriberId,
        service_state: Arc<ServiceState<Service>>,
        static_config: &StaticConfig,
        buffer_size: usize,
    ) -> Self {
        Self {
            connections: (0..capacity).map(|_| UnsafeCell::new(None)).collect(),
            subscriber_id,
            service_state,
            static_config: static_config.clone(),
            buffer_size,
        }
    }

    pub(crate) fn subscriber_id(&self) -> UniqueSubscriberId {
        self.subscriber_id
    }

    pub(crate) fn get(&self, index: usize) -> &Option<Arc<Connection<Service>>> {
        unsafe { &*self.connections[index].get() }
    }

    // only used internally as convinience function
    #[allow(clippy::mut_from_ref)]
    pub(crate) fn get_mut(&self, index: usize) -> &mut Option<Arc<Connection<Service>>> {
        #[deny(clippy::mut_from_ref)]
        unsafe {
            &mut *self.connections[index].get()
        }
    }

    pub(crate) fn create(
        &self,
        index: usize,
        details: &PublisherDetails,
    ) -> Result<(), ConnectionFailure> {
        *self.get_mut(index) = Some(Arc::new(Connection::new(self, details)?));

        Ok(())
    }

    pub(crate) fn remove(&self, index: usize) {
        *self.get_mut(index) = None;
    }

    pub(crate) fn len(&self) -> usize {
        self.connections.len()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.connections.capacity()
    }
}
