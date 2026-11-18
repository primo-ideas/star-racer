//! The cars, as drawn: stylized cars of the future, built from flat-shaded triangles like
//! everything else in the game.
//!
//! Six to choose from in the garage, every one driving the same, since the handling is the
//! simulation's and the simulation knows nothing of looks. The first three are free, the others
//! won by playing (see `protocol::rewards`):
//!
//! - **COMET**: a rocket of a car, narrow, tapering to one big thruster, a single fin on its spine.
//! - **BOLT**: a muscle car, a long bonnet, broad shoulders, fat rear wheels and a big wing.
//! - **PULSE**: a compact, short and round, a bubble of a cabin, big wheels at its corners.
//! - **PHANTOM** (won): a hypercar, the lowest of all, its canopy forward, twin fins at the tail.
//! - **TITAN** (won): an off-roader, high on huge wheels, a light bar across its roof.
//! - **NOVA** (won): an open-wheel racer, a needle of a body, its wheels out on arms, two wings.
//!
//! They float above the road: their wheels are hover-wheels, discs ringed with neon whose lowest
//! point clears it by a fifth of a meter, and a soft shadow and colored light spilling out from
//! under the body say the car hovers. Thrusters glow at the back, their flames lengthening with
//! the speed and roaring out on a boost. The player picks the paint, which covers the body, and
//! the neon, which lines it, rings the wheels and lights the road under it (see [`PAINTS`],
//! [`NEONS`]); the neon and the thrusters are drawn brighter than white, so the bloom gives them
//! a halo.
//!
//! The models are written along `+x`, the direction of a zero heading, `y` up and `z` to the right,
//! their origin on the road under the middle of the car, where the simulation places it. They are
//! 4.2 to 5 m long and 2.2 to 2.8 m wide, about the 2.4 m circle the simulation collides.

use std::f32::consts::TAU;

use bevy::color::LinearRgba;
use bevy::prelude::*;
use star_racer_protocol::CarLook;

use crate::geometry::{Frame, Geometry};

/// The cars, in the order the garage offers them and the wire numbers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CarModel {
    Comet,
    Bolt,
    Pulse,
    Phantom,
    Titan,
    Nova,
}

impl CarModel {
    pub const ALL: [Self; 6] = [
        Self::Comet,
        Self::Bolt,
        Self::Pulse,
        Self::Phantom,
        Self::Titan,
        Self::Nova,
    ];

    /// The car numbered `index` on the wire; an unknown one is the first.
    pub fn of(index: u8) -> Self {
        Self::ALL
            .get(usize::from(index))
            .copied()
            .unwrap_or(Self::Comet)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Comet => "COMET",
            Self::Bolt => "BOLT",
            Self::Pulse => "PULSE",
            Self::Phantom => "PHANTOM",
            Self::Titan => "TITAN",
            Self::Nova => "NOVA",
        }
    }

    /// A line about it, for the garage.
    pub fn blurb(self) -> &'static str {
        match self {
            Self::Comet => "A rocket of a car, narrow, one fin, one big thruster.",
            Self::Bolt => "Broad shoulders, fat wheels and a big wing.",
            Self::Pulse => "Short and round, big wheels at its corners.",
            Self::Phantom => "The lowest of all, twin fins at its tail.",
            Self::Titan => "High on huge wheels, a light bar on its roof.",
            Self::Nova => "An open-wheel racer with two wings.",
        }
    }

    fn design(self) -> &'static Design {
        match self {
            Self::Comet => &COMET,
            Self::Bolt => &BOLT,
            Self::Pulse => &PULSE,
            Self::Phantom => &PHANTOM,
            Self::Titan => &TITAN,
            Self::Nova => &NOVA,
        }
    }
}

/// The paints, in the order the wire numbers them, with their names: a palette of sixteen.
pub const PAINTS: [(&str, Color); 16] = [
    ("ORANGE", Color::srgb(0.95, 0.36, 0.08)),
    ("BLUE", Color::srgb(0.14, 0.36, 0.86)),
    ("CRIMSON", Color::srgb(0.78, 0.07, 0.12)),
    ("LIME", Color::srgb(0.45, 0.80, 0.12)),
    ("GOLD", Color::srgb(0.95, 0.70, 0.12)),
    ("VIOLET", Color::srgb(0.46, 0.18, 0.82)),
    ("TEAL", Color::srgb(0.05, 0.60, 0.60)),
    ("PEARL", Color::srgb(0.85, 0.86, 0.90)),
    ("ROSE", Color::srgb(0.95, 0.30, 0.60)),
    ("CARBON", Color::srgb(0.20, 0.21, 0.24)),
    ("SKY", Color::srgb(0.30, 0.65, 0.95)),
    ("MINT", Color::srgb(0.35, 0.90, 0.65)),
    ("CORAL", Color::srgb(1.00, 0.45, 0.35)),
    ("PLUM", Color::srgb(0.45, 0.12, 0.40)),
    ("SAND", Color::srgb(0.85, 0.72, 0.50)),
    ("JADE", Color::srgb(0.00, 0.42, 0.28)),
];

/// The neon colors, brighter than white so they bloom, with their names: a palette of twelve,
/// which the trails take their colors from too.
pub const NEONS: [(&str, LinearRgba); 12] = [
    ("ORANGE", LinearRgba::new(3.2, 0.95, 0.15, 1.0)),
    ("MAGENTA", LinearRgba::new(2.4, 0.35, 2.6, 1.0)),
    ("CYAN", LinearRgba::new(0.2, 2.4, 3.2, 1.0)),
    ("GREEN", LinearRgba::new(0.4, 3.0, 0.6, 1.0)),
    ("RED", LinearRgba::new(3.2, 0.25, 0.25, 1.0)),
    ("YELLOW", LinearRgba::new(3.0, 2.6, 0.3, 1.0)),
    ("PURPLE", LinearRgba::new(1.6, 0.5, 3.2, 1.0)),
    ("WHITE", LinearRgba::new(2.6, 2.6, 2.8, 1.0)),
    ("ICE", LinearRgba::new(1.2, 2.6, 3.2, 1.0)),
    ("LIME", LinearRgba::new(1.6, 3.2, 0.3, 1.0)),
    ("PINK", LinearRgba::new(3.2, 0.6, 1.6, 1.0)),
    ("AMBER", LinearRgba::new(3.2, 1.6, 0.2, 1.0)),
];

/// How a car is built and painted.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Livery {
    pub model: CarModel,
    /// The body.
    pub body: Color,
    /// The neon lines along the body, the wheels' rings and the light under the car.
    pub accent: LinearRgba,
}

impl Livery {
    /// What a look on the wire draws, anything out of range brought back in.
    pub fn of(look: CarLook) -> Self {
        let look = look.checked();
        Self {
            model: CarModel::of(look.model),
            body: PAINTS[usize::from(look.paint)].1,
            accent: NEONS[usize::from(look.neon)].1,
        }
    }
}

/// The dark parts: the body's lower flanks, the wheels, the wings' undersides.
const GRAPHITE: Color = Color::srgb(0.075, 0.08, 0.095);
/// The underside, which the road never lights.
const UNDERSIDE: Color = Color::srgb(0.025, 0.025, 0.035);
/// The racing stripe.
const STRIPE: Color = Color::srgb(0.88, 0.88, 0.9);
/// The cabin's glass: dark, with a lighter sheen on top where the sky shows in it.
const GLASS: Color = Color::srgb(0.03, 0.05, 0.09);
const GLASS_SHEEN: Color = Color::srgb(0.12, 0.26, 0.38);
/// A hover-wheel's face, inside its neon ring.
const HUB: Color = Color::srgb(0.16, 0.17, 0.2);
/// The inside of a thruster, white hot and a little blue.
const THRUSTER: LinearRgba = LinearRgba::new(2.2, 2.9, 3.6, 1.0);
/// A thruster's flame where it leaves the nozzle, fading to nothing at its tip.
const FLAME: LinearRgba = LinearRgba::new(0.28, 0.5, 1.5, 1.0);

/// A cross-section of a body or a cabin: where it stands along the car, how wide it is either
/// side of its middle, and how high its underside and its top are.
#[derive(Debug, Clone, Copy)]
struct Section {
    x: f32,
    half_width: f32,
    bottom: f32,
    top: f32,
}

const fn section(x: f32, half_width: f32, bottom: f32, top: f32) -> Section {
    Section {
        x,
        half_width,
        bottom,
        top,
    }
}

/// A thruster: where the middle of its mouth is, and its half width (along `z`) and half height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nozzle {
    pub at: Vec3,
    pub half: Vec2,
}

const fn nozzle(x: f32, y: f32, z: f32, half_z: f32, half_y: f32) -> Nozzle {
    Nozzle {
        at: Vec3::new(x, y, z),
        half: Vec2::new(half_z, half_y),
    }
}

/// A pair of wheels, one either side: how far along the car, how far out from its middle their
/// outer faces are, their radius and their width.
#[derive(Debug, Clone, Copy)]
struct Axle {
    x: f32,
    out: f32,
    radius: f32,
    width: f32,
}

const fn axle(x: f32, out: f32, radius: f32, width: f32) -> Axle {
    Axle {
        x,
        out,
        radius,
        width,
    }
}

/// A wing across the car: from `back` to `front` along it, its top at `height`, reaching `span`
/// either side, and whether it stands on two struts from the body and has end plates.
#[derive(Debug, Clone, Copy)]
struct Wing {
    back: f32,
    front: f32,
    height: f32,
    span: f32,
    struts: bool,
    end_plates: bool,
}

/// Everything a car is built from.
struct Design {
    body: &'static [Section],
    cabin: &'static [Section],
    axles: [Axle; 2],
    wings: &'static [Wing],
    /// Fins standing at the tail, `z` out from the middle: (from, to, top) along `x` and up.
    fins: &'static [(f32, f32, f32, f32)],
    nozzles: &'static [Nozzle],
    /// The stripe's runs along the body's top, from and to along `x`.
    stripe: &'static [(f32, f32)],
    /// A bar of neon across the roof, from and to along `x` at this height.
    light_bar: Option<(f32, f32, f32)>,
}

/// Where the lowest point of every wheel is: the road is cleared, the car hovers.
const WHEEL_CLEARANCE: f32 = 0.22;
/// The sides of a wheel's disc: few, so a grid of cars stays cheap to draw.
const WHEEL_SIDES: usize = 8;

// Comet: a rocket of a car, a teardrop seen from above, narrow, its tail tapering to a round
// point round one big thruster, a single fin standing on its spine, small wheels; nothing like
// the Bolt's square shoulders and wing (since v1.0.0-rc4, the user's word: "La voiture comet et
// bolt se ressemblent trop"; it was a low coupé with a duck tail).
const COMET: Design = Design {
    body: &[
        section(-2.25, 0.30, 0.42, 0.66),
        section(-1.98, 0.56, 0.36, 0.74),
        section(-1.40, 0.76, 0.30, 0.78),
        section(-0.30, 0.82, 0.28, 0.74),
        section(0.90, 0.76, 0.28, 0.64),
        section(1.80, 0.52, 0.29, 0.52),
        section(2.40, 0.22, 0.31, 0.42),
        section(2.68, 0.04, 0.33, 0.38),
    ],
    cabin: &[
        section(-0.95, 0.38, 0.74, 0.80),
        section(-0.45, 0.50, 0.73, 1.06),
        section(0.35, 0.48, 0.70, 1.02),
        section(0.95, 0.30, 0.64, 0.70),
    ],
    axles: [axle(1.50, 0.98, 0.30, 0.22), axle(-1.35, 1.02, 0.32, 0.24)],
    wings: &[],
    fins: &[(-2.15, -1.05, 1.16, 0.0)],
    nozzles: &[nozzle(-2.27, 0.55, 0.0, 0.26, 0.14)],
    stripe: &[(-2.1, 2.5)],
    light_bar: None,
};

// Bolt: a muscle car, a long bonnet, broad shoulders, fat rear wheels and a big wing.
const BOLT: Design = Design {
    body: &[
        section(-2.25, 0.92, 0.36, 0.84),
        section(-1.80, 1.12, 0.30, 0.90),
        section(-0.80, 1.06, 0.29, 0.84),
        section(0.60, 1.00, 0.29, 0.82),
        section(1.90, 0.92, 0.30, 0.74),
        section(2.45, 0.56, 0.32, 0.56),
        section(2.65, 0.06, 0.36, 0.44),
    ],
    cabin: &[
        section(-1.75, 0.56, 0.82, 0.88),
        section(-1.30, 0.70, 0.83, 1.12),
        section(-0.35, 0.66, 0.80, 1.10),
        section(0.30, 0.50, 0.76, 0.82),
    ],
    axles: [axle(1.55, 1.10, 0.36, 0.28), axle(-1.40, 1.24, 0.42, 0.40)],
    wings: &[Wing {
        back: -2.24,
        front: -1.80,
        height: 1.16,
        span: 1.15,
        struts: true,
        end_plates: true,
    }],
    fins: &[],
    nozzles: &[
        nozzle(-2.27, 0.58, -0.50, 0.22, 0.12),
        nozzle(-2.27, 0.58, 0.50, 0.22, 0.12),
    ],
    stripe: &[(-2.2, -1.85), (0.4, 2.5)],
    light_bar: None,
};

// Pulse: a compact, short and round, a bubble of a cabin, big wheels at its corners.
const PULSE: Design = Design {
    body: &[
        section(-2.00, 0.80, 0.40, 0.80),
        section(-1.70, 1.02, 0.32, 0.92),
        section(-0.40, 1.04, 0.30, 0.90),
        section(1.10, 0.98, 0.31, 0.80),
        section(1.95, 0.70, 0.34, 0.62),
        section(2.25, 0.08, 0.40, 0.50),
    ],
    cabin: &[
        section(-1.45, 0.56, 0.86, 0.92),
        section(-1.00, 0.76, 0.87, 1.16),
        section(0.25, 0.74, 0.84, 1.15),
        section(1.05, 0.50, 0.74, 0.82),
    ],
    axles: [axle(1.35, 1.16, 0.40, 0.32), axle(-1.30, 1.16, 0.40, 0.32)],
    wings: &[],
    fins: &[],
    nozzles: &[nozzle(-2.02, 0.60, 0.0, 0.42, 0.11)],
    stripe: &[(-1.95, -1.6), (1.15, 2.1)],
    light_bar: None,
};

// Phantom: a hypercar, the lowest of all, its canopy forward, twin fins at the tail.
const PHANTOM: Design = Design {
    body: &[
        section(-2.28, 0.88, 0.32, 0.66),
        section(-1.80, 1.12, 0.27, 0.72),
        section(-0.60, 1.06, 0.26, 0.70),
        section(0.80, 0.92, 0.26, 0.60),
        section(1.90, 0.62, 0.27, 0.46),
        section(2.50, 0.22, 0.29, 0.38),
        section(2.70, 0.03, 0.31, 0.35),
    ],
    cabin: &[
        section(-0.70, 0.42, 0.66, 0.70),
        section(-0.25, 0.56, 0.67, 0.96),
        section(0.65, 0.52, 0.62, 0.92),
        section(1.35, 0.30, 0.52, 0.58),
    ],
    axles: [axle(1.55, 1.12, 0.32, 0.26), axle(-1.45, 1.22, 0.36, 0.34)],
    wings: &[],
    fins: &[(-2.26, -1.25, 1.08, 0.62), (-2.26, -1.25, 1.08, -0.62)],
    nozzles: &[
        nozzle(-2.30, 0.48, -0.55, 0.16, 0.08),
        nozzle(-2.30, 0.48, 0.0, 0.16, 0.08),
        nozzle(-2.30, 0.48, 0.55, 0.16, 0.08),
    ],
    stripe: &[(-2.2, -1.3), (1.0, 2.55)],
    light_bar: None,
};

// Titan: an off-roader, high on huge wheels, a light bar across its roof.
const TITAN: Design = Design {
    body: &[
        section(-2.15, 0.92, 0.52, 0.90),
        section(-1.85, 1.04, 0.46, 0.96),
        section(-0.40, 1.02, 0.45, 0.94),
        section(1.20, 0.98, 0.46, 0.88),
        section(2.00, 0.88, 0.50, 0.80),
        section(2.30, 0.10, 0.56, 0.66),
    ],
    cabin: &[
        section(-1.55, 0.72, 0.90, 0.96),
        section(-1.20, 0.82, 0.91, 1.14),
        section(0.30, 0.80, 0.88, 1.13),
        section(0.95, 0.62, 0.84, 0.92),
    ],
    axles: [axle(1.40, 1.30, 0.48, 0.40), axle(-1.35, 1.30, 0.48, 0.40)],
    wings: &[],
    fins: &[],
    nozzles: &[
        nozzle(-2.17, 0.72, -0.45, 0.20, 0.10),
        nozzle(-2.17, 0.72, 0.45, 0.20, 0.10),
    ],
    stripe: &[(1.25, 2.15)],
    light_bar: Some((-0.15, 0.15, 1.16)),
};

// Nova: an open-wheel racer, a needle of a body, its wheels out on arms, two wings.
const NOVA: Design = Design {
    body: &[
        section(-2.10, 0.42, 0.34, 0.74),
        section(-1.60, 0.56, 0.30, 0.82),
        section(-0.50, 0.52, 0.29, 0.76),
        section(0.80, 0.36, 0.30, 0.62),
        section(1.90, 0.20, 0.32, 0.48),
        section(2.68, 0.03, 0.35, 0.40),
    ],
    cabin: &[
        section(-0.95, 0.20, 0.76, 0.80),
        section(-0.60, 0.30, 0.78, 1.02),
        section(0.05, 0.28, 0.72, 0.96),
        section(0.45, 0.10, 0.62, 0.68),
    ],
    axles: [axle(1.55, 1.18, 0.34, 0.30), axle(-1.35, 1.24, 0.38, 0.38)],
    wings: &[
        Wing {
            back: 2.05,
            front: 2.45,
            height: 0.47,
            span: 1.05,
            struts: false,
            end_plates: true,
        },
        Wing {
            back: -2.25,
            front: -1.80,
            height: 1.12,
            span: 0.95,
            struts: true,
            end_plates: true,
        },
    ],
    fins: &[],
    nozzles: &[nozzle(-2.12, 0.54, 0.0, 0.26, 0.15)],
    stripe: &[(-2.05, -1.65), (0.9, 2.5)],
    light_bar: None,
};

/// Where a car's thrusters are, whose flames are drawn from them.
pub fn nozzles(model: CarModel) -> &'static [Nozzle] {
    model.design().nozzles
}

/// The eight points round a section, in order, as `(z, y)`: the top, the shoulder, the widest
/// point, the lower flank, the keel, and back up the other side. A flat deck, a crisp shoulder
/// line, and the lower flank tucked in under it.
fn ring(section: &Section) -> [Vec2; 8] {
    let Section {
        half_width: w,
        bottom,
        top,
        ..
    } = *section;
    let height = top - bottom;
    let shoulder = Vec2::new(0.84 * w, top - 0.1 * height);
    let widest = Vec2::new(w, bottom + 0.45 * height);
    let flank = Vec2::new(0.72 * w, bottom);
    [
        Vec2::new(0.0, top),
        shoulder,
        widest,
        flank,
        Vec2::new(0.0, bottom - 0.02),
        flank * Vec2::new(-1.0, 1.0),
        widest * Vec2::new(-1.0, 1.0),
        shoulder * Vec2::new(-1.0, 1.0),
    ]
}

/// The colors of the eight faces between the points of [`ring`]: the top two and the upper flanks
/// in `upper`, the lower flanks in `flank`, the underside in `under`.
fn face_colors(upper: Color, flank: Color, under: Color) -> [Color; 8] {
    [upper, upper, flank, under, under, flank, upper, upper]
}

/// Builds a closed body through `sections`, its faces colored by `colors` and its two ends closed
/// in `tail` and `nose`.
fn loft(geometry: &mut Geometry, sections: &[Section], colors: [Color; 8], tail: Color, nose: Color) {
    let point = |section: &Section, at: Vec2| Vec3::new(section.x, at.y, at.x);
    let middle = |section: &Section| Vec3::new(section.x, (section.bottom + section.top) / 2.0, 0.0);
    for pair in sections.windows(2) {
        let (near, far) = (&pair[0], &pair[1]);
        let (near_ring, far_ring) = (ring(near), ring(far));
        let inside = (middle(near) + middle(far)) / 2.0;
        for face in 0..8 {
            let next = (face + 1) % 8;
            geometry.quad_facing_away(
                [
                    point(near, near_ring[face]),
                    point(near, near_ring[next]),
                    point(far, far_ring[next]),
                    point(far, far_ring[face]),
                ],
                inside,
                colors[face],
            );
        }
    }
    // The ends, as fans of triangles round their middle, each turned away from the body.
    for (end, color, inward) in [
        (&sections[0], tail, 1.0),
        (&sections[sections.len() - 1], nose, -1.0),
    ] {
        let center = middle(end);
        let inside = center + Vec3::X * inward;
        let points = ring(end);
        for face in 0..8 {
            let a = point(end, points[face]);
            let b = point(end, points[(face + 1) % 8]);
            geometry.quad_facing_away([center, a, b, b], inside, color);
        }
    }
}

/// The height of the top of `sections` at `x`, along their middle.
fn top_at(sections: &[Section], x: f32) -> f32 {
    let pair = sections
        .windows(2)
        .find(|pair| x >= pair[0].x && x <= pair[1].x)
        .expect("on the body");
    let share = (x - pair[0].x) / (pair[1].x - pair[0].x);
    pair[0].top + (pair[1].top - pair[0].top) * share
}

/// A white stripe down the middle of the body, just over its top, along each of `runs`.
fn stripe(geometry: &mut Geometry, sections: &[Section], runs: &[(f32, f32)]) {
    for &(from, to) in runs {
        let steps = 4;
        for step in 0..steps {
            let a = from + (to - from) * step as f32 / steps as f32;
            let b = from + (to - from) * (step + 1) as f32 / steps as f32;
            let lift = |x: f32| top_at(sections, x) + 0.006;
            geometry.quad_facing_up(
                [
                    Vec3::new(a, lift(a), -0.11),
                    Vec3::new(b, lift(b), -0.11),
                    Vec3::new(b, lift(b), 0.11),
                    Vec3::new(a, lift(a), 0.11),
                ],
                STRIPE,
            );
        }
    }
}

/// The points round a wheel's disc, `z` across the car, as world points of the car.
fn wheel_ring(axle: &Axle, z: f32, radius: f32) -> [Vec3; WHEEL_SIDES] {
    let center_y = WHEEL_CLEARANCE + axle.radius;
    std::array::from_fn(|index| {
        // A flat side at the bottom, so the lowest point is the clearance all along it.
        let angle = (index as f32 + 0.5) / WHEEL_SIDES as f32 * TAU;
        let corner = radius / (std::f32::consts::PI / WHEEL_SIDES as f32).cos();
        Vec3::new(
            axle.x + corner * angle.sin(),
            center_y - corner * angle.cos(),
            z,
        )
    })
}

/// A hover-wheel either side of the car: a graphite disc, its outer face a lighter hub.
fn wheels(geometry: &mut Geometry, axle: &Axle) {
    for side in [-1.0f32, 1.0] {
        let outer = wheel_ring(axle, side * axle.out, axle.radius);
        let inner = wheel_ring(axle, side * (axle.out - axle.width), axle.radius);
        let center = Vec3::new(axle.x, WHEEL_CLEARANCE + axle.radius, side * (axle.out - axle.width / 2.0));
        for index in 0..WHEEL_SIDES {
            let next = (index + 1) % WHEEL_SIDES;
            geometry.quad_facing_away(
                [outer[index], outer[next], inner[next], inner[index]],
                center,
                GRAPHITE,
            );
            let hub = Vec3::new(axle.x, center.y, side * axle.out);
            geometry.quad_facing_away([hub, outer[index], outer[next], outer[next]], center, HUB);
        }
    }
}

/// A wing across the car, on struts or set on the body, with end plates or not.
fn wing(geometry: &mut Geometry, frame: &Frame, wing: &Wing, livery: &Livery, body: &[Section]) {
    let thickness = 0.07;
    geometry.shaded_block(
        frame,
        Vec3::new(wing.back, wing.height - thickness, -wing.span),
        Vec3::new(wing.front, wing.height, wing.span),
        UNDERSIDE,
        livery.body,
    );
    if wing.struts {
        for z in [-0.45f32, 0.45] {
            let x = (wing.back + wing.front) / 2.0;
            let root = top_at(body, x.clamp(body[0].x, body[body.len() - 1].x)) - 0.05;
            geometry.tapered_block(
                frame,
                Vec3::new(x - 0.12, root, z - 0.035),
                Vec3::new(x + 0.12, wing.height - thickness, z + 0.035),
                0.8,
                GRAPHITE,
            );
        }
    }
    if wing.end_plates {
        for side in [-1.0f32, 1.0] {
            geometry.block(
                frame,
                Vec3::new(wing.back - 0.03, wing.height - 0.24, side * wing.span - 0.03),
                Vec3::new(wing.front + 0.03, wing.height + 0.04, side * wing.span + 0.03),
                GRAPHITE,
            );
        }
    }
}

/// Everything of a car that takes the light: body, cabin, wheels, wings and fins.
pub fn hull_geometry(livery: &Livery) -> Geometry {
    let design = livery.model.design();
    let mut geometry = Geometry::default();
    let frame = Frame::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    let g = &mut geometry;
    loft(
        g,
        design.body,
        face_colors(livery.body, GRAPHITE, UNDERSIDE),
        GRAPHITE,
        livery.body,
    );
    loft(g, design.cabin, face_colors(GLASS_SHEEN, GLASS, GLASS), GLASS, GLASS);
    for axle in &design.axles {
        wheels(g, axle);
        if livery.model == CarModel::Nova {
            // The arm out to each wheel.
            for side in [-1.0f32, 1.0] {
                g.block(
                    &frame,
                    Vec3::new(axle.x - 0.06, 0.5, side * 0.3),
                    Vec3::new(axle.x + 0.06, 0.56, side * (axle.out - axle.width)),
                    GRAPHITE,
                );
            }
        }
    }
    for wing_design in design.wings {
        wing(g, &frame, wing_design, livery, design.body);
    }
    for &(from, to, top, z) in design.fins {
        let root = top_at(design.body, (from + to) / 2.0) - 0.04;
        g.tapered_block(
            &frame,
            Vec3::new(from, root, z - 0.035),
            Vec3::new(to, top, z + 0.035),
            0.35,
            livery.body,
        );
    }
    stripe(g, design.body, design.stripe);
    geometry
}

/// A thruster's glowing mouth, just behind the tail it sits in.
fn nozzle_light(geometry: &mut Geometry, frame: &Frame, nozzle: &Nozzle) {
    let Nozzle { at, half } = *nozzle;
    geometry.block(
        frame,
        Vec3::new(at.x - 0.02, at.y - half.y, at.z - half.x),
        Vec3::new(at.x + 0.03, at.y + half.y, at.z + half.x),
        Color::LinearRgba(THRUSTER),
    );
}

/// A neon line through `points`.
fn neon(geometry: &mut Geometry, frame: &Frame, points: &[Vec3], width: f32, color: Color) {
    for pair in points.windows(2) {
        geometry.bar(frame, pair[0], pair[1], width, color);
    }
}

/// The line along the widest point of a body, on one side, from its tail toward its nose, leaving
/// out its last `skip` sections.
fn widest_line(sections: &[Section], side: f32, skip: usize) -> Vec<Vec3> {
    sections[..sections.len() - skip]
        .iter()
        .map(|section| {
            let widest = ring(section)[2];
            Vec3::new(section.x, widest.y, side * (widest.x + 0.01))
        })
        .collect()
}

/// The parts that shine by themselves: the thrusters' insides, the neon line along each flank, the
/// ring round each wheel and the light bar, drawn unlit and brighter than white.
pub fn lights_geometry(livery: &Livery) -> Geometry {
    let design = livery.model.design();
    let mut geometry = Geometry::default();
    let frame = Frame::new(Vec3::ZERO, Vec3::X, Vec3::Z);
    let accent = Color::LinearRgba(livery.accent);
    let g = &mut geometry;
    for nozzle in design.nozzles {
        nozzle_light(g, &frame, nozzle);
    }
    for side in [-1.0f32, 1.0] {
        neon(g, &frame, &widest_line(design.body, side, 1), 0.045, accent);
        // A ring of neon round each wheel's hub, on its outer face.
        for axle in &design.axles {
            let z = side * (axle.out + 0.012);
            let outer = wheel_ring(axle, z, axle.radius * 0.86);
            let inner = wheel_ring(axle, z, axle.radius * 0.68);
            let center = Vec3::new(axle.x, WHEEL_CLEARANCE + axle.radius, 0.0);
            for index in 0..WHEEL_SIDES {
                let next = (index + 1) % WHEEL_SIDES;
                g.quad_facing_away(
                    [outer[index], outer[next], inner[next], inner[index]],
                    center,
                    accent,
                );
            }
        }
    }
    if let Some((from, to, height)) = design.light_bar {
        g.block(
            &frame,
            Vec3::new(from, height - 0.03, -0.62),
            Vec3::new(to, height + 0.03, 0.62),
            accent,
        );
    }
    geometry
}

/// A car's outline on the road, as an oval: half its length and half its width, about the same for
/// every car.
/// The light and the shadow under the car are drawn in multiples of it.
const FOOTPRINT: Vec2 = Vec2::new(2.25, 1.35);

/// Around the oval the light and the shadow are drawn as, in this many steps: enough that its
/// rim never shows a corner.
const OVAL_STEPS: usize = 24;

/// Concentric ovals on the road under the car, from its middle outward: each ring is a size, in
/// multiples of [`FOOTPRINT`], and the color everything at that size takes, blended between rings.
/// A first ring of size 0 is the middle itself. The road may curve away under a flat shape on a
/// banked turn, so whatever is drawn this way fades to nothing at its rim, where it would sink.
fn oval_rings(height: f32, rings: &[(f32, Color)]) -> Geometry {
    let mut geometry = Geometry::default();
    let point = |size: f32, step: usize| {
        let angle = step as f32 / OVAL_STEPS as f32 * std::f32::consts::TAU;
        let at = FOOTPRINT * size * Vec2::new(angle.cos(), angle.sin());
        Vec3::new(at.x, height, at.y)
    };
    for pair in rings.windows(2) {
        let ((inner, inner_color), (outer, outer_color)) = (pair[0], pair[1]);
        for step in 0..OVAL_STEPS {
            geometry.quad_shaded(
                [
                    point(inner, step),
                    point(inner, step + 1),
                    point(outer, step + 1),
                    point(outer, step),
                ],
                Vec3::Y,
                [inner_color, inner_color, outer_color, outer_color],
            );
        }
    }
    geometry
}

/// The light the car throws on the road under it, added to whatever it falls on. It spills out
/// from under the hull's edge: none in the middle, which the hull hides and the shadow darkens,
/// most of it just outside the hull's outline, and none a meter and a half beyond.
///
/// It used to be a rectangle as wide as the car and a half, evenly lit to its middle: from the
/// chase camera a slab of muddy orange, wider than the car, whose straight edge cut across a
/// banked road.
pub fn under_glow_geometry(livery: &Livery) -> Geometry {
    let glow = |share: f32| {
        Color::LinearRgba(LinearRgba::new(
            livery.accent.red * share,
            livery.accent.green * share,
            livery.accent.blue * share,
            1.0,
        ))
    };
    oval_rings(
        0.11,
        &[
            (0.7, glow(0.0)),
            (1.0, glow(GLOW)),
            (1.25, glow(GLOW * 0.4)),
            (1.6, glow(0.0)),
        ],
    )
}

/// How much of a livery's neon the light under the car is at its brightest, just outside the hull.
const GLOW: f32 = 0.1;

/// The shadow under the car: a dark oval, darkest under the middle of the hull and fading out by
/// the hull's edge, laid over the road with its own transparency (see `race.rs`). The moon casts
/// a shadow of the car too, but it is nearly overhead and dim against the night's ambient, so the
/// car hovered over nothing; this is what sits it over the road, the way a hovercraft's air
/// cushion would darken what it presses on.
pub fn shadow_geometry() -> Geometry {
    let shade = |alpha: f32| Color::LinearRgba(LinearRgba::new(0.0, 0.0, 0.0, alpha));
    oval_rings(
        0.09,
        &[
            (0.0, shade(SHADOW)),
            (0.55, shade(SHADOW)),
            (0.9, shade(SHADOW * 0.55)),
            (1.15, shade(0.0)),
        ],
    )
}

/// How dark the shadow is at its darkest, from 0 to 1.
const SHADOW: f32 = 0.7;

/// One thruster's flame, one meter long and pointing back from the origin, its mouth a unit square
/// either side of it: the entity it hangs from stands at the nozzle, scales it across to the
/// nozzle's size and along `x` to the length the speed calls for (see [`flame_scale`]). Its color
/// fades from the mouth to nothing at the tip, and it is added to what is behind it.
pub fn flame_geometry() -> Geometry {
    let mut geometry = Geometry::default();
    let base = Color::LinearRgba(FLAME);
    let tip_color = Color::LinearRgba(LinearRgba::BLACK);
    let tip = -Vec3::X;
    let corners = [
        Vec3::new(0.0, -1.0, -1.0),
        Vec3::new(0.0, -1.0, 1.0),
        Vec3::new(0.0, 1.0, 1.0),
        Vec3::new(0.0, 1.0, -1.0),
    ];
    let inside = -Vec3::X * 0.3;
    for face in 0..4 {
        let a = corners[face];
        let b = corners[(face + 1) % 4];
        geometry.quad_facing_away_shaded(
            [a, b, tip, tip],
            inside,
            [base, base, tip_color, tip_color],
        );
    }
    geometry
}

/// Where a nozzle's flame hangs from: just behind its mouth.
pub fn flame_origin(nozzle: &Nozzle) -> Vec3 {
    nozzle.at - Vec3::X * 0.02
}

/// How a nozzle's flame is scaled across: a little inside its mouth.
pub fn flame_scale(nozzle: &Nozzle, length: f32) -> Vec3 {
    Vec3::new(length, nozzle.half.y * 0.9, nozzle.half.x * 0.9)
}

/// How long a car's flames are, in meters: a short blue tongue at rest, longer with the speed, and
/// roaring out on a boost. `flicker` is from -1 to 1, a different one every frame.
pub fn flame_length(speed_share: f32, boosting: bool, flicker: f32) -> f32 {
    let steady = 0.3 + 0.6 * speed_share.clamp(0.0, 1.3);
    let roar = if boosting { 0.85 } else { 0.0 };
    (steady + roar) * (1.0 + 0.08 * flicker.clamp(-1.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(geometry: &Geometry) -> (Vec3, Vec3) {
        geometry.positions().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(min, max), point| (min.min(point), max.max(point)),
        )
    }

    fn livery(model: CarModel) -> Livery {
        Livery {
            model,
            body: PAINTS[0].1,
            accent: NEONS[1].1,
        }
    }

    /// Every car is about as big as what the simulation collides, it hovers clear of the road, and
    /// its nose points forward.
    #[test]
    fn every_car_is_the_size_of_its_collisions_and_hovers() {
        for model in CarModel::ALL {
            let hull = hull_geometry(&livery(model));
            let (min, max) = bounds(&hull);
            assert!(max.x > 2.1 && max.x < 2.75, "{model:?} {max}");
            assert!(min.x > -2.3, "{model:?} {min}");
            assert!(max.z < 1.65 && min.z > -1.65, "{model:?} {min} {max}");
            assert!(
                min.y > 0.2,
                "{model:?}: the underside clears the road: {min}"
            );
            assert!(max.y < 1.2, "{model:?} {max}");
            // The tip of the nose is on the middle of the car.
            let tip = hull.positions().max_by(|a, b| a.x.total_cmp(&b.x)).unwrap();
            assert!(tip.z.abs() < 0.1, "{model:?} {tip}");
        }
    }

    /// Every car's thrusters are at the back, and the flames hang behind them.
    /// the flames hang behind them.
    #[test]
    fn the_thrusters_are_at_the_tail() {
        for model in CarModel::ALL {
            let (min, _) = bounds(&hull_geometry(&livery(model)));
            for nozzle in nozzles(model) {
                assert!(nozzle.at.x - min.x < 0.8, "{model:?} {nozzle:?} {min}");
                assert!(flame_origin(nozzle).x < nozzle.at.x);
            }
        }
    }

    #[test]
    fn every_car_stays_light_to_draw() {
        for model in CarModel::ALL {
            let livery = livery(model);
            let triangles = hull_geometry(&livery).triangles()
                + lights_geometry(&livery).triangles()
                + under_glow_geometry(&livery).triangles()
                + shadow_geometry().triangles()
                + flame_geometry().triangles() * nozzles(model).len();
            // A few thousand for a full grid of eight, against the 155,000 of the skyway and its
            // city.
            assert!(triangles < 1200, "{model:?} {triangles}");
        }
    }

    /// The neon and the thrusters are brighter than white, so they bloom; the hull is not, whatever
    /// its paint.
    #[test]
    fn the_lights_glow_and_the_hull_does_not() {
        let brightest = |geometry: Geometry| {
            geometry
                .colors()
                .map(|[r, g, b, _]| r.max(g).max(b))
                .fold(0.0f32, f32::max)
        };
        for model in CarModel::ALL {
            for paint in 0..PAINTS.len() {
                let livery = Livery::of(CarLook {
                    model: model as u8,
                    paint: paint as u8,
                    neon: 0,
                    trail: 1,
                    trail_color: 2,
                    horn: 0,
                });
                assert!(brightest(hull_geometry(&livery)) <= 1.0);
            }
            for neon in 0..NEONS.len() {
                let livery = Livery::of(CarLook {
                    model: model as u8,
                    paint: 0,
                    neon: neon as u8,
                    trail: 1,
                    trail_color: 2,
                    horn: 0,
                });
                assert!(
                    brightest(lights_geometry(&livery)) > 2.0,
                    "{model:?} {neon}"
                );
            }
        }
    }

    /// The lists match what the wire allows, and every look on it draws.
    #[test]
    fn every_look_on_the_wire_draws() {
        use star_racer_protocol::{CAR_MODELS, CAR_NEONS, CAR_PAINTS};
        assert_eq!(CarModel::ALL.len(), usize::from(CAR_MODELS));
        assert_eq!(PAINTS.len(), usize::from(CAR_PAINTS));
        assert_eq!(NEONS.len(), usize::from(CAR_NEONS));
        for (index, model) in CarModel::ALL.into_iter().enumerate() {
            assert_eq!(CarModel::of(index as u8), model);
        }
        let wild = Livery::of(CarLook {
            model: 200,
            paint: 200,
            neon: 200,
            trail: 1,
            trail_color: 2,
            horn: 0,
        });
        assert!(CarModel::ALL.contains(&wild.model));
    }

    /// The light and the shadow under the car are ovals around it, fading to nothing at their rim,
    /// where a banked road could curve up through them; the light is brightest at the hull's edge,
    /// not in its middle, and is never a lit sheet that blooms.
    #[test]
    fn the_light_and_the_shadow_under_the_car_fade_out_at_their_rims() {
        let reach = |geometry: &Geometry| {
            geometry
                .positions()
                .map(|point| (point.x / FOOTPRINT.x).hypot(point.z / FOOTPRINT.y))
                .fold(0.0f32, f32::max)
        };
        let glow = under_glow_geometry(&livery(CarModel::Comet));
        let shadow = shadow_geometry();
        // Round: every point of the rim is as far out, in the oval's own measure.
        for geometry in [&glow, &shadow] {
            let rim = reach(geometry);
            let colors: Vec<_> = geometry.colors().collect();
            for (point, color) in geometry.positions().zip(colors) {
                let out = (point.x / FOOTPRINT.x).hypot(point.z / FOOTPRINT.y);
                if (out - rim).abs() < 1e-3 {
                    assert!(
                        color
                            .iter()
                            .all(|&channel| channel == 0.0 || channel == 1.0)
                    );
                    assert!(color[3] == 0.0 || color[..3] == [0.0; 3], "{color:?}");
                }
            }
        }
        assert!(reach(&glow) < 1.7 && reach(&shadow) < 1.2);
        // Brightest at the hull's edge, dim in the middle, and nowhere near blooming.
        let brightness_at = |size: f32| {
            glow.positions()
                .zip(glow.colors())
                .filter(|(point, _)| {
                    ((point.x / FOOTPRINT.x).hypot(point.z / FOOTPRINT.y) - size).abs() < 1e-3
                })
                .map(|(_, [r, g, b, _])| r.max(g).max(b))
                .fold(0.0f32, f32::max)
        };
        assert_eq!(brightness_at(0.7), 0.0);
        assert!(brightness_at(1.0) < 0.5);
    }

    #[test]
    fn the_flames_grow_with_the_speed_and_roar_on_a_boost() {
        let resting = flame_length(0.0, false, 0.0);
        let cruising = flame_length(1.0, false, 0.0);
        let boosting = flame_length(1.0, true, 0.0);
        assert!(resting > 0.2 && resting < cruising && cruising < boosting);
        assert!(boosting > 1.8 * cruising);
        assert!((flame_length(1.0, false, 1.0) / cruising - 1.08).abs() < 1e-4);
    }
}
