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

mod shm_allocator_pool_allocator {
    use std::{alloc::Layout, collections::HashSet, ptr::NonNull};

    use iceoryx2_bb_elementary::allocator::AllocationError;
    use iceoryx2_bb_memory::bump_allocator::BumpAllocator;
    use iceoryx2_bb_testing::assert_that;
    use iceoryx2_cal::{
        shm_allocator::{pool_allocator::*, ShmAllocationError, ShmAllocator},
        zero_copy_connection::PointerOffset,
    };

    const MAX_SUPPORTED_ALIGNMENT: usize = 32;
    const BUCKET_CONFIG: Layout = unsafe { Layout::from_size_align_unchecked(32, 4) };
    const MEM_SIZE: usize = 8192;
    const PAYLOAD_SIZE: usize = 1024;

    struct TestFixture {
        _payload_memory: Box<[u8; MEM_SIZE]>,
        _base_address: NonNull<[u8]>,
        sut: Box<PoolAllocator>,
    }

    impl TestFixture {
        fn new(bucket_layout: Layout) -> Self {
            let mut payload_memory = Box::new([0u8; MEM_SIZE]);
            let base_address =
                unsafe { NonNull::<[u8]>::new_unchecked(&mut payload_memory[0..PAYLOAD_SIZE]) };
            let allocator = BumpAllocator::new(
                unsafe { NonNull::new_unchecked(payload_memory[PAYLOAD_SIZE..].as_mut_ptr()) },
                MEM_SIZE,
            );
            let config = &Config { bucket_layout };
            let sut = Box::new(unsafe {
                PoolAllocator::new_uninit(MAX_SUPPORTED_ALIGNMENT, base_address, config)
            });

            unsafe { sut.init(&allocator).unwrap() };

            Self {
                _payload_memory: payload_memory,
                _base_address: base_address,
                sut,
            }
        }
    }

    #[test]
    fn is_setup_correctly() {
        let test = TestFixture::new(Layout::from_size_align(2, 1).unwrap());

        assert_that!(test.sut.number_of_buckets() as usize, eq PAYLOAD_SIZE / 2);
        assert_that!(test.sut.relative_start_address() as usize, eq 0);

        let test = TestFixture::new(BUCKET_CONFIG);

        assert_that!(test.sut.bucket_size(), eq BUCKET_CONFIG.size());
        assert_that!(test.sut.max_alignment(), eq BUCKET_CONFIG.align());
    }

    #[test]
    fn allocate_and_release_all_buckets_works() {
        const REPETITIONS: usize = 10;
        let test = TestFixture::new(BUCKET_CONFIG);

        for _ in 0..REPETITIONS {
            let mut mem_set = HashSet::new();
            for _ in 0..test.sut.number_of_buckets() {
                let memory = unsafe { test.sut.allocate(BUCKET_CONFIG).unwrap() };
                // the returned offset must be a multiple of the bucket size
                assert_that!((memory.value() - test.sut.relative_start_address()) % BUCKET_CONFIG.size(), eq 0);
                assert_that!(mem_set.insert(memory.value()), eq true);
            }

            assert_that!(unsafe { test.sut.allocate(BUCKET_CONFIG) }, eq Err(ShmAllocationError::AllocationError(AllocationError::OutOfMemory)));

            for memory in mem_set {
                unsafe {
                    test.sut
                        .deallocate(PointerOffset::new(memory), BUCKET_CONFIG)
                }
            }
        }
    }

    #[test]
    fn allocate_twice_release_once_until_memory_is_exhausted_works() {
        const REPETITIONS: usize = 10;
        let test = TestFixture::new(BUCKET_CONFIG);

        for _ in 0..REPETITIONS {
            let mut mem_set = HashSet::new();
            for _ in 0..(test.sut.number_of_buckets() - 1) {
                let memory_1 = unsafe { test.sut.allocate(BUCKET_CONFIG).unwrap() };
                // the returned offset must be a multiple of the bucket size
                assert_that!((memory_1.value() - test.sut.relative_start_address()) % BUCKET_CONFIG.size(), eq 0);

                let memory_2 = unsafe { test.sut.allocate(BUCKET_CONFIG).unwrap() };
                // the returned offset must be a multiple of the bucket size
                assert_that!((memory_2.value() - test.sut.relative_start_address()) % BUCKET_CONFIG.size(), eq 0);
                assert_that!(mem_set.insert(memory_2.value()), eq true);

                unsafe {
                    test.sut.deallocate(memory_1, BUCKET_CONFIG);
                }
            }

            let memory = unsafe { test.sut.allocate(BUCKET_CONFIG).unwrap() };
            // the returned offset must be a multiple of the bucket size
            assert_that!((memory.value() - test.sut.relative_start_address()) % BUCKET_CONFIG.size(), eq 0);
            assert_that!(mem_set.insert(memory.value()), eq true);

            assert_that!(unsafe { test.sut.allocate(BUCKET_CONFIG) }, eq Err(ShmAllocationError::AllocationError(AllocationError::OutOfMemory)));

            for memory in mem_set {
                unsafe {
                    test.sut
                        .deallocate(PointerOffset::new(memory), BUCKET_CONFIG)
                }
            }
        }
    }

    #[test]
    fn allocate_with_unsupported_alignment_fails() {
        let test = TestFixture::new(Layout::from_size_align(BUCKET_CONFIG.size(), 1).unwrap());
        assert_that!(unsafe { test.sut.allocate(BUCKET_CONFIG) }, eq Err(ShmAllocationError::ExceedsMaxSupportedAlignment));
    }
}
