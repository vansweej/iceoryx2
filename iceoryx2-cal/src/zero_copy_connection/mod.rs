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

pub mod common;
pub mod posix_shared_memory;
pub mod process_local;
pub mod used_chunk_list;

use core::fmt::Debug;
use core::time::Duration;

pub use crate::shared_memory::PointerOffset;
use crate::static_storage::file::{NamedConcept, NamedConceptBuilder, NamedConceptMgmt};
pub use iceoryx2_bb_system_types::file_name::*;
pub use iceoryx2_bb_system_types::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZeroCopyPortRemoveError {
    InternalError,
    VersionMismatch,
    InsufficientPermissions,
    DoesNotExist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ZeroCopyCreationError {
    InternalError,
    AnotherInstanceIsAlreadyConnected,
    InsufficientPermissions,
    VersionMismatch,
    ConnectionMaybeCorrupted,
    InvalidSampleSize,
    InitializationNotYetFinalized,
    IncompatibleBufferSize,
    IncompatibleMaxBorrowedSampleSetting,
    IncompatibleOverflowSetting,
    IncompatibleNumberOfSamples,
    IncompatibleNumberOfSegments,
}

impl core::fmt::Display for ZeroCopyCreationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "{}::{:?}", std::stringify!(Self), self)
    }
}

impl core::error::Error for ZeroCopyCreationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroCopySendError {
    ConnectionCorrupted,
    ReceiveBufferFull,
    UsedChunkListFull,
}

impl core::fmt::Display for ZeroCopySendError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "{}::{:?}", std::stringify!(Self), self)
    }
}

impl core::error::Error for ZeroCopySendError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroCopyReceiveError {
    ReceiveWouldExceedMaxBorrowValue,
}

impl core::fmt::Display for ZeroCopyReceiveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "{}::{:?}", std::stringify!(Self), self)
    }
}

impl core::error::Error for ZeroCopyReceiveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroCopyReclaimError {
    ReceiverReturnedCorruptedPointerOffset,
}

impl core::fmt::Display for ZeroCopyReclaimError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "{}::{:?}", std::stringify!(Self), self)
    }
}

impl core::error::Error for ZeroCopyReclaimError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroCopyReleaseError {
    RetrieveBufferFull,
}

impl core::fmt::Display for ZeroCopyReleaseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "{}::{:?}", std::stringify!(Self), self)
    }
}

impl core::error::Error for ZeroCopyReleaseError {}

pub const DEFAULT_BUFFER_SIZE: usize = 4;
pub const DEFAULT_ENABLE_SAFE_OVERFLOW: bool = false;
pub const DEFAULT_MAX_BORROWED_SAMPLES: usize = 4;
pub const DEFAULT_MAX_SUPPORTED_SHARED_MEMORY_SEGMENTS: u8 = 1;

pub trait ZeroCopyConnectionBuilder<C: ZeroCopyConnection>: NamedConceptBuilder<C> {
    fn buffer_size(self, value: usize) -> Self;
    fn enable_safe_overflow(self, value: bool) -> Self;
    fn receiver_max_borrowed_samples(self, value: usize) -> Self;
    fn max_supported_shared_memory_segments(self, value: u8) -> Self;
    fn number_of_samples_per_segment(self, value: usize) -> Self;
    /// The timeout defines how long the [`ZeroCopyConnectionBuilder`] should wait for
    /// concurrent
    /// [`ZeroCopyConnectionBuilder::create_sender()`] or
    /// [`ZeroCopyConnectionBuilder::create_receiver()`] call to finalize its initialization.
    /// By default it is set to [`Duration::ZERO`] for no timeout.
    fn timeout(self, value: Duration) -> Self;

    fn create_sender(self) -> Result<C::Sender, ZeroCopyCreationError>;
    fn create_receiver(self) -> Result<C::Receiver, ZeroCopyCreationError>;
}

pub trait ZeroCopyPortDetails {
    fn buffer_size(&self) -> usize;
    fn has_enabled_safe_overflow(&self) -> bool;
    fn max_borrowed_samples(&self) -> usize;
    fn max_supported_shared_memory_segments(&self) -> u8;
    fn is_connected(&self) -> bool;
}

pub trait ZeroCopySender: Debug + ZeroCopyPortDetails + NamedConcept {
    fn try_send(
        &self,
        ptr: PointerOffset,
        sample_size: usize,
    ) -> Result<Option<PointerOffset>, ZeroCopySendError>;

    fn blocking_send(
        &self,
        ptr: PointerOffset,
        sample_size: usize,
    ) -> Result<Option<PointerOffset>, ZeroCopySendError>;

    fn reclaim(&self) -> Result<Option<PointerOffset>, ZeroCopyReclaimError>;

    /// # Safety
    ///
    /// * must ensure that no receiver is still holding data, otherwise data races may occur on
    ///     receiver side
    /// * must ensure that [`ZeroCopySender::try_send()`] and [`ZeroCopySender::blocking_send()`]
    ///     are not called after using this method
    unsafe fn acquire_used_offsets<F: FnMut(PointerOffset)>(&self, callback: F);
}

pub trait ZeroCopyReceiver: Debug + ZeroCopyPortDetails + NamedConcept {
    fn has_data(&self) -> bool;
    fn receive(&self) -> Result<Option<PointerOffset>, ZeroCopyReceiveError>;
    fn release(&self, ptr: PointerOffset) -> Result<(), ZeroCopyReleaseError>;
}

pub trait ZeroCopyConnection: Debug + Sized + NamedConceptMgmt {
    type Sender: ZeroCopySender;
    type Receiver: ZeroCopyReceiver;
    type Builder: ZeroCopyConnectionBuilder<Self>;

    /// Removes the [`ZeroCopySender`] forcefully from the [`ZeroCopyConnection`]. This shall only
    /// be called when the [`ZeroCopySender`] died and the connection shall be cleaned up without
    /// causing any problems on the living [`ZeroCopyReceiver`] side.
    ///
    /// # Safety
    ///
    ///  * must ensure that the [`ZeroCopySender`] died while being connected.
    unsafe fn remove_sender(
        name: &FileName,
        config: &Self::Configuration,
    ) -> Result<(), ZeroCopyPortRemoveError>;

    /// Removes the [`ZeroCopyReceiver`] forcefully from the [`ZeroCopyConnection`]. This shall
    /// only be called when the [`ZeroCopySender`] died and the connection shall be cleaned up
    /// without causing any problems on the living [`ZeroCopySender`] side.
    ///
    /// # Safety
    ///
    ///  * must ensure that the [`ZeroCopyReceiver`] died while being connected.
    unsafe fn remove_receiver(
        name: &FileName,
        config: &Self::Configuration,
    ) -> Result<(), ZeroCopyPortRemoveError>;

    /// Returns true if the connection supports safe overflow
    fn does_support_safe_overflow() -> bool {
        false
    }

    /// Returns true if the buffer size of the connection can be configured
    fn has_configurable_buffer_size() -> bool {
        false
    }

    /// The default suffix of every zero copy connection
    fn default_suffix() -> FileName {
        unsafe { FileName::new_unchecked(b".rx") }
    }
}
