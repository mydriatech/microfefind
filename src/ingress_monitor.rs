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

//! Monitor configured namespaces in Kubernetes for labeled `Ingress`es.

mod ingress_host_path;

use crossbeam_skiplist::SkipMap;
use futures::TryStreamExt;
use k8s_openapi::api::networking::v1::Ingress;
use kube::api::ListParams;
use kube::runtime::watcher::Config;
use kube::Api;
use kube::ResourceExt;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::conf::AppConfig;

pub use self::ingress_host_path::IngressHostPath;

/**
Object instance monitors (watches) configured namespaces in Kubernetes for
`Ingress`es with labels matching configured values.

This object maintains a full list of relevant hostname + path combinations and
also owns monitoring of related `Service`s and `Pod`s.
 */
pub struct IngressMonitor {
    /// Reference to the application's configuration.
    app_config: Arc<AppConfig>,
    /// Thread safe boolean used to indicate application readyness.
    health_ready: AtomicBool,
    /// Map of hostname + path combinations and the full meta-data object.
    monitored_ingress_host_paths: SkipMap<String, Arc<IngressHostPath>>,
}

impl IngressMonitor {
    /// Return a new instance.
    pub fn new(app_config: Arc<AppConfig>) -> Arc<Self> {
        Arc::new(Self {
            app_config,
            health_ready: AtomicBool::new(false),
            monitored_ingress_host_paths: SkipMap::new(),
        })
        .start_background_monitoring()
    }

    /// Return true if the [IngressMonitor] has started.
    pub fn is_health_started(self: &Arc<Self>) -> bool {
        self.health_ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Return true if the [IngressMonitor] is ready to serve requests.
    pub fn is_health_ready(self: &Arc<Self>) -> bool {
        self.health_ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    /**
       Return true if the [IngressMonitor] is still able to serve relevant data.

       *NOTE: This always returns `true`, even if the application is locked out
       of one of the configured namespaces to prevent a single µFE namespace
       owner to DoS the entire application.*
    */
    pub fn is_health_live(self: &Arc<Self>) -> bool {
        true
    }

    /// Start background monitoring of all configured namespaces
    fn start_background_monitoring(self: Arc<Self>) -> Arc<Self> {
        let namespaces = self.app_config.ingress.namespaces();
        if namespaces.is_empty() {
            let self_clone = Arc::clone(&self);
            tokio::spawn(async move { self_clone.watch_ingresses(None).await });
        } else {
            for namespace in namespaces {
                let self_clone = Arc::clone(&self);
                tokio::spawn(async move {
                    self_clone
                        .watch_ingresses(Some(namespace.to_string()))
                        .await
                });
            }
        }
        self
    }

    /**
      Watch all `Ingress` objects for changes and load all pre-existing
      `Ingress`es in the namespace.
    */
    async fn watch_ingresses(self: &Arc<Self>, namespace: Option<String>) {
        let label_selector = &self.app_config.ingress.match_labels();
        let client = kube::Client::try_default().await.unwrap();
        let namespace = namespace.unwrap_or(client.default_namespace().to_owned());
        // Prepare to watch for Ingress updates
        let stream = kube::runtime::watcher(
            Api::<Ingress>::namespaced(client.clone(), &namespace),
            Config::default().labels(label_selector),
        );
        // Process any already existing Ingress
        let api = &Api::<Ingress>::namespaced(client.clone(), &namespace);
        let lp = &ListParams::default().labels(label_selector);
        let self_clone = &self.clone();
        let namespace = &namespace.to_owned();
        match api.list(lp).await {
            Ok(object_list) => {
                for ingress in object_list {
                    self_clone
                        .update_ingress_host_paths(&Arc::new(ingress), namespace)
                        .await;
                }
                self.health_ready
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Err(e) => {
                log::warn!("Canceling monitoring of namespace '{namespace}' due to error: {e:?}");
                return;
            }
        }
        // Watch for Ingress updates
        stream
            .try_for_each(|event| async move {
                match event {
                    kube::runtime::watcher::Event::Deleted(ingress) => {
                        // Ingress was deleted, so remove all host paths
                        self_clone.remove_ingress_host_paths(&Arc::new(ingress), namespace);
                    }
                    kube::runtime::watcher::Event::Applied(ingress) => {
                        //log::info!("MODIFIED ingress: {:?}", ingress);
                        // Ingress was modified, so check if labels still match, remove otherwise
                        if let Ok(object_list) = api.list_metadata(lp).await {
                            let still_present = object_list
                                .into_iter()
                                .any(|object| ingress.metadata.name == object.metadata.name);
                            if still_present {
                                self_clone
                                    .update_ingress_host_paths(&Arc::new(ingress), namespace)
                                    .await;
                            } else {
                                log::info!(
                                    "ingress.metadata.labels change and no longer matches: {:?}",
                                    ingress.metadata.labels
                                );
                                // Nuke it
                                self_clone.remove_ingress_host_paths(&Arc::new(ingress), namespace);
                            }
                        } else {
                            // Just use any error, just make sure that we bail out of the stream
                            return Err(kube::runtime::watcher::Error::NoResourceVersion);
                        }
                    }
                    kube::runtime::watcher::Event::Restarted(_) => {
                        log::debug!("Ingress restarted");
                    }
                }
                Ok(())
            })
            .await
            .map_err(|e| {
                log::warn!("Canceling monitoring of namespace '{namespace}' due to error: {e:?}");
            })
            .ok();
    }

    /// Remove [IngressHostPath] from local cache.
    fn remove_ingress_host_paths(self: &Arc<Self>, ingress: &Arc<Ingress>, namespace: &str) {
        let ingress_rules = ingress.spec.as_ref().unwrap().rules.as_ref().unwrap();
        for ingress_rule in ingress_rules {
            let host = ingress_rule.host.as_ref().unwrap();
            for http_ingress_path in &ingress_rule.http.as_ref().unwrap().paths {
                let path = http_ingress_path.path.as_ref().unwrap();
                self.monitored_ingress_host_paths
                    .remove(&IngressHostPath::identifier(host, path));
                log::info!("Ingress path '{host}{path}' in 'ns/{namespace}' was deleted.");
            }
        }
    }

    /// Add or update [IngressHostPath] in local cache.
    async fn update_ingress_host_paths(self: &Arc<Self>, ingress: &Arc<Ingress>, namespace: &str) {
        let tag_prefix = self.app_config.ingress.annotation_prefix();
        let ingress_rules = ingress.spec.as_ref().unwrap().rules.as_ref().unwrap();
        for ingress_rule in ingress_rules {
            let host = ingress_rule.host.as_ref().unwrap();
            for http_ingress_path in &ingress_rule.http.as_ref().unwrap().paths {
                let path = http_ingress_path.path.as_ref().unwrap();
                let service_name = &http_ingress_path.backend.service.as_ref().unwrap().name;
                let key = IngressHostPath::identifier(host, path);
                if !self.monitored_ingress_host_paths.contains_key(&key) {
                    log::info!("New labeled Ingress path '{host}{path}' in 'ns/{namespace}' ->  'svc/{service_name}'");
                    let value = IngressHostPath::new(host, path, namespace, service_name).await;
                    self.monitored_ingress_host_paths
                        .insert(key.to_owned(), value);
                }
                let entry = self.monitored_ingress_host_paths.get(&key).unwrap();
                let ingress_host_path = entry.value();
                // Update backend service (if needed)
                ingress_host_path.service_name_update(service_name).await;
                let annotations: SkipMap<String, String> = ingress
                    .annotations()
                    .iter()
                    .filter_map(|(annotation_key, annotation_value)| {
                        if annotation_key.starts_with(&tag_prefix) {
                            Some((
                                annotation_key.replacen(&tag_prefix, "", 1),
                                annotation_value.to_owned(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                // Update annotations (if needed)
                ingress_host_path.annotations_update(&annotations);
            }
        }
    }

    /// Return all known [IngressHostPath]s from local cache.
    pub fn get_all(self: &Arc<Self>) -> Vec<Arc<IngressHostPath>> {
        self.monitored_ingress_host_paths
            .iter()
            .map(|entry| Arc::clone(entry.value()))
            .collect()
    }
}
