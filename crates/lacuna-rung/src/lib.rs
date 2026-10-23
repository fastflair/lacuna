//! `lacuna-rung` — connects `lacuna-core` high-dimensional geometry to `rung` ladders,
//! `rung_std::decide` decision models, `rung_std::llm`, and `rung-agent-core`.

pub mod agent;
pub mod decide;
pub mod ladder;

pub use agent::{abduct_deterministic, abduct_hypothesis, run_rung_agent_turn};
pub use decide::{
    CalibratedGeometricDecider, DecisionScorecard, DraftHypothesis, build_viability_ask,
    evaluate_with_decider, jev_from_env, resolve_decider,
};
pub use ladder::{
    ClosedLoopProof, SynthesisSpec, SynthesisState, VerifiedOpportunity, run_synthesis_ladder,
    synthesizevoid,
};

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use lacuna_core::{VoidSolverConfig, find_interior_voids, steam_seed_snapshot};

    use super::*;

    #[test]
    fn synthesis_ladder_produces_verified_opportunity_with_closed_loop_proof() {
        let snapshot = steam_seed_snapshot();
        let voids = find_interior_voids(
            &snapshot.schema,
            &snapshot.entities,
            &VoidSolverConfig::default(),
        );
        assert!(!voids.is_empty());

        let spec = SynthesisSpec {
            schema: snapshot.schema.clone(),
            corpus: Arc::new(snapshot.entities),
            void: voids[0].clone(),
            decider: Arc::new(CalibratedGeometricDecider),
            live_llm: false,
            min_corpus_separation: 0.12,
            min_viability: 0.45,
            max_attempts: 3,
        };

        let verified = run_synthesis_ladder(spec).expect("ladder should converge to Verified");
        assert_eq!(verified.void_id, "void-01");
        assert!(verified.decision.composite_viability >= 0.45);
        assert!(verified.closed_loop_proof.verified_in_void);
        assert!(verified.closed_loop_proof.min_distance_to_existing_entity >= 0.12);
    }
}
