// Acid techno, as simply as possible: a kick, hats, a clap and a 303-style bass line (a sawtooth
// through a resonant low-pass whose cutoff an envelope opens on every note, accents and slides),
// arranged over 44 bars at 132 BPM, written to a 16-bit stereo WAV.
//
//     rustc -O tools/film/acid.rs -o target/film/acid.exe && target/film/acid.exe target/film/acid.wav

use std::f32::consts::TAU;
use std::fs::File;
use std::io::{BufWriter, Write};

const RATE: f32 = 44_100.0;
const BPM: f32 = 132.0;
const BARS: usize = 44;

struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

// One step of the bass line: semitones from A1 (None is a rest), accent, slide into the next.
type Step = (Option<i32>, bool, bool);

fn pattern() -> [Step; 16] {
    [
        (Some(0), true, false),
        (Some(0), false, false),
        (Some(12), false, true),
        (Some(0), false, false),
        (None, false, false),
        (Some(3), true, false),
        (Some(0), false, false),
        (Some(15), false, true),
        (Some(12), true, false),
        (Some(0), false, false),
        (Some(7), false, true),
        (Some(5), false, false),
        (None, false, false),
        (Some(0), true, false),
        (Some(10), false, true),
        (Some(12), false, false),
    ]
}

/// The second line, for the second half.
fn pattern_b() -> [Step; 16] {
    let mut steps = pattern();
    steps[4] = (Some(-2), false, true);
    steps[12] = (Some(3), true, false);
    steps[15] = (Some(17), true, false);
    steps
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "acid.wav".into());
    let step_seconds = 60.0 / BPM / 4.0;
    let total_steps = BARS * 16;
    let length = (total_steps as f32 * step_seconds * RATE) as usize + (RATE as usize * 2);
    let mut left = vec![0.0f32; length];
    let mut right = vec![0.0f32; length];
    let mut rng = Rng(0x5eed_ac1d);

    // Which parts play in which bar.
    let kick_on = |bar: usize| (4..24).contains(&bar) || (28..43).contains(&bar);
    let hats_on = |bar: usize| (8..24).contains(&bar) || (28..43).contains(&bar);
    let clap_on = |bar: usize| (12..24).contains(&bar) || (30..43).contains(&bar);
    let bass_on = |bar: usize| bar < 43;

    // Drums, one hit at a time.
    for step in 0..total_steps {
        let bar = step / 16;
        let at = (step as f32 * step_seconds * RATE) as usize;
        let beat = step % 4 == 0;
        if beat && kick_on(bar) {
            let mut phase = 0.0f32;
            for i in 0..(0.4 * RATE) as usize {
                let t = i as f32 / RATE;
                let pitch = 45.0 + 110.0 * (-t * 28.0).exp();
                phase += pitch / RATE;
                let amp = (-t * 7.5).exp() * (t / 0.001).min(1.0);
                let click = if t < 0.004 { (1.0 - t / 0.004) * 0.3 } else { 0.0 };
                let s = (TAU * phase).sin() * amp * 0.95 + click;
                if at + i < length {
                    left[at + i] += s;
                    right[at + i] += s;
                }
            }
        }
        if hats_on(bar) {
            let open = step % 4 == 2;
            let level = if open { 0.09 } else { 0.035 };
            let decay = if open { 30.0 } else { 90.0 };
            let mut last = 0.0f32;
            for i in 0..(0.25 * RATE) as usize {
                let t = i as f32 / RATE;
                let noise = rng.next();
                // A crude high pass: the difference of successive samples.
                let high = noise - last;
                last = noise;
                let s = high * level * (-t * decay).exp();
                if at + i < length {
                    left[at + i] += s * 0.8;
                    right[at + i] += s * 1.1;
                }
            }
        }
        if step % 16 == 4 || step % 16 == 12 {
            if clap_on(bar) {
                let mut band = (0.0f32, 0.0f32);
                for i in 0..(0.3 * RATE) as usize {
                    let t = i as f32 / RATE;
                    // Three quick bursts, then the tail.
                    let burst = [0.0, 0.011, 0.022]
                        .iter()
                        .map(|start| {
                            let since = t - start;
                            if since >= 0.0 { (-since * 140.0).exp() } else { 0.0 }
                        })
                        .fold(0.0f32, f32::max)
                        .max((-t * 22.0).exp() * 0.55);
                    // A band around 1.2 kHz (a state-variable filter).
                    let tune = 2.0 * (std::f32::consts::PI * 1200.0 / RATE).sin();
                    let input = rng.next();
                    band.0 += tune * band.1;
                    let high = input - band.0 - 0.8 * band.1;
                    band.1 += tune * high;
                    let s = band.1 * burst * 0.55;
                    if at + i < length {
                        left[at + i] += s * 1.05;
                        right[at + i] += s * 0.95;
                    }
                }
            }
        }
    }

    // The bass line, sample by sample: a sawtooth, its pitch gliding on slides, through a
    // four-pole resonant low pass.
    let mut phase = 0.0f32;
    let mut pitch = 55.0f32;
    let mut stages = [0.0f32; 4];
    let mut envelope = 0.0f32;
    let mut accent = 0.0f32;
    let mut gate = 0.0f32;
    let steps_per_sample = 1.0 / (step_seconds * RATE);
    let mut current_step = usize::MAX;
    let mut target = 55.0f32;
    let mut sliding = false;
    for i in 0..(total_steps as f32 * step_seconds * RATE) as usize {
        let t = i as f32 / RATE;
        let step = (i as f32 * steps_per_sample) as usize;
        let bar = step / 16;
        let line = if bar >= 28 { pattern_b() } else { pattern() };
        if step != current_step {
            current_step = step;
            let (note, accented, slide) = line[step % 16];
            match note {
                Some(semitones) if bass_on(bar) => {
                    target = 55.0 * 2f32.powf(semitones as f32 / 12.0);
                    if !sliding {
                        pitch = target;
                        envelope = 1.0;
                        accent = if accented { 1.0 } else { 0.0 };
                    }
                    gate = 1.0;
                }
                _ => gate = 0.0,
            }
            sliding = slide;
        }
        pitch += (target - pitch) * (1.0 - (-60.0 / RATE).exp());
        // The step's second half lets go unless it slides on.
        let within = (i as f32 * steps_per_sample).fract();
        let held = if within < 0.6 || sliding { gate } else { 0.0 };
        envelope *= (-1.0 / (0.16 * RATE)).exp();
        accent *= (-1.0 / (0.09 * RATE)).exp();

        phase = (phase + pitch / RATE).fract();
        let saw = 2.0 * phase - 1.0;

        // The filter opens over the track, breathes every 8 bars, and opens wider on accents.
        let progress = (bar as f32 / BARS as f32).min(1.0);
        let sweep = 0.5 - 0.5 * (TAU * t / (8.0 * 16.0 * step_seconds)).cos();
        let base = 180.0 + 900.0 * progress * sweep + if (24..28).contains(&bar) { 1200.0 * sweep } else { 0.0 };
        let cutoff = (base + 2400.0 * envelope + 2600.0 * accent).min(7000.0);
        let g = 1.0 - (-TAU * cutoff / RATE).exp();
        let resonance = 3.4 + 0.4 * accent;
        let input = saw - resonance * stages[3];
        stages[0] += g * (input.tanh() - stages[0]);
        stages[1] += g * (stages[0] - stages[1]);
        stages[2] += g * (stages[1] - stages[2]);
        stages[3] += g * (stages[2] - stages[3]);
        let amp_env = held * (0.55 + 0.45 * accent);
        let s = (stages[3] * 2.2).tanh() * amp_env * 0.45;
        left[i] += s;
        right[i] += s;
    }

    // Fade the end out, soft clip, normalize to just under full scale.
    let fade_from = ((BARS - 2) as f32 * 16.0 * step_seconds * RATE) as usize;
    for i in 0..length {
        let fade = if i > fade_from {
            (1.0 - (i - fade_from) as f32 / (RATE * 4.0)).max(0.0)
        } else {
            1.0
        };
        left[i] = (left[i] * fade * 0.9).tanh();
        right[i] = (right[i] * fade * 0.9).tanh();
    }
    let peak = left
        .iter()
        .chain(&right)
        .fold(0.0f32, |most, s| most.max(s.abs()))
        .max(1e-6);
    let gain = 0.89 / peak;

    let mut file = BufWriter::new(File::create(&out).expect("the WAV can be created"));
    let data_len = (length * 4) as u32;
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap();
    file.write_all(&2u16.to_le_bytes()).unwrap();
    file.write_all(&(RATE as u32).to_le_bytes()).unwrap();
    file.write_all(&(RATE as u32 * 4).to_le_bytes()).unwrap();
    file.write_all(&4u16.to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_len.to_le_bytes()).unwrap();
    for i in 0..length {
        for s in [left[i], right[i]] {
            let v = (s * gain * 32767.0).clamp(-32768.0, 32767.0) as i16;
            file.write_all(&v.to_le_bytes()).unwrap();
        }
    }
    println!(
        "{out}: {:.1} s at {BPM} BPM",
        length as f32 / RATE
    );
}
