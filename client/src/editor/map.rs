//! The editor's map: the circuit built by the race's own code and seen from above, the camera
//! that looks at it, clicks on it, and what is drawn over it: the part selected, the cursor, and
//! the stretch the editor added to close the road.

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::light::NotShadowCaster;
use bevy::pbr::DistanceFog;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use star_racer_sim::track::Track;
use star_racer_sim::track::editing;

use super::model::{self, Item, Tab};
use super::{Editor, panel};
use crate::camera::MainCamera;
use crate::geometry::Geometry;
use crate::ui::touch::Touchscreen;
use crate::world;

/// Everything the editor draws in the world, which goes when it closes.
#[derive(Component, Clone)]
pub struct EditorScene;

/// What is drawn over the circuit, rebuilt whenever the selection or the cursor moves.
#[derive(Component, Clone)]
pub(super) struct Overlay;

/// Where the camera looks from: the point of the plane it looks at, how high over it it stands,
/// and how far round it has turned.
#[derive(Resource)]
pub struct View {
    center: Vec2,
    height: f32,
    framed: bool,
}

impl Default for View {
    fn default() -> Self {
        Self {
            center: Vec2::ZERO,
            height: 600.0,
            framed: false,
        }
    }
}

/// The camera leans this far off straight down, so the relief reads, as a share of its height.
const LEAN: f32 = 0.45;
/// How high over the road what is drawn over it floats, in meters.
const OVER: f32 = 0.6;

/// Builds the circuit's scene again when the draft changed: the race's road, walls, markings and
/// scenery when it builds; the line its road draws when it does not, in red.
pub(super) fn rebuild(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scenes: Query<Entity, (With<EditorScene>, Without<Overlay>)>,
    existing_view: Option<Res<View>>,
) {
    if existing_view.is_none() {
        commands.insert_resource(View::default());
    }
    if !editor.scene_dirty {
        return;
    }
    editor.scene_dirty = false;
    for scene in &scenes {
        commands.entity(scene).despawn();
    }
    match &editor.built {
        Ok(built) => {
            crate::track::spawn_scene(
                &mut commands,
                &mut meshes,
                &mut materials,
                &built.track,
                &built.description.scenery,
                EditorScene,
            );
        }
        Err(_) => {
            // The road as far as it goes, so the player sees what does not close.
            let Ok(trace) = editing::trace(&editor.draft, 4.0) else {
                return;
            };
            let mut lines = Geometry::default();
            line(&mut lines, &trace.points, 0.0, 3.0, Color::srgb(1.0, 0.25, 0.3));
            commands.spawn((
                EditorScene,
                Mesh3d(meshes.add(lines.into_mesh())),
                MeshMaterial3d(materials.add(StandardMaterial {
                    unlit: true,
                    ..default()
                })),
                NotShadowCaster,
            ));
        }
    }
}

/// A flat band `width` meters wide along `points`, `height` meters up.
fn line(geometry: &mut Geometry, points: &[Vec2], height: f32, width: f32, color: Color) {
    for pair in points.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let Some(direction) = (to - from).try_normalize() else {
            continue;
        };
        let side = direction.perp() * width / 2.0;
        geometry.quad_facing_up(
            [
                world::position(from - side, height),
                world::position(to - side, height),
                world::position(to + side, height),
                world::position(from + side, height),
            ],
            color,
        );
    }
}

/// The wheel zooms, the right or middle button held drags the map, the arrows and WASD pan; the
/// first time the editor opens, the camera frames the whole circuit.
pub fn steer_camera(
    mut commands: Commands,
    mut view: Option<ResMut<View>>,
    mut editor: ResMut<Editor>,
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    motion: Res<AccumulatedMouseMotion>,
    scroll: Res<AccumulatedMouseScroll>,
    time: Res<Time>,
    touchscreen: Res<Touchscreen>,
    mut cameras: Query<(Entity, &mut Transform, &mut Projection), With<MainCamera>>,
) {
    let Some(view) = view.as_mut() else {
        return;
    };
    if std::mem::take(&mut editor.reframe) {
        view.framed = false;
    }
    if !view.framed
        && let Ok(built) = &editor.built
    {
        let (low, high) = bounds(&built.track);
        let window = windows.single().map_or(Vec2::new(1280.0, 720.0), Window::size);
        let map = (window.x - panel::width(*touchscreen)).max(200.0);
        let size = high - low;
        // High enough for the circuit to fit the map, the part of the window right of the panel.
        view.height = (size.y.max(size.x * window.y / map) * 1.15).clamp(150.0, 6000.0);
        // Its middle in the middle of the map rather than of the window.
        let meters = view.height / window.y * 1.1;
        view.center = (low + high) / 2.0 - Vec2::X * panel::width(*touchscreen) / 2.0 * meters;
        view.framed = true;
    }
    let over_map = windows
        .single()
        .ok()
        .and_then(Window::cursor_position)
        .is_some_and(|cursor| cursor.x > panel::width(*touchscreen));
    if over_map && scroll.delta.y != 0.0 {
        let notches = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        view.height = (view.height * 0.85f32.powf(notches)).clamp(40.0, 6000.0);
    }
    // Pixels of the window to meters of the plane, about.
    let meters = view.height / windows.single().map_or(720.0, |window| window.height()) * 1.1;
    if over_map && (buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle)) {
        view.center += Vec2::new(-motion.delta.x, motion.delta.y) * meters;
    }
    let pan = Vec2::new(
        f32::from(u8::from(keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight])))
            - f32::from(u8::from(keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]))),
        f32::from(u8::from(keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp])))
            - f32::from(u8::from(keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]))),
    );
    let height = view.height;
    view.center += pan * height * time.delta_secs();

    for (camera, mut transform, mut projection) in &mut cameras {
        // Seen from above there is no distance to fade out.
        commands.entity(camera).remove::<DistanceFog>();
        let target = world::position(view.center, 0.0);
        let eye = target + Vec3::new(0.0, view.height, view.height * LEAN);
        transform.set_if_neq(Transform::from_translation(eye).looking_at(target, Vec3::NEG_Z));
        if let Projection::Perspective(perspective) = projection.as_mut() {
            perspective.fov = 50f32.to_radians();
            perspective.far = (view.height * 6.0).max(3000.0);
        }
    }
}

/// The corners of the plane the circuit covers.
fn bounds(track: &Track) -> (Vec2, Vec2) {
    track.points().iter().fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(low, high), point| (low.min(point.position), high.max(point.position)),
    )
}

/// Where the mouse is over the road, as a place along the lap, or `None` off the map or the road.
fn under_mouse(
    editor: &Editor,
    windows: &Query<&Window, With<PrimaryWindow>>,
    cameras: &Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    touchscreen: Touchscreen,
) -> Option<f32> {
    let cursor = windows.single().ok().and_then(Window::cursor_position)?;
    if cursor.x <= panel::width(touchscreen) {
        return None;
    }
    let (camera, transform) = cameras.single().ok()?;
    let track = &editor.built.as_ref().ok()?.track;
    let ray = camera.viewport_to_world(transform, cursor).ok()?;
    // Where the ray meets the road: first the ground at sea level, then the height of the road
    // nearest that, twice more, which is close enough for a road that climbs gently.
    let mut height = 0.0;
    let mut hit = None;
    for _ in 0..3 {
        let distance = ray.intersect_plane(Vec3::Y * height, InfinitePlane3d::new(Vec3::Y))?;
        let point = ray.get_point(distance);
        let projection = track.project(Vec2::new(point.x, -point.z));
        height = track.height_at(&projection);
        hit = Some(projection);
    }
    let projection = hit?;
    (projection.lateral.abs() <= track.half_width_at(projection.distance) + 60.0)
        .then_some(projection.distance)
}

/// What the mouse is doing on the map: the part under it, and the one being dragged.
#[derive(Default)]
pub(super) struct Pointer {
    dragging: Option<Item>,
    moved: bool,
}

/// The mouse on the map (since v1.0.0-rc5, the user's word "Améliore l'éditeur, c'est pas
/// intuitif du tout"): the part of the open tab nearest it is lit as it passes; a left click puts
/// the cursor on the road and selects that part; held on a part placed along the lap, it drags it
/// along the road, and letting go builds the circuit again.
pub(super) fn pointer(
    mut editor: ResMut<Editor>,
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    cameras: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    touchscreen: Res<Touchscreen>,
    mut state: Local<Pointer>,
) {
    let at = under_mouse(&editor, &windows, &cameras, *touchscreen);
    // Let go: the part dragged is where it was dropped, and the circuit is built with it there.
    if state.dragging.is_some() && !buttons.pressed(MouseButton::Left) {
        state.dragging = None;
        if std::mem::take(&mut state.moved) {
            editor.changed();
        }
        return;
    }
    if let (Some(item), Some(at)) = (state.dragging, at) {
        let along = model::fields(&editor.draft, item)
            .iter()
            .position(|field| field.label == "ALONG");
        if let Some(field) = along
            && model::at(&editor.draft, item).is_some_and(|now| (now - at).abs() > 0.5)
        {
            let item = model::set(&mut editor.draft, item, field, at.round());
            editor.selected = Some(item);
            state.dragging = Some(item);
            state.moved = true;
            editor.cursor = Some(at);
            editor.overlay_dirty = true;
            editor.panel_dirty = true;
        }
        return;
    }
    let hovered = at.and_then(|at| nearest(&editor, at, false));
    if hovered != editor.hovered {
        editor.hovered = hovered;
        editor.overlay_dirty = true;
    }
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(at) = at else {
        return;
    };
    editor.cursor = Some(at);
    editor.selected = nearest(&editor, at, true);
    // Pressed on a part placed along the lap: it follows the mouse until let go.
    if let Some(item) = editor.selected
        && model::at(&editor.draft, item).is_some_and(|place| (place - at).abs() < 30.0)
    {
        state.dragging = Some(item);
    }
    editor.panel_dirty = true;
    editor.overlay_dirty = true;
}

/// The part of the open tab nearest `at` meters along the lap.
fn nearest(editor: &Editor, at: f32, keep: bool) -> Option<Item> {
    // When nothing is near, a click keeps the selection; the mouse passing lights nothing.
    let fallback = if keep { editor.selected } else { None };
    if editor.tab == Tab::Road {
        let count = editor.draft.segments.len();
        return (0..count)
            .find(|index| {
                let (start, end) = editor.segment_span(*index);
                (start..end).contains(&at)
            })
            .map(Item::Segment)
            .or(fallback);
    }
    model::items(&editor.draft, editor.tab)
        .into_iter()
        .filter_map(|item| model::at(&editor.draft, item).map(|place| (item, (place - at).abs())))
        .filter(|(_, off)| *off < 80.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(item, _)| item)
        .or(fallback)
}

/// Draws over the circuit the part selected (a segment's whole stretch, or a bar across the road
/// where a part stands), the cursor, and the stretch the editor added to close the road.
pub(super) fn draw_overlay(
    mut commands: Commands,
    mut editor: ResMut<Editor>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    overlays: Query<Entity, With<Overlay>>,
) {
    if !editor.overlay_dirty {
        return;
    }
    editor.overlay_dirty = false;
    for overlay in &overlays {
        commands.entity(overlay).despawn();
    }
    let Ok(built) = &editor.built else {
        return;
    };
    let track = &built.track;
    let lap = track.length();
    let mut geometry = Geometry::default();
    let stretch = |geometry: &mut Geometry, from: f32, to: f32, width: f32, color: Color| {
        let mut points = Vec::new();
        let mut at = from;
        while at <= to {
            let point = track.point_at(at.rem_euclid(lap));
            points.push(point.position);
            at += 2.0;
        }
        let height = track.point_at(from.rem_euclid(lap)).height;
        line(geometry, &points, height + OVER, width, color);
    };
    let across = |geometry: &mut Geometry, at: f32, color: Color| {
        let point = track.point_at(at.rem_euclid(lap));
        let normal = point.direction.perp() * (point.half_width + 4.0);
        line(
            geometry,
            &[point.position - normal, point.position + normal],
            point.height + OVER,
            3.0,
            color,
        );
    };
    // The editor's own stretch: from the end of the player's road to the line.
    if built.closing > 0 {
        let count = editor.draft.segments.len();
        let (_, end) = editor.segment_span(count.saturating_sub(1));
        stretch(&mut geometry, end, lap, 2.0, Color::srgb(0.5, 0.55, 1.0));
    }
    // What the mouse is over, quieter than what is selected.
    let hover = Color::srgb(0.45, 0.5, 0.6);
    match editor.hovered.filter(|hovered| Some(*hovered) != editor.selected) {
        Some(Item::Segment(index)) => {
            let (from, to) = editor.segment_span(index);
            stretch(&mut geometry, from, to, 4.0, hover);
        }
        Some(item) => {
            if let Some(at) = model::at(&editor.draft, item) {
                across(&mut geometry, at, hover);
            }
        }
        None => {}
    }
    match editor.selected {
        Some(Item::Segment(index)) => {
            let (from, to) = editor.segment_span(index);
            stretch(&mut geometry, from, to, 6.0, Color::WHITE);
        }
        Some(item) => {
            if let Some(at) = model::at(&editor.draft, item) {
                across(&mut geometry, at, Color::WHITE);
            }
        }
        None => {}
    }
    if let Some(at) = editor.cursor {
        across(&mut geometry, at, Color::srgb(1.0, 0.35, 0.8));
    }
    if geometry.triangles() == 0 {
        return;
    }
    commands.spawn((
        EditorScene,
        Overlay,
        Mesh3d(meshes.add(geometry.into_mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            unlit: true,
            ..default()
        })),
        NotShadowCaster,
    ));
}
