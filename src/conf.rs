/*
    Copyright 2024 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Parsing of application configuration.

mod api_config;
mod filter_config;
mod limits_config;

use config::builder::BuilderState;
use config::{Config, ConfigBuilder, Environment, File};
use serde::{Deserialize, Serialize};

use self::api_config::ApiConfig;
use self::filter_config::IngressFilterConfig;
use self::limits_config::ResourceLimitsConfig;

/// Package name reported by Cargo at build time.
const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");
/// Package version reported by Cargo at build time.
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Static trait for tracking implementations.
trait AppConfigDefaults {
    fn set_defaults<T: BuilderState>(
        config_builder: ConfigBuilder<T>,
        prefix: &str,
    ) -> ConfigBuilder<T>;
}

/**
Application configration root.

The application name defaults to the Rust package name, but can be overridden
with the environment variable `APP_NAME`.

Configuration will be loaded from

1. the file `{application name}.json` in the current working directory.
2. environment variable overrides in the form
    `{APPLICATION_NAME}_MODULE_CONFIGKEYWITHOUTSPACES`
 */
#[derive(Debug, Deserialize, Serialize)]
pub struct AppConfig {
    /// Configuration of the exposed REST API.
    pub api: ApiConfig,
    /// Ingress detection and annotation filtering configuration.
    pub ingress: IngressFilterConfig,
    /// Resource detection and configuration overrides.
    pub limits: ResourceLimitsConfig,

    /// Lower case application name. Ignored when loading configuration.
    #[serde(skip_deserializing)]
    app_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl AppConfig {
    /**
       The application name defaults to the Rust package name, but can be overridden
       with the environment variable `APP_NAME`.
    */
    pub fn read_app_name_lowercase() -> String {
        std::env::var("APP_NAME")
            .map_err(|e| {
                log::debug!(
                    "Environment variable APP_NAME: {e:?} -> Default app name '{}' will be used.",
                    CARGO_PKG_NAME.to_owned()
                );
            })
            .ok()
            .map(|value| value.to_lowercase())
            .unwrap_or(CARGO_PKG_NAME.to_owned())
    }

    /// Lower case application name.
    pub fn app_name_lowercase(&self) -> &str {
        &self.app_name
    }

    /// SemVer application version derived fromt the Rust package version.
    pub fn app_version(&self) -> &'static str {
        CARGO_PKG_VERSION
    }

    /**
       Creates a new instance pre-populated with defaults, an optional
       configrations file and environment variable overrides.
    */
    pub fn new() -> Self {
        let app_name = Self::read_app_name_lowercase();
        let config_filename = app_name.to_owned() + ".json";
        let config_env_prefix = &app_name.to_uppercase();
        let mut config_builder = Config::builder();
        config_builder = ApiConfig::set_defaults(config_builder, "api");
        config_builder = IngressFilterConfig::set_defaults(config_builder, "ingressfilter");
        config_builder = ResourceLimitsConfig::set_defaults(config_builder, "limits");
        let conf_file = std::env::current_dir().unwrap().join(config_filename);
        if log::log_enabled!(log::Level::Debug) {
            log::debug!(
                "Will load '{}' configuration if present.",
                conf_file.display()
            );
        }
        let config = config_builder
            .add_source(File::with_name(conf_file.as_os_str().to_str().unwrap()).required(false))
            .add_source(
                Environment::with_prefix(config_env_prefix)
                    //.try_parsing(true)
                    .separator("_")
                    .list_separator(","),
            )
            .build()
            .unwrap();
        let mut app_config: AppConfig = config.try_deserialize().unwrap();
        app_config.app_name = app_name;
        if log::log_enabled!(log::Level::Debug) {
            log::info!(
                "Running with configuration: {}",
                serde_json::to_string(&app_config).unwrap()
            );
        }
        app_config
    }
}
