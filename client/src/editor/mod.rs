//! The level editor (since v1.0.0-rc2, the user's word: "fais moi un éditeur de niveau"; asked,
//! they chose it in the game, its circuits sent to the server, and everything editable: the road,
//! what is played on it, and the scenery).
//!
//! A screen of its own, reached from the first screen once connected: the circuit as it would be
//! raced, built by the race's own code and seen from above, and a panel to lay its road segment
//! by segment and to place along it the heights, gaps, breaches, bottlenecks, pads, checkpoints
//! and props. The road need not end where it began: the editor closes it with the shortest way
//! back to the line (see `star_racer_sim::track::editing`), drawn apart in the panel's status. A
//! click on the map puts the cursor there, where what is added goes, and picks the part of the
//! open tab nearest it.
//!
//! TRY races the circuit alone in a lobby nobody else sees, and leaving it comes back here; PUBLISH
//! puts it on the server, under its name, for every lobby to race on (an account's, not a
//! guest's). The draft is kept beside the player's other choices, so it survives a restart; LOAD
//! takes any circuit the server has as a new draft.

mod map;
pub mod model;
mod panel;

use std::sync::Arc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use star_racer_protocol::{ClientMessage, PublishError};
use star_racer_sim::track::editing;
use star_racer_sim::track::{Segment, Track, TrackDescription};

use crate::lobby::{Tracks, join_error_text};
use crate::net::{NetEvent, Network};
use crate::preferences::StoredPreferences;
use crate::screen::{AfterLobby, Screen};

use self::model::{Item, Tab};

/// Where the draft is kept, beside the player's other choices.
const SLOT: &str = "editor.ron";

/// The radius of the turns the editor closes a circuit with, in meters, unless the player changes
/// it.
const CLOSING_RADIUS: f32 = 70.0;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Editor), (open, panel::spawn).chain())
            .add_systems(OnExit(Screen::Editor), close)
            // The circuit tried is nobody's to pick in the lobby creation dialog.
            .add_systems(OnEnter(Screen::Login), forget_try)
            .add_systems(OnEnter(Screen::Lobbies), forget_try)
            .add_systems(
                Update,
                (
                    hear_server,
                    panel::read_name,
                    panel::refresh,
                    panel::size_buttons,
                    map::rebuild,
                    map::steer_camera,
                    map::click,
                    map::draw_overlay,
                    leave_on_back,
                    scripted_try,
                )
                    .chain()
                    .run_if(in_state(Screen::Editor)),
            );
        panel::observe(app);
    }
}

/// What is kept of the draft between runs.
#[derive(Serialize, Deserialize)]
struct Kept {
    description: TrackDescription,
    closing_radius: f32,
}

/// The editor's state: the draft and how it is being looked at.
#[derive(Resource)]
pub struct Editor {
    /// The circuit as the player laid it, without the segments that close it.
    pub draft: TrackDescription,
    pub closing_radius: f32,
    pub tab: Tab,
    pub selected: Option<Item>,
    /// Meters along the lap where the last click on the map fell: where what is added goes.
    pub cursor: Option<f32>,
    /// What the editor last has to say, and whether it is bad news.
    pub message: Option<(String, bool)>,
    /// Which of the server's tracks LOAD would take, by place in the list.
    pub load_choice: usize,
    /// The circuit as it will be raced, closed, or why it cannot be.
    pub built: Result<Built, String>,
    /// Something changed that the panel shows, or that the map shows.
    pub panel_dirty: bool,
    pub scene_dirty: bool,
    pub overlay_dirty: bool,
    /// The camera should frame the whole circuit again.
    pub reframe: bool,
}

/// A draft closed into a circuit that builds.
pub struct Built {
    pub description: TrackDescription,
    pub track: Arc<Track>,
    /// How many of its segments the editor added to close it, at the end.
    pub closing: usize,
}

impl Editor {
    fn new(draft: TrackDescription, closing_radius: f32) -> Self {
        let mut editor = Self {
            draft,
            closing_radius,
            tab: Tab::Road,
            selected: None,
            cursor: None,
            message: None,
            load_choice: 0,
            built: Err(String::new()),
            panel_dirty: true,
            scene_dirty: true,
            overlay_dirty: true,
            reframe: true,
        };
        editor.build();
        editor
    }

    /// Closes the draft and builds it, as the server will.
    fn build(&mut self) {
        self.built = editing::closing(&self.draft, self.closing_radius)
            .map_err(str::to_owned)
            .and_then(|closing| {
                let mut description = self.draft.clone();
                let count = closing.len();
                description.segments.extend(closing);
                Track::build(&description)
                    .map(|track| Built {
                        description,
                        track: Arc::new(track),
                        closing: count,
                    })
                    .map_err(|error| error.to_string())
            });
    }

    /// Everything changed: the draft is built again, and both the panel and the map redrawn.
    pub fn changed(&mut self) {
        self.build();
        self.panel_dirty = true;
        self.scene_dirty = true;
        self.overlay_dirty = true;
    }

    pub fn say(&mut self, text: impl Into<String>, bad: bool) {
        self.message = Some((text.into(), bad));
        self.panel_dirty = true;
    }

    /// Where segment `index` of the draft starts and ends, in meters along the lap.
    pub fn segment_span(&self, index: usize) -> (f32, f32) {
        let length = |segment: &Segment| match *segment {
            Segment::Straight { length } => length,
            Segment::Turn { angle, radius, .. } => angle.abs().to_radians() * radius,
        };
        let start: f32 = self.draft.segments[..index].iter().map(length).sum();
        (start, start + self.draft.segments.get(index).map_or(0.0, length))
    }

    fn keep(&self, preferences: &StoredPreferences) {
        let kept = Kept {
            description: self.draft.clone(),
            closing_radius: self.closing_radius,
        };
        match ron::ser::to_string_pretty(&kept, ron::ser::PrettyConfig::default()) {
            Ok(text) => {
                if let Err(error) = preferences.store().write(SLOT, &text) {
                    warn!("cannot keep the editor's draft: {error:#}");
                }
            }
            Err(error) => warn!("cannot write down the editor's draft: {error}"),
        }
    }
}

/// On entering: the draft as it was left, or a first circuit.
fn open(mut commands: Commands, existing: Option<ResMut<Editor>>, preferences: Res<StoredPreferences>) {
    if let Some(mut editor) = existing {
        editor.changed();
        return;
    }
    let kept = preferences
        .store()
        .read(SLOT)
        .and_then(|text| ron::from_str::<Kept>(&text).ok());
    let editor = match kept {
        Some(kept) => Editor::new(kept.description, kept.closing_radius),
        None => Editor::new(model::starter(), CLOSING_RADIUS),
    };
    commands.insert_resource(editor);
}

fn close(
    mut commands: Commands,
    editor: Option<Res<Editor>>,
    preferences: Res<StoredPreferences>,
    scenes: Query<Entity, With<map::EditorScene>>,
) {
    if let Some(editor) = editor {
        editor.keep(&preferences);
    }
    for scene in &scenes {
        commands.entity(scene).despawn();
    }
}

/// What the server answers the editor: a circuit published or refused, a try refused.
fn hear_server(
    mut events: MessageReader<NetEvent>,
    mut editor: ResMut<Editor>,
) {
    for event in events.read() {
        match event {
            NetEvent::Published(Ok(key)) => {
                info!(key, "circuit published");
                editor.say("PUBLISHED: every lobby can race it now.", false);
            }
            NetEvent::Published(Err(error)) => {
                let text = match error {
                    PublishError::AccountRequired => {
                        "Guests cannot publish: sign in to an account.".to_owned()
                    }
                    PublishError::NameTaken => {
                        "Another circuit has this name: choose another.".to_owned()
                    }
                    PublishError::Invalid(why) => format!("Refused: {why}"),
                    PublishError::TooBig => "Too big to publish.".to_owned(),
                };
                editor.say(text, true);
            }
            NetEvent::JoinFailed(error) => editor.say(join_error_text(*error), true),
            _ => {}
        }
    }
}

/// Escape, or B, goes back to the first screen, the draft kept.
fn leave_on_back(input: crate::ui::navigation::MenuInput, mut next: ResMut<NextState<Screen>>) {
    if input.back() {
        next.set(Screen::Login);
    }
}

/// TRY: the circuit is kept for the lobby to draw, and the server asked for a lobby racing it.
pub fn try_draft(
    editor: &mut Editor,
    network: &Network,
    tracks: &mut Tracks,
    images: &mut Assets<Image>,
    after: &mut AfterLobby,
) {
    let Ok(built) = &editor.built else {
        editor.say("The circuit does not build yet: see why below.", true);
        return;
    };
    let track = Track::build(&built.description).expect("it built a moment ago");
    tracks.store_try(&built.description, track, images);
    after.0 = Screen::Editor;
    network.send(ClientMessage::TryTrack(built.description.clone()));
    editor.say("Opening a lobby to try it…", false);
}

/// PUBLISH: the closed circuit goes to the server, which answers.
pub fn publish_draft(editor: &mut Editor, network: &Network) {
    let Ok(built) = &editor.built else {
        editor.say("The circuit does not build yet: see why below.", true);
        return;
    };
    network.send(ClientMessage::PublishTrack(built.description.clone()));
    editor.say("Publishing…", false);
}

/// LOAD: one of the server's circuits as a new draft, its own segments all kept.
pub fn load(editor: &mut Editor, description: &TrackDescription) {
    editor.draft = description.clone();
    editor.selected = None;
    editor.cursor = None;
    editor.say(format!("Loaded {}.", description.name), false);
    editor.reframe = true;
    editor.changed();
}

fn forget_try(mut tracks: ResMut<Tracks>) {
    tracks.0.remove(star_racer_protocol::TRY_TRACK);
}

/// `--editor-try`: TRY pressed once, as soon as the draft builds.
fn scripted_try(
    mut script: ResMut<crate::settings::Script>,
    mut editor: ResMut<Editor>,
    network: Res<Network>,
    mut tracks: ResMut<Tracks>,
    mut images: ResMut<Assets<Image>>,
    mut after: ResMut<AfterLobby>,
) {
    if script.editor_try && editor.built.is_ok() {
        script.editor_try = false;
        try_draft(&mut editor, &network, &mut tracks, &mut images, &mut after);
    }
}
