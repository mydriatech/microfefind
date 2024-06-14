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

//! Utilities to simplify use of kube.rs.

use core::hash::Hash;
use futures::stream;
use futures::TryStreamExt;
use kube::runtime::reflector;
use kube::runtime::reflector::Lookup;
use kube::runtime::watcher;
use kube::runtime::watcher::Config;
use kube::runtime::WatchStreamExt;
use kube::Api;
use serde::de::DeserializeOwned;
use std::sync::Arc;

/// Return a stream of existing and future Kubernet resources of type `K`.
pub async fn reflector_stream<K>(
    api: Api<K>,
    watcher_config: Config,
) -> impl futures_util::Stream<Item = Result<Arc<K>, kube::runtime::watcher::Error>>
where
    K: std::fmt::Debug + DeserializeOwned + kube::Resource + Clone + std::marker::Send + 'static,
    <K as kube::Resource>::DynamicType: std::default::Default,
    <K as Lookup>::DynamicType: Eq + Hash + Clone,
{
    let (reader, writer) = reflector::store();
    let reflector = reflector(writer, watcher(api, watcher_config));
    let reflector_stream = reflector
        .applied_objects()
        .and_then(|x| async { Ok(Arc::new(x)) });
    let store_stream = stream::iter({
        /* This hangs for unknown reasons.
        if let Err(e) = reader.wait_until_ready().await {
            log::warn!("Unable to read resources in namespace '{namespace}' : {e:?}");
        }
        */
        reader.state().into_iter().map(Ok).collect::<Vec<_>>()
    });
    stream::select(reflector_stream, store_stream)
}
