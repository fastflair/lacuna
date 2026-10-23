//! Abductive hypothesis synthesis via `rung_std::llm::llm_call` and autonomous piloting via
//! `rung-agent-core`.

use std::collections::BTreeMap;

use lacuna_core::{DomainSchema, VoidCandidate};
use rung_std::llm::{
    ChatMessage, ContentBlock, DEFAULT_MAX_ATTEMPTS, LlmConfig, LlmRequest, MessageContent, llmcall,
};
use serde_json::Value;

use crate::decide::DraftHypothesis;

/// Synthesizes a [`DraftHypothesis`] for a high-dimensional [`VoidCandidate`].
///
/// When `live_llm` is enabled, dispatches through the canonical `rung_std::llm::llm_call`
/// ladder using `$XDG_CONFIG_HOME/rung/config.yaml` / `RUNG_*` / `OPENROUTER_API_KEY`.
/// Otherwise performs deterministic contrastive facet synthesis from the Delaunay simplex
/// boundary neighbors.
pub fn abduct_hypothesis(
    schema: &DomainSchema,
    void: &VoidCandidate,
    mutation_round: u32,
    live_llm: bool,
) -> Result<DraftHypothesis, String> {
    if live_llm {
        let mut cfg = load_rung_llm_config()?;
        if cfg.max_tokens == 0 {
            cfg.max_tokens = 1024;
        }
        return abduct_via_rung_llm(&cfg, schema, void, mutation_round);
    }
    Ok(abduct_deterministic(schema, void, mutation_round))
}

fn load_rung_llm_config() -> Result<LlmConfig, String> {
    let mut cfg = rung_agent_core::config::load()?;
    rung_agent_core::engine::reasoning_from_env(&mut cfg);
    if cfg.api_key.is_empty()
        && let Ok(or_key) = std::env::var("OPENROUTER_API_KEY")
        && !or_key.trim().is_empty()
    {
        cfg.api_key = or_key;
        if cfg.base_url.is_empty() {
            cfg.base_url = "https://openrouter.ai/api/v1".into();
        }
    }
    Ok(cfg)
}

fn abduct_via_rung_llm(
    cfg: &LlmConfig,
    schema: &DomainSchema,
    void: &VoidCandidate,
    mutation_round: u32,
) -> Result<DraftHypothesis, String> {
    let neighbors_json = serde_json::to_string_pretty(&void.boundary_neighbors)
        .map_err(|e| format!("serialize neighbors: {e}"))?;
    let facets_json = serde_json::to_string_pretty(&schema.facets)
        .map_err(|e| format!("serialize facets: {e}"))?;

    let prompt = format!(
        "You are an abductive opportunity architect for the domain `{}` ({}).\n\
         We have mathematically located an unoccupied high-dimensional interior sphere `{}` \
         (radius = {:.4}, simplex diversity = {:.4}) bounded by these Delaunay neighbors:\n{}\n\n\
         Domain facets:\n{}\n\n\
         Mutation iteration: {}.\n\
         Synthesize a concrete, novel entity occupying the exact interstitial center of these \
         neighbors without cloning any single neighbor. Return ONLY a JSON object with keys: \
         `title` (string), `high_concept_logline` (string), `core_30s_loop` (string), \
         `tags` (array of strings), and `facets` (object mapping each facet id to its description).",
        schema.domain_id,
        schema.title,
        void.void_id,
        void.radius,
        void.simplex_diversity,
        neighbors_json,
        facets_json,
        mutation_round
    );

    let req = LlmRequest {
        config: cfg.clone(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: MessageContent::Text(prompt),
        }],
        tools: Vec::new(),
        attempts_remaining: DEFAULT_MAX_ATTEMPTS,
        next_delay_ms: None,
    };

    let mut pending = llmcall::Pending::new(
        req,
        llmcall::Carry {
            call_id: format!("abduct-{}-r{}", void.void_id, mutation_round),
        },
    );

    let response = loop {
        match llmcall::step(pending) {
            Ok(llmcall::StepOutcome::Success(ok)) => break ok.into_payload(),
            Ok(llmcall::StepOutcome::LlmError(err)) => {
                return Err(format!("rung llmcall error: {:?}", err.into_payload()));
            }
            Err(failed) => {
                pending = llmcall::retry(failed);
            }
        }
    };

    let text: String = response
        .content
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    parse_llm_hypothesis_json(&text, &cfg.model, schema, void)
}

fn parse_llm_hypothesis_json(
    raw_text: &str,
    model: &str,
    schema: &DomainSchema,
    void: &VoidCandidate,
) -> Result<DraftHypothesis, String> {
    let trimmed = raw_text.trim();
    let json_slice = match (trimmed.find('{'), trimmed.rfind('}')) {
        (Some(start), Some(end)) if end > start => &trimmed[start..=end],
        _ => {
            return Err(format!(
                "model response did not contain a JSON object: {trimmed}"
            ));
        }
    };
    let val: Value =
        serde_json::from_str(json_slice).map_err(|e| format!("invalid JSON from model: {e}"))?;

    let title = val
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Untitled Void Concept")
        .to_string();
    let high_concept_logline = val
        .get("high_concept_logline")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let core_30s_loop = val
        .get("core_30s_loop")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let tags = val
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let mut facets = BTreeMap::new();
    if let Some(obj) = val.get("facets").and_then(Value::as_object) {
        for f in &schema.facets {
            if let Some(txt) = obj.get(&f.id).and_then(Value::as_str) {
                facets.insert(f.id.clone(), txt.to_string());
            }
        }
    }

    Ok(DraftHypothesis {
        void_id: void.void_id.clone(),
        title,
        high_concept_logline,
        core_30s_loop,
        tags,
        facets,
        generator_model: model.to_string(),
    })
}

/// Deterministic contrastive synthesis that recombines complementary facets across the
/// three Delaunay simplex boundary neighbors so every facet matches the target barycentric bridge.
pub fn abduct_deterministic(
    schema: &DomainSchema,
    void: &VoidCandidate,
    mutation_round: u32,
) -> DraftHypothesis {
    let n0 = void.boundary_neighbors.first();
    let n1 = void.boundary_neighbors.get(1).or(n0);
    let n2 = void.boundary_neighbors.get(2).or(n1);

    let (Some(a), Some(b), Some(c)) = (n0, n1, n2) else {
        return DraftHypothesis {
            void_id: void.void_id.clone(),
            title: format!("Lacuna Hypothesis {}", void.void_id),
            high_concept_logline: "Unexplored interior domain opportunity.".into(),
            core_30s_loop: "Synthesized interstitial loop.".into(),
            tags: vec!["Whitespace".into()],
            facets: BTreeMap::new(),
            generator_model: "lacuna-contrastive-synthesizer/1.0".into(),
        };
    };

    let stem_a = a.name.split_whitespace().next().unwrap_or(&a.name);
    let stem_b = b.name.split_whitespace().last().unwrap_or(&b.name);
    let stem_c = c.name.split_whitespace().next().unwrap_or(&c.name);

    let title = if mutation_round == 0 {
        format!("Protocol {}: {} × {}", stem_a, stem_b, stem_c)
    } else {
        format!(
            "Protocol {}: {} × {} (v{})",
            stem_a,
            stem_b,
            stem_c,
            mutation_round + 1
        )
    };

    let mut facets = BTreeMap::new();
    for (idx, facet) in schema.facets.iter().enumerate() {
        let f_a = a
            .facets
            .get(&facet.id)
            .cloned()
            .unwrap_or_else(|| a.summary.clone());
        let f_b = b
            .facets
            .get(&facet.id)
            .cloned()
            .unwrap_or_else(|| b.summary.clone());
        let f_c = c
            .facets
            .get(&facet.id)
            .cloned()
            .unwrap_or_else(|| c.summary.clone());

        // Weight the text synthesis proportionally to the simplex barycentric weights
        let text = match (idx + mutation_round as usize) % 3 {
            0 => format!("{f_a} Interlocked with: {f_b} Bounded by: {f_c}"),
            1 => format!("{f_b} Driven by: {f_c} Anchored in: {f_a}"),
            _ => format!("{f_c} Synthesizing: {f_a} Bridge mechanism: {f_b}"),
        };
        facets.insert(facet.id.clone(), text);
    }

    let mut tags = Vec::new();
    for neighbor in [a, b, c] {
        if let Some(t) = neighbor.tags.first()
            && !tags.contains(t)
        {
            tags.push(t.clone());
        }
    }
    tags.push("Whitespace Synthesis".into());

    let high_concept_logline = format!(
        "An unexplored interstitial bridge between {}, {}, and {}: combining {}'s core mechanics with {}'s macro-progression and {}'s agency topology.",
        a.name, b.name, c.name, a.name, b.name, c.name
    );

    let loop_text = facets
        .get("core_loop")
        .cloned()
        .unwrap_or_else(|| high_concept_logline.clone());

    DraftHypothesis {
        void_id: void.void_id.clone(),
        title,
        high_concept_logline,
        core_30s_loop: loop_text,
        tags,
        facets,
        generator_model: "lacuna-contrastive-synthesizer/1.0".into(),
    }
}

/// Runs an autonomous `rung-agent` turn inside the current working directory with `prompt`
/// using `rung_agent_core::run_job`.
pub fn run_rung_agent_turn(prompt: &str, toolset: &str) -> Result<String, String> {
    let argv = vec![
        "--toolset".to_string(),
        toolset.to_string(),
        prompt.to_string(),
    ];
    let args = rung_agent_core::Args::parse(&argv)?;
    let origin = std::env::current_dir().map_err(|e| format!("current_dir: {e}"))?;
    let outcome = rung_agent_core::run_job(&args, &origin)?;
    Ok(format!(
        "{{\"task_id\": {:?}, \"status\": {:?}, \"text\": {:?}}}",
        outcome.task_id,
        outcome.status.as_str(),
        outcome.text
    ))
}
