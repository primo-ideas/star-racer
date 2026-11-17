//! A sound worked out once, sample by sample, and played back as it is: for a sound no [`Tone`]
//! can make, such as the bang of an engine firing, whose body comes from a pipe ringing rather
//! than from partials.
//!
//! [`Tone`]: super::tone::Tone

use std::f32::consts::TAU;
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use bevy::audio::{ChannelCount, Decodable, Sample, SampleRate, Source};
use bevy::prelude::*;

use super::tone::{Filter, Hiss, SAMPLE_RATE};

/// Samples worked out in advance, played once.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct Recorded(pub Arc<[f32]>);

impl Decodable for Recorded {
    type Decoder = RecordedSamples;

    fn decoder(&self) -> Self::Decoder {
        RecordedSamples {
            samples: self.0.clone(),
            next: 0,
        }
    }
}

pub struct RecordedSamples {
    samples: Arc<[f32]>,
    next: usize,
}

impl Iterator for RecordedSamples {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.samples.get(self.next).copied();
        self.next += 1;
        sample
    }
}

impl Source for RecordedSamples {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        NonZero::new(1).expect("one channel")
    }

    fn sample_rate(&self) -> SampleRate {
        NonZero::new(SAMPLE_RATE).expect("a sample rate above zero")
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(
            self.samples.len() as f32 / SAMPLE_RATE as f32,
        ))
    }
}

/// How long the bang lasts, in seconds.
const BANG_SECONDS: f32 = 0.42;
/// The exhaust pipe: the note it rings at, in Hz, how much of each echo comes back, and how much
/// of its brightness each echo keeps (0 dull, 1 all of it).
const PIPE_PITCH: f32 = 96.0;
const PIPE_FEEDBACK: f32 = 0.8;
const PIPE_BRIGHTNESS: f32 = 0.45;
/// The second firing, fainter, one turn of the crank later at a rev the engine settles back to:
/// what makes it an engine rather than a gunshot.
const AFTER_FIRE: f32 = 0.125;
const AFTER_FIRE_LEVEL: f32 = 0.22;
/// How hard the whole is driven into soft clipping: the bark of an exhaust.
const DRIVE: f32 = 2.2;
/// How loud the bang is at its peak, beside the tones' 0.35.
const PEAK: f32 = 0.5;
/// Seconds over which it fades in and out, so it neither starts nor stops on a click.
const EDGE: f32 = 0.0004;
const RELEASE: f32 = 0.03;

/// "PAF": the bang of a single-cylinder four-stroke firing, the hop's sound. A pressure pulse with
/// the crack of the spark in it rings the exhaust pipe, a low thump of the engine block under it
/// and a breath of exhaust after it, and a faint second firing a crank turn later.
pub fn four_stroke_bang() -> Recorded {
    let rate = SAMPLE_RATE as f32;
    let count = (BANG_SECONDS * rate) as usize;
    let mut hiss = Hiss::new(0xba46);
    let mut breath = Filter::default();
    let delay = (rate / PIPE_PITCH).round() as usize;
    let mut pipe = vec![0.0_f32; count];
    let mut damped = 0.0_f32;
    let mut thump_phase = 0.0_f32;
    let mut samples = Vec::with_capacity(count);

    for index in 0..count {
        let seconds = index as f32 / rate;
        let noise = hiss.next();
        // Each firing: a pressure pulse rising in half a millisecond and gone in a few, and the
        // crack of the burn, broadband and shorter still.
        let firing = |at: f32| {
            let since = seconds - at;
            if since < 0.0 {
                return 0.0;
            }
            let pulse = (since / 0.0005).min(1.0) * (-since / 0.004).exp();
            let crack = (-since / 0.0025).exp();
            pulse + 0.7 * crack * noise
        };
        let excitation = firing(0.0) + AFTER_FIRE_LEVEL * firing(AFTER_FIRE);

        // The pipe: what went in comes back a period later, a little duller each time.
        let echo = if index >= delay { pipe[index - delay] } else { 0.0 };
        damped += PIPE_BRIGHTNESS * (echo - damped);
        pipe[index] = excitation + PIPE_FEEDBACK * damped;

        // The block: a low thump falling in pitch.
        let thump_pitch = 72.0 * (0.62_f32).powf(seconds / BANG_SECONDS);
        thump_phase = (thump_phase + thump_pitch / rate).fract();
        let thump = (TAU * thump_phase).sin() * (-seconds * 20.0).exp();

        // The breath: exhaust rushing out after the bang, a band of noise darkening as it goes.
        let band = 1100.0 * (0.4_f32).powf(seconds / BANG_SECONDS);
        let (_, rush) = breath.run(noise, band, 0.9);
        let rush = rush * (seconds / 0.006).min(1.0) * (-seconds * 11.0).exp();

        samples.push(0.6 * pipe[index] + 0.3 * thump + 0.35 * rush);
    }

    // Driven into a soft clip for the bark, then brought to its peak, faded in and out.
    let loudest = samples.iter().fold(0.0_f32, |most, sample| most.max(sample.abs()));
    let scale = if loudest > 0.0 { 1.0 / loudest } else { 0.0 };
    let total = count as f32 / rate;
    let samples: Arc<[f32]> = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let seconds = index as f32 / rate;
            let edges = (seconds / EDGE).min(1.0) * ((total - seconds) / RELEASE).clamp(0.0, 1.0);
            PEAK * (DRIVE * sample * scale).tanh() / DRIVE.tanh() * edges
        })
        .collect();
    Recorded(samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bang starts and ends silent, never clips, is loud at once and dies away: most of its
    /// energy is in its first tenth of a second.
    #[test]
    fn the_bang_is_sharp_and_silent_at_both_ends() {
        let Recorded(samples) = four_stroke_bang();
        assert_eq!(samples[0], 0.0);
        assert!(samples.last().unwrap().abs() < 1e-3);
        assert!(samples.iter().all(|sample| sample.abs() <= PEAK + 1e-4));
        let energy = |range: std::ops::Range<f32>| {
            let rate = SAMPLE_RATE as f32;
            samples[(range.start * rate) as usize..(range.end * rate) as usize]
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>()
        };
        let peak_at = samples
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .map(|(index, _)| index as f32 / SAMPLE_RATE as f32)
            .unwrap();
        assert!(peak_at < 0.02, "the bang peaks late, at {peak_at} s");
        let (head, tail) = (energy(0.0..0.1), energy(0.1..BANG_SECONDS));
        assert!(head > 1.5 * tail, "{head} {tail}");
        assert!(energy(0.3..0.4) < 0.05 * head, "it rings on");
    }

    /// The pipe rings: the bang has a pitch, near the pipe's note, rather than being a click.
    #[test]
    fn the_pipe_rings_at_its_note() {
        let Recorded(samples) = four_stroke_bang();
        let rate = SAMPLE_RATE as f32;
        let stretch = &samples[(0.02 * rate) as usize..(0.1 * rate) as usize];
        // How strongly the stretch repeats after one period of the pipe, against half of one.
        let likeness = |lag: usize| {
            stretch
                .iter()
                .zip(&stretch[lag..])
                .map(|(a, b)| a * b)
                .sum::<f32>()
        };
        let period = (rate / PIPE_PITCH).round() as usize;
        assert!(likeness(period) > 0.0);
        assert!(likeness(period) > likeness(period / 2), "no ring at the pipe's note: {} {}", likeness(period), likeness(period / 2));
    }

    /// Writes the bang to `target/sounds/hop.wav`, to hear it outside the game:
    /// `cargo test --workspace -- --ignored write_the_bang`.
    #[test]
    #[ignore]
    fn write_the_bang() {
        let Recorded(samples) = four_stroke_bang();
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/sounds");
        std::fs::create_dir_all(&directory).unwrap();
        let data: Vec<u8> = samples
            .iter()
            .flat_map(|sample| ((sample.clamp(-1.0, 1.0) * 32_767.0) as i16).to_le_bytes())
            .collect();
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        std::fs::write(directory.join("hop.wav"), wav).unwrap();
    }
}
