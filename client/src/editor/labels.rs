//! What the editor writes over its map (since v1.0.0-rc5, the user's word "Améliore l'éditeur,
//! c'est pas intuitif du tout"): a line saying what the mouse and the keys do, and each segment's
//! number on the road, as the panel's list numbers them, so a row of the list and a stretch of the
//! road are seen to be the same.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use super::model::{Item, Tab};
use super::{Editor, panel};
use crate::camera::MainCamera;
use crate::screen::Screen;
use crate::ui::theme::{self, Fonts};
use crate::ui::touch::Touchscreen;
use crate::world;

#[derive(Component)]
pub(super) struct LabelRoot;

/// A segment's number over the road.
#[derive(Component)]
pub(super) struct SegmentLabel(usize);

const HELP: &str = "CLICK the road to place the cursor and select  ·  DRAG a part to slide it \
                    along  ·  RIGHT-DRAG or WASD to move  ·  WHEEL to zoom  ·  CTRL+Z undo  ·  \
                    DEL delete";

pub(super) fn spawn(mut commands: Commands, fonts: Res<Fonts>, touchscreen: Res<Touchscreen>) {
    let left = panel::width(*touchscreen);
    commands
        .spawn((
            LabelRoot,
            DespawnOnExit(Screen::Editor),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: px(left),
                right: px(0),
                top: px(0),
                bottom: px(0),
                ..default()
            },
        ))
        .with_child((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                right: px(16),
                // At the bottom: the player's card holds the top right corner.
                bottom: px(12),
                padding: UiRect::axes(px(14), px(8)),
                border: px(1).all(),
                border_radius: BorderRadius::all(px(theme::RADIUS_SMALL)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.03, 0.06, 0.8)),
            BorderColor::all(theme::PANEL_BORDER),
            children![(
                Text::new(HELP),
                fonts.text(13.0, 500),
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            )],
        ));
}

/// Keeps one number per segment of the draft, over the middle of its stretch of road, bright on
/// the road tab and for the segment selected.
pub(super) fn place(
    mut commands: Commands,
    editor: Res<Editor>,
    fonts: Res<Fonts>,
    touchscreen: Res<Touchscreen>,
    roots: Query<Entity, With<LabelRoot>>,
    mut labels: Query<(Entity, &SegmentLabel, &mut Node, &mut Visibility, &mut TextColor)>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    let count = editor.draft.segments.len();
    let mut seen = vec![false; count];
    for (entity, label, ..) in &labels {
        if label.0 >= count {
            commands.entity(entity).despawn();
        } else {
            seen[label.0] = true;
        }
    }
    for (index, _) in seen.iter().enumerate().filter(|(_, seen)| !**seen) {
        commands.entity(root).with_child((
            SegmentLabel(index),
            Pickable::IGNORE,
            Text::new(format!("{}", index + 1)),
            fonts.caption(13.0),
            TextColor(theme::TEXT),
            theme::TEXT_SHADOW,
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Visibility::Hidden,
        ));
    }
    let (Ok((camera, transform)), Ok(built), Ok(window)) =
        (cameras.single(), &editor.built, windows.single())
    else {
        for (_, _, _, mut visibility, _) in &mut labels {
            visibility.set_if_neq(Visibility::Hidden);
        }
        return;
    };
    let left = panel::width(*touchscreen);
    let track = &built.track;
    for (_, label, mut node, mut visibility, mut color) in &mut labels {
        if label.0 >= count {
            continue;
        }
        let (from, to) = editor.segment_span(label.0);
        let point = track.point_at(((from + to) / 2.0).rem_euclid(track.length()));
        let world = world::position(point.position, point.height + 3.0);
        let Ok(at) = camera.world_to_viewport(transform, world) else {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        };
        let on_map = at.x > left + 10.0 && at.x < window.width() - 10.0 && at.y > 50.0;
        visibility.set_if_neq(if on_map {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        node.left = px(at.x - left - 6.0);
        node.top = px(at.y - 10.0);
        let wanted = if editor.selected == Some(Item::Segment(label.0)) {
            theme::NEON
        } else if editor.tab == Tab::Road {
            theme::TEXT
        } else {
            theme::TEXT_FAINT
        };
        color.set_if_neq(TextColor(wanted));
    }
}
