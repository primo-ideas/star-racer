//! Rings of neon under a car leaving the road (since v1.0.0-rc1, the user's word "quand on jump, il
//! faut un effet sympa sous la voiture [...] pour le vol, il faut un effet aussi, pas trop éloigné
//! en terme de style du jump").
//!
//! A hop throws a ring out across the road from under the car, with a second, smaller one just
//! after it, both fading as they widen. The thruster, while it burns, sheds smaller rings under
//! the hull that widen as they drop away from the car, a pulse every [`PULSE_SECONDS`], and its
//! ignition a ring as large as a hop's. All of them in the car's neon, unlit and added to what is
//! behind them, like the trails; the simulation never sees them.

use std::collections::HashMap;

use bevy::color::LinearRgba;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use star_racer_protocol::{CarId, CarLook};
use star_racer_sim::car::Car;

use crate::camera::MainCamera;
use crate::car_model::NEONS;
use crate::lobby::CurrentLobby;
use crate::race::{RaceUpdate, RaceView};
use crate::screen::Screen;
use crate::track::CurrentTrack;
use crate::world;

/// Seconds between two rings shed by a burning thruster.
const PULSE_SECONDS: f32 = 0.09;

/// A ring's shape and life: how wide it starts and ends, in meters, how long it lasts, in
/// seconds, how fast it drops, in m/s, and how bright it is at first.
#[derive(Debug, Clone, Copy)]
struct Kind {
    from: f32,
    to: f32,
    life: f32,
    drop: f32,
    bright: f32,
}

/// The ring a hop throws out over the road.
const HOP: Kind = Kind {
    from: 0.9,
    to: 5.0,
    life: 0.45,
    drop: 0.0,
    bright: 2.0,
};
/// The smaller ring that follows it.
const HOP_ECHO: Kind = Kind {
    from: 0.5,
    to: 3.0,
    life: 0.35,
    drop: 0.0,
    bright: 1.3,
};
/// What the thruster sheds while it burns.
const PULSE: Kind = Kind {
    from: 0.7,
    to: 2.4,
    life: 0.32,
    drop: 7.0,
    bright: 0.9,
};
/// The thruster's ignition.
const IGNITION: Kind = Kind {
    from: 0.9,
    to: 4.2,
    life: 0.4,
    drop: 3.0,
    bright: 1.6,
};

pub struct BurstPlugin;

impl Plugin for BurstPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_ring)
            .add_systems(OnEnter(Screen::Lobby), warm_up)
            .add_systems(OnExit(Screen::Lobby), forget_rings)
            .add_systems(
                Update,
                (shed_rings, grow_rings)
                    .chain()
                    .after(RaceUpdate)
                    .run_if(in_state(Screen::Lobby)),
            );
    }
}

/// The ring every burst is drawn with, lying flat, one meter across at its outer edge.
#[derive(Resource)]
struct RingMesh(Handle<Mesh>);

fn create_ring(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mesh = Annulus::new(0.86, 1.0).mesh().resolution(40).build();
    commands.insert_resource(RingMesh(meshes.add(mesh)));
}

/// A ring on its way out.
#[derive(Component)]
struct Ring {
    kind: Kind,
    born: f32,
    at: Vec3,
    /// The road's up where it was shed, which it drops along, and the plane it lies in.
    up: Vec3,
    color: LinearRgba,
    /// Seconds before it shows, for the echo of a hop.
    delay: f32,
    material: Handle<StandardMaterial>,
}

/// What each car was doing the last frame, to see it leave the road or light its thruster.
#[derive(Default)]
struct Seen {
    airborne: bool,
    thrusting: bool,
    last_pulse: f32,
}

/// A ring drawn black, which adds nothing, just in front of the camera for as long as the player
/// is in a lobby: its pipelines are made on entering it rather than at the first hop, which they
/// would miss, a ring living less than the half second they take (the drift arc's reason too;
/// docs/web.md#shaders-compiled-mid-race).
#[derive(Component)]
struct WarmRing;

fn warm_up(
    mut commands: Commands,
    mesh: Res<RingMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    cameras: Query<Entity, With<MainCamera>>,
) {
    let material = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        unlit: true,
        alpha_mode: AlphaMode::Add,
        cull_mode: None,
        ..default()
    });
    for camera in &cameras {
        commands.entity(camera).with_child((
            WarmRing,
            DespawnOnExit(Screen::Lobby),
            Mesh3d(mesh.0.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, 0.0, -2.0).with_scale(Vec3::splat(0.05)),
            NotShadowCaster,
        ));
    }
}

fn forget_rings(mut commands: Commands, rings: Query<Entity, With<Ring>>) {
    for ring in &rings {
        commands.entity(ring).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn shed_rings(
    mut commands: Commands,
    view: Res<RaceView>,
    track: Option<Res<CurrentTrack>>,
    current: Res<CurrentLobby>,
    time: Res<Time>,
    mesh: Res<RingMesh>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut seen: Local<HashMap<CarId, Seen>>,
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
    seen.retain(|id, _| view.cars.iter().any(|car| car.id == *id));
    for snapshot in &view.cars {
        let car = &snapshot.car;
        let was = seen.entry(snapshot.id).or_default();
        let airborne = car.is_airborne() && !car.is_falling();
        let mut shed = Vec::new();
        // Leaving the road upward: a hop, or the thruster lifting the car off.
        if airborne && !was.airborne && car.climb > 1.0 {
            if car.thrusting {
                shed.push((IGNITION, 0.0));
            } else {
                shed.push((HOP, 0.0));
                shed.push((HOP_ECHO, 0.07));
            }
        } else if car.thrusting && !was.thrusting {
            shed.push((IGNITION, 0.0));
            was.last_pulse = now;
        }
        if car.thrusting && now - was.last_pulse >= PULSE_SECONDS {
            shed.push((PULSE, 0.0));
            was.last_pulse = now;
        }
        was.airborne = airborne;
        was.thrusting = car.thrusting;
        if shed.is_empty() {
            continue;
        }
        let (at, up) = under(car, &track.0);
        let color = NEONS[usize::from(look_of(snapshot.id).neon)].1;
        for (kind, delay) in shed {
            let material = materials.add(StandardMaterial {
                base_color: Color::BLACK,
                unlit: true,
                alpha_mode: AlphaMode::Add,
                cull_mode: None,
                ..default()
            });
            commands.spawn((
                Ring {
                    kind,
                    born: now,
                    at,
                    up,
                    color,
                    delay,
                    material: material.clone(),
                },
                Mesh3d(mesh.0.clone()),
                MeshMaterial3d(material),
                Transform::from_translation(at).with_scale(Vec3::ZERO),
                NotShadowCaster,
            ));
        }
    }
}

/// Where a car's rings start: on the road under it, or just under its hull in the air, and the
/// road's up there.
fn under(car: &Car, track: &star_racer_sim::track::Track) -> (Vec3, Vec3) {
    let surface = track.surface(car.position);
    let up = world::rotation_on_surface(car.heading, surface.gradient) * Vec3::Y;
    let at = world::position(car.position, surface.height) + up * (car.altitude.max(0.0) + 0.08);
    (at, up)
}

fn grow_rings(
    mut commands: Commands,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut rings: Query<(Entity, &Ring, &mut Transform)>,
) {
    let now = time.elapsed_secs();
    for (entity, ring, mut transform) in &mut rings {
        let age = now - ring.born - ring.delay;
        let share = age / ring.kind.life;
        if share >= 1.0 {
            materials.remove(&ring.material);
            commands.entity(entity).despawn();
            continue;
        }
        if share < 0.0 {
            continue;
        }
        // Out fast, then slowing: an explosion's front.
        let out = 1.0 - (1.0 - share).powi(3);
        let radius = ring.kind.from + (ring.kind.to - ring.kind.from) * out;
        transform.translation = ring.at - ring.up * ring.kind.drop * age;
        transform.rotation = Quat::from_rotation_arc(Vec3::Z, ring.up);
        transform.scale = Vec3::splat(radius);
        let fade = (1.0 - share).powi(2) * ring.kind.bright;
        if let Some(mut material) = materials.get_mut(&ring.material) {
            material.base_color = Color::from(ring.color * fade);
        }
    }
}
