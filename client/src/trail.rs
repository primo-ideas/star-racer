//! The trail every car leaves on the road behind it: a stylized line of neon laid where its tail
//! passed, like the mark a skid leaves, fading out in a second or so. The player picks one of three
//! effects and its color in the garage, with the rest of the car's look (see [`TRAILS`]).
//!
//! The trail is laid from the back of the car's body, not its middle: in a drift the body swings
//! across the travel and the tail swings out with it, so a drift draws its arc on the road the
//! way a skid would. Nothing is laid in the air or over a gap, there being no road there, and a
//! car brought back to a checkpoint starts a new trail rather than drawing a line across the
//! circuit. The simulation never sees it.

use std::collections::VecDeque;

use bevy::color::LinearRgba;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use star_racer_protocol::{CarId, CarLook};
use star_racer_sim::car::Car;

use crate::car_model::NEONS;
use crate::geometry::Geometry;
use crate::lobby::CurrentLobby;
use crate::race::{RaceUpdate, RaceView};
use crate::screen::Screen;
use crate::track::CurrentTrack;
use crate::world;

/// The trails, in the order the garage offers them and the wire numbers them, with their names.
/// The first three are free, the others won by playing (see `protocol::rewards`).
pub const TRAILS: [(&str, Effect); 6] = [
    ("RIBBON", Effect::Ribbon),
    ("SKID", Effect::Skid),
    ("PIXELS", Effect::Pixels),
    ("ZIGZAG", Effect::Zigzag),
    ("HELIX", Effect::Helix),
    ("ARROWS", Effect::Arrows),
];

/// The trail's colors: the neons'.
pub const TRAIL_COLORS: &[(&str, LinearRgba)] = &NEONS;

/// How a trail is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// A band of light as wide as the tail, edged with a brighter line, narrowing as it fades.
    Ribbon,
    /// Two thin, bright lines, one under each side of the tail: the marks of a skid, in neon.
    Skid,
    /// Squares of light dropped behind the tail, one side then the other, shrinking as they fade.
    Pixels,
    /// A bolt of lightning: one bright line snapping from one side of the tail to the other.
    Zigzag,
    /// Two lines winding round each other, the one in front brighter than the one behind.
    Helix,
    /// Arrows of light pointing the way the car went, every meter and a half.
    Arrows,
}

impl Effect {
    /// The effect numbered `index` on the wire; an unknown one is the first.
    pub fn of(index: u8) -> Self {
        TRAILS
            .get(usize::from(index))
            .map_or(Self::Ribbon, |(_, effect)| *effect)
    }
}

/// How long a trail lasts on the road, in seconds.
const LIFE: f32 = 1.1;
/// A new point every this many meters the tail moves.
const SAMPLE: f32 = 0.5;
/// A tail that moved further than this between two frames was put somewhere else: the trail
/// starts again rather than drawing a line there.
const BROKEN: f32 = 12.0;
/// How far behind the car's middle its tail is, along its body, in meters.
const TAIL: f32 = 1.9;
/// How far over the road the trail lies, in meters: over the road's flat strips, which stand a
/// few centimeters over its curve in the steepest banking, and under the car's shadow.
const LIFT: f32 = 0.08;
/// How wide the ribbon is and how wide its bright edges are; how far apart the skid's two lines
/// are and how wide each is; how big a pixel is and how far off the middle it lies; in meters.
const RIBBON_WIDTH: f32 = 1.5;
const RIBBON_EDGE: f32 = 0.12;
const SKID_APART: f32 = 1.7;
const SKID_WIDTH: f32 = 0.24;
const PIXEL_SIZE: f32 = 0.62;
const PIXEL_OFFSET: f32 = 0.42;
/// How bright each is against its color, which is already brighter than white.
const RIBBON_FILL: f32 = 0.35;
const RIBBON_BRIGHT: f32 = 0.9;
const SKID_BRIGHT: f32 = 0.85;
const PIXEL_BRIGHT: f32 = 0.75;

pub struct TrailPlugin;

impl Plugin for TrailPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_material)
            .add_systems(OnExit(Screen::Lobby), forget_trails)
            .add_systems(
                Update,
                lay_trails.after(RaceUpdate).run_if(in_state(Screen::Lobby)),
            );
    }
}

/// What every trail is drawn with: unlit and added to what is behind it, so a fading color fades
/// to nothing.
#[derive(Resource)]
pub struct TrailMaterial(pub Handle<StandardMaterial>);

fn create_material(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.insert_resource(TrailMaterial(materials.add(StandardMaterial {
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    })));
}

/// A point of a trail: where the tail was, on the road, and when.
#[derive(Debug, Clone, Copy)]
pub struct TrailPoint {
    pub at: Vec3,
    /// Seconds since the game started.
    pub time: f32,
    /// Whether the trail runs on from the point before this one.
    pub joined: bool,
    /// Counts the points laid, so a pixel stays where it was dropped.
    pub number: u32,
    /// Where it lies on the road: meters along the lap and from the centerline, so what is drawn
    /// beside it can be laid on the road too, banked and climbing as the road is.
    pub road: Vec2,
}

/// A car's trail as it is laid.
#[derive(Component)]
struct Trail {
    car: CarId,
    look: CarLook,
    points: VecDeque<TrailPoint>,
    laid: u32,
    /// The tail left the road since the last point: the next one starts a new trail.
    broken: bool,
    mesh: Handle<Mesh>,
}

fn forget_trails(mut commands: Commands, trails: Query<Entity, With<Trail>>) {
    for trail in &trails {
        commands.entity(trail).despawn();
    }
}

/// Lays every car's trail where its tail now is, lets the old end fade out, and draws it again.
#[allow(clippy::too_many_arguments)]
fn lay_trails(
    mut commands: Commands,
    view: Res<RaceView>,
    track: Option<Res<CurrentTrack>>,
    current: Res<CurrentLobby>,
    time: Res<Time>,
    material: Res<TrailMaterial>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut trails: Query<(Entity, &mut Trail)>,
) {
    let Some(track) = track else {
        return;
    };
    let now = time.elapsed_secs();
    let look_of = |id: CarId| {
        current
            .0
            .as_ref()
            .and_then(|membership| membership.player(id))
            .map_or(CarLook::default(), |player| player.look.checked())
    };

    // Each car's tail, on the road under it if there is road there.
    let tail = |car: &Car| {
        let heading = car.body_heading();
        let position = car.position - Vec2::new(heading.cos(), heading.sin()) * TAIL;
        let projection = track.0.project(position);
        let on_road = car.altitude <= 0.05 && track.0.over_road(&projection);
        let height = track.0.surface_at(&projection).height + LIFT;
        let road = Vec2::new(projection.distance, projection.lateral);
        (world::position(position, height), road, on_road)
    };

    let mut seen = Vec::new();
    for (entity, mut trail) in &mut trails {
        let Some(snapshot) = view.cars.iter().find(|car| car.id == trail.car) else {
            meshes.remove(&trail.mesh);
            commands.entity(entity).despawn();
            continue;
        };
        seen.push(trail.car);
        trail.look = look_of(trail.car);
        let (at, road, on_road) = tail(&snapshot.car);
        // Only while drifting (since v1.0.0-rc4, the user's word "Pense bien à n'activer la
        // visualisation du sillon que pendant le drift"): the mark a skid leaves, nothing else.
        let drifting = snapshot.car.is_drifting();
        lay(&mut trail, at, road, on_road && drifting, now);
        let effect = Effect::of(trail.look.trail);
        let color = TRAIL_COLORS[usize::from(trail.look.trail_color)].1;
        let points: Vec<TrailPoint> = trail.points.iter().copied().collect();
        // Every corner laid on the road beside the point it belongs to: across the road its height
        // follows the banking, along it the climb.
        let on_road = |point: &TrailPoint, corner: Vec3| {
            let road = track.0.point_at(point.road.x);
            let offset = Vec2::new(corner.x - point.at.x, -(corner.z - point.at.z));
            let along = point.road.x + offset.dot(road.direction);
            let lateral = point.road.y + offset.dot(road.direction.perp());
            let height = track.0.height_beside(&track.0.point_at(along), lateral);
            Vec3::new(corner.x, height + LIFT, corner.z)
        };
        let geometry = never_empty(trail_geometry(effect, color, &points, now, &on_road));
        if let Some(mut mesh) = meshes.get_mut(&trail.mesh) {
            *mesh = geometry.into_mesh();
        }
    }
    for snapshot in &view.cars {
        if seen.contains(&snapshot.id) {
            continue;
        }
        let mesh = meshes.add(never_empty(Geometry::default()).into_mesh());
        commands.spawn((
            Trail {
                car: snapshot.id,
                look: look_of(snapshot.id),
                points: VecDeque::new(),
                laid: 0,
                broken: false,
                mesh: mesh.clone(),
            },
            Mesh3d(mesh),
            MeshMaterial3d(material.0.clone()),
            Transform::default(),
            Visibility::default(),
            NotShadowCaster,
        ));
    }
}

/// The trail with at least one triangle, a black one of no size, which an added material shows as
/// nothing. A mesh emptied and filled again from frame to frame had the renderer copy into
/// storage it had already freed, and log it every frame.
fn never_empty(mut geometry: Geometry) -> Geometry {
    if geometry.triangles() == 0 {
        geometry.quad_shaded([Vec3::ZERO; 4], Vec3::Y, [Color::BLACK; 4]);
    }
    geometry
}

/// Lays a point where the tail is if it has moved far enough, breaks the trail where there is no
/// road or the car was put elsewhere, and drops what has faded out.
fn lay(trail: &mut Trail, at: Vec3, road: Vec2, on_road: bool, now: f32) {
    while trail
        .points
        .front()
        .is_some_and(|point| now - point.time > LIFE)
    {
        trail.points.pop_front();
    }
    if !on_road {
        trail.broken = true;
        return;
    }
    let moved = trail.points.back().map(|last| last.at.distance(at));
    if !trail.broken && moved.is_some_and(|moved| moved < SAMPLE) {
        return;
    }
    let joined = !trail.broken && moved.is_some_and(|moved| moved < BROKEN);
    trail.broken = false;
    trail.laid += 1;
    trail.points.push_back(TrailPoint {
        at,
        time: now,
        joined,
        number: trail.laid,
        road,
    });
}

/// Corners left where they are: the garage's floor is flat.
pub fn flat(_: &TrailPoint, corner: Vec3) -> Vec3 {
    corner
}

/// The trail's triangles, from its points, oldest first, faded by their age at `now`, each corner
/// laid on the `ground` beside the point it belongs to.
pub fn trail_geometry(
    effect: Effect,
    color: LinearRgba,
    points: &[TrailPoint],
    now: f32,
    ground: &dyn Fn(&TrailPoint, Vec3) -> Vec3,
) -> Geometry {
    let mut geometry = Geometry::default();
    let fade = |point: &TrailPoint| (1.0 - (now - point.time) / LIFE).clamp(0.0, 1.0);
    let shade = |share: f32| Color::LinearRgba(color * share);
    // A quad, its corners laid on the ground beside `point`, the one they belong to.
    let put = |geometry: &mut Geometry, point: &TrailPoint, corners: [Vec3; 4], colors: [Color; 4]| {
        geometry.quad_shaded(corners.map(|corner| ground(point, corner)), Vec3::Y, colors);
    };
    // Across the trail at each point, level, to the right of the way it runs.
    let across = |index: usize| {
        let before = points[index.saturating_sub(1)].at;
        let after = points[(index + 1).min(points.len() - 1)].at;
        let along = Vec3::new(after.x - before.x, 0.0, after.z - before.z);
        along
            .try_normalize()
            .map_or(Vec3::X, |along| along.cross(Vec3::Y))
    };

    match effect {
        Effect::Ribbon | Effect::Skid => {
            // Strips between each pair of joined points, each `(offset, width, brightness)`.
            let strips: &[(f32, f32, f32)] = match effect {
                Effect::Ribbon => &[
                    (0.0, RIBBON_WIDTH, RIBBON_FILL),
                    (-RIBBON_WIDTH / 2.0, RIBBON_EDGE, RIBBON_BRIGHT),
                    (RIBBON_WIDTH / 2.0, RIBBON_EDGE, RIBBON_BRIGHT),
                ],
                _ => &[
                    (-SKID_APART / 2.0, SKID_WIDTH, SKID_BRIGHT),
                    (SKID_APART / 2.0, SKID_WIDTH, SKID_BRIGHT),
                ],
            };
            for index in 1..points.len() {
                let (a, b) = (&points[index - 1], &points[index]);
                if !b.joined {
                    continue;
                }
                let (fade_a, fade_b) = (fade(a), fade(b));
                let (across_a, across_b) = (across(index - 1), across(index));
                for &(offset, width, bright) in strips {
                    // The ribbon narrows as it fades; the edges follow it in.
                    let narrow = |fade: f32| {
                        if effect == Effect::Ribbon {
                            0.35 + 0.65 * fade
                        } else {
                            1.0
                        }
                    };
                    let edge = |point: &TrailPoint, across: Vec3, fade: f32, side: f32| {
                        let middle = offset * narrow(fade);
                        point.at + across * (middle + side * width / 2.0 * narrow(fade))
                    };
                    put(&mut geometry, a,
                        [
                            edge(a, across_a, fade_a, -1.0),
                            edge(a, across_a, fade_a, 1.0),
                            edge(b, across_b, fade_b, 1.0),
                            edge(b, across_b, fade_b, -1.0),
                        ],
                        [
                            shade(bright * fade_a * fade_a),
                            shade(bright * fade_a * fade_a),
                            shade(bright * fade_b * fade_b),
                            shade(bright * fade_b * fade_b),
                        ],
                    );
                }
            }
        }
        Effect::Pixels => {
            for (index, point) in points.iter().enumerate() {
                // Every other point, one side of the tail then the other.
                if point.number % 2 != 0 {
                    continue;
                }
                let fade = fade(point);
                let side = if point.number % 4 == 0 { 1.0 } else { -1.0 };
                let across = across(index);
                let along = across.cross(Vec3::NEG_Y);
                let half = PIXEL_SIZE * (0.25 + 0.75 * fade) / 2.0;
                let middle = point.at + across * side * PIXEL_OFFSET;
                let corner = |x: f32, z: f32| middle + along * (x * half) + across * (z * half);
                let color = shade(PIXEL_BRIGHT * fade);
                put(&mut geometry, point,
                    [
                        corner(-1.0, -1.0),
                        corner(-1.0, 1.0),
                        corner(1.0, 1.0),
                        corner(1.0, -1.0),
                    ],
                    [color; 4],
                );
            }
        }
        Effect::Zigzag | Effect::Helix => {
            // Lines whose place across the tail changes from point to point: `(offset, light)` of
            // each line at a point, from its number.
            let lines: &[fn(u32) -> (f32, f32)] = match effect {
                Effect::Zigzag => &[|number| {
                    let side = if number % 2 == 0 { 1.0 } else { -1.0 };
                    (side * ZIGZAG_SWING, 1.0)
                }],
                _ => &[
                    |number| {
                        let angle = number as f32 * HELIX_TURN;
                        (HELIX_RADIUS * angle.sin(), 0.55 + 0.45 * angle.cos())
                    },
                    |number| {
                        let angle = number as f32 * HELIX_TURN + std::f32::consts::PI;
                        (HELIX_RADIUS * angle.sin(), 0.55 + 0.45 * angle.cos())
                    },
                ],
            };
            let width = if effect == Effect::Zigzag {
                ZIGZAG_WIDTH
            } else {
                HELIX_WIDTH
            };
            for index in 1..points.len() {
                let (a, b) = (&points[index - 1], &points[index]);
                if !b.joined {
                    continue;
                }
                let (across_a, across_b) = (across(index - 1), across(index));
                for line in lines {
                    let ((offset_a, light_a), (offset_b, light_b)) = (line(a.number), line(b.number));
                    let (fade_a, fade_b) = (fade(a), fade(b));
                    let (shade_a, shade_b) = (
                        shade(LINE_BRIGHT * light_a * fade_a * fade_a),
                        shade(LINE_BRIGHT * light_b * fade_b * fade_b),
                    );
                    put(&mut geometry, a,
                        [
                            a.at + across_a * (offset_a - width / 2.0),
                            a.at + across_a * (offset_a + width / 2.0),
                            b.at + across_b * (offset_b + width / 2.0),
                            b.at + across_b * (offset_b - width / 2.0),
                        ],
                        [shade_a, shade_a, shade_b, shade_b],
                    );
                }
            }
        }
        Effect::Arrows => {
            for (index, point) in points.iter().enumerate() {
                if point.number % ARROW_EVERY != 0 {
                    continue;
                }
                let fade = fade(point);
                let across = across(index);
                let along = across.cross(Vec3::NEG_Y);
                let color = shade(LINE_BRIGHT * fade);
                // Two strokes from the tail's sides, behind, to the tip in the middle, ahead.
                for side in [-1.0f32, 1.0] {
                    let back = point.at + across * (side * ARROW_HALF) - along * ARROW_DEPTH;
                    let stroke = along * ARROW_STROKE;
                    put(&mut geometry, point,
                        [back, point.at, point.at + stroke, back + stroke],
                        [color; 4],
                    );
                }
            }
        }
    }
    geometry
}

/// The zigzag: how far either side of the middle it snaps, and how wide its line is; the helix:
/// how far its lines wind out, how far round they turn from one point to the next, in radians,
/// and how wide each is; the arrows: one every this many points, how far either side their arms
/// reach, how far behind the tip, and how thick; and how bright the lines are against their color,
/// in meters where they are lengths.
const ZIGZAG_SWING: f32 = 0.55;
const ZIGZAG_WIDTH: f32 = 0.2;
const HELIX_RADIUS: f32 = 0.75;
const HELIX_TURN: f32 = 0.55;
const HELIX_WIDTH: f32 = 0.16;
const ARROW_EVERY: u32 = 3;
const ARROW_HALF: f32 = 0.75;
const ARROW_DEPTH: f32 = 0.7;
const ARROW_STROKE: f32 = 0.22;
const LINE_BRIGHT: f32 = 0.85;

/// A trail as the garage shows it under its turning car: a curve swept out behind it, laid as if
/// the car had just driven it, in the car's own coordinates.
pub fn preview_points() -> Vec<TrailPoint> {
    let count = 20;
    (0..count)
        .map(|index| {
            // From the far end, oldest, to just behind the tail.
            let share = index as f32 / (count - 1) as f32;
            let back = TAIL + 9.0 * (1.0 - share);
            let swing = 1.6 * (1.0 - share) * (1.0 - share);
            TrailPoint {
                at: Vec3::new(-back, LIFT, swing),
                time: share * LIFE * 0.85,
                joined: index > 0,
                number: index as u32,
                road: Vec2::ZERO,
            }
        })
        .collect()
}

/// The moment [`preview_points`] are seen at.
pub const PREVIEW_NOW: f32 = LIFE * 0.85;

#[cfg(test)]
mod tests {
    use super::*;

    fn line(count: usize, spacing: f32) -> Vec<TrailPoint> {
        (0..count)
            .map(|index| TrailPoint {
                at: Vec3::new(index as f32 * spacing, 0.0, 0.0),
                time: index as f32 * 0.02,
                joined: index > 0,
                number: index as u32,
                road: Vec2::ZERO,
            })
            .collect()
    }

    /// Every effect draws something, flat on the road, and its oldest end has faded the most.
    #[test]
    fn every_trail_draws_and_fades_toward_its_old_end() {
        let points = line(30, SAMPLE);
        let now = 30.0 * 0.02;
        for (_, effect) in TRAILS {
            let geometry = trail_geometry(effect, NEONS[2].1, &points, now, &flat);
            assert!(geometry.triangles() > 0, "{effect:?}");
            assert!(
                geometry.positions().all(|at| at.y.abs() < 1e-4),
                "{effect:?}"
            );
            // Brightness by where it lies along the line.
            let mut by_place: Vec<(f32, f32)> = geometry
                .positions()
                .zip(geometry.colors())
                .map(|(at, [r, g, b, _])| (at.x, r.max(g).max(b)))
                .collect();
            by_place.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (old, new) = (by_place[0].1, by_place[by_place.len() - 1].1);
            assert!(new > old, "{effect:?}: {old} {new}");
        }
    }

    /// Every corner of every effect is laid on the ground it is given, beside its own point: on a
    /// road banked one way, the trail's edges stand as high as the road does under them.
    #[test]
    fn every_corner_lies_on_the_ground() {
        let points = line(30, SAMPLE);
        // A road banked across: a meter up for every ten meters to the right (`+z`).
        let banked = |_: &TrailPoint, corner: Vec3| Vec3::new(corner.x, corner.z * 0.1, corner.z);
        for (_, effect) in TRAILS {
            let geometry = trail_geometry(effect, NEONS[3].1, &points, 0.6, &banked);
            for corner in geometry.positions() {
                assert!((corner.y - corner.z * 0.1).abs() < 1e-5, "{effect:?} {corner}");
            }
            assert!(geometry.positions().any(|corner| corner.y.abs() > 0.01), "{effect:?}");
        }
    }

    /// Nothing is drawn across a break, and a faded trail draws nothing visible.
    #[test]
    fn a_trail_is_not_drawn_across_a_break() {
        let mut points = line(10, SAMPLE);
        for point in &mut points {
            point.joined = false;
        }
        let geometry = trail_geometry(Effect::Ribbon, NEONS[0].1, &points, 0.2, &flat);
        assert_eq!(geometry.triangles(), 0);
        let faded = trail_geometry(Effect::Skid, NEONS[0].1, &line(10, SAMPLE), 10.0, &flat);
        assert!(faded.colors().all(|[r, g, b, _]| r.max(g).max(b) < 1e-4));
    }

    /// The lists match what the wire allows, and an unknown trail is the first.
    #[test]
    fn the_lists_match_the_wire() {
        use star_racer_protocol::{CAR_TRAIL_COLORS, CAR_TRAILS};
        assert_eq!(TRAILS.len(), usize::from(CAR_TRAILS));
        assert_eq!(TRAIL_COLORS.len(), usize::from(CAR_TRAIL_COLORS));
        assert_eq!(Effect::of(200), Effect::Ribbon);
    }
}
