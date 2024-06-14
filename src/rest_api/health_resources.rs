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

//! Health check API resources.

use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::{get, HttpResponse, Responder};
use serde::Serialize;
use utoipa::ToSchema;

use super::AppState;

/** Helth check status definitions according to Eclipse MicroProfile Health 3.1.

See also

* [Eclipse MicroProfile Health `protocol-wireformat.asciidoc`
](https://github.com/eclipse/microprofile-health/blob/main/spec/src/main/asciidoc/protocol-wireformat.asciidoc).
 */
enum HealthStatus {
    /// Status is `UP` with HTTP status code 200.
    Up,
    /// Status is `DOWN` with HTTP status code 503.
    Down,
    /// Status is `UNDETERMINED` with HTTP status code 500.
    #[allow(dead_code)]
    Undetermined,
}
impl HealthStatus {
    /// See [Up](Self::Up), [Down](Self::Down) and [Undetermined](Self::Undetermined) for possible return values.
    fn http_status(&self) -> u16 {
        match *self {
            Self::Up => 200,
            Self::Down => 503,
            Self::Undetermined => 500,
        }
    }

    /// See [Up](Self::Up), [Down](Self::Down) and [Undetermined](Self::Undetermined) for possible return values.
    fn status(&self) -> String {
        match *self {
            Self::Up => "UP".to_owned(),
            Self::Down => "DOWN".to_owned(),
            Self::Undetermined => "UNDETERMINED".to_owned(),
        }
    }

    /// Return the status as [HttpResponse] with correct return code and JSON serialized body.
    fn as_response(&self) -> impl Responder {
        HttpResponse::build(StatusCode::from_u16(self.http_status()).unwrap()).json(
            HealthResponse {
                status: self.status(),
            },
        )
    }
}

/**
HTTP response body object for health requests. Only basic status is supported.
 */
#[derive(ToSchema, Serialize)]
struct HealthResponse {
    status: String,
}

/**
This endpoint returns the combined status of initialized, readiness and
liveness of a microservice.

It corresponds to the Kubernetes readiness probe.
 */
#[utoipa::path(
    responses(
        (status = 200, description = "Up", body = inline(HealthResponse), content_type = "application/json",),
        (status = 500, description = "Undetermined"),
        (status = 503, description = "Down"),
    ),
)]
#[get("/health")]
pub async fn health(app_state: Data<AppState>) -> impl Responder {
    // Combo: Liveness + Readiness + Startup
    if app_state.ingress_monitor.is_health_started()
        && app_state.ingress_monitor.is_health_ready()
        && app_state.ingress_monitor.is_health_live()
    {
        HealthStatus::Up.as_response()
    } else {
        HealthStatus::Down.as_response()
    }
}

/**
This endpoint returns the readiness of a microservice, or whether it is ready
to process requests.

It corresponds to the Kubernetes readiness probe.
 */
#[utoipa::path(
    responses(
        (status = 200, description = "Up", body = inline(HealthResponse), content_type = "application/json",),
        (status = 500, description = "Undetermined"),
        (status = 503, description = "Down"),
    ),
)]
#[get("/health/ready")]
pub async fn health_ready(app_state: Data<AppState>) -> impl Responder {
    if app_state.ingress_monitor.is_health_ready() {
        HealthStatus::Up.as_response()
    } else {
        HealthStatus::Down.as_response()
    }
}

/**
This endpoint returns the liveness of a microservice, or whether it encountered
a bug or deadlock. If this check fails, the microservice is not running and can
be stopped.

This endpoint corresponds to the Kubernetes liveness probe, which automatically
restarts the pod if the check fails.
 */
#[utoipa::path(
    responses(
        (status = 200, description = "Up", body = inline(HealthResponse), content_type = "application/json",),
        (status = 500, description = "Undetermined"),
        (status = 503, description = "Down"),
    ),
)]
#[get("/health/live")]
pub async fn health_live(app_state: Data<AppState>) -> impl Responder {
    if app_state.ingress_monitor.is_health_live() {
        HealthStatus::Up.as_response()
    } else {
        HealthStatus::Down.as_response()
    }
}

/**
In MicroProfile Health 3.1 and later, you can use this endpoint to determine
whether your deployed applications are initialized, according to criteria that
you define.

It corresponds to the Kubernetes startup probe.
 */
#[utoipa::path(
    responses(
        (status = 200, description = "Up", body = inline(HealthResponse), content_type = "application/json",),
        (status = 500, description = "Undetermined"),
        (status = 503, description = "Down"),
    ),
)]
#[get("/health/started")]
pub async fn health_started(app_state: Data<AppState>) -> impl Responder {
    if app_state.ingress_monitor.is_health_started() {
        HealthStatus::Up.as_response()
    } else {
        HealthStatus::Down.as_response()
    }
}
