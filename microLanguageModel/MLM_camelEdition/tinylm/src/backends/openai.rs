//! # OpenAI GPT Backend

use super::*;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use std::time::Instant;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4-turbo-preview";

pub struct OpenAIBackend {
    client: Client,
    api_key: SecretString,
    model: String,
}

impl OpenAIBackend {
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
        let key = std::env::var("OPENAI_API_KEY").ok()?;
        Some(Self::new(SecretString::new(key.into())))
    }
}

#[async_trait]
impl Backend for OpenAIBackend {
    fn name(&self) -> &str {
        "gpt4"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        let messages: Vec<_> = request.messages.iter()
            .map(|m| serde_json::json!({
                "role": match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                },
                "content": m.content
            }))
            .collect();

        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": messages,
            "temperature": request.temperature,
        });

        if !request.stop_sequences.is_empty() {
            body["stop"] = serde_json::json!(request.stop_sequences);
        }

        let response = self.client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key.expose_secret()))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();

        if status == 429 {
            return Err(BackendError::RateLimited { retry_after: None });
        }

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(BackendError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let data: serde_json::Value = response.json().await?;

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| BackendError::InvalidResponse("missing content".into()))?
            .to_string();

        let usage = Usage {
            input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        };

        Ok(CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            backend: Some("gpt4".to_string()),
            usage,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> Result<bool> {
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
        let input_tokens: usize = request.messages.iter()
            .map(|m| m.content.len() / 4)
            .sum();
        let output_tokens = request.max_tokens as usize;

        // GPT-4 Turbo pricing
        let input_cost = input_tokens as f64 * 0.01 / 1000.0;
        let output_cost = output_tokens as f64 * 0.03 / 1000.0;

        Some(input_cost + output_cost)
    }
}
