//! Multi-facet metric space operations and interior geodesic simplex geometry.

use std::collections::BTreeMap;

use crate::schema::{DomainEntity, DomainSchema, normalize_in_place};

/// Cosine distance `1.0 - cos(u, v)` clamped to `[0.0, 2.0]`.
pub fn cosine_distance(u: &[f32], v: &[f32]) -> f64 {
    if u.is_empty() || v.is_empty() {
        return 1.0;
    }
    let len = u.len().min(v.len());
    let mut dot = 0.0_f64;
    let mut nu = 0.0_f64;
    let mut nv = 0.0_f64;
    for i in 0..len {
        let a = f64::from(u[i]);
        let b = f64::from(v[i]);
        dot += a * b;
        nu += a * a;
        nv += b * b;
    }
    let denom = (nu * nv).sqrt();
    if denom < 1e-12 {
        1.0
    } else {
        (1.0 - (dot / denom)).clamp(0.0, 2.0)
    }
}

/// Weighted multi-facet cosine distance between two entities.
pub fn entity_distance(a: &DomainEntity, b: &DomainEntity, weights: &BTreeMap<String, f32>) -> f64 {
    point_to_entity_distance(&a.vectors, b, weights)
}

/// Weighted multi-facet cosine distance between a candidate point (`facet_id -> Vec<f32>`)
/// and a corpus entity.
pub fn point_to_entity_distance(
    point: &BTreeMap<String, Vec<f32>>,
    entity: &DomainEntity,
    weights: &BTreeMap<String, f32>,
) -> f64 {
    let mut total_w = 0.0_f64;
    let mut weighted_sum = 0.0_f64;
    for (facet_id, &w) in weights {
        if w <= 0.0 {
            continue;
        }
        if let (Some(u), Some(v)) = (point.get(facet_id), entity.vectors.get(facet_id)) {
            let wf = f64::from(w);
            weighted_sum += wf * cosine_distance(u, v);
            total_w += wf;
        }
    }
    if total_w < 1e-12 {
        1.0
    } else {
        weighted_sum / total_w
    }
}

/// Weighted multi-facet distance between two arbitrary multi-facet coordinate maps.
pub fn point_to_point_distance(
    p1: &BTreeMap<String, Vec<f32>>,
    p2: &BTreeMap<String, Vec<f32>>,
    weights: &BTreeMap<String, f32>,
) -> f64 {
    let mut total_w = 0.0_f64;
    let mut weighted_sum = 0.0_f64;
    for (facet_id, &w) in weights {
        if w <= 0.0 {
            continue;
        }
        if let (Some(u), Some(v)) = (p1.get(facet_id), p2.get(facet_id)) {
            let wf = f64::from(w);
            weighted_sum += wf * cosine_distance(u, v);
            total_w += wf;
        }
    }
    if total_w < 1e-12 {
        1.0
    } else {
        weighted_sum / total_w
    }
}

/// Computes a geodesic combination inside the interior simplex of `anchors` using
/// strictly positive barycentric coordinates `barycentric` (which are normalized to sum to 1).
///
/// Because every facet vector is interpolated as a positive convex combination of real
/// domain entities before unit normalization, the resulting point is guaranteed to lie
/// inside the geodesic convex hull of the domain manifold rather than drifting into
/// orthogonal off-manifold regions.
pub fn geodesic_simplex_point(
    anchors: &[&DomainEntity],
    barycentric: &[f64],
    schema: &DomainSchema,
) -> BTreeMap<String, Vec<f32>> {
    let mut result = BTreeMap::new();
    if anchors.is_empty() {
        return result;
    }

    let sum_b: f64 = barycentric.iter().copied().sum();
    let norm_b: Vec<f64> = if sum_b > 1e-12 {
        barycentric.iter().map(|b| b / sum_b).collect()
    } else {
        vec![1.0 / (anchors.len() as f64); anchors.len()]
    };

    for facet in &schema.facets {
        let dim = anchors
            .iter()
            .find_map(|e| e.vectors.get(&facet.id).map(Vec::len))
            .unwrap_or(64);
        let mut combined = vec![0.0_f32; dim];
        for (entity, &weight) in anchors.iter().zip(norm_b.iter()) {
            if let Some(vec) = entity.vectors.get(&facet.id) {
                for (dst, &src) in combined.iter_mut().zip(vec.iter()) {
                    *dst += (weight as f32) * src;
                }
            }
        }
        normalize_in_place(&mut combined);
        result.insert(facet.id.clone(), combined);
    }

    result
}

/// Optimizes barycentric weights `w` inside the interior simplex of `anchors`
/// (`w_i >= min_barycentric`, `sum(w) = 1`) via projected gradient ascent to maximize
/// the empty-sphere radius `min_{e in corpus} dist(x(w), e)`.
pub fn optimize_interior_simplex_center(
    anchors: &[&DomainEntity],
    corpus: &[DomainEntity],
    schema: &DomainSchema,
    weights: &BTreeMap<String, f32>,
) -> (BTreeMap<String, Vec<f32>>, Vec<f64>, f64) {
    let k = anchors.len();
    if k == 0 {
        return (BTreeMap::new(), Vec::new(), 0.0);
    }
    let min_bary = (0.15 / (k as f64)).min(1.0 / (k as f64));
    let mut bary = vec![1.0 / (k as f64); k];
    let mut best_bary = bary.clone();
    let mut best_point = geodesic_simplex_point(anchors, &bary, schema);
    let mut best_radius = min_distance_to_corpus(&best_point, corpus, weights);

    // Coordinate-wise simplex search with shrinking step size
    let steps = [0.12_f64, 0.06, 0.03, 0.015];
    for &delta in &steps {
        for _iter in 0..6 {
            let mut improved = false;
            for i in 0..k {
                for j in 0..k {
                    if i == j {
                        continue;
                    }
                    if bary[j] - delta < min_bary {
                        continue;
                    }
                    let mut candidate_bary = bary.clone();
                    candidate_bary[i] += delta;
                    candidate_bary[j] -= delta;
                    let candidate_pt = geodesic_simplex_point(anchors, &candidate_bary, schema);
                    let radius = min_distance_to_corpus(&candidate_pt, corpus, weights);
                    if radius > best_radius + 1e-6 {
                        best_radius = radius;
                        best_bary = candidate_bary.clone();
                        best_point = candidate_pt;
                        bary = candidate_bary;
                        improved = true;
                    }
                }
            }
            if !improved {
                break;
            }
        }
    }

    (best_point, best_bary, best_radius)
}

/// Computes `min_{e in corpus} dist(point, e)`.
pub fn min_distance_to_corpus(
    point: &BTreeMap<String, Vec<f32>>,
    corpus: &[DomainEntity],
    weights: &BTreeMap<String, f32>,
) -> f64 {
    corpus
        .iter()
        .map(|e| point_to_entity_distance(point, e, weights))
        .fold(f64::INFINITY, f64::min)
}
