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

#[derive(Clone)]
enum AllowedDomainOverride {
    All,
    Limited(Vec<String>),
}

async fn handle_get(query: Query<PsnQuery>, Extension(client): Extension<SoapClient>) -> Response {
    create_response(client, &query.psn, None, None).await
}

async fn handle_get_with_domain(
    query: Query<PsnDomainQuery>,
    Extension(client): Extension<SoapClient>,
    Extension(allowed_domain_override): Extension<AllowedDomainOverride>,
) -> Response {
    create_response(
        client,
        &query.psn,
        query.domain.clone(),
        Some(allowed_domain_override),
    )
    .await
}

#[allow(clippy::expect_used)]
pub(crate) async fn create_response(
    client: SoapClient,
    psn: &str,
    domain: Option<String>,
    allowed_domain_override: Option<AllowedDomainOverride>,
) -> Response {
    if let Some(domain) = &domain
        && let Some(AllowedDomainOverride::Limited(allowed_domains)) = allowed_domain_override
        && !allowed_domains.contains(domain)
    {
        log::error!("Access to domain '{domain}' forbidden");
        return Response::builder()
            .status(403)
            .body("Forbidden".into())
            .expect("valid response expected");
    }

    match client.get_value_for(psn, domain).await {
        Ok(response) => Response::new(response.into()),
        Err(SoapClientError::Fault(msg)) => {
            log::error!("SOAP fault: {msg}");
            Response::builder()
                .status(400)
                .body(msg.into())
                .expect("valid response expected")
        }
        Err(SoapClientError::Error(msg)) => {
            log::error!("SOAP error: {msg}");
            Response::builder()
                .status(500)
                .body(msg.into())
                .expect("valid response expected")
        }
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

    let allowed_domain_override = if config.allow_domain_override {
        AllowedDomainOverride::All
    } else {
        let mut allowed_domains = vec![config.gpas_domain_name.clone()];
        allowed_domains.append(&mut config.allowed_domains.clone());
        AllowedDomainOverride::Limited(allowed_domains)
    };

    log::info!(
        "Starting psn-to-pid v{} with PID {}",
        env!("CARGO_PKG_VERSION"),
        std::process::id()
    );
    log::info!("Default domain: '{}'", config.gpas_domain_name);
    log::info!(
        "Allow domain override: '{}'",
        config.allow_domain_override || !config.allowed_domains.is_empty()
    );
    if !config.allowed_domains.is_empty() {
        log::info!(
            "Domain override limited to: {}",
            config
                .allowed_domains
                .iter()
                .map(|d| format!("'{d}'"))
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    log::info!(
        "Accept invalid certificates: '{}'",
        config.accept_invalid_certs
    );

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
            if config.allow_domain_override || !config.allowed_domains.is_empty() {
                get(handle_get_with_domain)
            } else {
                get(handle_get)
            },
        )
        .layer(Extension(client))
        .layer(Extension(allowed_domain_override))
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::soap::SoapClient;
    use axum::body::to_bytes;
    use httpmock::MockServer;

    fn make_client(base_url: String) -> SoapClient {
        SoapClient::new(base_url, "demo.study.demo".to_string(), None, None, false)
    }

    #[tokio::test]
    async fn create_response_should_return_http200_on_success() {
        let mock_server = MockServer::start();
        let _mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::POST);
            then.status(200).body(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope
    xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
    xmlns:psn="http://psn.ttp.ganimed.icmvc.emau.org/">
    <soapenv:Body>
        <psn:getValueForResponse>
            <value>1001000000011</value>
        </psn:getValueForResponse>
    </soapenv:Body>
</soapenv:Envelope>"#,
            );
        });

        let client = make_client(mock_server.base_url());
        let response = create_response(client, "demo_01234567890", None, None).await;

        assert_eq!(response.status(), 200);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body")
            .to_vec();
        assert_eq!(body, b"1001000000011");
    }

    #[tokio::test]
    async fn create_response_should_return_http400_on_soap_fault() {
        let mock_server = MockServer::start();
        let _mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::POST);
            then.status(200).body(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope
    xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
    xmlns:psn="http://psn.ttp.ganimed.icmvc.emau.org/">
    <soapenv:Body>
        <psn:Fault>
            <faultstring>invalid check digits for 'bad_psn'</faultstring>
        </psn:Fault>
    </soapenv:Body>
</soapenv:Envelope>"#,
            );
        });

        let client = make_client(mock_server.base_url());
        let response = create_response(client, "bad_psn", None, None).await;

        assert_eq!(response.status(), 400);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body")
            .to_vec();
        assert_eq!(body, b"invalid check digits for 'bad_psn'");
    }

    #[tokio::test]
    async fn create_response_should_return_http500_on_soap_error() {
        let mock_server = MockServer::start();
        let _mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::POST);
            then.status(500);
        });

        let client = make_client(mock_server.base_url());
        let response = create_response(client, "demo_01234567890", None, None).await;

        assert_eq!(response.status(), 500);
    }

    #[tokio::test]
    async fn create_response_should_return_http403_on_forbidden_domain() {
        let mock_server = MockServer::start();
        let client = make_client(mock_server.base_url());
        let allowed = AllowedDomainOverride::Limited(vec!["demo.study.allowed".to_string()]);
        let response = create_response(
            client,
            "demo_01234567890",
            Some("demo.study.forbidden".to_string()),
            Some(allowed),
        )
        .await;

        assert_eq!(response.status(), 403);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body")
            .to_vec();
        assert_eq!(body, b"Forbidden");
    }

    #[tokio::test]
    async fn create_response_should_allow_domain_in_allowed_list() {
        let mock_server = MockServer::start();
        let _mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::POST);
            then.status(200).body(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope
    xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
    xmlns:psn="http://psn.ttp.ganimed.icmvc.emau.org/">
    <soapenv:Body>
        <psn:getValueForResponse>
            <value>1001000000011</value>
        </psn:getValueForResponse>
    </soapenv:Body>
</soapenv:Envelope>"#,
            );
        });

        let client = make_client(mock_server.base_url());
        let allowed = AllowedDomainOverride::Limited(vec!["demo.study.other".to_string()]);
        let response = create_response(
            client,
            "demo_01234567890",
            Some("demo.study.other".to_string()),
            Some(allowed),
        )
        .await;

        assert_eq!(response.status(), 200);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body")
            .to_vec();
        assert_eq!(body, b"1001000000011");
    }

    #[tokio::test]
    async fn create_response_should_allow_all_domains_when_unrestricted() {
        let mock_server = MockServer::start();
        let _mock = mock_server.mock(|when, then| {
            when.method(httpmock::Method::POST);
            then.status(200).body(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope
    xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
    xmlns:psn="http://psn.ttp.ganimed.icmvc.emau.org/">
    <soapenv:Body>
        <psn:getValueForResponse>
            <value>1001000000011</value>
        </psn:getValueForResponse>
    </soapenv:Body>
</soapenv:Envelope>"#,
            );
        });

        let client = make_client(mock_server.base_url());
        let allowed = AllowedDomainOverride::All;
        let response = create_response(
            client,
            "demo_01234567890",
            Some("demo.study.demo".to_string()),
            Some(allowed),
        )
        .await;

        assert_eq!(response.status(), 200);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("failed to read body")
            .to_vec();
        assert_eq!(body, b"1001000000011");
    }
}
