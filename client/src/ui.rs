//! The game's interface: the login screen, the lobby browser and its creation dialog, the race HUD,
//! the settings, and what they are built from.

mod back;
mod backdrop;
mod create_lobby;
mod garage;
mod gauges;
pub mod hud;
pub mod leaderboard;
mod lobbies;
mod login;
pub mod navigation;
pub mod safe_area;
mod settings;
pub mod theme;
pub mod thumbnail;
pub mod touch;
pub mod widgets;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;

/// The interface is laid out for a window this many logical pixels high, and scaled to the actual
/// height, within these bounds.
const DESIGN_HEIGHT: f32 = 720.0;
const MIN_SCALE: f32 = 0.75;
const MAX_SCALE: f32 = 2.5;
/// A phone in landscape is a short window held close, not a tall one across a desk. The same
/// screens are laid out for a window this high instead, so everything on them comes out larger
/// against the glass, and they may shrink further before they stop fitting at all. A phone's
/// browser in landscape leaves the page 330 to 360 pixels of its 375 to 412 while its address bar
/// shows, so at 400 a caption is still 9 to 10 pixels high and a thumb target 7 mm.
const TOUCH_DESIGN_HEIGHT: f32 = 400.0;
const TOUCH_MIN_SCALE: f32 = 0.55;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            theme::ThemePlugin,
            widgets::WidgetsPlugin,
            navigation::NavigationPlugin,
            backdrop::BackdropPlugin,
            login::LoginPlugin,
            lobbies::LobbiesPlugin,
            create_lobby::CreateLobbyPlugin,
            hud::HudPlugin,
            touch::TouchPlugin,
            back::BackPlugin,
            leaderboard::LeaderboardPlugin,
            settings::SettingsPlugin,
            garage::GaragePlugin,
        ))
        .add_systems(PreUpdate, scale_to_window)
        .add_systems(PostStartup, spawn_version_tag)
        .add_systems(Update, show_version_tag);
    }
}

/// The game's version as players read it: `v0.1.78`.
pub fn version() -> String {
    format!("v{}", env!("CARGO_PKG_VERSION"))
}

/// What the corner of every screen says the game is: an alpha, and which version.
pub fn version_tag() -> String {
    format!("STAR RACER · ALPHA {}", version())
}

/// The version, small in a corner of every screen but the race, where the HUD says it beside the
/// frame rate (since v0.1.78).
#[derive(Component)]
struct VersionTag;

fn spawn_version_tag(mut commands: Commands, fonts: Res<theme::Fonts>) {
    commands.spawn((
        VersionTag,
        safe_area::SafeInset,
        Pickable::IGNORE,
        GlobalZIndex(50),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            ..default()
        },
        children![(
            Text::new(version_tag()),
            fonts.caption(11.0),
            theme::CAPTION_SPACING,
            TextColor(theme::TEXT_FAINT),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: px(12),
                bottom: px(8),
                ..default()
            },
        )],
    ));
}

fn show_version_tag(
    screen: Res<State<crate::screen::Screen>>,
    mut tags: Query<&mut Visibility, With<VersionTag>>,
) {
    let shown = if *screen.get() == crate::screen::Screen::Lobby {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut tags {
        visibility.set_if_neq(shown);
    }
}

/// Keeps the interface the same size relative to the window, from a phone in landscape to a 4K
/// screen.
fn scale_to_window(
    windows: Query<&Window, With<PrimaryWindow>>,
    touchscreen: Res<touch::Touchscreen>,
    mut scale: ResMut<UiScale>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let design = touchscreen.pick(DESIGN_HEIGHT, TOUCH_DESIGN_HEIGHT);
    let smallest = touchscreen.pick(MIN_SCALE, TOUCH_MIN_SCALE);
    let wanted = (window.height() / design).clamp(smallest, MAX_SCALE);
    if (scale.0 - wanted).abs() > 0.01 {
        scale.0 = wanted;
    }
}
