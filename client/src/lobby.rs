//! What the client knows of lobbies: the tracks they race on, the lobby list, and the lobby the
//! player is in. Every screen reads these; only this module writes them from the server's messages.

use std::collections::BTreeMap;
use std::sync::Arc;

use bevy::prelude::*;
use star_racer_protocol::ranked::{LeaderboardEntry, RankedProfile};
use star_racer_protocol::rewards::{self, Reward};
use star_racer_protocol::{
    CarId, ClientMessage, JoinError, LobbyId, LobbyPhase, LobbyPlayer, LobbySettings, LobbyState,
    LobbySummary, OnlinePlayer, PlayerStatus, RatingChange,
};
use star_racer_sim::track::{Track, TrackDescription};

use crate::net::{Connection, NetEvent, Network};
use crate::race::SnapshotBuffer;
use crate::screen::{AfterLobby, Screen};
use crate::settings::Script;
use crate::ui::thumbnail;

pub struct LobbyPlugin;

impl Plugin for LobbyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Tracks>()
            .init_resource::<LobbyList>()
            .init_resource::<OnlinePlayers>()
            .init_resource::<CurrentLobby>()
            .init_resource::<Ladder>()
            .init_resource::<PlayerProgress>()
            .init_resource::<Notice>()
            .add_systems(
                PreUpdate,
                (receive_lobby_events, follow_script, end_round)
                    .chain()
                    .in_set(LobbyUpdate),
            );
    }
}

/// Updates the lobby resources from the server's messages. Runs before the race and the UI.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LobbyUpdate;

/// Every track of the server, by key.
#[derive(Resource, Default)]
pub struct Tracks(pub BTreeMap<String, TrackEntry>);

pub struct TrackEntry {
    pub description: TrackDescription,
    pub track: Arc<Track>,
    /// The track seen from above, for lists and pickers.
    pub thumbnail: Handle<Image>,
}

impl Tracks {
    /// The track's name, or its key if the server did not send it.
    pub fn name<'a>(&'a self, key: &'a str) -> &'a str {
        self.0
            .get(key)
            .map_or(key, |entry| entry.description.name.as_str())
    }
}

/// The lobbies, as last listed by the server.
#[derive(Resource, Default)]
pub struct LobbyList {
    pub lobbies: Vec<LobbySummary>,
    /// Real time the list arrived, in seconds: its countdowns count from then.
    pub received_at: f64,
    /// Whether a list arrived since connecting.
    pub loaded: bool,
}

/// Everyone connected to the server, in the order they arrived, as last listed. The server only
/// tells players who browse lobbies, so this is what the browser shows and nothing else.
#[derive(Resource, Default)]
pub struct OnlinePlayers(pub Vec<OnlinePlayer>);

/// The lobby the player is in, if any.
#[derive(Resource, Default)]
pub struct CurrentLobby(pub Option<Membership>);

pub struct Membership {
    /// Which lobby it is.
    pub lobby: LobbyId,
    /// The player's own car id, whether or not they have a car right now.
    pub car: CarId,
    pub settings: LobbySettings,
    /// `None` until the server sends it, right after entering.
    pub state: Option<LobbyState>,
}

impl Membership {
    pub fn player(&self, car: CarId) -> Option<&LobbyPlayer> {
        self.state
            .as_ref()?
            .players
            .iter()
            .find(|player| player.car == car)
    }

    pub fn own_status(&self) -> Option<PlayerStatus> {
        self.player(self.car).map(|player| player.status)
    }
}

/// The player's experience (see `protocol::rewards`), as last heard from the server, and what the
/// last race paid and unlocked, for the results to show.
#[derive(Resource, Default, Debug)]
pub struct PlayerProgress {
    /// `None` until they have connected once; kept across connections.
    pub xp: Option<u32>,
    /// What the last race paid, and what it unlocked, until the next one pays.
    pub last_gain: Option<u32>,
    pub unlocked: Vec<Reward>,
    /// The end of the results already counted, so a lobby state sent again does not pay twice.
    counted: Option<u32>,
}

impl PlayerProgress {
    /// Counts what the race whose results end at `end_tick` paid, once: the server keeps the
    /// same, and says so when the player is back in the menus.
    pub fn paid(&mut self, end_tick: u32, gained: u32) {
        if self.counted == Some(end_tick) {
            return;
        }
        self.counted = Some(end_tick);
        let before = self.xp.unwrap_or(0);
        let after = before.saturating_add(gained);
        self.unlocked = rewards::unlocked_between(before, after);
        self.last_gain = Some(gained);
        self.xp = Some(after);
    }
}

/// Where the player stands in ranked, as last heard from the server.
#[derive(Resource, Default)]
pub struct Ladder {
    /// Their standing. `None` until they have connected once; kept across connections.
    pub profile: Option<RankedProfile>,
    pub leaderboard: Vec<LeaderboardEntry>,
    /// Their place in the queue, if they are in it.
    pub queue: Option<Queueing>,
    /// What their last ranked race did to their rating, until they queue again.
    pub last_change: Option<RatingChange>,
}

/// A player in the ranked queue.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Queueing {
    /// Everyone in the queue, the player included.
    pub players: u16,
    /// Real time they entered it, in seconds.
    pub since: f64,
}

/// A short message for the player, shown on the menus, such as why joining failed.
#[derive(Resource, Default)]
pub struct Notice(pub Option<(String, f64)>);

/// How long a notice stays, in seconds.
const NOTICE_SECONDS: f64 = 5.0;

impl Notice {
    pub fn show(&mut self, text: impl Into<String>, now: f64) {
        self.0 = Some((text.into(), now + NOTICE_SECONDS));
    }

    pub fn current(&self, now: f64) -> Option<&str> {
        self.0
            .as_ref()
            .filter(|(_, until)| now < *until)
            .map(|(text, _)| text.as_str())
    }
}

pub fn join_error_text(error: JoinError) -> &'static str {
    match error {
        JoinError::NoSuchLobby => "This lobby no longer exists.",
        JoinError::LobbyFull => "This lobby is full.",
        JoinError::InvalidSettings => "The server refused these lobby settings.",
        JoinError::UnknownTrack => "The server does not have this track.",
        JoinError::AccountRequired => "Ranked is for players with an account: sign in to play it.",
    }
}

/// Leaves the current lobby and goes back to the menu the player came from.
pub fn leave_lobby(
    network: &Network,
    current: &mut CurrentLobby,
    after: &AfterLobby,
    next_screen: &mut NextState<Screen>,
) {
    network.send(ClientMessage::LeaveLobby);
    current.0 = None;
    next_screen.set(after.0);
}

fn receive_lobby_events(
    mut events: MessageReader<NetEvent>,
    mut tracks: ResMut<Tracks>,
    mut list: ResMut<LobbyList>,
    mut online: ResMut<OnlinePlayers>,
    mut current: ResMut<CurrentLobby>,
    mut notice: ResMut<Notice>,
    mut ladder: ResMut<Ladder>,
    mut progress: ResMut<PlayerProgress>,
    mut buffer: ResMut<SnapshotBuffer>,
    mut prediction: ResMut<crate::prediction::Prediction>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time<Real>>,
) {
    let now = time.elapsed_secs_f64();
    for event in events.read() {
        match event {
            NetEvent::Welcome { tracks: infos, .. } => {
                tracks.0.clear();
                for info in infos {
                    match Track::build(&info.description) {
                        Ok(track) => {
                            let thumbnail = images.add(thumbnail::render(&track));
                            tracks.0.insert(
                                info.key.clone(),
                                TrackEntry {
                                    description: info.description.clone(),
                                    track: Arc::new(track),
                                    thumbnail,
                                },
                            );
                        }
                        Err(error) => {
                            error!(key = %info.key, "the server sent an invalid track: {error}")
                        }
                    }
                }
                *list = LobbyList::default();
                online.0.clear();
                current.0 = None;
                ladder.queue = None;
            }
            NetEvent::Lobbies(lobbies) => {
                *list = LobbyList {
                    lobbies: lobbies.clone(),
                    received_at: now,
                    loaded: true,
                };
            }
            NetEvent::Players(players) => online.0 = players.clone(),
            NetEvent::JoinedLobby {
                lobby,
                car,
                settings,
            } => {
                *buffer = SnapshotBuffer::default();
                // A new lobby counts its ticks from 0: the old one's timeline, which never goes
                // back, would hold the player's car still in it (the bug of leaving and quick
                // playing again, fixed in v0.1.71).
                *prediction = crate::prediction::Prediction::default();
                ladder.queue = None;
                current.0 = Some(Membership {
                    lobby: *lobby,
                    car: *car,
                    settings: settings.clone(),
                    state: None,
                });
            }
            NetEvent::JoinFailed(error) => notice.show(join_error_text(*error), now),
            NetEvent::Lobby(state) => {
                // Ignored once the player has left: the server may have sent it just before.
                if let Some(membership) = &mut current.0 {
                    membership.state = Some(state.clone());
                    if let LobbyPhase::Results { ratings, .. } = &state.phase
                        && let Some(change) =
                            ratings.iter().find(|change| change.car == membership.car)
                    {
                        ladder.last_change = Some(*change);
                    }
                    if let LobbyPhase::Results { xp, end_tick, .. } = &state.phase
                        && let Some(gain) = xp.iter().find(|gain| gain.car == membership.car)
                    {
                        progress.paid(*end_tick, gain.gained);
                    }
                }
            }
            NetEvent::LobbyEnded => current.0 = None,
            NetEvent::Profile(profile) => ladder.profile = Some(*profile),
            NetEvent::Progress(xp) => progress.xp = Some(*xp),
            NetEvent::Leaderboard(board) => ladder.leaderboard = board.clone(),
            NetEvent::Queue(status) => {
                let since = ladder.queue.map_or(now, |queue| queue.since);
                if status.is_some() {
                    ladder.last_change = None;
                }
                ladder.queue = status.map(|status| Queueing {
                    players: status.players,
                    since,
                });
            }
            NetEvent::Rejected(_) | NetEvent::Disconnected(_) => {
                current.0 = None;
                *list = LobbyList::default();
                online.0.clear();
                ladder.queue = None;
            }
            NetEvent::Snapshot { .. } | NetEvent::Pong { .. } => {}
        }
    }
}

const SCRIPT_TRACK: &str = "odyssey";

/// Enters the scripted lobby once connected and listed.
/// With `--rounds`, leaves the lobby `round_seconds` after entering it, the way a player does
/// from the menu, for the first screen to quick play again (see `ui/login.rs`).
fn end_round(
    mut script: ResMut<Script>,
    mut current: ResMut<CurrentLobby>,
    network: Res<Network>,
    after: Res<crate::screen::AfterLobby>,
    mut next_screen: ResMut<NextState<crate::screen::Screen>>,
    time: Res<Time<Real>>,
    mut entered: Local<Option<(LobbyId, f64)>>,
) {
    let Some(lobby) = current.0.as_ref().map(|membership| membership.lobby) else {
        *entered = None;
        return;
    };
    if script.rounds == 0 {
        return;
    }
    let now = time.elapsed_secs_f64();
    let since = match *entered {
        Some((id, since)) if id == lobby => since,
        _ => {
            *entered = Some((lobby, now));
            now
        }
    };
    if now - since >= f64::from(script.round_seconds) {
        script.rounds -= 1;
        script.rounds_left += 1;
        info!(lobby = %lobby, "leaving the lobby, as --rounds asks");
        leave_lobby(&network, &mut current, &after, &mut next_screen);
    }
}

fn follow_script(
    mut script: ResMut<Script>,
    connection: Res<Connection>,
    list: Res<LobbyList>,
    tracks: Res<Tracks>,
    network: Res<Network>,
) {
    if !matches!(*connection, Connection::Online { .. }) || !list.loaded {
        return;
    }
    // `--watch`: in to watch, once the lobby of that name is listed.
    if let Some(name) = &script.watch {
        if let Some(summary) = list
            .lobbies
            .iter()
            .find(|summary| &summary.settings.name == name)
        {
            network.send(ClientMessage::Watch(summary.id));
            script.watch = None;
        }
        return;
    }
    let Some(lobby) = script.lobby.take() else {
        return;
    };

    let existing = list
        .lobbies
        .iter()
        .find(|summary| summary.settings.name == lobby.name);
    let message = match existing {
        Some(summary) => ClientMessage::JoinLobby(summary.id),
        None => {
            // The track asked for, or the test circuit when there is one, so unattended runs look
            // the same every time.
            let wanted = lobby.track.as_deref().unwrap_or(SCRIPT_TRACK);
            let Some(track) = tracks
                .0
                .keys()
                .find(|key| *key == wanted)
                .or_else(|| tracks.0.keys().next())
            else {
                error!("the server has no track to create a lobby on");
                return;
            };
            if track != wanted {
                warn!(wanted, used = %track, "no such track, creating the lobby on another one");
            }
            ClientMessage::CreateLobby(LobbySettings {
                name: lobby.name,
                track: track.clone(),
                laps: lobby.laps,
                min_players: lobby.min_players,
                // With bots, as many cars as a lobby holds.
                max_players: if lobby.bots > 0 {
                    star_racer_protocol::MAX_LOBBY_PLAYERS
                } else {
                    8
                },
                bots: lobby.bots.min(star_racer_protocol::MAX_LOBBY_PLAYERS - 1),
            })
        }
    };
    network.send(message);
}
