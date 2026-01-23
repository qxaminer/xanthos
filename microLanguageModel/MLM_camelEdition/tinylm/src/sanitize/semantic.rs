//! # Semantic NER Detection (Pure Regex Heuristics)
//!
//! Detects names and places using contextual patterns only.
//! No dictionaries - relies on linguistic structure.
//!
//! Patterns:
//! - Titles before capitalized words (Dr. Smith)
//! - Attribution patterns (John said)
//! - Possessives (Sarah's car)
//! - Geographic prepositions (in Paris, from Tokyo)
//! - Adjacent capitalized words (likely full names)

use lazy_static::lazy_static;
use regex::Regex;

use super::{DetectionMethod, PiiCategory, PiiMatch, SanitizerConfig};

lazy_static! {
    // === NAME PATTERNS ===

    // Titled names: Mr., Mrs., Ms., Dr., Prof., Rev., etc.
    static ref TITLED_NAME: Regex = Regex::new(
        r"(?i)\b(?:Mr|Mrs|Ms|Miss|Dr|Prof|Professor|Rev|Reverend|Hon|Honorable|Sir|Dame|Lord|Lady|Sen|Senator|Rep|Representative|Gov|Governor|Pres|President|Gen|General|Col|Colonel|Capt|Captain|Lt|Lieutenant|Sgt|Sergeant)\.?\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,2})"
    ).unwrap();

    // Attribution: "Name said/asked/replied/wrote/explained/stated"
    static ref ATTRIBUTION_NAME: Regex = Regex::new(
        r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\s+(?:said|says|asked|asks|replied|replies|wrote|writes|explained|explains|stated|states|added|adds|noted|notes|told|tells|shouted|whispered|exclaimed|announced|declared|mentioned|commented|observed|remarked|continued|answered|responded|argued|claimed|suggested|insisted|admitted|confirmed|denied|warned|promised|agreed|disagreed)\b"
    ).unwrap();

    // Explicit naming: "named X", "called X", "known as X"
    static ref EXPLICIT_NAME: Regex = Regex::new(
        r"(?i)\b(?:named|called|known\s+as|nicknamed|dubbed|entitled)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,2})"
    ).unwrap();

    // Possessive names: "John's", "Mary's" (followed by noun-like word)
    static ref POSSESSIVE_NAME: Regex = Regex::new(
        r"\b([A-Z][a-z]+)'s\s+(?:[a-z]+)"
    ).unwrap();

    // Two adjacent capitalized words (high probability of full name)
    // Excludes sentence starts by requiring preceding lowercase/punctuation
    static ref ADJACENT_CAPS: Regex = Regex::new(
        r"(?:^|[.!?]\s+|,\s*|\band\s+|\bor\s+|\bwith\s+|\bby\s+|\bfor\s+|\bto\s+|\bfrom\s+)([A-Z][a-z]+\s+[A-Z][a-z]+)(?:\s+[a-z]|\s*[,.]|\s*$)"
    ).unwrap();

    // Name in quotes: "John" or 'Mary'
    static ref QUOTED_NAME: Regex = Regex::new(
        r#"["']([A-Z][a-z]+)["']"#
    ).unwrap();

    // Email-derived name: john.doe@ or john_doe@
    static ref EMAIL_NAME: Regex = Regex::new(
        r"(?i)\b([a-z]+)[._]([a-z]+)@"
    ).unwrap();

    // Greeting patterns: "Hi John", "Dear Mary", "Hello Dr. Smith"
    static ref GREETING_NAME: Regex = Regex::new(
        r"(?i)\b(?:hi|hello|hey|dear|greetings|welcome)\s+(?:(?:Mr|Mrs|Ms|Dr)\.?\s+)?([A-Z][a-z]+)"
    ).unwrap();

    // Signature patterns: "Sincerely, John" "Best, Mary Smith"
    static ref SIGNATURE_NAME: Regex = Regex::new(
        r"(?i)(?:sincerely|regards|best|thanks|cheers|yours|respectfully|cordially)[,\s]+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)"
    ).unwrap();

    // === PLACE PATTERNS ===

    // Geographic prepositions: in/at/from/to/near + Capitalized
    static ref GEO_PREPOSITION: Regex = Regex::new(
        r"(?i)\b(?:in|at|from|to|near|around|outside|inside|within|throughout|across|toward|towards)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?(?:,\s*[A-Z]{2})?)"
    ).unwrap();

    // Movement verbs: traveled to, moved to, flew to, drove to
    static ref MOVEMENT_PLACE: Regex = Regex::new(
        r"(?i)\b(?:traveled|travelling|traveling|moved|moving|flew|flying|drove|driving|walked|walking|went|going|headed|heading|relocated|relocating|visited|visiting|arrived|arriving|departed|departing|left|leaving)\s+(?:to|from|in|at)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)"
    ).unwrap();

    // Residence: lived in, born in, raised in, based in
    static ref RESIDENCE_PLACE: Regex = Regex::new(
        r"(?i)\b(?:live[ds]?|living|born|raised|based|located|situated|reside[ds]?|residing|stay(?:ed|ing)?|settled?)\s+(?:in|at|near)\s+([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)"
    ).unwrap();

    // City, State pattern: "Austin, TX" or "New York, NY"
    static ref CITY_STATE: Regex = Regex::new(
        r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?),\s*([A-Z]{2})\b"
    ).unwrap();

    // Country indicators: "in the UK", "from Japan", "to Germany"
    static ref COUNTRY_INDICATOR: Regex = Regex::new(
        r"(?i)\b(?:in|from|to|of)\s+(?:the\s+)?([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\s+(?:government|embassy|border|coast|capital|president|prime\s+minister|people|citizens|nationals)"
    ).unwrap();

    // Address-like: "Street/Ave/Blvd" preceded by name
    static ref STREET_NAME: Regex = Regex::new(
        r"\b([A-Z][a-z]+(?:\s+[A-Z][a-z]+)?)\s+(?:Street|St|Avenue|Ave|Boulevard|Blvd|Road|Rd|Drive|Dr|Lane|Ln|Way|Court|Ct|Place|Pl|Circle|Cir|Highway|Hwy|Parkway|Pkwy)\b"
    ).unwrap();

    // Words to exclude (not names/places despite capitalization)
    static ref EXCLUDE_WORDS: Regex = Regex::new(
        r"(?i)^(?:Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday|January|February|March|April|May|June|July|August|September|October|November|December|The|A|An|And|Or|But|If|Then|When|Where|What|Who|Why|How|This|That|These|Those|I|You|He|She|It|We|They|My|Your|His|Her|Its|Our|Their|Chapter|Section|Figure|Table|Appendix|Part|Volume|Page|Internet|Web|Email|Online|Digital|Software|Hardware|Database|API|URL|HTTP|HTML|CSS|JSON|XML|PDF|README|TODO|FIXME|NOTE|WARNING|ERROR|OK|NULL|TRUE|FALSE|Yes|No)$"
    ).unwrap();
}

pub struct SemanticDetector {
    enabled_categories: Vec<PiiCategory>,
    min_confidence: f32,
}

impl SemanticDetector {
    pub fn new(config: &SanitizerConfig) -> Self {
        Self {
            enabled_categories: config.categories.clone(),
            min_confidence: config.min_confidence,
        }
    }

    pub fn detect(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        if self.enabled_categories.contains(&PiiCategory::Name) {
            matches.extend(self.detect_names(text));
        }

        if self.enabled_categories.contains(&PiiCategory::Place) {
            matches.extend(self.detect_places(text));
        }

        // Filter excluded words and low confidence
        matches.retain(|m| {
            !EXCLUDE_WORDS.is_match(&m.original) && m.confidence >= self.min_confidence
        });

        matches
    }

    fn detect_names(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        // Titled names (highest confidence)
        for cap in TITLED_NAME.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                matches.push(PiiMatch {
                    category: PiiCategory::Name,
                    original: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                    confidence: 0.95,
                    method: DetectionMethod::Semantic,
                });
            }
        }

        // Attribution names
        for cap in ATTRIBUTION_NAME.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Name,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.90,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Explicit naming
        for cap in EXPLICIT_NAME.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Name,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.90,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Possessive names
        for cap in POSSESSIVE_NAME.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Name,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.80,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Greeting names
        for cap in GREETING_NAME.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Name,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.85,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Signature names
        for cap in SIGNATURE_NAME.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Name,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.85,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Adjacent caps (lower confidence - could be many things)
        for cap in ADJACENT_CAPS.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Name,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.70,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Email-derived names
        for cap in EMAIL_NAME.captures_iter(text) {
            let first = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let last = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            if !first.is_empty() && !last.is_empty() && first.len() > 1 && last.len() > 1 {
                let full_match = cap.get(0).unwrap();
                let name = format!("{} {}", capitalize(first), capitalize(last));
                matches.push(PiiMatch {
                    category: PiiCategory::Name,
                    original: name,
                    start: full_match.start(),
                    end: full_match.end() - 1, // exclude @
                    confidence: 0.75,
                    method: DetectionMethod::Semantic,
                });
            }
        }

        matches
    }

    fn detect_places(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        // City, State pattern (highest confidence for places)
        for cap in CITY_STATE.captures_iter(text) {
            if let (Some(city), Some(state)) = (cap.get(1), cap.get(2)) {
                let full = format!("{}, {}", city.as_str(), state.as_str());
                matches.push(PiiMatch {
                    category: PiiCategory::Place,
                    original: full,
                    start: city.start(),
                    end: state.end(),
                    confidence: 0.95,
                    method: DetectionMethod::Semantic,
                });
            }
        }

        // Residence places
        for cap in RESIDENCE_PLACE.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Place,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.90,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Movement places
        for cap in MOVEMENT_PLACE.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    matches.push(PiiMatch {
                        category: PiiCategory::Place,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.85,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Geographic prepositions
        for cap in GEO_PREPOSITION.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                if !self.already_matched(&matches, m.start(), m.end()) {
                    // Lower confidence - "in" can precede many things
                    matches.push(PiiMatch {
                        category: PiiCategory::Place,
                        original: m.as_str().to_string(),
                        start: m.start(),
                        end: m.end(),
                        confidence: 0.70,
                        method: DetectionMethod::Semantic,
                    });
                }
            }
        }

        // Street names
        for cap in STREET_NAME.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                matches.push(PiiMatch {
                    category: PiiCategory::Place,
                    original: m.as_str().to_string(),
                    start: m.start(),
                    end: m.end(),
                    confidence: 0.85,
                    method: DetectionMethod::Semantic,
                });
            }
        }

        matches
    }

    fn already_matched(&self, matches: &[PiiMatch], start: usize, end: usize) -> bool {
        matches.iter().any(|m| {
            (start >= m.start && start < m.end) ||
            (end > m.start && end <= m.end) ||
            (start <= m.start && end >= m.end)
        })
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detector() -> SemanticDetector {
        SemanticDetector::new(&SanitizerConfig::default())
    }

    #[test]
    fn test_titled_name() {
        let d = detector();
        let matches = d.detect("Dr. Sarah Connor presented the research");
        // Should detect Sarah Connor as a name after title
        assert!(matches.iter().any(|m|
            m.category == PiiCategory::Name &&
            m.original.contains("Sarah")
        ));
    }

    #[test]
    fn test_attribution() {
        let d = detector();
        let matches = d.detect("John Smith said the project was complete");
        assert!(matches.iter().any(|m| m.original == "John Smith"));
    }

    #[test]
    fn test_possessive() {
        let d = detector();
        let matches = d.detect("We borrowed Michael's car yesterday");
        assert!(matches.iter().any(|m| m.original == "Michael"));
    }

    #[test]
    fn test_city_state() {
        let d = detector();
        let matches = d.detect("The office is in Austin, TX");
        assert!(matches.iter().any(|m| m.original == "Austin, TX"));
    }

    #[test]
    fn test_residence() {
        let d = detector();
        let matches = d.detect("She lived in San Francisco for years");
        assert!(matches.iter().any(|m| m.original.contains("San Francisco")));
    }

    #[test]
    fn test_excludes_common_words() {
        let d = detector();
        let matches = d.detect("On Monday in January the meeting started");
        assert!(matches.iter().all(|m| m.original != "Monday"));
        assert!(matches.iter().all(|m| m.original != "January"));
    }
}
