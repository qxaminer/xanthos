//! # Anthropic Claude Backend

use super::*;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use std::time::Instant;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

pub struct ClaudeBackend {
    client: Client,
    api_key: SecretString,
    model: String,
}

impl ClaudeBackend {
    pub fn new(api_key: SecretString) -> Self {
        Self::with_model(api_key, DEFAULT_MODEL.to_string())
    }

    pub fn with_model(api_key: SecretString, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub fn from_env() -> Option<Self> {
        let key = std::env::var("ANTHROPIC_API_KEY").ok()?;
        Some(Self::new(SecretString::new(key.into())))
    }
}

#[async_trait]
impl Backend for ClaudeBackend {
    fn name(&self) -> &str {
        "claude"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        // Convert messages to Anthropic format
        let (system, messages): (Option<String>, Vec<_>) = {
            let mut sys = None;
            let msgs: Vec<_> = request.messages.iter()
                .filter_map(|m| {
                    match m.role {
                        Role::System => {
                            sys = Some(m.content.clone());
                            None
                        }
                        Role::User => Some(serde_json::json!({
                            "role": "user",
                            "content": m.content
                        })),
                        Role::Assistant => Some(serde_json::json!({
                            "role": "assistant",
                            "content": m.content
                        })),
                    }
                })
                .collect();
            (sys, msgs)
        };

        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": messages,
        });

        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        if (request.temperature - 0.7).abs() > f32::EPSILON {
            body["temperature"] = serde_json::json!(request.temperature);
        }

        if !request.stop_sequences.is_empty() {
            body["stop_sequences"] = serde_json::json!(request.stop_sequences);
        }

        let response = self.client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", self.api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();

        if status == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok())
                .map(Duration::from_secs);
            return Err(BackendError::RateLimited { retry_after });
        }

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let data: serde_json::Value = response.json().await?;

        let content = data["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .ok_or_else(|| BackendError::InvalidResponse("missing content".into()))?
            .to_string();

        let usage = Usage {
            input_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        };

        Ok(CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            backend: Some("claude".to_string()),
            usage,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        // Minimal request to check API connectivity
        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: "Hi".to_string(),
            }],
            max_tokens: 1,
            ..Default::default()
        };

        self.complete(request).await.map(|_| true)
    }

    fn estimate_cost(&self, request: &CompletionRequest) -> Option<f64> {
        // Rough token estimate (4 chars per token)
        let input_tokens: usize = request.messages.iter()
            .map(|m| m.content.len() / 4)
            .sum();
        let output_tokens = request.max_tokens as usize;

        // Claude Sonnet pricing (as of 2024)
        let input_cost = input_tokens as f64 * 0.003 / 1000.0;
        let output_cost = output_tokens as f64 * 0.015 / 1000.0;

        Some(input_cost + output_cost)
    }
}
