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

#![warn(missing_docs)]
#![doc(issue_tracker_base_url = "https://github.com/mydriatech/microfefind/issues/")]

//! # Micro front end discovery on Kubernetes.
//!
//! Enable discovery of micro front ends via labeled Kubernetes `Ingress`
//! declarations.
//!

pub mod conf;
mod ingress_monitor;
mod kubers_util;
mod rest_api;
mod time;

use std::process::ExitCode;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};

use crate::conf::AppConfig;
use crate::ingress_monitor::IngressMonitor;

/// Application entry point.
fn main() -> ExitCode {
    if let Err(e) = init_logger() {
        log::error!("Failed to initialize configuration: {e:?}");
        return ExitCode::FAILURE;
    }
    let app_config = Arc::new(AppConfig::new());
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(app_config.limits.available_parallelism())
        .build()
        .unwrap()
        .block_on(run_async(app_config))
}

/// Initialize the logging system and apply filters.
fn init_logger() -> Result<(), log::SetLoggerError> {
    let env_prefex = AppConfig::read_app_name_lowercase().to_uppercase();
    env_logger::builder()
        // Set default log level
        .filter_level(log::LevelFilter::Debug)
        // Customize logging for dependencies
        .filter(Some("actix_server"), log::LevelFilter::Warn)
        .filter(Some("rustls::client"), log::LevelFilter::Info)
        .filter(Some("rustls::common_state"), log::LevelFilter::Info)
        .filter(Some("hyper_util::client"), log::LevelFilter::Info)
        .filter(Some("kube_client::client"), log::LevelFilter::Info)
        .filter(Some("tower::buffer::worker"), log::LevelFilter::Info)
        //.write_style(env_logger::fmt::WriteStyle::Never)
        .write_style(env_logger::fmt::WriteStyle::Auto)
        .target(env_logger::fmt::Target::Stdout)
        .is_test(false)
        .parse_env(
            env_logger::Env::new()
                .filter(env_prefex.to_owned() + "_LOG_LEVEL")
                .write_style(env_prefex.to_owned() + "_LOG_STYLE"),
        )
        .try_init()
}

/// Async code entry point.
async fn run_async(app_config: Arc<AppConfig>) -> ExitCode {
    // Make a quick check that we have a k8s context that we can use.
    let client_result = kube::Client::try_default().await;
    match client_result {
        Ok(client) => {
            let info = client.apiserver_version().await.unwrap();
            log::info!("Kubernetes API version: {info:?}");
        }
        Err(e) => {
            log::error!("Failed to access Kubernetes API. Is this container deployed? {e:?}");
            return ExitCode::FAILURE;
        }
    }
    let ingress_monitor = IngressMonitor::new(Arc::clone(&app_config));
    let ingress_monitor_api_future =
        rest_api::run_http_server(app_config, Arc::clone(&ingress_monitor));
    let signals_future = block_until_signaled();
    tokio::select! {
        _ = ingress_monitor_api_future => {
            log::trace!("ingress_monitor_api_future finished");
        },
        _ = signals_future => {
            log::trace!("signals_future finished");
        },
    };
    ExitCode::SUCCESS
}

/// Block until SIGTERM or SIGINT is recieved.
async fn block_until_signaled() {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {
            log::debug!("SIGTERM recieved.")
        },
        _ = sigint.recv() => {
            log::debug!("SIGINT recieved.")
        },
    };
}
