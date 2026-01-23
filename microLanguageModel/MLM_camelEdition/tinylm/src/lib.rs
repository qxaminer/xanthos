//! # tinyLM
//!
//! Multi-LLM orchestrator with local inference and frontier API backends.
//!
//! ## Features
//! - wgpu compute for local models (Metal/Vulkan/DX12)
//! - Frontier backends: Claude, GPT-4, Gemini, Mistral
//! - Blind testing infrastructure
//! - PII sanitization layer
//!
//! ## Quick Start
//!
//! ```no_run
//! use tinylm::sanitize::{Sanitizer, SanitizerConfig};
//!
//! let sanitizer = Sanitizer::with_defaults().unwrap();
//! let output = sanitizer.sanitize("Contact john.doe@example.com");
//! assert!(output.text.contains("[EMAIL]"));
//! ```

pub mod sanitize;
pub mod backends;
pub mod orchestrate;

// Future modules (Phase 2+)
// pub mod arena;
// pub mod compute;
// pub mod stable;
// pub mod testing;

/// Re-exports for convenience
pub use sanitize::{
    Sanitizer,
    SanitizerConfig,
    SanitizedOutput,
    PiiCategory,
    PiiMatch,
    scanner::{Scanner, ScannerBuilder, ScanResult},
};

pub use backends::{
    Backend,
    BackendError,
    BackendRegistry,
    CompletionRequest,
    CompletionResponse,
    Message,
    Role,
    Usage,
    claude::ClaudeBackend,
    openai::OpenAIBackend,
    gemini::GeminiBackend,
};

pub use orchestrate::{
    Orchestrator,
    OrchestratorConfig,
    OrchestratorError,
    BlindResponse,
    BlindTestResult,
};
