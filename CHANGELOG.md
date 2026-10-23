# Changelog

## 0.2.0 (2026-10-23)

- `lacuna-core::numeric`: invertible numeric facet encoding (`NumericAxis`, `encode_numeric`, `decode_numeric`) so void centers in decision-parameter domains decode to concrete values.
- `VoidSolverConfig::anchor_distance_window` (default `[0.35, 0.92]`, unchanged behavior) and `#[serde(default)]` on every config field so JSON configs may be partial.
- `lacuna-wasm`: wasm32 C-ABI JSON bridge (`lacuna_call`) with zero imports, for in-browser use by CAPPZ HARVEST.
- `skills/farm-economics`: onboarding lacuna as the opportunity-search stage of a farm economic discovery engine.

## 0.1.0 (2026-10-23)

- Initial release of `lacuna`: domain-agnostic high-dimensional whitespace analyzer, finder, visualizer, and explorer.
- `lacuna-core`: Four-layer universal input contract (`DomainSchema`, `FacetSpec`, `DomainEntity`, `FacetVector`), high-D interior Delaunay empty-sphere solver, supply-vs-demand gold whitespace ranker, bounded 2D Voronoi tessellation, and built-in 5-axis Steam Games benchmark (`@SyntheticBeef` 100k+ Steam challenge).
- `lacuna-rung`: Type-enforced `rung::ladder!` discovery pipeline (`VoidCandidate => SynthesizedDraft => DecidedViability => VerifiedOpportunity`), `rung_std::decide` (`Decider` / `JevDecider` / `Recorded`) viability judge, `rung_std::llm` abduction, and `rung-agent-core` turn engine integration.
- `lacuna-gpu`: WGSL compute and fragment shaders (`field.wgsl`), `naga` shader validation, and interactive WebGPU + SVG topographic field rendering.
- `lacuna-cli`: Agent-first `lacuna` CLI (`schema`, `ingest`, `voids`, `probe`, `synthesize`, `field`, `agent`, `demo steam`) with deterministic `--json` output across all commands.
