//! A spectator's cameras (since v0.1.71): whoever watches a race, having come in only to watch it
//! or joined it too late to drive, chooses how to see it.
//!
//! - **CHASE**: behind the car watched, as a driver sees their own (`camera.rs`); left and right
//!   change car, in race order.
//! - **ORBIT**: round the car watched, turning slowly, close and low: the car shown off.
//! - **TOP**: high over the car watched, looking down on it and the road around it: the race
//!   played out from above.
//! - **FREE**: flown anywhere: the left stick or W, A, S, D move, the right stick or the arrows
//!   look, the triggers or Space and Shift climb and sink.
//!
//! V, or triangle / Y on a gamepad, goes to the next. `--camera-cycle <seconds>` changes camera and
//! car on its own, for unattended runs and films.

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use star_racer_sim::car::Car;

use crate::camera::{FollowCar, MainCamera};
use crate::race::{RaceUpdate, RaceView, Spectated};
use crate::screen::Screen;
use crate::settings::Script;
use crate::world;

/// How a spectator sees the race.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraMode {
    #[default]
    Chase,
    Orbit,
    Top,
    Free,
}

impl CameraMode {
    const ALL: [Self; 4] = [Self::Chase, Self::Orbit, Self::Top, Self::Free];

    pub fn name(self) -> &'static str {
        match self {
            Self::Chase => "CHASE",
            Self::Orbit => "ORBIT",
            Self::Top => "TOP",
            Self::Free => "FREE",
        }
    }

    fn next(self) -> Self {
        let index = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

/// The spectator's camera, and where the free one is.
#[derive(Resource, Default)]
pub struct SpectatorCamera {
    pub mode: CameraMode,
    /// The free camera: where it is, which way it looks round and how far up or down, in radians.
    free: Option<(Vec3, f32, f32)>,
    /// Where the top camera is, easing after the car.
    top: Option<Vec3>,
    /// How far round the orbit has turned, in radians.
    orbit: f32,
    /// When the camera last changed on its own, with `--camera-cycle`.
    cycled: f64,
}

/// How far from the car the orbit turns, how high, and how fast, in meters and radians per second.
const ORBIT_DISTANCE: f32 = 9.0;
const ORBIT_HEIGHT: f32 = 2.6;
const ORBIT_RATE: f32 = 0.35;
/// How high over the car the top camera stands, and how far behind it, in meters.
const TOP_HEIGHT: f32 = 70.0;
const TOP_BEHIND: f32 = 18.0;
/// How fast the free camera flies and turns, in meters and radians per second.
const FREE_SPEED: f32 = 45.0;
const FREE_TURN: f32 = 1.6;
/// How far a mouse moved by one pixel turns the free camera, in radians.
const MOUSE_TURN: f32 = 0.004;

pub struct SpectatorPlugin;

impl Plugin for SpectatorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpectatorCamera>()
            .add_systems(
                OnEnter(Screen::Lobby),
                |mut camera: ResMut<SpectatorCamera>, script: Res<Script>| {
                    camera.free = None;
                    camera.top = None;
                    if let Some(wanted) = &script.camera {
                        camera.mode = CameraMode::ALL
                            .into_iter()
                            .find(|mode| mode.name().eq_ignore_ascii_case(wanted))
                            .unwrap_or_default();
                    }
                },
            )
            .add_systems(
                Update,
                (switch_camera, place_camera)
                    .chain()
                    .after(RaceUpdate)
                    .after(FollowCar)
                    .run_if(in_state(Screen::Lobby)),
            );
    }
}

/// Whether the player is watching rather than driving.
fn watching(view: &RaceView) -> bool {
    view.own_car.is_none()
}

/// V or triangle goes to the next camera; `--camera-cycle` does it on its own, changing car too.
fn switch_camera(
    view: Res<RaceView>,
    script: Res<Script>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time<Real>>,
    mut camera: ResMut<SpectatorCamera>,
    mut spectated: ResMut<Spectated>,
) {
    if !watching(&view) {
        return;
    }
    let pressed = keyboard.just_pressed(KeyCode::KeyV)
        || gamepads
            .iter()
            .any(|pad| pad.just_pressed(GamepadButton::North));
    if pressed {
        camera.mode = camera.mode.next();
    }
    if let Some(every) = script.camera_cycle {
        let now = time.elapsed_secs_f64();
        if now - camera.cycled >= f64::from(every) {
            camera.cycled = now;
            camera.mode = match camera.mode {
                // FREE needs hands: the cycle goes round the others.
                CameraMode::Top => CameraMode::Chase,
                mode => mode.next(),
            };
            if camera.mode == CameraMode::Chase {
                spectated.step(&view, 1);
            }
        }
    }
}

/// Places the camera as the spectator's mode says, after the chase camera has placed it.
fn place_camera(
    view: Res<RaceView>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    motion: Res<AccumulatedMouseMotion>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut camera: ResMut<SpectatorCamera>,
    mut cameras: Query<&mut Transform, With<MainCamera>>,
) {
    if !watching(&view) || camera.mode == CameraMode::Chase {
        return;
    }
    let Ok(mut transform) = cameras.single_mut() else {
        return;
    };
    let seconds = time.delta_secs();
    let watched = view.followed.map(|(_, car)| car);
    let at = |car: &Car| world::position(car.position, view.followed_height + car.altitude.max(0.0));
    match camera.mode {
        CameraMode::Chase => {}
        CameraMode::Orbit => {
            let Some(car) = watched else {
                return;
            };
            camera.orbit += ORBIT_RATE * seconds;
            let center = at(&car) + Vec3::Y * 0.8;
            let round = Vec3::new(camera.orbit.cos(), 0.0, camera.orbit.sin()) * ORBIT_DISTANCE;
            transform.translation = center + round + Vec3::Y * ORBIT_HEIGHT;
            transform.look_at(center, Vec3::Y);
        }
        CameraMode::Top => {
            let Some(car) = watched else {
                return;
            };
            let center = at(&car);
            let back = world::direction(Vec2::from_angle(car.heading)) * TOP_BEHIND;
            let wanted = center - back + Vec3::Y * TOP_HEIGHT;
            // Eased toward where it should be, so the view drifts with the car rather than jerks.
            let ease = 1.0 - (-4.0 * seconds).exp();
            let position = camera.top.map_or(wanted, |top| top.lerp(wanted, ease));
            camera.top = Some(position);
            transform.translation = position;
            transform.look_at(center, Vec3::Y);
        }
        CameraMode::Free => {
            let (mut position, mut yaw, mut pitch) = camera.free.unwrap_or_else(|| {
                // From wherever the camera was, looking the same way.
                let forward = transform.forward();
                (
                    transform.translation,
                    forward.z.atan2(forward.x),
                    forward.y.asin(),
                )
            });
            let key = |code: KeyCode| f32::from(u8::from(keyboard.pressed(code)));
            let mut walk = Vec2::new(key(KeyCode::KeyD) - key(KeyCode::KeyA), key(KeyCode::KeyW) - key(KeyCode::KeyS));
            let mut look = Vec2::new(
                key(KeyCode::ArrowRight) - key(KeyCode::ArrowLeft),
                key(KeyCode::ArrowUp) - key(KeyCode::ArrowDown),
            );
            let mut climb = key(KeyCode::Space) - key(KeyCode::ShiftLeft);
            for pad in &gamepads {
                walk += pad.left_stick();
                look += pad.right_stick();
                climb += pad.get(GamepadButton::RightTrigger2).unwrap_or(0.0)
                    - pad.get(GamepadButton::LeftTrigger2).unwrap_or(0.0);
            }
            yaw += look.x * FREE_TURN * seconds;
            pitch += look.y * FREE_TURN * seconds;
            if mouse.pressed(MouseButton::Right) {
                yaw += motion.delta.x * MOUSE_TURN;
                pitch -= motion.delta.y * MOUSE_TURN;
            }
            pitch = pitch.clamp(-1.45, 1.45);
            let forward = Vec3::new(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos());
            let flat = Vec3::new(yaw.cos(), 0.0, yaw.sin());
            let right = flat.cross(Vec3::Y);
            position += (forward * walk.y + right * walk.x + Vec3::Y * climb) * FREE_SPEED * seconds;
            transform.translation = position;
            transform.look_to(forward, Vec3::Y);
            camera.free = Some((position, yaw, pitch));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cameras_go_round() {
        let mut mode = CameraMode::Chase;
        for _ in 0..CameraMode::ALL.len() {
            mode = mode.next();
        }
        assert_eq!(mode, CameraMode::Chase);
    }
}
