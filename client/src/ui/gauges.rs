//! The three gauges of the car, drawn as arcs of one circle around it, near the middle of the
//! screen where the eye already is: the drift charge (then the boost it gave) on the left, the
//! turbo meter's charges underneath, the thruster's tank on the right.
//!
//! Bevy's UI has no arc, so each one is a row of short segments set end to end along the circle
//! and turned with it: one neon line, lit from one end as it fills. The HUD writes a [`Gauges`]
//! reading every frame; [`update_gauges`] colors the segments from it.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use super::theme::{self, Fonts};

/// The three arcs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arc {
    Drift,
    Turbo,
    Thrust,
}

/// Where each arc runs, in degrees clockwise from the right (the screen's y points down, so 90 is
/// straight down), from the end it fills from to the other.
const DRIFT_ARC: (f32, f32) = (128.0, 178.0);
const THRUST_ARC: (f32, f32) = (52.0, 2.0);
/// The turbo meter, under the car, from left to right, one stretch per charge.
const TURBO_ARC: (f32, f32) = (106.0, 74.0);
/// Between two charges of the turbo meter, in degrees.
const TURBO_GAP: f32 = 2.5;
const TURBO_CHARGES: usize = star_racer_sim::car::TURBO_CHARGES as usize;
/// Segments of the drift and thrust arcs, and of each charge of the turbo meter.
const ARC_SEGMENTS: usize = 30;
const CHARGE_SEGMENTS: usize = 8;
/// The share of each step along the arc a segment covers: all of it, so the segments meet and
/// draw one line.
const SEGMENT_SHARE: f32 = 1.0;

/// The circle's center, as a share of the screen's width and height: a little under the middle,
/// where the chase camera holds the car.
const CENTER: (f32, f32) = (50.0, 58.0);

/// An unlit segment.
const UNLIT: Color = Color::srgba(0.02, 0.03, 0.07, 0.5);

/// What the gauges read this frame, written by the HUD.
#[derive(Resource, Default)]
pub struct Gauges {
    /// Whether there is a car to read them from.
    pub shown: bool,
    /// The drift arc: how full, its color and its name.
    pub drift: (f32, Color, &'static str),
    /// Whether the drift arc is lit at all (a drift, or the boost after one).
    pub drift_active: bool,
    /// The turbo meter, in charges, and whether one is being fired.
    pub turbo: (f32, bool),
    /// The thruster's tank, from 0 to 1; whether the car is in the air or the tank filling; and
    /// whether it burns.
    pub thrust: (f32, bool, bool),
}

/// One segment of an arc: which arc, and its place along it, counted from the end it fills from.
#[derive(Component)]
pub struct Segment {
    arc: Arc,
    index: usize,
}

/// An arc's name, beside its far end.
#[derive(Component)]
pub struct Label(Arc);

/// The node holding every arc, shown while there is a car.
#[derive(Component)]
pub struct GaugeRing;

/// Sizes on a desk and on a phone: the circle's radius, and a segment's thickness.
fn sizes(phone: bool) -> (f32, f32, f32) {
    if phone {
        (108.0, 5.0, 10.5)
    } else {
        (178.0, 7.0, 12.5)
    }
}

/// Every segment of an arc: its index, and its middle's angle in degrees.
fn segments(arc: Arc) -> Vec<(usize, f32)> {
    let along = |(from, to): (f32, f32), count: usize| {
        let step = (to - from) / count as f32;
        (0..count).map(move |index| (index, from + step * (index as f32 + 0.5)))
    };
    match arc {
        Arc::Drift => along(DRIFT_ARC, ARC_SEGMENTS).collect(),
        Arc::Thrust => along(THRUST_ARC, ARC_SEGMENTS).collect(),
        Arc::Turbo => {
            let (from, to) = TURBO_ARC;
            let gaps = TURBO_GAP * (TURBO_CHARGES - 1) as f32;
            let charge = ((to - from).abs() - gaps) / TURBO_CHARGES as f32 * (to - from).signum();
            (0..TURBO_CHARGES)
                .flat_map(|index| {
                    let start = from + index as f32 * (charge + TURBO_GAP * charge.signum());
                    along((start, start + charge), CHARGE_SEGMENTS)
                        .map(move |(segment, angle)| (index * CHARGE_SEGMENTS + segment, angle))
                })
                .collect()
        }
    }
}

/// The length of a segment of an arc, in pixels, on a circle of `radius`.
fn segment_length(arc: Arc, radius: f32) -> f32 {
    let (from, to, count) = match arc {
        Arc::Drift => (DRIFT_ARC.0, DRIFT_ARC.1, ARC_SEGMENTS),
        Arc::Thrust => (THRUST_ARC.0, THRUST_ARC.1, ARC_SEGMENTS),
        Arc::Turbo => {
            let gaps = TURBO_GAP * (TURBO_CHARGES - 1) as f32;
            let span = (TURBO_ARC.0 - TURBO_ARC.1).abs() - gaps;
            (0.0, span, TURBO_CHARGES * CHARGE_SEGMENTS)
        }
    };
    (to - from).abs().to_radians() / count as f32 * radius * SEGMENT_SHARE
}

/// Spawns the ring of gauges under `root`, the HUD's full-screen node.
pub fn spawn_gauges(root: &mut ChildSpawnerCommands, fonts: &Fonts, phone: bool) {
    let (radius, thickness, label_size) = sizes(phone);
    root.spawn((
        GaugeRing,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            left: percent(CENTER.0),
            top: percent(CENTER.1),
            width: px(0),
            height: px(0),
            display: Display::None,
            ..default()
        },
    ))
    .with_children(|ring| {
        for arc in [Arc::Drift, Arc::Turbo, Arc::Thrust] {
            let length = segment_length(arc, radius);
            for (index, angle) in segments(arc) {
                let at = Vec2::from_angle(angle.to_radians()) * radius;
                ring.spawn((
                    Segment { arc, index },
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(at.x - length / 2.0),
                        top: px(at.y - thickness / 2.0),
                        width: px(length),
                        height: px(thickness),
                        ..default()
                    },
                    // Along the circle: a segment lies across the radius through its middle.
                    UiTransform::from_rotation(Rot2::radians(angle.to_radians() + FRAC_PI_2)),
                    BackgroundColor(Color::NONE),
                ));
            }
            // The name, past the arc's far end, or under the turbo meter.
            let (angle, out) = match arc {
                Arc::Drift => (DRIFT_ARC.1 + 9.0, 0.0),
                Arc::Thrust => (THRUST_ARC.1 - 9.0, 0.0),
                Arc::Turbo => (90.0, label_size + 6.0),
            };
            let at = Vec2::from_angle(angle.to_radians()) * (radius + out);
            const WIDTH: f32 = 120.0;
            ring.spawn((
                Label(arc),
                Text::default(),
                fonts.caption(label_size),
                theme::CAPTION_SPACING,
                TextColor(theme::TEXT_FAINT),
                theme::TEXT_SHADOW,
                TextLayout::justify(Justify::Center),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(at.x - WIDTH / 2.0),
                    top: px(at.y - label_size * 0.7),
                    width: px(WIDTH),
                    ..default()
                },
            ));
        }
    });
}

/// How lit a segment is, from 0 to 1: an arc of `count` segments `fill` full lights its first
/// segments whole and the next one by its share.
fn lit(fill: f32, index: usize, count: usize) -> f32 {
    (fill.clamp(0.0, 1.0) * count as f32 - index as f32).clamp(0.0, 1.0)
}

/// A segment's color: `color` as far as it is lit, a dark track otherwise, which reads over the
/// pale road as over the night.
fn shade(color: Color, share: f32, strength: f32) -> Color {
    if share <= 0.0 {
        UNLIT
    } else {
        color.with_alpha((0.4 + 0.6 * share) * strength)
    }
}

/// Colors the arcs and writes their names from the HUD's reading.
pub fn update_gauges(
    gauges: Res<Gauges>,
    mut rings: Query<&mut Node, With<GaugeRing>>,
    mut segments: Query<(&Segment, &mut BackgroundColor)>,
    mut labels: Query<(&Label, &mut Text, &mut TextColor)>,
) {
    let display = if gauges.shown {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut rings {
        if node.display != display {
            node.display = display;
        }
    }
    if !gauges.shown {
        return;
    }
    let (drift, drift_color, _) = gauges.drift;
    let (turbo, firing) = gauges.turbo;
    let (thrust, thrust_active, burning) = gauges.thrust;
    for (segment, mut background) in &mut segments {
        let color = match segment.arc {
            Arc::Drift => {
                let color = if gauges.drift_active {
                    drift_color
                } else {
                    theme::GREEN
                };
                let share = if gauges.drift_active {
                    lit(drift, segment.index, ARC_SEGMENTS)
                } else {
                    0.0
                };
                shade(color, share, 1.0)
            }
            Arc::Turbo => {
                let charge = segment.index / CHARGE_SEGMENTS;
                let filled = (turbo - charge as f32).clamp(0.0, 1.0);
                let share = lit(filled, segment.index % CHARGE_SEGMENTS, CHARGE_SEGMENTS);
                // A whole charge, ready to fire, is lit; one still filling is dim.
                let strength = if filled >= 1.0 { 1.0 } else { 0.5 };
                let color = if firing && filled >= 1.0 {
                    theme::TEXT
                } else {
                    theme::MAGENTA
                };
                shade(color, share, strength)
            }
            Arc::Thrust => {
                let share = lit(thrust, segment.index, ARC_SEGMENTS);
                let color = if burning { theme::TEXT } else { theme::NEON };
                // A full tank on the road is there, but quiet.
                shade(color, share, if thrust_active { 1.0 } else { 0.45 })
            }
        };
        background.set_if_neq(BackgroundColor(color));
    }
    for (label, mut text, mut color) in &mut labels {
        let (name, name_color) = match label.0 {
            Arc::Drift => {
                let (_, color, name) = gauges.drift;
                let color = if gauges.drift_active {
                    color
                } else {
                    theme::TEXT_FAINT
                };
                (name, color)
            }
            Arc::Turbo => {
                let color = if firing {
                    theme::TEXT
                } else if turbo >= 1.0 {
                    theme::MAGENTA
                } else {
                    theme::TEXT_FAINT
                };
                ("TURBO", color)
            }
            Arc::Thrust => {
                let color = if burning {
                    theme::TEXT
                } else if thrust_active {
                    theme::NEON
                } else {
                    theme::TEXT_FAINT
                };
                ("THRUST", color)
            }
        };
        if text.0 != name {
            text.0 = name.to_owned();
        }
        color.set_if_neq(TextColor(name_color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arcs_have_their_segments_in_order_and_apart() {
        for arc in [Arc::Drift, Arc::Turbo, Arc::Thrust] {
            let all = segments(arc);
            let indices: Vec<usize> = all.iter().map(|(index, _)| *index).collect();
            assert_eq!(indices, (0..all.len()).collect::<Vec<_>>(), "{arc:?}");
            // Each segment is past the one before it, and the arcs never overlap.
            for pair in all.windows(2) {
                assert!((pair[1].1 - pair[0].1).abs() > 1.0, "{arc:?}");
            }
        }
        let (drift, turbo, thrust) = (
            segments(Arc::Drift),
            segments(Arc::Turbo),
            segments(Arc::Thrust),
        );
        assert_eq!(turbo.len(), TURBO_CHARGES * CHARGE_SEGMENTS);
        // Drift on the left, the turbo underneath from left to right, thrust on the right.
        assert!(drift.iter().all(|(_, angle)| *angle > 110.0));
        assert!(turbo.iter().all(|(_, angle)| (70.0..110.0).contains(angle)));
        assert!(turbo.first().unwrap().1 > turbo.last().unwrap().1);
        assert!(thrust.iter().all(|(_, angle)| *angle < 70.0));
        // Segments are no longer than their step along the circle, so they never overlap.
        for arc in [Arc::Drift, Arc::Turbo, Arc::Thrust] {
            let radius = sizes(false).0;
            let step = (segments(arc)[1].1 - segments(arc)[0].1).abs().to_radians() * radius;
            assert!(segment_length(arc, radius) <= step + 1e-3, "{arc:?}");
        }
    }

    #[test]
    fn segments_light_up_from_the_start() {
        assert_eq!(lit(0.0, 0, 10), 0.0);
        assert_eq!(lit(1.0, 9, 10), 1.0);
        assert_eq!(lit(0.25, 1, 10), 1.0);
        assert!((lit(0.25, 2, 10) - 0.5).abs() < 1e-5);
        assert_eq!(lit(0.25, 3, 10), 0.0);
        assert_eq!(lit(2.0, 9, 10), 1.0);
    }
}
