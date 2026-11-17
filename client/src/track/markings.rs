//! What is painted on the road, and the light its walls throw onto it.
//!
//! The road was a dark band with a dashed line down its middle: speed showed, but a turn did not
//! until the walls began to curve. Kerbs now line the inside of every turn, so a bend reads from
//! far off and its apex from close up; a solid line runs inside each edge the whole lap round; the
//! starting grid is drawn on the road, one box per slot; and every neon stripe of the walls lights
//! the road at its foot, a pool of blue fading out across the tarmac, as a lamp would. The paint is
//! lit like the road is; the pools of light are added to whatever they fall on, kerbs included.

use std::f32::consts::PI;

use bevy::color::LinearRgba;
use bevy::prelude::*;
use star_racer_sim::track::Track;

use super::{
    MARKING, MARKING_HEIGHT, NEON_BLUE, SLAB, WALL_DARK, WALL_STRIPE, for_each_road_segment,
    on_road,
};
use crate::geometry::Geometry;

/// Kerbs line both edges of the road wherever its centerline turns more tightly than this, in
/// radians per meter: a radius of 110 m. Every turn of the skyway is tighter than that at its
/// apex, and the transitions bring the kerbs in partway along.
const KERB_CURVATURE: f32 = 1.0 / 110.0;
/// How far a kerb reaches in from the foot of the wall, and how long each of its blocks is, in
/// meters.
const KERB_WIDTH: f32 = 1.6;
const KERB_BLOCK: f32 = 2.5;
/// The two colors the blocks of a kerb alternate between: the magenta of a race on, and white.
const KERB_RED: Color = Color::srgb(0.74, 0.09, 0.3);
const KERB_WHITE: Color = Color::srgb(0.62, 0.62, 0.6);

/// The line inside each edge: how far in from the foot of the wall, how wide, and how bright, in
/// meters. Inside the kerbs, and duller than the markings across the road.
const EDGE_INSET: f32 = KERB_WIDTH + 0.6;
const EDGE_WIDTH: f32 = 0.22;
const EDGE: Color = Color::srgb(0.48, 0.48, 0.45);

/// The boxes of the starting grid: how many, how wide across the road, how far back their arms
/// reach, how thick their lines are, and how far ahead of a car's middle the front line stands, in
/// meters.
const GRID_BOXES: usize = 8;
const BOX_WIDTH: f32 = 4.4;
const BOX_ARMS: f32 = 2.0;
const BOX_LINE: f32 = 0.22;
const BOX_AHEAD: f32 = 2.4;

/// The light a neon stripe throws on the road at its foot: how far across the road it reaches, in
/// meters, and how much of the stripe's color it adds at the wall. It fades out across the road and
/// toward both ends of the stripe.
const POOL_REACH: f32 = 4.2;
const POOL_ALPHA: f32 = 0.45;
/// The pools lie over everything painted on the road, so the kerbs catch the light as well.
const POOL_HEIGHT: f32 = 4.0 * MARKING_HEIGHT;

/// Kerbs of alternating blocks along the inside edge of every turn, from the foot of the wall in.
///
/// The inside only: the skyway is turns almost all the way round, and kerbs on both edges were a
/// lap of magenta and white. On the inside they say which way the road goes and where its apex is.
pub(super) fn kerbs(lit: &mut Geometry, track: &Track) {
    let blocks = (track.length() / KERB_BLOCK) as usize;
    for block in 0..blocks {
        let start = block as f32 * KERB_BLOCK;
        let curvature = track.point_at(start + KERB_BLOCK / 2.0).curvature;
        if curvature.abs() < KERB_CURVATURE
            || track.in_gap(start)
            || track.in_gap(start + KERB_BLOCK)
        {
            continue;
        }
        // A turn to the left has its inside on the left, where lateral offsets are positive.
        let side = curvature.signum();
        let color = if block % 2 == 0 { KERB_RED } else { KERB_WHITE };
        // Two pieces a block, so a kerb follows the curve it lines.
        for piece in 0..2 {
            let a = track.point_at(start + piece as f32 * KERB_BLOCK / 2.0);
            let b = track.point_at(start + (piece + 1) as f32 * KERB_BLOCK / 2.0);
            let wall = |half_width: f32| side * (half_width - 0.02);
            let inner = |half_width: f32| side * (half_width - KERB_WIDTH);
            lit.quad_facing_up(
                [
                    on_road(track, a, wall(a.half_width), 1.5 * MARKING_HEIGHT),
                    on_road(track, a, inner(a.half_width), 1.5 * MARKING_HEIGHT),
                    on_road(track, b, inner(b.half_width), 1.5 * MARKING_HEIGHT),
                    on_road(track, b, wall(b.half_width), 1.5 * MARKING_HEIGHT),
                ],
                color,
            );
        }
    }
}

/// A solid line inside each edge, all the way round.
pub(super) fn edge_lines(lit: &mut Geometry, track: &Track) {
    for side in [1.0_f32, -1.0] {
        let outer = |half_width: f32| side * (half_width - EDGE_INSET);
        let inner = |half_width: f32| side * (half_width - EDGE_INSET - EDGE_WIDTH);
        for_each_road_segment(track, |a, b, _| {
            lit.quad_facing_up(
                [
                    on_road(track, a, outer(a.half_width), MARKING_HEIGHT),
                    on_road(track, a, inner(a.half_width), MARKING_HEIGHT),
                    on_road(track, b, inner(b.half_width), MARKING_HEIGHT),
                    on_road(track, b, outer(b.half_width), MARKING_HEIGHT),
                ],
                EDGE,
            );
        });
    }
}

/// The starting grid, one box for each of the first slots: a line across the front of where a car
/// stands, and two arms reaching back along its sides.
pub(super) fn grid_boxes(lit: &mut Geometry, track: &Track) {
    for slot in 0..GRID_BOXES {
        let (position, _) = track.grid_slot(slot);
        let here = track.project(position);
        let front = here.distance + BOX_AHEAD;
        let (right, left) = (
            here.lateral - BOX_WIDTH / 2.0,
            here.lateral + BOX_WIDTH / 2.0,
        );
        // Across the front, then each arm, as (back, front, right, left).
        let lines = [
            (front - BOX_LINE, front, right, left),
            (front - BOX_ARMS, front, right, right + BOX_LINE),
            (front - BOX_ARMS, front, left - BOX_LINE, left),
        ];
        for (back, ahead, from, to) in lines {
            let (a, b) = (track.point_at(back), track.point_at(ahead));
            lit.quad_facing_up(
                [
                    on_road(track, a, from, 1.5 * MARKING_HEIGHT),
                    on_road(track, a, to, 1.5 * MARKING_HEIGHT),
                    on_road(track, b, to, 1.5 * MARKING_HEIGHT),
                    on_road(track, b, from, 1.5 * MARKING_HEIGHT),
                ],
                MARKING,
            );
        }
    }
}

/// The speed pads (see `sim::track::Pad`): on each, three chevrons pointing the way, drawn in the
/// neon, brighter than white so they bloom, over a pool of the same light added to the road.
pub(super) fn pads(neon: &mut Geometry, glow: &mut Geometry, track: &Track) {
    for pad in track.pads() {
        let half = pad.width / 2.0;
        let (left, right) = (pad.lateral + half, pad.lateral - half);
        // The pool under the whole pad, fading toward its ends.
        let steps = 6;
        for step in 0..steps {
            let a = pad.at + pad.length * step as f32 / steps as f32;
            let b = pad.at + pad.length * (step + 1) as f32 / steps as f32;
            let fade = |at: f32| {
                let share = ((at - pad.at) / pad.length * PI).sin();
                PAD_GLOW.with_alpha(PAD_POOL_ALPHA * share)
            };
            let (pa, pb) = (track.point_at(a), track.point_at(b));
            glow.quad_shaded(
                [
                    on_road(track, pa, right, POOL_HEIGHT),
                    on_road(track, pa, left, POOL_HEIGHT),
                    on_road(track, pb, left, POOL_HEIGHT),
                    on_road(track, pb, right, POOL_HEIGHT),
                ],
                Vec3::Y,
                [fade(a), fade(a), fade(b), fade(b)],
            );
        }
        // Chevrons: each arm runs from the pad's edge, behind, to its middle, ahead.
        for chevron in 0..PAD_CHEVRONS {
            let back = pad.at + pad.length * (0.12 + 0.3 * chevron as f32);
            let tip = back + PAD_CHEVRON_DEPTH;
            for edge in [left, right] {
                let (pb, pt) = (track.point_at(back), track.point_at(tip));
                let (pb2, pt2) = (
                    track.point_at(back + PAD_STROKE),
                    track.point_at(tip + PAD_STROKE),
                );
                neon.quad_facing_up(
                    [
                        on_road(track, pb, edge, 2.0 * MARKING_HEIGHT),
                        on_road(track, pt, pad.lateral, 2.0 * MARKING_HEIGHT),
                        on_road(track, pt2, pad.lateral, 2.0 * MARKING_HEIGHT),
                        on_road(track, pb2, edge, 2.0 * MARKING_HEIGHT),
                    ],
                    Color::LinearRgba(PAD_NEON),
                );
            }
        }
    }
}

/// How many chevrons a pad has, how far each one's tip is ahead of its arms' ends, and how thick
/// its strokes are, in meters.
const PAD_CHEVRONS: usize = 3;
const PAD_CHEVRON_DEPTH: f32 = 2.2;
const PAD_STROKE: f32 = 0.7;
/// The pads' neon, an amber brighter than white, and the light it throws on the road.
const PAD_NEON: LinearRgba = LinearRgba::new(3.4, 1.6, 0.2, 1.0);
const PAD_GLOW: Color = Color::srgb(1.0, 0.55, 0.1);
const PAD_POOL_ALPHA: f32 = 0.55;

/// Where the road stops at a gap and takes up again (see `sim::track::Gap`): the cut face of the
/// slab, a line of hot magenta neon across the road at its edge, and the light of it thrown back
/// along the road, so a gap reads from far off as a bright line with nothing beyond it.
pub(super) fn gap_lips(lit: &mut Geometry, neon: &mut Geometry, glow: &mut Geometry, track: &Track) {
    for gap in track.gaps() {
        // The lip, the road behind it; then the far edge, the road ahead of it.
        for (edge, back) in [(gap.at, -1.0_f32), (gap.at + gap.length, 1.0)] {
            let point = track.point_at(edge);
            let half = point.half_width;
            let across = |lateral: f32, above: f32| on_road(track, point, lateral, above);
            // The cut face, from the surface down to the underside of the slab, facing the gap.
            let facing = world_direction(point.direction * -back);
            let strips = 8;
            for strip in 0..strips {
                let right = -half + 2.0 * half * strip as f32 / strips as f32;
                let left = right + 2.0 * half / strips as f32;
                lit.quad(
                    [
                        across(right, -SLAB),
                        across(left, -SLAB),
                        across(left, 0.0),
                        across(right, 0.0),
                    ],
                    facing,
                    WALL_DARK,
                );
                // A line of the same neon along the foot of the cut face, so the slab's end shows
                // as a lit edge from below and from the far side, not a dark slot.
                let out = facing * 0.03;
                neon.quad(
                    [
                        across(right, -SLAB) + out,
                        across(left, -SLAB) + out,
                        across(left, -SLAB + LIP_FOOT) + out,
                        across(right, -SLAB + LIP_FOOT) + out,
                    ],
                    facing,
                    Color::LinearRgba(LIP_NEON),
                );
            }
            // Before the lip, chevrons across the whole road pointing over it: jump here.
            if back < 0.0 {
                lip_chevrons(neon, track, edge, half);
            }
            // The line across the road at its edge.
            let inside = track.point_at(edge + back * LIP_LINE);
            for strip in 0..strips {
                let right = -half + 2.0 * half * strip as f32 / strips as f32;
                let left = right + 2.0 * half / strips as f32;
                neon.quad_facing_up(
                    [
                        on_road(track, point, right, 2.0 * MARKING_HEIGHT),
                        on_road(track, point, left, 2.0 * MARKING_HEIGHT),
                        on_road(track, inside, left, 2.0 * MARKING_HEIGHT),
                        on_road(track, inside, right, 2.0 * MARKING_HEIGHT),
                    ],
                    Color::LinearRgba(LIP_NEON),
                );
            }
            // Its light on the road, fading away from the edge.
            let far = track.point_at(edge + back * LIP_GLOW_REACH);
            let bright = LIP_GLOW.with_alpha(LIP_GLOW_ALPHA);
            let dark = LIP_GLOW.with_alpha(0.0);
            for strip in 0..strips {
                let right = -half + 2.0 * half * strip as f32 / strips as f32;
                let left = right + 2.0 * half / strips as f32;
                glow.quad_shaded(
                    [
                        on_road(track, point, right, POOL_HEIGHT),
                        on_road(track, point, left, POOL_HEIGHT),
                        on_road(track, far, left, POOL_HEIGHT),
                        on_road(track, far, right, POOL_HEIGHT),
                    ],
                    Vec3::Y,
                    [bright, bright, dark, dark],
                );
            }
        }
    }
}

/// Chevrons over the last meters before a gap's lip, each an arrow across the whole road with its
/// tip ahead in the middle, in pieces short enough to follow the road's cross-section.
fn lip_chevrons(neon: &mut Geometry, track: &Track, lip: f32, half: f32) {
    let reach = half - 1.5;
    let pieces = 8;
    for chevron in 0..LIP_CHEVRONS {
        let tip = lip - LIP_CHEVRON_FIRST - LIP_CHEVRON_SPACING * chevron as f32;
        // The farther from the lip, the dimmer: they draw the eye on toward it.
        let fade = 1.0 - 0.28 * chevron as f32;
        let color = Color::LinearRgba(LIP_NEON * fade);
        for side in [1.0_f32, -1.0] {
            for piece in 0..pieces {
                let share = |step: usize| step as f32 / pieces as f32;
                let at = |step: usize, stroke: f32| {
                    let along = tip - LIP_CHEVRON_DEPTH * share(step) + stroke;
                    on_road(
                        track,
                        track.point_at(along),
                        side * reach * share(step),
                        2.0 * MARKING_HEIGHT,
                    )
                };
                let corners = [
                    at(piece, -LIP_CHEVRON_STROKE),
                    at(piece + 1, -LIP_CHEVRON_STROKE),
                    at(piece + 1, 0.0),
                    at(piece, 0.0),
                ];
                neon.quad_facing_up(corners, color);
            }
        }
    }
}

/// How many chevrons stand before a lip, how far before it the first one's tip is and how far apart
/// they are, how far their arms sweep back from the tip to the road's edges, and how thick their
/// strokes are, in meters.
const LIP_CHEVRONS: usize = 3;
const LIP_CHEVRON_FIRST: f32 = 5.0;
const LIP_CHEVRON_SPACING: f32 = 11.0;
const LIP_CHEVRON_DEPTH: f32 = 16.0;
const LIP_CHEVRON_STROKE: f32 = 1.1;
/// How tall the neon line along the foot of a lip's cut face is, in meters.
const LIP_FOOT: f32 = 0.3;
/// How deep the neon line along a gap's edge is, and how far along the road its light reaches, in
/// meters, and its colors: the neon brighter than white so it blooms.
const LIP_LINE: f32 = 0.8;
const LIP_GLOW_REACH: f32 = 9.0;
const LIP_NEON: LinearRgba = LinearRgba::new(3.2, 0.25, 1.4, 1.0);
const LIP_GLOW: Color = Color::srgb(1.0, 0.15, 0.55);
const LIP_GLOW_ALPHA: f32 = 0.5;

/// The checkpoints (see `sim::track::Track::gates`): a line of green neon across the road at each,
/// and its light on the road either side, so a gate shows from far off without anything standing
/// in the way.
pub(super) fn checkpoints(neon: &mut Geometry, glow: &mut Geometry, track: &Track) {
    for gate in track.gates().iter().skip(1) {
        let a = track.point_at(gate - CHECKPOINT_LINE / 2.0);
        let b = track.point_at(gate + CHECKPOINT_LINE / 2.0);
        let strips = 8;
        for strip in 0..strips {
            let share = |s: f32, point: &star_racer_sim::track::TrackPoint| {
                -point.half_width + 2.0 * point.half_width * s / strips as f32
            };
            let (r, l) = (strip as f32, strip as f32 + 1.0);
            neon.quad_facing_up(
                [
                    on_road(track, a, share(r, &a), 2.0 * MARKING_HEIGHT),
                    on_road(track, a, share(l, &a), 2.0 * MARKING_HEIGHT),
                    on_road(track, b, share(l, &b), 2.0 * MARKING_HEIGHT),
                    on_road(track, b, share(r, &b), 2.0 * MARKING_HEIGHT),
                ],
                Color::LinearRgba(CHECKPOINT_NEON),
            );
            for reach in [-CHECKPOINT_GLOW_REACH, CHECKPOINT_GLOW_REACH] {
                let near = track.point_at(*gate);
                let far = track.point_at(gate + reach);
                let bright = CHECKPOINT_GLOW.with_alpha(CHECKPOINT_GLOW_ALPHA);
                let dark = CHECKPOINT_GLOW.with_alpha(0.0);
                glow.quad_shaded(
                    [
                        on_road(track, near, share(r, &near), POOL_HEIGHT),
                        on_road(track, near, share(l, &near), POOL_HEIGHT),
                        on_road(track, far, share(l, &far), POOL_HEIGHT),
                        on_road(track, far, share(r, &far), POOL_HEIGHT),
                    ],
                    Vec3::Y,
                    [bright, bright, dark, dark],
                );
            }
        }
    }
}

/// How deep a checkpoint's line is and how far its light reaches either side, in meters, and its
/// colors.
const CHECKPOINT_LINE: f32 = 0.6;
const CHECKPOINT_GLOW_REACH: f32 = 5.0;
const CHECKPOINT_NEON: LinearRgba = LinearRgba::new(0.3, 2.6, 1.0, 1.0);
const CHECKPOINT_GLOW: Color = Color::srgb(0.2, 1.0, 0.55);
const CHECKPOINT_GLOW_ALPHA: f32 = 0.35;

/// A world direction from one on the plane.
fn world_direction(direction: Vec2) -> Vec3 {
    crate::world::direction(direction)
}

/// A pool of blue light on the road at the foot of every neon stripe of the walls, brightest at
/// the wall and in the middle of the stripe, fading out across the road and toward its ends.
pub(super) fn wall_light(glow: &mut Geometry, track: &Track) {
    // The walls' neon stripes are the odd ones, counted from the start line.
    let neon = |distance: f32| (distance / WALL_STRIPE) as usize % 2 == 1;
    let along = |distance: f32| ((distance / WALL_STRIPE).fract() * PI).sin();
    let spacing = track.spacing();
    for side in [1.0_f32, -1.0] {
        for_each_road_segment(track, |a, b, distance| {
            // Where the wall is missing there is no neon to light the road: the open edge glows
            // amber instead, the color of what is worth a look.
            let open = track.breached(distance + spacing / 2.0, side);
            if !neon(distance) && !open {
                return;
            }
            let [near, far] = [distance, distance + spacing].map(|at| {
                if open {
                    return PAD_GLOW.with_alpha(PAD_POOL_ALPHA);
                }
                // Past the end of the stripe, the next one starts dark.
                let share = if neon(at) { along(at) } else { 0.0 };
                NEON_BLUE.with_alpha(POOL_ALPHA * share)
            });
            let dark = near.with_alpha(0.0);
            glow.quad_shaded(
                [
                    on_road(track, a, side * a.half_width, POOL_HEIGHT),
                    on_road(track, a, side * (a.half_width - POOL_REACH), POOL_HEIGHT),
                    on_road(track, b, side * (b.half_width - POOL_REACH), POOL_HEIGHT),
                    on_road(track, b, side * b.half_width, POOL_HEIGHT),
                ],
                Vec3::Y,
                [near, dark, dark, far],
            );
        });
    }
}

#[cfg(test)]
pub(super) mod tests {
    use star_racer_sim::track::{Segment, TrackDescription};

    use super::*;

    /// A counter-clockwise oval, 16 m wide: two straights of 100 m, two turns of radius 40.
    fn oval() -> Track {
        let turn = Segment::Turn {
            angle: 180.0,
            radius: 40.0,
            transition: 20.0,
            banking: 20.0,
        };
        Track::build(&TrackDescription {
            name: "Oval".into(),
            width: 16.0,
            segments: vec![
                Segment::Straight { length: 50.0 },
                turn,
                Segment::Straight { length: 100.0 },
                turn,
                Segment::Straight { length: 50.0 },
            ],
            elevation: Vec::new(),
            narrows: Vec::new(),
            scenery: Vec::new(),
            pads: Vec::new(),
            gaps: Vec::new(),
            checkpoints: Vec::new(),
            breaches: Vec::new(),
        })
        .unwrap()
    }

    fn on_plane(position: Vec3) -> Vec2 {
        Vec2::new(position.x, -position.z)
    }

    /// The oval with a gap of 10 m 60 m past the line, and a checkpoint at 150 m.
    pub(in crate::track) fn gapped_oval() -> Track {
        use star_racer_sim::track::Gap;
        let mut description = TrackDescription {
            name: "Gapped".into(),
            width: 16.0,
            segments: vec![
                Segment::Straight { length: 100.0 },
                Segment::Turn {
                    angle: 180.0,
                    radius: 40.0,
                    transition: 20.0,
                    banking: 0.0,
                },
                Segment::Straight { length: 200.0 },
                Segment::Turn {
                    angle: 180.0,
                    radius: 40.0,
                    transition: 20.0,
                    banking: 0.0,
                },
                Segment::Straight { length: 100.0 },
            ],
            elevation: Vec::new(),
            narrows: Vec::new(),
            scenery: Vec::new(),
            pads: Vec::new(),
            gaps: Vec::new(),
            checkpoints: vec![150.0],
            breaches: Vec::new(),
        };
        description.gaps = vec![Gap {
            at: 60.0,
            length: 10.0,
        }];
        Track::build(&description).unwrap()
    }

    /// The road, its underside and its paint stop at a gap, whose two edges are lined with neon;
    /// a checkpoint is a line of neon across the road.
    #[test]
    fn the_road_stops_at_a_gap_and_a_checkpoint_is_drawn_across_it() {
        let track = gapped_oval();
        let mut lit = Geometry::default();
        super::super::road(&mut lit, &track);
        super::super::underside(&mut lit, &track);
        edge_lines(&mut lit, &track);
        for position in lit.positions() {
            let at = track.project(on_plane(position)).distance;
            assert!(!(60.6..69.4).contains(&at), "road drawn in the gap, at {at} m");
        }
        let (mut neon, mut glow) = (Geometry::default(), Geometry::default());
        gap_lips(&mut lit, &mut neon, &mut glow, &track);
        let near = |at: f32| {
            neon.positions()
                .any(|position| (track.project(on_plane(position)).distance - at).abs() < 1.0)
        };
        assert!(near(60.0) && near(70.0), "a gap's edge is not lit");

        let mut neon = Geometry::default();
        checkpoints(&mut neon, &mut glow, &track);
        assert!(neon.triangles() > 0);
        for position in neon.positions() {
            let at = track.project(on_plane(position)).distance;
            assert!((at - 150.0).abs() < 1.0, "a checkpoint line at {at} m");
        }
    }

    /// Kerbs line the turns and nothing else, from the wall in, on the inside: the oval turns left,
    /// so every kerb is on its left.
    #[test]
    fn kerbs_line_the_inside_of_the_turns_only() {
        let track = oval();
        let mut lit = Geometry::default();
        kerbs(&mut lit, &track);
        assert!(lit.triangles() > 0);
        for position in lit.positions() {
            let here = track.project(on_plane(position));
            let curvature = track.point_at(here.distance).curvature;
            assert!(curvature > 0.0, "a kerb on a straight, {here:?}");
            assert!(here.lateral > 0.0, "a kerb on the outside, {here:?}");
            let from_wall = track.half_width_at(here.distance) - here.lateral;
            assert!((-0.05..=KERB_WIDTH + 0.05).contains(&from_wall), "{from_wall}");
        }
    }

    /// One box for each of the first grid slots, every one of them on the road and around its car.
    #[test]
    fn the_grid_is_drawn_around_its_slots() {
        let track = oval();
        let mut lit = Geometry::default();
        grid_boxes(&mut lit, &track);
        // Three lines a box, two triangles a line.
        assert_eq!(lit.triangles(), GRID_BOXES * 3 * 2);
        for slot in 0..GRID_BOXES {
            let (car, _) = track.grid_slot(slot);
            let near = lit
                .positions()
                .filter(|position| on_plane(*position).distance(car) < BOX_WIDTH)
                .count();
            assert!(near >= 12, "slot {slot}: {near} corners near the car");
        }
        for position in lit.positions() {
            let here = track.project(on_plane(position));
            assert!(here.lateral.abs() < track.half_width());
        }
    }

    /// The light of a stripe is gone by the time it reaches across the road, and only ever lies at
    /// the foot of a neon stripe.
    #[test]
    fn the_walls_light_fades_across_the_road() {
        let track = oval();
        let mut glow = Geometry::default();
        wall_light(&mut glow, &track);
        assert!(glow.triangles() > 0);
        for (position, color) in glow.positions().zip(glow.colors()) {
            let here = track.project(on_plane(position));
            let from_wall = track.half_width_at(here.distance) - here.lateral.abs();
            if from_wall > POOL_REACH - 0.05 {
                assert_eq!(color[3], 0.0, "light {from_wall} m from the wall");
            }
            assert!(color[3] <= POOL_ALPHA + 1e-6);
        }
    }
}
