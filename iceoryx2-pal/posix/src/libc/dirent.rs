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

#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use crate::{posix::types::*, scandir_impl};

pub unsafe fn scandir(path: *const c_char, namelist: *mut *mut *mut dirent) -> int {
    scandir_impl(path, namelist)
}

pub unsafe fn mkdir(pathname: *const c_char, mode: mode_t) -> int {
    libc::mkdir(pathname, mode)
}

pub unsafe fn opendir(dirname: *const c_char) -> *mut DIR {
    libc::opendir(dirname)
}

pub unsafe fn closedir(dirp: *mut DIR) -> int {
    libc::closedir(dirp)
}

pub unsafe fn dirfd(dirp: *mut DIR) -> int {
    libc::dirfd(dirp)
}

pub unsafe fn readdir(dirp: *mut DIR) -> *const dirent {
    libc::readdir(dirp)
}

pub unsafe fn readdir_r(dirp: *mut DIR, entry: *mut dirent, result: *mut *mut dirent) -> int {
    libc::readdir_r(dirp, entry, result)
}
