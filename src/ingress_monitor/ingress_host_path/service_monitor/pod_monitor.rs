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

//! Monitor configured namespaces in Kubernetes for labeled `Pod`s.

use crossbeam_skiplist::SkipMap;
use futures::lock::Mutex;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::ListParams;
use kube::runtime::watcher::Config;
use kube::{Api, Client};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct PodMonitor {
    /// Handle used to abort the background monitoring.
    abort_handle: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
    /// Shared atomic counter used to communicate potential changes.
    updated_millis: Arc<AtomicU64>,
    /// The Kubernetes namespace to monitor.
    namespace: String,
    /// The lables to use when monitoring `Pod`s for updates.
    label_selector: String,
    /// Currently known owner references of `Pod`s.
    owner_references: SkipMap<String, u64>,
}

impl PodMonitor {
    /// Return a new instance.
    pub async fn new(
        namespace: &str,
        label_selector: &str,
        updated_millis: Arc<AtomicU64>,
    ) -> Arc<Self> {
        Arc::new(Self {
            abort_handle: Arc::new(Mutex::new(None)),
            updated_millis,
            namespace: namespace.to_owned(),
            label_selector: label_selector.to_owned(),
            owner_references: SkipMap::new(),
        })
        .start_background_tasks()
        .await
    }

    /// Return the current label selector as a comma separated `key=value` pairs.
    pub fn label_selector(self: &Arc<Self>) -> String {
        self.label_selector.to_owned()
    }

    /// Start background monitoring of the labeled `Pod`s.
    async fn start_background_tasks(self: Arc<Self>) -> Arc<Self> {
        let self_clone = Arc::clone(&self);
        tokio::spawn(async move {
            let client = Client::try_default().await.unwrap();
            let k8s_resource_stream = crate::kubers_util::reflector_stream::<Pod>(
                Api::namespaced(client, &self_clone.namespace),
                Config::default().labels(&self_clone.label_selector),
            )
            .await;
            let self_clone = &self_clone.clone();
            k8s_resource_stream
                .try_for_each(|resource| async move {
                    self_clone.handle_update(&resource).await;
                    Ok(())
                })
                .await
                .map_err(|e| {
                    log::warn!("Canceling monitoring of service due to error: {e:?}");
                })
                .ok();
        });
        let self_clone = Arc::clone(&self);
        let join_handle = tokio::spawn(async move {
            // TODO: Query all Pods from time to time and remove owners that are no longer relevant
            let client = kube::Client::try_default().await.unwrap();

            // Set timestamp of all current owners
            let now = crate::time::now_as_secs();
            let api = &Api::<Pod>::namespaced(client.clone(), &self_clone.namespace);
            let lp = &ListParams::default().labels(&self_clone.label_selector);
            let namespace = &self_clone.namespace.to_owned();
            match api.list(lp).await {
                Ok(object_list) => {
                    for pod in object_list {
                        let pod_metadata = &pod.metadata;
                        let pod_owner_reference = pod_metadata.owner_references.as_ref().unwrap();
                        // It would be an exception case if there are multiple owner refs, but API wont exclude it...
                        let owners_iter = pod_owner_reference.iter().map(|owner_reference| {
                            owner_reference.kind.to_owned() + "/" + &owner_reference.name
                        });
                        for owner in owners_iter {
                            if self_clone.owner_references.get(&owner).is_some() {
                                self_clone.owner_references.insert(owner.to_owned(), now);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Pod monitoring failed in namespace '{namespace}' due to error: {e:?}"
                    );
                    return;
                }
            }
            // Remove all owners that are older than now
            for entry in self_clone.owner_references.iter() {
                if entry.value() < &now {
                    self_clone.owner_references.remove(entry.key());
                    log::info!(
                        "Removing owner '{}' that is no longer referenced by any Pod.",
                        entry.key()
                    );
                }
            }
        });
        Arc::clone(&self.abort_handle)
            .lock()
            .await
            .replace(join_handle.abort_handle());
        self
    }

    /// Abort the background monitoring of the labeled `Pod`s.
    pub async fn abort_background_tasks(self: &Arc<Self>) {
        if let Some(abort_handle) = Arc::clone(&self.abort_handle).lock().await.as_mut() {
            abort_handle.abort();
        }
    }

    /**
      If the `Pod` update also introduced a previously unknown owner, we can
      deduct that there probably is a new `ReplicaSet` (caused by an updated
      `Deployment`), so `microfefind` clients need to be informed.
    */
    async fn handle_update(self: &Arc<Self>, pod: &Arc<Pod>) {
        let pod_phase = pod
            .as_ref()
            .status
            .as_ref()
            .unwrap()
            .phase
            .as_ref()
            .unwrap();
        let pod_metadata = &pod.as_ref().metadata;
        let pod_name = pod_metadata.name.as_ref().unwrap();
        log::trace!("pod/{pod_name} has pod.status.phase {pod_phase}");
        let pod_owner_reference = pod_metadata.owner_references.as_ref().unwrap();
        // It would be an exception case if there are multiple owner refs, but API wont exclude it...
        let owners_iter = pod_owner_reference
            .iter()
            .map(|owner_reference| owner_reference.kind.to_owned() + "/" + &owner_reference.name);
        let mut changed = false;
        for owner in owners_iter {
            self.owner_references
                .get_or_insert_with(owner.to_owned(), || {
                    log::info!("New owner '{owner}' detected for 'pod/{pod_name}'.");
                    changed = true;
                    // Update timestamp of when it was last seen to avoid garbage collection races
                    crate::time::now_as_secs()
                });
        }
        if changed {
            self.updated_millis
                .store(crate::time::now_as_millis(), Ordering::Relaxed);
        }
    }
}
