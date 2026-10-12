use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const REPOSITORY: &str = "neko233-com/clash-go";
const API_ROOT: &str = "https://api.github.com";
const DEFAULT_MIRROR: &str = "https://gh-proxy.org/";

#[derive(Clone, Debug, Serialize)]
pub struct RouteStatus {
    pub name: String,
    pub prefix: String,
    pub reachable: bool,
    pub latency_ms: Option<u128>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetworkReport {
    pub routes: Vec<RouteStatus>,
    pub selected: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub digest: Option<String>,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Release {
    pub tag_name: String,
    pub name: String,
    pub html_url: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Clone)]
pub struct GithubClient {
    client: Client,
}

impl GithubClient {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent(format!("clash-go/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(6))
            .build()
            .map_err(|error| format!("Could not initialize GitHub client: {error}"))?;
        Ok(Self { client })
    }

    pub fn network_report(&self) -> NetworkReport {
        let mut candidates = vec![("github-direct".to_string(), String::new())];
        let mirrors = std::env::var("CLASH_GO_GITHUB_MIRRORS").ok();
        let mirrors: Vec<String> = mirrors
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| format!("{}/", value.trim_end_matches('/')))
                    .collect()
            })
            .unwrap_or_else(|| vec![DEFAULT_MIRROR.to_string()]);
        for (index, mirror) in mirrors.into_iter().enumerate() {
            candidates.push((format!("github-mirror-{}", index + 1), mirror));
        }

        let mut routes = Vec::with_capacity(candidates.len());
        for (name, prefix) in candidates {
            let endpoint = Self::through(&prefix, &format!("{API_ROOT}/repos/{REPOSITORY}"));
            let started = Instant::now();
            let result = self.client.get(endpoint).send();
            let elapsed = started.elapsed().as_millis();
            match result {
                Ok(response) if response.status().is_success() => routes.push(RouteStatus {
                    name,
                    prefix,
                    reachable: true,
                    latency_ms: Some(elapsed),
                    detail: None,
                }),
                Ok(response) => routes.push(RouteStatus {
                    name,
                    prefix,
                    reachable: false,
                    latency_ms: Some(elapsed),
                    detail: Some(format!("HTTP {}", response.status())),
                }),
                Err(error) => routes.push(RouteStatus {
                    name,
                    prefix,
                    reachable: false,
                    latency_ms: Some(elapsed),
                    detail: Some(error.to_string()),
                }),
            }
        }
        let selected = routes
            .iter()
            .filter(|route| route.reachable)
            .min_by_key(|route| route.latency_ms.unwrap_or(u128::MAX))
            .map(|route| route.name.clone());
        NetworkReport { routes, selected }
    }

    pub fn latest_release(&self, report: &NetworkReport) -> Result<Option<Release>, String> {
        let selected_name = report.selected.as_deref().ok_or_else(|| {
            "GitHub is unreachable directly and through configured mirrors".to_string()
        })?;
        let route = report
            .routes
            .iter()
            .find(|route| route.name == selected_name)
            .ok_or_else(|| "Selected GitHub route disappeared".to_string())?;
        let url = Self::through(
            &route.prefix,
            &format!("{API_ROOT}/repos/{REPOSITORY}/releases/latest"),
        );
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|error| format!("Could not check GitHub releases: {error}"))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(format!(
                "GitHub release endpoint returned {}",
                response.status()
            ));
        }
        response
            .json::<Release>()
            .map(Some)
            .map_err(|error| format!("Invalid GitHub release response: {error}"))
    }

    fn through(prefix: &str, official_url: &str) -> String {
        if prefix.is_empty() {
            official_url.to_string()
        } else {
            format!("{prefix}{official_url}")
        }
    }
}

pub fn is_newer_version(latest_tag: &str, current: &str) -> bool {
    let latest = latest_tag.trim_start_matches(['v', 'V']);
    match (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        (Ok(latest), Ok(current)) => latest > current,
        _ => latest_tag != format!("v{current}") && latest_tag != current,
    }
}
