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

#[generic_tests::define]
mod service_request_response {
    use iceoryx2::node::NodeBuilder;
    use iceoryx2::prelude::*;
    use iceoryx2::service::builder::request_response::{
        RequestResponseCreateError, RequestResponseOpenError,
    };
    use iceoryx2::testing::*;
    use iceoryx2_bb_testing::assert_that;

    #[test]
    fn open_existing_service_works<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open();

        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_non_existing_service_fails<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open();

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotExist) );
    }

    #[test]
    fn creating_existing_service_fails<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let _sut = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create()
            .unwrap();

        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        assert_that!(sut_create.err(), eq Some(RequestResponseCreateError::AlreadyExists) );
    }

    #[test]
    fn open_or_create_works_with_existing_and_non_existing_services<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open_or_create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open_or_create();

        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn when_created_service_goes_out_of_scope_the_service_is_removed<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(false));

        let sut = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(true));

        drop(sut);

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(false));
    }

    #[test]
    fn when_last_opened_service_goes_out_of_scope_the_service_is_removed<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(false));

        let sut = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        let sut_open_1 = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open();

        let sut_open_2 = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open();

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(true));

        drop(sut);

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(true));

        drop(sut_open_1);

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(true));

        drop(sut_open_2);

        assert_that!(
            Sut::does_exist(&service_name, &config, MessagingPattern::RequestResponse), eq Ok(false));
    }

    #[test]
    fn opening_service_with_mismatching_request_type_fails<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<i64, u64>()
            .open();

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleRequestType));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .request_user_header::<u64>()
            .open();

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleRequestType));
    }

    #[test]
    fn opening_service_with_incompatible_request_type_alignment_fails<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .request_payload_alignment(Alignment::new(512).unwrap())
            .open();

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleRequestType));
    }

    #[test]
    fn opening_service_with_compatible_request_type_alignment_works<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .request_payload_alignment(Alignment::new(512).unwrap())
            .create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .request_payload_alignment(Alignment::new(128).unwrap())
            .open();

        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn opening_service_with_mismatching_response_type_fails<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, i64>()
            .open();

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleResponseType));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .response_user_header::<u64>()
            .open();

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleResponseType));
    }

    #[test]
    fn opening_service_with_incompatible_response_type_alignment_fails<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .response_payload_alignment(Alignment::new(512).unwrap())
            .open();

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleResponseType));
    }

    #[test]
    fn opening_service_with_compatible_response_type_alignment_works<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .request_payload_alignment(Alignment::new(512).unwrap())
            .create();

        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .request_payload_alignment(Alignment::new(128).unwrap())
            .open();

        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn opening_service_with_attributes_and_acquiring_attributes_works<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let attribute_key = "wanna try this head";
        let attribute_value = "no its a brainslug";

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let _sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create_with_attributes(
                &AttributeSpecifier::new().define(attribute_key, attribute_value),
            )
            .unwrap();

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open()
            .unwrap();

        let attributes = sut_open.attributes();
        assert_that!(attributes.len(), eq 1);
        assert_that!(attributes.get_key_value_at(attribute_key, 0), eq Some(attribute_value));
    }

    #[test]
    fn opening_service_with_incompatible_attributes_fails<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let attribute_key = "there is a muffin";
        let attribute_value = "with molten chocolate";
        let attribute_incompatible_value = "its delicious";

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let _sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create_with_attributes(
                &AttributeSpecifier::new().define(attribute_key, attribute_value),
            )
            .unwrap();

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open_with_attributes(
                &AttributeVerifier::new().require(&attribute_key, &attribute_incompatible_value),
            );

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleAttributes));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open_with_attributes(
                &AttributeVerifier::new().require_key(&attribute_incompatible_value),
            );

        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleAttributes));
    }

    #[test]
    fn opening_service_with_compatible_attributes_works<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let attribute_key = "kermit the brave knight";
        let attribute_value = "rides on hypnotoad into the sunset";

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let _sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create_with_attributes(
                &AttributeSpecifier::new().define(attribute_key, attribute_value),
            )
            .unwrap();

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open_with_attributes(
                &AttributeVerifier::new().require(&attribute_key, &attribute_value),
            );

        assert_that!(sut_open, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open_with_attributes(&AttributeVerifier::new().require_key(&attribute_key));

        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn recreate_after_drop_works<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();
        assert_that!(sut, is_ok);

        drop(sut);

        let sut2 = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();
        assert_that!(sut2, is_ok);
    }

    #[test]
    fn open_fails_when_service_does_not_satisfy_request_overflow_requirement<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .enable_safe_overflow_for_requests(true)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .enable_safe_overflow_for_requests(false)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleOverflowBehaviorForRequests));
    }

    #[test]
    fn open_fails_when_service_does_not_satisfy_response_overflow_requirement<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .enable_safe_overflow_for_responses(true)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .enable_safe_overflow_for_responses(false)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::IncompatibleOverflowBehaviorForResponses));
    }

    #[test]
    fn open_verifies_max_active_responses_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_active_responses(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_active_responses(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedAmountOfActiveResponses));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_active_responses(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_active_requests_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_active_requests(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_active_requests(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedAmountOfActiveRequests));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_active_requests(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_borrowed_responses_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_borrowed_responses(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_borrowed_responses(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedAmountOfBorrowedResponses));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_borrowed_responses(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_borrowed_requests_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_borrowed_requests(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_borrowed_requests(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedAmountOfBorrowedRequests));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_borrowed_requests(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_response_buffer_size_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_response_buffer_size(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_response_buffer_size(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedResponseBufferSize));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_response_buffer_size(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_request_buffer_size_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_request_buffer_size(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_request_buffer_size(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedRequestBufferSize));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_request_buffer_size(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_amount_of_servers_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_servers(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_servers(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedAmountOfServers));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_servers(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_amount_of_clients_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_clients(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_clients(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedAmountOfClients));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_clients(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn open_verifies_max_amount_of_nodes_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_nodes(10)
            .create();
        assert_that!(sut_create, is_ok);

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_nodes(11)
            .open();
        assert_that!(sut_open.err(), eq Some(RequestResponseOpenError::DoesNotSupportRequestedAmountOfNodes));

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_nodes(9)
            .open();
        assert_that!(sut_open, is_ok);
    }

    #[test]
    fn service_builder_adjusts_config_to_sane_values<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_active_requests(0)
            .max_active_responses(0)
            .max_borrowed_requests(0)
            .max_borrowed_responses(0)
            .max_request_buffer_size(0)
            .max_response_buffer_size(0)
            .max_servers(0)
            .max_clients(0)
            .max_nodes(0)
            .create();
        assert_that!(sut_create, is_ok);
        let sut_create = sut_create.unwrap();

        assert_that!(sut_create.static_config().max_active_requests(), eq 1);
        assert_that!(sut_create.static_config().max_active_responses(), eq 1);
        assert_that!(sut_create.static_config().max_borrowed_requests(), eq 1);
        assert_that!(sut_create.static_config().max_borrowed_responses(), eq 1);
        assert_that!(sut_create.static_config().max_request_buffer_size(), eq 1);
        assert_that!(sut_create.static_config().max_response_buffer_size(), eq 1);
        assert_that!(sut_create.static_config().max_servers(), eq 1);
        assert_that!(sut_create.static_config().max_clients(), eq 1);
        assert_that!(sut_create.static_config().max_nodes(), eq 1);
    }

    #[test]
    fn service_builder_parameters_override_default_config<Sut: Service>() {
        let service_name = generate_service_name();
        let mut config = generate_isolated_config();
        let rpc_config = &mut config.defaults.request_response;
        rpc_config.enable_safe_overflow_for_requests = true;
        rpc_config.enable_safe_overflow_for_responses = true;
        rpc_config.max_active_requests = 100;
        rpc_config.max_active_responses = 100;
        rpc_config.max_borrowed_requests = 100;
        rpc_config.max_borrowed_responses = 100;
        rpc_config.max_request_buffer_size = 100;
        rpc_config.max_response_buffer_size = 100;
        rpc_config.max_servers = 100;
        rpc_config.max_clients = 100;
        rpc_config.max_nodes = 100;

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .enable_safe_overflow_for_requests(false)
            .enable_safe_overflow_for_responses(false)
            .max_active_requests(1)
            .max_active_responses(2)
            .max_borrowed_requests(3)
            .max_borrowed_responses(4)
            .max_request_buffer_size(5)
            .max_response_buffer_size(6)
            .max_servers(7)
            .max_clients(8)
            .max_nodes(9)
            .create();
        assert_that!(sut_create, is_ok);
        let sut_create = sut_create.unwrap();

        assert_that!(sut_create.static_config().has_safe_overflow_for_requests(), eq false);
        assert_that!(sut_create.static_config().has_safe_overflow_for_responses(), eq false);
        assert_that!(sut_create.static_config().max_active_requests(), eq 1);
        assert_that!(sut_create.static_config().max_active_responses(), eq 2);
        assert_that!(sut_create.static_config().max_borrowed_requests(), eq 3);
        assert_that!(sut_create.static_config().max_borrowed_responses(), eq 4);
        assert_that!(sut_create.static_config().max_request_buffer_size(), eq 5);
        assert_that!(sut_create.static_config().max_response_buffer_size(), eq 6);
        assert_that!(sut_create.static_config().max_servers(), eq 7);
        assert_that!(sut_create.static_config().max_clients(), eq 8);
        assert_that!(sut_create.static_config().max_nodes(), eq 9);
    }

    #[test]
    fn service_builder_uses_config_when_user_sets_nothing<Sut: Service>() {
        let service_name = generate_service_name();
        let mut config = generate_isolated_config();
        let rpc_config = &mut config.defaults.request_response;
        rpc_config.enable_safe_overflow_for_requests = true;
        rpc_config.enable_safe_overflow_for_responses = true;
        rpc_config.max_active_requests = 11;
        rpc_config.max_active_responses = 12;
        rpc_config.max_borrowed_requests = 13;
        rpc_config.max_borrowed_responses = 14;
        rpc_config.max_request_buffer_size = 15;
        rpc_config.max_response_buffer_size = 16;
        rpc_config.max_servers = 17;
        rpc_config.max_clients = 18;
        rpc_config.max_nodes = 19;

        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .create();
        assert_that!(sut_create, is_ok);
        let sut_create = sut_create.unwrap();

        assert_that!(sut_create.static_config().has_safe_overflow_for_requests(), eq true);
        assert_that!(sut_create.static_config().has_safe_overflow_for_responses(), eq true);
        assert_that!(sut_create.static_config().max_active_requests(), eq 11);
        assert_that!(sut_create.static_config().max_active_responses(), eq 12);
        assert_that!(sut_create.static_config().max_borrowed_requests(), eq 13);
        assert_that!(sut_create.static_config().max_borrowed_responses(), eq 14);
        assert_that!(sut_create.static_config().max_request_buffer_size(), eq 15);
        assert_that!(sut_create.static_config().max_response_buffer_size(), eq 16);
        assert_that!(sut_create.static_config().max_servers(), eq 17);
        assert_that!(sut_create.static_config().max_clients(), eq 18);
        assert_that!(sut_create.static_config().max_nodes(), eq 19);
    }

    #[test]
    fn opened_service_reads_config_correctly<Sut: Service>() {
        let service_name = generate_service_name();
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let _sut_create = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .enable_safe_overflow_for_requests(false)
            .enable_safe_overflow_for_responses(false)
            .max_active_requests(1)
            .max_active_responses(2)
            .max_borrowed_requests(3)
            .max_borrowed_responses(4)
            .max_request_buffer_size(5)
            .max_response_buffer_size(6)
            .max_servers(7)
            .max_clients(8)
            .max_nodes(9)
            .create()
            .unwrap();

        let sut_open = node
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open()
            .unwrap();

        assert_that!(sut_open.static_config().has_safe_overflow_for_requests(), eq false);
        assert_that!(sut_open.static_config().has_safe_overflow_for_responses(), eq false);
        assert_that!(sut_open.static_config().max_active_requests(), eq 1);
        assert_that!(sut_open.static_config().max_active_responses(), eq 2);
        assert_that!(sut_open.static_config().max_borrowed_requests(), eq 3);
        assert_that!(sut_open.static_config().max_borrowed_responses(), eq 4);
        assert_that!(sut_open.static_config().max_request_buffer_size(), eq 5);
        assert_that!(sut_open.static_config().max_response_buffer_size(), eq 6);
        assert_that!(sut_open.static_config().max_servers(), eq 7);
        assert_that!(sut_open.static_config().max_clients(), eq 8);
        assert_that!(sut_open.static_config().max_nodes(), eq 9);
    }

    #[test]
    fn list_finds_created_services<Sut: Service>() {
        const NUMBER_OF_SERVICES: usize = 12;
        let config = generate_isolated_config();
        let node = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let mut services = vec![];
        let mut service_names = vec![];

        for _ in 0..NUMBER_OF_SERVICES {
            let service_name = generate_service_name();
            let sut = node
                .service_builder(&service_name)
                .request_response::<u64, u64>()
                .create()
                .unwrap();
            services.push(sut);
            service_names.push(service_name);
        }

        for name in &service_names {
            assert_that!(Sut::does_exist(name, &config, MessagingPattern::RequestResponse).unwrap(), eq true);
        }

        let mut number_of_listed_services = 0;
        Sut::list(&config, |service| {
            assert_that!(service_names, contains * service.static_details.name());
            number_of_listed_services += 1;
            CallbackProgression::Continue
        })
        .unwrap();

        assert_that!(number_of_listed_services, eq service_names.len());
    }

    #[test]
    fn service_cannot_be_opened_by_more_nodes_than_specified<Sut: Service>() {
        let config = generate_isolated_config();
        let service_name = generate_service_name();

        let node_1 = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let node_2 = NodeBuilder::new().config(&config).create::<Sut>().unwrap();
        let node_3 = NodeBuilder::new().config(&config).create::<Sut>().unwrap();

        let sut_1 = node_1
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .max_nodes(2)
            .create();
        assert_that!(sut_1, is_ok);

        let sut_2 = node_2
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open();
        assert_that!(sut_2, is_ok);

        let sut_3 = node_3
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open();
        assert_that!(sut_3.err(), eq Some(RequestResponseOpenError::ExceedsMaxNumberOfNodes));
        drop(sut_2);

        let sut_3 = node_3
            .service_builder(&service_name)
            .request_response::<u64, u64>()
            .open();
        assert_that!(sut_3, is_ok);
    }

    #[instantiate_tests(<iceoryx2::service::ipc::Service>)]
    mod ipc {}

    #[instantiate_tests(<iceoryx2::service::local::Service>)]
    mod local {}
}
