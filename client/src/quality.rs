//! How fine the game is drawn, fitted to the machine it runs on (since v1.0.0-rc4, the user's
//! word: "Tu vas analyser à chaque fois et au max possible la machine sur laquelle tu tournes en
//! terme de puissance. Silencieusement [...] Sur mon téléphone moto à 100 euros, ça doit être moche
//! en proche des 30 à 60 fps [...] réglages minimum -> 60 fps sur un smartphone entrée de gamme.
//! Maximum : 60fps avec une 1050 ti en navigateur. Et donc on pourra régler tout cela dans un sous
//! menu des settings").
//!
//! Four levels, LOW to ULTRA. What the game can learn without asking anything, nothing that brings
//! up a prompt: the graphics adapter the renderer was given (its name, whether it is a discrete
//! GPU, an integrated one or software), the processor's threads, on the web the memory the browser
//! admits to, the screen's pixel density and whether it is a touchscreen. That picks a first level
//! ([`detect`]). Then the race itself is measured: with the setting on AUTO, a level that holds the
//! frame rate under [`SLOW_FPS`] for a few seconds steps down, never under LOW and never back up
//! past what was detected ([`adapt`]). The player can also pick a level in the settings, which
//! then stays put.
//!
//! What a level changes ([`Level::look`]): how many pixels the web page draws (a phone's screen
//! is dense, and the cheapest saving is not to fill all of it), the moon's shadows and their
//! resolution, the glow, multisampling, and the city's moving lights.

use bevy::render::view::Msaa;
use bevy::light::DirectionalLightShadowMap;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::window::PrimaryWindow;
use serde::{Deserialize, Serialize};

use crate::camera::{BLOOM, MainCamera};
use crate::preferences::StoredPreferences;
use crate::screen::Screen;
use crate::ui::touch::Touchscreen;

/// How fine the game is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Level {
    Low,
    Medium,
    High,
    Ultra,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Low, Level::Medium, Level::High, Level::Ultra];

    pub fn name(self) -> &'static str {
        match self {
            Level::Low => "LOW",
            Level::Medium => "MEDIUM",
            Level::High => "HIGH",
            Level::Ultra => "ULTRA",
        }
    }

    fn lower(self) -> Option<Level> {
        match self {
            Level::Low => None,
            Level::Medium => Some(Level::Low),
            Level::High => Some(Level::Medium),
            Level::Ultra => Some(Level::High),
        }
    }

    /// What this level draws.
    pub fn look(self) -> Look {
        match self {
            Level::Low => Look {
                pixels: 0.55,
                shadows: None,
                bloom: false,
                msaa: Msaa::Off,
                city_lights: false,
            },
            Level::Medium => Look {
                pixels: 0.8,
                shadows: Some(1024),
                bloom: true,
                msaa: Msaa::Off,
                city_lights: false,
            },
            Level::High => Look {
                pixels: 1.0,
                shadows: Some(2048),
                bloom: true,
                msaa: Msaa::Off,
                city_lights: true,
            },
            Level::Ultra => Look {
                pixels: 1.5,
                shadows: Some(4096),
                bloom: true,
                msaa: Msaa::Sample4,
                city_lights: true,
            },
        }
    }
}

/// What a level draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// On the web, pixels drawn per pixel of the page, at most the screen's own density: under 1
    /// draws fewer and lets the browser stretch them.
    pub pixels: f32,
    /// The size of the moon's shadow map, or no shadows.
    pub shadows: Option<usize>,
    pub bloom: bool,
    pub msaa: Msaa,
    /// The city's traffic and searchlights.
    pub city_lights: bool,
}

/// The player's choice in the settings: fitted to the machine, or a level of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Setting {
    #[default]
    Auto,
    Fixed(Level),
}

impl Setting {
    /// The next choice, as the settings step through them.
    pub fn next(self) -> Self {
        match self {
            Setting::Auto => Setting::Fixed(Level::Low),
            Setting::Fixed(Level::Ultra) => Setting::Auto,
            Setting::Fixed(level) => Setting::Fixed(
                Level::ALL[Level::ALL.iter().position(|l| *l == level).unwrap_or(0) + 1],
            ),
        }
    }
}

/// What the machine looks like, as far as the game can tell without asking.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Machine {
    /// The graphics adapter's name, and what kind it is.
    pub gpu: String,
    pub gpu_kind: GpuKind,
    /// Threads the processor runs at once.
    pub threads: usize,
    /// Memory in gigabytes, where the browser says.
    pub memory: Option<f32>,
    /// The screen's pixels per page pixel.
    pub density: f32,
    pub touch: bool,
    pub web: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuKind {
    Discrete,
    Integrated,
    Software,
    #[default]
    Unknown,
}

/// Below this many frames a second, held a few seconds in a race, AUTO steps down a level.
pub const SLOW_FPS: f32 = 48.0;
/// How long the frame rate must stay slow, in seconds, and how long after a step before another.
const SLOW_FOR: f32 = 3.0;

/// The first level for `machine`: the strongest it can hold at 60 frames a second, from what it
/// says of itself.
pub fn detect(machine: &Machine) -> Level {
    let name = machine.gpu.to_lowercase();
    let strong_name = ["geforce", "rtx", "gtx", "radeon rx", "radeon pro", "arc a"]
        .iter()
        .any(|part| name.contains(part));
    let phone_gpu = ["adreno", "mali", "powervr", "apple gpu", "apple a"]
        .iter()
        .any(|part| name.contains(part));
    let mut level = match machine.gpu_kind {
        GpuKind::Software => Level::Low,
        GpuKind::Discrete => Level::Ultra,
        GpuKind::Integrated if strong_name => Level::Ultra,
        GpuKind::Integrated => Level::Medium,
        // A browser often hides the kind and gives a name, or nothing at all.
        GpuKind::Unknown if strong_name => Level::Ultra,
        GpuKind::Unknown if name.contains("apple m") => Level::High,
        GpuKind::Unknown if name.contains("intel") || name.contains("radeon") => Level::Medium,
        GpuKind::Unknown => Level::High,
    };
    if machine.touch || phone_gpu {
        // A phone or a tablet: a big Adreno or Apple chip holds MEDIUM, most hold LOW.
        let big = ["adreno (tm) 7", "adreno (tm) 8", "apple gpu", "apple a1"]
            .iter()
            .any(|part| name.contains(part));
        level = level.min(if big { Level::Medium } else { Level::Low });
    }
    if machine.threads > 0 && machine.threads <= 4 {
        level = level.min(Level::Medium);
    }
    if machine.memory.is_some_and(|memory| memory <= 4.0) {
        level = level.min(Level::Low);
    }
    level
}

/// The level the game draws at, and what it knows of the machine.
#[derive(Resource, Debug)]
pub struct Quality {
    pub level: Level,
    /// What the machine was taken to hold, which AUTO never goes past.
    pub detected: Level,
    pub machine: Machine,
    /// The look last put in place, so it is put in place again only when it changes.
    applied: Option<(Level, bool)>,
    /// Seconds the frame rate has been slow, and since the last step down.
    slow: f32,
    since_step: f32,
}

pub struct QualityPlugin;

impl Plugin for QualityPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Quality {
            level: Level::High,
            detected: Level::High,
            machine: Machine::default(),
            applied: None,
            slow: 0.0,
            since_step: 0.0,
        })
        .add_systems(Update, (measure_machine, follow_setting, adapt, apply).chain());
    }
}

/// Once the renderer has its adapter: what the machine says of itself, and the level it holds.
fn measure_machine(
    mut quality: ResMut<Quality>,
    adapter: Option<Res<RenderAdapterInfo>>,
    touchscreen: Res<Touchscreen>,
    mut measured: Local<bool>,
) {
    if *measured {
        return;
    }
    let Some(adapter) = adapter else {
        return;
    };
    *measured = true;
    // Read by name: the type is wgpu's own, which the game does not depend on.
    let gpu_kind = match format!("{:?}", adapter.device_type).as_str() {
        "DiscreteGpu" => GpuKind::Discrete,
        "IntegratedGpu" => GpuKind::Integrated,
        "Cpu" => GpuKind::Software,
        _ => GpuKind::Unknown,
    };
    let mut machine = Machine {
        gpu: adapter.name.clone(),
        gpu_kind,
        threads: std::thread::available_parallelism().map_or(0, usize::from),
        memory: None,
        density: 1.0,
        touch: touchscreen.0,
        web: cfg!(target_arch = "wasm32"),
    };
    web_probe(&mut machine);
    let level = detect(&machine);
    info!(
        gpu = %machine.gpu,
        kind = ?machine.gpu_kind,
        threads = machine.threads,
        memory = ?machine.memory,
        density = machine.density,
        touch = machine.touch,
        level = level.name(),
        "graphics fitted to the machine"
    );
    quality.machine = machine;
    quality.detected = level;
    quality.level = level;
}

/// In a browser: the threads, the memory and the pixel density it gives without asking.
#[cfg(target_arch = "wasm32")]
fn web_probe(machine: &mut Machine) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let navigator = window.navigator();
    machine.threads = navigator.hardware_concurrency() as usize;
    machine.memory = js_sys::Reflect::get(&navigator, &"deviceMemory".into())
        .ok()
        .and_then(|value| value.as_f64())
        .map(|memory| memory as f32);
    machine.density = window.device_pixel_ratio() as f32;
}

#[cfg(not(target_arch = "wasm32"))]
fn web_probe(_: &mut Machine) {}

/// A level picked in the settings holds; AUTO goes back to what the machine was taken to hold.
fn follow_setting(
    mut quality: ResMut<Quality>,
    preferences: Res<StoredPreferences>,
    mut last: Local<Option<Setting>>,
) {
    let setting = preferences.get().graphics;
    if *last == Some(setting) {
        return;
    }
    *last = Some(setting);
    quality.level = match setting {
        Setting::Auto => quality.detected,
        Setting::Fixed(level) => level,
    };
    quality.slow = 0.0;
    quality.since_step = 0.0;
}

/// On AUTO, in a race, a frame rate held under [`SLOW_FPS`] steps the level down.
fn adapt(
    mut quality: ResMut<Quality>,
    preferences: Res<StoredPreferences>,
    screen: Res<State<Screen>>,
    time: Res<Time<Real>>,
) {
    if preferences.get().graphics != Setting::Auto || *screen.get() != Screen::Lobby {
        quality.slow = 0.0;
        return;
    }
    let seconds = time.delta_secs();
    if seconds <= 0.0 || seconds > 0.5 {
        // A hitch, loading or a hidden tab: not the frame rate.
        return;
    }
    quality.since_step += seconds;
    if 1.0 / seconds < SLOW_FPS {
        quality.slow += seconds;
    } else {
        quality.slow = (quality.slow - seconds * 0.5).max(0.0);
    }
    if quality.slow >= SLOW_FOR
        && quality.since_step >= SLOW_FOR * 2.0
        && let Some(lower) = quality.level.lower()
    {
        info!(from = quality.level.name(), to = lower.name(), "frame rate too low: drawing less");
        quality.level = lower;
        quality.slow = 0.0;
        quality.since_step = 0.0;
    }
}

/// Puts the level's look in place: the camera, the moon, the page's pixels, the city's lights.
#[allow(clippy::too_many_arguments)]
fn apply(
    mut commands: Commands,
    mut quality: ResMut<Quality>,
    screen: Res<State<Screen>>,
    cameras: Query<Entity, With<MainCamera>>,
    mut lights: Query<&mut DirectionalLight>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut city: Query<&mut Visibility, With<crate::city::CityLight>>,
    new_city: Query<(), Added<crate::city::CityLight>>,
) {
    let in_race = *screen.get() == Screen::Lobby;
    let key = (quality.level, in_race);
    if quality.applied == Some(key) && new_city.is_empty() {
        return;
    }
    quality.applied = Some(key);
    let look = quality.level.look();
    for camera in &cameras {
        let mut camera = commands.entity(camera);
        camera.insert(look.msaa);
        if look.bloom {
            camera.insert(BLOOM);
        } else {
            camera.remove::<Bloom>();
        }
    }
    for mut light in &mut lights {
        light.shadow_maps_enabled = look.shadows.is_some();
    }
    commands.insert_resource(DirectionalLightShadowMap {
        size: look.shadows.unwrap_or(1024),
    });
    for mut visibility in &mut city {
        visibility.set_if_neq(if look.city_lights {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    // Only a page draws fewer pixels than its screen has: a native window keeps its own.
    if quality.machine.web {
        let wanted = (look.pixels * quality.machine.density.max(1.0)).clamp(0.5, quality.machine.density.max(1.0));
        for mut window in &mut windows {
            if window.resolution.scale_factor_override() != Some(wanted) {
                window.resolution.set_scale_factor_override(Some(wanted));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(gpu: &str, kind: GpuKind, threads: usize, touch: bool) -> Machine {
        Machine {
            gpu: gpu.into(),
            gpu_kind: kind,
            threads,
            memory: None,
            density: 1.0,
            touch,
            web: false,
        }
    }

    /// The user's two ends: a GTX 1050 Ti draws everything, a cheap phone the least.
    #[test]
    fn the_machine_picks_its_level() {
        let desk = machine("NVIDIA GeForce GTX 1050 Ti", GpuKind::Discrete, 6, false);
        assert_eq!(detect(&desk), Level::Ultra);
        // In a browser the kind is often hidden, the name not.
        let browser = machine("ANGLE (NVIDIA, NVIDIA GeForce GTX 1050 Ti Direct3D11)", GpuKind::Unknown, 6, false);
        assert_eq!(detect(&browser), Level::Ultra);
        let phone = machine("Adreno (TM) 610", GpuKind::Unknown, 8, true);
        assert_eq!(detect(&phone), Level::Low);
        let laptop = machine("Intel(R) UHD Graphics 620", GpuKind::Integrated, 8, false);
        assert_eq!(detect(&laptop), Level::Medium);
        let software = machine("llvmpipe", GpuKind::Software, 8, false);
        assert_eq!(detect(&software), Level::Low);
        let mut small = machine("Mali-G57", GpuKind::Unknown, 8, true);
        small.memory = Some(3.0);
        assert_eq!(detect(&small), Level::Low);
    }

    /// The settings step through AUTO and every level, and back.
    #[test]
    fn the_setting_steps_round() {
        let mut setting = Setting::Auto;
        let mut seen = Vec::new();
        for _ in 0..5 {
            setting = setting.next();
            seen.push(setting);
        }
        assert_eq!(seen.last(), Some(&Setting::Auto));
        assert_eq!(seen[3], Setting::Fixed(Level::Ultra));
        // Each level draws at least what the one under it does.
        for pair in Level::ALL.windows(2) {
            let (low, high) = (pair[0].look(), pair[1].look());
            assert!(high.pixels >= low.pixels);
            assert!(high.shadows.unwrap_or(0) >= low.shadows.unwrap_or(0));
        }
    }
}
