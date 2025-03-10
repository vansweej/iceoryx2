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

#![allow(non_camel_case_types, non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use crate::posix::types::*;
use crate::posix::Struct;

pub unsafe fn open_with_mode(pathname: *const c_char, flags: int, mode: mode_t) -> int {
    libc::open(pathname, flags, mode)
}

pub unsafe fn fstat(fd: int, buf: *mut stat_t) -> int {
    let mut os_specific_buffer = native_stat_t::new();
    match libc::fstat(fd, &mut os_specific_buffer) {
        0 => {
            *buf = os_specific_buffer.into();
            0
        }
        v => v,
    }
}

pub unsafe fn fcntl_int(fd: int, cmd: int, arg: int) -> int {
    libc::fcntl(fd, cmd, arg)
}

pub unsafe fn fcntl(fd: int, cmd: int, arg: *mut flock) -> int {
    libc::fcntl(fd, cmd, arg)
}

pub unsafe fn fcntl2(fd: int, cmd: int) -> int {
    libc::fcntl(fd, cmd)
}

pub unsafe fn fchmod(fd: int, mode: mode_t) -> int {
    libc::fchmod(fd, mode)
}

pub unsafe fn open(pathname: *const c_char, flags: int) -> int {
    libc::open(pathname, flags)
}
