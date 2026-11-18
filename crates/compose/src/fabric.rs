//! Woven and knitted fabrics: knit, basket weave, goose-eye twill and slub silk.
//!
//! Every pattern repeats every unit in U and V, so it tiles without seams. Values are shaded like
//! a height map: threads round across, their ends tucked under the threads that cross them.

use texturecraft_geom::glam::{Vec2, Vec3};
use texturecraft_texel::noise;

use crate::organic::{smoothstep, unit};

/// A thread's round cross-section: 0 at its edges, 1 along its crest, for `across` in 0..1
fn round(across: f32) -> f32 {
    let c = 2.0 * across - 1.0;
    (1.0 - c * c).max(0.0).sqrt()
}

/// How a thread swells between the crossings at its ends, for `along` in 0..1
fn bulge(along: f32) -> f32 {
    0.55 + 0.45 * (std::f32::consts::PI * along).sin()
}

/// Distance from `p` to the segment `a`..`b`, and where along it the nearest point lies (0..1)
fn segment(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-9)).clamp(0.0, 1.0);
    ((p - (a + ab * t)).length(), t)
}

/// Stockinette: columns of V-shaped stitches, each of two twisted legs of yarn that lean towards
/// each other and meet below. The stitches of one row sit over those of the next.
pub(crate) fn knit(p: Vec3, seed: u32) -> f32 {
    const COLUMNS: i32 = 10;
    const ROWS: i32 = 14;
    // Yarn radius, in stitch widths
    const RADIUS: f32 = 0.21;
    let q = unit(p) * Vec2::new(COLUMNS as f32, ROWS as f32);
    let (ci, cj) = (q.x.floor() as i32, q.y.floor() as i32);

    let mut height = 0.0f32;
    let mut tone = 0.5f32;
    for j in (cj - 1)..=(cj + 1) {
        for i in (ci - 1)..=(ci + 1) {
            let cell = Vec2::new(i as f32, j as f32);
            let yarn = 0.85 + 0.15 * noise::cell_hash(i.rem_euclid(COLUMNS), j.rem_euclid(ROWS), 0, seed ^ 0x4B);
            // The two legs: from the top corners to the bottom middle
            for (side, top) in [(-1.0f32, Vec2::new(0.1, -0.05)), (1.0, Vec2::new(0.9, -0.05))] {
                let (distance, along) = segment(q, cell + top, cell + Vec2::new(0.5, 1.05));
                if distance >= RADIUS {
                    continue;
                }

                let across = distance / RADIUS;
                // The yarn is twisted: slanted ridges along the leg
                let twist = 0.5 + 0.5 * (std::f32::consts::TAU * (along * 2.5 + side * across * 0.3)).cos();
                let leg = (1.0 - across * across).sqrt() * (0.82 + 0.18 * twist) * yarn;
                if leg > height {
                    height = leg;
                    tone = yarn;
                }
            }
        }
    }

    (0.12 + 0.88 * height * (0.8 + 0.2 * tone)).clamp(0.0, 1.0)
}

/// A basket weave: threads in pairs, each pair lying over and then under the next pair, so the
/// pairs make a checkerboard of squares whose threads alternately run across and down.
pub(crate) fn basket_weave(p: Vec3, seed: u32) -> f32 {
    // Even, so that the checkerboard lines up again after one unit
    const BLOCKS: i32 = 4;
    const PAIR: f32 = 2.0;
    let q = unit(p) * BLOCKS as f32;
    let (bx, by) = (q.x.floor() as i32, q.y.floor() as i32);
    let (fx, fy) = (q.x - bx as f32, q.y - by as f32);

    let across_the_page = (bx + by).rem_euclid(2) == 0;
    let (along, position) = if across_the_page { (fx, fy) } else { (fy, fx) };
    let thread = (position * PAIR).floor() as i32;
    let across = position * PAIR - thread as f32;

    let tone = noise::cell_hash((bx * 2 + thread).rem_euclid(BLOCKS * 2), by.rem_euclid(BLOCKS), across_the_page as i32, seed ^ 0x2D);
    // Fine fibres along the thread
    let fibre = 0.9 + 0.1 * (std::f32::consts::TAU * across * 6.0).cos();
    let crest = round(across) * bulge(along) * fibre;
    ((0.1 + 0.9 * crest) * (0.75 + 0.25 * tone) * if across_the_page { 1.0 } else { 0.9 }).clamp(0.0, 1.0)
}

/// Goose-eye twill: a 2/2 twill whose diagonal turns back every few threads, across and down, so
/// the ribs close into small diamonds with an eye at the middle. The warp, which runs down, is
/// brighter than the weft.
pub(crate) fn goose_eye(p: Vec3, seed: u32) -> f32 {
    const THREADS: i32 = 24;
    // Threads from one turn of the diagonal to the next; two of them go to the repeat
    const HALF: i32 = 6;
    let q = unit(p) * THREADS as f32;
    let (i, j) = (q.x.floor() as i32, q.y.floor() as i32);
    let (fx, fy) = (q.x - i as f32, q.y - j as f32);

    // Threads away from the nearest turn
    let from_turn = |k: i32| (k.rem_euclid(2 * HALF) - HALF).abs();
    let warp_over = |i: i32, j: i32| (from_turn(i) + from_turn(j)).rem_euclid(4) < 2;

    let tone = noise::cell_hash(i.rem_euclid(THREADS), j.rem_euclid(THREADS), 0, seed ^ 0x3E);
    // A float runs on over the next crossing when its thread stays on top there; it tucks under
    // only at its ends
    let tuck = |along: f32, before: bool, after: bool| {
        let start = if before { 1.0 } else { 0.55 + 0.45 * smoothstep(0.0, 0.3, along) };
        let end = if after { 1.0 } else { 0.55 + 0.45 * smoothstep(1.0, 0.7, along) };
        start * end
    };
    let (crest, brightness) = if warp_over(i, j) {
        (round(fx) * tuck(fy, warp_over(i, j - 1), warp_over(i, j + 1)), 1.0)
    } else {
        (round(fy) * tuck(fx, !warp_over(i - 1, j), !warp_over(i + 1, j)), 0.4)
    };
    ((0.1 + 0.9 * crest) * brightness * (0.9 + 0.1 * tone)).clamp(0.0, 1.0)
}

/// A fine plain weave of raw silk: here and there a thread swells into a slub, a thick, bright
/// stretch of yarn that floats over the threads it crosses, and each thread has a sheen of its own.
pub(crate) fn slub_silk(p: Vec3, seed: u32) -> f32 {
    const THREADS: i32 = 36;
    // Slubs along a thread, per unit
    const SLUBS: i32 = 7;
    let u = unit(p);
    let q = u * THREADS as f32;
    let (i, j) = (q.x.floor() as i32, q.y.floor() as i32);
    let (fx, fy) = (q.x - i as f32, q.y - j as f32);

    // Slub strength along a thread: smooth noise, mostly low
    let slub_of = |thread: i32, along_unit: f32, warp: bool| {
        let thread = thread.rem_euclid(THREADS);
        let s = along_unit * SLUBS as f32;
        let k = s.floor() as i32;
        let f = s - k as f32;
        let at = |step: i32| noise::cell_hash(thread, (k + step).rem_euclid(SLUBS), warp as i32, seed ^ 0x5A);
        smoothstep(0.74, 0.93, at(0) + (at(1) - at(0)) * f * f * (3.0 - 2.0 * f))
    };
    // Warp threads run down the page and weft threads across it
    let (warp_slub, weft_slub) = (slub_of(i, u.y, true), slub_of(j, u.x, false));

    // A slubbed thread floats over the threads it crosses; elsewhere they interlace one over one
    let warp_over = if warp_slub > 0.5 {
        true
    } else if weft_slub > 0.5 {
        false
    } else {
        (i + j).rem_euclid(2) == 0
    };
    let (thread, slub, across, along) = if warp_over { (i, warp_slub, fx, fy) } else { (j, weft_slub, fy, fx) };

    let sheen = 0.45 + 0.25 * noise::cell_hash(thread.rem_euclid(THREADS), 99, warp_over as i32, seed ^ 0x6B);
    let tuck = if slub > 0.5 { 1.0 } else { bulge(along) };
    let crest = round(across) * tuck * (0.75 + 0.45 * slub);
    ((0.25 + 0.75 * crest) * (sheen + 0.28 * slub)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stitch_is_a_v_and_a_basket_weave_has_pairs() {
        // Between two stitches of a row the yarn leaves a hollow; in the middle of a stitch it does not
        let grid = |size: usize| (0..size * size).map(move |i| Vec3::new((i % size) as f32 / size as f32, (i / size) as f32 / size as f32, 0.0));
        let hollows = grid(128).filter(|p| knit(*p, 1) < 0.15).count();
        assert!((300..8000).contains(&hollows), "hollows cover {hollows} of 16384 samples");

        // Pairs of threads: across a block whose threads run along the page there are two crests,
        // with a hollow between them and one at each side
        let across = |y: f32| basket_weave(Vec3::new(0.125, y, 0.0), 1);
        for crest in [0.0625, 0.1875] {
            assert!(across(crest) > 0.5, "no crest at {crest}: {}", across(crest));
        }
        for hollow in [0.001, 0.125, 0.249] {
            assert!(across(hollow) < 0.3, "no hollow at {hollow}: {}", across(hollow));
        }
    }

    #[test]
    fn goose_eye_turns_its_diagonal() {
        // A plain twill would repeat every four threads along a row; the turns make the weave differ from block to block
        let row = |y: f32| (0..48).map(|i| goose_eye(Vec3::new((i as f32 + 0.5) / 48.0 / 2.0, y, 0.0), 1) > 0.5).collect::<Vec<_>>();
        let (first, second) = (row(0.5 / 24.0), row(5.5 / 24.0));
        assert_ne!(first, second, "every row weaves the same");
    }
}
