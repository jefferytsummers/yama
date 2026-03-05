//! Yama Browser Tool
//!
//! Provides web browsing capabilities for agents including fetching pages,
//! extracting content, and following links.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, Level};
use tracing_subscriber::FmtSubscriber;
use url::Url;

use yama_container_sdk::{EventBusClient, HealthReporter};

/// Browser operation.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum BrowserOperation {
    /// Fetch a URL and return the content.
    Fetch {
        url: String,
        #[serde(default)]
        extract_text: bool,
    },
    /// Extract specific elements from a page.
    Extract {
        url: String,
        selector: String,
    },
    /// Get all links from a page.
    GetLinks {
        url: String,
    },
    /// Search for text on a page.
    Search {
        url: String,
        query: String,
    },
}

/// Browser request.
#[derive(Debug, Clone, Deserialize)]
struct BrowserRequest {
    request_id: String,
    #[serde(flatten)]
    operation: BrowserOperation,
}

/// Browser result.
#[derive(Debug, Clone, Serialize)]
struct BrowserResult {
    request_id: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elements: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    links: Option<Vec<LinkInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    matches: Option<Vec<SearchMatch>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Link information.
#[derive(Debug, Clone, Serialize)]
struct LinkInfo {
    href: String,
    text: String,
}

/// Search match.
#[derive(Debug, Clone, Serialize)]
struct SearchMatch {
    context: String,
    index: usize,
}

/// Service configuration.
#[derive(Debug, Clone)]
struct Config {
    event_bus_socket: String,
    user_agent: String,
    timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            event_bus_socket: std::env::var("YAMA_EVENT_BUS")
                .unwrap_or_else(|_| "/tmp/yama-event.sock".to_string()),
            user_agent: "Yama-Agent/1.0".to_string(),
            timeout_secs: 30,
        }
    }
}

/// Browser service.
struct BrowserService {
    config: Config,
    client: Client,
    event_bus: Arc<EventBusClient>,
    health: HealthReporter,
}

impl BrowserService {
    async fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .user_agent(&config.user_agent)
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;

        let event_bus = Arc::new(
            EventBusClient::connect(&config.event_bus_socket, "tool-browser")
                .await
                .context("Failed to connect to event bus")?,
        );

        let health = HealthReporter::new(event_bus.clone(), "tool-browser");

        Ok(Self {
            config,
            client,
            event_bus,
            health,
        })
    }

    async fn run(self) -> Result<()> {
        info!("Starting browser service");

        self.health.healthy().await;
        self.health.start().await?;

        self.event_bus
            .subscribe(&["tool.browser.*"])
            .await?;

        info!("Browser service ready");

        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    async fn handle_request(&self, request: BrowserRequest) -> BrowserResult {
        let request_id = request.request_id.clone();

        match self.execute_operation(request.operation).await {
            Ok(result) => result.with_request_id(request_id),
            Err(e) => BrowserResult {
                request_id,
                success: false,
                content: None,
                elements: None,
                links: None,
                matches: None,
                error: Some(e.to_string()),
            },
        }
    }

    async fn execute_operation(&self, operation: BrowserOperation) -> Result<PartialResult> {
        match operation {
            BrowserOperation::Fetch { url, extract_text } => {
                let response = self.client.get(&url).send().await?;
                let html = response.text().await?;

                let content = if extract_text {
                    let document = Html::parse_document(&html);
                    extract_text_content(&document)
                } else {
                    html
                };

                Ok(PartialResult::Content(content))
            }

            BrowserOperation::Extract { url, selector } => {
                let response = self.client.get(&url).send().await?;
                let html = response.text().await?;
                let document = Html::parse_document(&html);

                let selector = Selector::parse(&selector)
                    .map_err(|e| anyhow::anyhow!("Invalid selector: {:?}", e))?;

                let elements: Vec<String> = document
                    .select(&selector)
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                Ok(PartialResult::Elements(elements))
            }

            BrowserOperation::GetLinks { url } => {
                let base_url = Url::parse(&url)?;
                let response = self.client.get(&url).send().await?;
                let html = response.text().await?;
                let document = Html::parse_document(&html);

                let selector = Selector::parse("a[href]").unwrap();

                let links: Vec<LinkInfo> = document
                    .select(&selector)
                    .filter_map(|el| {
                        let href = el.value().attr("href")?;
                        let absolute_url = base_url.join(href).ok()?.to_string();
                        let text = el.text().collect::<String>().trim().to_string();
                        Some(LinkInfo {
                            href: absolute_url,
                            text,
                        })
                    })
                    .collect();

                Ok(PartialResult::Links(links))
            }

            BrowserOperation::Search { url, query } => {
                let response = self.client.get(&url).send().await?;
                let html = response.text().await?;
                let document = Html::parse_document(&html);
                let text = extract_text_content(&document);

                let query_lower = query.to_lowercase();
                let text_lower = text.to_lowercase();

                let mut matches = Vec::new();
                let mut start = 0;

                while let Some(idx) = text_lower[start..].find(&query_lower) {
                    let abs_idx = start + idx;

                    // Extract context around match
                    let context_start = abs_idx.saturating_sub(50);
                    let context_end = (abs_idx + query.len() + 50).min(text.len());
                    let context = text[context_start..context_end].to_string();

                    matches.push(SearchMatch {
                        context,
                        index: abs_idx,
                    });

                    start = abs_idx + query.len();
                }

                Ok(PartialResult::Matches(matches))
            }
        }
    }
}

/// Extract text content from HTML, removing scripts and styles.
fn extract_text_content(document: &Html) -> String {
    let body_selector = Selector::parse("body").unwrap();
    let script_selector = Selector::parse("script, style, noscript").unwrap();

    let mut text = String::new();

    if let Some(body) = document.select(&body_selector).next() {
        for node in body.text() {
            let trimmed = node.trim();
            if !trimmed.is_empty() {
                text.push_str(trimmed);
                text.push(' ');
            }
        }
    }

    text.trim().to_string()
}

enum PartialResult {
    Content(String),
    Elements(Vec<String>),
    Links(Vec<LinkInfo>),
    Matches(Vec<SearchMatch>),
}

impl PartialResult {
    fn with_request_id(self, request_id: String) -> BrowserResult {
        match self {
            PartialResult::Content(content) => BrowserResult {
                request_id,
                success: true,
                content: Some(content),
                elements: None,
                links: None,
                matches: None,
                error: None,
            },
            PartialResult::Elements(elements) => BrowserResult {
                request_id,
                success: true,
                content: None,
                elements: Some(elements),
                links: None,
                matches: None,
                error: None,
            },
            PartialResult::Links(links) => BrowserResult {
                request_id,
                success: true,
                content: None,
                elements: None,
                links: Some(links),
                matches: None,
                error: None,
            },
            PartialResult::Matches(matches) => BrowserResult {
                request_id,
                success: true,
                content: None,
                elements: None,
                links: None,
                matches: Some(matches),
                error: None,
            },
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .pretty()
        .init();

    info!("Starting Yama Browser Tool");

    let config = Config::default();
    let service = BrowserService::new(config).await?;
    service.run().await
}
