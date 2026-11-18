//! The player's rank and level, small and bright in the top right corner of every menu, the
//! editor and a lobby between races (since v1.0.0-rc2, the user's word: "Tu feras que son rank (ou
//! unranked) et son niveau, soient souvent bien visibles et jolis").
//!
//! A pill: a gem in the tier's color, glowing, then the tier and the rating, or UNRANKED for a
//! guest and for an account still playing its placement games (with how many are left); then LVL
//! and the level, over a thin bar of how far the next level is. Nothing until the server has said
//! who the player is.

use bevy::prelude::*;
use star_racer_protocol::ranked::{PLACEMENT_GAMES, Tier};
use star_racer_protocol::{LobbyPhase, rewards};

use super::safe_area::SafeInset;
use super::theme::{self, Fonts};
use super::touch::Touchscreen;
use crate::lobby::{CurrentLobby, Ladder, PlayerProgress};
use crate::net::Connection;
use crate::screen::Screen;

pub struct PlayerCardPlugin;

impl Plugin for PlayerCardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn).add_systems(Update, update);
    }
}

#[derive(Component)]
struct Card;

/// The full-screen root the card is laid out in, which says where it sits.
#[derive(Component)]
struct CardRoot;

#[derive(Component)]
struct Gem;

#[derive(Component)]
struct RankText;

#[derive(Component)]
struct LevelText;

#[derive(Component)]
struct LevelFill;

/// Each tier's color.
fn tier_color(tier: Tier) -> Color {
    match tier {
        Tier::Bronze => Color::srgb(0.85, 0.52, 0.28),
        Tier::Silver => Color::srgb(0.78, 0.82, 0.9),
        Tier::Gold => Color::srgb(1.0, 0.8, 0.25),
        Tier::Platinum => Color::srgb(0.3, 0.92, 0.85),
        Tier::Diamond => Color::srgb(0.45, 0.7, 1.0),
        Tier::Champion => Color::srgb(1.0, 0.35, 0.85),
    }
}

/// What the card says of the rank: its words and color.
fn rank_words(guest: bool, ladder: &Ladder) -> (String, Color) {
    let unranked = Color::srgb(0.55, 0.6, 0.7);
    if guest {
        return ("UNRANKED".into(), unranked);
    }
    match ladder.profile {
        Some(profile) if profile.placed() => {
            let tier = Tier::of(profile.rating);
            let place = profile
                .place
                .map_or(String::new(), |place| format!("  #{place}"));
            (format!("{}  {}{place}", tier.name(), profile.rating), tier_color(tier))
        }
        Some(profile) => (
            format!("UNRANKED  {}/{PLACEMENT_GAMES}", profile.games),
            unranked,
        ),
        None => ("UNRANKED".into(), unranked),
    }
}

fn spawn(mut commands: Commands, fonts: Res<Fonts>, touchscreen: Res<Touchscreen>) {
    let pick = |desk: f32, phone: f32| touchscreen.pick(desk, phone);
    commands
        .spawn((
            CardRoot,
            SafeInset,
            Pickable::IGNORE,
            // Over the menus, under the dialogs they open.
            GlobalZIndex(5),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::FlexStart,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                Card,
                Pickable::IGNORE,
                Node {
                    padding: UiRect::axes(px(pick(16.0, 10.0)), px(pick(9.0, 6.0))),
                    column_gap: px(pick(12.0, 8.0)),
                    align_items: AlignItems::Center,
                    border: px(1.5).all(),
                    border_radius: BorderRadius::MAX,
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.02, 0.025, 0.045, 0.85)),
                BorderColor::all(theme::PANEL_BORDER),
                BoxShadow::new(Color::BLACK.with_alpha(0.6), px(0), px(4), px(0), px(14)),
            ))
            .with_children(|card| {
                let gem = pick(14.0, 10.0);
                card.spawn((
                    Gem,
                    Node {
                        width: px(gem),
                        height: px(gem),
                        border: px(2).all(),
                        ..default()
                    },
                    UiTransform::from_rotation(Rot2::degrees(45.0)),
                    BackgroundColor(theme::TEXT_DIM),
                    BorderColor::all(Color::WHITE.with_alpha(0.7)),
                    BoxShadow(Vec::new()),
                ));
                card.spawn((
                    RankText,
                    Text::default(),
                    fonts.caption(pick(14.0, 11.0)),
                    theme::CAPTION_SPACING,
                    TextColor(theme::TEXT),
                ));
                card.spawn((
                    Node {
                        width: px(1.5),
                        height: px(pick(20.0, 14.0)),
                        ..default()
                    },
                    BackgroundColor(theme::PANEL_BORDER),
                ));
                card.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3),
                    ..default()
                })
                .with_children(|level| {
                    level.spawn((
                        LevelText,
                        Text::default(),
                        fonts.caption(pick(14.0, 11.0)),
                        theme::CAPTION_SPACING,
                        TextColor(theme::TEXT),
                    ));
                    level
                        .spawn((
                            Node {
                                width: px(pick(64.0, 44.0)),
                                height: px(3),
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE.with_alpha(0.12)),
                        ))
                        .with_child((
                            LevelFill,
                            Node {
                                width: percent(0),
                                height: percent(100),
                                border_radius: BorderRadius::MAX,
                                ..default()
                            },
                            BackgroundColor(theme::NEON),
                            BoxShadow::new(theme::NEON.with_alpha(0.8), px(0), px(0), px(0), px(6)),
                        ));
                });
            });
        });
}

/// Where the card shows: the menus, the garage, the editor, and a lobby while nobody races.
fn shown(screen: Screen, current: &CurrentLobby) -> bool {
    match screen {
        Screen::Login | Screen::Lobbies | Screen::Garage | Screen::Editor => true,
        Screen::Lobby => current
            .0
            .as_ref()
            .and_then(|membership| membership.state.as_ref())
            .is_some_and(|state| !matches!(state.phase, LobbyPhase::Racing { .. })),
    }
}

#[allow(clippy::too_many_arguments)]
fn update(
    screen: Res<State<Screen>>,
    connection: Res<Connection>,
    ladder: Res<Ladder>,
    progress: Res<PlayerProgress>,
    current: Res<CurrentLobby>,
    mut cards: Query<&mut Node, (With<Card>, Without<LevelFill>, Without<CardRoot>)>,
    mut gems: Query<(&mut BackgroundColor, &mut BoxShadow), (With<Gem>, Without<LevelFill>)>,
    mut ranks: Query<(&mut Text, &mut TextColor), (With<RankText>, Without<LevelText>)>,
    mut levels: Query<&mut Text, (With<LevelText>, Without<RankText>)>,
    mut fills: Query<&mut Node, (With<LevelFill>, Without<Card>, Without<CardRoot>)>,
    mut roots: Query<&mut Node, (With<CardRoot>, Without<Card>, Without<LevelFill>)>,
    touchscreen: Res<Touchscreen>,
) {
    let guest = match &*connection {
        Connection::Online { guest, .. } => Some(*guest),
        _ => None,
    };
    // Where it sits: the top right corner, which the first screen, the garage and the editor leave
    // free; the middle of the top in the lobby list, whose corner holds its buttons; under the
    // lobby's name in a lobby, whose corner holds the standings.
    let phone = touchscreen.0;
    let (justify, padding) = match *screen.get() {
        // On a phone the title and the name leave room only just right of the title.
        Screen::Lobbies if phone => (
            JustifyContent::FlexStart,
            UiRect {
                left: px(165.0),
                top: px(10.0),
                ..default()
            },
        ),
        Screen::Lobbies => (JustifyContent::Center, UiRect::top(px(40.0))),
        Screen::Lobby => (
            JustifyContent::FlexStart,
            UiRect {
                left: px(if phone { 20.0 } else { 26.0 }),
                top: px(if phone { 58.0 } else { 86.0 }),
                ..default()
            },
        ),
        _ => (
            JustifyContent::FlexEnd,
            UiRect {
                right: px(if phone { 10.0 } else { 22.0 }),
                top: px(if phone { 8.0 } else { 18.0 }),
                ..default()
            },
        ),
    };
    for mut root in &mut roots {
        if root.justify_content != justify || root.padding != padding {
            root.justify_content = justify;
            root.padding = padding;
        }
    }
    let known = guest.is_some() && progress.xp.is_some();
    let display = if known && shown(*screen.get(), &current) {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut cards {
        if node.display != display {
            node.display = display;
        }
    }
    if !known {
        return;
    }
    let (words, color) = rank_words(guest.unwrap_or(true), &ladder);
    for (mut text, mut text_color) in &mut ranks {
        if text.0 != words {
            text.0 = words.clone();
        }
        text_color.set_if_neq(TextColor(color));
    }
    for (mut background, mut shadow) in &mut gems {
        background.set_if_neq(BackgroundColor(color));
        let glow = BoxShadow::new(color.with_alpha(0.85), px(0), px(0), px(1), px(10));
        shadow.set_if_neq(glow);
    }
    let xp = progress.xp.unwrap_or(0);
    let level = format!("LVL {}", rewards::level(xp));
    for mut text in &mut levels {
        if text.0 != level {
            text.0 = level.clone();
        }
    }
    let share = percent(rewards::level_share(xp).clamp(0.0, 1.0) * 100.0);
    for mut node in &mut fills {
        if node.width != share {
            node.width = share;
        }
    }
}

#[cfg(test)]
mod tests {
    use star_racer_protocol::ranked::RankedProfile;

    use super::*;

    #[test]
    fn the_card_says_unranked_until_placed() {
        let mut ladder = Ladder::default();
        assert_eq!(rank_words(true, &ladder).0, "UNRANKED");
        ladder.profile = Some(RankedProfile {
            rating: 1000,
            games: 2,
            wins: 1,
            place: None,
        });
        assert_eq!(rank_words(false, &ladder).0, "UNRANKED  2/5");
        ladder.profile = Some(RankedProfile {
            rating: 1180,
            games: 9,
            wins: 4,
            place: Some(3),
        });
        assert_eq!(rank_words(false, &ladder), ("GOLD  1180  #3".into(), tier_color(Tier::Gold)));
        // A guest is unranked whatever the ladder says.
        assert_eq!(rank_words(true, &ladder).0, "UNRANKED");
    }
}
