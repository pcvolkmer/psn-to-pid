mod config;
mod soap;

use crate::soap::{SoapClient, SoapClientError};
use axum::extract::Query;
use axum::response::Response;
use axum::routing::get;
use axum::{Extension, Router};
use clap::Parser;
use serde::Deserialize;
use tower_http::trace::TraceLayer;

#[derive(Deserialize)]
struct PsnQuery {
    psn: String,
}

#[derive(Deserialize)]
struct PsnDomainQuery {
    psn: String,
    domain: Option<String>,
}

#[allow(clippy::expect_used)]
async fn handle_get(query: Query<PsnQuery>, Extension(client): Extension<SoapClient>) -> Response {
    match client.get_value_for(&query.psn, None).await {
        Ok(response) => Response::new(response.into()),
        Err(SoapClientError::Fault(msg)) => Response::builder()
            .status(400)
            .body(msg.into())
            .expect("valid response expected"),
        Err(SoapClientError::Error(msg)) => Response::builder()
            .status(500)
            .body(msg.into())
            .expect("valid response expected"),
    }
}

#[allow(clippy::expect_used)]
async fn handle_get_with_domain(
    query: Query<PsnDomainQuery>,
    Extension(client): Extension<SoapClient>,
) -> Response {
    match client.get_value_for(&query.psn, query.domain.clone()).await {
        Ok(response) => Response::new(response.into()),
        Err(SoapClientError::Fault(msg)) => Response::builder()
            .status(400)
            .body(msg.into())
            .expect("valid response expected"),
        Err(SoapClientError::Error(msg)) => Response::builder()
            .status(500)
            .body(msg.into())
            .expect("valid response expected"),
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let config = config::Config::parse();

    let client = SoapClient::new(
        config.gpas_soap_url,
        config.gpas_domain_name,
        config.gpas_username,
        config.gpas_password,
    );

    let routes = Router::new()
        // POST requests
        .route(
            "/",
            if config.allow_domain_override {
                get(handle_get_with_domain)
            } else {
                get(handle_get)
            },
        )
        .layer(Extension(client))
        .layer(TraceLayer::new_for_http());

    match tokio::net::TcpListener::bind(&config.listen).await {
        Ok(listener) => {
            log::info!("Starting application listening on '{}'", config.listen);
            if let Err(err) = axum::serve(listener, routes)
                .with_graceful_shutdown(shutdown_signal())
                .await
            {
                return Err(err.to_string());
            }
        }
        Err(err) => return Err(format!("Cannot listening on '{}': {}", config.listen, err)),
    }

    Ok(())
}

#[allow(clippy::expect_used)]
async fn shutdown_signal() {
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    terminate.await;
}
