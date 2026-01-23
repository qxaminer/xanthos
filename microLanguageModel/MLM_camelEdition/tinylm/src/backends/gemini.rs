//! # Google Gemini Backend

use super::*;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use std::time::Instant;

const DEFAULT_MODEL: &str = "gemini-1.5-pro";

pub struct GeminiBackend {
    client: Client,
    api_key: SecretString,
    model: String,
}

impl GeminiBackend {
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
        let key = std::env::var("GOOGLE_API_KEY").ok()?;
        Some(Self::new(SecretString::new(key.into())))
    }

    fn api_url(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        )
    }
}

#[async_trait]
impl Backend for GeminiBackend {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        // Convert to Gemini format
        let contents: Vec<_> = request.messages.iter()
            .filter(|m| m.role != Role::System)  // Gemini handles system differently
            .map(|m| serde_json::json!({
                "role": match m.role {
                    Role::User => "user",
                    Role::Assistant => "model",
                    Role::System => "user",  // Fallback
                },
                "parts": [{"text": m.content}]
            }))
            .collect();

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": request.max_tokens,
                "temperature": request.temperature,
            }
        });

        // Add system instruction if present
        if let Some(sys) = request.messages.iter().find(|m| m.role == Role::System) {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": sys.content}]
            });
        }

        let response = self.client
            .post(&self.api_url())
            .query(&[("key", self.api_key.expose_secret())])
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

        let content = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| BackendError::InvalidResponse("missing content".into()))?
            .to_string();

        // Gemini usage is in a different format
        let usage = Usage {
            input_tokens: data["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
        };

        Ok(CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            backend: Some("gemini".to_string()),
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

        // Gemini 1.5 Pro pricing
        let input_cost = input_tokens as f64 * 0.00125 / 1000.0;
        let output_cost = output_tokens as f64 * 0.005 / 1000.0;

        Some(input_cost + output_cost)
    }
}
