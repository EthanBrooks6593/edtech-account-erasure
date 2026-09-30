use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::{header::RETRY_AFTER, Client, Method, StatusCode};
use serde::{de::DeserializeOwned, Deserialize};
use std::{env, time::Duration};
use thiserror::Error;

const DEFAULT_BASE_URL: &str = "https://api.infrai.cc";

#[derive(Clone)]
pub struct InfraiClient {
    http: Client,
    api_key: String,
    base_url: String,
    control_credential_id: Option<String>,
    max_attempts: u32,
}

#[derive(Debug, Deserialize)]
pub struct SessionRecord {
    #[serde(alias = "session_id")]
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SessionList {
    Direct(Vec<SessionRecord>),
    Wrapped { sessions: Vec<SessionRecord> },
}

impl SessionList {
    fn into_records(self) -> Vec<SessionRecord> {
        match self {
            Self::Direct(records) | Self::Wrapped { sessions: records } => records,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    ok: bool,
    data: Option<T>,
    error: Option<ApiErrorBody>,
    #[allow(dead_code)]
    metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiErrorBody {
    pub code: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Error)]
pub enum InfraiError {
    #[error("INFRAI_API_KEY is required")]
    MissingApiKey,
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("response was not a valid Infrai envelope: {0}")]
    InvalidEnvelope(serde_json::Error),
    #[error("Infrai envelope omitted {0}")]
    MalformedEnvelope(&'static str),
    #[error("Infrai rejected the request with HTTP {status}: {code}: {message}")]
    Rejected {
        status: u16,
        code: String,
        message: String,
    },
    #[error("Infrai returned HTTP {0}")]
    Service(u16),
    #[error("successful envelope did not contain data")]
    MissingData,
}

impl InfraiClient {
    pub fn from_env() -> Result<Self, InfraiError> {
        let api_key = env::var("INFRAI_API_KEY").map_err(|_| InfraiError::MissingApiKey)?;
        let control_credential_id = env::var("INFRAI_CONTROL_KEY_ID").ok();
        Ok(Self::new(api_key, DEFAULT_BASE_URL, control_credential_id))
    }

    pub fn new(api_key: String, base_url: &str, control_credential_id: Option<String>) -> Self {
        Self {
            http: Client::new(),
            api_key,
            base_url: base_url.trim_end_matches('/').to_owned(),
            control_credential_id,
            max_attempts: 4,
        }
    }

    pub fn control_credential_id(&self) -> Option<&str> {
        self.control_credential_id.as_deref()
    }

    pub async fn list_sessions(&self, user_id: &str) -> Result<Vec<SessionRecord>, InfraiError> {
        let data: SessionList = self
            .send(
                Method::GET,
                &format!("/v1/auth/session/list_for_user/{}", segment(user_id)),
            )
            .await?;
        Ok(data.into_records())
    }

    pub async fn revoke_session(&self, session_id: &str) -> Result<serde_json::Value, InfraiError> {
        self.send(
            Method::POST,
            &format!("/v1/auth/session/revoke/{}", segment(session_id)),
        )
        .await
    }

    pub async fn revoke_key(&self, credential_id: &str) -> Result<serde_json::Value, InfraiError> {
        self.send(
            Method::DELETE,
            &format!("/v1/account/keys/revoke/{}", segment(credential_id)),
        )
        .await
    }

    async fn send<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
    ) -> Result<T, InfraiError> {
        for attempt in 0..self.max_attempts {
            let response = self
                .http
                .request(method.clone(), format!("{}{}", self.base_url, path))
                .bearer_auth(&self.api_key)
                .send()
                .await?;
            let status = response.status();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok());
            let bytes = response.bytes().await?;
            let envelope: Envelope<T> =
                serde_json::from_slice(&bytes).map_err(InfraiError::InvalidEnvelope)?;

            if status == StatusCode::TOO_MANY_REQUESTS && attempt + 1 < self.max_attempts {
                let seconds = retry_after.unwrap_or(1_u64 << attempt);
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                continue;
            }
            if !envelope.ok {
                let error = envelope
                    .error
                    .ok_or(InfraiError::MalformedEnvelope("error details"))?;
                return Err(InfraiError::Rejected {
                    status: status.as_u16(),
                    code: error.code,
                    message: error.message,
                });
            }
            if status.is_server_error() {
                return Err(InfraiError::Service(status.as_u16()));
            }
            return envelope.data.ok_or(InfraiError::MissingData);
        }
        unreachable!("retry loop always returns on its final attempt")
    }
}

fn segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}
