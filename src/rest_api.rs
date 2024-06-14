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

//! REST API server and resources.

mod api_resources;
mod health_resources;

use actix_web::http::header::ContentType;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use std::sync::Arc;
use utoipa::OpenApi;

use crate::conf::AppConfig;
use crate::ingress_monitor::IngressMonitor;

/// Number of parallel requests the can be served for each assigned CPU core.
const WORKERS_PER_CORE: usize = 256;

/// Shared state between requests.
#[derive(Clone)]
struct AppState {
    ingress_monitor: Arc<IngressMonitor>,
}

/// Run HTTP server.
pub async fn run_http_server(
    app_config: Arc<AppConfig>,
    ingress_monitor: Arc<IngressMonitor>,
) -> std::io::Result<()> {
    let app_config = Arc::clone(&app_config);
    let workers = app_config.limits.available_parallelism();
    let max_connections = WORKERS_PER_CORE * workers;
    log::info!(
        "API described by http://{}:{}/openapi.json allows {max_connections} concurrent.",
        &app_config.api.bind_address(),
        &app_config.api.bind_port(),
    );
    let app_state: AppState = AppState { ingress_monitor };
    let app_data = web::Data::<AppState>::new(app_state);

    HttpServer::new(move || {
        let scope = web::scope("/api/v1")
            .service(openapi)
            .service(api_resources::get_all);
        App::new()
            .app_data(app_data.clone())
            .service(web::redirect("/openapi", "/api/v1/openapi.json"))
            .service(web::redirect("/openapi.json", "/api/v1/openapi.json"))
            .service(scope)
            .service(health_resources::health)
            .service(health_resources::health_live)
            .service(health_resources::health_ready)
            .service(health_resources::health_started)
    })
    .workers(workers)
    .backlog(u32::try_from(max_connections / 2).unwrap()) // Default is 2048
    .worker_max_blocking_threads(max_connections)
    .max_connections(max_connections)
    .bind_auto_h2c((app_config.api.bind_address(), app_config.api.bind_port()))?
    .disable_signals()
    .shutdown_timeout(5) // Default 30
    .run()
    .await
}

/// Serve Open API documentation.
#[get("/openapi.json")]
async fn openapi() -> impl Responder {
    #[derive(OpenApi)]
    #[openapi(
        // Use Cargo.toml as source for the "info" section
        paths(
            api_resources::get_all,
            health_resources::health,
            health_resources::health_live,
            health_resources::health_ready,
            health_resources::health_started,
        )
    )]
    struct ApiDoc;
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(ApiDoc::openapi().to_pretty_json().unwrap())
}
