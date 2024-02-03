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

use iceoryx2_bb_container::semantic_string::SemanticString;
use iceoryx2_bb_posix::config::*;
use iceoryx2_bb_posix::file::{File, FileBuilder};
use iceoryx2_bb_posix::file_descriptor::FileDescriptorBased;
use iceoryx2_bb_posix::file_descriptor::FileDescriptorManagement;
use iceoryx2_bb_posix::file_lock::LockType;
use iceoryx2_bb_posix::shared_memory::Permission;
use iceoryx2_bb_posix::unix_datagram_socket::CreationMode;
use iceoryx2_bb_posix::{process_state::*, unique_system_id::UniqueSystemId};
use iceoryx2_bb_system_types::{file_name::FileName, file_path::FilePath};
use iceoryx2_bb_testing::assert_that;
use iceoryx2_pal_posix::posix::{self, Struct};

fn generate_file_path() -> FilePath {
    let mut file = FileName::new(b"process_state_tests").unwrap();
    file.push_bytes(
        UniqueSystemId::new()
            .unwrap()
            .value()
            .to_string()
            .as_bytes(),
    )
    .unwrap();

    FilePath::from_path_and_file(&test_directory(), &file).unwrap()
}

#[test]
pub fn process_state_guard_can_be_created() {
    let path = generate_file_path();

    let guard = ProcessGuard::new(&path).unwrap();

    assert_that!(*guard.path(), eq path);
    assert_that!(File::does_exist(&path).unwrap(), eq true);
}

#[test]
pub fn process_state_guard_removes_file_when_dropped() {
    let path = generate_file_path();

    let guard = ProcessGuard::new(&path).unwrap();
    assert_that!(File::does_exist(&path).unwrap(), eq true);
    drop(guard);
    assert_that!(File::does_exist(&path).unwrap(), eq false);
}

#[test]
pub fn process_state_guard_cannot_use_already_existing_file() {
    let path = generate_file_path();

    let file = FileBuilder::new(&path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    let guard = ProcessGuard::new(&path);
    assert_that!(guard.is_err(), eq true);
    assert_that!(guard.err().unwrap(), eq ProcessGuardCreateError::AlreadyExists);

    file.remove_self().unwrap();
}

#[test]
pub fn process_state_guard_can_remove_already_existing_file() {
    let path = generate_file_path();
    let mut cleaner_path = path.clone();
    cleaner_path.push_bytes(b"_cleanup").unwrap();

    FileBuilder::new(&path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    FileBuilder::new(&cleaner_path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    let guard = unsafe { ProcessGuard::remove(&path) };
    assert_that!(guard.is_ok(), eq true);
    assert_that!(guard.ok().unwrap(), eq true);
}

#[test]
pub fn process_state_monitor_detects_dead_state() {
    let path = generate_file_path();
    let mut cleaner_path = path.clone();
    cleaner_path.push_bytes(b"_cleanup").unwrap();

    let file = FileBuilder::new(&path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();
    let cleaner_file = FileBuilder::new(&cleaner_path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    let monitor = ProcessMonitor::new(&path).unwrap();

    assert_that!(monitor.state().unwrap(), eq ProcessState::Dead);
    file.remove_self().unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::DoesNotExist);
    cleaner_file.remove_self().unwrap();
}

#[test]
pub fn process_state_monitor_detects_non_existing_state() {
    let path = generate_file_path();

    let monitor = ProcessMonitor::new(&path).unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::DoesNotExist);
}

#[test]
pub fn process_state_monitor_transitions_work_starting_from_non_existing_process() {
    let path = generate_file_path();
    let mut cleaner_path = path.clone();
    cleaner_path.push_bytes(b"_cleanup").unwrap();

    let monitor = ProcessMonitor::new(&path).unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::DoesNotExist);
    let file = FileBuilder::new(&path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    let cleaner_file = FileBuilder::new(&cleaner_path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    assert_that!(monitor.state().unwrap(), eq ProcessState::Dead);
    cleaner_file.remove_self().unwrap();
    assert_that!(monitor.state().err().unwrap(), eq ProcessMonitorStateError::CorruptedState);
    file.remove_self().unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::DoesNotExist);
}

#[test]
pub fn process_state_monitor_transitions_work_starting_from_existing_process() {
    let path = generate_file_path();
    let mut cleaner_path = path.clone();
    cleaner_path.push_bytes(b"_cleanup").unwrap();

    let file = FileBuilder::new(&path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();
    let cleaner_file = FileBuilder::new(&cleaner_path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    let monitor = ProcessMonitor::new(&path).unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::Dead);
    file.remove_self().unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::DoesNotExist);

    let file = FileBuilder::new(&path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::Dead);

    file.remove_self().unwrap();
    cleaner_file.remove_self().unwrap();
}

#[test]
pub fn process_state_monitor_detects_initialized_state() {
    let path = generate_file_path();

    let mut file = FileBuilder::new(&path)
        .creation_mode(CreationMode::PurgeAndCreate)
        .permission(Permission::OWNER_WRITE)
        .create()
        .unwrap();

    let monitor = ProcessMonitor::new(&path).unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::Starting);
    file.set_permission(Permission::OWNER_ALL).unwrap();
    file.remove_self().unwrap();
    assert_that!(monitor.state().unwrap(), eq ProcessState::DoesNotExist);
}

#[test]
pub fn process_state_cleaner_cannot_be_created_when_process_does_not_exist() {
    let path = generate_file_path();
    let mut cleaner_path = path.clone();
    cleaner_path.push_bytes(b"_cleanup").unwrap();

    let cleaner = ProcessCleaner::new(&path);
    assert_that!(cleaner, is_err);
    assert_that!(
        cleaner.err().unwrap(), eq
        ProcessCleanerCreateError::DoesNotExist
    );

    let _file = FileBuilder::new(&path)
        .has_ownership(true)
        .creation_mode(CreationMode::PurgeAndCreate)
        .permission(Permission::OWNER_WRITE)
        .create()
        .unwrap();

    let cleaner = ProcessCleaner::new(&path);
    assert_that!(cleaner, is_err);
    assert_that!(
        cleaner.err().unwrap(), eq
        ProcessCleanerCreateError::DoesNotExist
    );

    let _file = FileBuilder::new(&cleaner_path)
        .has_ownership(true)
        .creation_mode(CreationMode::PurgeAndCreate)
        .permission(Permission::OWNER_WRITE)
        .create()
        .unwrap();

    let cleaner = ProcessCleaner::new(&path);
    assert_that!(cleaner, is_err);
    assert_that!(
        cleaner.err().unwrap(), eq
        ProcessCleanerCreateError::DoesNotExist
    );
}

// START: OS with IPC only lock detection
//
// the lock detection does work on some OS only in the inter process context.
// In the process local context the lock is not detected when the fcntl GETLK call is originating
// from the same thread os the fcntl SETLK call. If it is called from a different thread GETLK
// blocks despite it should be non-blocking.
#[test]
#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
pub fn process_state_monitor_detects_alive_state_from_existing_process() {
    let path = generate_file_path();

    let guard = ProcessGuard::new(&path).unwrap();
    let monitor = ProcessMonitor::new(&path).unwrap();

    assert_that!(monitor.state().unwrap(), eq ProcessState::Alive);
    drop(guard);
    assert_that!(monitor.state().unwrap(), eq ProcessState::DoesNotExist);
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
pub fn process_state_guard_cannot_be_removed_when_locked() {
    let path = generate_file_path();

    let _guard = ProcessGuard::new(&path).unwrap();
    let result = unsafe { ProcessGuard::remove(&path) };

    assert_that!(result, is_err);
    assert_that!(
        result.err().unwrap(), eq
        ProcessGuardRemoveError::OwnedByAnotherProcess
    );
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
pub fn process_state_cleaner_cannot_be_acquired_from_living_process() {
    let path = generate_file_path();

    let _guard = ProcessGuard::new(&path).unwrap();
    let cleaner = ProcessCleaner::new(&path);
    assert_that!(cleaner, is_err);
    assert_that!(
        cleaner.err().unwrap(), eq
        ProcessCleanerCreateError::ProcessIsStillAlive
    );
}

#[test]
#[cfg(not(any(target_os = "linux", target_os = "freebsd", target_os = "macos")))]
pub fn process_state_cleaner_cannot_be_acquired_twice() {
    let path = generate_file_path();
    let mut cleaner_path = path.clone();
    cleaner_path.push_bytes(b"_cleanup").unwrap();

    let _file = FileBuilder::new(&path)
        .has_ownership(true)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    let cleaner_file = FileBuilder::new(&cleaner_path)
        .has_ownership(true)
        .creation_mode(CreationMode::PurgeAndCreate)
        .create()
        .unwrap();

    let mut new_lock_state = posix::flock::new();
    new_lock_state.l_type = LockType::Write as _;
    new_lock_state.l_whence = posix::SEEK_SET as _;

    unsafe {
        posix::fcntl(
            cleaner_file.file_descriptor().native_handle(),
            posix::F_SETLK,
            &mut new_lock_state,
        )
    };

    let _cleaner = ProcessCleaner::new(&path).unwrap();
    let cleaner = ProcessCleaner::new(&path);
    assert_that!(cleaner, is_err);
    assert_that!(
        cleaner.err().unwrap(), eq
        ProcessCleanerCreateError::OwnedByAnotherProcess
    );
}

// END: OS with IPC only lock detection
