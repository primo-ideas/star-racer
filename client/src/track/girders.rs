//! Where the road is missing, a broken bridge: over a gap (see `sim::track::Gap`) each wall goes on
//! as an open girder, a truss of the wall's height and outline, so it still holds a car no higher
//! than it, as the simulation says, but the void shows through it. A solid post closes the wall at
//! each end, and the truss's top chord carries the wall's dark and neon stripes on over the void.
//!
//! The walls used to carry on over a gap as they were, solid panels hanging in the air with nothing
//! under them: from the chase camera, a fence floating over a hole.

use bevy::prelude::*;
use star_racer_sim::track::{Track, TrackPoint};

use super::{NEON_BLUE, SLAB, WALL_DARK, WALL_HEIGHT, WALL_STRIPE, WALL_THICKNESS, beside};
use crate::geometry::{Frame, Geometry};
use crate::world;

/// How deep the top and bottom chords are, in meters.
const CHORD: f32 = 0.42;
/// A panel of the truss, between two posts, about this long along the road, in meters.
const PANEL: f32 = 4.5;
/// How thick a post and a brace are, and the end posts' length along the road, in meters.
const MEMBER: f32 = 0.26;
const END_POST: f32 = 0.7;
/// The steel: the walls' dark gray, a little lighter so the lattice reads against the night.
const STEEL: Color = Color::srgb(0.22, 0.22, 0.25);

/// The girders over every gap, both sides.
pub(super) fn girders(lit: &mut Geometry, neon: &mut Geometry, track: &Track) {
    // Built in world coordinates: a frame that leaves them as they are.
    let world_frame = Frame::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    let spacing = track.spacing();
    let points = track.points();
    for side in [1.0_f32, -1.0] {
        for run in gap_runs(track) {
            let (first, last) = (run.start, run.end);
            // The wall's cross-section at a point: its inner and outer faces, where it stands, its
            // top, and the bottom of the slab it would have stood on.
            let inner = |point: &TrackPoint| side * point.half_width;
            let outer = |point: &TrackPoint| side * (point.half_width + WALL_THICKNESS);
            let base = |point: &TrackPoint| track.height_beside(point, inner(point));
            let at = |point: &TrackPoint, lateral: f32, height: f32| {
                world::position(beside(*point, lateral), height)
            };
            let middle = |point: &TrackPoint, height: f32| {
                at(
                    point,
                    side * (point.half_width + WALL_THICKNESS / 2.0),
                    height,
                )
            };

            // The chords, a box per segment following the road.
            for index in first..last {
                let (a, b) = (&points[index], &points[(index + 1) % points.len()]);
                let distance = index as f32 * spacing;
                let ring = |height: fn(f32) -> f32| {
                    [
                        at(a, inner(a), height(base(a))),
                        at(a, outer(a), height(base(a))),
                        at(b, outer(b), height(base(b))),
                        at(b, inner(b), height(base(b))),
                    ]
                };
                let (geometry, color) = if (distance / WALL_STRIPE) as usize % 2 == 0 {
                    (&mut *lit, WALL_DARK)
                } else {
                    (&mut *neon, NEON_BLUE)
                };
                geometry.solid(
                    ring(|base| base + WALL_HEIGHT - CHORD),
                    ring(|base| base + WALL_HEIGHT),
                    color,
                    color,
                );
                lit.solid(
                    ring(|base| base - SLAB),
                    ring(|base| base - SLAB + CHORD),
                    STEEL,
                    STEEL,
                );
            }

            // The posts and the braces between the chords, panel by panel; the ends are solid.
            let length = (last - first) as f32 * spacing;
            let panels = (length / PANEL).round().max(1.0) as usize;
            let node_at = |panel: usize| {
                (first as f32 + (last - first) as f32 * panel as f32 / panels as f32) * spacing
            };
            let node = |panel: usize| track.point_at(node_at(panel));
            let low = |point: &TrackPoint| middle(point, base(point) - SLAB + CHORD);
            let high = |point: &TrackPoint| middle(point, base(point) + WALL_HEIGHT - CHORD);
            for panel in 0..=panels {
                let point = node(panel);
                if panel == 0 || panel == panels {
                    // Back into the road a little, so it caps the wall's open end.
                    let back = if panel == 0 { -1.0 } else { 1.0 };
                    let post =
                        track.point_at(node_at(panel) + back * (END_POST / 2.0 - spacing / 2.0));
                    let frame = Frame::new(
                        world::position(
                            beside(post, side * (post.half_width + WALL_THICKNESS / 2.0)),
                            base(&post),
                        ),
                        world::direction(post.direction),
                        world::direction(post.direction.perp() * side),
                    );
                    lit.block(
                        &frame,
                        Vec3::new(-END_POST / 2.0, -SLAB - 0.02, -WALL_THICKNESS / 2.0 - 0.06),
                        Vec3::new(
                            END_POST / 2.0,
                            WALL_HEIGHT + 0.08,
                            WALL_THICKNESS / 2.0 + 0.06,
                        ),
                        WALL_DARK,
                    );
                    continue;
                }
                lit.bar(&world_frame, low(&point), high(&point), MEMBER, STEEL);
            }
            for panel in 0..panels {
                let (near, far) = (node(panel), node(panel + 1));
                // A Warren truss: the braces zigzag, up then down.
                let (from, to) = if panel % 2 == 0 {
                    (low(&near), high(&far))
                } else {
                    (high(&near), low(&far))
                };
                lit.bar(&world_frame, from, to, MEMBER, STEEL);
            }
        }
    }
}

/// The runs of centerline segments over each gap, as ranges of the index of their first point: the
/// segments the road leaves out, so the girders meet the walls where the road stops.
fn gap_runs(track: &Track) -> Vec<std::ops::Range<usize>> {
    let spacing = track.spacing();
    let count = track.points().len();
    let missing = |index: usize| track.in_gap(index as f32 * spacing + spacing / 2.0);
    let mut runs = Vec::new();
    let mut index = 0;
    while index < count {
        if missing(index) {
            let start = index;
            while index < count && missing(index) {
                index += 1;
            }
            runs.push(start..index);
        } else {
            index += 1;
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::markings::tests::gapped_oval;

    /// A gap of 10 m gets one run of segments per gap, and its girders stay within the wall's
    /// outline: as high as the wall, no lower than the slab, beside the road and not over it.
    #[test]
    fn girders_span_the_gap_inside_the_walls_outline() {
        let track = gapped_oval();
        let runs = gap_runs(&track);
        assert_eq!(runs.len(), 1);
        let spacing = track.spacing();
        let (start, end) = (runs[0].start as f32 * spacing, runs[0].end as f32 * spacing);
        assert!(
            (start - 60.0).abs() <= spacing && (end - 70.0).abs() <= spacing,
            "{start} {end}"
        );

        let (mut lit, mut neon) = (Geometry::default(), Geometry::default());
        girders(&mut lit, &mut neon, &track);
        assert!(lit.triangles() > 0 && neon.triangles() > 0);
        for position in lit.positions().chain(neon.positions()) {
            let plane = Vec2::new(position.x, -position.z);
            let projection = track.project(plane);
            let point = track.point_at(projection.distance);
            assert!(
                projection.lateral.abs() >= point.half_width - 0.3,
                "a girder over the road, {} m from the centerline",
                projection.lateral
            );
            let base = track.height_beside(&point, projection.lateral.signum() * point.half_width);
            assert!(position.y <= base + WALL_HEIGHT + 0.2 && position.y >= base - SLAB - 0.2);
        }
    }
}
