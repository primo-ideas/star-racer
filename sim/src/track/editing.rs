//! What the level editor needs of a circuit that is not finished yet (since v1.0.0-rc2): the line
//! its segments draw while they do not close a loop, and the segments that close it.
//!
//! A circuit's segments must turn through one whole turn and end where they started, heading the
//! way they started (see [`Track::build`](super::Track::build)), which nobody laying turns by hand
//! hits. The editor lets the player lay the road as far as they want, and closes it for them with
//! the shortest way back to the start line made of a turn, a straight and a turn, all of
//! [`closing`]'s radius: the shortest path of a car that can only turn that tightly, as Dubins
//! found it.

use std::f64::consts::TAU;

use glam::{DVec2, Vec2};

use super::{CurvatureProfile, Segment, TrackDescription, TrackError};

/// The line a circuit's segments draw, whether or not it closes.
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    /// Points of the centerline every `spacing` meters or so, from the start line to the end of
    /// the last segment, both included.
    pub points: Vec<Vec2>,
    /// Where the last segment ends, and the heading there, in radians from `+x`.
    pub end: Vec2,
    pub heading: f32,
    /// The angle the segments turn through in all, in degrees, positive to the left.
    pub turned: f32,
    /// The length of the centerline, in meters.
    pub length: f32,
}

/// Steps of integration between two points of a trace.
const SUBSTEPS: usize = 8;

/// Traces the segments of `description` from the start line, a point every `spacing` meters.
pub fn trace(description: &TrackDescription, spacing: f32) -> Result<Trace, TrackError> {
    let (profile, turned) = CurvatureProfile::open(description)?;
    let length = profile.length();
    let count = (length / f64::from(spacing.max(0.1))).ceil().max(1.0) as usize;
    let step = length / count as f64;
    let mut position = DVec2::ZERO;
    let mut heading = 0.0;
    let mut points = vec![Vec2::ZERO];
    for index in 0..count {
        for substep in 0..SUBSTEPS {
            let start = index as f64 * step + substep as f64 * step / SUBSTEPS as f64;
            profile.advance(
                &mut position,
                &mut heading,
                start,
                start + step / SUBSTEPS as f64,
            );
        }
        points.push(position.as_vec2());
    }
    Ok(Trace {
        points,
        end: position.as_vec2(),
        heading: heading as f32,
        turned,
        length: length as f32,
    })
}

/// The segments that take the end of `description`'s road back to the start line, heading the way
/// it starts: a turn, a straight and a turn, of `radius` meters (at least a little more than half
/// the road's width), the shortest of the four ways that turns through one whole turn with the rest
/// of the circuit. Either turn or the straight is left out when it would be nothing. `Err` says why
/// there is none: the road cannot be traced, or every way back loops round once more.
pub fn closing(description: &TrackDescription, radius: f32) -> Result<Vec<Segment>, &'static str> {
    let trace = trace(description, 4.0).map_err(|_| "the road as laid cannot be traced")?;
    // Already closed, as a published circuit loaded back into the editor is: nothing to add.
    if trace.end.length() < 0.5 && (trace.turned.abs() - 360.0).abs() < 0.01 {
        return Ok(Vec::new());
    }
    let radius = f64::from(radius.max(description.width / 2.0 + 1.0));
    let from = trace.end.as_dvec2();
    let heading = f64::from(trace.heading);
    let turned = f64::from(trace.turned).to_radians();
    let left = |at: DVec2, heading: f64| at + radius * DVec2::new(-heading.sin(), heading.cos());
    let right = |at: DVec2, heading: f64| at + radius * DVec2::new(heading.sin(), -heading.cos());
    let wrap = |angle: f64| angle.rem_euclid(TAU);

    // Each way back: the first turn, signed (positive to the left), the straight, the second turn.
    let mut ways = Vec::new();
    // Left, straight, left: along the outer tangent of the two left circles.
    let d = left(DVec2::ZERO, 0.0) - left(from, heading);
    let phi = d.y.atan2(d.x);
    ways.push((wrap(phi - heading), d.length(), wrap(-phi)));
    // Right, straight, right.
    let d = right(DVec2::ZERO, 0.0) - right(from, heading);
    let phi = d.y.atan2(d.x);
    ways.push((-wrap(heading - phi), d.length(), -wrap(phi)));
    // Left, straight, right: across the inner tangent.
    let d = right(DVec2::ZERO, 0.0) - left(from, heading);
    if d.length() >= 2.0 * radius {
        let straight = (d.length_squared() - 4.0 * radius * radius).sqrt();
        let phi = d.y.atan2(d.x) + (2.0 * radius).atan2(straight);
        ways.push((wrap(phi - heading), straight, -wrap(phi)));
    }
    // Right, straight, left.
    let d = left(DVec2::ZERO, 0.0) - right(from, heading);
    if d.length() >= 2.0 * radius {
        let straight = (d.length_squared() - 4.0 * radius * radius).sqrt();
        let phi = d.y.atan2(d.x) - (2.0 * radius).atan2(straight);
        ways.push((-wrap(heading - phi), straight, wrap(-phi)));
    }

    let (first, straight, second) = ways
        .into_iter()
        // The whole circuit must turn through exactly one turn, either way round.
        .filter(|(first, _, second)| ((turned + first + second).abs() - TAU).abs() < 1e-3)
        .min_by(|a, b| {
            let length = |(first, straight, second): &(f64, f64, f64)| {
                radius * (first.abs() + second.abs()) + straight
            };
            length(a).total_cmp(&length(b))
        })
        .ok_or("every way back to the line loops round once more: turn the road the other way")?;

    let turn = |angle: f64| Segment::Turn {
        angle: angle.to_degrees() as f32,
        radius: radius as f32,
        transition: 0.0,
        banking: 0.0,
    };
    let mut segments = Vec::new();
    if first.abs() > 1e-4 {
        segments.push(turn(first));
    }
    if straight > 0.05 {
        segments.push(Segment::Straight {
            length: straight as f32,
        });
    }
    if second.abs() > 1e-4 {
        segments.push(turn(second));
    }
    // Rounding may leave the angles a hair off a whole turn: the last turn takes it up.
    let total: f64 = f64::from(trace.turned)
        + segments
            .iter()
            .map(|segment| match segment {
                Segment::Turn { angle, .. } => f64::from(*angle),
                Segment::Straight { .. } => 0.0,
            })
            .sum::<f64>();
    let off = (total.abs() - 360.0) * total.signum();
    if let Some(Segment::Turn { angle, .. }) = segments
        .iter_mut()
        .rev()
        .find(|segment| matches!(segment, Segment::Turn { .. }))
    {
        *angle -= off as f32;
    }
    Ok(segments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::track::Track;

    fn open(segments: Vec<Segment>) -> TrackDescription {
        TrackDescription {
            name: "Draft".into(),
            width: 24.0,
            segments,
            elevation: Vec::new(),
            narrows: Vec::new(),
            scenery: Vec::new(),
            pads: Vec::new(),
            gaps: Vec::new(),
            checkpoints: Vec::new(),
            breaches: Vec::new(),
        }
    }

    fn turn(angle: f32, radius: f32) -> Segment {
        Segment::Turn {
            angle,
            radius,
            transition: 20.0,
            banking: 10.0,
        }
    }

    /// A straight traces a straight line, a half turn ends heading back.
    #[test]
    fn a_trace_follows_the_segments() {
        let trace = trace(
            &open(vec![Segment::Straight { length: 100.0 }, turn(180.0, 50.0)]),
            2.0,
        )
        .unwrap();
        assert!((trace.turned - 180.0).abs() < 1e-4);
        assert!((trace.heading - std::f32::consts::PI).abs() < 1e-3);
        assert!(trace.points.len() > 50);
        assert_eq!(trace.points[0], Vec2::ZERO);
        assert!((trace.points.last().unwrap() - trace.end).length() < 1e-3);
    }

    /// Whatever the road laid so far, closed it builds into a circuit, both ways round.
    #[test]
    fn a_closed_road_builds() {
        let drafts = [
            vec![Segment::Straight { length: 200.0 }],
            vec![Segment::Straight { length: 150.0 }, turn(90.0, 60.0)],
            vec![Segment::Straight { length: 150.0 }, turn(-120.0, 60.0), turn(70.0, 80.0)],
            vec![
                Segment::Straight { length: 300.0 },
                turn(180.0, 50.0),
                Segment::Straight { length: 120.0 },
                turn(-90.0, 40.0),
            ],
        ];
        for segments in drafts {
            let mut description = open(segments.clone());
            let closing = closing(&description, 60.0).unwrap();
            description.segments.extend(closing);
            if let Err(error) = Track::build(&description) {
                panic!("{segments:?} closed did not build: {error}");
            }
        }
    }
}
