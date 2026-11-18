//! The editor's panel, down the left of the screen: the circuit's name, the tabs, what can be added
//! to the open one, the list of its parts, the numbers of the one selected with buttons to step
//! them, what the editor has to say, and its actions. It is built again whenever something it
//! shows changes; only the name is typed, everything else is pressed.

use bevy::input_focus::InputFocus;
use bevy::picking::events::{Pointer, Scroll};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use star_racer_protocol::{MAX_LOBBY_NAME_CHARS, TRY_TRACK};

use super::model::{self, Item, New, Tab};
use super::{Editor, load, publish_draft, try_draft};
use crate::lobby::Tracks;
use crate::net::Network;
use crate::screen::{AfterLobby, Screen};
use crate::ui::theme::{self, Fonts};
use crate::ui::touch::Touchscreen;
use crate::ui::widgets::{ButtonKind, button_of_size, panel, text_field};

/// How wide the panel is, in logical pixels: what lies right of it is the map.
pub fn width(touchscreen: Touchscreen) -> f32 {
    touchscreen.pick(440.0, 320.0)
}

/// The panel's root.
#[derive(Component)]
pub(super) struct PanelRoot;

/// The circuit's name, typed.
#[derive(Component)]
pub(super) struct NameField;

#[derive(Component, Clone, Copy)]
struct TabButton(Tab);

#[derive(Component, Clone, Copy)]
struct AddButton(New);

#[derive(Component, Clone, Copy)]
struct ItemButton(Item);

/// Steps number `field` of the selected part by `steps` of its step.
#[derive(Component, Clone, Copy)]
struct FieldButton {
    field: usize,
    steps: f32,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Action {
    Delete,
    New,
    Try,
    Publish,
    Back,
    Load,
    /// Steps the circuit LOAD would take.
    LoadStep(i32),
    /// Steps the radius of the editor's closing turns.
    Closing(i32),
    Undo,
    Redo,
    /// Turns the selected turn the other way.
    Flip,
}

/// The list of parts, which scrolls.
#[derive(Component)]
struct PartList;

pub fn observe(app: &mut App) {
    app.add_observer(on_tab)
        .add_observer(on_add)
        .add_observer(on_item)
        .add_observer(on_field)
        .add_observer(on_action)
        .add_observer(scroll_list);
}

pub fn spawn(mut commands: Commands, mut editor: ResMut<Editor>, touchscreen: Res<Touchscreen>) {
    commands.spawn((
        PanelRoot,
        DespawnOnExit(Screen::Editor),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            bottom: px(0),
            width: px(width(*touchscreen)),
            flex_direction: FlexDirection::Column,
            padding: px(touchscreen.pick(16.0, 8.0)).all(),
            row_gap: px(touchscreen.pick(8.0, 4.0)),
            border: UiRect::right(px(1)),
            ..default()
        },
        panel(),
    ));
    editor.panel_dirty = true;
}

/// The name typed goes into the draft as it is typed.
pub(super) fn read_name(
    mut editor: ResMut<Editor>,
    fields: Query<&EditableText, (With<NameField>, Changed<EditableText>)>,
) {
    for field in &fields {
        let name = field.value().to_string();
        if editor.draft.name != name {
            editor.draft.name = name;
        }
    }
}

/// Builds the panel again when what it shows changed.
pub(super) fn refresh(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    fonts: Res<Fonts>,
    tracks: Res<Tracks>,
    touchscreen: Res<Touchscreen>,
    roots: Query<Entity, With<PanelRoot>>,
    mut focus: ResMut<InputFocus>,
) {
    if !editor.panel_dirty {
        return;
    }
    editor.panel_dirty = false;
    let Ok(root) = roots.single() else {
        return;
    };
    focus.clear();
    commands.entity(root).despawn_children();
    let small = touchscreen.0;
    let size = |desk: f32, phone: f32| if small { phone } else { desk };
    let editor = &*editor;
    commands.entity(root).with_children(|panel| {
        panel.spawn((
            Text::new("EDITOR"),
            fonts.display(size(34.0, 22.0)),
            TextColor(theme::TEXT),
            TextShadow {
                offset: Vec2::new(3.0, 3.0),
                color: theme::NEON.with_alpha(0.8),
            },
        ));
        panel
            .spawn(Node {
                column_gap: px(8),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Text::new("NAME"),
                    fonts.caption(12.0),
                    theme::CAPTION_SPACING,
                    TextColor(theme::TEXT_FAINT),
                ));
                row.spawn((
                    NameField,
                    text_field(&fonts, "Circuit name", &editor.draft.name, MAX_LOBBY_NAME_CHARS),
                ));
            });

        // The tabs.
        panel
            .spawn(Node {
                column_gap: px(4),
                ..default()
            })
            .with_children(|row| {
                for tab in Tab::ALL {
                    let kind = if tab == editor.tab {
                        ButtonKind::Secondary
                    } else {
                        ButtonKind::Quiet
                    };
                    row.spawn((TabButton(tab), small_button(&fonts, kind, tab.name(), size(13.0, 11.0))));
                }
            });

        // What the tab is for, in a line.
        panel.spawn((
            Text::new(tab_help(editor.tab)),
            fonts.text(size(13.0, 11.0), 400),
            TextColor(theme::TEXT_DIM),
        ));

        // What can be added to it, at the cursor.
        panel
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(4),
                row_gap: px(4),
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Text::new("ADD"),
                    fonts.caption(11.0),
                    theme::CAPTION_SPACING,
                    TextColor(theme::TEXT_FAINT),
                ));
                for new in New::of(editor.tab) {
                    row.spawn((
                        AddButton(new),
                        small_button(&fonts, ButtonKind::Quiet, new.name(), size(11.0, 10.0)),
                    ));
                }
            });

        // The parts of the tab.
        panel
            .spawn((
                PartList,
                Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    flex_basis: px(0),
                    min_height: px(60),
                    overflow: Overflow::scroll_y(),
                    row_gap: px(2),
                    ..default()
                },
                ScrollPosition::default(),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
            ))
            .with_children(|list| {
                let items = model::items(&editor.draft, editor.tab);
                if items.is_empty() {
                    list.spawn((
                        Text::new("Nothing yet: click the map, then ADD."),
                        fonts.text(size(13.0, 11.0), 400),
                        TextColor(theme::TEXT_FAINT),
                        Node {
                            margin: px(8).all(),
                            ..default()
                        },
                    ));
                }
                for item in items {
                    let chosen = editor.selected == Some(item);
                    list.spawn((
                        ItemButton(item),
                        bevy::ui_widgets::Button,
                        Node {
                            padding: UiRect::axes(px(10), px(size(5.0, 3.0))),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        BackgroundColor(if chosen {
                            theme::NEON.with_alpha(0.3)
                        } else {
                            Color::NONE
                        }),
                    ))
                    .with_child((
                        Text::new(model::describe(&editor.draft, item)),
                        fonts.text(size(14.0, 11.0), if chosen { 600 } else { 400 }),
                        TextColor(if chosen { theme::TEXT } else { theme::TEXT_DIM }),
                        Pickable::IGNORE,
                    ));
                }
            });

        // The selected part's numbers.
        if let Some(item) = editor.selected {
            for (index, field) in model::fields(&editor.draft, item).iter().enumerate() {
                panel
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(field.label),
                            fonts.caption(11.0),
                            theme::CAPTION_SPACING,
                            TextColor(theme::TEXT_FAINT),
                            Node {
                                width: px(size(110.0, 80.0)),
                                ..default()
                            },
                        ));
                        let side = field.unit == model::Unit::Side;
                        let steps: &[(f32, &str)] = if side {
                            &[(-1.0, "‹"), (1.0, "›")]
                        } else {
                            &[(-10.0, "«"), (-1.0, "‹"), (1.0, "›"), (10.0, "»")]
                        };
                        for (place, (steps, label)) in steps.iter().enumerate() {
                            if place == steps_half(side) {
                                row.spawn((
                                    Text::new(field.text()),
                                    fonts.text(size(16.0, 12.0), 600),
                                    TextColor(theme::TEXT),
                                    TextLayout::justify(Justify::Center),
                                    Node {
                                        width: px(size(86.0, 64.0)),
                                        ..default()
                                    },
                                ));
                            }
                            row.spawn((
                                FieldButton {
                                    field: index,
                                    steps: *steps,
                                },
                                small_button(&fonts, ButtonKind::Quiet, *label, size(16.0, 13.0)),
                            ));
                        }
                    });
            }
            if item != Item::Road {
                let turn = matches!(item, Item::Segment(index)
                    if matches!(editor.draft.segments.get(index), Some(star_racer_sim::track::Segment::Turn { .. })));
                panel
                    .spawn(Node {
                        column_gap: px(6),
                        ..default()
                    })
                    .with_children(|row| {
                        if turn {
                            row.spawn((
                                Action::Flip,
                                small_button(&fonts, ButtonKind::Quiet, "FLIP", size(12.0, 10.0)),
                            ));
                        }
                        row.spawn((
                            Action::Delete,
                            small_button(&fonts, ButtonKind::Quiet, "DELETE", size(12.0, 10.0)),
                        ));
                    });
            } else {
                // The road as a whole also has the editor's own closing turns.
                panel
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: px(4),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new("CLOSING TURNS"),
                            fonts.caption(11.0),
                            theme::CAPTION_SPACING,
                            TextColor(theme::TEXT_FAINT),
                            Node {
                                width: px(size(110.0, 80.0)),
                                ..default()
                            },
                        ));
                        row.spawn((
                            Action::Closing(-1),
                            small_button(&fonts, ButtonKind::Quiet, "‹", size(16.0, 13.0)),
                        ));
                        row.spawn((
                            Text::new(format!("R {:.0} m", editor.closing_radius)),
                            fonts.text(size(16.0, 12.0), 600),
                            TextColor(theme::TEXT),
                            TextLayout::justify(Justify::Center),
                            Node {
                                width: px(size(86.0, 64.0)),
                                ..default()
                            },
                        ));
                        row.spawn((
                            Action::Closing(1),
                            small_button(&fonts, ButtonKind::Quiet, "›", size(16.0, 13.0)),
                        ));
                    });
            }
        }

        // What the editor has to say: the last answer, then whether the circuit builds.
        if let Some((text, bad)) = &editor.message {
            panel.spawn((
                Text::new(text.clone()),
                fonts.text(size(13.0, 11.0), 500),
                TextColor(if *bad { theme::RED } else { theme::GREEN }),
            ));
        }
        let (status, color) = match &editor.built {
            Ok(built) => (
                format!(
                    "{:.2} km  ·  {} segments, {} added to close it",
                    built.track.length() / 1000.0,
                    built.description.segments.len(),
                    built.closing
                ),
                theme::TEXT_DIM,
            ),
            Err(why) => (format!("Does not build: {why}"), theme::RED),
        };
        panel.spawn((
            Text::new(status),
            fonts.text(size(13.0, 11.0), 400),
            TextColor(color),
        ));

        // Loading one of the server's circuits.
        let names: Vec<&str> = tracks
            .0
            .iter()
            .filter(|(key, _)| key.as_str() != TRY_TRACK)
            .map(|(_, entry)| entry.description.name.as_str())
            .collect();
        if !names.is_empty() {
            let chosen = names[editor.load_choice % names.len()];
            panel
                .spawn(Node {
                    align_items: AlignItems::Center,
                    column_gap: px(4),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Action::Load,
                        small_button(&fonts, ButtonKind::Quiet, "LOAD", size(12.0, 10.0)),
                    ));
                    row.spawn((
                        Action::LoadStep(-1),
                        small_button(&fonts, ButtonKind::Quiet, "‹", size(16.0, 13.0)),
                    ));
                    row.spawn((
                        Text::new(chosen.to_uppercase()),
                        fonts.text(size(14.0, 11.0), 600),
                        TextColor(theme::TEXT_DIM),
                        TextLayout::justify(Justify::Center),
                        Node {
                            width: px(size(170.0, 110.0)),
                            ..default()
                        },
                    ));
                    row.spawn((
                        Action::LoadStep(1),
                        small_button(&fonts, ButtonKind::Quiet, "›", size(16.0, 13.0)),
                    ));
                });
        }

        panel
            .spawn(Node {
                column_gap: px(6),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    Action::Try,
                    small_button(&fonts, ButtonKind::Primary, "TRY", size(15.0, 12.0)),
                ));
                row.spawn((
                    Action::Publish,
                    small_button(&fonts, ButtonKind::Secondary, "PUBLISH", size(15.0, 12.0)),
                ));
                for (action, label, can) in [
                    (Action::Undo, "UNDO", editor.can_undo()),
                    (Action::Redo, "REDO", editor.can_redo()),
                ] {
                    let mut button =
                        row.spawn((action, small_button(&fonts, ButtonKind::Quiet, label, size(13.0, 11.0))));
                    if !can {
                        button.insert(InteractionDisabled);
                    }
                }
                row.spawn((
                    Action::New,
                    small_button(&fonts, ButtonKind::Quiet, "NEW", size(13.0, 11.0)),
                ));
                row.spawn((
                    Action::Back,
                    small_button(&fonts, ButtonKind::Quiet, "BACK", size(13.0, 11.0)),
                ));
            });
    });
}

/// Where the value stands among a field's buttons: between the steps down and up.
fn steps_half(side: bool) -> usize {
    if side { 1 } else { 2 }
}

fn small_button(
    fonts: &Fonts,
    kind: ButtonKind,
    label: impl Into<String>,
    size: f32,
) -> impl Bundle {
    (button_of_size(fonts, kind, label, size), SmallButton(size))
}

fn on_tab(activate: On<Activate>, tabs: Query<&TabButton>, editor: Option<ResMut<Editor>>) {
    let Some(mut editor) = editor else {
        return;
    };
    if let Ok(tab) = tabs.get(activate.entity) {
        editor.tab = tab.0;
        editor.selected = (tab.0 == Tab::Road).then_some(Item::Road);
        editor.panel_dirty = true;
        editor.overlay_dirty = true;
    }
}

fn on_add(activate: On<Activate>, adds: Query<&AddButton>, editor: Option<ResMut<Editor>>) {
    let Some(mut editor) = editor else {
        return;
    };
    let Ok(add) = adds.get(activate.entity) else {
        return;
    };
    let after = match editor.selected {
        Some(Item::Segment(index)) => Some(index),
        _ => None,
    };
    let at = editor.cursor.unwrap_or(100.0);
    let item = model::add(&mut editor.draft, add.0, at, after);
    editor.selected = Some(item);
    editor.changed();
}

fn on_item(activate: On<Activate>, items: Query<&ItemButton>, editor: Option<ResMut<Editor>>) {
    let Some(mut editor) = editor else {
        return;
    };
    if let Ok(item) = items.get(activate.entity) {
        editor.selected = Some(item.0);
        if let Some(at) = model::at(&editor.draft, item.0) {
            editor.cursor = Some(at);
        }
        editor.panel_dirty = true;
        editor.overlay_dirty = true;
    }
}

fn on_field(activate: On<Activate>, buttons: Query<&FieldButton>, editor: Option<ResMut<Editor>>) {
    let Some(mut editor) = editor else {
        return;
    };
    let (Ok(button), Some(item)) = (buttons.get(activate.entity), editor.selected) else {
        return;
    };
    let Some(field) = model::fields(&editor.draft, item).get(button.field).cloned() else {
        return;
    };
    let value = field.value + field.step * button.steps;
    let item = model::set(&mut editor.draft, item, button.field, value);
    editor.selected = Some(item);
    editor.changed();
}

#[allow(clippy::too_many_arguments)]
fn on_action(
    activate: On<Activate>,
    actions: Query<&Action>,
    editor: Option<ResMut<Editor>>,
    network: Res<Network>,
    mut tracks: ResMut<Tracks>,
    mut images: ResMut<Assets<Image>>,
    mut after: ResMut<AfterLobby>,
    mut next: ResMut<NextState<Screen>>,
) {
    let Ok(action) = actions.get(activate.entity) else {
        return;
    };
    let Some(mut editor) = editor else {
        return;
    };
    match *action {
        Action::Delete => {
            if let Some(item) = editor.selected.take() {
                model::remove(&mut editor.draft, item);
                editor.changed();
            }
        }
        Action::New => {
            editor.draft = model::starter();
            editor.selected = None;
            editor.cursor = None;
            editor.say("A new circuit.", false);
            editor.changed();
        }
        Action::Try => try_draft(&mut editor, &network, &mut tracks, &mut images, &mut after),
        Action::Publish => publish_draft(&mut editor, &network),
        Action::Back => next.set(Screen::Login),
        Action::Load => {
            let chosen = tracks
                .0
                .iter()
                .filter(|(key, _)| key.as_str() != TRY_TRACK)
                .nth(editor.load_choice % tracks.0.len().max(1))
                .map(|(_, entry)| entry.description.clone());
            if let Some(description) = chosen {
                load(&mut editor, &description);
            }
        }
        Action::LoadStep(step) => {
            let count = tracks.0.keys().filter(|key| key.as_str() != TRY_TRACK).count().max(1);
            editor.load_choice =
                (editor.load_choice as i32 + step).rem_euclid(count as i32) as usize;
            editor.panel_dirty = true;
        }
        Action::Undo => editor.undo(),
        Action::Redo => editor.redo(),
        Action::Flip => {
            if let Some(Item::Segment(index)) = editor.selected
                && let Some(star_racer_sim::track::Segment::Turn { angle, .. }) =
                    editor.draft.segments.get_mut(index)
            {
                *angle = -*angle;
                editor.changed();
            }
        }
        Action::Closing(step) => {
            editor.closing_radius = (editor.closing_radius + 5.0 * step as f32).clamp(20.0, 500.0);
            editor.changed();
        }
    }
}

/// The wheel over the list of parts scrolls it.
fn scroll_list(
    scroll: On<Pointer<Scroll>>,
    ancestors: Query<&ChildOf>,
    mut lists: Query<(&ComputedNode, &mut ScrollPosition), With<PartList>>,
) {
    if scroll.entity != scroll.original_event_target() {
        return;
    }
    let mut at = Some(scroll.entity);
    while let Some(entity) = at {
        if let Ok((computed, mut position)) = lists.get_mut(entity) {
            let overflow =
                ((computed.content_size.y - computed.size.y) * computed.inverse_scale_factor).max(0.0);
            position.y = (position.y - scroll.event.y * 24.0).clamp(0.0, overflow);
            return;
        }
        at = ancestors.get(entity).ok().map(ChildOf::parent);
    }
}

/// A button of the panel, smaller than the menus' own: its text size, which its padding follows.
#[derive(Component)]
pub(super) struct SmallButton(f32);

/// Gives the panel's buttons their smaller padding, as they appear.
pub(super) fn size_buttons(mut buttons: Query<(&SmallButton, &mut Node), Added<SmallButton>>) {
    for (button, mut node) in &mut buttons {
        node.padding = UiRect::axes(px(button.0 * 0.7), px(button.0 * 0.4));
        node.justify_content = JustifyContent::Center;
        node.align_items = AlignItems::Center;
        node.width = Val::Auto;
        node.min_width = Val::Auto;
    }
}

/// What each tab is for, said under the tabs (since v1.0.0-rc5, the user's word: "Améliore
/// l'éditeur, c'est pas intuitif du tout").
fn tab_help(tab: Tab) -> &'static str {
    match tab {
        Tab::Road => {
            "The road from the start line, segment by segment. ADD puts a segment after the one \
             selected; the editor closes the road back to the line by itself (the pale line)."
        }
        Tab::Relief => {
            "Heights the road climbs and dives through. Click the road where it should be high \
             or low, then ADD a HEIGHT."
        }
        Tab::Game => {
            "What is raced: gaps to jump, missing walls, narrow stretches, speed pads and \
             checkpoints. Click the road, then ADD."
        }
        Tab::Scenery => {
            "What stands beside the road. Click the road, then ADD; SIDE and OFF THE WALL \
             place it."
        }
    }
}
