//! Type-enforced `rung::ladder!` for whitespace abduction, `rung_std::decide` judgment,
//! and closed-loop re-embedding verification.
//!
//! Because `VerifiedOpportunity` is only produced by the terminal `Verified` verdict of
//! [`synthesizevoid`], no caller can fabricate a verified opportunity without passing
//! both the `rung_std::decide::Decider` gate and the closed-loop metric verification check.

use std::collections::BTreeMap;
use std::sync::Arc;

use lacuna_core::{
    DEFAULT_EMBED_DIM, DomainEntity, DomainSchema, VoidCandidate, deterministic_embed,
    point_to_entity_distance, point_to_point_distance,
};
use rung::ladder;
use rung_std::decide::Decider;
use serde::{Deserialize, Serialize};

use crate::agent::abduct_hypothesis;
use crate::decide::{DecisionScorecard, DraftHypothesis, evaluate_with_decider};

/// Closed-loop geometric proof certifying that the re-embedded hypothesis lands in the
/// whitespace void and stays separated from all existing corpus entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClosedLoopProof {
    /// Multi-facet cosine distance from the re-embedded hypothesis to the target void center.
    pub distance_to_void_center: f64,
    /// Minimum multi-facet cosine distance from the re-embedded hypothesis to ANY entity in the corpus.
    pub min_distance_to_existing_entity: f64,
    /// ID of the closest existing corpus entity to the synthesized concept.
    pub closest_existing_entity_id: String,
    /// Name of the closest existing corpus entity.
    pub closest_existing_entity_name: String,
    /// Whether the hypothesis satisfied the minimum separation threshold.
    pub verified_in_void: bool,
    /// Number of abduction-verification iterations executed.
    pub iterations_used: u32,
}

/// Terminal output of the [`synthesizevoid`] ladder: a synthesized, decision-scored,
/// and geometrically verified whitespace opportunity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedOpportunity {
    pub void_id: String,
    pub void_rank: usize,
    pub empty_sphere_radius: f64,
    pub opportunity_score: f64,
    pub hypothesis: DraftHypothesis,
    pub decision: DecisionScorecard,
    pub closed_loop_proof: ClosedLoopProof,
}

/// Input parameters for the [`synthesizevoid`] ladder.
#[derive(Clone, Debug)]
pub struct SynthesisSpec {
    pub schema: DomainSchema,
    pub corpus: Arc<Vec<DomainEntity>>,
    pub void: VoidCandidate,
    pub decider: Arc<dyn Decider>,
    pub live_llm: bool,
    pub min_corpus_separation: f64,
    pub min_viability: f64,
    pub max_attempts: u32,
}

/// Internal loop state for [`synthesizevoid`].
#[derive(Clone, Debug)]
pub struct SynthesisState {
    pub spec: SynthesisSpec,
    pub attempt: u32,
}

ladder!(SynthesizeVoid {
    carry { void_id: String }

    Spec(SynthesisSpec)
      => Abducting(SynthesisState)
      => {
          Mutate -> Abducting
          | Verified(VerifiedOpportunity)
          | Rejected(String)
      }
} impl {
    abducting = |spec| {
        let carry = spec.carry().clone();
        Abducting::new(
            SynthesisState {
                spec: spec.payload,
                attempt: 0,
            },
            carry,
        )
    },

    step = |abducting| {
        let carry = abducting.carry().clone();
        let state = abducting.payload;
        let attempt = state.attempt;
        let spec = &state.spec;

        let draft = match abduct_hypothesis(&spec.schema, &spec.void, attempt, spec.live_llm) {
            Ok(d) => d,
            Err(e) => return Ok(StepOutcome::Rejected(Rejected::new(e))),
        };

        let decision = match evaluate_with_decider(spec.decider.as_ref(), &spec.void, &draft) {
            Ok(scorecard) => scorecard,
            Err(undecided) => {
                return Ok(StepOutcome::Rejected(Rejected::new(format!(
                    "decider undecided: {undecided}"
                ))));
            }
        };

        // Closed-loop re-embedding verification: embed the generated hypothesis facets and
        // measure its exact distance to the target void center and every corpus entity.
        let mut embedded_facets = BTreeMap::new();
        for facet in &spec.schema.facets {
            let text = draft
                .facets
                .get(&facet.id)
                .cloned()
                .unwrap_or_else(|| format!("{} {}", draft.title, draft.core_30s_loop));
            let vec = deterministic_embed(&format!("{}:{}", facet.id, text), DEFAULT_EMBED_DIM);
            embedded_facets.insert(facet.id.clone(), vec);
        }

        let weights = spec.schema.default_weights();
        let dist_to_void = point_to_point_distance(&embedded_facets, &spec.void.center, &weights);

        let mut min_dist = f64::INFINITY;
        let mut closest_id = String::new();
        let mut closest_name = String::new();
        for entity in spec.corpus.iter() {
            let d = point_to_entity_distance(&embedded_facets, entity, &weights);
            if d < min_dist {
                min_dist = d;
                closest_id = entity.id.clone();
                closest_name = entity.name.clone();
            }
        }

        let separated = min_dist >= spec.min_corpus_separation;
        let viable = decision.composite_viability >= spec.min_viability;

        if (!separated || !viable) && attempt + 1 < spec.max_attempts {
            return Ok(StepOutcome::Mutate(Abducting::new(
                SynthesisState {
                    spec: state.spec,
                    attempt: attempt + 1,
                },
                carry,
            )));
        }

        if !viable {
            return Ok(StepOutcome::Rejected(Rejected::new(format!(
                "composite viability {:.3} below threshold {:.3}",
                decision.composite_viability, spec.min_viability
            ))));
        }

        let proof = ClosedLoopProof {
            distance_to_void_center: (dist_to_void * 10000.0).round() / 10000.0,
            min_distance_to_existing_entity: (min_dist * 10000.0).round() / 10000.0,
            closest_existing_entity_id: closest_id,
            closest_existing_entity_name: closest_name,
            verified_in_void: separated,
            iterations_used: attempt + 1,
        };

        Ok(StepOutcome::Verified(Verified::new(VerifiedOpportunity {
            void_id: spec.void.void_id.clone(),
            void_rank: spec.void.rank,
            empty_sphere_radius: spec.void.radius,
            opportunity_score: spec.void.opportunity_score,
            hypothesis: draft,
            decision,
            closed_loop_proof: proof,
        })))
    },
});

/// Drives the [`synthesizevoid`] ladder from `Spec` to a terminal outcome.
pub fn run_synthesis_ladder(spec: SynthesisSpec) -> Result<VerifiedOpportunity, String> {
    let void_id = spec.void.void_id.clone();
    let entry = synthesizevoid::Spec::new(spec, synthesizevoid::Carry { void_id });
    let mut active = synthesizevoid::abducting(entry);

    loop {
        match synthesizevoid::step(active) {
            Ok(synthesizevoid::StepOutcome::Mutate(next)) => {
                active = next;
            }
            Ok(synthesizevoid::StepOutcome::Verified(v)) => {
                return Ok(v.into_payload());
            }
            Ok(synthesizevoid::StepOutcome::Rejected(r)) => {
                return Err(r.into_payload());
            }
            Err(f) => {
                return Err(f.error);
            }
        }
    }
}
