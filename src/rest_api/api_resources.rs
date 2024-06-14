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

//! API resources

use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{get, Error, HttpResponse};
use futures::stream;
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

use crate::ingress_monitor::IngressHostPath;

use super::AppState;

/// HTTP response body object for the [get_all] resource.
#[derive(ToSchema, Serialize)]
struct IngressHostPathResponse {
    /// Combined hostname and path servied via a correctly labeled `Ingress`.
    host_path: String,
    /// Last update timestamp in milliseconds sinch Unix Epoch.
    updated: u64,
    /// Prefixed annotations of the serving `Ingress` (without the prefix part)
    annotations: HashMap<String, String>,
}

impl IngressHostPathResponse {
    /// Convert to a JSON serializable response object
    async fn from_ingress_host_path(source: Arc<IngressHostPath>) -> Self {
        Self {
            host_path: source.host_path(),
            updated: source.updated_millis().await,
            annotations: source.annotations_map(),
        }
    }
}

/// Return all currently known labeled micro front end entrypoints. See also [IngressHostPathResponse].
#[utoipa::path(
    responses(
        (status = 200, description = "Up", body = inline(IngressHostPathResponse), content_type = "application/json",),
    ),
)]
#[get("/all")]
pub async fn get_all(
    app_state: Data<AppState>,
    //req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let results: Vec<_> = stream::iter(app_state.ingress_monitor.get_all())
        .then(IngressHostPathResponse::from_ingress_host_path)
        .collect()
        .await;
    log::trace!(
        "GET /all -> body: {}",
        serde_json::to_string_pretty(&results).unwrap()
    );
    let response = HttpResponse::build(StatusCode::OK).json(results);
    Ok(response)
}
