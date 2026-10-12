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
            .map_err(|error| {
                format!(
                    "Could not select proxy: {}",
                    self.sanitize_error(&error.to_string())
                )
            })?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "Mihomo rejected the selection ({})",
                response.status()
            ))
        }
    }

    pub fn redacted_endpoint(&self) -> String {
        url::Url::parse(&self.base_url)
            .map(|mut endpoint| {
                let _ = endpoint.set_username("");
                let _ = endpoint.set_password(None);
                endpoint.set_query(None);
                endpoint.set_fragment(None);
                endpoint.to_string()
            })
            .unwrap_or_else(|_| "<invalid-controller-url>".to_string())
    }

    pub fn sanitize_error(&self, error: &str) -> String {
        let mut sanitized = error.to_string();
        if let Some(secret) = &self.secret {
            sanitized = sanitized.replace(secret, "[REDACTED]");
        }
        if let Ok(endpoint) = url::Url::parse(&self.base_url) {
            let mut safe_endpoint = endpoint;
            let _ = safe_endpoint.set_username("");
            let _ = safe_endpoint.set_password(None);
            safe_endpoint.set_query(None);
            safe_endpoint.set_fragment(None);
            sanitized = sanitized.replace(&self.base_url, safe_endpoint.as_str());
        }
        redact_embedded_urls(&sanitized)
    }

    pub fn get_json(&self, path: &str) -> Result<serde_json::Value, String> {
        self.get(path)
    }

    pub fn proxy_groups(&self) -> Result<serde_json::Value, String> {
        let value: serde_json::Value = self.get("/proxies")?;
        let mut groups = serde_json::Map::new();
        if let Some(proxies) = value.get("proxies").and_then(serde_json::Value::as_object) {
            for (name, proxy) in proxies {
                if proxy
                    .get("all")
                    .and_then(serde_json::Value::as_array)
                    .is_some()
                {
                    groups.insert(name.clone(), proxy.clone());
                }
            }
        }
        Ok(serde_json::Value::Object(groups))
    }

    pub fn patch_config(&self, key: &str, value: serde_json::Value) -> Result<(), String> {
        let response = self
            .authorize(self.client.patch(format!("{}/configs", self.base_url)))
            .json(&serde_json::json!({ key: value }))
            .send()
            .map_err(|error| {
                format!(
                    "Could not update Mihomo config: {}",
                    self.sanitize_error(&error.to_string())
                )
            })?;
        self.ensure_success(response, "PATCH /configs")
    }

    pub fn reload_config(&self) -> Result<(), String> {
        let response = self
            .authorize(
                self.client
                    .put(format!("{}/configs?force=true", self.base_url)),
            )
            .json(&serde_json::json!({ "path": "", "payload": "" }))
            .send()
            .map_err(|error| {
                format!(
                    "Could not reload Mihomo config: {}",
                    self.sanitize_error(&error.to_string())
                )
            })?;
        self.ensure_success(response, "PUT /configs?force=true")
    }

    pub fn delete(&self, path: &str) -> Result<(), String> {
        let response = self
            .authorize(self.client.delete(format!("{}{path}", self.base_url)))
            .send()
            .map_err(|error| {
                format!(
                    "Could not send delete request: {}",
                    self.sanitize_error(&error.to_string())
                )
            })?;
        self.ensure_success(response, &format!("DELETE {path}"))
    }

    pub fn post_json(&self, path: &str, body: serde_json::Value) -> Result<(), String> {
        let response = self
            .authorize(self.client.post(format!("{}{path}", self.base_url)))
            .json(&body)
            .send()
            .map_err(|error| {
                format!(
                    "Could not send POST request: {}",
                    self.sanitize_error(&error.to_string())
                )
            })?;
        self.ensure_success(response, &format!("POST {path}"))
    }

    pub fn post(&self, path: &str) -> Result<(), String> {
        let response = self
            .authorize(self.client.post(format!("{}{path}", self.base_url)))
            .send()
            .map_err(|error| {
                format!(
                    "Could not send POST request: {}",
                    self.sanitize_error(&error.to_string())
                )
            })?;
        self.ensure_success(response, &format!("POST {path}"))
    }

    fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, String> {
        let response = self
            .authorize(self.client.get(format!("{}{path}", self.base_url)))
            .send()
            .map_err(|error| {
                format!(
                    "Controller request failed: {}",
                    self.sanitize_error(&error.to_string())
                )
            })?;
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

    fn ensure_success(
        &self,
        response: reqwest::blocking::Response,
        operation: &str,
    ) -> Result<(), String> {
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("{operation} failed with {}", response.status()))
        }
    }
}

fn redact_embedded_urls(message: &str) -> String {
    let mut output = String::with_capacity(message.len());
    let mut remaining = message;
    loop {
        let http = remaining.find("http://");
        let https = remaining.find("https://");
        let start = match (http, https) {
            (Some(http), Some(https)) => http.min(https),
            (Some(http), None) => http,
            (None, Some(https)) => https,
            (None, None) => {
                output.push_str(remaining);
                break;
            }
        };
        output.push_str(&remaining[..start]);
        let url_end = remaining[start..]
            .find(|character: char| character.is_whitespace() || character == ')')
            .map(|offset| start + offset)
            .unwrap_or(remaining.len());
        let token = &remaining[start..url_end];
        if let Ok(mut url) = url::Url::parse(token) {
            let _ = url.set_username("");
            let _ = url.set_password(None);
            url.set_query(None);
            url.set_fragment(None);
            output.push_str(url.as_str());
        } else {
            output.push_str("[redacted URL]");
        }
        remaining = &remaining[url_end..];
    }
    output
}
