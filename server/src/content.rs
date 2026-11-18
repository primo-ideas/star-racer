//! Game data read from the data directory: tracks and car tuning.
//!
//! ```text
//! <data dir>/
//!   car.ron            car tuning, reloaded while the server runs
//!   tracks/<key>.ron   one file per track, named by its key
//! ```
//!
//! And, beside the ladder, the circuits players published from the level editor (since
//! v1.0.0-rc2), `user-tracks/user-<name>.ron` by default (see [`UserTracks`]).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use star_racer_protocol::{MAX_TRACK_ITEMS, MAX_TRACK_LENGTH, PublishError};
use star_racer_sim::car::CarTuning;
use star_racer_sim::track::{Track, TrackDescription};
use tokio::sync::watch;
use tracing::{info, warn};

use crate::session::PlayerId;

pub const CAR_TUNING_FILE: &str = "car.ron";
pub const TRACKS_DIR: &str = "tracks";

/// How often the car tuning file is checked for changes.
const TUNING_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// Every track lobbies can race on, by key.
pub type Tracks = BTreeMap<String, Arc<LoadedTrack>>;

/// A track as written in its file, and the geometry built from it.
pub struct LoadedTrack {
    pub description: TrackDescription,
    pub track: Track,
    /// Who published it from the level editor; `None` for the game's own.
    pub author: Option<Author>,
}

/// Loads every `.ron` file of the tracks directory. A server without tracks has nothing to race on,
/// so that is an error.
pub fn load_tracks(data_dir: &Path) -> Result<Tracks> {
    let dir = data_dir.join(TRACKS_DIR);
    let entries = fs::read_dir(&dir).with_context(|| format!("cannot read {}", dir.display()))?;
    let mut tracks = Tracks::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_none_or(|extension| extension != "ron") {
            continue;
        }
        let Some(key) = path.file_stem().and_then(|stem| stem.to_str()) else {
            bail!("track file name is not valid UTF-8: {}", path.display());
        };
        tracks.insert(key.to_owned(), Arc::new(load_track(&path)?));
    }
    if tracks.is_empty() {
        bail!("no track in {}", dir.display());
    }
    Ok(tracks)
}

pub fn load_track(path: &Path) -> Result<LoadedTrack> {
    let description: TrackDescription = read_ron(path)?;
    let track =
        Track::build(&description).with_context(|| format!("invalid track {}", path.display()))?;
    Ok(LoadedTrack {
        description,
        track,
        author: None,
    })
}

/// Loads the car tuning, which must suit every track.
pub fn load_car_tuning(path: &Path, tracks: &Tracks) -> Result<CarTuning> {
    let tuning: CarTuning = read_ron(path)?;
    tuning
        .validate()
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("invalid car tuning {}", path.display()))?;
    for (key, loaded) in tracks {
        let track = &loaded.track;
        // The car has to fit through the bottlenecks, not just along the open road.
        if tuning.collision_radius >= track.narrowest_half_width() {
            bail!(
                "invalid car tuning {}: a collision radius of {} m does not fit on track {key}, \
                 {} m wide where it pinches in",
                path.display(),
                tuning.collision_radius,
                track.narrowest_half_width() * 2.0
            );
        }
    }
    Ok(tuning)
}

/// Watches the car tuning file, so the feel of the car can be adjusted while driving.
pub struct CarTuningFile {
    path: PathBuf,
    modified: Option<SystemTime>,
    tracks: Tracks,
}

impl CarTuningFile {
    pub fn load(path: PathBuf, tracks: &Tracks) -> Result<(Self, CarTuning)> {
        let modified = modified_time(&path);
        let tuning = load_car_tuning(&path, tracks)?;
        let file = Self {
            path,
            modified,
            tracks: tracks.clone(),
        };
        Ok((file, tuning))
    }

    /// Checks the file every second on a task of its own, and publishes every valid change. Every
    /// lobby follows the receiver.
    pub fn watch(mut self, tuning: CarTuning) -> watch::Receiver<CarTuning> {
        let (sender, receiver) = watch::channel(tuning);
        tokio::spawn(async move {
            let mut checks = tokio::time::interval(TUNING_CHECK_INTERVAL);
            loop {
                checks.tick().await;
                if let Some(tuning) = self.reload_if_changed() {
                    sender.send_replace(tuning);
                }
            }
        });
        receiver
    }

    /// The new tuning if the file changed since the last check and is valid. An invalid file is
    /// reported and ignored, so a typo while tuning never stops a race.
    fn reload_if_changed(&mut self) -> Option<CarTuning> {
        let modified = modified_time(&self.path);
        if modified == self.modified {
            return None;
        }
        self.modified = modified;

        match load_car_tuning(&self.path, &self.tracks) {
            Ok(tuning) => {
                info!(path = %self.path.display(), "car tuning reloaded");
                Some(tuning)
            }
            Err(error) => {
                warn!("car tuning not reloaded, keeping the previous one: {error:#}");
                None
            }
        }
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
}

/// The key every circuit published from the level editor starts with (since v1.0.0-rc2), so it
/// never takes the key of one of the game's own.
pub const USER_TRACK_PREFIX: &str = "user-";

/// A circuit published from the level editor, as its file holds it: who published it, and the
/// circuit.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserTrackFile {
    /// The author's identity, in hexadecimal: only they may publish it again.
    author: String,
    author_name: String,
    description: TrackDescription,
}

/// Where the circuits published from the level editor are kept, one file each, outside the data
/// directory a deployment replaces. `None` keeps them in memory, for tests.
#[derive(Debug, Clone)]
pub struct UserTracks {
    dir: Option<PathBuf>,
}

impl UserTracks {
    pub fn kept_in(dir: PathBuf) -> Self {
        Self { dir: Some(dir) }
    }

    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self { dir: None }
    }

    /// Every circuit published so far. No directory is none published; a file that does not
    /// load, or no longer builds, is reported and left out rather than stopping the server.
    pub fn load(&self) -> Tracks {
        let mut tracks = Tracks::new();
        let Some(dir) = &self.dir else {
            return tracks;
        };
        let Ok(entries) = fs::read_dir(dir) else {
            return tracks;
        };
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.extension().is_none_or(|extension| extension != "ron") {
                continue;
            }
            let Some(key) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let loaded = read_ron::<UserTrackFile>(&path).and_then(|file| {
                let author = crate::ranking::player_id(&file.author)
                    .context("the author is not an identity")?;
                let track = Track::build(&file.description)
                    .with_context(|| format!("invalid track {}", path.display()))?;
                Ok(LoadedTrack {
                    description: file.description,
                    track,
                    author: Some(Author {
                        id: author,
                        name: file.author_name,
                    }),
                })
            });
            match loaded {
                Ok(loaded) => {
                    tracks.insert(key.to_owned(), Arc::new(loaded));
                }
                Err(problem) => warn!("published track left out: {problem:#}"),
            }
        }
        tracks
    }

    /// Writes the circuit `key` to its file, replacing the one before.
    pub fn save(&self, key: &str, author: &Author, description: &TrackDescription) -> Result<()> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let file = UserTrackFile {
            author: crate::ranking::hex(&author.id.0),
            author_name: author.name.clone(),
            description: description.clone(),
        };
        let text = ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::default())
            .context("writing the track")?;
        crate::ranking::write_whole(&dir.join(format!("{key}.ron")), &text)
    }
}

/// Who published a circuit from the level editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub id: PlayerId,
    pub name: String,
}

/// The key a circuit named `name` is published under: its name in lower case, every run of other
/// characters than letters and digits a dash, after [`USER_TRACK_PREFIX`]. `None` for a name that
/// is not a valid one for a lobby, or holds no letter or digit.
pub fn user_track_key(name: &str) -> Option<String> {
    if !star_racer_protocol::is_valid_lobby_name(name) {
        return None;
    }
    let mut slug = String::new();
    for character in name.trim().chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-');
    (!slug.is_empty()).then(|| format!("{USER_TRACK_PREFIX}{slug}"))
}

/// Builds a circuit from the level editor, refusing what a lobby could not race: too much of
/// anything, a lap too long, a circuit that does not build, or one the car does not fit.
pub fn check_user_track(
    description: &TrackDescription,
    tuning: &CarTuning,
) -> Result<Track, PublishError> {
    let lists = [
        description.segments.len(),
        description.elevation.len(),
        description.narrows.len(),
        description.scenery.len(),
        description.pads.len(),
        description.gaps.len(),
        description.checkpoints.len(),
        description.breaches.len(),
    ];
    if lists.iter().any(|count| *count > MAX_TRACK_ITEMS) {
        return Err(PublishError::TooBig);
    }
    let track =
        Track::build(description).map_err(|error| PublishError::Invalid(error.to_string()))?;
    if track.length() > MAX_TRACK_LENGTH {
        return Err(PublishError::TooBig);
    }
    if tuning.collision_radius >= track.narrowest_half_width() {
        return Err(PublishError::Invalid(format!(
            "the road is {:.1} m wide where it pinches in: the car needs more than {:.1} m",
            track.narrowest_half_width() * 2.0,
            tuning.collision_radius * 2.0
        )));
    }
    Ok(track)
}

fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    ron::from_str(&text).with_context(|| format!("cannot parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use star_racer_sim::autopilot::{self, Style};
    use star_racer_sim::track::scenery::Prop;

    use super::*;

    fn data_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
    }

    /// A published circuit is written to its file and read back with its author; a key's name
    /// is its name in lower case with dashes, and a name with no letter nor digit makes none.
    #[test]
    fn published_tracks_are_kept_and_read_back() {
        assert_eq!(user_track_key("Ada's  Loop!").as_deref(), Some("user-ada-s-loop"));
        assert_eq!(user_track_key("--"), None);
        let dir = std::env::temp_dir().join(format!("star-racer-user-tracks-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let kept = UserTracks::kept_in(dir.clone());
        assert!(kept.load().is_empty(), "no directory is no track");
        let tracks = load_tracks(&data_dir()).unwrap();
        let (_, odyssey) = tracks.iter().next().unwrap();
        let author = Author {
            id: PlayerId([7; 32]),
            name: "Ada".into(),
        };
        kept.save("user-mine", &author, &odyssey.description).unwrap();
        let loaded = kept.load();
        let mine = &loaded["user-mine"];
        assert_eq!(mine.author.as_ref(), Some(&author));
        assert_eq!(mine.description, odyssey.description);
        let _ = fs::remove_dir_all(&dir);
    }

    /// Every shipped track loads, and the autopilot laps it with the shipped car: drifting without
    /// touching a wall and faster than gripping, and cleanly in some style when a few ticks late, gripping without a fall. A
    /// track or tuning edit that makes a circuit undrivable, or drifting pointless, fails here.
    #[test]
    fn shipped_tracks_are_drivable() {
        let tracks = load_tracks(&data_dir()).unwrap();
        let tuning = load_car_tuning(&data_dir().join(CAR_TUNING_FILE), &tracks).unwrap();
        for (key, loaded) in &tracks {
            // Driving from the live car, like the server, and a few ticks late, like a client.
            let mut lap_times = Vec::new();
            let mut clean_while_late = false;
            for style in [Style::Grip, Style::Drift] {
                for delay_ticks in [0, 6] {
                    let report =
                        autopilot::drive_lap(&loaded.track, &tuning, style, 120.0, delay_ticks);
                    let context = format!("{key}, {style:?}, {delay_ticks} ticks late");
                    println!("{context}: {report:?}");
                    let lap_time = report.lap_time.expect(&context);
                    // Every gap is jumped: a fall is a lap the autopilot cannot drive. Six ticks
                    // late, drifting may fall: the lap is chaotic there, a millionth of top speed
                    // or the platform's math library deciding whether a drift is still under way
                    // at a gap's lip (docs/simulation.md#the-late-drifting-lap). It must still get
                    // round, and gripping late must not fall.
                    if delay_ticks == 0 || style == Style::Grip {
                        assert_eq!(report.falls, 0, "{context}: {report:?}");
                    }
                    if delay_ticks == 0 {
                        // Gripping may have to scrape through turns meant for drifting. A
                        // drifting lap gets a small budget instead of none: a slide worth having
                        // runs wide, and a bottleneck is drawn to leave it little room. A graze
                        // of a few hundredths of a second is not a circuit that cannot be driven;
                        // a lap spent against the wall is, and that is what this still catches.
                        if style == Style::Drift {
                            assert!(report.wall_ticks <= 3, "{context}: {report:?}");
                        }
                        lap_times.push(lap_time);
                    } else {
                        clean_while_late |= report.wall_ticks == 0;
                    }
                    if style == Style::Drift {
                        assert!(report.boost_ticks > 0, "{context}: {report:?}");
                    }
                }
            }
            // Late information can make a style unfit for a track, gripping through tight turns for
            // one, but a client must still have a clean way round.
            assert!(
                clean_while_late,
                "{key}: no style laps cleanly 6 ticks late"
            );
            // Drifting through the turns must get round faster: drifting has to pay.
            let [grip, drift] = lap_times[..] else {
                unreachable!();
            };
            assert!(drift < grip, "{key}: drifting {drift} s, gripping {grip} s");
        }
    }

    /// Where a car set straight down the road `at` meters along the Odyssey, `lateral` from its
    /// centerline, is three seconds later, the jump held for `hold` seconds: how far along the
    /// lap it has got, or nothing if it was brought back to a checkpoint on the way.
    fn straight_on(at: f32, lateral: f32, speed: f32, hold: f32) -> Option<f32> {
        use star_racer_sim::car::{Car, CarInput};
        use star_racer_sim::{TICK_RATE, TICK_SECONDS};
        let tracks = load_tracks(&data_dir()).unwrap();
        let tuning = load_car_tuning(&data_dir().join(CAR_TUNING_FILE), &tracks).unwrap();
        let track = &tracks["odyssey"].track;
        let point = track.point_at(at);
        let mut car = Car::new(
            point.position + point.direction.perp() * lateral,
            point.direction.to_angle(),
        );
        car.velocity = point.direction * speed;
        car.thrust = tuning.thrust_seconds;
        car.checkpoint = track.gates().iter().rposition(|gate| *gate <= at).unwrap() as u8;
        // Faster than the car goes by itself: on a boost, or on the turbo.
        car.boost = if speed > tuning.top_speed { 3.0 } else { 0.0 };
        car.turbo_boost = if speed > tuning.boost_top_speed { 3.0 } else { 0.0 };
        for tick in 0..3 * TICK_RATE {
            let held = (tick as f32 * TICK_SECONDS) < hold;
            car.step(CarInput::new(true, 0.0).with_jump(held).with_thrust(held), &tuning, track);
            if car.respawns > 0 {
                return None;
            }
            if tick > TICK_RATE && !car.is_airborne() && !car.is_astray() {
                return Some(track.project(car.position).distance - at);
            }
        }
        None
    }

    /// The Odyssey's three shortcuts are there to be found, and cost what they should
    /// (docs/tracks.md#shortcuts): the keyhole and the long way round are flown straight on over
    /// the wall on the thruster, from top speed (until v0.1.51 the long way round needed the turbo,
    /// the tank being smaller); the balcony is driven off through its breach, with nothing
    /// pressed. Each skips hundreds of meters of road. A hop alone does not clear a wall. A change
    /// to the track or to the car that closes one fails here.
    #[test]
    fn the_shortcuts_of_the_odyssey_are_there() {
        // From the straight before the loop: later, the car flies into the banked road of its first turn.
        let keyhole = straight_on(620.0, 0.0, 58.0, 1.6);
        assert!(keyhole.is_some_and(|gained| gained > 400.0), "keyhole: {keyhole:?}");
        // Without flying, or with a hop only, the wall holds the car on the road.
        for hold in [0.0, 0.05] {
            let held = straight_on(644.0, 0.0, 58.0, hold);
            assert!(held.is_some_and(|gained| gained < 200.0), "keyhole, on the road: {held:?}");
        }

        let balcony = straight_on(2000.0, 0.0, 58.0, 0.0);
        assert!(balcony.is_some_and(|gained| gained > 350.0), "balcony: {balcony:?}");

        let long_way = straight_on(4275.0, 0.0, 58.0, 1.6);
        assert!(long_way.is_some_and(|gained| gained > 400.0), "long way round: {long_way:?}");
    }

    /// Scenery is decoration, so building a track only checks that a prop stands off the road where
    /// it is placed. A circuit doubles back on itself, though, so a stand or a slab put far out on
    /// one turn can land on another part of the lap: every shipped prop is checked against the whole
    /// centerline here. A prop is sampled along its length and out to its depth, which is rough but
    /// enough to catch one standing on a road it does not belong to.
    #[test]
    fn shipped_scenery_stands_clear_of_the_rest_of_the_lap() {
        /// How much room a prop must leave around any other stretch of road, in meters past its
        /// edge.
        const MARGIN: f32 = 3.0;
        /// How much of the lap around a prop is the road it belongs to, in meters either way.
        const OWN_ROAD: f32 = 40.0;

        let tracks = load_tracks(&data_dir()).unwrap();
        for (key, loaded) in &tracks {
            let track = &loaded.track;
            // How close the road comes to a position, leaving out the stretch the prop stands by.
            let elsewhere = |position, at: f32| {
                track
                    .points()
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| {
                        let distance = *index as f32 * track.spacing();
                        let gap = (distance - at.rem_euclid(track.length())).abs();
                        gap.min(track.length() - gap) > OWN_ROAD
                    })
                    .map(|(_, point)| point.position.distance(position))
                    .fold(f32::INFINITY, f32::min)
            };

            for (index, prop) in loaded.description.scenery.iter().enumerate() {
                // A gantry or a tunnel spans the road on purpose; every other prop stands beside
                // it. A district needs no checking here: it lays its towers out clear of every part
                // of the road itself.
                let Some(side) = prop.side() else {
                    continue;
                };
                if matches!(prop, Prop::Skyline { .. }) {
                    continue;
                }
                let (from, to, depth) = footprint(prop);
                let offset = prop.offset().unwrap_or_default();
                let mut closest = f32::INFINITY;
                for step in 0..=4 {
                    let at = prop.at() + from + (to - from) * step as f32 / 4.0;
                    let point = track.point_at(at);
                    for out in [0.0, depth / 2.0, depth] {
                        let lateral = side.sign() * (point.half_width + offset + out);
                        let position = point.position + point.direction.perp() * lateral;
                        closest = closest.min(elsewhere(position, at));
                    }
                }
                println!("{key}: prop {index} comes {closest:.0} m from the rest of the lap");
                assert!(
                    closest > track.half_width() + MARGIN,
                    "{key}: prop {index} ({prop:?}) stands {closest:.1} m from another part of the \
                     road, which is {:.0} m wide",
                    track.half_width() * 2.0
                );
            }
        }
    }

    /// Roughly where a prop runs along the road, from and to, in meters from where the file places
    /// it, and how far it reaches away from the road.
    fn footprint(prop: &Prop) -> (f32, f32, f32) {
        match *prop {
            Prop::Grandstand { length, rows, .. } => (0.0, length, rows as f32 * 2.0),
            Prop::Facade { length, depth, .. } => (0.0, length, depth),
            Prop::Billboard { width, .. } => (-width / 2.0, width / 2.0, 1.0),
            Prop::Monolith { width, .. } => (-width / 2.0, width / 2.0, width * 0.5),
            Prop::Tower { width, depth, .. } => (-width / 2.0, width / 2.0, depth),
            _ => (0.0, 0.0, 3.0),
        }
    }

    #[test]
    fn typos_in_track_files_are_errors() {
        let text = "(name: \"Typo\", width: 16.0, segments: [Turn(angle: 360.0, radius: 40.0, \
                    transition: 10.0)])";
        assert!(ron::from_str::<TrackDescription>(text).is_ok());

        let typo = text.replace("transition", "transtion");
        let error = ron::from_str::<TrackDescription>(&typo).unwrap_err();
        assert!(error.to_string().contains("transtion"), "{error}");
    }
}
