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

#[cfg(test)]
mod service_static_config_message_type_details {
    use core::mem::size_of;
    use iceoryx2::service::static_config::message_type_details::{TypeDetail, TypeVariant};
    use iceoryx2_bb_testing::assert_that;

    #[cfg(target_pointer_width = "32")]
    const ALIGNMENT: usize = 4;
    #[cfg(target_pointer_width = "64")]
    const ALIGNMENT: usize = 8;

    #[test]
    fn test_internal_new() {
        #[repr(C)]
        struct Tmp;
        let sut = TypeDetail::__internal_new::<Tmp>(TypeVariant::FixedSize);
        let expected = TypeDetail {
            variant: TypeVariant::FixedSize,
            type_name: core::any::type_name::<Tmp>().to_string(),
            size: 0,
            alignment: 1,
        };
        assert_that!(sut, eq expected);

        let sut = TypeDetail::__internal_new::<i64>(TypeVariant::FixedSize);
        let expected = TypeDetail {
            variant: TypeVariant::FixedSize,
            type_name: core::any::type_name::<i64>().to_string(),
            size: 8,
            alignment: ALIGNMENT,
        };

        assert_that!(sut, eq expected);

        let sut = TypeDetail::__internal_new::<TypeDetail>(TypeVariant::FixedSize);
        let expected = TypeDetail {
            variant: TypeVariant::FixedSize,
            type_name: core::any::type_name::<TypeDetail>().to_string(),
            size: size_of::<TypeDetail>(),
            alignment: ALIGNMENT,
        };

        assert_that!(sut, eq expected);
    }
}
