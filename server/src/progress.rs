//! Every player's experience, which makes their level and what they have unlocked (see
//! `protocol::rewards`), and the file it is kept in.
//!
//! Guests have it as well as accounts: a guest's player is their device's key, an account's its
//! identity, so a guest who signs up later starts again. Like the ladder, the file lives outside
//! the data directory and is written whole, through a temporary file, after every race.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use star_racer_protocol::rewards;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::ranking::{hex, player_id, spawn_writer};
use crate::session::PlayerId;

/// A player's line in the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Record {
    /// Their public key, in hexadecimal.
    player: String,
    xp: u32,
}

/// Every player who has raced.
pub struct Progress {
    xp: HashMap<PlayerId, u32>,
    /// Hands the file's new contents to the task that writes it. `None` keeps it in memory.
    file: Option<mpsc::UnboundedSender<String>>,
}

impl Progress {
    /// Progress that is never saved, for tests.
    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self {
            xp: HashMap::new(),
            file: None,
        }
    }

    /// Reads it from `path`, and keeps it there from now on. No file is nobody having raced; a
    /// file that cannot be understood is moved aside rather than overwritten.
    pub fn load(path: PathBuf) -> Result<Self> {
        let records = match std::fs::read_to_string(&path) {
            Ok(text) => match ron::from_str::<Vec<Record>>(&text) {
                Ok(records) => records,
                Err(problem) => {
                    let aside = path.with_extension("ron.corrupt");
                    error!(
                        file = %path.display(), aside = %aside.display(), %problem,
                        "the progress file cannot be read: moving it aside and starting again"
                    );
                    std::fs::rename(&path, &aside)
                        .with_context(|| format!("moving {} aside", path.display()))?;
                    Vec::new()
                }
            },
            Err(problem) if problem.kind() == ErrorKind::NotFound => Vec::new(),
            Err(problem) => {
                return Err(problem).with_context(|| format!("reading {}", path.display()));
            }
        };
        let xp: HashMap<PlayerId, u32> = records
            .into_iter()
            .filter_map(|record| match player_id(&record.player) {
                Some(id) => Some((id, record.xp)),
                None => {
                    warn!(player = %record.player, "skipping a progress entry with a bad key");
                    None
                }
            })
            .collect();
        info!(file = %path.display(), players = xp.len(), "progress loaded");
        Ok(Self {
            xp,
            file: Some(spawn_writer(path, "progress")),
        })
    }

    /// The player's experience, 0 if they have never raced.
    pub fn xp(&self, id: PlayerId) -> u32 {
        self.xp.get(&id).copied().unwrap_or(0)
    }

    /// Adds what a race paid each of its players.
    pub fn add(&mut self, gains: &[(PlayerId, u32)]) {
        for &(id, gained) in gains {
            let xp = self.xp.entry(id).or_insert(0);
            let before = *xp;
            *xp = xp.saturating_add(gained);
            info!(
                player = %id, gained, xp = *xp, level = rewards::level(*xp),
                unlocked = ?rewards::unlocked_between(before, *xp), "experience gained"
            );
        }
        self.save();
    }

    fn save(&self) {
        let Some(file) = &self.file else {
            return;
        };
        let mut records: Vec<Record> = self
            .xp
            .iter()
            .map(|(id, xp)| Record {
                player: hex(&id.0),
                xp: *xp,
            })
            .collect();
        records.sort_by(|a, b| b.xp.cmp(&a.xp).then_with(|| a.player.cmp(&b.player)));
        match ron::ser::to_string_pretty(&records, ron::ser::PrettyConfig::default()) {
            Ok(text) => {
                // Fails only if the writer is gone, which is the server stopping.
                let _ = file.send(text);
            }
            Err(problem) => error!(%problem, "the progress could not be encoded"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experience_adds_up_per_player() {
        let mut progress = Progress::in_memory();
        let (a, b) = (PlayerId([1; 32]), PlayerId([2; 32]));
        assert_eq!(progress.xp(a), 0);
        progress.add(&[(a, 180), (b, 60)]);
        progress.add(&[(a, 240)]);
        assert_eq!((progress.xp(a), progress.xp(b)), (420, 60));
    }

    #[test]
    fn it_is_kept_in_its_file() {
        let directory = std::env::temp_dir().join(format!("star-racer-progress-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("progress.ron");
        let text = format!(
            "[(player: \"{}\", xp: 777)]",
            hex(&PlayerId([9; 32]).0)
        );
        std::fs::write(&path, text).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let progress = runtime.block_on(async { Progress::load(path.clone()).unwrap() });
        assert_eq!(progress.xp(PlayerId([9; 32])), 777);
        drop(runtime);
        let _ = std::fs::remove_dir_all(&directory);
    }
}
