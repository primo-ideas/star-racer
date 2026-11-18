//! GRAPHICS & SOUND, a dialog opened from the settings (since v1.0.0-rc4, the user's word: "des
//! jauges pour le volume générale, musique (grisé car a venir), et klaxxon" and, for the graphics,
//! "on pourra régler tout cela dans un sous menu des settings").
//!
//! The general volume and the horns' as neon bars stepped with ‹ and ›, the music's greyed until
//! there is music, and how fine the game is drawn: AUTO, fitted to the machine (`quality.rs`), or
//! a level of the player's own, with what the game made of the machine written under it.

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;

use super::navigation::MenuInput;
use super::theme::{self, Fonts};
use super::touch::Touchscreen;
use super::widgets::{ButtonKind, ButtonLabel, button, panel};
use crate::preferences::{AudioPreferences, StoredPreferences};
use crate::quality::{Quality, Setting};
use crate::screen::Screen;

/// Whether the dialog is up.
#[derive(Resource, Default)]
pub struct SoundGraphicsDialog {
    pub open: bool,
}

#[derive(Component)]
pub(super) struct Root;

/// Which volume a row holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Volume {
    Master,
    Music,
    Horns,
}

impl Volume {
    fn get(self, audio: &AudioPreferences) -> u8 {
        match self {
            Volume::Master => audio.master,
            Volume::Music => audio.music,
            Volume::Horns => audio.horns,
        }
    }

    fn set(self, audio: &mut AudioPreferences, value: u8) {
        let value = value.min(10);
        match self {
            Volume::Master => audio.master = value,
            Volume::Music => audio.music = value,
            Volume::Horns => audio.horns = value,
        }
    }
}

/// A ‹ or › beside a volume.
#[derive(Component, Clone, Copy)]
struct Step {
    volume: Volume,
    by: i8,
}

/// A volume's bar, filled to it.
#[derive(Component, Clone, Copy)]
pub(super) struct Fill(Volume);

#[derive(Component)]
pub(super) struct GraphicsButton;

#[derive(Component)]
pub(super) struct GraphicsLine;

/// Its systems run in the settings' own chain, after theirs, so a press of back closes only it.
pub fn plugin(app: &mut App) {
    app.init_resource::<SoundGraphicsDialog>()
        .add_systems(OnExit(Screen::Login), close);
}

fn close(mut dialog: ResMut<SoundGraphicsDialog>) {
    dialog.open = false;
}

pub(super) fn close_on_back(input: MenuInput, mut dialog: ResMut<SoundGraphicsDialog>) {
    if dialog.open && input.back() {
        dialog.open = false;
    }
}

/// Opens the dialog: what the settings' button does.
pub fn open(_: On<Activate>, mut dialog: ResMut<SoundGraphicsDialog>) {
    dialog.open = true;
}

pub(super) fn show(
    mut commands: Commands,
    dialog: Res<SoundGraphicsDialog>,
    fonts: Res<Fonts>,
    touchscreen: Res<Touchscreen>,
    roots: Query<Entity, With<Root>>,
    mut focus: ResMut<InputFocus>,
    mut opener: Local<Option<Entity>>,
) {
    match (dialog.open, roots.single()) {
        (true, Err(_)) => {
            *opener = focus.get();
            let first = spawn(&mut commands, &fonts, *touchscreen);
            focus.set(first, FocusCause::Navigated);
        }
        (false, Ok(root)) => {
            commands.entity(root).despawn();
            match opener.take() {
                Some(opener) if commands.get_entity(opener).is_ok() => {
                    focus.set(opener, FocusCause::Navigated);
                }
                _ => focus.clear(),
            }
        }
        _ => {}
    }
}

/// Spawns the dialog and returns what takes the focus first.
fn spawn(commands: &mut Commands, fonts: &Fonts, touchscreen: Touchscreen) -> Entity {
    let pick = |desk: f32, phone: f32| touchscreen.pick(desk, phone);
    let mut first = None;
    let card = commands
        .spawn((
            Node {
                width: px(pick(560.0, 470.0)),
                flex_direction: FlexDirection::Column,
                padding: px(pick(24.0, 12.0)).all(),
                row_gap: px(pick(14.0, 7.0)),
                border: px(1).all(),
                border_radius: BorderRadius::all(px(theme::RADIUS)),
                ..default()
            },
            panel(),
        ))
        .insert(BackgroundColor(Color::srgba(0.015, 0.03, 0.06, 0.97)))
        .id();
    commands.entity(card).with_child((
        Text::new("GRAPHICS & SOUND"),
        fonts.display(pick(34.0, 22.0)),
        TextColor(theme::TEXT),
        TextShadow {
            offset: Vec2::new(3.0, 3.0),
            color: theme::NEON.with_alpha(0.7),
        },
    ));

    for (volume, label, help) in [
        (Volume::Master, "Volume", "Everything the game plays."),
        (Volume::Music, "Music", "Coming soon."),
        (Volume::Horns, "Horns", "Yours and everyone else's."),
    ] {
        let soon = volume == Volume::Music;
        let mut buttons = Vec::new();
        for by in [-1, 1] {
            let mut entity = commands.spawn((
                Step { volume, by },
                button(fonts, ButtonKind::Secondary, if by < 0 { "‹" } else { "›" }),
            ));
            entity.insert(Node {
                width: px(40),
                height: px(36),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: px(1.5).all(),
                border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                ..default()
            });
            if soon {
                entity.insert(InteractionDisabled);
            } else {
                entity.observe(step);
            }
            buttons.push(entity.id());
        }
        if !soon {
            first.get_or_insert(buttons[1]);
        }
        let bar = commands
            .spawn((
                Node {
                    width: px(pick(170.0, 130.0)),
                    height: px(10),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(Color::WHITE.with_alpha(0.1)),
            ))
            .with_child((
                Fill(volume),
                Node {
                    width: percent(0),
                    height: percent(100),
                    border_radius: BorderRadius::MAX,
                    ..default()
                },
                BackgroundColor(if soon { theme::TEXT_FAINT } else { theme::NEON }),
                BoxShadow::new(
                    if soon { Color::NONE } else { theme::NEON.with_alpha(0.8) },
                    px(0),
                    px(0),
                    px(0),
                    px(8),
                ),
            ))
            .id();
        let row = commands
            .spawn(Node {
                column_gap: px(10),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_child((
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    ..default()
                },
                children![
                    (
                        Text::new(label),
                        fonts.text(16.5, 600),
                        TextColor(if soon { theme::TEXT_FAINT } else { theme::TEXT }),
                    ),
                    (
                        Text::new(help),
                        fonts.text(13.0, 400),
                        TextColor(theme::TEXT_FAINT),
                    ),
                ],
            ))
            .add_children(&[buttons[0], bar, buttons[1]])
            .id();
        commands.entity(card).add_child(row);
    }

    // The graphics: AUTO or a level, and what the machine was taken for.
    let graphics = commands
        .spawn((GraphicsButton, button(fonts, ButtonKind::Secondary, "")))
        .insert(Node {
            width: px(pick(220.0, 170.0)),
            height: px(36),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: px(1.5).all(),
            border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
            ..default()
        })
        .observe(step_graphics)
        .id();
    let line = commands
        .spawn((
            GraphicsLine,
            Text::default(),
            fonts.text(13.0, 400),
            TextColor(theme::TEXT_FAINT),
        ))
        .id();
    let row = commands
        .spawn(Node {
            column_gap: px(10),
            align_items: AlignItems::Center,
            ..default()
        })
        .with_child((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                ..default()
            },
            children![(
                Text::new("Graphics"),
                fonts.text(16.5, 600),
                TextColor(theme::TEXT),
            )],
        ))
        .add_child(graphics)
        .id();
    commands.entity(card).add_children(&[row, line]);

    commands.entity(card).with_children(|card| {
        card.spawn(Node {
            justify_content: JustifyContent::FlexEnd,
            margin: UiRect::top(px(pick(8.0, 3.0))),
            ..default()
        })
        .with_children(|buttons| {
            buttons
                .spawn(button(fonts, ButtonKind::Primary, "DONE"))
                .observe(done);
        });
    });

    commands
        .spawn((
            Root,
            DespawnOnExit(Screen::Login),
            // Over the settings, which opened it.
            GlobalZIndex(20),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(theme::BACKDROP),
        ))
        .add_child(card);
    first.unwrap_or(graphics)
}

fn step(activate: On<Activate>, steps: Query<&Step>, mut preferences: ResMut<StoredPreferences>) {
    let Ok(step) = steps.get(activate.entity) else {
        return;
    };
    preferences.update(|preferences| {
        let value = step.volume.get(&preferences.audio) as i8 + step.by;
        step.volume.set(&mut preferences.audio, value.clamp(0, 10) as u8);
    });
}

fn step_graphics(_: On<Activate>, mut preferences: ResMut<StoredPreferences>) {
    preferences.update(|preferences| preferences.graphics = preferences.graphics.next());
}

fn done(_: On<Activate>, mut dialog: ResMut<SoundGraphicsDialog>) {
    dialog.open = false;
}

/// Keeps the bars, the graphics button and its line in step with what is stored and detected.
pub(super) fn refresh(
    preferences: Res<StoredPreferences>,
    quality: Res<Quality>,
    mut fills: Query<(&Fill, &mut Node)>,
    graphics: Query<&Children, With<GraphicsButton>>,
    mut labels: Query<&mut Text, (With<ButtonLabel>, Without<GraphicsLine>)>,
    mut lines: Query<&mut Text, (With<GraphicsLine>, Without<ButtonLabel>)>,
) {
    let stored = preferences.get();
    for (fill, mut node) in &mut fills {
        let width = percent(f32::from(fill.0.get(&stored.audio).min(10)) * 10.0);
        if node.width != width {
            node.width = width;
        }
    }
    let label = match stored.graphics {
        Setting::Auto => format!("AUTO  ·  {}", quality.level.name()),
        Setting::Fixed(level) => level.name().to_owned(),
    };
    for children in &graphics {
        for child in children {
            if let Ok(mut text) = labels.get_mut(*child)
                && text.0 != label
            {
                text.0 = label.clone();
            }
        }
    }
    let machine = &quality.machine;
    let line = if machine.gpu.is_empty() {
        "AUTO fits the game to this machine.".to_owned()
    } else {
        format!(
            "This machine: {}, {} threads. It holds {}; AUTO draws less if a race runs slow.",
            machine.gpu,
            machine.threads,
            quality.detected.name()
        )
    };
    for mut text in &mut lines {
        if text.0 != line {
            text.0 = line.clone();
        }
    }
}
