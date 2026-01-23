//! # LLM Backends
//!
//! Unified interface for frontier APIs and local models.
//! All backends implement the same trait for blind testing.

pub mod claude;
pub mod openai;
pub mod gemini;

use async_trait::async_trait;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackendError {
    #[error("API request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API returned error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited { retry_after: Option<Duration> },

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),

    #[error("Inference error: {0}")]
    Inference(String),
}

pub type Result<T> = std::result::Result<T, BackendError>;

/// Message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self { role: Role::System, content: content.into() }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self { role: Role::User, content: content.into() }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self { role: Role::Assistant, content: content.into() }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// Request to an LLM backend
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub stop_sequences: Vec<String>,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            max_tokens: 1024,
            temperature: 0.7,
            stop_sequences: Vec::new(),
        }
    }
}

impl CompletionRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            ..Default::default()
        }
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    pub fn with_stop(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = sequences;
        self
    }
}

/// Response from an LLM backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Unique ID for this response (for tracking)
    pub id: String,

    /// The generated content
    pub content: String,

    /// Which backend generated this (hidden in blind testing)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,

    /// Token usage stats
    pub usage: Usage,

    /// Latency in milliseconds
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

impl Usage {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Unified backend interface
#[async_trait]
pub trait Backend: Send + Sync {
    /// Human-readable name
    fn name(&self) -> &str;

    /// Generate a completion
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Check if backend is available
    async fn health_check(&self) -> Result<bool>;

    /// Estimate cost for a request (in USD, if applicable)
    fn estimate_cost(&self, _request: &CompletionRequest) -> Option<f64> {
        None // Local models return None
    }
}

/// Registry of available backends
pub struct BackendRegistry {
    backends: Vec<Box<dyn Backend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self { backends: Vec::new() }
    }

    pub fn register(&mut self, backend: Box<dyn Backend>) {
        tracing::info!("Registered backend: {}", backend.name());
        self.backends.push(backend);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Backend> {
        self.backends.iter()
            .find(|b| b.name() == name)
            .map(|b| b.as_ref())
    }

    pub fn all(&self) -> impl Iterator<Item = &dyn Backend> {
        self.backends.iter().map(|b| b.as_ref())
    }

    pub fn frontier(&self) -> impl Iterator<Item = &dyn Backend> {
        self.backends.iter()
            .filter(|b| !b.name().starts_with("local:"))
            .map(|b| b.as_ref())
    }

    pub fn local(&self) -> impl Iterator<Item = &dyn Backend> {
        self.backends.iter()
            .filter(|b| b.name().starts_with("local:"))
            .map(|b| b.as_ref())
    }

    pub fn len(&self) -> usize {
        self.backends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builders() {
        let sys = Message::system("You are helpful");
        assert_eq!(sys.role, Role::System);

        let user = Message::user("Hello");
        assert_eq!(user.role, Role::User);

        let asst = Message::assistant("Hi there");
        assert_eq!(asst.role, Role::Assistant);
    }

    #[test]
    fn test_request_builder() {
        let req = CompletionRequest::new(vec![Message::user("test")])
            .with_max_tokens(2048)
            .with_temperature(0.5);

        assert_eq!(req.max_tokens, 2048);
        assert_eq!(req.temperature, 0.5);
    }
}
