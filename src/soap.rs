use reqwest::Client;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize, Debug)]
#[serde(rename = "Envelope")]
struct Envelope {
    #[serde(rename = "Body")]
    body: Body,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Body")]
struct Body {
    #[serde(rename = "getValueForResponse")]
    get_value_for_response: Option<GetValueForResponse>,

    #[serde(rename = "Fault")]
    fault: Option<Fault>,
}

#[derive(Deserialize, Debug)]
struct GetValueForResponse {
    value: String,
}

#[derive(Deserialize, Debug)]
struct Fault {
    faultstring: String,
}

#[derive(Debug)]
pub enum SoapClientError {
    Fault(String),
    Error(String),
}

#[derive(Clone)]
pub struct SoapClient {
    gpas_url: String,
    client: Client,
    domain: String,
    username: String,
    password: Option<String>,
}

impl SoapClient {
    #[allow(clippy::expect_used)]
    pub fn new(
        gpas_url: String,
        domain: String,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            "Content-Type",
            "text/xml".parse().expect("valid header value expected"),
        );

        Self {
            gpas_url,
            client: Client::builder()
                .default_headers(default_headers)
                .connect_timeout(Duration::from_secs(10))
                .user_agent("mv64e-vnr-to-pid")
                .build()
                .expect("client created"),
            domain,
            username: username.unwrap_or_default(),
            password,
        }
    }

    pub async fn get_value_for(&self, pseudonym: &str) -> Result<String, SoapClientError> {
        let response_text = self
            .client
            .post(&self.gpas_url)
            .body(get_value_for_request_body(pseudonym, &self.domain))
            .basic_auth(self.username.clone(), self.password.clone())
            .send()
            .await
            .map_err(|e| SoapClientError::Error(e.to_string()))?
            .text()
            .await
            .map_err(|e| SoapClientError::Error(e.to_string()))?;

        let response_value = quick_xml::de::from_str::<Envelope>(&response_text)
            .map_err(|e| SoapClientError::Error(e.to_string()))?;

        if let Some(fault) = response_value.body.fault {
            return Err(SoapClientError::Fault(fault.faultstring));
        }

        if let Some(get_value_for_response) = response_value.body.get_value_for_response {
            return Ok(get_value_for_response.value);
        }

        Err(SoapClientError::Error("Unexpected response".to_string()))
    }
}

fn get_value_for_request_body(pseudonym: &str, domain: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<soapenv:Envelope
    xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/"
    xmlns:psn="http://psn.ttp.ganimed.icmvc.emau.org/">

    <soapenv:Header />

    <soapenv:Body>
        <psn:getValueFor>
            <psn>{pseudonym}</psn>
            <domainName>{domain}</domainName>
        </psn:getValueFor>
    </soapenv:Body>
</soapenv:Envelope>"#
    )
}
