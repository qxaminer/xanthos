//! # PII Sanitization Layer
//!
//! Auto-strips personally identifiable information from input files
//! before feeding to the orchestrator. Combines:
//! - Regex patterns for structured PII (emails, phones, SSNs, etc.)
//! - Semantic NER for names and places
//!
//! Operates on .json files or any text files via glob patterns.

pub mod patterns;
pub mod semantic;
pub mod scanner;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SanitizeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Pattern error: {0}")]
    Pattern(#[from] regex::Error),

    #[error("Glob pattern error: {0}")]
    Glob(#[from] glob::PatternError),
}

pub type Result<T> = std::result::Result<T, SanitizeError>;

/// Categories of PII that can be detected and redacted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PiiCategory {
    Email,
    Phone,
    Ssn,
    CreditCard,
    IpAddress,
    Name,
    Place,
    Address,
    Date,
    Custom,
}

impl PiiCategory {
    pub fn redaction_token(&self) -> &'static str {
        match self {
            PiiCategory::Email => "[EMAIL]",
            PiiCategory::Phone => "[PHONE]",
            PiiCategory::Ssn => "[SSN]",
            PiiCategory::CreditCard => "[CARD]",
            PiiCategory::IpAddress => "[IP]",
            PiiCategory::Name => "[NAME]",
            PiiCategory::Place => "[PLACE]",
            PiiCategory::Address => "[ADDRESS]",
            PiiCategory::Date => "[DATE]",
            PiiCategory::Custom => "[REDACTED]",
        }
    }
}

/// A detected PII instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    pub category: PiiCategory,
    pub original: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
    pub method: DetectionMethod,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DetectionMethod {
    Regex,
    Semantic,
    Hybrid,
}

/// Configuration for the sanitizer
#[derive(Debug, Clone)]
pub struct SanitizerConfig {
    pub enable_regex: bool,
    pub enable_semantic: bool,
    pub min_confidence: f32,
    pub preserve_format: bool,
    pub categories: Vec<PiiCategory>,
    pub custom_patterns: Vec<(String, PiiCategory)>,
    pub allowlist: Vec<String>,
}

impl Default for SanitizerConfig {
    fn default() -> Self {
        Self {
            enable_regex: true,
            enable_semantic: true,
            min_confidence: 0.7,
            preserve_format: true,
            categories: vec![
                PiiCategory::Email,
                PiiCategory::Phone,
                PiiCategory::Ssn,
                PiiCategory::CreditCard,
                PiiCategory::IpAddress,
                PiiCategory::Name,
                PiiCategory::Place,
            ],
            custom_patterns: Vec::new(),
            allowlist: Vec::new(),
        }
    }
}

/// Main sanitizer that combines regex and semantic detection
pub struct Sanitizer {
    config: SanitizerConfig,
    regex_detector: patterns::RegexDetector,
    semantic_detector: semantic::SemanticDetector,
}

impl Sanitizer {
    pub fn new(config: SanitizerConfig) -> Result<Self> {
        let regex_detector = patterns::RegexDetector::new(&config)?;
        let semantic_detector = semantic::SemanticDetector::new(&config);

        Ok(Self {
            config,
            regex_detector,
            semantic_detector,
        })
    }

    pub fn with_defaults() -> Result<Self> {
        Self::new(SanitizerConfig::default())
    }

    /// Detect all PII in text
    pub fn detect(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        if self.config.enable_regex {
            matches.extend(self.regex_detector.detect(text));
        }

        if self.config.enable_semantic {
            let semantic_matches = self.semantic_detector.detect(text);
            // Merge semantic matches, avoiding duplicates
            for sm in semantic_matches {
                if !self.overlaps_existing(&matches, &sm) {
                    matches.push(sm);
                }
            }
        }

        // Filter by confidence
        matches.retain(|m| m.confidence >= self.config.min_confidence);

        // Filter allowlisted terms
        matches.retain(|m| !self.config.allowlist.iter().any(|a|
            m.original.to_lowercase() == a.to_lowercase()
        ));

        // Sort by position
        matches.sort_by_key(|m| m.start);

        matches
    }

    /// Sanitize text by replacing PII with redaction tokens
    pub fn sanitize(&self, text: &str) -> SanitizedOutput {
        let matches = self.detect(text);
        let mut result = String::with_capacity(text.len());
        let mut last_end = 0;
        let mut redactions = Vec::new();

        for m in &matches {
            // Add text before this match
            if m.start > last_end {
                result.push_str(&text[last_end..m.start]);
            }

            // Add redaction token
            let token = m.category.redaction_token();
            result.push_str(token);

            redactions.push(Redaction {
                category: m.category,
                token: token.to_string(),
                position: result.len() - token.len(),
            });

            last_end = m.end;
        }

        // Add remaining text
        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }

        SanitizedOutput {
            text: result,
            redactions,
            original_length: text.len(),
            pii_count: matches.len(),
        }
    }

    /// Sanitize a JSON value recursively
    pub fn sanitize_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                serde_json::Value::String(self.sanitize(s).text)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(
                    arr.iter().map(|v| self.sanitize_json(v)).collect()
                )
            }
            serde_json::Value::Object(obj) => {
                serde_json::Value::Object(
                    obj.iter()
                        .map(|(k, v)| (k.clone(), self.sanitize_json(v)))
                        .collect()
                )
            }
            other => other.clone(),
        }
    }

    fn overlaps_existing(&self, existing: &[PiiMatch], new: &PiiMatch) -> bool {
        existing.iter().any(|e| {
            (new.start >= e.start && new.start < e.end) ||
            (new.end > e.start && new.end <= e.end) ||
            (new.start <= e.start && new.end >= e.end)
        })
    }
}

/// Output from sanitization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizedOutput {
    pub text: String,
    pub redactions: Vec<Redaction>,
    pub original_length: usize,
    pub pii_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Redaction {
    pub category: PiiCategory,
    pub token: String,
    pub position: usize,
}

/// Statistics from a sanitization run
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SanitizeStats {
    pub files_processed: usize,
    pub total_pii_found: usize,
    pub by_category: HashMap<PiiCategory, usize>,
    pub by_method: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_detection() {
        let sanitizer = Sanitizer::with_defaults().unwrap();
        let text = "Contact john.doe@example.com for details";
        let output = sanitizer.sanitize(text);
        assert!(output.text.contains("[EMAIL]"));
        assert!(!output.text.contains("john.doe@example.com"));
    }

    #[test]
    fn test_phone_detection() {
        let sanitizer = Sanitizer::with_defaults().unwrap();
        let text = "Call me at 555-123-4567";
        let output = sanitizer.sanitize(text);
        assert!(output.text.contains("[PHONE]"));
    }
}
