# Claude Code Prompt: tinyLM (wgpu + Multi-Backend)

## IDENTITY

**Name:** tinyLM  
**Purpose:** Multi-LLM orchestrator with local inference and frontier API backends  
**License:** AGPL-3.0  
**Target:** 38.83 GB arena, wgpu compute, blind testing infrastructure

---

## WHY WGPU

```
Raw Metal:  macOS only, vendor lock-in
wgpu:       Metal on M4, Vulkan on Linux, DX12 on Windows
            Same performance (it's a thin abstraction)
            Rust-native, better ecosystem
            More contributors can test/improve
```

---

## ARCHITECTURE OVERVIEW

```
┌─────────────────────────────────────────────────────────────────┐
│                         tinyLM Orchestrator                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                   Frontier Backends                      │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │    │
│  │  │ Claude  │ │  GPT-4  │ │ Gemini  │ │ Mistral │  ...   │    │
│  │  │   API   │ │   API   │ │   API   │ │   API   │        │    │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘        │    │
│  │       └───────────┴──────────┴───────────┘               │    │
│  └─────────────────────────┬───────────────────────────────┘    │
│                            │                                     │
│                    ┌───────▼───────┐                            │
│                    │  Response     │                            │
│                    │  Arena        │◄──── 32-bit pointers       │
│                    │  (38.83 GB)   │◄──── mmap'd to disk        │
│                    └───────┬───────┘                            │
│                            │                                     │
│  ┌─────────────────────────▼───────────────────────────────┐    │
│  │                   Local Stable (microLMs)                │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │    │
│  │  │ Phi-3   │ │ Qwen-3B │ │TinyLlama│ │  RWKV   │  ...   │    │
│  │  │  3.8B   │ │   3B    │ │  1.1B   │ │  1.5B   │        │    │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘        │    │
│  │                                                          │    │
│  │  wgpu compute ──► Metal (M4) / Vulkan / DX12            │    │
│  └──────────────────────────────────────────────────────────┘    │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Blind Testing                           │   │
│  │  branch/gemini ◄── training data                         │   │
│  │  branch/claude ◄── comparison data                       │   │
│  │  branch/gpt4   ◄── comparison data                       │   │
│  │                                                           │   │
│  │  Orchestrator merges without knowing source              │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## GUARDRAILS

```yaml
allowed:
  - cargo commands
  - file operations within ./tinylm/ only
  - wgpu shader compilation
  - HTTPS to: api.anthropic.com, api.openai.com, generativelanguage.googleapis.com
  - HTTPS to: huggingface.co (model downloads, with approval)
  - git branch/merge operations within project

forbidden:
  - storing API keys in code (use env vars or keychain)
  - modifying files outside project directory
  - accessing ~/.ssh, credentials beyond designated keychain
  - running as root
  - disabling memory protections

api_key_handling:
  - read from environment variables only
  - never log or print keys
  - never include in error messages
  - use SecretString type for in-memory handling

on_error:
  - show full error (redact any keys)
  - explain root cause
  - propose fix
  - wait for approval
```

---

## DIRECTORY STRUCTURE

```
tinylm/
├── Cargo.toml
├── LICENSE                         # AGPL-3.0
├── README.md
├── .env.example                    # Template for API keys
├── src/
│   ├── lib.rs
│   ├── arena/
│   │   ├── mod.rs
│   │   ├── ptr32.rs
│   │   └── mmap.rs
│   ├── compute/
│   │   ├── mod.rs
│   │   ├── wgpu_context.rs         # wgpu device/queue setup
│   │   ├── shaders/
│   │   │   ├── matmul.wgsl
│   │   │   ├── rmsnorm.wgsl
│   │   │   └── softmax.wgsl
│   │   └── kernels.rs              # Kernel dispatch
│   ├── backends/
│   │   ├── mod.rs                  # Backend trait
│   │   ├── claude.rs               # Anthropic API
│   │   ├── openai.rs               # OpenAI API
│   │   ├── gemini.rs               # Google API
│   │   ├── mistral.rs              # Mistral API
│   │   └── local.rs                # Local model inference
│   ├── stable/
│   │   ├── mod.rs                  # Local model management
│   │   ├── loader.rs               # GGUF/safetensors loading
│   │   ├── phi3.rs                 # Phi-3 specific
│   │   ├── qwen.rs                 # Qwen specific
│   │   ├── tinyllama.rs            # TinyLlama specific
│   │   └── rwkv.rs                 # RWKV (linear attention)
│   ├── orchestrate/
│   │   ├── mod.rs
│   │   ├── router.rs               # Route queries to backends
│   │   ├── diff.rs                 # Semantic diff
│   │   ├── merge.rs                # Merge strategies
│   │   └── blind.rs                # Blind testing harness
│   └── testing/
│       ├── mod.rs
│       ├── branches.rs             # Git-like branching for data
│       └── eval.rs                 # Evaluation metrics
├── data/
│   └── branches/
│       ├── gemini/                 # Training data from Gemini
│       ├── claude/                 # Comparison data
│       ├── gpt4/                   # Comparison data
│       └── merged/                 # Orchestrator output
├── models/                         # Local model weights (gitignored)
└── benches/
    └── inference_bench.rs
```

---

## PHASE 0: FOUNDATION

### Cargo.toml

```toml
[package]
name = "tinylm"
version = "0.1.0"
edition = "2021"
license = "AGPL-3.0"
description = "Multi-LLM orchestrator with local inference"

[dependencies]
# Compute
wgpu = "0.19"
bytemuck = { version = "1.14", features = ["derive"] }

# Async runtime
tokio = { version = "1.36", features = ["full"] }

# HTTP clients for APIs
reqwest = { version = "0.11", features = ["json", "rustls-tls"], default-features = false }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Secret handling
secrecy = "0.8"

# Model loading
safetensors = "0.4"
memmap2 = "0.9"
half = "2.3"

# Utilities
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1.7", features = ["v4"] }

# Environment
dotenvy = "0.15"

[dev-dependencies]
criterion = { version = "0.5", features = ["async_tokio"] }
tokio-test = "0.4"
tempfile = "3.10"

[[bench]]
name = "inference_bench"
harness = false

[profile.release]
opt-level = 3
lto = "fat"
codegen-units = 1
```

### .env.example

```bash
# Frontier API Keys (never commit actual values)
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
GOOGLE_API_KEY=...
MISTRAL_API_KEY=...

# Local model paths
TINYLM_MODELS_DIR=./models
TINYLM_ARENA_PATH=./arena.bin

# Logging
RUST_LOG=tinylm=debug
```

---

## PHASE 1: BACKEND TRAIT

### src/backends/mod.rs

```rust
//! # LLM Backends
//!
//! Unified interface for frontier APIs and local models.
//! All backends implement the same trait for blind testing.

pub mod claude;
pub mod openai;
pub mod gemini;
pub mod mistral;
pub mod local;

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
    fn estimate_cost(&self, request: &CompletionRequest) -> Option<f64> {
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
}
```

---

## PHASE 2: FRONTIER BACKENDS

### src/backends/claude.rs

```rust
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
        Some(Self::new(SecretString::new(key)))
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
        
        if request.temperature != 0.7 {
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
```

### src/backends/openai.rs

```rust
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
        Some(Self::new(SecretString::new(key)))
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
}
```

### src/backends/gemini.rs

```rust
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
        Some(Self::new(SecretString::new(key)))
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
}
```

---

## PHASE 3: LOCAL STABLE (microLMs)

### src/stable/mod.rs

```rust
//! # Local Model Stable
//!
//! Manages a "stable" of small local models for orchestration.
//! These run on wgpu, inference happens in the arena.

pub mod loader;

use crate::arena::Arena;
use crate::backends::{Backend, BackendError, CompletionRequest, CompletionResponse, Result, Usage};
use crate::compute::WgpuContext;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A local model loaded into the arena
pub struct LocalModel {
    pub name: String,
    pub weights_offset: usize,
    pub config: ModelConfig,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub vocab_size: u32,
    pub hidden_size: u32,
    pub num_layers: u32,
    pub num_heads: u32,
    pub context_length: u32,
}

/// The stable manages multiple local models
pub struct Stable {
    arena: Arc<RwLock<Arena>>,
    wgpu: Arc<WgpuContext>,
    models: Vec<LocalModel>,
}

impl Stable {
    pub fn new(arena: Arc<RwLock<Arena>>, wgpu: Arc<WgpuContext>) -> Self {
        Self {
            arena,
            wgpu,
            models: Vec::new(),
        }
    }
    
    /// Load a model from a GGUF file into the arena
    pub async fn load_gguf(&mut self, path: &str, name: &str) -> Result<()> {
        let mut arena = self.arena.write().await;
        
        let (weights_offset, config) = loader::load_gguf(&mut arena, path)
            .map_err(|e| BackendError::ModelNotLoaded(e.to_string()))?;
        
        self.models.push(LocalModel {
            name: name.to_string(),
            weights_offset,
            config,
        });
        
        tracing::info!("Loaded model {} from {} (offset: {})", name, path, weights_offset);
        
        Ok(())
    }
    
    /// Get a backend interface for a loaded model
    pub fn backend(&self, name: &str) -> Option<LocalBackend> {
        let model = self.models.iter().find(|m| m.name == name)?;
        
        Some(LocalBackend {
            name: format!("local:{}", name),
            arena: Arc::clone(&self.arena),
            wgpu: Arc::clone(&self.wgpu),
            weights_offset: model.weights_offset,
            config: model.config.clone(),
        })
    }
    
    /// List loaded models
    pub fn list(&self) -> Vec<&str> {
        self.models.iter().map(|m| m.name.as_str()).collect()
    }
}

/// Backend implementation for a local model
pub struct LocalBackend {
    name: String,
    arena: Arc<RwLock<Arena>>,
    wgpu: Arc<WgpuContext>,
    weights_offset: usize,
    config: ModelConfig,
}

#[async_trait]
impl Backend for LocalBackend {
    fn name(&self) -> &str {
        &self.name
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let start = std::time::Instant::now();
        
        // Tokenize input (simplified)
        let input_text: String = request.messages.iter()
            .map(|m| format!("{}: {}", 
                match m.role {
                    crate::backends::Role::System => "system",
                    crate::backends::Role::User => "user",
                    crate::backends::Role::Assistant => "assistant",
                },
                m.content
            ))
            .collect::<Vec<_>>()
            .join("\n");
        
        // TODO: Actual inference using arena + wgpu
        // For now, placeholder
        let content = format!(
            "[Local model {} would generate response to: {}...]",
            self.name,
            &input_text[..input_text.len().min(50)]
        );
        
        Ok(CompletionResponse {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            backend: Some(self.name.clone()),
            usage: Usage {
                input_tokens: (input_text.len() / 4) as u32,
                output_tokens: (content.len() / 4) as u32,
            },
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
    
    async fn health_check(&self) -> Result<bool> {
        // Check that weights are accessible in arena
        let arena = self.arena.read().await;
        let stats = arena.stats();
        Ok(self.weights_offset < stats.allocated)
    }
    
    fn estimate_cost(&self, _request: &CompletionRequest) -> Option<f64> {
        // Local models are free (just electricity)
        Some(0.0)
    }
}
```

---

## PHASE 4: WGPU COMPUTE

### src/compute/wgpu_context.rs

```rust
//! # wgpu Context
//!
//! Manages GPU device, queue, and shader pipelines.
//! Works on Metal (M4), Vulkan (Linux), DX12 (Windows).

use std::sync::Arc;
use wgpu::util::DeviceExt;

pub struct WgpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    
    // Compiled pipelines
    pub matmul_pipeline: wgpu::ComputePipeline,
    pub rmsnorm_pipeline: wgpu::ComputePipeline,
    pub softmax_pipeline: wgpu::ComputePipeline,
}

impl WgpuContext {
    pub async fn new() -> Option<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await?;
        
        tracing::info!("Using GPU: {:?}", adapter.get_info());
        
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("tinylm"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .ok()?;
        
        // Load shaders
        let matmul_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("matmul"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/matmul.wgsl").into()),
        });
        
        let rmsnorm_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rmsnorm"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/rmsnorm.wgsl").into()),
        });
        
        let softmax_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("softmax"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/softmax.wgsl").into()),
        });
        
        // Create pipelines
        let matmul_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("matmul_pipeline"),
            layout: None,
            module: &matmul_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let rmsnorm_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rmsnorm_pipeline"),
            layout: None,
            module: &rmsnorm_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        let softmax_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("softmax_pipeline"),
            layout: None,
            module: &softmax_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        
        Some(Self {
            device,
            queue,
            matmul_pipeline,
            rmsnorm_pipeline,
            softmax_pipeline,
        })
    }
    
    /// Create a buffer from arena memory (zero-copy on unified memory)
    pub fn buffer_from_slice(&self, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: data,
            usage,
        })
    }
}
```

### src/compute/shaders/matmul.wgsl

```wgsl
// Matrix multiplication: A[M,K] × B[K,N] = C[M,N]
// Supports Q4 quantized A matrix

struct Params {
    M: u32,
    K: u32,
    N: u32,
    _pad: u32,
}

@group(0) @binding(0) var<storage, read> A: array<u32>;  // Quantized weights
@group(0) @binding(1) var<storage, read> B: array<f32>;  // Input activations
@group(0) @binding(2) var<storage, read_write> C: array<f32>;  // Output
@group(0) @binding(3) var<uniform> params: Params;

const BLOCK_SIZE: u32 = 32u;
const BLOCK_BYTES: u32 = 18u;  // 2 bytes scale + 16 bytes data

fn dequantize_q4(packed: u32, idx: u32, scale: f32) -> f32 {
    let shift = (idx % 2u) * 4u;
    let byte_idx = idx / 2u;
    let nibble = (packed >> (byte_idx * 8u + shift)) & 0xFu;
    var q = i32(nibble);
    if (q > 7) { q -= 16; }
    return f32(q) * scale;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let row = gid.y;
    let col = gid.x;
    
    if (row >= params.M || col >= params.N) {
        return;
    }
    
    var sum: f32 = 0.0;
    let blocks_per_row = (params.K + BLOCK_SIZE - 1u) / BLOCK_SIZE;
    
    for (var block = 0u; block < blocks_per_row; block++) {
        // Read scale (stored as f16 in first 2 bytes)
        let block_start = (row * blocks_per_row + block) * BLOCK_BYTES / 4u;
        let scale_bits = A[block_start] & 0xFFFFu;
        let scale = unpack2x16float(scale_bits).x;
        
        // Process 32 elements
        for (var i = 0u; i < BLOCK_SIZE && (block * BLOCK_SIZE + i) < params.K; i++) {
            let k = block * BLOCK_SIZE + i;
            let a_val = dequantize_q4(A[block_start + 1u + i / 8u], i % 8u, scale);
            sum += a_val * B[k * params.N + col];
        }
    }
    
    C[row * params.N + col] = sum;
}
```

---

## PHASE 5: BLIND TESTING

### src/testing/branches.rs

```rust
//! # Branch-based Data Management
//!
//! Stores LLM outputs in separate "branches" for blind testing.
//! The orchestrator sees data without knowing the source.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const BRANCHES_DIR: &str = "data/branches";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub id: String,
    pub prompt: String,
    pub response: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub name: String,
    pub samples: Vec<Sample>,
}

impl Branch {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            samples: Vec::new(),
        }
    }
    
    pub fn load(name: &str) -> std::io::Result<Self> {
        let path = PathBuf::from(BRANCHES_DIR).join(name).join("data.json");
        let data = fs::read_to_string(&path)?;
        let branch: Branch = serde_json::from_str(&data)?;
        Ok(branch)
    }
    
    pub fn save(&self) -> std::io::Result<()> {
        let dir = PathBuf::from(BRANCHES_DIR).join(&self.name);
        fs::create_dir_all(&dir)?;
        let path = dir.join("data.json");
        let data = serde_json::to_string_pretty(&self)?;
        fs::write(&path, data)?;
        Ok(())
    }
    
    pub fn add_sample(&mut self, sample: Sample) {
        self.samples.push(sample);
    }
}

/// Manages multiple branches for blind testing
pub struct BranchManager {
    branches: HashMap<String, Branch>,
}

impl BranchManager {
    pub fn new() -> Self {
        Self {
            branches: HashMap::new(),
        }
    }
    
    pub fn load_all() -> std::io::Result<Self> {
        let mut manager = Self::new();
        
        let branches_path = Path::new(BRANCHES_DIR);
        if branches_path.exists() {
            for entry in fs::read_dir(branches_path)? {
                let entry = entry?;
                if entry.file_type()?.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Ok(branch) = Branch::load(&name) {
                        manager.branches.insert(name, branch);
                    }
                }
            }
        }
        
        Ok(manager)
    }
    
    pub fn get_or_create(&mut self, name: &str) -> &mut Branch {
        self.branches.entry(name.to_string())
            .or_insert_with(|| Branch::new(name))
    }
    
    /// Get samples for blind testing (source hidden)
    pub fn blind_samples(&self) -> Vec<BlindSample> {
        let mut samples: Vec<BlindSample> = self.branches
            .iter()
            .flat_map(|(branch_name, branch)| {
                branch.samples.iter().map(move |s| BlindSample {
                    id: s.id.clone(),
                    prompt: s.prompt.clone(),
                    response: s.response.clone(),
                    // Source is hidden but tracked internally
                    _source: branch_name.clone(),
                })
            })
            .collect();
        
        // Shuffle to prevent ordering bias
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        samples.sort_by(|a, b| {
            let mut ha = DefaultHasher::new();
            let mut hb = DefaultHasher::new();
            a.id.hash(&mut ha);
            b.id.hash(&mut hb);
            ha.finish().cmp(&hb.finish())
        });
        
        samples
    }
    
    /// Reveal source after evaluation
    pub fn reveal_source(&self, id: &str) -> Option<&str> {
        for (name, branch) in &self.branches {
            if branch.samples.iter().any(|s| s.id == id) {
                return Some(name);
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct BlindSample {
    pub id: String,
    pub prompt: String,
    pub response: String,
    _source: String,  // Private, not exposed until reveal
}
```

### src/orchestrate/blind.rs

```rust
//! # Blind Testing Harness
//!
//! Runs orchestrator decisions without revealing which LLM produced what.

use crate::backends::{Backend, BackendRegistry, CompletionRequest, Message, Role};
use crate::testing::branches::{BranchManager, Sample};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindTestResult {
    pub prompt: String,
    pub responses: Vec<BlindResponse>,
    pub orchestrator_choice: Option<String>,
    pub orchestrator_confidence: f32,
    pub revealed_sources: HashMap<String, String>,  // id -> source
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindResponse {
    pub id: String,
    pub content: String,
    // Source hidden until reveal
}

pub struct BlindTestHarness {
    registry: BackendRegistry,
    branch_manager: BranchManager,
}

impl BlindTestHarness {
    pub fn new(registry: BackendRegistry) -> Self {
        Self {
            registry,
            branch_manager: BranchManager::load_all().unwrap_or_else(|_| BranchManager::new()),
        }
    }
    
    /// Run a prompt against all backends, store results in branches
    pub async fn collect_responses(&mut self, prompt: &str) -> Vec<String> {
        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: prompt.to_string(),
            }],
            ..Default::default()
        };
        
        let mut ids = Vec::new();
        
        for backend in self.registry.all() {
            match backend.complete(request.clone()).await {
                Ok(response) => {
                    let sample = Sample {
                        id: response.id.clone(),
                        prompt: prompt.to_string(),
                        response: response.content,
                        metadata: HashMap::new(),
                    };
                    
                    // Store in branch named after backend
                    let branch = self.branch_manager.get_or_create(backend.name());
                    branch.add_sample(sample);
                    
                    ids.push(response.id);
                }
                Err(e) => {
                    tracing::warn!("Backend {} failed: {}", backend.name(), e);
                }
            }
        }
        
        // Save all branches
        for (_, branch) in self.branch_manager.branches.iter() {
            if let Err(e) = branch.save() {
                tracing::error!("Failed to save branch {}: {}", branch.name, e);
            }
        }
        
        ids
    }
    
    /// Get blind samples for orchestrator evaluation
    pub fn get_blind_samples(&self) -> Vec<crate::testing::branches::BlindSample> {
        self.branch_manager.blind_samples()
    }
    
    /// Reveal sources after orchestrator makes decisions
    pub fn reveal(&self, id: &str) -> Option<&str> {
        self.branch_manager.reveal_source(id)
    }
}
```

---

## PHASE 6: GEMINI TRAINING DATA BRANCH

### data/branches/gemini/README.md

```markdown
# Gemini Training Data Branch

This branch contains training data from Google Gemini for blind testing.

## Data Format

Each sample in `data.json`:
```json
{
  "id": "unique-uuid",
  "prompt": "The original prompt",
  "response": "Gemini's response",
  "metadata": {
    "model": "gemini-1.5-pro",
    "timestamp": "2024-01-23T12:00:00Z"
  }
}
```

## Usage

This data is used for:
1. Training the local orchestrator model
2. Blind comparison against other backends
3. Evaluating merge strategies

## Adding Data

Paste Gemini responses here, or run:
```bash
cargo run -- collect --backend gemini --prompts prompts.txt
```
```

### data/branches/gemini/data.json (placeholder)

```json
{
  "name": "gemini",
  "samples": []
}
```

---

## EXECUTION PLAN

```bash
# Phase 0: Scaffold
mkdir -p tinylm/src/{arena,compute/shaders,backends,stable,orchestrate,testing}
mkdir -p tinylm/data/branches/{gemini,claude,gpt4,merged}
cd tinylm
cargo init --name tinylm

# Phase 1: Foundation (ptr32, mmap, arena)
# Phase 2: Backend trait + frontier APIs
# Phase 3: wgpu context + shaders
# Phase 4: Local stable (model loading)
# Phase 5: Blind testing harness
# Phase 6: Orchestrator logic

# Between phases: cargo check, cargo test
```

---

## ADDING GEMINI TRAINING DATA

To populate the gemini branch, either:

1. **Manually**: Paste data into `data/branches/gemini/data.json`
2. **Via API**: Run collection script with Gemini backend
3. **Import**: Convert existing Gemini outputs to the Sample format

**Give me the Gemini training data** and I'll add it to the branch structure, or tell me where to fetch it from.

---

## START

```bash
mkdir -p tinylm
cd tinylm
cargo init --name tinylm
```

Show me output, then we proceed with Cargo.toml.
