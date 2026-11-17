//! The horns, six of them, written sample by sample like the bang of the hop: three free and three
//! won by playing (see `protocol::rewards`). Each is a few notes of a waveform under an envelope,
//! worked out once at start-up.
//!
//! - **TWO-TONE**: the car horn everyone knows, two notes a major third apart at once, reedy.
//! - **AIR HORN**: a truck's, a low chord blown hard, its pitch settling as the air comes.
//! - **BEEP BEEP**: two short, bright beeps.
//! - **LASER** (won): two zaps falling fast through the spectrum.
//! - **FANFARE** (won): a brassy rising arpeggio, the last note held.
//! - **SAUCER** (won): a theremin's warble, climbing.

use std::f32::consts::TAU;
use std::sync::Arc;

use super::bang::Recorded;
use super::tone::{Filter, SAMPLE_RATE};

/// The horns' names, in the order the wire numbers them.
pub const HORNS: [&str; 6] = ["TWO-TONE", "AIR HORN", "BEEP BEEP", "LASER", "FANFARE", "SAUCER"];

/// How loud a horn is at its peak, beside the tones' 0.35: a horn is meant to be heard.
const PEAK: f32 = 0.45;
/// Seconds over which a note fades in and out, so none clicks.
const EDGE: f32 = 0.006;

#[derive(Clone, Copy)]
enum Wave {
    Sine,
    /// A sawtooth with its top softened: reedy and brassy.
    Saw,
    /// A square: hollow and bright.
    Square,
}

impl Wave {
    /// The wave at `phase`, from 0 to 1, built from partials up to a ceiling so it does not alias.
    fn at(self, phase: f32, pitch: f32) -> f32 {
        let partials = ((6_000.0 / pitch.max(20.0)) as usize).clamp(1, 24);
        match self {
            Self::Sine => (TAU * phase).sin(),
            Self::Saw => (1..=partials)
                .map(|k| (TAU * phase * k as f32).sin() / k as f32)
                .sum::<f32>()
                * 0.6,
            Self::Square => (1..=partials)
                .step_by(2)
                .map(|k| (TAU * phase * k as f32).sin() / k as f32)
                .sum::<f32>()
                * 0.8,
        }
    }
}

/// A note: when it starts and how long it lasts, in seconds, its pitch over its own time (from 0
/// to 1), its wave, and how loud it is.
struct Note {
    at: f32,
    seconds: f32,
    pitch: fn(f32) -> f32,
    wave: Wave,
    level: f32,
}

fn note(at: f32, seconds: f32, pitch: fn(f32) -> f32, wave: Wave, level: f32) -> Note {
    Note {
        at,
        seconds,
        pitch,
        wave,
        level,
    }
}

/// The notes of each horn, and how bright it is: the band its notes are heard through, in Hz.
fn score(horn: u8) -> (Vec<Note>, f32) {
    use Wave::{Saw, Sine, Square};
    match horn {
        // Two notes at once, a major third apart, a little apart from each other in pitch.
        0 => (
            vec![
                note(0.0, 0.5, |_| 415.3, Saw, 0.6),
                note(0.0, 0.5, |_| 523.3, Saw, 0.5),
            ],
            2_400.0,
        ),
        // A low chord, sagging into pitch as the air comes.
        1 => (
            vec![
                note(0.0, 0.8, |t| 185.0 * (1.0 - 0.05 * (1.0 - t).powi(8)), Saw, 0.5),
                note(0.0, 0.8, |t| 233.1 * (1.0 - 0.05 * (1.0 - t).powi(8)), Saw, 0.45),
                note(0.0, 0.8, |t| 277.2 * (1.0 - 0.05 * (1.0 - t).powi(8)), Saw, 0.4),
            ],
            3_200.0,
        ),
        // Two short beeps.
        2 => (
            vec![
                note(0.0, 0.11, |_| 932.3, Square, 0.6),
                note(0.18, 0.13, |_| 932.3, Square, 0.6),
            ],
            4_000.0,
        ),
        // Each zap falls three octaves in a tenth of a second and a half.
        3 => (
            vec![
                note(0.0, 0.16, |t| 2_400.0 * 0.125f32.powf(t), Square, 0.55),
                note(0.2, 0.16, |t| 2_400.0 * 0.125f32.powf(t), Square, 0.55),
            ],
            5_000.0,
        ),
        // C, E, G, then the C above, held.
        4 => (
            vec![
                note(0.0, 0.11, |_| 523.3, Saw, 0.5),
                note(0.11, 0.11, |_| 659.3, Saw, 0.5),
                note(0.22, 0.11, |_| 784.0, Saw, 0.5),
                note(0.33, 0.42, |_| 1_046.5, Saw, 0.55),
            ],
            3_000.0,
        ),
        // Warbling seven times a second, climbing a fifth.
        _ => (
            vec![note(
                0.0,
                0.9,
                |t| 600.0 * 1.5f32.powf(t) * (1.0 + 0.06 * (TAU * 7.0 * t * 0.9).sin()),
                Sine,
                0.8,
            )],
            4_000.0,
        ),
    }
}

/// The horn numbered `horn`, worked out sample by sample.
pub fn horn(horn: u8) -> Recorded {
    let rate = SAMPLE_RATE as f32;
    let (notes, brightness) = score(horn);
    let seconds = notes
        .iter()
        .map(|note| note.at + note.seconds)
        .fold(0.0f32, f32::max);
    let count = (seconds * rate) as usize + 1;
    let mut mix = vec![0.0f32; count];
    for note in &notes {
        let mut phase = 0.0f32;
        let start = (note.at * rate) as usize;
        let length = (note.seconds * rate) as usize;
        for index in 0..length.min(count.saturating_sub(start)) {
            let local = index as f32 / rate;
            let share = local / note.seconds;
            let pitch = (note.pitch)(share);
            let edges = (local / EDGE).min(1.0) * ((note.seconds - local) / EDGE).clamp(0.0, 1.0);
            mix[start + index] += note.level * edges * note.wave.at(phase, pitch);
            phase = (phase + pitch / rate).fract();
        }
    }
    // Through a band that takes the fizz off the top, then brought to its peak with a soft clip.
    let mut filter = Filter::default();
    let filtered: Vec<f32> = mix
        .iter()
        .map(|sample| {
            let (low, _) = filter.run(*sample, brightness, 0.9);
            low
        })
        .collect();
    let loudest = filtered.iter().fold(0.0f32, |most, sample| most.max(sample.abs()));
    let scale = if loudest > 0.0 { 1.4 / loudest } else { 0.0 };
    let samples: Arc<[f32]> = filtered
        .iter()
        .map(|sample| PEAK * (sample * scale).tanh() / 1.4f32.tanh())
        .collect();
    Recorded(samples)
}

#[cfg(test)]
mod tests {
    use super::*;
    use star_racer_protocol::CAR_HORNS;

    /// Every horn is heard, stays in range, starts and ends silent, and none is the same as another.
    #[test]
    fn every_horn_is_heard_and_its_own() {
        assert_eq!(HORNS.len(), usize::from(CAR_HORNS));
        let horns: Vec<Arc<[f32]>> = (0..CAR_HORNS).map(|index| horn(index).0).collect();
        for (index, samples) in horns.iter().enumerate() {
            let peak = samples.iter().fold(0.0f32, |most, sample| most.max(sample.abs()));
            assert!(peak > 0.2 && peak <= PEAK + 1e-4, "{index}: {peak}");
            assert!(samples[0].abs() < 1e-3, "{index} clicks in");
            assert!(samples.last().unwrap().abs() < 1e-2, "{index} clicks out");
            let seconds = samples.len() as f32 / SAMPLE_RATE as f32;
            assert!((0.2..1.2).contains(&seconds), "{index}: {seconds} s");
        }
        for a in 0..horns.len() {
            for b in a + 1..horns.len() {
                assert!(horns[a][..] != horns[b][..], "{a} and {b} are the same");
            }
        }
    }
}
