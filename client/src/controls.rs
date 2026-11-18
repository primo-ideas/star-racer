//! What the player presses, sent to the server whenever it changes.
//!
//! There is no brake: the game is full speed all the time.
//!
//! Which key and which button does what is the player's to say, in the settings screen (see
//! `bindings.rs`, which also holds the defaults). Gamepad, by default: the right bumper (R, above
//! the trigger) accelerates, the left stick or D-pad steers, the right trigger drifts, Y (the top
//! face button) fires the turbo. Keyboard, by default: W accelerates, A and D steer, Shift drifts,
//! E fires the turbo. Keys are read by their position on the keyboard, not by the letter printed
//! on them, so other layouts get the same physical keys. On a touchscreen, two thumbs on the
//! controls drawn over the race do all of it (see `ui/touch.rs`).
//!
//! Every device is read at once and they are added together, so a gamepad, a keyboard and a
//! screen can be used one after another, or at the same time, without anything being chosen.
//!
//! Each device's steering is shaped for what it is (docs/simulation.md#steering-from-each-device):
//!
//! - **A stick** has a dead zone of its own, so a worn stick resting a little off center does not
//!   steer, and reaches full lock a little short of its rim, which a stick pushed at an angle
//!   never reaches.
//! - **Keys and the D-pad** say nothing between straight and full lock, so they ease in: half lock
//!   on the press, full lock 0.2 s later. A tap is a correction and a held key a turn, where every
//!   tap was a full turn of the wheel. With both directions held, the last one pressed steers.
//! - **Thumbs on a screen** already say how hard, by where they rest (see `ui/touch.rs`).
//!
//! Every action has a second key: by default the arrow keys and Space do what W, A, D and Shift
//! do, and the down arrow what E does. A gamepad rumbles when its car hits a wall, breaks into a
//! drift or fires a boost, harder for the turbo, as a phone vibrates (see `ui/touch.rs`), unless
//! the player turned vibration off.

use std::time::Duration;

use bevy::input::gamepad::{GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::platform::time::Instant;
use bevy::prelude::*;
use star_racer_protocol::{ClientMessage, LobbyPhase};
use star_racer_sim::autopilot;
use star_racer_sim::car::{CarInput, Reset};

use crate::bindings::{Action, Bindings};
use crate::latency::Latency;
use crate::lobby::CurrentLobby;
use crate::net::Network;
use crate::prediction::Prediction;
use crate::preferences::StoredPreferences;
use crate::race::{RaceUpdate, RaceView};
use crate::screen::Screen;
use crate::settings::Script;
use crate::track::CurrentTrack;
use crate::ui::hud::LeaveDialog;
use crate::ui::touch::{TouchControls, TouchUpdate};
use crate::weight::FollowedPush;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastSent>()
            .add_systems(OnEnter(Screen::Lobby), reset_last_sent)
            .add_systems(
                Update,
                (held_actions, send_input)
                    .chain()
                    .after(RaceUpdate)
                    .after(TouchUpdate)
                    .run_if(in_state(Screen::Lobby)),
            )
            .init_resource::<OwnHonk>()
            .init_resource::<HeldActions>()
            .add_systems(
                Update,
                honk.after(RaceUpdate)
                    .after(TouchUpdate)
                    .run_if(in_state(Screen::Lobby)),
            )
            .add_systems(PostUpdate, rumble.run_if(in_state(Screen::Lobby)));
    }
}

/// The input the server knows, and its number in this lobby.
#[derive(Resource, Default)]
struct LastSent {
    input: CarInput,
    /// 0 before the first one, which is 1: snapshots say 0 until the server has one.
    seq: u32,
}

/// A player enters a lobby with nothing pressed, as far as the server knows, and counts their
/// inputs there from the start.
fn reset_last_sent(mut last_sent: ResMut<LastSent>) {
    *last_sent = LastSent::default();
}

fn send_input(
    script: Res<Script>,
    view: Res<RaceView>,
    current: Res<CurrentLobby>,
    track: Option<Res<CurrentTrack>>,
    leave_dialog: Res<LeaveDialog>,
    gamepads: Query<&Gamepad>,
    keyboard: Res<ButtonInput<KeyCode>>,
    touch: Res<TouchControls>,
    preferences: Res<StoredPreferences>,
    time: Res<Time>,
    mut digital: Local<DigitalSteering>,
    network: Res<Network>,
    mut last_sent: ResMut<LastSent>,
    mut latency: ResMut<Latency>,
    mut prediction: ResMut<Prediction>,
    held: Res<HeldActions>,
) {
    let input = if leave_dialog.open || view.own_car.is_none() {
        // Asking whether to leave, or spectating: the car lets go.
        CarInput::default()
    } else if let Some(style) = script.autopilot {
        match (view.own_car, track) {
            // Waits for the start on screen to press, for a perfect start.
            _ if before_start(&current, &view) => CarInput::default(),
            // With the manner the server gives the same car once it has finished (see lobby.rs).
            (Some(car), Some(track)) => {
                let seed = current.0.as_ref().map_or(0, |membership| u32::from(membership.car.0));
                let personality = autopilot::Personality::of(seed);
                let held = script
                    .jump
                    .is_some_and(|hold| time.elapsed_secs() % crate::settings::JUMP_EVERY < hold);
                // `--jump` presses on top of the jumps the autopilot makes over gaps.
                let input = autopilot::drive_in_style_as(style, &personality, &car, &track.0);
                let input = input.with_jump(input.jump || held).with_thrust(input.thrust || held);
                // `--dash`: a press halfway between the jumps, the stick to its side for it.
                let every = crate::settings::JUMP_EVERY;
                match script.dash {
                    Some(side) if (time.elapsed_secs() + every / 2.0) % every < 0.1 => {
                        CarInput::new(input.accelerate, side)
                            .with_drift(input.drift)
                            .with_jump(input.jump)
                            .with_thrust(input.thrust)
                            .with_dash(true)
                    }
                    _ => input,
                }
            }
            _ => CarInput::default(),
        }
    } else {
        player_input(
            &preferences.get().controls,
            &gamepads,
            &keyboard,
            &touch,
            &mut digital,
            time.delta_secs(),
        )
        .with_reset(held.reset)
    };

    if last_sent.input != input {
        // Placed on the next tick of the player's own timeline, where their car is predicted,
        // and where the server will apply it too (see prediction.rs).
        let tick = prediction.place(input);
        let seq = last_sent.seq + 1;
        network.send(ClientMessage::Input { input, tick, seq });
        latency.input_sent(seq, Instant::now(), tick);
        *last_sent = LastSent { input, seq };
    }
}

/// What the actions held for a while, or toggled, have come to (since v0.1.70): a reset once
/// its button has been held long enough, the camera looking back while its button is held, and
/// whether the standings show.
#[derive(Resource)]
pub struct HeldActions {
    pub reset: Reset,
    pub rear_view: bool,
    pub standings: bool,
    /// How long each held action has been held, in seconds, and whether giving up was sent.
    held: [f32; 3],
    gave_up: bool,
}

impl Default for HeldActions {
    fn default() -> Self {
        Self {
            reset: Reset::None,
            rear_view: false,
            standings: true,
            held: [0.0; 3],
            gave_up: false,
        }
    }
}

impl HeldActions {
    /// How far a held action is to doing its thing, from 0 to 1, for the HUD to show it filling;
    /// `None` when none is held.
    pub fn filling(&self) -> Option<(Action, f32)> {
        [Action::ResetAsPassed, Action::ResetStanding, Action::GiveUp]
            .into_iter()
            .zip(self.held)
            .filter(|(_, held)| *held > 0.0)
            .map(|(action, held)| (action, (held / action.hold()).min(1.0)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    }
}

/// Times the resets and giving up as they are held, sends the give-up once it has been held long
/// enough, and follows the rear view and the standings' switch. The menu itself is Start or
/// Escape, which are not bindable (see `ui/hud.rs`).
#[allow(clippy::too_many_arguments)]
fn held_actions(
    view: Res<RaceView>,
    leave_dialog: Res<LeaveDialog>,
    gamepads: Query<&Gamepad>,
    keyboard: Res<ButtonInput<KeyCode>>,
    preferences: Res<StoredPreferences>,
    time: Res<Time>,
    network: Res<Network>,
    mut held: ResMut<HeldActions>,
) {
    let bindings = &preferences.get().controls;
    let down = |action: Action| {
        let binding = bindings.of(action);
        keyboard.any_pressed(binding.keys()) || gamepads.iter().any(|pad| binding.held_on(pad))
    };
    let pressed = |action: Action| {
        let binding = bindings.of(action);
        keyboard.any_just_pressed(binding.keys())
            || gamepads.iter().any(|pad| {
                binding
                    .gamepad
                    .0
                    .is_some_and(|button| pad.just_pressed(button))
            })
    };
    if pressed(Action::Standings) {
        held.standings = !held.standings;
    }
    let driving = !leave_dialog.open && view.own_car.is_some();
    held.rear_view = driving && down(Action::RearView);
    let seconds = time.delta_secs();
    for (index, action) in [Action::ResetAsPassed, Action::ResetStanding, Action::GiveUp]
        .into_iter()
        .enumerate()
    {
        held.held[index] = if driving && down(action) {
            held.held[index] + seconds
        } else {
            0.0
        };
    }
    held.reset = if held.held[0] >= Action::ResetAsPassed.hold() {
        Reset::AsPassed
    } else if held.held[1] >= Action::ResetStanding.hold() {
        Reset::Standing
    } else {
        Reset::None
    };
    if held.held[2] >= Action::GiveUp.hold() {
        if !held.gave_up {
            held.gave_up = true;
            network.send(ClientMessage::Abandon);
        }
    } else if held.held[2] == 0.0 {
        held.gave_up = false;
    }
}

/// The player sounded their horn this frame: the sound plays it at once, without waiting for the
/// server to carry it back (see `sound.rs`).
#[derive(Resource, Default)]
pub struct OwnHonk(pub bool);

/// Sounds the horn on a press of its key, its button or the touch cross's horn branch, while the
/// player has a car and is not asking to leave. The server spaces honks out; so does this, for
/// the sound played at once.
#[allow(clippy::too_many_arguments)]
fn honk(
    view: Res<RaceView>,
    leave_dialog: Res<LeaveDialog>,
    gamepads: Query<&Gamepad>,
    keyboard: Res<ButtonInput<KeyCode>>,
    touch: Res<TouchControls>,
    preferences: Res<StoredPreferences>,
    time: Res<Time>,
    network: Res<Network>,
    mut own: ResMut<OwnHonk>,
    mut touching: Local<bool>,
    mut last: Local<f32>,
) {
    own.0 = false;
    let on_branch = touch.honking();
    let touched = on_branch && !*touching;
    *touching = on_branch;
    if leave_dialog.open || view.own_car.is_none() {
        return;
    }
    let horn = &preferences.get().controls.horn;
    let pressed = touched
        || keyboard.any_just_pressed(horn.keys())
        || gamepads.iter().any(|gamepad| {
            horn.gamepad
                .0
                .is_some_and(|button| gamepad.just_pressed(button))
        });
    let now = time.elapsed_secs();
    let spaced = now - *last >= star_racer_protocol::HONK_EVERY_TICKS as f32 / star_racer_sim::TICK_RATE as f32;
    if pressed && spaced {
        *last = now;
        own.0 = true;
        network.send(ClientMessage::Honk);
    }
}

/// Whether the cars on screen wait on the grid for the start.
fn before_start(current: &CurrentLobby, view: &RaceView) -> bool {
    let phase = current
        .0
        .as_ref()
        .and_then(|membership| membership.state.as_ref())
        .map(|state| &state.phase);
    // On the player's own timeline while their car is predicted: the start their car can make.
    match (phase, view.own_tick.or(view.tick)) {
        (Some(LobbyPhase::Racing { start_tick }), Some(tick)) => tick < f64::from(*start_tick),
        _ => false,
    }
}

/// Every connected gamepad, the keyboard and the thumbs on the screen at once: whichever is used
/// drives.
fn player_input(
    bindings: &Bindings,
    gamepads: &Query<&Gamepad>,
    keyboard: &ButtonInput<KeyCode>,
    touch: &TouchControls,
    digital: &mut DigitalSteering,
    seconds: f32,
) -> CarInput {
    let mut accelerate = keyboard.any_pressed(bindings.accelerate.keys()) || touch.accelerating();
    let mut drift = keyboard.any_pressed(bindings.drift.keys()) || touch.drifting();
    let mut turbo = keyboard.any_pressed(bindings.turbo.keys()) || touch.firing_turbo();
    let mut jump = keyboard.any_pressed(bindings.jump.keys()) || touch.jumping();
    // The touch cross has one branch for both: arriving on it hops, staying holds the thruster.
    let mut thrust = keyboard.any_pressed(bindings.thrust.keys()) || touch.jumping();
    let mut steer: f32 = touch.steer();

    let mut dash = keyboard.any_pressed(bindings.dash.keys());
    let mut air_roll = keyboard.any_pressed(bindings.air_roll.keys());
    // On the keyboard the Air Roll dives on the accelerator and holds up on S.
    let mut pitch = if air_roll {
        f32::from(u8::from(keyboard.any_pressed(bindings.accelerate.keys())))
            - f32::from(u8::from(keyboard.pressed(KeyCode::KeyS)))
    } else {
        0.0
    };
    let mut left = keyboard.any_pressed(bindings.steer_left.keys());
    let mut right = keyboard.any_pressed(bindings.steer_right.keys());
    for gamepad in gamepads {
        dash |= bindings.dash.held_on(gamepad);
        air_roll |= bindings.air_roll.held_on(gamepad);
        pitch += stick_steering(gamepad.left_stick().y);
        accelerate |= bindings.accelerate.held_on(gamepad);
        drift |= bindings.drift.held_on(gamepad);
        turbo |= bindings.turbo.held_on(gamepad);
        jump |= bindings.jump.held_on(gamepad);
        thrust |= bindings.thrust.held_on(gamepad);
        steer += stick_steering(gamepad.left_stick().x);
        // The buttons bound to steering say left or right and nothing between, as keys do.
        left |= bindings.steer_left.held_on(gamepad);
        right |= bindings.steer_right.held_on(gamepad);
    }
    steer += digital.steer(left, right, seconds);

    CarInput::new(accelerate, steer)
        .with_drift(drift)
        .with_turbo(turbo)
        .with_jump(jump)
        .with_thrust(thrust)
        .with_dash(dash)
        .with_air_roll(air_roll, pitch)
}

/// A stick resting this far off center does not steer: a worn one rests a little off it.
const STICK_DEAD: f32 = 0.1;
/// A stick pushed this far steers at full lock: one pushed at an angle never reaches its rim.
const STICK_FULL: f32 = 0.94;

/// The steering a stick's sideways push asks for: nothing in its dead zone, then in proportion up
/// to full lock, a little short of the rim. In proportion in between: the handling with a gamepad
/// was judged right as it was, and only its two ends change.
fn stick_steering(x: f32) -> f32 {
    let amount = ((x.abs() - STICK_DEAD) / (STICK_FULL - STICK_DEAD)).clamp(0.0, 1.0);
    amount.copysign(x)
}

/// How far a key or a D-pad button steers the moment it is pressed, and how much further every
/// second it stays held, up to full lock: half lock at once, full lock 0.2 s later.
const KEY_FIRST: f32 = 0.5;
const KEY_RAMP: f32 = 2.5;
/// The ramp moves in steps of a simulation tick, so a held key sends no more inputs than the
/// server has ticks, however fast the frames come.
const KEY_STEP: f32 = 1.0 / 60.0;

/// Steering from keys and the D-pad, which say only left or right: it eases in while they are held.
#[derive(Default)]
struct DigitalSteering {
    /// The side held, -1 left or 1 right, and for how long, in seconds.
    side: i8,
    held: f32,
    /// The last side pressed, which steers while both are held.
    last: i8,
    /// Whether left and right were held on the frame before, to tell a press from a hold: a
    /// trigger bound to steering is held from a third of its travel, and never says "pressed".
    was: (bool, bool),
}

impl DigitalSteering {
    /// The steering for this frame, `seconds` after the last, with `left` and `right` held.
    fn steer(&mut self, left: bool, right: bool, seconds: f32) -> f32 {
        if left && !self.was.0 {
            self.last = -1;
        } else if right && !self.was.1 {
            self.last = 1;
        }
        self.was = (left, right);
        let side = match (left, right) {
            (true, false) => -1,
            (false, true) => 1,
            (true, true) => self.last,
            (false, false) => 0,
        };
        if side != self.side {
            self.side = side;
            self.held = 0.0;
        } else {
            self.held += seconds;
        }
        if side == 0 {
            return 0.0;
        }
        let held = (self.held / KEY_STEP + 1e-3).floor() * KEY_STEP;
        (KEY_FIRST + KEY_RAMP * held).min(1.0) * f32::from(side)
    }
}

/// How soon a rumble for a hit may follow the last one: scraping along a wall is a hit on every
/// frame.
const RUMBLE_EVERY: f64 = 0.3;

/// What the gamepad last told the player.
#[derive(Default)]
struct Rumbled {
    hit: f64,
    drifting: bool,
    boosting: bool,
    turbo: bool,
    airborne: bool,
    thrusting: bool,
}

/// Rumbles every gamepad when the player's car hits a wall, harder the harder the hit, breaks into
/// a drift, or fires a boost: what a phone's vibration motor says on a touchscreen.
fn rumble(
    push: Res<FollowedPush>,
    view: Res<RaceView>,
    time: Res<Time<Real>>,
    preferences: Res<StoredPreferences>,
    gamepads: Query<Entity, With<Gamepad>>,
    mut requests: MessageWriter<GamepadRumbleRequest>,
    mut rumbled: Local<Rumbled>,
) {
    if !preferences.get().vibration {
        return;
    }
    let Some(car) = view.own_car else {
        *rumbled = Rumbled::default();
        return;
    };
    let now = time.elapsed_secs_f64();
    let mut send = |strong: f32, weak: f32, millis: u64| {
        for gamepad in &gamepads {
            requests.write(GamepadRumbleRequest::Add {
                gamepad,
                duration: Duration::from_millis(millis),
                intensity: GamepadRumbleIntensity {
                    strong_motor: strong,
                    weak_motor: weak,
                },
            });
        }
    };
    if let Some(push) = push.0
        && let Some((strength, millis)) = hit_rumble(push.impact)
        && now - rumbled.hit >= RUMBLE_EVERY
    {
        rumbled.hit = now;
        send(strength, strength * 0.5, millis);
    }
    if car.is_drifting() && !rumbled.drifting {
        send(0.0, 0.3, 50);
    }
    if car.turbo_boost > 0.0 && !rumbled.turbo {
        // The turbo kicks: both motors, hard, for longer than any boost.
        send(0.8, 1.0, 320);
    } else if car.boost > 0.0 && !rumbled.boosting {
        send(0.35, 0.6, 180);
    }
    // The hop is a tap of the weak motor; the thruster's ignition a push of both, and its burn
    // a light hum for as long as it lasts; landing, a knock.
    if car.is_airborne() && !rumbled.airborne && car.climb > 0.0 {
        send(0.0, 0.35, 60);
    }
    if car.thrusting && !rumbled.thrusting {
        send(0.55, 0.8, 200);
    } else if car.thrusting {
        send(0.12, 0.2, 40);
    }
    if !car.is_airborne() && rumbled.airborne {
        send(0.5, 0.3, 90);
    }
    rumbled.airborne = car.is_airborne();
    rumbled.thrusting = car.thrusting;
    rumbled.drifting = car.is_drifting();
    rumbled.boosting = car.boost > 0.0;
    rumbled.turbo = car.turbo_boost > 0.0;
}

/// How hard, from 0 to 1, and how long, in milliseconds, a hit that took `impact` m/s from the car
/// rumbles, if it is more than a graze.
fn hit_rumble(impact: f32) -> Option<(f32, u64)> {
    (impact >= 1.5).then(|| {
        let strength = (impact / 12.0).clamp(0.25, 1.0);
        (strength, (60.0 + 100.0 * strength) as u64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stick_has_a_dead_zone_and_reaches_full_lock_short_of_its_rim() {
        assert_eq!(stick_steering(0.0), 0.0);
        assert_eq!(stick_steering(0.08), 0.0);
        assert_eq!(stick_steering(-0.08), 0.0);
        assert_eq!(stick_steering(0.95), 1.0);
        assert_eq!(stick_steering(-1.0), -1.0);
        let half = stick_steering(0.52);
        assert!((half - 0.5).abs() < 1e-4, "{half}");
        // In proportion in between.
        let quarter = stick_steering(-0.31);
        assert!((quarter + 0.25).abs() < 1e-4, "{quarter}");
    }

    /// Steps a held key for `seconds`, frame by frame at `fps`, and returns the last steering.
    fn hold(digital: &mut DigitalSteering, left: bool, right: bool, seconds: f32, fps: f32) -> f32 {
        let mut steer = 0.0;
        for _ in 0..(seconds * fps).round() as usize {
            steer = digital.steer(left, right, 1.0 / fps);
        }
        steer
    }

    #[test]
    fn a_key_steers_at_half_lock_at_once_and_at_full_lock_soon_after() {
        let mut digital = DigitalSteering::default();
        assert_eq!(digital.steer(false, true, 1.0 / 60.0), 0.5);
        let after_a_tenth = hold(&mut digital, false, true, 0.1, 60.0);
        assert!(
            after_a_tenth > 0.7 && after_a_tenth < 0.8,
            "{after_a_tenth}"
        );
        assert_eq!(hold(&mut digital, false, true, 0.2, 60.0), 1.0);
        // Let go, and it steers straight at once; the car's own steering eases it back.
        assert_eq!(digital.steer(false, false, 1.0 / 60.0), 0.0);
    }

    /// At 300 frames a second the ramp moves no more often than at 60: one input per tick.
    #[test]
    fn a_held_key_changes_the_steering_no_faster_than_the_server_ticks() {
        let mut digital = DigitalSteering::default();
        let mut changes = 0;
        let mut last = digital.steer(true, false, 1.0 / 300.0);
        for _ in 0..90 {
            let steer = digital.steer(true, false, 1.0 / 300.0);
            if steer != last {
                changes += 1;
                last = steer;
            }
        }
        assert_eq!(last, -1.0);
        assert!(changes <= 13, "{changes}");
    }

    /// Both held: the last pressed steers, as on the touchscreen, rather than the two cancelling
    /// out and driving the car straight into the wall.
    #[test]
    fn with_both_keys_held_the_last_one_pressed_steers() {
        let mut digital = DigitalSteering::default();
        digital.steer(true, false, 1.0 / 60.0);
        assert_eq!(digital.steer(true, true, 1.0 / 60.0), 0.5);
        // Letting go of the right one goes back to the left, from half lock again.
        assert_eq!(digital.steer(true, false, 1.0 / 60.0), -0.5);
    }

    #[test]
    fn a_hit_rumbles_as_hard_as_it_was() {
        assert_eq!(hit_rumble(1.0), None);
        let (light, short) = hit_rumble(2.0).unwrap();
        let (hard, long) = hit_rumble(20.0).unwrap();
        assert!(light < hard && short < long);
        assert_eq!(hard, 1.0);
    }
}
