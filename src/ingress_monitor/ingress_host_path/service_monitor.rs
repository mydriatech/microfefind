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

//! Monitor a named Kubernetes `Service`.

mod pod_monitor;

use futures::lock::Mutex;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Service;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use self::pod_monitor::PodMonitor;

pub struct ServiceMonitor {
    /// Handle used to abort the background monitoring.
    abort_handle: Arc<Mutex<Option<tokio::task::AbortHandle>>>,
    /// Shared atomic counter used to communicate potential changes.
    updated_millis: Arc<AtomicU64>,
    /// The Kubernetes namespace to monitor.
    namespace: String,
    /// The name of the `Service` to monitor.
    service_name: String,
    /// Reference to object responsible for montitoring of labeled `Pod`s.
    pod_monitor: Arc<Mutex<Option<Arc<PodMonitor>>>>,
}

impl ServiceMonitor {
    /// Return a new instance.
    pub async fn new(
        namespace: &str,
        service_name: &str,
        updated_millis: Arc<AtomicU64>,
    ) -> Arc<Self> {
        Arc::new(Self {
            abort_handle: Arc::new(Mutex::new(None)),
            updated_millis,
            namespace: namespace.to_owned(),
            service_name: service_name.to_owned(),
            pod_monitor: Arc::new(Mutex::new(None)),
        })
        .start_background_tasks()
        .await
    }

    /// Return the `Service`'s name.
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Return the `Service`'s namespace.
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Start background monitoring of the named `Service`.
    async fn start_background_tasks(self: Arc<Self>) -> Arc<Self> {
        let self_clone = Arc::clone(&self);
        let join_handle = tokio::spawn(async move {
            let field_selector = "metadata.name=".to_string() + &self_clone.service_name;
            let client = kube::Client::try_default().await.unwrap();
            let k8s_resource_stream = crate::kubers_util::reflector_stream::<Service>(
                kube::Api::namespaced(client, &self_clone.namespace),
                kube::runtime::watcher::Config::default().fields(&field_selector),
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
        Arc::clone(&self.abort_handle)
            .lock()
            .await
            .replace(join_handle.abort_handle());
        self
    }

    /// Abort background monitoring of the named `Service`.
    pub async fn abort_background_tasks(self: &Arc<Self>) {
        if let Some(abort_handle) = Arc::clone(&self.abort_handle).lock().await.as_mut() {
            abort_handle.abort();
        }
        // Also abort the related monitoring of Pods
        let mutex = Arc::clone(&self.pod_monitor);
        {
            let pod_monitor_opt = mutex.lock().await;
            if let Some(pod_monitor) = pod_monitor_opt.as_ref() {
                pod_monitor.abort_background_tasks().await;
            }
        }
    }

    /**
      If the `Service` update also changed the selector labels, we need to
      update the `Pod` monitoring as well.
    */
    async fn handle_update(self: &Arc<Self>, service: &Arc<Service>) {
        let service_spec = service.as_ref().spec.as_ref().unwrap();
        let pod_selector = service_spec.selector.as_ref().unwrap();
        // Transform into a label_selector "key1=value1,key2=value2" etc
        let mut label_selector = String::new();
        for (i, (key, value)) in pod_selector.iter().enumerate() {
            label_selector.push_str(key);
            label_selector.push('=');
            label_selector.push_str(value);
            if i < pod_selector.len() - 1 {
                label_selector.push(',');
            }
        }
        // Check if current PodMonitor uses this label-selector
        let mut changed = true;
        let mutex = Arc::clone(&self.pod_monitor);
        {
            let mut pod_monitor_opt = mutex.lock().await;
            if let Some(pod_montor) = pod_monitor_opt.as_ref() {
                if pod_montor.clone().label_selector() == label_selector {
                    changed = false;
                }
            }
            if changed {
                let old_pod_monitor = pod_monitor_opt.insert(
                    PodMonitor::new(
                        &self.namespace,
                        &label_selector,
                        Arc::clone(&self.updated_millis),
                    )
                    .await,
                );
                old_pod_monitor.abort_background_tasks().await;
            }
        }
        if changed {
            log::info!("New service label_selector: '{label_selector}'.");
            self.updated_millis
                .store(crate::time::now_as_millis(), Ordering::Relaxed);
        }
    }
}
