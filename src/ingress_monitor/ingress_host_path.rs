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

//! Home of [IngressHostPath] and related `Service` and `Pod` monitoring.

mod service_monitor;

use crossbeam_skiplist::SkipMap;
use futures::lock::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use self::service_monitor::ServiceMonitor;

/**
   Representation of a hostname + path mapped by an `Ingress` to a `Service` and
   relevant meta-data.
*/
pub struct IngressHostPath {
    /// Last update timestamp in milliseconds sinch Unix Epoch.
    updated_millis: Arc<AtomicU64>,
    /// Hostname defined in `Ingress`.
    host: String,
    /// Path defined in `Ingress`.
    path: String,
    /// Prefixed `Ingress` annotations with the prefix removed.
    annotations: SkipMap<String, String>,
    /// Reference to object responsible for montitoring of mapped `Service`.
    service_monitor: Arc<Mutex<Option<Arc<ServiceMonitor>>>>,
}

impl IngressHostPath {
    /// Return a new instance.
    pub async fn new(host: &str, path: &str, namespace: &str, service_name: &str) -> Arc<Self> {
        let updated_millis = Arc::new(AtomicU64::new(0));
        Arc::new(Self {
            updated_millis: Arc::clone(&updated_millis),
            host: host.to_owned(),
            path: path.to_owned(),
            annotations: SkipMap::new(),
            service_monitor: Arc::new(Mutex::new(Some(
                ServiceMonitor::new(namespace, service_name, updated_millis).await,
            ))),
        })
    }

    /// Return the concatinated hostname and path.
    pub fn host_path(self: &Arc<Self>) -> String {
        Self::identifier(&self.host, &self.path)
    }

    /// Return the concatinated hostname and path.
    pub fn identifier(host: &str, path: &str) -> String {
        host.to_owned() + path
    }

    /**
      Last update of this `Ingress`, the `Service` mapped by the `Ingress` or
      change in ownership of any `Pod` backing the `Service`.
    */
    pub async fn updated_millis(self: &Arc<Self>) -> u64 {
        self.updated_millis.load(Ordering::Relaxed)
    }

    /// Prefixed `Ingress` annotations with the prefix removed.
    pub fn annotations_map(self: &Arc<Self>) -> HashMap<String, String> {
        HashMap::from_iter(
            self.annotations
                .iter()
                .map(|entry| (entry.key().to_owned(), entry.value().to_owned())),
        )
    }

    /**
      Invoked when `Ingress` has been modified to check if the mapped `Service` has
      changed.
    */
    pub async fn service_name_update(self: &Arc<Self>, service_name: &str) {
        let mutex = Arc::clone(&self.service_monitor);
        {
            let mut service_monitor_opt = mutex.lock().await;
            let service_monitor = service_monitor_opt.as_ref().unwrap();
            if service_monitor.service_name() != service_name {
                log::info!(
                    "Service for Ingress changes from '{}' to '{service_name}'.",
                    &service_monitor.service_name()
                );
                service_monitor.abort_background_tasks().await;
                let namespace = service_monitor.namespace().to_owned();
                service_monitor_opt.replace(
                    ServiceMonitor::new(&namespace, service_name, Arc::clone(&self.updated_millis))
                        .await,
                );
                self.updated_millis
                    .store(crate::time::now_as_millis(), Ordering::Relaxed);
            }
        }
    }

    /**
      Invoked when `Ingress` has been modified to check if prefixed
      annotations on the `Ingress` has changed.
    */
    pub fn annotations_update(self: &Arc<Self>, annotations: &SkipMap<String, String>) {
        let mut change = false;
        if annotations.len() != self.annotations.len() {
            change = true;
        } else {
            for entry in annotations.iter() {
                if let Some(old_entry) = self.annotations.get(entry.key()) {
                    if entry.value() != old_entry.value() {
                        change = true;
                    }
                } else {
                    change = true;
                }
            }
        }
        if change {
            log::info!(
                "Prefixed annotations for '{}' changed to {:?}.",
                self.host_path(),
                annotations
                    .iter()
                    .map(|entry| { entry.key().to_string() + "=" + entry.value() })
                    .collect::<Vec<_>>()
            );
            // TODO: Fix race condition here and avoid String creations
            self.annotations.clear();
            annotations.iter().for_each(|entry| {
                self.annotations
                    .insert(entry.key().to_owned(), entry.value().to_owned());
            });
            self.updated_millis
                .store(crate::time::now_as_millis(), Ordering::Relaxed);
        }
    }
}
