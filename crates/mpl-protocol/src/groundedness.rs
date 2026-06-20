//! Groundedness Verification
//!
//! Implements hybrid groundedness checking for AI responses:
//! - Local citation matching (fast, deterministic)
//! - LLM-based verification (for uncertain cases)
//!
//! Groundedness measures whether claims in a response are supported by
//! provided sources/citations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::debug;

/// Configuration for groundedness checking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundednessConfig {
    /// Minimum similarity threshold for local matching (0.0 - 1.0)
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f64,

    /// Whether to use LLM for uncertain cases
    #[serde(default)]
    pub use_llm_fallback: bool,

    /// Minimum confidence to consider a claim grounded without LLM
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: f64,

    /// Extract claims from response automatically
    #[serde(default = "default_true")]
    pub auto_extract_claims: bool,
}

fn default_similarity_threshold() -> f64 {
    0.7
}

fn default_confidence_threshold() -> f64 {
    0.8
}

fn default_true() -> bool {
    true
}

impl Default for GroundednessConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: default_similarity_threshold(),
            use_llm_fallback: false,
            confidence_threshold: default_confidence_threshold(),
            auto_extract_claims: true,
        }
    }
}

/// A source document for grounding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocument {
    /// Unique identifier
    pub id: String,

    /// Source content
    pub content: String,

    /// Optional title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Optional URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Relevance score (if from retrieval)
    #[serde(default = "default_relevance")]
    pub relevance: f64,
}

fn default_relevance() -> f64 {
    1.0
}

/// A claim extracted from a response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// The claim text
    pub text: String,

    /// Start position in original text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<usize>,

    /// End position in original text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<usize>,

    /// Claim type (factual, opinion, etc.)
    #[serde(default)]
    pub claim_type: ClaimType,
}

/// Types of claims
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    /// Factual claim that should be grounded
    #[default]
    Factual,
    /// Opinion or subjective statement
    Opinion,
    /// Common knowledge (doesn't need grounding)
    CommonKnowledge,
    /// Procedural/instructional content
    Procedural,
}

/// Result of grounding a single claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimGroundingResult {
    /// The original claim
    pub claim: Claim,

    /// Whether the claim is grounded
    pub grounded: bool,

    /// Confidence in the grounding assessment (0.0 - 1.0)
    pub confidence: f64,

    /// Supporting source (if grounded)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,

    /// Matching excerpt from source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_excerpt: Option<String>,

    /// Similarity score with best matching source
    pub similarity: f64,

    /// Method used for verification
    pub method: GroundingMethod,

    /// Whether this claim needs further verification
    pub needs_review: bool,
}

/// Method used for grounding verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundingMethod {
    /// Local text matching
    LocalMatch,
    /// Exact quote found
    ExactQuote,
    /// Semantic similarity
    SemanticSimilarity,
    /// LLM-based verification
    LlmVerification,
    /// Skipped (opinion/common knowledge)
    Skipped,
}

/// Overall groundedness result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundednessResult {
    /// Overall groundedness score (0.0 - 1.0)
    pub score: f64,

    /// Number of claims checked
    pub total_claims: usize,

    /// Number of grounded claims
    pub grounded_claims: usize,

    /// Number of ungrounded claims
    pub ungrounded_claims: usize,

    /// Number of claims needing review
    pub needs_review_count: usize,

    /// Individual claim results
    pub claim_results: Vec<ClaimGroundingResult>,

    /// Method used
    pub method: GroundingMethod,
}

/// Groundedness checker implementation
pub struct GroundednessChecker {
    config: GroundednessConfig,
}

impl Default for GroundednessChecker {
    fn default() -> Self {
        Self::new(GroundednessConfig::default())
    }
}

impl GroundednessChecker {
    /// Create a new groundedness checker
    pub fn new(config: GroundednessConfig) -> Self {
        Self { config }
    }

    /// Check groundedness of a response against sources
    pub fn check(
        &self,
        response: &str,
        sources: &[SourceDocument],
        explicit_claims: Option<Vec<Claim>>,
    ) -> GroundednessResult {
        // Extract claims if not provided
        let claims = explicit_claims.unwrap_or_else(|| {
            if self.config.auto_extract_claims {
                self.extract_claims(response)
            } else {
                vec![Claim {
                    text: response.to_string(),
                    start: None,
                    end: None,
                    claim_type: ClaimType::Factual,
                }]
            }
        });

        if claims.is_empty() {
            return GroundednessResult {
                score: 1.0,
                total_claims: 0,
                grounded_claims: 0,
                ungrounded_claims: 0,
                needs_review_count: 0,
                claim_results: vec![],
                method: GroundingMethod::LocalMatch,
            };
        }

        // Check each claim
        let mut claim_results = Vec::with_capacity(claims.len());
        let mut grounded_count = 0;
        let mut needs_review_count = 0;

        for claim in claims {
            let result = self.check_claim(&claim, sources);

            if result.grounded {
                grounded_count += 1;
            }
            if result.needs_review {
                needs_review_count += 1;
            }

            claim_results.push(result);
        }

        let total = claim_results.len();
        let score = if total > 0 {
            grounded_count as f64 / total as f64
        } else {
            1.0
        };

        GroundednessResult {
            score,
            total_claims: total,
            grounded_claims: grounded_count,
            ungrounded_claims: total - grounded_count,
            needs_review_count,
            claim_results,
            method: GroundingMethod::LocalMatch,
        }
    }

    /// Extract claims from response text
    fn extract_claims(&self, response: &str) -> Vec<Claim> {
        let mut claims = Vec::new();

        // Simple sentence-based extraction
        // In production, this would use NLP or LLM for better extraction
        for sentence in self.split_sentences(response) {
            let trimmed = sentence.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Skip very short sentences (likely not claims)
            if trimmed.len() < 10 {
                continue;
            }

            // Classify claim type
            let claim_type = self.classify_claim(trimmed);

            claims.push(Claim {
                text: trimmed.to_string(),
                start: None,
                end: None,
                claim_type,
            });
        }

        claims
    }

    /// Split text into sentences
    fn split_sentences<'a>(&self, text: &'a str) -> Vec<&'a str> {
        // Simple sentence splitting on . ! ?
        // In production, use proper NLP sentence tokenizer
        let mut sentences = Vec::new();
        let mut start = 0;

        for (i, c) in text.char_indices() {
            if c == '.' || c == '!' || c == '?' {
                let sentence = &text[start..=i];
                if !sentence.trim().is_empty() {
                    sentences.push(sentence.trim());
                }
                start = i + 1;
            }
        }

        // Add remaining text
        if start < text.len() {
            let remaining = &text[start..];
            if !remaining.trim().is_empty() {
                sentences.push(remaining.trim());
            }
        }

        sentences
    }

    /// Classify a claim's type
    fn classify_claim(&self, text: &str) -> ClaimType {
        let lower = text.to_lowercase();

        // Opinion indicators
        let opinion_words = [
            "i think",
            "i believe",
            "in my opinion",
            "probably",
            "might",
            "could be",
            "seems like",
            "apparently",
        ];
        for word in &opinion_words {
            if lower.contains(word) {
                return ClaimType::Opinion;
            }
        }

        // Procedural indicators
        let procedural_words = [
            "to do this",
            "first,",
            "then,",
            "finally,",
            "step ",
            "you should",
            "you can",
            "run the",
            "execute",
        ];
        for word in &procedural_words {
            if lower.contains(word) {
                return ClaimType::Procedural;
            }
        }

        ClaimType::Factual
    }

    /// Check a single claim against sources
    fn check_claim(&self, claim: &Claim, sources: &[SourceDocument]) -> ClaimGroundingResult {
        // Skip non-factual claims
        if claim.claim_type != ClaimType::Factual {
            return ClaimGroundingResult {
                claim: claim.clone(),
                grounded: true,
                confidence: 1.0,
                source_id: None,
                source_excerpt: None,
                similarity: 1.0,
                method: GroundingMethod::Skipped,
                needs_review: false,
            };
        }

        let claim_text = &claim.text;
        let claim_lower = claim_text.to_lowercase();
        let claim_words: HashSet<&str> = claim_lower.split_whitespace().collect();

        let mut best_match: Option<(f64, &SourceDocument, String)> = None;

        for source in sources {
            let source_lower = source.content.to_lowercase();

            // Check for exact match first
            if source_lower.contains(&claim_lower) {
                return ClaimGroundingResult {
                    claim: claim.clone(),
                    grounded: true,
                    confidence: 1.0,
                    source_id: Some(source.id.clone()),
                    source_excerpt: Some(self.extract_excerpt(&source.content, claim_text)),
                    similarity: 1.0,
                    method: GroundingMethod::ExactQuote,
                    needs_review: false,
                };
            }

            // Calculate word overlap similarity
            let source_words: HashSet<&str> = source_lower.split_whitespace().collect();
            let intersection = claim_words.intersection(&source_words).count();
            let union = claim_words.union(&source_words).count();

            let jaccard = if union > 0 {
                intersection as f64 / union as f64
            } else {
                0.0
            };

            // Also check for significant word overlap
            let claim_coverage = if !claim_words.is_empty() {
                intersection as f64 / claim_words.len() as f64
            } else {
                0.0
            };

            // Combined similarity score
            let similarity = (jaccard + claim_coverage) / 2.0;

            if similarity > best_match.as_ref().map(|(s, _, _)| *s).unwrap_or(0.0) {
                let excerpt = self.find_best_excerpt(&source.content, claim_text);
                best_match = Some((similarity, source, excerpt));
            }
        }

        if let Some((similarity, source, excerpt)) = best_match {
            let grounded = similarity >= self.config.similarity_threshold;
            let confidence = similarity;
            let needs_review = !grounded
                && similarity >= self.config.similarity_threshold * 0.7
                && self.config.use_llm_fallback;

            debug!(
                "Claim grounding: similarity={:.2}, grounded={}, needs_review={}",
                similarity, grounded, needs_review
            );

            ClaimGroundingResult {
                claim: claim.clone(),
                grounded,
                confidence,
                source_id: if grounded || needs_review {
                    Some(source.id.clone())
                } else {
                    None
                },
                source_excerpt: if grounded || needs_review {
                    Some(excerpt)
                } else {
                    None
                },
                similarity,
                method: GroundingMethod::LocalMatch,
                needs_review,
            }
        } else {
            ClaimGroundingResult {
                claim: claim.clone(),
                grounded: false,
                confidence: 0.0,
                source_id: None,
                source_excerpt: None,
                similarity: 0.0,
                method: GroundingMethod::LocalMatch,
                needs_review: self.config.use_llm_fallback,
            }
        }
    }

    /// Extract an excerpt from source around the matching text
    fn extract_excerpt(&self, source: &str, claim: &str) -> String {
        let source_lower = source.to_lowercase();
        let claim_lower = claim.to_lowercase();

        if let Some(pos) = source_lower.find(&claim_lower) {
            // Get context around the match
            let start = pos.saturating_sub(50);
            let end = (pos + claim.len() + 50).min(source.len());

            let excerpt = &source[start..end];
            if start > 0 {
                format!("...{}", excerpt.trim())
            } else {
                excerpt.trim().to_string()
            }
        } else {
            // Return first part of source
            source.chars().take(200).collect()
        }
    }

    /// Find the best matching excerpt in source for a claim
    fn find_best_excerpt(&self, source: &str, claim: &str) -> String {
        let claim_words: Vec<&str> = claim.split_whitespace().take(5).collect();

        // Try to find a region with high word overlap
        let source_lower = source.to_lowercase();

        for word in &claim_words {
            let word_lower = word.to_lowercase();
            if let Some(pos) = source_lower.find(&word_lower) {
                let start = pos.saturating_sub(30);
                let end = (pos + 150).min(source.len());
                return format!("...{}...", source[start..end].trim());
            }
        }

        // Fallback: return beginning of source
        source.chars().take(150).collect::<String>() + "..."
    }
}

// ============ LLM Provider Interface ============

/// LLM verification request for groundedness
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmVerificationRequest {
    /// The claim to verify
    pub claim: String,
    /// Source documents to check against
    pub sources: Vec<SourceDocument>,
    /// System prompt for the LLM
    pub system_prompt: String,
}

/// LLM verification response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmVerificationResponse {
    /// Whether the claim is grounded
    pub grounded: bool,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Source ID that supports the claim (if grounded)
    pub supporting_source_id: Option<String>,
    /// Explanation from the LLM
    pub explanation: Option<String>,
}

/// Trait for LLM-based groundedness verification
///
/// Implement this trait to integrate different LLM providers
/// (OpenAI, Anthropic, local models, etc.)
#[async_trait]
pub trait LlmGroundednessVerifier: Send + Sync {
    /// Verify a single claim against sources
    async fn verify_claim(
        &self,
        request: LlmVerificationRequest,
    ) -> Result<LlmVerificationResponse, String>;

    /// Verify multiple claims in batch (for efficiency)
    async fn verify_claims_batch(
        &self,
        requests: Vec<LlmVerificationRequest>,
    ) -> Result<Vec<LlmVerificationResponse>, String> {
        // Default implementation: sequential calls
        let mut results = Vec::with_capacity(requests.len());
        for req in requests {
            results.push(self.verify_claim(req).await?);
        }
        Ok(results)
    }
}

/// Default system prompt for LLM verification
pub const DEFAULT_VERIFICATION_PROMPT: &str = r#"You are a groundedness verification assistant. Your task is to determine if a claim is supported by the provided source documents.

Respond with a JSON object containing:
- "grounded": true/false - whether the claim is supported
- "confidence": 0.0-1.0 - how confident you are
- "source_id": the ID of the supporting source, or null
- "explanation": brief explanation of your reasoning

Be strict: a claim is only grounded if the sources directly support it, not if it's merely plausible."#;

/// Mock LLM verifier for testing
#[derive(Default)]
pub struct MockLlmVerifier;

#[async_trait]
impl LlmGroundednessVerifier for MockLlmVerifier {
    async fn verify_claim(
        &self,
        request: LlmVerificationRequest,
    ) -> Result<LlmVerificationResponse, String> {
        // Simple mock: check if any source contains key words from the claim
        let claim_lower = request.claim.to_lowercase();
        let claim_words: std::collections::HashSet<&str> = claim_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .collect();

        for source in &request.sources {
            let source_lower = source.content.to_lowercase();
            let matching_words = claim_words
                .iter()
                .filter(|w| source_lower.contains(*w))
                .count();
            let overlap = matching_words as f64 / claim_words.len().max(1) as f64;

            if overlap > 0.5 {
                return Ok(LlmVerificationResponse {
                    grounded: true,
                    confidence: overlap,
                    supporting_source_id: Some(source.id.clone()),
                    explanation: Some("Mock: word overlap detected".to_string()),
                });
            }
        }

        Ok(LlmVerificationResponse {
            grounded: false,
            confidence: 0.8,
            supporting_source_id: None,
            explanation: Some("Mock: no supporting source found".to_string()),
        })
    }
}

/// Extended groundedness checker with LLM support
pub struct LlmGroundednessChecker<V: LlmGroundednessVerifier> {
    config: GroundednessConfig,
    local_checker: GroundednessChecker,
    llm_verifier: V,
}

impl<V: LlmGroundednessVerifier> LlmGroundednessChecker<V> {
    /// Create a new LLM-enabled groundedness checker
    pub fn new(config: GroundednessConfig, llm_verifier: V) -> Self {
        Self {
            config: config.clone(),
            local_checker: GroundednessChecker::new(config),
            llm_verifier,
        }
    }

    /// Check groundedness with LLM fallback for uncertain cases
    pub async fn check_with_llm(
        &self,
        response: &str,
        sources: &[SourceDocument],
        explicit_claims: Option<Vec<Claim>>,
    ) -> GroundednessResult {
        // First, run local matching
        let mut result = self.local_checker.check(response, sources, explicit_claims);

        // If LLM fallback is disabled, return local result
        if !self.config.use_llm_fallback {
            return result;
        }

        // Find claims that need LLM verification (low confidence local matches)
        let uncertain_indices: Vec<usize> = result
            .claim_results
            .iter()
            .enumerate()
            .filter(|(_, cr)| {
                // Use LLM for low-confidence local matches that aren't skipped
                cr.method != GroundingMethod::Skipped
                    && cr.confidence < self.config.confidence_threshold
            })
            .map(|(i, _)| i)
            .collect();

        if uncertain_indices.is_empty() {
            return result;
        }

        // Build LLM verification requests
        let requests: Vec<LlmVerificationRequest> = uncertain_indices
            .iter()
            .map(|&i| LlmVerificationRequest {
                claim: result.claim_results[i].claim.text.clone(),
                sources: sources.to_vec(),
                system_prompt: DEFAULT_VERIFICATION_PROMPT.to_string(),
            })
            .collect();

        // Call LLM verifier asynchronously
        match self.llm_verifier.verify_claims_batch(requests).await {
            Ok(responses) => {
                // Update results with LLM verification
                for (idx_offset, llm_response) in responses.into_iter().enumerate() {
                    let i = uncertain_indices[idx_offset];
                    result.claim_results[i].grounded = llm_response.grounded;
                    result.claim_results[i].confidence = llm_response.confidence;
                    result.claim_results[i].method = GroundingMethod::LlmVerification;
                    result.claim_results[i].source_id = llm_response.supporting_source_id;
                }

                // Recalculate totals
                result.grounded_claims =
                    result.claim_results.iter().filter(|cr| cr.grounded).count();
                result.ungrounded_claims = result
                    .claim_results
                    .iter()
                    .filter(|cr| !cr.grounded && cr.method != GroundingMethod::Skipped)
                    .count();
                result.needs_review_count = 0;
                result.score = if result.total_claims > 0 {
                    result.grounded_claims as f64 / result.total_claims as f64
                } else {
                    1.0
                };
                // Mark as LLM-assisted verification
                result.method = GroundingMethod::LlmVerification;
            }
            Err(e) => {
                debug!("LLM verification failed: {}", e);
                // Keep local results, method stays as is
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let checker = GroundednessChecker::default();
        let sources = vec![SourceDocument {
            id: "doc1".to_string(),
            content: "The capital of France is Paris.".to_string(),
            title: None,
            url: None,
            relevance: 1.0,
        }];

        let result = checker.check(
            "The capital of France is Paris.",
            &sources,
            Some(vec![Claim {
                text: "The capital of France is Paris.".to_string(),
                start: None,
                end: None,
                claim_type: ClaimType::Factual,
            }]),
        );

        assert_eq!(result.score, 1.0);
        assert_eq!(result.grounded_claims, 1);
    }

    #[test]
    fn test_partial_match() {
        let checker = GroundednessChecker::new(GroundednessConfig {
            similarity_threshold: 0.5,
            ..Default::default()
        });

        let sources = vec![SourceDocument {
            id: "doc1".to_string(),
            content: "Paris is the capital city of France and has many monuments.".to_string(),
            title: None,
            url: None,
            relevance: 1.0,
        }];

        let result = checker.check(
            "Paris is the capital of France.",
            &sources,
            Some(vec![Claim {
                text: "Paris is the capital of France.".to_string(),
                start: None,
                end: None,
                claim_type: ClaimType::Factual,
            }]),
        );

        assert!(result.score > 0.0);
    }

    #[test]
    fn test_no_match() {
        let checker = GroundednessChecker::default();
        let sources = vec![SourceDocument {
            id: "doc1".to_string(),
            content: "The weather today is sunny.".to_string(),
            title: None,
            url: None,
            relevance: 1.0,
        }];

        let result = checker.check(
            "The capital of France is Paris.",
            &sources,
            Some(vec![Claim {
                text: "The capital of France is Paris.".to_string(),
                start: None,
                end: None,
                claim_type: ClaimType::Factual,
            }]),
        );

        assert_eq!(result.score, 0.0);
        assert_eq!(result.ungrounded_claims, 1);
    }

    #[test]
    fn test_opinion_skipped() {
        let checker = GroundednessChecker::default();
        let sources = vec![];

        let result = checker.check(
            "I think this is a good idea.",
            &sources,
            Some(vec![Claim {
                text: "I think this is a good idea.".to_string(),
                start: None,
                end: None,
                claim_type: ClaimType::Opinion,
            }]),
        );

        assert_eq!(result.score, 1.0);
        assert_eq!(result.claim_results[0].method, GroundingMethod::Skipped);
    }

    #[test]
    fn test_auto_extract_claims() {
        let checker = GroundednessChecker::default();
        let sources = vec![SourceDocument {
            id: "doc1".to_string(),
            content: "Python is a programming language. It was created by Guido van Rossum."
                .to_string(),
            title: None,
            url: None,
            relevance: 1.0,
        }];

        let result = checker.check(
            "Python is a programming language. It is very popular.",
            &sources,
            None,
        );

        assert!(result.total_claims >= 1);
    }
}
