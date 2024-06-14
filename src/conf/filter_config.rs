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

//! Parsing of configuration for detection of labeled Kubernetes `Ingress`es.

use config::builder::BuilderState;
use config::ConfigBuilder;
use serde::{Deserialize, Serialize};

use super::AppConfigDefaults;

/// Configuration for detection of labeled Kubernetes `Ingress`es.
#[derive(Debug, Deserialize, Serialize)]
pub struct IngressFilterConfig {
    /// Comma separated list of `key=value` labels to match
    labels: String,
    /// Prefix for `Ingress` annotations that will be exposed to API clients.
    annotationprefix: String,
    /// Comma separated list of namespaces. None to use context namespace.
    namespaces: Option<String>,
}

impl AppConfigDefaults for IngressFilterConfig {
    /// Provide defaults for this part of the configuration
    fn set_defaults<T: BuilderState>(
        config_builder: ConfigBuilder<T>,
        prefix: &str,
    ) -> ConfigBuilder<T> {
        config_builder
            .set_default(prefix.to_string() + "." + "labels", "microfe=true")
            .unwrap()
            .set_default(prefix.to_string() + "." + "annotationprefix", "microfe/")
            .unwrap()
            .set_default(prefix.to_string() + "." + "namespaces", "")
            .unwrap()
    }
}

impl IngressFilterConfig {
    /// Comma separated list of `key=value` labels to match
    pub fn match_labels(&self) -> String {
        self.labels.clone()
    }

    /// Prefix for `Ingress` annotations that will be exposed to API clients (without the `prefix/`).
    pub fn annotation_prefix(&self) -> String {
        self.annotationprefix.clone()
    }

    /// Comma separated list of namespaces. Empty to use context namespace.
    pub fn namespaces(&self) -> Vec<String> {
        let mut ret = Vec::new();
        if let Some(namespaces) = &self.namespaces {
            if !namespaces.is_empty() {
                ret = namespaces
                    .split(',')
                    .map(|x| x.trim().to_string())
                    .collect();
            }
        }
        ret
    }
}
