//! # Blind Testing Orchestrator
//!
//! Run prompts against multiple LLM backends in parallel,
//! collect responses without revealing which backend generated each,
//! and support evaluation/comparison.

use crate::backends::{Backend, CompletionRequest, CompletionResponse, Message};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A blind response - backend identity hidden until reveal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindResponse {
    /// Unique ID for this response
    pub id: String,

    /// The generated content
    pub content: String,

    /// Latency in milliseconds
    pub latency_ms: u64,

    /// Token usage
    pub input_tokens: u32,
    pub output_tokens: u32,

    /// Hidden backend name (only visible after reveal)
    #[serde(skip_serializing_if = "Option::is_none")]
    backend: Option<String>,
}

impl BlindResponse {
    /// Create from a CompletionResponse, hiding the backend
    fn from_response(resp: CompletionResponse) -> Self {
        Self {
            id: resp.id,
            content: resp.content,
            latency_ms: resp.latency_ms,
            input_tokens: resp.usage.input_tokens,
            output_tokens: resp.usage.output_tokens,
            backend: resp.backend,
        }
    }

    /// Reveal which backend generated this response
    pub fn reveal(&self) -> Option<&str> {
        self.backend.as_deref()
    }
}

/// Result of running a prompt against multiple backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlindTestResult {
    /// Unique test ID
    pub test_id: String,

    /// The original prompt
    pub prompt: String,

    /// Responses from each backend (shuffled, backend hidden)
    pub responses: Vec<BlindResponse>,

    /// Any backends that failed
    pub errors: HashMap<String, String>,

    /// Total time for the test
    pub total_ms: u64,
}

impl BlindTestResult {
    /// Reveal all backend identities
    pub fn reveal_all(&self) -> HashMap<&str, &BlindResponse> {
        self.responses
            .iter()
            .filter_map(|r| r.reveal().map(|name| (name, r)))
            .collect()
    }

    /// Get response by revealed backend name
    pub fn get_by_backend(&self, name: &str) -> Option<&BlindResponse> {
        self.responses.iter().find(|r| r.reveal() == Some(name))
    }
}

/// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Maximum concurrent requests
    pub max_concurrent: usize,

    /// Timeout per backend (ms)
    pub timeout_ms: u64,

    /// Whether to shuffle response order (true for blind testing)
    pub shuffle_responses: bool,

    /// Whether to continue if some backends fail
    pub allow_partial: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            timeout_ms: 30_000,
            shuffle_responses: true,
            allow_partial: true,
        }
    }
}

/// The blind testing orchestrator
pub struct Orchestrator {
    backends: Vec<Arc<dyn Backend>>,
    config: OrchestratorConfig,
    /// History of test results (for analysis)
    history: Arc<RwLock<Vec<BlindTestResult>>>,
}

impl Orchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            backends: Vec::new(),
            config,
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(OrchestratorConfig::default())
    }

    /// Register a backend for testing
    pub fn register(&mut self, backend: Arc<dyn Backend>) {
        tracing::info!("Orchestrator: registered backend '{}'", backend.name());
        self.backends.push(backend);
    }

    /// Register multiple backends
    pub fn register_all(&mut self, backends: Vec<Arc<dyn Backend>>) {
        for backend in backends {
            self.register(backend);
        }
    }

    /// Get list of registered backend names
    pub fn backend_names(&self) -> Vec<&str> {
        self.backends.iter().map(|b| b.name()).collect()
    }

    /// Run a prompt against all backends in parallel (blind test)
    pub async fn run_blind(
        &self,
        messages: Vec<Message>,
        max_tokens: u32,
    ) -> Result<BlindTestResult, OrchestratorError> {
        if self.backends.is_empty() {
            return Err(OrchestratorError::NoBackends);
        }

        let start = std::time::Instant::now();
        let test_id = Uuid::new_v4().to_string();

        // Build the prompt string for logging
        let prompt = messages
            .iter()
            .map(|m| format!("{:?}: {}", m.role, &m.content[..m.content.len().min(100)]))
            .collect::<Vec<_>>()
            .join(" | ");

        let request = CompletionRequest::new(messages).with_max_tokens(max_tokens);

        // Run all backends concurrently
        let futures: Vec<_> = self
            .backends
            .iter()
            .map(|backend| {
                let backend = Arc::clone(backend);
                let req = request.clone();
                let timeout = self.config.timeout_ms;

                async move {
                    let name = backend.name().to_string();
                    let result = tokio::time::timeout(
                        std::time::Duration::from_millis(timeout),
                        backend.complete(req),
                    )
                    .await;

                    match result {
                        Ok(Ok(response)) => Ok((name, response)),
                        Ok(Err(e)) => Err((name, e.to_string())),
                        Err(_) => Err((name, "Timeout".to_string())),
                    }
                }
            })
            .collect();

        let results = join_all(futures).await;

        // Separate successes and failures
        let mut responses: Vec<BlindResponse> = Vec::new();
        let mut errors: HashMap<String, String> = HashMap::new();

        for result in results {
            match result {
                Ok((name, response)) => {
                    tracing::debug!("Backend '{}' responded in {}ms", name, response.latency_ms);
                    responses.push(BlindResponse::from_response(response));
                }
                Err((name, error)) => {
                    tracing::warn!("Backend '{}' failed: {}", name, error);
                    errors.insert(name, error);
                }
            }
        }

        // Check if we have enough responses
        if responses.is_empty() {
            return Err(OrchestratorError::AllFailed(errors));
        }

        if !self.config.allow_partial && !errors.is_empty() {
            return Err(OrchestratorError::PartialFailure(errors));
        }

        // Shuffle responses for blind testing
        if self.config.shuffle_responses {
            use rand::seq::SliceRandom;
            let mut rng = rand::thread_rng();
            responses.shuffle(&mut rng);
        }

        let result = BlindTestResult {
            test_id,
            prompt,
            responses,
            errors,
            total_ms: start.elapsed().as_millis() as u64,
        };

        // Store in history
        {
            let mut history = self.history.write().await;
            history.push(result.clone());
        }

        Ok(result)
    }

    /// Run a simple text prompt against all backends
    pub async fn run_prompt(
        &self,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<BlindTestResult, OrchestratorError> {
        self.run_blind(vec![Message::user(prompt)], max_tokens).await
    }

    /// Run with system prompt + user prompt
    pub async fn run_with_system(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
    ) -> Result<BlindTestResult, OrchestratorError> {
        self.run_blind(
            vec![Message::system(system), Message::user(user)],
            max_tokens,
        )
        .await
    }

    /// Get test history
    pub async fn history(&self) -> Vec<BlindTestResult> {
        self.history.read().await.clone()
    }

    /// Clear test history
    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    /// Health check all backends
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let futures: Vec<_> = self
            .backends
            .iter()
            .map(|backend| {
                let backend = Arc::clone(backend);
                async move {
                    let name = backend.name().to_string();
                    let healthy = backend.health_check().await.unwrap_or(false);
                    (name, healthy)
                }
            })
            .collect();

        join_all(futures).await.into_iter().collect()
    }

    /// Estimate total cost for a request across all backends
    pub fn estimate_total_cost(&self, request: &CompletionRequest) -> f64 {
        self.backends
            .iter()
            .filter_map(|b| b.estimate_cost(request))
            .sum()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("No backends registered")]
    NoBackends,

    #[error("All backends failed: {0:?}")]
    AllFailed(HashMap<String, String>),

    #[error("Some backends failed (partial not allowed): {0:?}")]
    PartialFailure(HashMap<String, String>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{CompletionResponse, Result as BackendResult, Usage};

    // Mock backend for testing
    struct MockBackend {
        name: String,
        response: String,
        latency_ms: u64,
        should_fail: bool,
    }

    impl MockBackend {
        fn new(name: &str, response: &str, latency_ms: u64) -> Self {
            Self {
                name: name.to_string(),
                response: response.to_string(),
                latency_ms,
                should_fail: false,
            }
        }

        fn failing(name: &str) -> Self {
            Self {
                name: name.to_string(),
                response: String::new(),
                latency_ms: 0,
                should_fail: true,
            }
        }
    }

    #[async_trait::async_trait]
    impl Backend for MockBackend {
        fn name(&self) -> &str {
            &self.name
        }

        async fn complete(&self, _request: CompletionRequest) -> BackendResult<CompletionResponse> {
            if self.should_fail {
                return Err(BackendError::ApiError {
                    status: 500,
                    message: "Mock failure".into(),
                });
            }

            // Simulate latency
            tokio::time::sleep(std::time::Duration::from_millis(self.latency_ms)).await;

            Ok(CompletionResponse {
                id: Uuid::new_v4().to_string(),
                content: self.response.clone(),
                backend: Some(self.name.clone()),
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
                latency_ms: self.latency_ms,
            })
        }

        async fn health_check(&self) -> BackendResult<bool> {
            Ok(!self.should_fail)
        }
    }

    #[tokio::test]
    async fn test_orchestrator_basic() {
        let mut orchestrator = Orchestrator::with_defaults();

        orchestrator.register(Arc::new(MockBackend::new("mock-a", "Response A", 10)));
        orchestrator.register(Arc::new(MockBackend::new("mock-b", "Response B", 20)));

        let result = orchestrator.run_prompt("Hello", 100).await.unwrap();

        assert_eq!(result.responses.len(), 2);
        assert!(result.errors.is_empty());

        // Reveal should work
        let revealed = result.reveal_all();
        assert!(revealed.contains_key("mock-a"));
        assert!(revealed.contains_key("mock-b"));
    }

    #[tokio::test]
    async fn test_orchestrator_partial_failure() {
        let mut orchestrator = Orchestrator::with_defaults();

        orchestrator.register(Arc::new(MockBackend::new("mock-good", "Good response", 10)));
        orchestrator.register(Arc::new(MockBackend::failing("mock-bad")));

        let result = orchestrator.run_prompt("Hello", 100).await.unwrap();

        assert_eq!(result.responses.len(), 1);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors.contains_key("mock-bad"));
    }

    #[tokio::test]
    async fn test_orchestrator_no_backends() {
        let orchestrator = Orchestrator::with_defaults();

        let result = orchestrator.run_prompt("Hello", 100).await;

        assert!(matches!(result, Err(OrchestratorError::NoBackends)));
    }

    #[tokio::test]
    async fn test_health_check_all() {
        let mut orchestrator = Orchestrator::with_defaults();

        orchestrator.register(Arc::new(MockBackend::new("healthy", "ok", 10)));
        orchestrator.register(Arc::new(MockBackend::failing("unhealthy")));

        let health = orchestrator.health_check_all().await;

        assert_eq!(health.get("healthy"), Some(&true));
        assert_eq!(health.get("unhealthy"), Some(&false));
    }
}
