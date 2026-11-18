//! Organic procedural surfaces: fur, clouds, reptile scales, bark, coral and camouflage.
//!
//! Every pattern repeats every unit in U and V, tones and details included, so it tiles without
//! seams. A point is first moved by whole units into the unit square, so huge or non-finite
//! coordinates keep their precision and never overflow a cell index.

use texturecraft_geom::glam::{Vec2, Vec3};
use texturecraft_texel::noise;

pub(crate) fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// `p` moved by whole units into the unit square; a coordinate that is not finite becomes 0
pub(crate) fn unit(p: Vec3) -> Vec2 {
    let wrap = |x: f32| if x.is_finite() { x.rem_euclid(1.0) } else { 0.0 };
    Vec2::new(wrap(p.x), wrap(p.y))
}

/// Smooth value noise in 0..1 on a lattice of `cells` (across, down) per unit whose indices wrap,
/// so the noise repeats every unit
pub(crate) fn lattice_noise(at: Vec2, cells: (i32, i32), seed: u32) -> f32 {
    let q = Vec2::new(at.x * cells.0 as f32, at.y * cells.1 as f32);
    let (i, j) = (q.x.floor() as i32, q.y.floor() as i32);
    let f = q - Vec2::new(i as f32, j as f32);
    let s = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let corner = |di: i32, dj: i32| noise::cell_hash((i + di).rem_euclid(cells.0), (j + dj).rem_euclid(cells.1), 0, seed);
    let top = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * s.x;
    let bottom = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * s.x;
    top + (bottom - top) * s.y
}

/// Fractal [`lattice_noise`] in 0..1: octaves of (cells per unit, weight), the weights adding up to 1
pub(crate) fn lattice_fbm(at: Vec2, octaves: &[(i32, f32)], seed: u32) -> f32 {
    octaves.iter().enumerate().map(|(k, (cells, weight))| weight * lattice_noise(at, (*cells, *cells), seed ^ (k as u32).wrapping_mul(0x9E37))).sum()
}

/// One lock of fur: where its root lies in its cell and how it grows
#[derive(Clone, Copy, Default)]
struct Lock {
    /// Root, in cells from the corner of its cell
    root: Vec2,
    /// Length along its axis, in cells
    length: f32,
    sin: f32,
    cos: f32,
    /// Radians the axis turns over its length
    curl: f32,
    strength: f32,
}

/// Locks of fur on each side of the repeat
const FUR_CELLS: i32 = 6;

/// The locks of one seed, one per cell of the repeat. Every texel reads the whole neighbourhood of
/// roots, so the last seed's table is kept for the next call.
fn locks(seed: u32) -> std::rc::Rc<[Lock]> {
    thread_local! {
        static LAST: std::cell::RefCell<Option<(u32, std::rc::Rc<[Lock]>)>> = const { std::cell::RefCell::new(None) };
    }

    let build = || -> std::rc::Rc<[Lock]> {
        (0..FUR_CELLS * FUR_CELLS)
            .map(|index| {
                let h = |k: i32| noise::cell_hash(index % FUR_CELLS, index / FUR_CELLS, k, seed ^ 0x6D);
                let (sin, cos) = (0.1 + (h(3) * 2.0 - 1.0) * 0.45).sin_cos();
                Lock {
                    root: Vec2::new(0.5 + (h(1) - 0.5) * 0.7, 0.5 + (h(2) - 0.5) * 0.7),
                    length: 3.0 * (1.0 + (h(4) * 2.0 - 1.0) * 0.27),
                    sin,
                    cos,
                    curl: -0.8 * (0.8 + h(5) * 0.4),
                    strength: 0.8 + h(6) * 0.2,
                }
            })
            .collect()
    };

    LAST.with(|last| {
        let Ok(mut last) = last.try_borrow_mut() else { return build() };
        match last.as_ref() {
            Some((cached, table)) if *cached == seed => table.clone(),
            _ => {
                let table = build();
                *last = Some((seed, table.clone()));
                table
            }
        }
    })
}

/// Long smooth locks of fur, curling as they grow and tapering to a point, shaded like a height
/// map: buried at the root, round across, rising towards the tip. Where locks cross, the higher
/// one shows. Roots sit on a jittered grid whose cells wrap.
pub(crate) fn smooth_fur(p: Vec3, seed: u32) -> f32 {
    // In cells
    const WIDTH: f32 = 1.55;
    // No lock, root to tip and half a width aside, reaches further than this
    const REACH: f32 = 4.0;
    let q = unit(p) * FUR_CELLS as f32;
    let (ci, cj) = (q.x.floor() as i32, q.y.floor() as i32);
    let locks = locks(seed);

    let mut height = 0.0f32;
    // Locks grow down the page and a little to either side, so their roots lie above and around
    for j in (cj - 5)..=(cj + 1) {
        for i in (ci - 5)..=(ci + 5) {
            let Some(lock) = locks.get((j.rem_euclid(FUR_CELLS) * FUR_CELLS + i.rem_euclid(FUR_CELLS)) as usize) else { continue };
            let d = q - (Vec2::new(i as f32, j as f32) + lock.root);
            if d.length_squared() > REACH * REACH {
                continue;
            }

            // The point in the lock's own frame: growing along V, turned by its angle
            let local = Vec2::new(lock.cos * d.x + lock.sin * d.y, -lock.sin * d.x + lock.cos * d.y);
            // Behind the root or past the tip, whatever the curl
            if local.y < -WIDTH * 0.5 || local.y > lock.length {
                continue;
            }

            let Some((t, offset)) = on_arc(local, lock.length, lock.curl) else { continue };
            let radius = WIDTH * 0.5 * (1.0 - t).powf(0.65);
            if radius <= 1e-6 || offset.abs() >= radius {
                continue;
            }

            let q = offset.abs() / radius;
            let round = (1.0 - q * q).sqrt();
            let h = lock.strength * round * smoothstep(0.0, 1.0, t / 0.06) * smoothstep(0.0, 1.0, (1.0 - t) / 0.025) * (0.28 + 0.72 * t);
            height = height.max(h);
        }
    }

    height.clamp(0.0, 1.0)
}

/// Where a point lies along a lock whose axis grows along +V and turns by `curl` radians over
/// `length`: the fraction of the length it is at and its distance from the axis, or None outside
/// the lock's length.
fn on_arc(local: Vec2, length: f32, curl: f32) -> Option<(f32, f32)> {
    if curl.abs() < 1e-4 {
        let t = local.y / length;
        return (0.0..=1.0).contains(&t).then_some((t, local.x));
    }

    // The axis is an arc of radius `length / curl` around a center beside the root
    let radius = length / curl;
    let t = (local.y / radius).atan2((radius - local.x) / radius) / curl;
    let distance = Vec2::new(local.x - radius, local.y).length();
    (0.0..=1.0).contains(&t).then_some((t, distance - radius.abs()))
}

/// Soft round cloudlets of mixed sizes on a jittered grid, crowded so they overlap; the dark sky
/// shows only where none reaches.
pub(crate) fn mackerel_sky(p: Vec3, seed: u32) -> f32 {
    const CELLS: i32 = 6;
    // A soft maximum, so cloudlets merge where they meet instead of leaving a seam
    const SOFTNESS: f32 = 14.0;
    let u = unit(p);
    let q = u * CELLS as f32;
    let (ci, cj) = (q.x.floor() as i32, q.y.floor() as i32);
    let mut sum = 0.0f32;
    for j in (cj - 2)..=(cj + 2) {
        for i in (ci - 2)..=(ci + 2) {
            let h = |k: i32| noise::cell_hash(i.rem_euclid(CELLS), j.rem_euclid(CELLS), k, seed ^ 0x5F);
            let center = Vec2::new(i as f32 + h(1), j as f32 + h(2));
            let t = (q - center).length() / (0.65 + 0.45 * h(3));
            if t >= 1.2 {
                continue;
            }

            // A blurred dome: flat and bright in the middle, fading softly past its rim
            let dome = smoothstep(1.2, 0.2, t) * (0.85 + 0.15 * (1.0 - t * t).max(0.0).sqrt());
            sum += (SOFTNESS * dome * (0.6 + 0.4 * h(4))).exp() - 1.0;
        }
    }
    let value = (1.0 + sum).ln() / SOFTNESS;

    let mottle = lattice_fbm(u, &[(12, 0.5), (24, 0.3), (48, 0.2)], seed ^ 0x3A);
    (value * (0.88 + 0.12 * mottle)).clamp(0.0, 1.0)
}

/// Domain-warped clouds, crossed by fine strands that follow a slowly turning flow: high frequency
/// noise averaged along the flow direction (a short line integral).
pub(crate) fn billow(p: Vec3, seed: u32) -> f32 {
    let u = unit(p);
    let warp = |salt: u32| lattice_fbm(u, &[(2, 0.5), (4, 0.3), (8, 0.2)], seed ^ salt) - 0.5;
    let q = u + Vec2::new(warp(0x11), warp(0x22)) * 0.2;
    // Mottled at several scales, like a cloud seen up close
    let body = lattice_fbm(q, &[(3, 0.3), (6, 0.25), (12, 0.2), (24, 0.15), (48, 0.1)], seed);

    let angle = std::f32::consts::TAU * lattice_fbm(q, &[(2, 0.6), (4, 0.4)], seed ^ 0x33);
    let flow = Vec2::new(angle.cos(), angle.sin());
    let strands = (-5..=5).map(|k| lattice_noise(q + flow * (k as f32 * 0.0045), (64, 64), seed ^ 0x44)).sum::<f32>() / 11.0;
    let speckle = lattice_noise(q, (32, 32), seed ^ 0x55);

    (0.5 + (body - 0.5) * 1.7 + (strands - 0.5) * 0.45 + (speckle - 0.5) * 0.1).clamp(0.0, 1.0)
}

/// Overlapping reptile scales: rounded shields on staggered rows, each of a size and tone of its
/// own, domed and lit from above, with a dark rim and a shadow cast on the scale below. A scale
/// covers the ones under it. The rows wrap, so the pattern repeats every unit.
pub(crate) fn reptile_scales(p: Vec3, seed: u32) -> f32 {
    const COLUMNS: i32 = 8;
    // Even, so that the staggered rows line up again after one unit
    const ROWS: i32 = 12;
    let u = unit(p);
    let q = Vec2::new(u.x * COLUMNS as f32, u.y * ROWS as f32);
    let (ci, cj) = (q.x.floor() as i32, q.y.floor() as i32);
    let light = Vec3::new(-0.35, -0.5, 0.8).normalize();

    // The scale showing here: the one whose center lies highest among those that reach the point
    let (mut top, mut top_row, mut shadow) = (None, i32::MAX, 0.0f32);
    for j in (cj - 2)..=(cj + 2) {
        for i in (ci - 1)..=(ci + 2) {
            let h = |k: i32| noise::cell_hash(i.rem_euclid(COLUMNS), j.rem_euclid(ROWS), k, seed ^ 0x2B);
            let stagger = if j.rem_euclid(2) == 1 { 0.5 } else { 0.0 };
            let center = Vec2::new(i as f32 + 0.5 + stagger + (h(1) - 0.5) * 0.3, j as f32 + 0.5 + (h(2) - 0.5) * 0.4);
            let (half_width, half_height) = (0.62 * (0.92 + 0.16 * h(3)), 1.3 * (0.92 + 0.16 * h(4)));
            let d = q - center;
            // Narrower towards the lower tip
            let narrowing = 1.0 - 0.3 * (d.y / half_height).clamp(0.0, 1.0);
            let s = Vec2::new(d.x / (half_width * narrowing), d.y / half_height);
            let rim = s.length();
            if rim < 1.0 {
                if j < top_row || (j == top_row && h(5) > 0.5) {
                    top_row = j;
                    top = Some((s, rim, h(6), h(7)));
                }
            } else if rim < 1.35 && d.y > 0.0 {
                // Under this scale's lower edge, where it throws a shadow
                shadow = shadow.max(smoothstep(1.35, 1.0, rim));
            }
        }
    }

    let Some((s, rim, tone, ridge)) = top else { return 0.1 };
    let height = (1.0 - rim * rim).max(0.0).sqrt();
    // A keel down the middle, stronger on some scales
    let keel = (-(s.x * 3.2) * (s.x * 3.2)).exp() * (0.1 + 0.25 * ridge);
    let normal = Vec3::new(s.x * 0.9, s.y * 0.9, height + keel).normalize();
    let lit = normal.dot(light).max(0.0);
    let grain = lattice_noise(u, (96, 96), seed ^ 0x3C);
    let covered = if top_row == i32::MAX { 0.0 } else { shadow };
    let body = (0.25 + 0.75 * tone) * (0.35 + 0.65 * lit) * (1.0 - 0.5 * covered) * (0.86 + 0.14 * grain);
    (body * (0.45 + 0.55 * smoothstep(1.0, 0.8, rim))).clamp(0.0, 1.0)
}

/// Young bark: tall plates of different tone parted by wandering furrows (0 = furrow), with fibres
/// running along the trunk. The plates are the cells of a stretched, jittered grid that wraps.
pub(crate) fn bark(p: Vec3, seed: u32) -> f32 {
    const COLUMNS: i32 = 7;
    const ROWS: i32 = 3;
    // How little a step along the trunk counts against one across it
    const STRETCH: f32 = 0.38;
    let u = unit(p);

    // The furrows wander
    let wobble = Vec2::new(lattice_noise(u, (5, 5), seed ^ 0x31), lattice_noise(u, (5, 5), seed ^ 0x32)) - Vec2::splat(0.5);
    let w = u + wobble * Vec2::new(0.06, 0.14);

    let q = Vec2::new(w.x * COLUMNS as f32, w.y * ROWS as f32);
    let (ci, cj) = (q.x.floor() as i32, q.y.floor() as i32);
    let (mut nearest, mut next, mut plate) = (f32::MAX, f32::MAX, 0.0f32);
    for j in (cj - 2)..=(cj + 2) {
        for i in (ci - 1)..=(ci + 1) {
            let h = |k: i32| noise::cell_hash(i.rem_euclid(COLUMNS), j.rem_euclid(ROWS), k, seed ^ 0x27);
            let center = Vec2::new(i as f32 + 0.5 + (h(1) - 0.5) * 0.85, j as f32 + 0.5 + (h(2) - 0.5) * 0.85);
            let d = q - center;
            let distance = Vec2::new(d.x / COLUMNS as f32, d.y / ROWS as f32 * STRETCH).length();
            if distance < nearest {
                (next, nearest, plate) = (nearest, distance, h(3));
            } else if distance < next {
                next = distance;
            }
        }
    }

    // Distance to the nearest furrow, in plate widths
    let to_furrow = (next - nearest) * COLUMNS as f32;
    let ridge = smoothstep(0.0, 0.4, to_furrow).sqrt();
    let fibre = lattice_noise(w, (96, 6), seed ^ 0x52);
    (ridge * (0.5 + 0.3 * plate + 0.2 * fibre)).clamp(0.0, 1.0)
}

/// A labyrinth of rounded walls like brain coral, where the walls break into dots and stripes in
/// places: a band-limited wave field (cosines of wave vectors on one ring of the lattice) cut at a
/// level that drifts slowly across the surface.
pub(crate) fn coral_maze(p: Vec3, seed: u32) -> f32 {
    use std::f32::consts::{PI, TAU};

    const WAVES: usize = 9;
    // Cycles per unit
    const RING: f32 = 9.0;
    let u = unit(p);

    let (mut sum, mut energy) = (0.0f32, 0.0f32);
    for k in 0..WAVES {
        let h = |n: i32| noise::cell_hash(k as i32, n, 0, seed ^ 0x5E);
        // Directions spread over a half turn (a cosine has no handedness); whole cycles per unit
        // across and down, so the field repeats
        let angle = (k as f32 + 0.8 * h(0)) * PI / WAVES as f32;
        let (across, down) = ((angle.cos() * RING).round(), (angle.sin() * RING).round());
        let amplitude = 0.7 + 0.6 * h(2);
        sum += amplitude * (TAU * (across * u.x + down * u.y) + TAU * h(1)).cos();
        energy += amplitude * amplitude * 0.5;
    }
    // Unit variance, whatever the amplitudes
    let field = sum / energy.sqrt();

    // The level drifts: walls are thick in places and thin in others
    let level = (lattice_noise(u, (2, 2), seed ^ 0x6C) - 0.5) * 1.5 - 0.15;
    let wall = smoothstep(-0.1, 0.45, field - level);
    // Rounded across, like a tube
    wall.sqrt()
}

/// Camouflage: four tones of interlocking blotches with ragged edges. The darkest tone is the
/// ground and each lighter one is laid over it in blotches of its own size, a little stretched
/// across the surface. Every tone is flat, so a gradient map turns it into woodland, desert or
/// any other colours.
pub(crate) fn camouflage(p: Vec3, seed: u32) -> f32 {
    let u = unit(p);
    // Fractal noise on a lattice stretched across the surface
    let blotches = |cells: (i32, i32), salt: u32| {
        0.55 * lattice_noise(u, cells, seed ^ salt)
            + 0.3 * lattice_noise(u, (cells.0 * 2, cells.1 * 2), seed ^ salt ^ 0x9E)
            + 0.15 * lattice_noise(u, (cells.0 * 4, cells.1 * 4), seed ^ salt ^ 0x3F)
    };
    // The same ragged edge for every tone
    let ragged = (lattice_fbm(u, &[(24, 0.6), (48, 0.4)], seed ^ 0x71) - 0.5) * 0.14;

    let layer = |field: f32, threshold: f32| smoothstep(threshold - 0.012, threshold + 0.012, field + ragged);
    let mut value = 0.05;
    for (tone, field, threshold) in [(0.35, blotches((3, 2), 0x11), 0.46), (0.65, blotches((4, 3), 0x22), 0.54), (0.95, blotches((6, 4), 0x33), 0.6)] {
        value += (tone - value) * layer(field, threshold);
    }

    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Pattern = fn(Vec3, u32) -> f32;
    const PATTERNS: [(&str, Pattern); 13] = [
        ("fur", smooth_fur),
        ("sky", mackerel_sky),
        ("billow", billow),
        ("scales", reptile_scales),
        ("bark", bark),
        ("coral", coral_maze),
        ("camouflage", camouflage),
        ("knit", crate::fabric::knit),
        ("basket weave", crate::fabric::basket_weave),
        ("goose-eye", crate::fabric::goose_eye),
        ("slub silk", crate::fabric::slub_silk),
        ("strip flooring", crate::timber::strip_flooring),
        ("knotty wood", crate::timber::knotty_wood),
    ];

    /// A spread of points that are away from any cell edge
    fn points() -> impl Iterator<Item = Vec3> {
        (0..400).map(|i| {
            let t = i as f32;
            Vec3::new((t * 0.618_034 + 0.013).fract(), (t * 0.414_214 + 0.071).fract(), 0.0)
        })
    }

    #[test]
    fn every_pattern_repeats_every_unit() {
        for (name, pattern) in PATTERNS {
            for seed in [1, 7] {
                for p in points() {
                    let value = pattern(p, seed);
                    for shift in [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(-3.0, 5.0, 0.0)] {
                        let shifted = pattern(p + shift, seed);
                        assert!((value - shifted).abs() < 1e-3, "{name} at {p:?} (seed {seed}), shifted by {shift:?}: {value} against {shifted}");
                    }
                }
            }
        }
    }

    #[test]
    fn every_pattern_stays_in_range_and_varies() {
        for (name, pattern) in PATTERNS {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for i in 0..4096 {
                let p = Vec3::new((i % 64) as f32 / 64.0, (i / 64) as f32 / 64.0, 0.0);
                let v = pattern(p, 1);
                assert!((0.0..=1.0).contains(&v), "{name} at {p:?}: {v}");
                (lo, hi) = (lo.min(v), hi.max(v));
            }
            assert!(hi - lo > 0.4, "{name} is flat: {lo}..{hi}");
        }
    }

    #[test]
    fn the_seed_changes_every_pattern() {
        for (name, pattern) in PATTERNS {
            let differs = points().any(|p| (pattern(p, 1) - pattern(p, 2)).abs() > 0.02);
            assert!(differs, "{name} ignores its seed");
        }
    }

    #[test]
    fn every_pattern_takes_extreme_coordinates() {
        let extremes = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, 1e30, -1e30, 16_777_216.5, -0.0];
        for (name, pattern) in PATTERNS {
            for x in extremes {
                for y in extremes {
                    let v = pattern(Vec3::new(x, y, x), 3);
                    assert!((0.0..=1.0).contains(&v), "{name} at ({x}, {y}): {v}");
                }
            }
        }
    }

    #[test]
    fn scales_leave_no_gap_and_bark_makes_furrows() {
        let grid = |size: usize| (0..size * size).map(move |i| Vec3::new((i % size) as f32 / size as f32, (i / size) as f32 / size as f32, 0.0));
        let darkest = grid(64).map(|p| reptile_scales(p, 1)).fold(f32::MAX, f32::min);
        assert!(darkest > 0.02, "a gap shows between scales: {darkest}");

        let furrows = grid(64).filter(|p| bark(*p, 1) < 0.1).count();
        assert!((100..1800).contains(&furrows), "furrows cover {furrows} of 4096 samples");
    }

    #[test]
    fn the_coral_maze_has_walls_and_gaps() {
        let walls = (0..4096).filter(|i| coral_maze(Vec3::new((i % 64) as f32 / 64.0, (i / 64) as f32 / 64.0, 0.0), 1) > 0.5).count();
        assert!((800..3300).contains(&walls), "walls cover {walls} of 4096 samples");
    }

    #[test]
    fn camouflage_lays_down_four_tones() {
        let mut counts = [0usize; 4];
        for i in 0..16384 {
            let v = camouflage(Vec3::new((i % 128) as f32 / 128.0, (i / 128) as f32 / 128.0, 0.0), 1);
            // The nearest of the four tones
            let tone = [0.05, 0.35, 0.65, 0.95].iter().enumerate().min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs())).map_or(0, |(k, _)| k);
            if let Some(count) = counts.get_mut(tone) {
                *count += 1;
            }
        }
        for (tone, count) in counts.iter().enumerate() {
            assert!(*count > 16384 / 25, "tone {tone} covers only {count} of 16384 samples: {counts:?}");
        }
    }
}
