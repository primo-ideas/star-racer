//! Recording what the game draws, for films (since v0.1.71): `--record <dir>` saves the window's
//! image up to `--record-fps` times a second for `--record-seconds`, then quits.
//!
//! Only what the game itself renders is saved, never the screen around it. Each frame is named by
//! the milliseconds since the recording began (`frame_00012345.png`), so frames the game was too
//! busy to take leave a gap in time rather than speeding the film up: the frames are put together
//! by their times (see docs/film.md). The PNGs are encoded on threads of their own, so the game
//! keeps its pace while it records.

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

/// Threads encoding frames.
const ENCODERS: usize = 4;
/// Frames waiting to be encoded at most; past it a frame is skipped rather than memory filled.
const QUEUE: usize = 48;

pub struct RecordPlugin {
    pub dir: PathBuf,
    pub fps: f32,
    pub seconds: f32,
}

#[derive(Resource)]
struct Recording {
    dir: PathBuf,
    every: f64,
    seconds: f64,
    started: Option<f64>,
    last: f64,
    queued: Arc<Mutex<usize>>,
    frames: mpsc::SyncSender<(PathBuf, Image)>,
}

impl Plugin for RecordPlugin {
    fn build(&self, app: &mut App) {
        std::fs::create_dir_all(&self.dir).expect("the recording's directory can be made");
        let (frames, receiver) = mpsc::sync_channel::<(PathBuf, Image)>(QUEUE);
        let receiver = Arc::new(Mutex::new(receiver));
        let queued = Arc::new(Mutex::new(0usize));
        for _ in 0..ENCODERS {
            let receiver = Arc::clone(&receiver);
            let queued = Arc::clone(&queued);
            std::thread::spawn(move || {
                loop {
                    let next = receiver.lock().map(|receiver| receiver.recv());
                    let Ok(Ok((path, image))) = next else {
                        return;
                    };
                    match image.try_into_dynamic() {
                        Ok(dynamic) => {
                            if let Err(problem) = dynamic.to_rgb8().save(&path) {
                                error!(%problem, path = %path.display(), "a frame could not be saved");
                            }
                        }
                        Err(problem) => error!(%problem, "a frame could not be converted"),
                    }
                    if let Ok(mut queued) = queued.lock() {
                        *queued = queued.saturating_sub(1);
                    }
                }
            });
        }
        app.insert_resource(Recording {
            dir: self.dir.clone(),
            every: 1.0 / f64::from(self.fps.max(1.0)),
            seconds: f64::from(self.seconds),
            started: None,
            last: f64::NEG_INFINITY,
            queued,
            frames,
        })
        .add_systems(Last, take_frame);
    }
}

fn take_frame(
    mut commands: Commands,
    mut recording: ResMut<Recording>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = time.elapsed_secs_f64();
    let started = *recording.started.get_or_insert(now);
    let elapsed = now - started;
    if elapsed > recording.seconds {
        // Every frame asked for is in the queue or written: give the encoders a moment.
        let waiting = recording.queued.lock().map_or(0, |queued| *queued);
        if waiting == 0 || elapsed > recording.seconds + 20.0 {
            exit.write(AppExit::Success);
        }
        return;
    }
    if now - recording.last < recording.every {
        return;
    }
    recording.last = now;
    let path = recording
        .dir
        .join(format!("frame_{:08}.png", (elapsed * 1000.0) as u64));
    let frames = recording.frames.clone();
    let queued = Arc::clone(&recording.queued);
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>| {
            if let Ok(mut count) = queued.lock() {
                if *count >= QUEUE {
                    return;
                }
                *count += 1;
            }
            if frames.try_send((path.clone(), captured.image.clone())).is_err()
                && let Ok(mut count) = queued.lock()
            {
                *count = count.saturating_sub(1);
            }
        },
    );
}
