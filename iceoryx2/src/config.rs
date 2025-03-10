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

//! # Examples
//!
//! ```
//! use iceoryx2::prelude::*;
//! use iceoryx2::config::Config;
//! use iceoryx2_bb_system_types::path::*;
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//!
//! // create a default config and override some entries
//! let mut custom_config = Config::default();
//! custom_config.defaults.publish_subscribe.max_publishers = 5;
//! custom_config.global.service.directory = Path::new(b"another_service_dir")?;
//!
//! let node = NodeBuilder::new()
//!     .config(&custom_config)
//!     .create::<ipc::Service>()?;
//!
//! let service = node.service_builder(&"MyServiceName".try_into()?)
//!     .publish_subscribe::<u64>()
//!     .open_or_create()?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! ## Set Global Config From Custom File
//!
//! The [`crate::config::Config::setup_global_config_from_file()`] call must be the first
//! call in the system. If another
//! instance accesses the global config, it will be loaded with default values and can no longer
//! be overridden with new values from a custom file.
//!
//! ```no_run
//! use iceoryx2::config::Config;
//! use iceoryx2_bb_system_types::file_path::FilePath;
//! use iceoryx2_bb_container::semantic_string::SemanticString;
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! Config::setup_global_config_from_file(
//!     &FilePath::new(b"my/custom/config/file.toml")?)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Generate Config From Custom File
//!
//! ```no_run
//! use iceoryx2::config::Config;
//! use iceoryx2_bb_system_types::file_path::FilePath;
//! use iceoryx2_bb_container::semantic_string::SemanticString;
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let custom_config = Config::from_file(
//!     &FilePath::new(b"my/custom/config/file.toml")?)?;
//! # Ok(())
//! # }
//! ```

use core::time::Duration;
use iceoryx2_bb_container::semantic_string::SemanticString;
use iceoryx2_bb_elementary::{lazy_singleton::*, CallbackProgression};
use iceoryx2_bb_posix::{
    file::{FileBuilder, FileOpenError},
    shared_memory::AccessMode,
    system_configuration::get_global_config_path,
};
use iceoryx2_bb_system_types::file_name::FileName;
use iceoryx2_bb_system_types::file_path::FilePath;
use iceoryx2_bb_system_types::path::Path;
use serde::{Deserialize, Serialize};

use iceoryx2_bb_log::{fail, fatal_panic, trace, warn};

use crate::service::port_factory::publisher::UnableToDeliverStrategy;

const DEFAULT_CONFIG_FILE_NAME: &[u8] = b"iceoryx2.toml";
const RELATIVE_LOCAL_CONFIG_PATH: &[u8] = b"config";
const RELATIVE_CONFIG_FILE_PATH: &[u8] = b"iceoryx2";

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
enum ConfigIterationFailure {
    #[allow(dead_code)] // TODO: #617
    UnableToAcquireCurrentUserDetails,
    TooLongUserConfigDirectory,
}

/// Failures occurring while creating a new [`Config`] object with [`Config::from_file()`] or
/// [`Config::setup_global_config_from_file()`]
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum ConfigCreationError {
    /// The config file could not be read.
    FailedToReadConfigFileContents,
    /// Parts of the config file could not be deserialized. Indicates some kind of syntax error.
    UnableToDeserializeContents,
    /// Insufficient permissions to open the config file.
    InsufficientPermissions,
    /// The provided config file does not exist
    ConfigFileDoesNotExist,
    /// Since the config file could not be opened
    UnableToOpenConfigFile,
}

impl core::fmt::Display for ConfigCreationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        std::write!(f, "ConfigCreationError::{:?}", self)
    }
}

impl core::error::Error for ConfigCreationError {}

/// All configurable settings of a [`crate::service::Service`].
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Service {
    /// The directory in which all service files are stored
    pub directory: Path,
    /// The suffix of the publishers data segment
    pub publisher_data_segment_suffix: FileName,
    /// The suffix of the static config file
    pub static_config_storage_suffix: FileName,
    /// The suffix of the dynamic config file
    pub dynamic_config_storage_suffix: FileName,
    /// Defines the time of how long another process will wait until the service creation is
    /// finalized
    pub creation_timeout: Duration,
    /// The suffix of a one-to-one connection
    pub connection_suffix: FileName,
    /// The suffix of a one-to-one connection
    pub event_connection_suffix: FileName,
}

/// All configurable settings of a [`crate::node::Node`].
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Node {
    /// The directory in which all node files are stored
    pub directory: Path,
    /// The suffix of the monitor token
    pub monitor_suffix: FileName,
    /// The suffix of the files where the node configuration is stored.
    pub static_config_suffix: FileName,
    /// The suffix of the service tags.
    pub service_tag_suffix: FileName,
    /// When true, the [`NodeBuilder`](crate::node::NodeBuilder) checks for dead nodes and
    /// cleans up all their stale resources whenever a new [`Node`](crate::node::Node) is
    /// created.
    pub cleanup_dead_nodes_on_creation: bool,
    /// When true, the [`NodeBuilder`](crate::node::NodeBuilder) checks for dead nodes and
    /// cleans up all their stale resources whenever an existing [`Node`](crate::node::Node) is
    /// going out of scope.
    pub cleanup_dead_nodes_on_destruction: bool,
}

/// The global settings
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Global {
    root_path_unix: Path,
    root_path_windows: Path,
    /// Prefix used for all files created during runtime
    pub prefix: FileName,
    /// [`crate::service::Service`] settings
    pub service: Service,
    /// [`crate::node::Node`] settings
    pub node: Node,
}

impl Global {
    /// The absolute path to the service directory where all static service infos are stored
    pub fn service_dir(&self) -> Path {
        let mut path = *self.root_path();
        path.add_path_entry(&self.service.directory).unwrap();
        path
    }

    /// The absolute path to the node directory where all node details are stored
    pub fn node_dir(&self) -> Path {
        let mut path = *self.root_path();
        path.add_path_entry(&self.node.directory).unwrap();
        path
    }

    /// The path under which all other directories or files will be created
    pub fn root_path(&self) -> &Path {
        #[cfg(target_os = "windows")]
        {
            &self.root_path_windows
        }
        #[cfg(not(target_os = "windows"))]
        {
            &self.root_path_unix
        }
    }

    /// Defines the path under which all other directories or files will be created
    pub fn set_root_path(&mut self, value: &Path) {
        #[cfg(target_os = "windows")]
        {
            self.root_path_windows = *value;
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.root_path_unix = *value;
        }
    }
}

/// Default settings. These values are used when the user in the code does not specify anything
/// else.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Defaults {
    /// Default settings for the messaging pattern publish-subscribe
    pub publish_subscribe: PublishSubscribe,
    /// Default settings for the messaging pattern event
    pub event: Event,
    /// Default settings for the messaging pattern request-response
    pub request_response: RequestResonse,
}

/// Default settings for the publish-subscribe messaging pattern. These settings are used unless
/// the user specifies custom QoS or port settings.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct PublishSubscribe {
    /// The maximum amount of supported [`crate::port::subscriber::Subscriber`]
    pub max_subscribers: usize,
    /// The maximum amount of supported [`crate::port::publisher::Publisher`]
    pub max_publishers: usize,
    /// The maximum amount of supported [`crate::node::Node`]s. Defines indirectly how many
    /// processes can open the service at the same time.
    pub max_nodes: usize,
    /// The maximum buffer size a [`crate::port::subscriber::Subscriber`] can have
    pub subscriber_max_buffer_size: usize,
    /// The maximum amount of [`crate::sample::Sample`]s a [`crate::port::subscriber::Subscriber`] can
    /// hold at the same time.
    pub subscriber_max_borrowed_samples: usize,
    /// The maximum amount of [`crate::sample_mut::SampleMut`]s a [`crate::port::publisher::Publisher`] can
    /// loan at the same time.
    pub publisher_max_loaned_samples: usize,
    /// The maximum history size a [`crate::port::subscriber::Subscriber`] can request from a
    /// [`crate::port::publisher::Publisher`].
    pub publisher_history_size: usize,
    /// Defines how the [`crate::port::subscriber::Subscriber`] buffer behaves when it is
    /// full. When safe overflow is activated, the [`crate::port::publisher::Publisher`] will
    /// replace the oldest [`crate::sample::Sample`] with the newest one.
    pub enable_safe_overflow: bool,
    /// If safe overflow is deactivated it defines the deliver strategy of the
    /// [`crate::port::publisher::Publisher`] when the [`crate::port::subscriber::Subscriber`]s
    /// buffer is full.
    pub unable_to_deliver_strategy: UnableToDeliverStrategy,
    /// Defines the size of the internal [`Subscriber`](crate::port::subscriber::Subscriber)
    /// buffer that contains expired connections. An
    /// connection is expired when the [`Publisher`](crate::port::publisher::Publisher)
    /// disconnected from a service and the connection
    /// still contains unconsumed [`Sample`](crate::sample::Sample)s.
    pub subscriber_expired_connection_buffer: usize,
}

/// Default settings for the event messaging pattern. These settings are used unless
/// the user specifies custom QoS or port settings.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Event {
    /// The maximum amount of supported [`crate::port::listener::Listener`]
    pub max_listeners: usize,
    /// The maximum amount of supported [`crate::port::notifier::Notifier`]
    pub max_notifiers: usize,
    /// The maximum amount of supported [`crate::node::Node`]s. Defines indirectly how many
    /// processes can open the service at the same time.
    pub max_nodes: usize,
    /// The largest event id supported by the event service
    pub event_id_max_value: usize,
    /// Defines the maximum allowed time between two consecutive notifications. If a notifiation
    /// is not sent after the defined time, every [`Listener`](crate::port::listener::Listener)
    /// that is attached to a [`WaitSet`](crate::waitset::WaitSet) will be notified.
    pub deadline: Option<Duration>,
    /// Defines the event id value that is emitted after a new notifier was created.
    pub notifier_created_event: Option<usize>,
    /// Defines the event id value that is emitted before a new notifier is dropped.
    pub notifier_dropped_event: Option<usize>,
    /// Defines the event id value that is emitted if a notifier was identified as dead.
    pub notifier_dead_event: Option<usize>,
}

/// Default settings for the request response messaging pattern. These settings are used unless
/// the user specifies custom QoS or port settings.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct RequestResonse {
    /// Defines if the request buffer of the [`Service`] safely overflows.
    pub enable_safe_overflow_for_requests: bool,
    /// Defines if the response buffer of the [`Service`] safely overflows.
    pub enable_safe_overflow_for_responses: bool,
    /// The maximum of active responses a [`crate::port::server::Server`] can hold in parallel.
    pub max_active_responses: usize,
    /// The maximum of active requests a [`crate::port::client::Client`] can hold in parallel.
    pub max_active_requests: usize,
    /// The maximum number of responses a [`crate::port::client::Client`] can borrow from
    /// an active request.
    pub max_borrowed_responses: usize,
    /// The maximum number of requests a [`crate::port::server::Server`] can borrow.
    pub max_borrowed_requests: usize,
    /// The maximum buffer size for responses for an active request.
    pub max_response_buffer_size: usize,
    /// The maximum buffer size for requests for a [`crate::port::server::Server`].
    pub max_request_buffer_size: usize,
    /// The maximum amount of supported [`crate::port::server::Server`]
    pub max_servers: usize,
    /// The maximum amount of supported [`crate::port::client::Client`]
    pub max_clients: usize,
    /// The maximum amount of supported [`crate::node::Node`]s. Defines indirectly how many
    /// processes can open the service at the same time.
    pub max_nodes: usize,
}

/// Represents the configuration that iceoryx2 will utilize. It is divided into two sections:
/// the [Global] settings, which must align with the iceoryx2 instance the application intends to
/// join, and the [Defaults] for communication within that iceoryx2 instance. The user has the
/// flexibility to override both sections.
#[non_exhaustive]
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Global settings for the iceoryx2 instance
    pub global: Global,
    /// Default settings
    pub defaults: Defaults,
}

static ICEORYX2_CONFIG: LazySingleton<Config> = LazySingleton::<Config>::new();

impl Default for Config {
    fn default() -> Self {
        Self {
            global: Global {
                root_path_unix: Path::new(b"/tmp/iceoryx2/").unwrap(),
                root_path_windows: Path::new(b"c:\\Temp\\iceoryx2\\").unwrap(),
                prefix: FileName::new(b"iox2_").unwrap(),
                service: Service {
                    directory: Path::new(b"services").unwrap(),
                    publisher_data_segment_suffix: FileName::new(b".publisher_data").unwrap(),
                    static_config_storage_suffix: FileName::new(b".service").unwrap(),
                    dynamic_config_storage_suffix: FileName::new(b".dynamic").unwrap(),
                    creation_timeout: Duration::from_millis(500),
                    connection_suffix: FileName::new(b".connection").unwrap(),
                    event_connection_suffix: FileName::new(b".event").unwrap(),
                },
                node: Node {
                    directory: Path::new(b"nodes").unwrap(),
                    monitor_suffix: FileName::new(b".node_monitor").unwrap(),
                    static_config_suffix: FileName::new(b".details").unwrap(),
                    service_tag_suffix: FileName::new(b".service_tag").unwrap(),
                    cleanup_dead_nodes_on_creation: true,
                    cleanup_dead_nodes_on_destruction: true,
                },
            },
            defaults: Defaults {
                request_response: RequestResonse {
                    enable_safe_overflow_for_requests: true,
                    enable_safe_overflow_for_responses: true,
                    max_active_responses: 4,
                    max_active_requests: 2,
                    max_borrowed_responses: 4,
                    max_borrowed_requests: 2,
                    max_response_buffer_size: 2,
                    max_request_buffer_size: 4,
                    max_servers: 2,
                    max_clients: 8,
                    max_nodes: 20,
                },
                publish_subscribe: PublishSubscribe {
                    max_subscribers: 8,
                    max_publishers: 2,
                    max_nodes: 20,
                    publisher_history_size: 0,
                    subscriber_max_buffer_size: 2,
                    subscriber_max_borrowed_samples: 2,
                    publisher_max_loaned_samples: 2,
                    enable_safe_overflow: true,
                    unable_to_deliver_strategy: UnableToDeliverStrategy::Block,
                    subscriber_expired_connection_buffer: 128,
                },
                event: Event {
                    max_listeners: 16,
                    max_notifiers: 16,
                    max_nodes: 36,
                    event_id_max_value: 4294967295,
                    deadline: None,
                    notifier_created_event: None,
                    notifier_dropped_event: None,
                    notifier_dead_event: None,
                },
            },
        }
    }
}

impl Config {
    fn relative_local_config_path() -> Path {
        fatal_panic!(from "Config::default_config_path",
            when Path::new(RELATIVE_LOCAL_CONFIG_PATH),
            "This should never happen! The relative local config path contains invalid symbols.")
    }

    /// The name of the default iceoryx2 config file
    pub fn default_config_file_name() -> FileName {
        fatal_panic!(from "Config::default_config_file",
            when FileName::new(DEFAULT_CONFIG_FILE_NAME),
            "This should never happen! The default config file name contains invalid symbols.")
    }

    /// Path to the default config file
    pub fn default_config_file_path() -> FilePath {
        fatal_panic!(from "Config::default_config_file_path",
            when FilePath::from_path_and_file(&Self::relative_local_config_path(), &Self::default_config_file_name()),
            "This should never happen! The default config file path contains invalid symbols.")
    }

    fn relative_config_path() -> Path {
        fatal_panic!(from "Config::relative_config_path",
            when Path::new(RELATIVE_CONFIG_FILE_PATH),
            "This should never happen! The relative config path contains invalid symbols.")
    }

    fn iterate_over_config_files<F: FnMut(FilePath) -> CallbackProgression>(
        mut callback: F,
    ) -> Result<(), ConfigIterationFailure> {
        let msg = "Unable to consider all possible config file paths";
        let origin = "Config::iterate_over_config_files";

        // prio 1: handle project local config file first
        let local_project_config = Self::default_config_file_path();
        if callback(local_project_config) == CallbackProgression::Stop {
            return Ok(());
        }

        // prio 2: lookup user config file
        #[cfg(not(target_os = "windows"))] // TODO: #617
        {
            let user = fail!(from origin,
                         when iceoryx2_bb_posix::user::User::from_self(),
                         with ConfigIterationFailure::UnableToAcquireCurrentUserDetails,
                         "{} since the current user details could not be acquired.", msg);
            let mut user_config = *user.config_dir();
            fail!(from origin,
                when user_config.add_path_entry(&Self::relative_config_path()),
                with ConfigIterationFailure::TooLongUserConfigDirectory,
                "{} since the resulting user config directory would be too long.", msg);
            let user_config = fail!(from origin,
                when FilePath::from_path_and_file(&user_config, &Self::default_config_file_name()),
                with ConfigIterationFailure::TooLongUserConfigDirectory,
                "{} since the resulting user config directory would be too long.", msg);

            if callback(user_config) == CallbackProgression::Stop {
                return Ok(());
            }
        }

        // prio 3: lookup global config file
        let mut global_config = get_global_config_path();
        fail!(from origin,
                when global_config.add_path_entry(&Self::relative_config_path()),
                with ConfigIterationFailure::TooLongUserConfigDirectory,
                "{} since the resulting global config directory would be too long.", msg);
        let global_config = fail!(from origin,
                when FilePath::from_path_and_file(&global_config, &Self::default_config_file_name()),
                with ConfigIterationFailure::TooLongUserConfigDirectory,
                "{} since the resulting global config directory would be too long.", msg);

        callback(global_config);

        Ok(())
    }

    /// Loads a configuration from a file. On success it returns a [`Config`] object otherwise a
    /// [`ConfigCreationError`] describing the failure.
    pub fn from_file(config_file: &FilePath) -> Result<Config, ConfigCreationError> {
        let msg = "Failed to create config";
        let mut new_config = Self::default();

        let file = match FileBuilder::new(config_file).open_existing(AccessMode::Read) {
            Ok(file) => file,
            Err(FileOpenError::InsufficientPermissions) => {
                fail!(from new_config,
                      with ConfigCreationError::InsufficientPermissions,
                      "{} since the config file \"{}\" could not be opened due to insufficient permissions.",
                      msg, config_file);
            }
            Err(FileOpenError::FileDoesNotExist) => {
                fail!(from new_config,
                      with ConfigCreationError::ConfigFileDoesNotExist,
                      "{} since the config file \"{}\" does not exist.",
                      msg, config_file);
            }
            Err(e) => {
                fail!(from new_config,
                      with ConfigCreationError::UnableToOpenConfigFile,
                      "{} since the config file \"{}\" could not be open due to an internal error ({:?}).",
                      msg, config_file, e);
            }
        };

        let mut contents = String::new();
        fail!(from new_config, when file.read_to_string(&mut contents),
                with ConfigCreationError::FailedToReadConfigFileContents,
                "{} since the config file contents could not be read.", msg);

        match toml::from_str(&contents) {
            Ok(v) => new_config = v,
            Err(e) => {
                fail!(from new_config, with ConfigCreationError::UnableToDeserializeContents,
                                "{} since the contents could not be deserialized ({}).", msg, e);
            }
        }

        trace!(from new_config, "Loaded.");
        Ok(new_config)
    }

    /// Sets up the global configuration from a file. If the global configuration was already setup
    /// it will print a warning and does not load the file. It returns the [`Config`] when the file
    /// could be successfully loaded otherwise a [`ConfigCreationError`] describing the error.
    pub fn setup_global_config_from_file(
        config_file: &FilePath,
    ) -> Result<&'static Config, ConfigCreationError> {
        if ICEORYX2_CONFIG.is_initialized() {
            return Ok(ICEORYX2_CONFIG.get());
        }

        if !ICEORYX2_CONFIG.set_value(Config::from_file(config_file)?) {
            warn!(
                from ICEORYX2_CONFIG.get(),
                "Configuration already loaded and set up, cannot load another one. This may happen when this function is called from multiple threads."
            );
            return Ok(ICEORYX2_CONFIG.get());
        }

        trace!(from ICEORYX2_CONFIG.get(), "Set as global config.");
        Ok(ICEORYX2_CONFIG.get())
    }

    /// Returns the global configuration. If the global configuration was not yet loaded it will
    /// load a default config by looking it up in the system. First it checks if a project local config file
    /// exists, then if a config file in the user directory exist and then if a global config file exist. If
    /// [`Config::setup_global_config_from_file()`]
    /// is called after this function was called, no file will be loaded since the global default
    /// config was already populated.
    pub fn global_config() -> &'static Config {
        let origin = "Config::global_config()";
        if !ICEORYX2_CONFIG.is_initialized() {
            let mut is_config_file_set = false;
            if let Err(e) = Self::iterate_over_config_files(|config_file_path| {
                match Config::setup_global_config_from_file(&config_file_path) {
                    Ok(_) => {
                        is_config_file_set = true;
                        CallbackProgression::Stop
                    }
                    Err(ConfigCreationError::ConfigFileDoesNotExist) => {
                        CallbackProgression::Continue
                    }
                    Err(e) => {
                        fatal_panic!(from origin,
                            "Config file found \"{}\" but a failure occurred ({:?}) while reading the content.",
                            config_file_path, e);
                    }
                }
            }) {
                fatal_panic!(from origin,
                    "A failure occurred ({:?}) while looking up the available config files.", e);
            }

            if !is_config_file_set {
                warn!(from origin,
                    "No config file was loaded, a config with default values will be used.");
                ICEORYX2_CONFIG.set_value(Config::default());
            }
        }

        ICEORYX2_CONFIG.get()
    }
}
