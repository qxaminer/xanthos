//! # Regex-based PII Detection
//!
//! Patterns for detecting structured PII:
//! - Email addresses
//! - Phone numbers (US, international)
//! - Social Security Numbers
//! - Credit card numbers
//! - IP addresses (v4, v6)
//! - Street addresses
//! - Dates (various formats)

use lazy_static::lazy_static;
use regex::Regex;

use super::{DetectionMethod, PiiCategory, PiiMatch, Result, SanitizerConfig};

lazy_static! {
    // Email: RFC 5322 simplified
    static ref EMAIL_PATTERN: Regex = Regex::new(
        r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}"
    ).unwrap();

    // Phone: US formats + international
    static ref PHONE_PATTERN: Regex = Regex::new(
        r"(?x)
        (?:
            # US: (555) 123-4567, 555-123-4567, 555.123.4567
            \(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}
            |
            # International: +1-555-123-4567
            \+\d{1,3}[-.\s]?\(?\d{1,4}\)?[-.\s]?\d{1,4}[-.\s]?\d{1,9}
        )"
    ).unwrap();

    // SSN: 123-45-6789 or 123456789
    static ref SSN_PATTERN: Regex = Regex::new(
        r"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b"
    ).unwrap();

    // Credit Card: Major formats (Visa, MC, Amex, Discover)
    static ref CREDIT_CARD_PATTERN: Regex = Regex::new(
        r"(?x)
        \b(?:
            # Visa
            4\d{3}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}
            |
            # Mastercard
            5[1-5]\d{2}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}
            |
            # Amex
            3[47]\d{2}[-\s]?\d{6}[-\s]?\d{5}
            |
            # Discover
            6(?:011|5\d{2})[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}
        )\b"
    ).unwrap();

    // IPv4
    static ref IPV4_PATTERN: Regex = Regex::new(
        r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b"
    ).unwrap();

    // IPv6 (simplified)
    static ref IPV6_PATTERN: Regex = Regex::new(
        r"(?i)\b(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}\b|(?:[0-9a-f]{1,4}:){1,7}:|(?:[0-9a-f]{1,4}:){1,6}:[0-9a-f]{1,4}"
    ).unwrap();

    // US Street Address (simplified)
    static ref ADDRESS_PATTERN: Regex = Regex::new(
        r"(?i)\b\d{1,5}\s+(?:[A-Z][a-z]+\s*){1,4}(?:Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Drive|Dr|Lane|Ln|Way|Court|Ct|Circle|Cir|Place|Pl)\b\.?"
    ).unwrap();

    // Dates: MM/DD/YYYY, YYYY-MM-DD, Month DD, YYYY
    static ref DATE_PATTERN: Regex = Regex::new(
        r"(?x)
        (?:
            # MM/DD/YYYY or MM-DD-YYYY
            \b(?:0?[1-9]|1[0-2])[-/](?:0?[1-9]|[12]\d|3[01])[-/](?:19|20)\d{2}\b
            |
            # YYYY-MM-DD (ISO)
            \b(?:19|20)\d{2}[-/](?:0?[1-9]|1[0-2])[-/](?:0?[1-9]|[12]\d|3[01])\b
            |
            # Month DD, YYYY
            \b(?:January|February|March|April|May|June|July|August|September|October|November|December|Jan|Feb|Mar|Apr|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\.?\s+\d{1,2}(?:st|nd|rd|th)?,?\s+(?:19|20)\d{2}\b
        )"
    ).unwrap();

    // ZIP codes (US)
    static ref ZIP_PATTERN: Regex = Regex::new(
        r"\b\d{5}(?:-\d{4})?\b"
    ).unwrap();
}

pub struct RegexDetector {
    enabled_categories: Vec<PiiCategory>,
    custom_patterns: Vec<(Regex, PiiCategory)>,
}

impl RegexDetector {
    pub fn new(config: &SanitizerConfig) -> Result<Self> {
        let mut custom_patterns = Vec::new();
        for (pattern, category) in &config.custom_patterns {
            let re = Regex::new(pattern)?;
            custom_patterns.push((re, *category));
        }

        Ok(Self {
            enabled_categories: config.categories.clone(),
            custom_patterns,
        })
    }

    pub fn detect(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        // Email
        if self.enabled_categories.contains(&PiiCategory::Email) {
            for cap in EMAIL_PATTERN.find_iter(text) {
                matches.push(PiiMatch {
                    category: PiiCategory::Email,
                    original: cap.as_str().to_string(),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: 0.95,
                    method: DetectionMethod::Regex,
                });
            }
        }

        // Phone
        if self.enabled_categories.contains(&PiiCategory::Phone) {
            for cap in PHONE_PATTERN.find_iter(text) {
                // Skip if it looks like a year or short number
                let s = cap.as_str();
                if s.len() >= 10 || s.contains('-') || s.contains('(') {
                    matches.push(PiiMatch {
                        category: PiiCategory::Phone,
                        original: s.to_string(),
                        start: cap.start(),
                        end: cap.end(),
                        confidence: 0.90,
                        method: DetectionMethod::Regex,
                    });
                }
            }
        }

        // SSN
        if self.enabled_categories.contains(&PiiCategory::Ssn) {
            for cap in SSN_PATTERN.find_iter(text) {
                // Validate SSN format (not starting with 000, 666, or 900-999)
                let digits: String = cap.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() == 9 {
                    let area: u32 = digits[0..3].parse().unwrap_or(0);
                    if area != 0 && area != 666 && area < 900 {
                        matches.push(PiiMatch {
                            category: PiiCategory::Ssn,
                            original: cap.as_str().to_string(),
                            start: cap.start(),
                            end: cap.end(),
                            confidence: 0.85,
                            method: DetectionMethod::Regex,
                        });
                    }
                }
            }
        }

        // Credit Card
        if self.enabled_categories.contains(&PiiCategory::CreditCard) {
            for cap in CREDIT_CARD_PATTERN.find_iter(text) {
                let digits: String = cap.as_str().chars().filter(|c| c.is_ascii_digit()).collect();
                if luhn_check(&digits) {
                    matches.push(PiiMatch {
                        category: PiiCategory::CreditCard,
                        original: cap.as_str().to_string(),
                        start: cap.start(),
                        end: cap.end(),
                        confidence: 0.95,
                        method: DetectionMethod::Regex,
                    });
                }
            }
        }

        // IP Address
        if self.enabled_categories.contains(&PiiCategory::IpAddress) {
            for cap in IPV4_PATTERN.find_iter(text) {
                matches.push(PiiMatch {
                    category: PiiCategory::IpAddress,
                    original: cap.as_str().to_string(),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: 0.90,
                    method: DetectionMethod::Regex,
                });
            }
            for cap in IPV6_PATTERN.find_iter(text) {
                matches.push(PiiMatch {
                    category: PiiCategory::IpAddress,
                    original: cap.as_str().to_string(),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: 0.90,
                    method: DetectionMethod::Regex,
                });
            }
        }

        // Address
        if self.enabled_categories.contains(&PiiCategory::Address) {
            for cap in ADDRESS_PATTERN.find_iter(text) {
                matches.push(PiiMatch {
                    category: PiiCategory::Address,
                    original: cap.as_str().to_string(),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: 0.80,
                    method: DetectionMethod::Regex,
                });
            }
        }

        // Date
        if self.enabled_categories.contains(&PiiCategory::Date) {
            for cap in DATE_PATTERN.find_iter(text) {
                matches.push(PiiMatch {
                    category: PiiCategory::Date,
                    original: cap.as_str().to_string(),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: 0.85,
                    method: DetectionMethod::Regex,
                });
            }
        }

        // Custom patterns
        for (pattern, category) in &self.custom_patterns {
            for cap in pattern.find_iter(text) {
                matches.push(PiiMatch {
                    category: *category,
                    original: cap.as_str().to_string(),
                    start: cap.start(),
                    end: cap.end(),
                    confidence: 0.80,
                    method: DetectionMethod::Regex,
                });
            }
        }

        matches
    }
}

/// Luhn algorithm for credit card validation
fn luhn_check(digits: &str) -> bool {
    let mut sum = 0;
    let mut alternate = false;

    for c in digits.chars().rev() {
        if let Some(mut n) = c.to_digit(10) {
            if alternate {
                n *= 2;
                if n > 9 {
                    n -= 9;
                }
            }
            sum += n;
            alternate = !alternate;
        } else {
            return false;
        }
    }

    sum % 10 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detector() -> RegexDetector {
        RegexDetector::new(&SanitizerConfig::default()).unwrap()
    }

    #[test]
    fn test_email() {
        let d = detector();
        let matches = d.detect("Email: test@example.com");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].category, PiiCategory::Email);
        assert_eq!(matches[0].original, "test@example.com");
    }

    #[test]
    fn test_phone_formats() {
        let d = detector();

        let matches = d.detect("Call (555) 123-4567");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].category, PiiCategory::Phone);

        let matches = d.detect("Call 555-123-4567");
        assert_eq!(matches.len(), 1);

        let matches = d.detect("Call +1-555-123-4567");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_ssn() {
        let d = detector();
        let matches = d.detect("SSN: 123-45-6789");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].category, PiiCategory::Ssn);
    }

    #[test]
    fn test_credit_card_luhn() {
        assert!(luhn_check("4532015112830366")); // Valid Visa
        assert!(!luhn_check("4532015112830367")); // Invalid
    }

    #[test]
    fn test_ipv4() {
        let d = detector();
        let matches = d.detect("Server at 192.168.1.1");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].category, PiiCategory::IpAddress);
    }
}
