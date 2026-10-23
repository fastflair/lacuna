//! Four-layer universal input contract for domain-agnostic whitespace analysis.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Default dimensionality for deterministic feature-hashing embeddings when no
/// external embedding model is called.
pub const DEFAULT_EMBED_DIM: usize = 64;

/// One orthogonal semantic dimension along which entities in a domain vary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacetSpec {
    /// Machine identifier (e.g., `"core_loop"`, `"mechanism"`, `"problem"`).
    pub id: String,
    /// Human-readable label.
    pub name: String,
    /// Instructions describing what belongs on this axis and what does not.
    pub description: String,
    /// Default relative weight in `[0.0, 1.0]` when computing composite distance.
    pub default_weight: f32,
}

/// Layer 2 of the universal input contract: the coordinate system for a domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainSchema {
    /// Short identifier (e.g., `"steam-games"`, `"vc-startups"`, `"patent-claims"`).
    pub domain_id: String,
    /// Display title.
    pub title: String,
    /// Overview of what constitutes an entity and an opportunity in this domain.
    pub description: String,
    /// Orthogonal semantic axes (`FacetSpec`).
    pub facets: Vec<FacetSpec>,
    /// Name of the demand/traction signal (e.g., `"log_reviews"`, `"funding_musd"`).
    pub demand_metric_name: String,
    /// Name of the quality/sentiment signal in `[0.0, 1.0]` (e.g., `"positive_review_ratio"`).
    pub quality_metric_name: String,
}

impl DomainSchema {
    /// Returns the default weight map across all declared facets.
    pub fn default_weights(&self) -> BTreeMap<String, f32> {
        self.facets
            .iter()
            .map(|f| (f.id.clone(), f.default_weight))
            .collect()
    }
}

/// Layer 1 of the universal input contract: a single incumbent entity in the domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainEntity {
    /// Unique identifier within the corpus (e.g., `"steam:646570"`).
    pub id: String,
    /// Display name (e.g., `"Slay the Spire"`).
    pub name: String,
    /// Concise summary or blurb.
    pub summary: String,
    /// Categorical tags or genres.
    pub tags: Vec<String>,
    /// Decomposed text description per facet id.
    pub facets: BTreeMap<String, String>,
    /// Unit-normalized embedding vector per facet id.
    pub vectors: BTreeMap<String, Vec<f32>>,
    /// Non-negative demand/volume signal (e.g., `ln(1 + review_count)`).
    pub demand_signal: f64,
    /// Normalized quality/sentiment signal in `[0.0, 1.0]`.
    pub quality_signal: f64,
    /// Arbitrary domain-specific metadata (price, release year, URL, etc.).
    #[serde(default)]
    pub metadata: Value,
}

impl DomainEntity {
    /// Ensures every facet declared in `schema` has a unit-normalized embedding vector,
    /// computing a deterministic locality-sensitive embedding from the facet text when absent.
    pub fn ensure_embedded(&mut self, schema: &DomainSchema, dim: usize) {
        for facet in &schema.facets {
            let needs_embed = self.vectors.get(&facet.id).is_none_or(|v| v.is_empty());
            if needs_embed {
                let text = self
                    .facets
                    .get(&facet.id)
                    .cloned()
                    .unwrap_or_else(|| format!("{} {}", self.name, self.summary));
                let vec = deterministic_embed(&format!("{}:{}", facet.id, text), dim);
                self.vectors.insert(facet.id.clone(), vec);
            } else if let Some(vec) = self.vectors.get_mut(&facet.id) {
                normalize_in_place(vec);
            }
        }
    }
}

/// Complete portable snapshot of a domain corpus and its schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CorpusSnapshot {
    pub schema: DomainSchema,
    pub entities: Vec<DomainEntity>,
}

impl CorpusSnapshot {
    /// Guarantees all entities have unit-normalized facet embeddings.
    pub fn ensure_all_embedded(&mut self, dim: usize) {
        for entity in &mut self.entities {
            entity.ensure_embedded(&self.schema, dim);
        }
    }
}

/// Computes a deterministic, locality-sensitive unit-normalized embedding vector
/// from text using weighted word + character 3-gram signed feature hashing.
///
/// Semantically overlapping phrases share token and subword projections while
/// remaining 100% deterministic offline without requiring an API key.
pub fn deterministic_embed(text: &str, dim: usize) -> Vec<f32> {
    let dim = dim.max(8);
    let mut acc = vec![0.0_f32; dim];
    let lower = text.to_lowercase();

    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .collect();

    if tokens.is_empty() {
        acc[0] = 1.0;
        return acc;
    }

    // 1. Whole-token projections (weight 1.0) and adjacent token bigrams (weight 0.7)
    for (i, &tok) in tokens.iter().enumerate() {
        project_feature(&mut acc, tok.as_bytes(), 1.0);
        if i + 1 < tokens.len() {
            let bigram = format!("{}_{}", tok, tokens[i + 1]);
            project_feature(&mut acc, bigram.as_bytes(), 0.7);
        }
        // 2. Character 3-grams for morphological / subword similarity (weight 0.35)
        let chars: Vec<char> = tok.chars().collect();
        if chars.len() >= 3 {
            for win in chars.windows(3) {
                let ngram: String = win.iter().collect();
                project_feature(&mut acc, ngram.as_bytes(), 0.35);
            }
        }
    }

    normalize_in_place(&mut acc);
    acc
}

fn project_feature(acc: &mut [f32], feature: &[u8], weight: f32) {
    let dim = acc.len();
    let digest = Sha256::digest(feature);
    // Project each feature onto 4 sparse coordinates with signed Rademacher weights.
    for slot in 0..4 {
        let base = slot * 8;
        let idx_bytes = [
            digest[base],
            digest[base + 1],
            digest[base + 2],
            digest[base + 3],
        ];
        let sign_byte = digest[base + 4];
        let mag_byte = digest[base + 5];
        let idx = (u32::from_le_bytes(idx_bytes) as usize) % dim;
        let sign = if sign_byte & 1 == 0 {
            1.0_f32
        } else {
            -1.0_f32
        };
        let scale = 0.5_f32 + (f32::from(mag_byte) / 255.0_f32) * 0.5_f32;
        acc[idx] += sign * weight * scale;
    }
}

/// Normalizes a vector in-place to unit L2 norm.
pub fn normalize_in_place(v: &mut [f32]) {
    let norm_sq: f32 = v.iter().map(|x| x * x).sum();
    if norm_sq > 1e-12 {
        let inv = 1.0 / norm_sq.sqrt();
        for x in v.iter_mut() {
            *x *= inv;
        }
    } else if !v.is_empty() {
        v[0] = 1.0;
    }
}
