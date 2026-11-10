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
    create_response(client, &query.psn, None).await
}

#[allow(clippy::expect_used)]
async fn handle_get_with_domain(
    query: Query<PsnDomainQuery>,
    Extension(client): Extension<SoapClient>,
) -> Response {
    create_response(client, &query.psn, query.domain.clone()).await
}

async fn create_response(client: SoapClient, psn: &str, domain: Option<String>) -> Response {
    match client.get_value_for(psn, domain).await {
        Ok(response) => Response::new(response.into()),
        Err(SoapClientError::Fault(msg)) => {
            log::error!("SOAP fault: {}", msg);
            Response::builder()
                .status(400)
                .body(msg.into())
                .expect("valid response expected")
        },
        Err(SoapClientError::Error(msg)) => {
            log::error!("SOAP error: {}", msg);
            Response::builder()
                .status(500)
                .body(msg.into())
                .expect("valid response expected")
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    println!(
        r"                       _                    _     _
 _ __  ___ _ __       | |_ ___        _ __ (_) __| |
| '_ \/ __| '_ \ _____| __/ _ \ _____| '_ \| |/ _` |
| |_) \__ \ | | |_____| || (_) |_____| |_) | | (_| |
| .__/|___/_| |_|      \__\___/      | .__/|_|\__,_|
|_|                                  |_|

 :: psn-to-pid ::    (v{})
",
        env!("CARGO_PKG_VERSION")
    );

    #[cfg(debug_assertions)]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();
    }

    #[cfg(not(debug_assertions))]
    {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .init();
    }

    let config = config::Config::parse();

    log::info!("Starting psn-to-pid v{} with PID {}", env!("CARGO_PKG_VERSION"), std::process::id());
    log::info!("Default domain: '{}'", config.gpas_domain_name);
    log::info!("Allow domain override: '{}'", config.allow_domain_override);
    log::info!("Accept invalid certificates: '{}'", config.accept_invalid_certs);

    let client = SoapClient::new(
        config.gpas_soap_url,
        config.gpas_domain_name,
        config.gpas_username,
        config.gpas_password,
        config.accept_invalid_certs,
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
            log::info!("Application listening on '{}'", config.listen);
            if let Err(err) = axum::serve(listener, routes)
                .with_graceful_shutdown(shutdown_signal())
                .await
            {
                return Err(err.to_string());
            }
        }
        Err(err) => return Err(format!("Cannot listen on '{}': {}", config.listen, err)),
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
