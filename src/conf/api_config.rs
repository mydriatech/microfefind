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

//! Parsing of configuration for the application's exposed REST API.

use config::builder::BuilderState;
use config::ConfigBuilder;
use serde::{Deserialize, Serialize};

use super::AppConfigDefaults;

/// Configuration for the application's exposed REST API.
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiConfig {
    /// IP address to bind to.
    address: String,
    /// IP port to bind to.
    port: u16,
}

impl AppConfigDefaults for ApiConfig {
    /// Provide defaults for this part of the configuration
    fn set_defaults<T: BuilderState>(
        config_builder: ConfigBuilder<T>,
        prefix: &str,
    ) -> ConfigBuilder<T> {
        config_builder
            .set_default(prefix.to_string() + "." + "address", "0.0.0.0")
            .unwrap()
            .set_default(prefix.to_string() + "." + "port", "8083")
            .unwrap()
    }
}

impl ApiConfig {
    /// IP address to bind to. Defaults to the IPv4 address `0.0.0.0`.
    pub fn bind_address(&self) -> String {
        self.address.clone()
    }

    /// IP port to bind to. Defaults to the unpriviliged port `8083`.
    pub fn bind_port(&self) -> u16 {
        self.port
    }
}
