//! The car's gauges, drawn as arcs of one circle around it, near the middle of the screen where
//! the eye already is: the drift on the left, the turbo meter underneath, the thruster's tank on
//! the right.
//!
//! The drift arc is where a drift's charge is read (since v0.1.78, when the arc of segments drawn
//! on the road under the car was folded into it, at the user's word "intègre y le chargement du
//! drift"): it fills with the charge itself, dim until the charge pays a boost, past a notch
//! where it starts to, bright with the boost on offer written beside it, white and pulsing once
//! full; then amber while the boost it paid runs out.
//!
//! Bevy's UI has no arc, so each one is a row of short segments set end to end along the circle
//! and turned with it: since v1.0.0-rc1 one thin line of neon in a light outline, the thruster's
//! arc hidden while its tank is full and the car on the road. What is lit
//! glows, and its leading end burns brighter. The HUD writes a [`Gauges`] reading every frame;
//! [`update_gauges`] draws the arcs from it.

use std::f32::consts::{FRAC_PI_2, TAU};

use bevy::color::Mix;
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
const TURBO_ARC: (f32, f32) = (118.0, 62.0);
/// Between two charges of the turbo meter, in degrees.
const TURBO_GAP: f32 = 3.0;
const TURBO_CHARGES: usize = star_racer_sim::car::TURBO_CHARGES as usize;
/// Segments of the drift and thrust arcs, and of each charge of the turbo meter.
const ARC_SEGMENTS: usize = 48;
const CHARGE_SEGMENTS: usize = 16;
/// The share of each step along the arc a segment covers: all of it, so the segments meet and
/// draw one line.
const SEGMENT_SHARE: f32 = 1.0;

/// The circle's center, as a share of the screen's width and height: a little under the middle,
/// where the chase camera holds the car.
const CENTER: (f32, f32) = (50.0, 58.0);

/// An unlit segment, and the track behind every arc.
const UNLIT: Color = Color::srgba(0.55, 0.65, 0.85, 0.10);
/// The light outline every arc lies in (since v1.0.0-rc1, the user's word "Fais que les jauges ne
/// soient qu'une ligne neon jolie avec une outline légère"; a dark band twice as thick before).
const TRACK: Color = Color::srgba(0.75, 0.85, 1.0, 0.16);
/// How much wider than the line its outline is, in pixels.
const OUTLINE: f32 = 3.0;
/// The thruster's tank below this share of itself turns red.
const THRUST_LOW: f32 = 0.25;
/// How fast a full drift pulses, in pulses per second.
const PULSE: f32 = 3.0;

/// Where a drift stands, for its arc.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Drift {
    /// No drift and no boost.
    #[default]
    Idle,
    /// Charging, not yet enough to pay a boost.
    Charging,
    /// Charged enough: ending it now pays this many seconds of boost.
    Ready(f32),
    /// Charged as far as it goes.
    Full(f32),
    /// The boost it paid, running out: seconds left.
    Boost(f32),
}

/// What the gauges read this frame, written by the HUD.
#[derive(Resource, Default)]
pub struct Gauges {
    /// Whether there is a car to read them from.
    pub shown: bool,
    /// The drift: where it stands, how full its arc is from 0 to 1, and where on the arc the
    /// charge starts to pay a boost.
    pub drift: (Drift, f32, f32),
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

/// An arc's name and reading, beside its far end.
#[derive(Component)]
pub struct Label(Arc);

/// The mark across the drift arc where its charge starts to pay a boost, with the radius and
/// thickness of the ring it was built on.
#[derive(Component)]
pub struct Notch {
    radius: f32,
    thickness: f32,
}

/// Everything drawn for an arc: its outline, its line and its label, shown or hidden together.
#[derive(Component)]
pub struct Part(Arc);

/// The node holding every arc, shown while there is a car.
#[derive(Component)]
pub struct GaugeRing;

/// Sizes on a desk and on a phone: the circle's radius, a segment's thickness, and the labels'.
fn sizes(phone: bool) -> (f32, f32, f32) {
    if phone {
        (108.0, 3.0, 10.5)
    } else {
        (178.0, 4.0, 13.0)
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

/// A node `length` long and `thickness` thick, its middle at `angle` degrees on a circle of
/// `radius`, lying along the circle.
fn on_circle(angle: f32, radius: f32, length: f32, thickness: f32) -> (Node, UiTransform) {
    let at = Vec2::from_angle(angle.to_radians()) * radius;
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x - length / 2.0),
            top: px(at.y - thickness / 2.0),
            width: px(length),
            height: px(thickness),
            ..default()
        },
        UiTransform::from_rotation(Rot2::radians(angle.to_radians() + FRAC_PI_2)),
    )
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
            // The track first, so the arc is drawn over it.
            for (_, angle) in segments(arc) {
                ring.spawn((
                    on_circle(angle, radius, length + 1.5, thickness + OUTLINE),
                    BackgroundColor(TRACK),
                    Part(arc),
                ));
            }
            for (index, angle) in segments(arc) {
                ring.spawn((
                    Part(arc),
                    Segment { arc, index },
                    on_circle(angle, radius, length, thickness),
                    BackgroundColor(UNLIT),
                    BoxShadow(Vec::new()),
                ));
            }
            // The name and its reading, past the arc's far end, or under the turbo meter.
            let (angle, out, width, justify) = match arc {
                Arc::Drift => (DRIFT_ARC.1 + 6.0, 0.0, 150.0, Justify::Right),
                Arc::Thrust => (THRUST_ARC.1 - 6.0, 0.0, 150.0, Justify::Left),
                Arc::Turbo => (90.0, label_size + 20.0, 150.0, Justify::Center),
            };
            let at = Vec2::from_angle(angle.to_radians()) * (radius + out);
            let left = match justify {
                Justify::Right => at.x - width,
                Justify::Left => at.x,
                _ => at.x - width / 2.0,
            };
            ring.spawn((
                Part(arc),
                Label(arc),
                Text::default(),
                fonts.caption(label_size),
                theme::CAPTION_SPACING,
                TextColor(theme::TEXT_FAINT),
                theme::TEXT_SHADOW,
                TextLayout::justify(justify),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(left),
                    top: px(at.y - label_size * 1.6),
                    width: px(width),
                    ..default()
                },
            ));
        }
        // Across the drift arc, where a boost starts: placed by the HUD's reading.
        let (node, transform) = on_circle(DRIFT_ARC.0, radius, 2.5, thickness + 9.0);
        ring.spawn((
            Notch { radius, thickness },
            node,
            transform,
            BackgroundColor(theme::TEXT.with_alpha(0.85)),
        ));
    });
}

/// How lit a segment is, from 0 to 1: an arc of `count` segments `fill` full lights its first
/// segments whole and the next one by its share.
fn lit(fill: f32, index: usize, count: usize) -> f32 {
    (fill.clamp(0.0, 1.0) * count as f32 - index as f32).clamp(0.0, 1.0)
}

/// How a segment is drawn.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Look {
    color: Color,
    glow: bool,
}

/// A segment `share` lit in `color`: unlit, dim, or, `bright`, glowing, its leading end (`head`)
/// burning toward white.
fn shade(color: Color, share: f32, bright: bool, head: bool) -> Look {
    if share <= 0.0 {
        return Look {
            color: UNLIT,
            glow: false,
        };
    }
    if !bright {
        return Look {
            color: color.with_alpha(0.3 + 0.35 * share),
            glow: false,
        };
    }
    let color = if head {
        Color::from(Srgba::from(color).mix(&Srgba::WHITE, 0.55))
    } else {
        color
    };
    Look {
        color: color.with_alpha(0.45 + 0.55 * share),
        glow: true,
    }
}

/// Whether segment `index` of `count` is the last one lit by `fill`.
fn is_head(fill: f32, index: usize, count: usize) -> bool {
    lit(fill, index, count) > 0.0 && (index + 1 == count || lit(fill, index + 1, count) <= 0.0)
}

/// The drift arc's color, how bright it is, and its label, for a drift standing at `drift`.
fn drift_look(drift: Drift, pulse: f32) -> (Color, bool, String, Color) {
    match drift {
        Drift::Idle => (theme::GREEN, false, "DRIFT".into(), theme::TEXT_FAINT),
        Drift::Charging => (theme::GREEN, false, "DRIFT".into(), theme::GREEN),
        Drift::Ready(boost) => (
            theme::GREEN,
            true,
            format!("DRIFT  +{boost:.1}s"),
            theme::GREEN,
        ),
        Drift::Full(boost) => {
            let color = Color::from(Srgba::from(theme::GREEN).mix(&Srgba::WHITE, 0.6 * pulse));
            (color, true, format!("MAX  +{boost:.1}s"), theme::TEXT)
        }
        Drift::Boost(left) => (theme::AMBER, true, format!("BOOST  {left:.1}s"), theme::AMBER),
    }
}

/// Draws the arcs from the HUD's reading.
pub fn update_gauges(
    gauges: Res<Gauges>,
    time: Res<Time>,
    mut rings: Query<&mut Node, (With<GaugeRing>, Without<Notch>)>,
    mut segments: Query<(&Segment, &mut BackgroundColor, &mut BoxShadow), Without<Notch>>,
    mut labels: Query<(&Label, &mut Text, &mut TextColor)>,
    mut notches: Query<(&Notch, &mut Node, &mut UiTransform, &mut Visibility)>,
    mut parts: Query<(&Part, &mut Visibility), Without<Notch>>,
) {
    // The thruster's arc shows only while its tank is not full or the car flies (since
    // v1.0.0-rc1, the user's word "La barre de thrust doit n'apparaitre que quand elle n'est pas à
    // 100%").
    for (part, mut visibility) in &mut parts {
        let shown = part.0 != Arc::Thrust || gauges.thrust.1;
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
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
    // From 0 to 1 and back, for what pulses.
    let pulse = 0.5 - 0.5 * (time.elapsed_secs() * PULSE * TAU).cos();
    let (drift, drift_fill, threshold) = gauges.drift;
    let (drift_color, drift_bright, drift_label, drift_label_color) = drift_look(drift, pulse);
    let (turbo, firing) = gauges.turbo;
    let (thrust, thrust_active, burning) = gauges.thrust;
    let thrust_color = if burning {
        Color::from(Srgba::from(theme::NEON).mix(&Srgba::WHITE, 0.45))
    } else if thrust < THRUST_LOW && thrust_active {
        theme::RED
    } else {
        theme::NEON
    };

    for (segment, mut background, mut shadow) in &mut segments {
        let look = match segment.arc {
            Arc::Drift => {
                // Charging, the arc fills in the dim and lights up past the notch.
                let share = lit(drift_fill, segment.index, ARC_SEGMENTS);
                let head = is_head(drift_fill, segment.index, ARC_SEGMENTS);
                shade(drift_color, share, drift_bright, head && drift_bright)
            }
            Arc::Turbo => {
                let charge = segment.index / CHARGE_SEGMENTS;
                let index = segment.index % CHARGE_SEGMENTS;
                let filled = (turbo - charge as f32).clamp(0.0, 1.0);
                let share = lit(filled, index, CHARGE_SEGMENTS);
                // A whole charge, ready to fire, glows; one still filling is dim, its head
                // marking how far it has come.
                let ready = filled >= 1.0;
                let color = if firing && ready {
                    theme::TEXT
                } else {
                    theme::MAGENTA
                };
                shade(color, share, ready, false)
            }
            Arc::Thrust => {
                let share = lit(thrust, segment.index, ARC_SEGMENTS);
                let head = is_head(thrust, segment.index, ARC_SEGMENTS);
                // A full tank on the road is there, but quiet.
                shade(thrust_color, share, thrust_active, head && burning)
            }
        };
        background.set_if_neq(BackgroundColor(look.color));
        let glow = if look.glow {
            BoxShadow::new(look.color.with_alpha(0.8), px(0), px(0), px(1), px(9))
        } else {
            BoxShadow(Vec::new())
        };
        shadow.set_if_neq(glow);
    }

    for (label, mut text, mut color) in &mut labels {
        let (name, name_color) = match label.0 {
            Arc::Drift => (drift_label.clone(), drift_label_color),
            Arc::Turbo => {
                let ready = turbo.floor() as usize;
                let color = if firing {
                    theme::TEXT
                } else if ready > 0 {
                    theme::MAGENTA
                } else {
                    theme::TEXT_FAINT
                };
                let name = if ready > 0 {
                    format!("TURBO  ×{ready}")
                } else {
                    "TURBO".to_owned()
                };
                (name, color)
            }
            Arc::Thrust => {
                let color = if burning {
                    theme::TEXT
                } else if thrust_active {
                    thrust_color
                } else {
                    theme::TEXT_FAINT
                };
                ("THRUST".to_owned(), color)
            }
        };
        if text.0 != name {
            text.0 = name;
        }
        color.set_if_neq(TextColor(name_color));
    }

    // The notch, where the charge starts to pay, shown while a drift charges.
    for (notch, mut node, mut transform, mut visibility) in &mut notches {
        let shown = matches!(drift, Drift::Charging | Drift::Ready(_) | Drift::Full(_));
        visibility.set_if_neq(if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let angle = DRIFT_ARC.0 + (DRIFT_ARC.1 - DRIFT_ARC.0) * threshold.clamp(0.0, 1.0);
        let (placed, turned) = on_circle(angle, notch.radius, 2.5, notch.thickness + 9.0);
        if node.left != placed.left || node.top != placed.top {
            node.left = placed.left;
            node.top = placed.top;
        }
        // Thin along the arc and long across it.
        if transform.rotation != turned.rotation {
            transform.rotation = turned.rotation;
        }
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
        assert!(drift.iter().all(|(_, angle)| *angle > 120.0));
        assert!(turbo.iter().all(|(_, angle)| (60.0..120.0).contains(angle)));
        assert!(turbo.first().unwrap().1 > turbo.last().unwrap().1);
        assert!(thrust.iter().all(|(_, angle)| *angle < 60.0));
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
        // The head is the last segment lit, however little.
        assert!(is_head(0.25, 2, 10));
        assert!(!is_head(0.25, 1, 10));
        assert!(is_head(1.0, 9, 10));
        assert!(!is_head(0.0, 0, 10));
    }

    #[test]
    fn a_drift_glows_once_it_pays() {
        assert!(!drift_look(Drift::Charging, 0.0).1);
        assert!(drift_look(Drift::Ready(0.4), 0.0).1);
        assert!(drift_look(Drift::Ready(0.4), 0.0).2.contains("+0.4s"));
        assert!(drift_look(Drift::Full(1.7), 0.0).2.starts_with("MAX"));
        assert_eq!(drift_look(Drift::Boost(1.0), 0.0).0, theme::AMBER);
        // Unlit, dim and glowing are told apart.
        assert_eq!(shade(theme::GREEN, 0.0, true, false).color, UNLIT);
        assert!(!shade(theme::GREEN, 1.0, false, false).glow);
        assert!(shade(theme::GREEN, 1.0, true, false).glow);
        assert_ne!(
            shade(theme::GREEN, 1.0, true, true).color,
            shade(theme::GREEN, 1.0, true, false).color
        );
    }
}
