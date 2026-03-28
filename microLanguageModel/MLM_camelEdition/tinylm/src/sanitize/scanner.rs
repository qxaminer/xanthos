//! # File Scanner
//!
//! Scans files matching glob patterns, sanitizes PII, outputs clean data.
//! Supports:
//! - JSON files (.json) - recursive value sanitization
//! - Text files (.*) - line-by-line sanitization
//! - Streaming for large files

use glob::glob;
use std::fs;
use std::path::{Path, PathBuf};

use super::{Result, SanitizeStats, Sanitizer, SanitizerConfig, SanitizedOutput};

/// File scanner configuration
#[derive(Debug, Clone)]
pub struct ScannerConfig {
    pub patterns: Vec<String>,
    pub recursive: bool,
    pub output_dir: Option<PathBuf>,
    pub in_place: bool,
    pub dry_run: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            patterns: vec!["**/*.json".to_string()],
            recursive: true,
            output_dir: None,
            in_place: false,
            dry_run: false,
        }
    }
}

/// Scans and sanitizes files matching patterns
pub struct Scanner {
    sanitizer: Sanitizer,
    config: ScannerConfig,
}

impl Scanner {
    pub fn new(sanitizer_config: SanitizerConfig, scanner_config: ScannerConfig) -> Result<Self> {
        let sanitizer = Sanitizer::new(sanitizer_config)?;
        Ok(Self {
            sanitizer,
            config: scanner_config,
        })
    }

    pub fn with_defaults() -> Result<Self> {
        Self::new(SanitizerConfig::default(), ScannerConfig::default())
    }

    /// Scan all files matching configured patterns
    pub fn scan(&self, base_path: &Path) -> Result<ScanResult> {
        let mut result = ScanResult::default();

        for pattern in &self.config.patterns {
            let full_pattern = base_path.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            for entry in glob(&pattern_str)? {
                match entry {
                    Ok(path) => {
                        if path.is_file() {
                            match self.scan_file(&path) {
                                Ok(file_result) => {
                                    result.stats.files_processed += 1;
                                    result.stats.total_pii_found += file_result.pii_count;
                                    result.files.push(file_result);
                                }
                                Err(e) => {
                                    result.errors.push((path, e.to_string()));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Glob error: {}", e);
                    }
                }
            }
        }

        Ok(result)
    }

    /// Scan a single file
    pub fn scan_file(&self, path: &Path) -> Result<FileResult> {
        let content = fs::read_to_string(path)?;
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let (sanitized, pii_count) = match extension {
            "json" => self.scan_json(&content)?,
            _ => self.scan_text(&content),
        };

        let mut file_result = FileResult {
            path: path.to_path_buf(),
            original_size: content.len(),
            sanitized_size: sanitized.len(),
            pii_count,
            sanitized_content: None,
        };

        if !self.config.dry_run {
            if self.config.in_place {
                fs::write(path, &sanitized)?;
            } else if let Some(ref output_dir) = self.config.output_dir {
                let relative = path.file_name().unwrap_or_default();
                let output_path = output_dir.join(relative);
                fs::create_dir_all(output_dir)?;
                fs::write(&output_path, &sanitized)?;
                file_result.sanitized_content = Some(output_path);
            } else {
                file_result.sanitized_content = None;
            }
        }

        Ok(file_result)
    }

    /// Scan JSON content
    fn scan_json(&self, content: &str) -> Result<(String, usize)> {
        let value: serde_json::Value = serde_json::from_str(content)?;
        let sanitized_value = self.sanitizer.sanitize_json(&value);

        // Count PII by comparing
        let pii_count = count_redactions(&sanitized_value);

        let output = serde_json::to_string_pretty(&sanitized_value)?;
        Ok((output, pii_count))
    }

    /// Scan plain text content
    fn scan_text(&self, content: &str) -> (String, usize) {
        let output = self.sanitizer.sanitize(content);
        (output.text, output.pii_count)
    }

    /// Scan a JSON string directly (for orchestrator integration)
    pub fn sanitize_json_str(&self, json_str: &str) -> Result<String> {
        let value: serde_json::Value = serde_json::from_str(json_str)?;
        let sanitized = self.sanitizer.sanitize_json(&value);
        Ok(serde_json::to_string(&sanitized)?)
    }

    /// Scan text directly (for orchestrator integration)
    pub fn sanitize_text(&self, text: &str) -> SanitizedOutput {
        self.sanitizer.sanitize(text)
    }
}

/// Count redaction tokens in a JSON value
fn count_redactions(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::String(s) => {
            s.matches("[EMAIL]").count()
                + s.matches("[PHONE]").count()
                + s.matches("[SSN]").count()
                + s.matches("[CARD]").count()
                + s.matches("[IP]").count()
                + s.matches("[NAME]").count()
                + s.matches("[PLACE]").count()
                + s.matches("[ADDRESS]").count()
                + s.matches("[DATE]").count()
                + s.matches("[REDACTED]").count()
        }
        serde_json::Value::Array(arr) => arr.iter().map(count_redactions).sum(),
        serde_json::Value::Object(obj) => obj.values().map(count_redactions).sum(),
        _ => 0,
    }
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub stats: SanitizeStats,
    pub files: Vec<FileResult>,
    pub errors: Vec<(PathBuf, String)>,
}

#[derive(Debug)]
pub struct FileResult {
    pub path: PathBuf,
    pub original_size: usize,
    pub sanitized_size: usize,
    pub pii_count: usize,
    pub sanitized_content: Option<PathBuf>,
}

/// Builder for creating scanner with custom configuration
pub struct ScannerBuilder {
    sanitizer_config: SanitizerConfig,
    scanner_config: ScannerConfig,
}

impl ScannerBuilder {
    pub fn new() -> Self {
        Self {
            sanitizer_config: SanitizerConfig::default(),
            scanner_config: ScannerConfig::default(),
        }
    }

    pub fn patterns(mut self, patterns: Vec<String>) -> Self {
        self.scanner_config.patterns = patterns;
        self
    }

    pub fn json_only(mut self) -> Self {
        self.scanner_config.patterns = vec!["**/*.json".to_string()];
        self
    }

    pub fn all_files(mut self) -> Self {
        self.scanner_config.patterns = vec!["**/*".to_string()];
        self
    }

    pub fn output_dir(mut self, dir: PathBuf) -> Self {
        self.scanner_config.output_dir = Some(dir);
        self
    }

    pub fn in_place(mut self) -> Self {
        self.scanner_config.in_place = true;
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.scanner_config.dry_run = true;
        self
    }

    pub fn min_confidence(mut self, confidence: f32) -> Self {
        self.sanitizer_config.min_confidence = confidence;
        self
    }

    pub fn disable_semantic(mut self) -> Self {
        self.sanitizer_config.enable_semantic = false;
        self
    }

    pub fn disable_regex(mut self) -> Self {
        self.sanitizer_config.enable_regex = false;
        self
    }

    pub fn allowlist(mut self, terms: Vec<String>) -> Self {
        self.sanitizer_config.allowlist = terms;
        self
    }

    pub fn build(self) -> Result<Scanner> {
        Scanner::new(self.sanitizer_config, self.scanner_config)
    }
}

impl Default for ScannerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scan_json_file() {
        let temp = TempDir::new().unwrap();
        let json_path = temp.path().join("test.json");

        let content = r#"{"name": "John Smith", "email": "john@example.com"}"#;
        fs::write(&json_path, content).unwrap();

        let scanner = Scanner::with_defaults().unwrap();
        let result = scanner.scan_file(&json_path).unwrap();

        assert!(result.pii_count > 0);
    }

    #[test]
    fn test_scan_text_file() {
        let temp = TempDir::new().unwrap();
        let txt_path = temp.path().join("test.txt");

        let content = "Contact Dr. Jane Doe at jane.doe@example.com";
        fs::write(&txt_path, content).unwrap();

        let scanner = ScannerBuilder::new()
            .patterns(vec!["*.txt".to_string()])
            .build()
            .unwrap();

        let result = scanner.scan_file(&txt_path).unwrap();
        assert!(result.pii_count > 0);
    }

    #[test]
    fn test_builder() {
        let scanner = ScannerBuilder::new()
            .json_only()
            .min_confidence(0.8)
            .dry_run()
            .allowlist(vec!["Acme Corp".to_string()])
            .build()
            .unwrap();

        let output = scanner.sanitize_text("Contact john@acme.com");
        assert!(output.text.contains("[EMAIL]"));
    }
}
