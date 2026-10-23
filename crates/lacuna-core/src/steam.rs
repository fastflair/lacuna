//! Built-in 5-axis Steam Games domain schema and catalog generator for the
//! flagship `@SyntheticBeef` 100,000+ Steam Games Whitespace Explorer benchmark.

use std::collections::BTreeMap;

use serde_json::json;

use crate::schema::{CorpusSnapshot, DEFAULT_EMBED_DIM, DomainEntity, DomainSchema, FacetSpec};

/// Returns the canonical 5-axis [`DomainSchema`] for Steam games, separating
/// gameplay mechanics from progression structure, social topology, aesthetic setting,
/// and cognitive/emotional hook.
pub fn steam_domain_schema() -> DomainSchema {
    DomainSchema {
        domain_id: "steam-games".into(),
        title: "Steam Games Whitespace Atlas".into(),
        description: "Decomposes video games into 5 orthogonal design axes to discover unexplored, commercially viable gameplay intersections bounded by high-demand incumbents.".into(),
        facets: vec![
            FacetSpec {
                id: "core_loop".into(),
                name: "Core Gameplay Loop & Mechanics".into(),
                description: "Moment-to-moment player verbs, physics, simulation rules, and tactical interactions.".into(),
                default_weight: 0.35,
            },
            FacetSpec {
                id: "progression".into(),
                name: "Macro-Structure & Progression".into(),
                description: "Run structure, knowledge-gating, tech trees, extraction loops, or narrative branching.".into(),
                default_weight: 0.25,
            },
            FacetSpec {
                id: "social_topology".into(),
                name: "Player & Agency Topology".into(),
                description: "Solo introspection, proximity-voice co-op, asymmetric roles, or asynchronous world-sharing.".into(),
                default_weight: 0.15,
            },
            FacetSpec {
                id: "aesthetic_world".into(),
                name: "Setting, Camera & Aesthetic".into(),
                description: "Visual perspective, world lore, sensory atmosphere, and art direction.".into(),
                default_weight: 0.10,
            },
            FacetSpec {
                id: "player_hook".into(),
                name: "Primary Cognitive / Emotional Hook".into(),
                description: "Why players love it: deductive epiphany, thermodynamic mastery, slapstick panic, or cozy flow.".into(),
                default_weight: 0.15,
            },
        ],
        demand_metric_name: "log_reviews".into(),
        quality_metric_name: "positive_review_ratio".into(),
    }
}

struct SeedGameSpec {
    app_id: u32,
    name: &'static str,
    summary: &'static str,
    tags: &'static [&'static str],
    core_loop: &'static str,
    progression: &'static str,
    social_topology: &'static str,
    aesthetic_world: &'static str,
    player_hook: &'static str,
    reviews: u64,
    positive_ratio: f64,
    price_usd: f64,
}

const CANONICAL_STEAM_ARCHETYPES: &[SeedGameSpec] = &[
    SeedGameSpec {
        app_id: 427520,
        name: "Factorio",
        summary: "Build and maintain automated factories on an alien planet with conveyor logistics, fluid networks, and train signaling.",
        tags: &[
            "Automation",
            "Base Building",
            "Resource Management",
            "Logistics",
        ],
        core_loop: "Conveyor belt routing, inserter throughput balancing, fluid pipe networks, and programmable rail signaling under wave defense.",
        progression: "Multi-tier science pack tech tree scaling from burner drills to megabase rocket launches and blueprint automation.",
        social_topology: "Solo or synchronous cooperative multiplayer shared factory engineering.",
        aesthetic_world: "Top-down 2D industrial dieselpunk machinery on a hostile insectoid frontier planet.",
        player_hook: "Systems optimization mastery, bottleneck eradication, and exponential throughput scaling.",
        reviews: 195_000,
        positive_ratio: 0.97,
        price_usd: 35.0,
    },
    SeedGameSpec {
        app_id: 653530,
        name: "Return of the Obra Dinn",
        summary: "An insurance adventure with minimal color: board an 1807 ghost ship and deduce the fate of all 60 crew members using a pocketwatch.",
        tags: &["Detective", "Mystery", "Deduction", "1-Bit"],
        core_loop: "Forensic freeze-frame diorama inspection, spatial audio clue correlation, and ledger cross-referencing without hand-holding.",
        progression: "Knowledge-gated deductive ledger validated in triplets of solved identities and causes of death.",
        social_topology: "Pure solo investigative contemplation.",
        aesthetic_world: "First-person 1-bit monochromatic dithering aboard an 1807 merchant sailing vessel.",
        player_hook: "Pure deductive epiphany and forensic detective satisfaction.",
        reviews: 28_000,
        positive_ratio: 0.96,
        price_usd: 19.99,
    },
    SeedGameSpec {
        app_id: 646570,
        name: "Slay the Spire",
        summary: "Craft a unique deck, encounter bizarre creatures, discover relics of immense power, and climb a procedurally generated spire.",
        tags: &[
            "Roguelike Deckbuilder",
            "Card Battler",
            "Turn-Based",
            "Strategy",
        ],
        core_loop: "Turn-based energy management, card draw manipulation, status synergy stacking, and telegraphed enemy intent countering.",
        progression: "Three-act branching node map with draft rewards, relic combinatorics, and 20 ascension difficulty tiers.",
        social_topology: "Solo strategic run optimization with daily leaderboard seeds.",
        aesthetic_world: "2D hand-illustrated dark fantasy climb up a living arcane tower.",
        player_hook: "Combinatorial synergy discovery and tight probabilistic risk management.",
        reviews: 148_000,
        positive_ratio: 0.97,
        price_usd: 24.99,
    },
    SeedGameSpec {
        app_id: 1966720,
        name: "Lethal Company",
        summary: "A co-op horror game about scavenging abandoned moons to sell scrap to the Company before your quota expires.",
        tags: &["Online Co-Op", "Horror", "Extraction", "Comedy"],
        core_loop: "First-person facility scavenging, weight-encumbered sprinting, terminal radar coordination, and monster evasion.",
        progression: "Three-day escalating profit quota cycle with ship equipment upgrades and deadlier moon hazards.",
        social_topology: "4-player proximity-voice co-op with spatial radio cutoffs and separated ship-operator vs field-scavenger roles.",
        aesthetic_world: "Lo-fi retro-industrial corporate dystopia across dark utilitarian bunkers and alien moons.",
        player_hook: "Slapstick emergent social panic and comedic acoustic tension.",
        reviews: 365_000,
        positive_ratio: 0.97,
        price_usd: 9.99,
    },
    SeedGameSpec {
        app_id: 753640,
        name: "Outer Wilds",
        summary: "An open-world mystery about a solar system trapped in an endless 22-minute time loop before its sun goes supernova.",
        tags: &["Exploration", "Space", "Time Loop", "Mystery"],
        core_loop: "Newtonian six-degree-of-freedom spacecraft piloting, zero-G jetpack platforming, and ancient alien text translation.",
        progression: "Pure knowledge-vania where nothing persists across 22-minute supernova loops except rumor-map insights in the player's mind.",
        social_topology: "Solo cosmic exploration.",
        aesthetic_world: "Handcrafted clockwork miniature solar system with dynamic orbital physics and campfire folk acoustics.",
        player_hook: "Cosmic awe, ontological curiosity, and paradigm-shifting revelations.",
        reviews: 64_000,
        positive_ratio: 0.95,
        price_usd: 24.99,
    },
    SeedGameSpec {
        app_id: 2379780,
        name: "Balatro",
        summary: "The poker roguelike: play illegal poker hands, discover game-changing jokers, and trigger adrenaline-pumping score combos.",
        tags: &["Deckbuilder", "Roguelike", "Card Game", "Synergy"],
        core_loop: "Poker hand construction, positional Joker multiplier sequencing, tarot deck mutation, and blind score threshold breaking.",
        progression: "Eight-ante escalating blind gauntlet with shop vouchers, booster packs, and stake modifiers.",
        social_topology: "Solo high-score engine building.",
        aesthetic_world: "Hypnotic CRT scanline shader aesthetic with tactile card physics and psychedelic synth audio.",
        player_hook: "Dopaminergic exponential number scaling and rule-bending engine alchemy.",
        reviews: 112_000,
        positive_ratio: 0.98,
        price_usd: 14.99,
    },
    SeedGameSpec {
        app_id: 239030,
        name: "Papers, Please",
        summary: "A dystopian document thriller: work as a border checkpoint immigration inspector in the fictional communist state of Arstotzka.",
        tags: &["Political Sim", "Deduction", "Dystopian", "Narrative"],
        core_loop: "Tactile desk document inspection, rulebook discrepancy stamping, fingerprint verification, and contraband scanning under time pressure.",
        progression: "31-day calendar survival balancing family rent/food/heat budgets against escalating ministry security directives and rebel moral choices.",
        social_topology: "Solo moral-bureaucratic pressure cooker.",
        aesthetic_world: "Bleak 1982 pixel-art Eastern Bloc border booth UI.",
        player_hook: "Bureaucratic vigilance fused with moral complicity under authoritarian stress.",
        reviews: 69_000,
        positive_ratio: 0.96,
        price_usd: 9.99,
    },
    SeedGameSpec {
        app_id: 881100,
        name: "Noita",
        summary: "A magical action roguelite set in a world where every pixel is physically simulated: burn, melt, freeze, and evaporate everything.",
        tags: &["Pixel Physics", "Roguelike", "Wand Crafting", "Simulation"],
        core_loop: "Cellular-automata falling-sand chemistry manipulation and programmable spell-wand compiler crafting.",
        progression: "Permadeath underground descent with chaotic perk altars and sprawling parallel-world endgame secrets.",
        social_topology: "Solo experimental alchemy.",
        aesthetic_world: "2D side-scrolling pixel-art subterranean alchemical caverns.",
        player_hook: "Unrestrained emergent physics chaos and god-tier wand programming.",
        reviews: 67_000,
        positive_ratio: 0.95,
        price_usd: 19.99,
    },
    SeedGameSpec {
        app_id: 1332010,
        name: "Against the Storm",
        summary: "A dark fantasy city builder where you must rebuild civilization in the face of apocalyptic torrential rains.",
        tags: &[
            "City Builder",
            "Roguelite",
            "Strategy",
            "Resource Management",
        ],
        core_loop: "Multi-species villager resolve management, production chain recipe routing, forest glade opening, and Queen impatience balancing.",
        progression: "Short 45-minute roguelite settlement runs feeding into an overworld blightstorm meta-map prior to cycle reset.",
        social_topology: "Solo tactical city-building sprints.",
        aesthetic_world: "Isometric rain-drenched dark fantasy woodland with anthropomorphic beavers, harpies, and lizards.",
        player_hook: "Opening-phase city-builder adaptation without late-game stagnation.",
        reviews: 29_000,
        positive_ratio: 0.95,
        price_usd: 29.99,
    },
    SeedGameSpec {
        app_id: 1167630,
        name: "Teardown",
        summary: "Plan the perfect heist using creative problem solving, brute force, and fully destructible voxel environments.",
        tags: &["Voxel Destruction", "Heist", "Physics", "Puzzle"],
        core_loop: "Pre-alarm environment demolition, plank/cable path rigging, vehicle staging, and 60-second sprint escape routing.",
        progression: "Mission-based heist contracts unlocking demolition tools, explosives, and sandbox maps.",
        social_topology: "Solo spatial heist planning and modding.",
        aesthetic_world: "Ray-traced first-person volumetric voxel physics world.",
        player_hook: "Creative architectural subversion and clockwork heist execution.",
        reviews: 95_000,
        positive_ratio: 0.96,
        price_usd: 29.99,
    },
    SeedGameSpec {
        app_id: 341800,
        name: "Keep Talking and Nobody Explodes",
        summary: "One player is trapped in a room with a ticking time bomb; the others have the manual to defuse it and cannot see the bomb.",
        tags: &["Asymmetric Co-Op", "Puzzle", "Communication", "Party"],
        core_loop: "Verbal symbol/wire state description, rulebook conditional logic decoding, and high-tempo spoken instruction execution.",
        progression: "Escalating modular bomb configurations with tighter countdowns and needy distraction modules.",
        social_topology: "Strict asymmetric information split between one blindfolded defuser and external manual experts.",
        aesthetic_world: "First-person tactile suitcase bomb workbench paired with a printable PDF technical manual.",
        player_hook: "High-bandwidth human communication precision under countdown panic.",
        reviews: 12_500,
        positive_ratio: 0.97,
        price_usd: 14.99,
    },
    SeedGameSpec {
        app_id: 986130,
        name: "Shadows of Doubt",
        summary: "An immersive sandbox detective stealth game set in a fully simulated sci-fi noir city of routines, apartments, and phone records.",
        tags: &["Immersive Sim", "Detective", "Procedural", "Stealth"],
        core_loop: "Lockpicking, CCTV/vmail hacking, citizen schedule stalking, fingerprint dusting, and corkboard string-pinning.",
        progression: "Open-ended procedural murder and espionage cases funding apartment furnishings and cybernetic sync-disk upgrades.",
        social_topology: "Solo systemic immersive-sim sandbox.",
        aesthetic_world: "First-person rain-slicked voxel neo-noir alternate-1979 industrial metropolis.",
        player_hook: "Self-directed unscripted sleuthing in a living clockwork city.",
        reviews: 13_200,
        positive_ratio: 0.91,
        price_usd: 24.99,
    },
];

/// Builds the curated canonical Steam archetype snapshot (12 distinct high-signal games)
/// with deterministic 64D embeddings pre-populated across all 5 design facets.
pub fn steam_seed_snapshot() -> CorpusSnapshot {
    generate_steam_catalog(CANONICAL_STEAM_ARCHETYPES.len())
}

/// Generates a Steam catalog snapshot with `count` games.
///
/// The first 12 entries are the exact canonical Steam archetypes; additional entries
/// up to `count` (e.g., 100, 1,000, or 100,000) are deterministically synthesized variations
/// clustered around overcrowded incumbent subgenres so whitespace basins remain crisp and
/// measurable at scale.
pub fn generate_steam_catalog(count: usize) -> CorpusSnapshot {
    let schema = steam_domain_schema();
    let target = count.max(3);
    let mut entities = Vec::with_capacity(target);

    for (idx, spec) in CANONICAL_STEAM_ARCHETYPES.iter().take(target).enumerate() {
        let mut facets = BTreeMap::new();
        facets.insert("core_loop".into(), spec.core_loop.into());
        facets.insert("progression".into(), spec.progression.into());
        facets.insert("social_topology".into(), spec.social_topology.into());
        facets.insert("aesthetic_world".into(), spec.aesthetic_world.into());
        facets.insert("player_hook".into(), spec.player_hook.into());

        let mut entity = DomainEntity {
            id: format!("steam:{}", spec.app_id),
            name: spec.name.into(),
            summary: spec.summary.into(),
            tags: spec.tags.iter().map(|&s| s.to_string()).collect(),
            facets,
            vectors: BTreeMap::new(),
            demand_signal: (1.0 + spec.reviews as f64).ln(),
            quality_signal: spec.positive_ratio,
            metadata: json!({
                "app_id": spec.app_id,
                "reviews": spec.reviews,
                "positive_ratio": spec.positive_ratio,
                "price_usd": spec.price_usd,
                "archetype_rank": idx + 1
            }),
        };
        entity.ensure_embedded(&schema, DEFAULT_EMBED_DIM);
        entities.push(entity);
    }

    // If the caller asks for a larger benchmark (e.g., 500 or 100,000 games), populate
    // realistic genre-cluster variants around the canonical archetypes.
    let modifiers = [
        ("Neon", "cyberpunk synthwave", 0.84),
        ("Iron", "dieselpunk trench", 0.79),
        ("Astral", "deep-space orbital", 0.88),
        ("Clockwork", "steampunk brass", 0.82),
        ("Verdant", "solarpunk overgrown", 0.90),
        ("Abyssal", "benthic submarine", 0.86),
    ];

    for extra_idx in CANONICAL_STEAM_ARCHETYPES.len()..target {
        let base = &CANONICAL_STEAM_ARCHETYPES[extra_idx % CANONICAL_STEAM_ARCHETYPES.len()];
        let (prefix, flavor, qual_scale) = modifiers[extra_idx % modifiers.len()];
        let app_id = 5_000_000 + (extra_idx as u32);
        let reviews = ((base.reviews / 12) + ((extra_idx as u64 * 137) % 4500)).max(50);
        let quality = (base.positive_ratio * qual_scale).clamp(0.55, 0.96);

        let mut facets = BTreeMap::new();
        facets.insert(
            "core_loop".into(),
            format!("{} ({} variant #{})", base.core_loop, flavor, extra_idx),
        );
        facets.insert("progression".into(), base.progression.into());
        facets.insert("social_topology".into(), base.social_topology.into());
        facets.insert(
            "aesthetic_world".into(),
            format!("{} in a {} setting", base.aesthetic_world, flavor),
        );
        facets.insert("player_hook".into(), base.player_hook.into());

        let mut entity = DomainEntity {
            id: format!("steam:{app_id}"),
            name: format!("{prefix} {} #{extra_idx}", base.name),
            summary: format!("{} ({flavor} indie cluster entry)", base.summary),
            tags: base.tags.iter().map(|&s| s.to_string()).collect(),
            facets,
            vectors: BTreeMap::new(),
            demand_signal: (1.0 + reviews as f64).ln(),
            quality_signal: quality,
            metadata: json!({
                "app_id": app_id,
                "reviews": reviews,
                "positive_ratio": quality,
                "price_usd": base.price_usd,
                "synthetic_cluster_of": base.name
            }),
        };
        entity.ensure_embedded(&schema, DEFAULT_EMBED_DIM);
        entities.push(entity);
    }

    CorpusSnapshot { schema, entities }
}
