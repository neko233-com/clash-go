use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::time::Duration;

#[derive(Clone)]
pub struct ControllerClient {
    base_url: String,
    secret: Option<String>,
    client: Client,
}

#[derive(Clone, Debug, Default)]
pub struct EngineSnapshot {
    pub version: String,
    pub mode: String,
    pub mixed_port: Option<u16>,
    pub group_count: usize,
    pub connection_count: usize,
    pub download_total: u64,
    pub upload_total: u64,
    pub proxies: HashMap<String, ProxyInfo>,
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct ProxyInfo {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub now: Option<String>,
    #[serde(default)]
    pub all: Vec<String>,
}

#[derive(Deserialize)]
struct VersionResponse {
    version: String,
}

#[derive(Deserialize)]
struct ConfigResponse {
    #[serde(default)]
    mode: String,
    #[serde(rename = "mixed-port")]
    mixed_port: Option<u16>,
}

#[derive(Deserialize)]
struct ProxiesResponse {
    proxies: HashMap<String, ProxyInfo>,
}

#[derive(Deserialize)]
struct ConnectionsResponse {
    #[serde(rename = "downloadTotal", default)]
    download_total: u64,
    #[serde(rename = "uploadTotal", default)]
    upload_total: u64,
    #[serde(default)]
    connections: Vec<serde_json::Value>,
}

impl ControllerClient {
    pub fn from_environment() -> Result<Self, String> {
        let base_url = std::env::var("CLASH_GO_CONTROLLER")
            .unwrap_or_else(|_| "http://127.0.0.1:9090".to_string())
            .trim_end_matches('/')
            .to_string();
        let secret = std::env::var("CLASH_GO_SECRET")
            .ok()
            .filter(|value| !value.is_empty());
        let client = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|error| format!("Could not initialize the HTTP client: {error}"))?;

        Ok(Self {
            base_url,
            secret,
            client,
        })
    }

    pub fn snapshot(&self) -> Result<EngineSnapshot, String> {
        let version: VersionResponse = self.get("/version")?;
        let config: ConfigResponse = self.get("/configs")?;
        let proxies: ProxiesResponse = self.get("/proxies")?;
        let connections: ConnectionsResponse = self.get("/connections")?;
        let group_count = proxies
            .proxies
            .values()
            .filter(|proxy| !proxy.all.is_empty())
            .count();

        Ok(EngineSnapshot {
            version: version.version,
            mode: config.mode,
            mixed_port: config.mixed_port,
            group_count,
            connection_count: connections.connections.len(),
            download_total: connections.download_total,
            upload_total: connections.upload_total,
            proxies: proxies.proxies,
        })
    }

    pub fn select_proxy(&self, group: &str, node: &str) -> Result<(), String> {
        let group = urlencoding::encode(group);
        let url = format!("{}/proxies/{group}", self.base_url);
        let response = self
            .authorize(
                self.client
                    .put(url)
                    .json(&serde_json::json!({ "name": node })),
            )
            .send()
            .map_err(|error| format!("Could not select proxy: {error}"))?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Mihomo rejected the selection ({})",
                response.status()
            ))
        }
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self
            .authorize(self.client.get(format!("{}{path}", self.base_url)))
            .send()
            .map_err(|error| format!("Controller request failed: {error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Controller returned {} for {path}",
                response.status()
            ));
        }
        response
            .json()
            .map_err(|error| format!("Invalid response from {path}: {error}"))
    }

    fn authorize(&self, request: RequestBuilder) -> RequestBuilder {
        if let Some(secret) = &self.secret {
            request.header(AUTHORIZATION, format!("Bearer {secret}"))
        } else {
            request
        }
    }
}
