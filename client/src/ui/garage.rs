//! The garage: where the player picks their car, paints it, and chooses its trail and its horn.
//!
//! Reached from the first screen, connected or not. The car stands over the menu's neon grid,
//! turning slowly with its trail swept out behind it, to the right of a card of six rows: the car,
//! its paint, its neon, its trail, the trail's color and the horn. The car, the trail and the horn
//! are stepped through with left and right, the arrows beside them, or a click, and what is not
//! won yet says at which level it will be (see `protocol::rewards`); the colors are palettes, every
//! color in a row at once, picked with a click or stepped along the same way. The player's level
//! stands under the title, small. What is chosen, of what is unlocked, is kept with the menu
//! choices and sent to the server, which shows it to everyone in the next lobby the player enters.
//! Every car drives the same: this is how a car looks and sounds, never how it handles.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use star_racer_protocol::rewards::{self, Reward};
use star_racer_protocol::{
    CAR_HORNS, CAR_MODELS, CAR_NEONS, CAR_PAINTS, CAR_TRAIL_COLORS, CAR_TRAILS, CarLook,
};

use super::navigation::MenuInput;
use super::safe_area::SafePadding;
use super::theme::{self, Fonts};
use super::touch::Touchscreen;
use super::widgets::{ButtonKind, Selectable, Stepper, button, hint, hint_row, panel, step_button};
use crate::car_model::{CarModel, NEONS, PAINTS};
use crate::lobby::PlayerProgress;
use crate::net::Network;
use crate::preferences::{LookPreference, StoredPreferences};
use crate::race::{CarAssets, spawn_car};
use crate::screen::Screen;
use crate::settings::Script;
use crate::sound::Sounds;
use crate::sound::horn::HORNS;
use crate::trail::{self, Effect, TRAIL_COLORS, TRAILS, TrailMaterial};

/// Where the car stands in front of the menu camera, which `backdrop.rs` places at 3 m over the
/// grid looking far ahead: a little to the right of the card, close enough to fill the screen.
const PREVIEW_AT: Vec3 = Vec3::new(2.6, 0.0, -8.5);
/// How fast it turns, in radians per second.
const TURN_RATE: f32 = 0.45;

pub struct GaragePlugin;

impl Plugin for GaragePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Login), open_scripted)
            .add_systems(OnEnter(Screen::Garage), spawn_garage)
            .add_observer(pick_swatch)
            .add_systems(
                Update,
                (
                    show_choices,
                    show_swatches,
                    show_level,
                    show_preview,
                    play_horn,
                    turn_preview,
                    leave,
                )
                    .chain()
                    .run_if(in_state(Screen::Garage)),
            );
    }
}

/// Which part of the look a row steps through.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum Part {
    Model,
    Paint,
    Neon,
    Trail,
    TrailColor,
    Horn,
}

impl Part {
    /// Whether the row is a palette of colors rather than a list of names.
    fn is_palette(self) -> bool {
        matches!(self, Self::Paint | Self::Neon | Self::TrailColor)
    }

    /// The reward the row's `value` is, if the row holds things to win.
    fn reward(self, value: u8) -> Option<Reward> {
        match self {
            Self::Model => Some(Reward::Car(value)),
            Self::Trail => Some(Reward::Trail(value)),
            Self::Horn => Some(Reward::Horn(value)),
            _ => None,
        }
    }

    /// The colors of a palette row, as they are seen: neon brought down to what a screen shows.
    fn colors(self) -> Vec<Color> {
        let seen = |neon: LinearRgba| {
            let max = neon.red.max(neon.green).max(neon.blue);
            Color::LinearRgba(neon * (1.0 / max)).with_alpha(1.0)
        };
        match self {
            Self::Paint => PAINTS.iter().map(|(_, color)| *color).collect(),
            Self::Neon => NEONS.iter().map(|(_, neon)| seen(*neon)).collect(),
            Self::TrailColor => TRAIL_COLORS.iter().map(|(_, neon)| seen(*neon)).collect(),
            _ => Vec::new(),
        }
    }
}

/// The name of what a row holds, and the line under it when it is locked.
#[derive(Component)]
struct ChoiceName(Part);

#[derive(Component)]
struct LockLine(Part);

/// A color of a palette row: a click on it picks it.
#[derive(Component)]
struct Swatch {
    row: Entity,
    value: u8,
}

/// The line describing the car.
#[derive(Component)]
struct Blurb;

/// The player's level, under the title.
#[derive(Component)]
struct LevelLine;

/// The car turning beside the card, the look it is drawn in, and the mesh of its trail.
#[derive(Component)]
struct Preview(CarLook, Handle<Mesh>);

/// The look as the rows now say it, whether or not it is unlocked.
fn look_of(rows: &Query<(&Part, &Stepper)>) -> CarLook {
    let mut look = CarLook::default();
    for (part, stepper) in rows {
        match part {
            Part::Model => look.model = stepper.value,
            Part::Paint => look.paint = stepper.value,
            Part::Neon => look.neon = stepper.value,
            Part::Trail => look.trail = stepper.value,
            Part::TrailColor => look.trail_color = stepper.value,
            Part::Horn => look.horn = stepper.value,
        }
    }
    look.checked()
}

fn xp_of(progress: &PlayerProgress) -> u32 {
    progress.xp.unwrap_or(0)
}

fn spawn_garage(
    mut commands: Commands,
    fonts: Res<Fonts>,
    preferences: Res<StoredPreferences>,
    touchscreen: Res<Touchscreen>,
    mut focus: ResMut<InputFocus>,
) {
    let pick = |desk: f32, phone: f32| touchscreen.pick(desk, phone);
    let look = preferences.get().car_look();

    let card = commands
        .spawn((
            Node {
                width: px(pick(460.0, 400.0)),
                flex_direction: FlexDirection::Column,
                padding: px(pick(22.0, 10.0)).all(),
                row_gap: px(pick(8.0, 3.0)),
                border: px(1).all(),
                border_radius: BorderRadius::all(px(theme::RADIUS)),
                ..default()
            },
            panel(),
        ))
        .with_children(|card| {
            card.spawn(Node {
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                ..default()
            })
            .with_children(|title| {
                title.spawn((
                    Text::new("GARAGE"),
                    fonts.display(pick(40.0, 26.0)),
                    TextColor(theme::TEXT),
                    TextShadow {
                        offset: Vec2::new(3.0, 3.0),
                        color: theme::NEON.with_alpha(0.8),
                    },
                ));
                title.spawn((
                    LevelLine,
                    Text::default(),
                    fonts.caption(12.0),
                    theme::CAPTION_SPACING,
                    TextColor(theme::TEXT_FAINT),
                ));
            });
            card.spawn((
                Blurb,
                Text::default(),
                fonts.text(pick(14.0, 12.0), 400),
                TextColor(theme::TEXT_DIM),
                Node {
                    min_height: px(pick(20.0, 15.0)),
                    ..default()
                },
            ));
        })
        .id();

    let mut first_row = None;
    for (part, label, value, count) in [
        (Part::Model, "CAR", look.model, CAR_MODELS),
        (Part::Paint, "PAINT", look.paint, CAR_PAINTS),
        (Part::Neon, "NEON", look.neon, CAR_NEONS),
        (Part::Trail, "TRAIL", look.trail, CAR_TRAILS),
        (Part::TrailColor, "COLOR", look.trail_color, CAR_TRAIL_COLORS),
        (Part::Horn, "HORN", look.horn, CAR_HORNS),
    ] {
        let row = commands
            .spawn((
                part,
                Stepper {
                    value,
                    min: 0,
                    max: count - 1,
                    wrap: true,
                },
                Selectable,
                Node {
                    padding: UiRect::axes(px(pick(12.0, 8.0)), px(pick(6.0, 2.0))),
                    align_items: AlignItems::Center,
                    column_gap: px(8),
                    border: px(1.5).all(),
                    border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                    ..default()
                },
            ))
            .id();
        first_row.get_or_insert(row);
        commands.entity(row).with_children(|content| {
            content.spawn((
                Text::new(label),
                fonts.caption(12.0),
                theme::CAPTION_SPACING,
                TextColor(theme::TEXT_FAINT),
                Node {
                    width: px(52),
                    flex_shrink: 0.0,
                    ..default()
                },
            ));
            if part.is_palette() {
                // Every color at once, the chosen one ringed.
                content
                    .spawn(Node {
                        flex_grow: 1.0,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        column_gap: px(pick(5.0, 4.0)),
                        row_gap: px(4),
                        padding: UiRect::vertical(px(pick(5.0, 3.0))),
                        ..default()
                    })
                    .with_children(|palette| {
                        for (value, color) in part.colors().into_iter().enumerate() {
                            palette.spawn((
                                Swatch {
                                    row,
                                    value: value as u8,
                                },
                                Button,
                                Node {
                                    width: px(pick(20.0, 17.0)),
                                    height: px(pick(20.0, 17.0)),
                                    border: px(2).all(),
                                    border_radius: BorderRadius::MAX,
                                    ..default()
                                },
                                BackgroundColor(color),
                                BorderColor::all(Color::NONE),
                            ));
                        }
                    });
                return;
            }
            content.spawn(step_button(&fonts, row, -1));
            content
                .spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|middle| {
                    middle.spawn((
                        ChoiceName(part),
                        Text::default(),
                        fonts.text(pick(18.0, 15.0), 700),
                        TextColor(theme::TEXT),
                    ));
                    middle.spawn((
                        LockLine(part),
                        Text::default(),
                        fonts.caption(10.0),
                        theme::CAPTION_SPACING,
                        TextColor(theme::AMBER),
                        Node {
                            display: Display::None,
                            ..default()
                        },
                    ));
                });
            content.spawn(step_button(&fonts, row, 1));
        });
        commands.entity(card).add_child(row);
    }

    commands.entity(card).with_children(|card| {
        card.spawn(button(&fonts, ButtonKind::Primary, "DONE"))
            .insert(Node {
                height: px(pick(48.0, 38.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: px(1.5).all(),
                border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                margin: UiRect::top(px(pick(6.0, 2.0))),
                ..default()
            })
            .observe(done);
    });

    commands
        .spawn((
            DespawnOnExit(Screen::Garage),
            SafePadding::default(),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                padding: UiRect::left(px(pick(60.0, 20.0))),
                ..default()
            },
        ))
        .add_child(card)
        .with_children(|root| {
            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(28),
                    bottom: px(22),
                    display: touchscreen.pick(Display::Flex, Display::None),
                    ..default()
                },
                children![(
                    hint_row(),
                    children![
                        hint(&fonts, "↑ ↓", "D-pad", "Choose"),
                        hint(&fonts, "← →", "D-pad", "Change"),
                        hint(&fonts, "Esc", "B", "Done"),
                    ],
                )],
            ));
        });

    commands.spawn((
        DespawnOnExit(Screen::Garage),
        PointLight {
            intensity: 250_000.0,
            range: 30.0,
            color: Color::srgb(0.85, 0.9, 1.0),
            ..default()
        },
        Transform::from_translation(PREVIEW_AT + Vec3::new(-2.0, 5.0, 4.0)),
    ));

    if let Some(row) = first_row {
        focus.set(row, FocusCause::Navigated);
    }
}

/// `--garage` opens it at once, for unattended looks at it.
fn open_scripted(mut script: ResMut<Script>, mut next: ResMut<NextState<Screen>>) {
    if std::mem::take(&mut script.garage) {
        next.set(Screen::Garage);
    }
}

/// A click on a color of a palette picks it.
fn pick_swatch(
    activate: On<Activate>,
    swatches: Query<&Swatch>,
    mut steppers: Query<&mut Stepper>,
) {
    if let Ok(swatch) = swatches.get(activate.entity)
        && let Ok(mut stepper) = steppers.get_mut(swatch.row)
    {
        stepper.value = swatch.value;
    }
}

/// Keeps the choice and tells the server, then goes back to the first screen.
fn done(
    _: On<Activate>,
    rows: Query<(&Part, &Stepper)>,
    progress: Res<PlayerProgress>,
    mut preferences: ResMut<StoredPreferences>,
    mut network: ResMut<Network>,
    mut next: ResMut<NextState<Screen>>,
) {
    keep(look_of(&rows), &progress, &mut preferences, &mut network);
    next.set(Screen::Login);
}

/// Keeps what is chosen of what is unlocked: anything still locked goes back to what it was, or to
/// the first if that is locked too.
fn keep(
    chosen: CarLook,
    progress: &PlayerProgress,
    preferences: &mut StoredPreferences,
    network: &mut Network,
) {
    let xp = xp_of(progress);
    let before = rewards::allowed(preferences.get().car_look(), xp);
    let mut look = chosen;
    if !rewards::is_unlocked(Reward::Car(look.model), xp) {
        look.model = before.model;
    }
    if !rewards::is_unlocked(Reward::Trail(look.trail), xp) {
        look.trail = before.trail;
    }
    if !rewards::is_unlocked(Reward::Horn(look.horn), xp) {
        look.horn = before.horn;
    }
    if preferences.get().car_look() != look {
        preferences.update(|stored| stored.look = Some(LookPreference::of(look)));
    }
    network.set_look(look);
}

/// Back, from the keyboard or a gamepad, is DONE: what is chosen is kept.
fn leave(
    input: MenuInput,
    rows: Query<(&Part, &Stepper)>,
    progress: Res<PlayerProgress>,
    mut preferences: ResMut<StoredPreferences>,
    mut network: ResMut<Network>,
    mut next: ResMut<NextState<Screen>>,
) {
    if input.back() {
        keep(look_of(&rows), &progress, &mut preferences, &mut network);
        next.set(Screen::Login);
    }
}

/// Writes what each named row holds, and whether it is locked; and the car's line. The steppers
/// step themselves (see `widgets.rs` and `navigation.rs`).
fn show_choices(
    rows: Query<(&Part, &Stepper)>,
    progress: Res<PlayerProgress>,
    mut names: Query<(&ChoiceName, &mut Text, &mut TextColor), (Without<Blurb>, Without<LockLine>)>,
    mut locks: Query<(&LockLine, &mut Text, &mut Node), (Without<Blurb>, Without<ChoiceName>)>,
    mut blurbs: Query<&mut Text, (With<Blurb>, Without<ChoiceName>, Without<LockLine>)>,
) {
    let look = look_of(&rows);
    let xp = xp_of(&progress);
    let model = CarModel::of(look.model);
    let value = |part: Part| match part {
        Part::Model => look.model,
        Part::Trail => look.trail,
        Part::Horn => look.horn,
        Part::Paint => look.paint,
        Part::Neon => look.neon,
        Part::TrailColor => look.trail_color,
    };
    let locked = |part: Part| {
        part.reward(value(part))
            .filter(|reward| !rewards::is_unlocked(*reward, xp))
    };
    for (name, mut text, mut color) in &mut names {
        let wanted = match name.0 {
            Part::Model => model.name(),
            Part::Trail => TRAILS[usize::from(look.trail)].0,
            Part::Horn => HORNS[usize::from(look.horn)],
            _ => "",
        };
        if text.0 != wanted {
            text.0 = wanted.to_owned();
        }
        color.0 = if locked(name.0).is_some() {
            theme::TEXT_FAINT
        } else {
            theme::TEXT
        };
    }
    for (lock, mut text, mut node) in &mut locks {
        match locked(lock.0) {
            Some(reward) => {
                let wanted = format!("LOCKED · LEVEL {}", rewards::unlock_level(reward));
                if text.0 != wanted {
                    text.0 = wanted;
                }
                node.display = Display::Flex;
            }
            None => node.display = Display::None,
        }
    }
    for mut text in &mut blurbs {
        if text.0 != model.blurb() {
            text.0 = model.blurb().to_owned();
        }
    }
}

/// Rings the chosen color of every palette.
fn show_swatches(
    rows: Query<&Stepper>,
    mut swatches: Query<(&Swatch, &mut BorderColor, &mut Node)>,
) {
    for (swatch, mut border, mut node) in &mut swatches {
        let chosen = rows
            .get(swatch.row)
            .is_ok_and(|stepper| stepper.value == swatch.value);
        *border = BorderColor::all(if chosen { theme::TEXT } else { Color::NONE });
        let outline = if chosen { px(3) } else { px(2) };
        if node.border.left != outline {
            node.border = outline.all();
        }
    }
}

/// The player's level and how far into it they are, small, under the title.
fn show_level(progress: Res<PlayerProgress>, mut lines: Query<&mut Text, With<LevelLine>>) {
    let xp = xp_of(&progress);
    let wanted = format!(
        "LEVEL {}  ·  {}%",
        rewards::level(xp),
        (rewards::level_share(xp) * 100.0) as u32
    );
    for mut text in &mut lines {
        if text.0 != wanted {
            text.0.clone_from(&wanted);
        }
    }
}

/// Plays the horn as the player steps to it.
fn play_horn(
    mut commands: Commands,
    rows: Query<(&Part, &Stepper), Changed<Stepper>>,
    sounds: Res<Sounds>,
    mut opened: Local<bool>,
) {
    // Not the row as it is first drawn.
    let first = !std::mem::replace(&mut *opened, true);
    for (part, stepper) in &rows {
        if *part == Part::Horn
            && !first
            && let Some(horn) = sounds.horns.get(usize::from(stepper.value))
        {
            sounds.play(&mut commands, horn, 0.7);
        }
    }
}

/// Draws the car in the look chosen, again whenever it changes, keeping its turn.
fn show_preview(
    mut commands: Commands,
    rows: Query<(&Part, &Stepper)>,
    previews: Query<(Entity, &Preview, &Transform)>,
    mut assets: ResMut<CarAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    trail_material: Res<TrailMaterial>,
) {
    let look = look_of(&rows);
    let mut rotation = Quat::from_rotation_y(-0.6);
    for (entity, preview, transform) in &previews {
        if preview.0 == look {
            return;
        }
        rotation = transform.rotation;
        meshes.remove(&preview.1);
        commands.entity(entity).despawn();
    }
    // The trail under it, swept out behind its tail as if it had just driven a bend.
    let trail = meshes.add(
        trail::trail_geometry(
            Effect::of(look.trail),
            TRAIL_COLORS[usize::from(look.trail_color)].1,
            &trail::preview_points(),
            trail::PREVIEW_NOW,
            &trail::flat,
        )
        .into_mesh(),
    );
    let car = spawn_car(
        &mut commands,
        &mut assets,
        &mut meshes,
        look,
        None,
        false,
        (
            Preview(look, trail.clone()),
            DespawnOnExit(Screen::Garage),
            Transform::from_translation(PREVIEW_AT).with_rotation(rotation),
            Visibility::default(),
        ),
    );
    commands.entity(car).with_child((
        Mesh3d(trail),
        MeshMaterial3d(trail_material.0.clone()),
        NotShadowCaster,
    ));
}

fn turn_preview(time: Res<Time>, mut previews: Query<&mut Transform, With<Preview>>) {
    for mut transform in &mut previews {
        transform.rotate_y(TURN_RATE * time.delta_secs());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows step round: past the last car comes the first.
    #[test]
    fn the_choices_wrap_round() {
        let mut stepper = Stepper {
            value: CAR_MODELS - 1,
            min: 0,
            max: CAR_MODELS - 1,
            wrap: true,
        };
        stepper.step(1);
        assert_eq!(stepper.value, 0);
        stepper.step(-1);
        assert_eq!(stepper.value, CAR_MODELS - 1);
    }

    /// Every palette row holds as many colors as the wire allows.
    #[test]
    fn the_palettes_hold_every_color() {
        assert_eq!(Part::Paint.colors().len(), usize::from(CAR_PAINTS));
        assert_eq!(Part::Neon.colors().len(), usize::from(CAR_NEONS));
        assert_eq!(Part::TrailColor.colors().len(), usize::from(CAR_TRAIL_COLORS));
    }
}
