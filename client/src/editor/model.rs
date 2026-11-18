//! What the level editor edits, apart from how it is shown: the parts of a circuit, grouped in
//! tabs, each with the numbers it is made of, how far a step moves each one, and the bounds it
//! keeps within. Adding, removing and changing a part goes through here, and keeps the lists the
//! track file wants in order (the heights and the checkpoints) in order.

use star_racer_sim::track::scenery::{Prop, Side};
use star_racer_sim::track::{Breach, Elevation, Gap, Narrows, Pad, Segment, TrackDescription};

/// The editor's tabs, each a kind of part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tab {
    /// The road's segments, and its width.
    #[default]
    Road,
    /// The heights the road climbs and dives through.
    Relief,
    /// Gaps, breaches, bottlenecks, pads and checkpoints.
    Game,
    /// The props beside the road.
    Scenery,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Road, Tab::Relief, Tab::Game, Tab::Scenery];

    pub fn name(self) -> &'static str {
        match self {
            Tab::Road => "ROAD",
            Tab::Relief => "RELIEF",
            Tab::Game => "GAME",
            Tab::Scenery => "SCENERY",
        }
    }
}

/// One part of the circuit, by its place in its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Item {
    /// The road as a whole: its width.
    Road,
    Segment(usize),
    Height(usize),
    Narrows(usize),
    Gap(usize),
    Breach(usize),
    Pad(usize),
    Checkpoint(usize),
    Prop(usize),
}

/// What can be added, in each tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum New {
    Straight,
    Left,
    Right,
    Height,
    Gap,
    Breach,
    Narrows,
    Pad,
    Checkpoint,
    Prop(PropKind),
}

impl New {
    pub fn of(tab: Tab) -> Vec<New> {
        match tab {
            Tab::Road => vec![New::Straight, New::Left, New::Right],
            Tab::Relief => vec![New::Height],
            Tab::Game => vec![New::Gap, New::Breach, New::Narrows, New::Pad, New::Checkpoint],
            Tab::Scenery => PropKind::ALL.iter().map(|kind| New::Prop(*kind)).collect(),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            New::Straight => "STRAIGHT",
            New::Left => "LEFT TURN",
            New::Right => "RIGHT TURN",
            New::Height => "HEIGHT",
            New::Gap => "GAP",
            New::Breach => "BREACH",
            New::Narrows => "NARROWS",
            New::Pad => "PAD",
            New::Checkpoint => "CHECKPOINT",
            New::Prop(kind) => kind.name(),
        }
    }
}

/// The kinds of prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropKind {
    Gantry,
    Pylon,
    Grandstand,
    Billboard,
    Chevrons,
    Monolith,
    Tower,
    Skyline,
    Facade,
    Tunnel,
}

impl PropKind {
    pub const ALL: [PropKind; 10] = [
        PropKind::Pylon,
        PropKind::Chevrons,
        PropKind::Billboard,
        PropKind::Grandstand,
        PropKind::Tower,
        PropKind::Skyline,
        PropKind::Facade,
        PropKind::Monolith,
        PropKind::Gantry,
        PropKind::Tunnel,
    ];

    pub fn name(self) -> &'static str {
        match self {
            PropKind::Gantry => "GANTRY",
            PropKind::Pylon => "PYLON",
            PropKind::Grandstand => "GRANDSTAND",
            PropKind::Billboard => "BILLBOARD",
            PropKind::Chevrons => "CHEVRONS",
            PropKind::Monolith => "MONOLITH",
            PropKind::Tower => "TOWER",
            PropKind::Skyline => "DISTRICT",
            PropKind::Facade => "FACADE",
            PropKind::Tunnel => "TUNNEL",
        }
    }

    pub fn of(prop: &Prop) -> Self {
        match prop {
            Prop::Gantry { .. } => PropKind::Gantry,
            Prop::Pylon { .. } => PropKind::Pylon,
            Prop::Grandstand { .. } => PropKind::Grandstand,
            Prop::Billboard { .. } => PropKind::Billboard,
            Prop::Chevrons { .. } => PropKind::Chevrons,
            Prop::Monolith { .. } => PropKind::Monolith,
            Prop::Tower { .. } => PropKind::Tower,
            Prop::Skyline { .. } => PropKind::Skyline,
            Prop::Facade { .. } => PropKind::Facade,
            Prop::Tunnel { .. } => PropKind::Tunnel,
        }
    }

    /// A new prop of this kind `at` meters along the lap, on the left, with the sizes a track file
    /// gets when it writes none: read through the file format itself, so they are the same.
    fn new(self, at: f32) -> Prop {
        let side = match self {
            PropKind::Gantry | PropKind::Tunnel => "",
            _ => ", side: Left",
        };
        let kind = match self {
            PropKind::Skyline => "Skyline",
            other => {
                let name = other.name();
                // "PYLON" → "Pylon".
                return ron::from_str(&format!(
                    "{}{}(at: {at:.1}{side})",
                    &name[..1],
                    name[1..].to_lowercase()
                ))
                .expect("every prop has defaults for all but its place");
            }
        };
        ron::from_str(&format!("{kind}(at: {at:.1}{side})")).expect("a district has defaults")
    }
}

/// One number of a part, as the editor shows and steps it.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub label: &'static str,
    pub value: f32,
    /// What one press moves it by; a long press, ten times that.
    pub step: f32,
    pub min: f32,
    pub max: f32,
    /// How it is written: meters, degrees, a count, or a side.
    pub unit: Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Meters,
    Degrees,
    Count,
    /// 1 on the left, -1 on the right.
    Side,
}

impl Field {
    /// The value as the editor writes it.
    pub fn text(&self) -> String {
        match self.unit {
            Unit::Meters => format!("{:.0} m", self.value),
            Unit::Degrees => format!("{:.0}°", self.value),
            Unit::Count => format!("{:.0}", self.value),
            Unit::Side => if self.value > 0.0 { "LEFT" } else { "RIGHT" }.to_owned(),
        }
    }
}

/// A number of a part, as stored.
enum Slot<'a> {
    Float(&'a mut f32),
    Byte(&'a mut u8),
    Side(&'a mut Side),
}

/// Every number of `item`, with where it is kept: read for [`fields`], written for [`set`].
fn slots<'a>(description: &'a mut TrackDescription, item: Item) -> Vec<(Field, Slot<'a>)> {
    let meters = |label, value: f32, step, min, max| Field {
        label,
        value,
        step,
        min,
        max,
        unit: Unit::Meters,
    };
    let along = |value| meters("ALONG", value, 10.0, 0.0, 20_000.0);
    let side_field = |side: Side| Field {
        label: "SIDE",
        value: side.sign(),
        step: 2.0,
        min: -1.0,
        max: 1.0,
        unit: Unit::Side,
    };
    let half_width = description.width / 2.0;
    match item {
        Item::Road => {
            let value = description.width;
            vec![(
                meters("WIDTH", value, 1.0, 8.0, 80.0),
                Slot::Float(&mut description.width),
            )]
        }
        Item::Segment(index) => match description.segments.get_mut(index) {
            Some(Segment::Straight { length }) => {
                vec![(meters("LENGTH", *length, 10.0, 1.0, 5000.0), Slot::Float(length))]
            }
            Some(Segment::Turn {
                angle,
                radius,
                transition,
                banking,
            }) => vec![
                (
                    Field {
                        label: "ANGLE",
                        value: *angle,
                        step: 5.0,
                        min: -720.0,
                        max: 720.0,
                        unit: Unit::Degrees,
                    },
                    Slot::Float(angle),
                ),
                (
                    meters("RADIUS", *radius, 5.0, half_width + 1.0, 2000.0),
                    Slot::Float(radius),
                ),
                (
                    meters("TRANSITION", *transition, 5.0, 0.0, 500.0),
                    Slot::Float(transition),
                ),
                (
                    Field {
                        label: "BANKING",
                        value: *banking,
                        step: 2.0,
                        min: 0.0,
                        max: 60.0,
                        unit: Unit::Degrees,
                    },
                    Slot::Float(banking),
                ),
            ],
            None => Vec::new(),
        },
        Item::Height(index) => match description.elevation.get_mut(index) {
            Some(Elevation { at, height }) => vec![
                (along(*at), Slot::Float(at)),
                (meters("HEIGHT", *height, 1.0, -200.0, 400.0), Slot::Float(height)),
            ],
            None => Vec::new(),
        },
        Item::Narrows(index) => match description.narrows.get_mut(index) {
            Some(Narrows {
                at,
                length,
                width,
                blend,
            }) => vec![
                (along(*at), Slot::Float(at)),
                (meters("LENGTH", *length, 5.0, 1.0, 2000.0), Slot::Float(length)),
                (meters("WIDTH", *width, 1.0, 4.0, 80.0), Slot::Float(width)),
                (meters("BLEND", *blend, 2.0, 0.0, 200.0), Slot::Float(blend)),
            ],
            None => Vec::new(),
        },
        Item::Gap(index) => match description.gaps.get_mut(index) {
            Some(Gap { at, length }) => vec![
                (along(*at), Slot::Float(at)),
                (meters("LENGTH", *length, 2.0, 4.0, 200.0), Slot::Float(length)),
            ],
            None => Vec::new(),
        },
        Item::Breach(index) => match description.breaches.get_mut(index) {
            Some(Breach { at, length, side }) => vec![
                (along(*at), Slot::Float(at)),
                (meters("LENGTH", *length, 5.0, 4.0, 2000.0), Slot::Float(length)),
                (side_field(*side), Slot::Side(side)),
            ],
            None => Vec::new(),
        },
        Item::Pad(index) => match description.pads.get_mut(index) {
            Some(Pad {
                at,
                lateral,
                length,
                width,
            }) => vec![
                (along(*at), Slot::Float(at)),
                (
                    meters("ACROSS", *lateral, 1.0, -half_width, half_width),
                    Slot::Float(lateral),
                ),
                (meters("LENGTH", *length, 2.0, 2.0, 60.0), Slot::Float(length)),
                (meters("WIDTH", *width, 1.0, 1.0, 40.0), Slot::Float(width)),
            ],
            None => Vec::new(),
        },
        Item::Checkpoint(index) => match description.checkpoints.get_mut(index) {
            Some(at) => vec![(along(*at), Slot::Float(at))],
            None => Vec::new(),
        },
        Item::Prop(index) => match description.scenery.get_mut(index) {
            Some(prop) => prop_slots(prop),
            None => Vec::new(),
        },
    }
}

fn prop_slots(prop: &mut Prop) -> Vec<(Field, Slot<'_>)> {
    let meters = |label, value: f32, step, min, max| Field {
        label,
        value,
        step,
        min,
        max,
        unit: Unit::Meters,
    };
    let count = |label, value: u8, max: f32| Field {
        label,
        value: f32::from(value),
        step: 1.0,
        min: 1.0,
        max,
        unit: Unit::Count,
    };
    let along = |value| meters("ALONG", value, 10.0, 0.0, 20_000.0);
    let side_field = |side: Side| Field {
        label: "SIDE",
        value: side.sign(),
        step: 2.0,
        min: -1.0,
        max: 1.0,
        unit: Unit::Side,
    };
    let offset = |value| meters("OFF THE WALL", value, 1.0, 0.0, 500.0);
    let size = |label, value| meters(label, value, 2.0, 1.0, 400.0);
    match prop {
        Prop::Gantry {
            at,
            clearance,
            depth,
        } => vec![
            (along(*at), Slot::Float(at)),
            (meters("CLEARANCE", *clearance, 1.0, 4.0, 400.0), Slot::Float(clearance)),
            (size("DEPTH", *depth), Slot::Float(depth)),
        ],
        Prop::Pylon {
            at,
            side,
            offset: off,
            height,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (size("HEIGHT", *height), Slot::Float(height)),
        ],
        Prop::Grandstand {
            at,
            side,
            offset: off,
            length,
            rows,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (size("LENGTH", *length), Slot::Float(length)),
            (count("ROWS", *rows, 20.0), Slot::Byte(rows)),
        ],
        Prop::Billboard {
            at,
            side,
            offset: off,
            width,
            height,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (size("WIDTH", *width), Slot::Float(width)),
            (size("HEIGHT", *height), Slot::Float(height)),
        ],
        Prop::Chevrons {
            at,
            side,
            offset: off,
            count: chevrons,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (count("ARROWS", *chevrons, 8.0), Slot::Byte(chevrons)),
        ],
        Prop::Monolith {
            at,
            side,
            offset: off,
            height,
            width,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (size("HEIGHT", *height), Slot::Float(height)),
            (size("WIDTH", *width), Slot::Float(width)),
        ],
        Prop::Tower {
            at,
            side,
            offset: off,
            height,
            width,
            depth,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (size("HEIGHT", *height), Slot::Float(height)),
            (size("WIDTH", *width), Slot::Float(width)),
            (size("DEPTH", *depth), Slot::Float(depth)),
        ],
        Prop::Skyline {
            at,
            side,
            offset: off,
            length,
            depth,
            height,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (size("LENGTH", *length), Slot::Float(length)),
            (size("DEPTH", *depth), Slot::Float(depth)),
            (size("HEIGHT", *height), Slot::Float(height)),
        ],
        Prop::Facade {
            at,
            side,
            offset: off,
            length,
            height,
            depth,
        } => vec![
            (along(*at), Slot::Float(at)),
            (side_field(*side), Slot::Side(side)),
            (offset(*off), Slot::Float(off)),
            (size("LENGTH", *length), Slot::Float(length)),
            (size("HEIGHT", *height), Slot::Float(height)),
            (size("DEPTH", *depth), Slot::Float(depth)),
        ],
        Prop::Tunnel {
            at,
            length,
            clearance,
        } => vec![
            (along(*at), Slot::Float(at)),
            (size("LENGTH", *length), Slot::Float(length)),
            (meters("CLEARANCE", *clearance, 1.0, 7.0, 100.0), Slot::Float(clearance)),
        ],
    }
}

/// Every number of `item`, in the order the editor shows them.
pub fn fields(description: &TrackDescription, item: Item) -> Vec<Field> {
    let mut copy = description.clone();
    slots(&mut copy, item)
        .into_iter()
        .map(|(field, _)| field)
        .collect()
}

/// Sets number `field` of `item` to `value`, kept within its bounds, and answers where the item
/// is now: a height or a checkpoint moved along the lap may have changed places in its list.
pub fn set(description: &mut TrackDescription, item: Item, field: usize, value: f32) -> Item {
    {
        let mut slots = slots(description, item);
        let Some((bounds, slot)) = slots.get_mut(field) else {
            return item;
        };
        let value = value.clamp(bounds.min, bounds.max);
        match slot {
            Slot::Float(stored) => {
                // A turn never turns by nothing: stepping through zero goes to the other side.
                **stored = if bounds.unit == Unit::Degrees && bounds.label == "ANGLE" && value == 0.0
                {
                    if **stored > 0.0 { -bounds.step } else { bounds.step }
                } else {
                    value
                };
            }
            Slot::Byte(stored) => **stored = value.round() as u8,
            Slot::Side(stored) => **stored = if value > 0.0 { Side::Left } else { Side::Right },
        }
    }
    keep_in_order(description, item)
}

/// Sorts the heights and the checkpoints along the lap, following `item` to its new place.
fn keep_in_order(description: &mut TrackDescription, item: Item) -> Item {
    match item {
        Item::Height(index) => {
            let moved = description.elevation[index];
            description
                .elevation
                .sort_by(|a, b| a.at.total_cmp(&b.at));
            let at = description
                .elevation
                .iter()
                .position(|height| *height == moved)
                .unwrap_or(index);
            Item::Height(at)
        }
        Item::Checkpoint(index) => {
            let moved = description.checkpoints[index];
            description.checkpoints.sort_by(f32::total_cmp);
            let at = description
                .checkpoints
                .iter()
                .position(|checkpoint| *checkpoint == moved)
                .unwrap_or(index);
            Item::Checkpoint(at)
        }
        other => other,
    }
}

/// Adds a part of kind `new` `at` meters along the lap (for a segment: after segment `after`, or
/// at the end of the road), and answers it.
pub fn add(description: &mut TrackDescription, new: New, at: f32, after: Option<usize>) -> Item {
    let at = at.max(0.0);
    let turn = |angle: f32| Segment::Turn {
        angle,
        radius: (description.width * 2.5).max(40.0),
        transition: 20.0,
        banking: 10.0,
    };
    let segment = match new {
        New::Straight => Some(Segment::Straight { length: 100.0 }),
        New::Left => Some(turn(90.0)),
        New::Right => Some(turn(-90.0)),
        _ => None,
    };
    if let Some(segment) = segment {
        let index = after.map_or(description.segments.len(), |after| {
            (after + 1).min(description.segments.len())
        });
        description.segments.insert(index, segment);
        return Item::Segment(index);
    }
    let item = match new {
        New::Height => {
            description.elevation.push(Elevation { at, height: 10.0 });
            Item::Height(description.elevation.len() - 1)
        }
        New::Gap => {
            description.gaps.push(Gap { at, length: 20.0 });
            Item::Gap(description.gaps.len() - 1)
        }
        New::Breach => {
            description.breaches.push(Breach {
                at,
                length: 40.0,
                side: Side::Left,
            });
            Item::Breach(description.breaches.len() - 1)
        }
        New::Narrows => {
            description.narrows.push(Narrows {
                at,
                length: 40.0,
                width: (description.width * 0.6).max(10.0),
                blend: 24.0,
            });
            Item::Narrows(description.narrows.len() - 1)
        }
        New::Pad => {
            description.pads.push(Pad {
                at,
                lateral: 0.0,
                length: 10.0,
                width: 6.0,
            });
            Item::Pad(description.pads.len() - 1)
        }
        New::Checkpoint => {
            description.checkpoints.push(at);
            Item::Checkpoint(description.checkpoints.len() - 1)
        }
        New::Prop(kind) => {
            description.scenery.push(kind.new(at));
            Item::Prop(description.scenery.len() - 1)
        }
        New::Straight | New::Left | New::Right => unreachable!("handled above"),
    };
    keep_in_order(description, item)
}

/// Removes `item`. The road as a whole, and its last segment, stay.
pub fn remove(description: &mut TrackDescription, item: Item) {
    match item {
        Item::Road => {}
        Item::Segment(index) => {
            if description.segments.len() > 1 && index < description.segments.len() {
                description.segments.remove(index);
            }
        }
        Item::Height(index) => remove_at(&mut description.elevation, index),
        Item::Narrows(index) => remove_at(&mut description.narrows, index),
        Item::Gap(index) => remove_at(&mut description.gaps, index),
        Item::Breach(index) => remove_at(&mut description.breaches, index),
        Item::Pad(index) => remove_at(&mut description.pads, index),
        Item::Checkpoint(index) => remove_at(&mut description.checkpoints, index),
        Item::Prop(index) => remove_at(&mut description.scenery, index),
    }
}

fn remove_at<T>(list: &mut Vec<T>, index: usize) {
    if index < list.len() {
        list.remove(index);
    }
}

/// Every part of `tab`, in order.
pub fn items(description: &TrackDescription, tab: Tab) -> Vec<Item> {
    match tab {
        Tab::Road => std::iter::once(Item::Road)
            .chain((0..description.segments.len()).map(Item::Segment))
            .collect(),
        Tab::Relief => (0..description.elevation.len()).map(Item::Height).collect(),
        Tab::Game => (0..description.gaps.len())
            .map(Item::Gap)
            .chain((0..description.breaches.len()).map(Item::Breach))
            .chain((0..description.narrows.len()).map(Item::Narrows))
            .chain((0..description.pads.len()).map(Item::Pad))
            .chain((0..description.checkpoints.len()).map(Item::Checkpoint))
            .collect(),
        Tab::Scenery => (0..description.scenery.len()).map(Item::Prop).collect(),
    }
}

/// Meters along the lap where `item` is, if it is placed along it.
pub fn at(description: &TrackDescription, item: Item) -> Option<f32> {
    match item {
        Item::Road | Item::Segment(_) => None,
        Item::Height(index) => description.elevation.get(index).map(|height| height.at),
        Item::Narrows(index) => description.narrows.get(index).map(|narrows| narrows.at),
        Item::Gap(index) => description.gaps.get(index).map(|gap| gap.at),
        Item::Breach(index) => description.breaches.get(index).map(|breach| breach.at),
        Item::Pad(index) => description.pads.get(index).map(|pad| pad.at),
        Item::Checkpoint(index) => description.checkpoints.get(index).copied(),
        Item::Prop(index) => description.scenery.get(index).map(Prop::at),
    }
}

/// One line saying what `item` is, for its row in the list.
pub fn describe(description: &TrackDescription, item: Item) -> String {
    let along = |at: f32| format!("at {at:.0} m");
    match item {
        Item::Road => format!("ROAD  ·  {:.0} m wide", description.width),
        Item::Segment(index) => match description.segments.get(index) {
            Some(Segment::Straight { length }) => format!("{}. STRAIGHT  ·  {length:.0} m", index + 1),
            Some(Segment::Turn {
                angle,
                radius,
                banking,
                ..
            }) => format!(
                "{}. {} {:.0}°  ·  R {radius:.0} m{}",
                index + 1,
                if *angle > 0.0 { "LEFT" } else { "RIGHT" },
                angle.abs(),
                if *banking > 0.0 {
                    format!("  ·  BANKED {banking:.0}°")
                } else {
                    String::new()
                }
            ),
            None => String::new(),
        },
        Item::Height(index) => description.elevation.get(index).map_or(String::new(), |height| {
            format!("HEIGHT {:.0} m  ·  {}", height.height, along(height.at))
        }),
        Item::Narrows(index) => description.narrows.get(index).map_or(String::new(), |narrows| {
            format!("NARROWS {:.0} m wide  ·  {}", narrows.width, along(narrows.at))
        }),
        Item::Gap(index) => description.gaps.get(index).map_or(String::new(), |gap| {
            format!("GAP {:.0} m  ·  {}", gap.length, along(gap.at))
        }),
        Item::Breach(index) => description.breaches.get(index).map_or(String::new(), |breach| {
            let side = if breach.side == Side::Left { "LEFT" } else { "RIGHT" };
            format!("BREACH {side} {:.0} m  ·  {}", breach.length, along(breach.at))
        }),
        Item::Pad(index) => description
            .pads
            .get(index)
            .map_or(String::new(), |pad| format!("PAD  ·  {}", along(pad.at))),
        Item::Checkpoint(index) => description
            .checkpoints
            .get(index)
            .map_or(String::new(), |at| format!("CHECKPOINT  ·  {}", along(*at))),
        Item::Prop(index) => description.scenery.get(index).map_or(String::new(), |prop| {
            let side = match prop.side() {
                Some(Side::Left) => " LEFT",
                Some(Side::Right) => " RIGHT",
                None => "",
            };
            format!("{}{side}  ·  {}", PropKind::of(prop).name(), along(prop.at()))
        }),
    }
}

/// A first circuit for a new draft: a straight, a banked half turn, a straight, the editor
/// closing it with the turn back to the line.
pub fn starter() -> TrackDescription {
    TrackDescription {
        name: "My Circuit".into(),
        width: 32.0,
        segments: vec![
            Segment::Straight { length: 250.0 },
            Segment::Turn {
                angle: 180.0,
                radius: 90.0,
                transition: 30.0,
                banking: 12.0,
            },
            Segment::Straight { length: 120.0 },
        ],
        elevation: Vec::new(),
        narrows: Vec::new(),
        scenery: Vec::new(),
        pads: Vec::new(),
        gaps: Vec::new(),
        checkpoints: Vec::new(),
        breaches: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind of prop can be added, with sizes the track file accepts.
    #[test]
    fn every_prop_can_be_added() {
        let mut description = starter();
        for kind in PropKind::ALL {
            let item = add(&mut description, New::Prop(kind), 50.0, None);
            assert_eq!(PropKind::of(&description.scenery[match item {
                Item::Prop(index) => index,
                _ => unreachable!(),
            }]), kind);
            assert!(!fields(&description, item).is_empty());
        }
        for prop in &description.scenery {
            prop.validate(1000.0).unwrap();
        }
    }

    /// A field steps within its bounds, a turn steps through zero to the other side, and heights
    /// keep in order, the one moved followed.
    #[test]
    fn fields_step_within_bounds_and_lists_stay_in_order() {
        let mut description = starter();
        let item = Item::Segment(1);
        set(&mut description, item, 0, 0.0);
        assert!(matches!(description.segments[1], Segment::Turn { angle, .. } if angle == -5.0));
        set(&mut description, item, 3, 400.0);
        assert!(matches!(description.segments[1], Segment::Turn { banking, .. } if banking == 60.0));

        let first = add(&mut description, New::Height, 100.0, None);
        let second = add(&mut description, New::Height, 300.0, None);
        assert_eq!((first, second), (Item::Height(0), Item::Height(1)));
        let moved = set(&mut description, Item::Height(1), 0, 50.0);
        assert_eq!(moved, Item::Height(0));
        assert_eq!(description.elevation[0].at, 50.0);
        assert!(describe(&description, moved).contains("50 m"));

        remove(&mut description, Item::Height(0));
        assert_eq!(description.elevation.len(), 1);
        // The last segment stays.
        while description.segments.len() > 1 {
            remove(&mut description, Item::Segment(0));
        }
        remove(&mut description, Item::Segment(0));
        assert_eq!(description.segments.len(), 1);
    }
}
