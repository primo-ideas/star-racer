//! The car's gauges, drawn as arcs of one circle around it, near the middle of the screen where
//! the eye already is: the drift on the left, the turbo meter underneath, the thruster's tank on
//! the right.
//!
//! The drift arc is where a drift's charge is read (since v0.1.78, when the arc of segments drawn
//! on the road under the car was folded into it, at the user's word "intègre y le chargement du
//! drift"): it fills with the charge itself, dim until the charge pays a boost, past a notch
//! where it starts to, bright with the boost on offer written beside it, white and pulsing once
//! full; then amber while the boost it paid runs out. The thruster's arc shows only while its
//! tank is not full or the car flies (since v1.0.0-rc1).
//!
//! Since v1.0.0-rc3 (the user's word: "Je veux des lignes beaucoup plus jolies pour les jauges,
//! des courbes neon en gros") each arc is a real curve: one node with a material of its own,
//! [`ArcMaterial`], whose shader (`arc.wgsl`) draws a round-ended line of neon, white-hot along its
//! middle and at its lit end, in a glow that fades out around it, over a quiet line where it is
//! not lit; the turbo's is cut into its three charges. Until then each arc was a row of short
//! rectangles laid end to end, which no glow could make into a curve.

use std::f32::consts::TAU;

use bevy::asset::embedded_asset;
use bevy::color::Mix;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use bevy::ui_render::UiMaterialPlugin;
use bevy::ui_render::ui_material::{MaterialNode, UiMaterial};

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
/// The turbo meter, under the car, from left to right, one piece per charge.
const TURBO_ARC: (f32, f32) = (118.0, 62.0);
/// Between two charges of the turbo meter, as a share of a charge.
const TURBO_GAP: f32 = 0.12;
const TURBO_CHARGES: f32 = star_racer_sim::car::TURBO_CHARGES as f32;

/// The circle's center, as a share of the screen's width and height: a little under the middle,
/// where the chase camera holds the car.
const CENTER: (f32, f32) = (50.0, 58.0);

/// An unlit line.
const UNLIT: Color = Color::srgba(0.55, 0.65, 0.85, 0.22);
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

/// What the arc shader reads (see `arc.wgsl`).
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct ArcMaterial {
    #[uniform(0)]
    arc: ArcUniform,
}

#[derive(Debug, Clone, Copy, Default, ShaderType)]
struct ArcUniform {
    shape: Vec4,
    fill: Vec4,
    color: Vec4,
    dim: Vec4,
    cut: Vec4,
}

impl UiMaterial for ArcMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://star_racer_client/ui/arc.wgsl".into()
    }
}

pub struct GaugesPlugin;

impl Plugin for GaugesPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "arc.wgsl");
        app.add_plugins(UiMaterialPlugin::<ArcMaterial>::default());
    }
}

/// Part of an arc, its line or its label, and which arc it is.
#[derive(Component)]
pub struct ArcNode(Arc);

/// An arc's name and reading, beside its far end.
#[derive(Component)]
pub struct Label(Arc);

/// The node holding every arc, shown while there is a car.
#[derive(Component)]
pub struct GaugeRing;

/// Sizes on a desk and on a phone: the circle's radius, the line's thickness, how far its glow
/// reaches, and the labels' size.
fn sizes(phone: bool) -> (f32, f32, f32, f32) {
    if phone {
        (114.0, 9.0, 10.0, 10.5)
    } else {
        (190.0, 15.0, 18.0, 13.0)
    }
}

/// Where an arc runs: its start and signed sweep, in radians.
fn span(arc: Arc) -> (f32, f32) {
    let (from, to) = match arc {
        Arc::Drift => DRIFT_ARC,
        Arc::Turbo => TURBO_ARC,
        Arc::Thrust => THRUST_ARC,
    };
    (from.to_radians(), (to - from).to_radians())
}

/// Spawns the ring of gauges under `root`, the HUD's full-screen node.
pub fn spawn_gauges(
    root: &mut ChildSpawnerCommands,
    fonts: &Fonts,
    materials: &mut Assets<ArcMaterial>,
    phone: bool,
) {
    let (radius, thickness, glow, label_size) = sizes(phone);
    let box_half = radius + thickness + glow * 2.5;
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
            let (start, sweep) = span(arc);
            let pieces = if arc == Arc::Turbo { TURBO_CHARGES } else { 1.0 };
            let material = materials.add(ArcMaterial {
                arc: ArcUniform {
                    shape: Vec4::new(start, sweep, radius, thickness),
                    fill: Vec4::new(0.0, 0.0, glow, 1.0),
                    color: color_vec(theme::NEON),
                    dim: color_vec(UNLIT),
                    cut: Vec4::new(pieces, TURBO_GAP, -1.0, 0.0),
                },
            });
            ring.spawn((
                ArcNode(arc),
                Pickable::IGNORE,
                MaterialNode(material),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-box_half),
                    top: px(-box_half),
                    width: px(box_half * 2.0),
                    height: px(box_half * 2.0),
                    ..default()
                },
            ));
            // The name and its reading, past the arc's far end, or under the turbo meter.
            let (angle, out, width, justify) = match arc {
                Arc::Drift => (DRIFT_ARC.1 + 6.0, 0.0, 160.0, Justify::Right),
                Arc::Thrust => (THRUST_ARC.1 - 6.0, 0.0, 160.0, Justify::Left),
                Arc::Turbo => (90.0, label_size + 22.0, 160.0, Justify::Center),
            };
            let at = Vec2::from_angle(angle.to_radians()) * (radius + out);
            let left = match justify {
                Justify::Right => at.x - width,
                Justify::Left => at.x,
                _ => at.x - width / 2.0,
            };
            ring.spawn((
                ArcNode(arc),
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
    });
}

/// A color as the shader takes it: linear, straight alpha.
fn color_vec(color: Color) -> Vec4 {
    Vec4::from_array(color.to_linear().to_f32_array())
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

/// What an arc is lit with: how far, its color, how bright and how white its lit end burns, and
/// where its notch is (negative for none).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Lit {
    fill: f32,
    color: Color,
    bright: f32,
    head: f32,
    notch: f32,
}

/// Draws the arcs from the HUD's reading.
pub fn update_gauges(
    gauges: Res<Gauges>,
    time: Res<Time>,
    mut materials: ResMut<Assets<ArcMaterial>>,
    mut rings: Query<&mut Node, With<GaugeRing>>,
    arcs: Query<(&ArcNode, &MaterialNode<ArcMaterial>)>,
    mut parts: Query<(&ArcNode, &mut Visibility)>,
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
    // From 0 to 1 and back, for what pulses.
    let pulse = 0.5 - 0.5 * (time.elapsed_secs() * PULSE * TAU).cos();
    let (drift, drift_fill, threshold) = gauges.drift;
    let (drift_color, drift_bright, drift_label, drift_label_color) = drift_look(drift, pulse);
    let (turbo, firing) = gauges.turbo;
    let (thrust, thrust_active, burning) = gauges.thrust;
    let thrust_color = if burning {
        Color::from(Srgba::from(theme::NEON).mix(&Srgba::WHITE, 0.35))
    } else if thrust < THRUST_LOW && thrust_active {
        theme::RED
    } else {
        theme::NEON
    };

    for (arc, handle) in &arcs {
        let lit = match arc.0 {
            Arc::Drift => Lit {
                fill: drift_fill,
                // Charging, it fills in the dim and lights up past the notch.
                color: if drift_bright {
                    drift_color
                } else {
                    drift_color.with_alpha(0.55)
                },
                bright: if drift_bright { 1.5 } else { 0.8 },
                head: if drift_bright { 0.8 } else { 0.0 },
                notch: if matches!(drift, Drift::Charging | Drift::Ready(_) | Drift::Full(_)) {
                    threshold.clamp(0.0, 1.0)
                } else {
                    -1.0
                },
            },
            Arc::Turbo => Lit {
                fill: (turbo / TURBO_CHARGES).clamp(0.0, 1.0),
                color: if firing { theme::TEXT } else { theme::MAGENTA },
                bright: if turbo >= 1.0 { 1.5 } else { 0.9 },
                head: 0.0,
                notch: -1.0,
            },
            Arc::Thrust => Lit {
                fill: thrust.clamp(0.0, 1.0),
                color: thrust_color,
                bright: if thrust_active { 1.5 } else { 0.9 },
                head: if burning { 0.9 } else { 0.0 },
                notch: -1.0,
            },
        };
        let current = match materials.get(&handle.0) {
            Some(material) => material.arc,
            None => continue,
        };
        let wanted = ArcUniform {
            fill: Vec4::new(0.0, lit.fill, current.fill.z, lit.bright),
            color: color_vec(lit.color),
            cut: Vec4::new(current.cut.x, current.cut.y, lit.notch, lit.head),
            ..current
        };
        // Only a change marks the material for upload again.
        if (wanted.fill, wanted.color, wanted.cut) != (current.fill, current.color, current.cut)
            && let Some(mut material) = materials.get_mut(&handle.0)
        {
            material.arc = wanted;
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drift glows once it pays, says what it pays, and turns amber as a boost.
    #[test]
    fn a_drift_glows_once_it_pays() {
        assert!(!drift_look(Drift::Charging, 0.0).1);
        assert!(drift_look(Drift::Ready(0.4), 0.0).1);
        assert!(drift_look(Drift::Ready(0.4), 0.0).2.contains("+0.4s"));
        assert!(drift_look(Drift::Full(1.7), 0.0).2.starts_with("MAX"));
        assert_eq!(drift_look(Drift::Boost(1.0), 0.0).0, theme::AMBER);
    }

    /// The drift on the left, the turbo under the car from left to right, the thrust on the right,
    /// none of them overlapping.
    #[test]
    fn the_arcs_keep_their_places() {
        let ends = |arc| {
            let (start, sweep) = span(arc);
            let (a, b) = (start.to_degrees(), (start + sweep).to_degrees());
            (a.min(b), a.max(b))
        };
        let (drift, turbo, thrust) = (ends(Arc::Drift), ends(Arc::Turbo), ends(Arc::Thrust));
        assert!(drift.0 > turbo.1 && turbo.0 > thrust.1);
        assert!(span(Arc::Turbo).1 < 0.0, "the turbo fills from the left");
    }
}
