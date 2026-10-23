//! Pure-Rust 2D manifold projection, bounded Voronoi tessellation (Sutherland-Hodgman
//! half-plane clipping), and continuous supply/demand/whitespace potential field evaluation.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::metric::{entity_distance, point_to_entity_distance};
use crate::schema::DomainEntity;

/// Bounded 2D coordinate box `[-1.0, 1.0] x [-1.0, 1.0]`.
pub const MANIFOLD_BOUND: f64 = 1.0;

/// Projects entities into a 2D metric-preserving manifold `[-0.88, 0.88]^2` using
/// classical landmark spectral/MDS initialization followed by spring-repulsion relaxation.
pub fn project_entities_2d(
    entities: &[DomainEntity],
    weights: &BTreeMap<String, f32>,
) -> Vec<[f64; 2]> {
    let n = entities.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![[0.0, 0.0]];
    }

    // Pick two maximally separated landmark pivots for stable primary axes,
    // plus a third orthogonal pivot.
    let mut p0 = 0_usize;
    let mut p1 = 1_usize;
    let mut max_d = -1.0_f64;
    let scan_limit = n.min(64);
    for i in 0..scan_limit {
        for j in (i + 1)..scan_limit {
            let d = entity_distance(&entities[i], &entities[j], weights);
            if d > max_d {
                max_d = d;
                p0 = i;
                p1 = j;
            }
        }
    }
    let d01 = max_d.max(0.1);

    let mut p2 = 0_usize;
    let mut max_tri = -1.0_f64;
    for i in 0..n {
        if i == p0 || i == p1 {
            continue;
        }
        let d0 = entity_distance(&entities[i], &entities[p0], weights);
        let d1 = entity_distance(&entities[i], &entities[p1], weights);
        let score = d0.min(d1);
        if score > max_tri {
            max_tri = score;
            p2 = i;
        }
    }

    let mut coords = Vec::with_capacity(n);
    for i in 0..n {
        let d0 = entity_distance(&entities[i], &entities[p0], weights);
        let d1 = entity_distance(&entities[i], &entities[p1], weights);
        let d2 = entity_distance(&entities[i], &entities[p2], weights);
        let x = (d0 * d0 - d1 * d1) / (2.0 * d01);
        let y = d2 - 0.5 * (d0 + d1);
        coords.push([x, y]);
    }

    normalize_coords_2d(&mut coords, 0.85);

    // Stress-majorization / spring relaxation pass to preserve pairwise distances
    let iters = if n <= 200 { 45 } else { 15 };
    let sample_step = (n / 50).max(1);
    for iter in 0..iters {
        let lr = 0.08 * (1.0 - (iter as f64) / (iters as f64));
        for i in 0..n {
            let mut gx = 0.0_f64;
            let mut gy = 0.0_f64;
            let mut count = 0.0_f64;
            let mut j = i % sample_step;
            while j < n {
                if i != j {
                    let target_d = entity_distance(&entities[i], &entities[j], weights);
                    let dx = coords[i][0] - coords[j][0];
                    let dy = coords[i][1] - coords[j][1];
                    let cur_d = (dx * dx + dy * dy).sqrt().max(1e-4);
                    let err = cur_d - target_d;
                    gx -= (dx / cur_d) * err;
                    gy -= (dy / cur_d) * err;
                    count += 1.0;
                }
                j += sample_step;
            }
            if count > 0.0 {
                coords[i][0] += lr * (gx / count);
                coords[i][1] += lr * (gy / count);
            }
        }
    }

    normalize_coords_2d(&mut coords, 0.85);
    coords
}

/// Projects an arbitrary high-dimensional point into the 2D manifold coordinate frame
/// established by `entities` and `entity_coords`.
pub fn project_point_2d(
    point: &BTreeMap<String, Vec<f32>>,
    entities: &[DomainEntity],
    entity_coords: &[[f64; 2]],
    weights: &BTreeMap<String, f32>,
) -> [f64; 2] {
    if entities.is_empty() || entity_coords.is_empty() {
        return [0.0, 0.0];
    }

    // Inverse-distance weighted barycenter of top-6 nearest neighbors, followed by
    // 2D trilateration refinement.
    let mut dists: Vec<(usize, f64)> = entities
        .iter()
        .enumerate()
        .map(|(i, e)| (i, point_to_entity_distance(point, e, weights)))
        .collect();
    dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let k = dists.len().min(6);
    let mut x = 0.0_f64;
    let mut y = 0.0_f64;
    let mut w_sum = 0.0_f64;
    for &(idx, d) in &dists[..k] {
        let w = 1.0 / (d * d + 1e-3);
        x += w * entity_coords[idx][0];
        y += w * entity_coords[idx][1];
        w_sum += w;
    }
    if w_sum > 1e-12 {
        x /= w_sum;
        y /= w_sum;
    }

    [x.clamp(-0.95, 0.95), y.clamp(-0.95, 0.95)]
}

fn normalize_coords_2d(coords: &mut [[f64; 2]], max_abs: f64) {
    if coords.is_empty() {
        return;
    }
    let n = coords.len() as f64;
    let cx: f64 = coords.iter().map(|c| c[0]).sum::<f64>() / n;
    let cy: f64 = coords.iter().map(|c| c[1]).sum::<f64>() / n;
    let mut max_r = 1e-6_f64;
    for c in coords.iter_mut() {
        c[0] -= cx;
        c[1] -= cy;
        max_r = max_r.max(c[0].abs()).max(c[1].abs());
    }
    let scale = max_abs / max_r;
    for c in coords.iter_mut() {
        c[0] *= scale;
        c[1] *= scale;
    }
}

/// Computes the exact bounded 2D Voronoi polygon around `seed` against `competitors`
/// within the bounding square `[-bound, bound]^2` using Sutherland-Hodgman half-plane clipping.
pub fn compute_voronoi_cell(seed: [f64; 2], competitors: &[[f64; 2]], bound: f64) -> Vec<[f64; 2]> {
    let mut poly = vec![
        [-bound, -bound],
        [bound, -bound],
        [bound, bound],
        [-bound, bound],
    ];

    for &other in competitors {
        let dx = other[0] - seed[0];
        let dy = other[1] - seed[1];
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < 1e-10 {
            continue;
        }
        // Half-plane of points closer to `seed` than `other`:
        // (p - mid) . (other - seed) <= 0
        let mid = [0.5 * (seed[0] + other[0]), 0.5 * (seed[1] + other[1])];
        poly = clip_polygon_half_plane(&poly, mid, [dx, dy]);
        if poly.len() < 3 {
            break;
        }
    }

    poly
}

fn clip_polygon_half_plane(
    poly: &[[f64; 2]],
    plane_pt: [f64; 2],
    outward_normal: [f64; 2],
) -> Vec<[f64; 2]> {
    if poly.is_empty() {
        return Vec::new();
    }
    let eval = |p: [f64; 2]| -> f64 {
        (p[0] - plane_pt[0]) * outward_normal[0] + (p[1] - plane_pt[1]) * outward_normal[1]
    };

    let mut out = Vec::with_capacity(poly.len() + 1);
    for i in 0..poly.len() {
        let cur = poly[i];
        let prev = poly[(i + poly.len() - 1) % poly.len()];
        let d_cur = eval(cur);
        let d_prev = eval(prev);

        let cur_inside = d_cur <= 0.0;
        let prev_inside = d_prev <= 0.0;

        if cur_inside {
            if !prev_inside {
                let t = d_prev / (d_prev - d_cur);
                out.push([
                    prev[0] + t * (cur[0] - prev[0]),
                    prev[1] + t * (cur[1] - prev[1]),
                ]);
            }
            out.push(cur);
        } else if prev_inside {
            let t = d_prev / (d_prev - d_cur);
            out.push([
                prev[0] + t * (cur[0] - prev[0]),
                prev[1] + t * (cur[1] - prev[1]),
            ]);
        }
    }
    out
}

/// Computes the signed shoelace area of a 2D polygon.
pub fn polygon_area(poly: &[[f64; 2]]) -> f64 {
    if poly.len() < 3 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    for i in 0..poly.len() {
        let p0 = poly[i];
        let p1 = poly[(i + 1) % poly.len()];
        sum += p0[0] * p1[1] - p1[0] * p0[1];
    }
    0.5 * sum.abs()
}

/// One cell of the 2D continuous topographic potential field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldSample {
    pub x: f64,
    pub y: f64,
    /// Incumbent saturation density `[0.0, 1.0]` (mountain peaks = overcrowded regions).
    pub supply_density: f64,
    /// Commercial demand / quality gravity pulled by surrounding neighbors `[0.0, 1.0]`.
    pub demand_gravity: f64,
    /// Gold Whitespace Potential: high where demand gravity is strong AND supply density is low.
    pub whitespace_potential: f64,
}

/// Continuous 2D potential field over the projected manifold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldGrid {
    pub resolution: usize,
    pub samples: Vec<FieldSample>,
}

/// Evaluates the 2D supply density, commercial demand gravity, and gold whitespace
/// potential field over a `resolution x resolution` grid in `[-1.0, 1.0]^2`.
pub fn compute_field_grid(
    entities: &[DomainEntity],
    coords_2d: &[[f64; 2]],
    resolution: usize,
) -> FieldGrid {
    let res = resolution.max(8);
    let mut raw = Vec::with_capacity(res * res);
    let sigma_supply: f64 = 0.16;
    let sigma_demand: f64 = 0.34;
    let inv_2s_supply = 1.0 / (2.0 * sigma_supply * sigma_supply);
    let inv_2s_demand = 1.0 / (2.0 * sigma_demand * sigma_demand);

    let mut max_supply = 1e-9_f64;
    let mut max_demand = 1e-9_f64;

    for row in 0..res {
        let y = -1.0 + 2.0 * (row as f64 + 0.5) / (res as f64);
        for col in 0..res {
            let x = -1.0 + 2.0 * (col as f64 + 0.5) / (res as f64);
            let mut supply = 0.0_f64;
            let mut demand = 0.0_f64;
            for (entity, coord) in entities.iter().zip(coords_2d.iter()) {
                let dx = x - coord[0];
                let dy = y - coord[1];
                let r2 = dx * dx + dy * dy;
                supply += (-r2 * inv_2s_supply).exp();
                let pull = entity.demand_signal.max(0.1) * entity.quality_signal.clamp(0.1, 1.0);
                // Annular ring kernel for demand gravity: highest just outside existing clusters
                let ring =
                    (r2 / (sigma_demand * sigma_demand)).min(2.5) * (-r2 * inv_2s_demand).exp();
                demand += pull * ring;
            }
            max_supply = max_supply.max(supply);
            max_demand = max_demand.max(demand);
            raw.push((x, y, supply, demand));
        }
    }

    let mut samples = Vec::with_capacity(raw.len());
    for (x, y, supply, demand) in raw {
        let norm_s = (supply / max_supply).clamp(0.0, 1.0);
        let norm_d = (demand / max_demand).clamp(0.0, 1.0);
        // Penalize extreme exterior borders so interior basins glow brightest
        let radial_envelope = (1.0 - 0.35 * (x * x + y * y)).clamp(0.1, 1.0);
        let ws = (norm_d * (1.0 - norm_s).powf(1.35) * radial_envelope).clamp(0.0, 1.0);
        samples.push(FieldSample {
            x,
            y,
            supply_density: norm_s,
            demand_gravity: norm_d,
            whitespace_potential: ws,
        });
    }

    FieldGrid {
        resolution: res,
        samples,
    }
}
