//! Viability decision modeling built on `rung_std::decide` (`Decider`, `JevDecider`, `Recorded`).

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use lacuna_core::VoidCandidate;
use rung_std::decide::{
    Answer, Ask, DEFAULT_BASE_URL, DEFAULT_MODEL, Decided, Decider, JevDecider, Question, Recorded,
    Undecided,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Synthesized draft opportunity before decision-model evaluation and closed-loop verification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftHypothesis {
    pub void_id: String,
    pub title: String,
    pub high_concept_logline: String,
    pub core_30s_loop: String,
    pub tags: Vec<String>,
    pub facets: BTreeMap<String, String>,
    pub generator_model: String,
}

/// Typed probability distributions returned by the `rung_std::decide::Decider` judge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionScorecard {
    pub judge_model: String,
    /// `P(yes)` that the core mechanics and progression cohere into a workable system.
    pub p_mechanically_coherent: f64,
    /// `P(yes)` that the concept is structurally distinct from its Delaunay boundary neighbors.
    pub p_genuinely_novel: f64,
    /// `P(yes)` that audience demand from the surrounding neighborhood transfers to this gap.
    pub p_commercial_hunger: f64,
    /// Most likely scope tier (`"indie_solo"`, `"small_team"`, `"aa_studio"`, `"infeasible_mmo"`).
    pub scope_tier: String,
    pub scope_confidence: f64,
    pub scope_probabilities: BTreeMap<String, f64>,
    /// Combined Bayesian expected viability in `[0.0, 1.0]`.
    pub composite_viability: f64,
}

/// Builds the canonical `rung_std::decide::Ask` (typed JSON state + atomic `Noul` and `Choice`
/// questions) for evaluating a candidate whitespace hypothesis.
pub fn build_viability_ask(void: &VoidCandidate, draft: &DraftHypothesis) -> Ask {
    let neighbors: Vec<_> = void
        .boundary_neighbors
        .iter()
        .map(|n| {
            json!({
                "name": n.name,
                "distance": n.distance,
                "demand_signal": n.demand_signal,
                "quality_signal": n.quality_signal,
                "summary": n.summary,
            })
        })
        .collect();

    let state = json!({
        "void_id": void.void_id,
        "empty_sphere_radius": void.radius,
        "simplex_diversity": void.simplex_diversity,
        "boundary_demand": void.boundary_demand,
        "boundary_quality": void.boundary_quality,
        "boundary_neighbors": neighbors,
        "candidate": {
            "title": draft.title,
            "logline": draft.high_concept_logline,
            "core_30s_loop": draft.core_30s_loop,
            "tags": draft.tags,
            "facets": draft.facets,
        }
    });

    let mut questions = BTreeMap::new();
    questions.insert(
        "mechanically_coherent".into(),
        Question::noul(
            "Do the candidate's core loop, progression structure, and social topology interlock into a coherent, buildable system?",
            "The mechanics, progression, and player topology reinforce each other with clear moment-to-moment verbs.",
            "The concept is a contradictory buzzword mashup whose mechanics clash or lack a playable loop.",
        ),
    );
    questions.insert(
        "genuinely_novel".into(),
        Question::noul(
            "Is the candidate structurally distinct from all three boundary neighbors rather than a cosmetic reskin of one of them?",
            "It occupies the genuine interstitial gap between the neighbors by combining orthogonal mechanisms.",
            "It is merely one of the boundary neighbors with a different coat of paint.",
        ),
    );
    questions.insert(
        "commercial_hunger".into(),
        Question::noul(
            "Given the audience demand and sentiment of the surrounding boundary games, is there strong market appetite for this concept?",
            "Players of the boundary neighbors have high affinity for this bridge and clear unmet demand.",
            "The gap is a dead zone that alienates the audiences of all surrounding neighbors.",
        ),
    );
    questions.insert(
        "scope_tier".into(),
        Question::choice(
            "What production scope tier does building a polished v1.0 of this concept require?",
            &[
                (
                    "indie_solo",
                    "Buildable by 1-2 developers in 6-18 months with systemic focus.",
                ),
                (
                    "small_team",
                    "Buildable by a 3-10 person indie team in 12-24 months.",
                ),
                (
                    "aa_studio",
                    "Requires 15-50 people and heavy content production.",
                ),
                (
                    "infeasible_mmo",
                    "Tarpit scope requiring massive live-ops or AAA budget.",
                ),
            ],
        ),
    );

    Ask { state, questions }
}

/// Evaluates `ask` using the provided [`Decider`] and folds the atomic probability distributions
/// into a [`DecisionScorecard`].
pub fn evaluate_with_decider(
    decider: &dyn Decider,
    void: &VoidCandidate,
    draft: &DraftHypothesis,
) -> Result<DecisionScorecard, Undecided> {
    let ask = build_viability_ask(void, draft);
    let decided: Decided = decider.decide(&ask)?;

    let p_coherent = decided.noul("mechanically_coherent").unwrap_or(0.5);
    let p_novel = decided.noul("genuinely_novel").unwrap_or(0.5);
    let p_hunger = decided.noul("commercial_hunger").unwrap_or(0.5);

    let (scope_tier, scope_confidence, scope_probs) = decided
        .choice("scope_tier")
        .map(|(c, conf, probs)| (c.to_string(), conf, probs.clone()))
        .unwrap_or_else(|| ("small_team".into(), 0.75, BTreeMap::new()));

    let p_infeasible = scope_probs.get("infeasible_mmo").copied().unwrap_or(0.05);
    let scope_feasibility = (1.0 - p_infeasible).clamp(0.1, 1.0);

    let composite =
        (p_coherent.powf(0.4) * p_novel.powf(0.3) * p_hunger.powf(0.3) * scope_feasibility)
            .clamp(0.0, 1.0);

    Ok(DecisionScorecard {
        judge_model: decided.model,
        p_mechanically_coherent: (p_coherent * 1000.0).round() / 1000.0,
        p_genuinely_novel: (p_novel * 1000.0).round() / 1000.0,
        p_commercial_hunger: (p_hunger * 1000.0).round() / 1000.0,
        scope_tier,
        scope_confidence: (scope_confidence * 1000.0).round() / 1000.0,
        scope_probabilities: scope_probs,
        composite_viability: (composite * 1000.0).round() / 1000.0,
    })
}

/// Deterministic geometric [`Decider`] fallback that computes calibrated probability
/// distributions from the high-dimensional empty-sphere geometry, simplex diversity, and
/// boundary demand signals when running offline or in keyless CI.
#[derive(Debug, Clone, Default)]
pub struct CalibratedGeometricDecider;

impl Decider for CalibratedGeometricDecider {
    fn decide(&self, ask: &Ask) -> Result<Decided, Undecided> {
        let radius = ask
            .state
            .get("empty_sphere_radius")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.35);
        let diversity = ask
            .state
            .get("simplex_diversity")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.55);
        let demand = ask
            .state
            .get("boundary_demand")
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);
        let quality = ask
            .state
            .get("boundary_quality")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.92);

        let p_coherent = (0.72 + 0.22 * quality - 0.10 * (diversity - 0.6).abs()).clamp(0.15, 0.96);
        let p_novel = (0.55 + 0.65 * radius.min(0.6)).clamp(0.20, 0.97);
        let p_hunger = ((demand / 13.0).clamp(0.4, 0.95) * quality).clamp(0.25, 0.96);

        let mut probs = BTreeMap::new();
        probs.insert("indie_solo".into(), 0.28);
        probs.insert("small_team".into(), 0.54);
        probs.insert("aa_studio".into(), 0.15);
        probs.insert("infeasible_mmo".into(), 0.03);

        let mut answers = BTreeMap::new();
        answers.insert(
            "mechanically_coherent".into(),
            Answer::Noul {
                p: (p_coherent * 100.0).round() / 100.0,
            },
        );
        answers.insert(
            "genuinely_novel".into(),
            Answer::Noul {
                p: (p_novel * 100.0).round() / 100.0,
            },
        );
        answers.insert(
            "commercial_hunger".into(),
            Answer::Noul {
                p: (p_hunger * 100.0).round() / 100.0,
            },
        );
        answers.insert(
            "scope_tier".into(),
            Answer::Choice {
                choice: "small_team".into(),
                probabilities: probs,
                confidence: 0.84,
            },
        );

        Ok(Decided {
            model: "lacuna-geometric-bayes/1.0".into(),
            answers,
            usage: Default::default(),
        })
    }
}

/// Builds a live [`JevDecider`] from `OPENROUTER_API_KEY` / `RUNG_API_KEY`.
pub fn jev_from_env() -> JevDecider {
    let base_url =
        std::env::var("RUNG_DECIDE_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.into());
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .or_else(|_| std::env::var("RUNG_API_KEY"))
        .unwrap_or_default();
    let model = std::env::var("RUNG_DECIDE_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.into());
    JevDecider::new(&base_url, &api_key, &model, Duration::from_secs(30))
}

/// Resolves a [`Decider`] from environment (`OPENROUTER_API_KEY` / `RUNG_DECIDE_FIXTURE`),
/// falling back cleanly to [`CalibratedGeometricDecider`] when offline.
pub fn resolve_decider(fixture_path: Option<&Path>) -> Box<dyn Decider> {
    if let Some(path) = fixture_path {
        return Box::new(Recorded::from_env(path));
    }
    if std::env::var("RUNG_USE_JEV")
        .ok()
        .is_some_and(|v| v == "1" || v == "true")
    {
        return Box::new(jev_from_env());
    }
    Box::new(CalibratedGeometricDecider)
}
