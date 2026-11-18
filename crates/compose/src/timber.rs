//! Timber: strip flooring and knotty wood grain.
//!
//! Both repeat every unit in U and V, so they tile without seams. The grain runs along U.

use texturecraft_geom::glam::{Vec2, Vec3};
use texturecraft_texel::noise;

use crate::organic::{lattice_fbm, lattice_noise, smoothstep, unit};

/// Strip flooring: long boards laid along U in rows, each row cut into pieces whose butt joints
/// are staggered from row to row (0 = gap). Every piece has a tone and growth rings of its own.
pub(crate) fn strip_flooring(p: Vec3, seed: u32) -> f32 {
    const ROWS: i32 = 6;
    // Pieces along a row, per unit
    const PIECES: f32 = 2.0;
    let u = unit(p);
    let y = u.y * ROWS as f32;
    let row = y.floor() as i32;
    let fy = y - row as f32;

    // Each row starts its pieces somewhere else
    let row_hash = |k: i32| noise::cell_hash(row.rem_euclid(ROWS), k, 0, seed ^ 0x13);
    let x = (u.x + row_hash(0)) * PIECES;
    let piece = x.floor() as i32;
    let fx = x - piece as f32;
    let h = |k: i32| noise::cell_hash(row.rem_euclid(ROWS), piece.rem_euclid(PIECES as i32), k + 1, seed ^ 0x13);

    // Distances to the board's edge and to the butt joint, in units
    let to_edge = fy.min(1.0 - fy) / ROWS as f32;
    let to_joint = fx.min(1.0 - fx) / PIECES;
    let gap = smoothstep(0.0012, 0.0035, to_edge.min(to_joint));

    // Growth rings seen on the face of the board: long cathedrals that wander along the piece
    let warp = lattice_fbm(u, &[(3, 0.6), (7, 0.4)], seed ^ 0x24) - 0.5;
    let rings = fy * (1.6 + 1.2 * h(0)) + h(1) * 7.0 + warp * 2.2;
    let ring = 0.5 + 0.5 * (std::f32::consts::TAU * rings).cos();
    let fibre = lattice_noise(u, (12, 240), seed ^ 0x35);

    let tone = 0.42 + 0.4 * h(2);
    (gap * tone * (0.68 + 0.32 * ring.powf(1.4)) * (0.86 + 0.14 * fibre)).clamp(0.0, 1.0)
}

/// Knotty wood: growth rings running along U that bend around knots, tight circles at the heart
/// of each knot and a dark core. Knots sit on a jittered grid whose cells wrap.
pub(crate) fn knotty_wood(p: Vec3, seed: u32) -> f32 {
    const RINGS: f32 = 9.0;
    const CELLS: i32 = 3;
    let u = unit(p);

    // Rings across the board, wandering a little
    let wander = (lattice_fbm(u, &[(2, 0.6), (5, 0.4)], seed ^ 0x14) - 0.5) * 1.4;
    let base = u.y * RINGS + wander;
    let mut t = base;
    let q = u * CELLS as f32;
    let (ci, cj) = (q.x.floor() as i32, q.y.floor() as i32);

    let mut core = 0.0f32;
    for j in (cj - 1)..=(cj + 1) {
        for i in (ci - 1)..=(ci + 1) {
            let h = |k: i32| noise::cell_hash(i.rem_euclid(CELLS), j.rem_euclid(CELLS), k, seed ^ 0x47);
            // Not every cell has a knot
            if h(0) > 0.45 {
                continue;
            }

            let center = Vec2::new(i as f32 + 0.2 + 0.6 * h(1), j as f32 + 0.2 + 0.6 * h(2)) / CELLS as f32;
            let radius = 0.04 + 0.035 * h(3);
            // Knots are longer along the grain
            let d = u - center;
            let distance = Vec2::new(d.x * 0.5, d.y).length() / radius;
            // The rings turn into circles around the knot; further out they only bend
            let circle = distance * 1.9 + h(4) * 3.0 + (lattice_noise(u, (24, 24), seed ^ 0x66) - 0.5) * 0.8;
            // The ring a knot pulls towards is measured from where the knot sits in its own copy of
            // the unit: a knot just past the bottom edge shapes the rings along the top edge, and
            // those must come out as the rings along the bottom edge do
            let home_y = (j.rem_euclid(CELLS) as f32 + 0.2 + 0.6 * h(2)) / CELLS as f32;
            let ring_here = (home_y + d.y) * RINGS + wander;
            t += (circle - ring_here) * smoothstep(2.2, 0.8, distance);
            core = core.max(smoothstep(0.5, 0.2, distance));
        }
    }

    let ring = (0.5 + 0.5 * (std::f32::consts::TAU * t).cos()).powf(1.6);
    let fibre = lattice_noise(u, (10, 260), seed ^ 0x58);
    let base = 0.34 + 0.5 * ring;
    (base * (0.86 + 0.14 * fibre) * (1.0 - 0.8 * core)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flooring_has_gaps_between_boards_and_pieces() {
        let grid = |size: usize| (0..size * size).map(move |i| Vec3::new((i % size) as f32 / size as f32, (i / size) as f32 / size as f32, 0.0));
        let gaps = grid(256).filter(|p| strip_flooring(*p, 1) < 0.02).count();
        assert!((300..6000).contains(&gaps), "gaps cover {gaps} of 65536 samples");

        // The joints of neighbouring rows do not line up
        let joints = |y: f32| (0..400).filter(|i| strip_flooring(Vec3::new(*i as f32 / 400.0, y, 0.0), 1) < 0.02).map(|i| i / 4).collect::<Vec<_>>();
        assert_ne!(joints(0.5 / 6.0), joints(1.5 / 6.0), "the rows share their joints");
    }

    #[test]
    fn knotty_wood_has_dark_knots() {
        let grid = |size: usize| (0..size * size).map(move |i| Vec3::new((i % size) as f32 / size as f32, (i / size) as f32 / size as f32, 0.0));
        let dark = grid(128).filter(|p| knotty_wood(*p, 1) < 0.12).count();
        assert!(dark > 10, "only {dark} samples are knot-dark");
    }
}
