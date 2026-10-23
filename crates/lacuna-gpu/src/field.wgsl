// lacuna WGSL Topographic Supply/Demand/Whitespace Potential Field Shader
// Evaluates continuous kernel density (incumbent saturation peaks) vs annular
// commercial demand gravity to illuminate high-value interior whitespace basins.

struct EntityPoint {
    pos: vec2<f32>,
    demand: f32,
    quality: f32,
};

struct FieldUniforms {
    entity_count: u32,
    sigma_supply: f32,
    sigma_demand: f32,
    whitespace_gain: f32,
};

@group(0) @binding(0) var<storage, read> entities: array<EntityPoint>;
@group(0) @binding(1) var<uniform> params: FieldUniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    let p = positions[idx];
    var out: VertexOutput;
    out.clip_position = vec4<f32>(p, 0.0, 1.0);
    out.world_pos = p;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.world_pos;
    let inv_2s_supply = 1.0 / (2.0 * params.sigma_supply * params.sigma_supply);
    let inv_2s_demand = 1.0 / (2.0 * params.sigma_demand * params.sigma_demand);

    var supply: f32 = 0.0;
    var demand: f32 = 0.0;

    for (var i: u32 = 0u; i < params.entity_count; i = i + 1u) {
        let e = entities[i];
        let d = p - e.pos;
        let r2 = dot(d, d);
        supply = supply + exp(-r2 * inv_2s_supply);
        let pull = max(e.demand, 0.1) * clamp(e.quality, 0.1, 1.0);
        let ring = min(r2 / (params.sigma_demand * params.sigma_demand), 2.5) * exp(-r2 * inv_2s_demand);
        demand = demand + pull * ring;
    }

    let norm_s = clamp(supply / max(f32(params.entity_count) * 0.18, 1.0), 0.0, 1.0);
    let norm_d = clamp(demand / max(f32(params.entity_count) * 1.4, 1.0), 0.0, 1.0);
    let envelope = clamp(1.0 - 0.35 * dot(p, p), 0.1, 1.0);
    let whitespace = clamp(norm_d * pow(1.0 - norm_s, 1.35) * envelope * params.whitespace_gain, 0.0, 1.0);

    // Topographic iso-contour lines
    let iso = abs(fract(whitespace * 10.0) - 0.5);
    let contour = smoothstep(0.03, 0.08, iso);

    let bg = vec3<f32>(0.04, 0.06, 0.10);
    let peak_col = vec3<f32>(0.85, 0.26, 0.32) * norm_s;
    let basin_col = vec3<f32>(0.06, 0.88, 0.92) * whitespace;
    let color = (bg + peak_col + basin_col) * (0.82 + 0.18 * contour);

    return vec4<f32>(color, 1.0);
}
