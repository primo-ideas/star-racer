# Tracks

A track is described by the centerline of its road, as a list of segments laid end to end: a
turtle drawing the road. The format is defined by `TrackDescription` in
`sim/src/track.rs`, and files are written in [RON](https://github.com/ron-rs/ron).

Track files live in `server/data/tracks/`, one per track, named by its key: the server loads them
all, and the player who creates a lobby picks one; quick play and ranked take them in turn. Since
v0.1.50 the game ships one circuit, [the Odyssey](#the-odyssey), which replaced the skyway and the
rift at the user's word ("tu vas supprimer les niveaux déjà fait et en faire un long et riche, en
pensant checkpoints ET shortcuts à découvrir"). Clients receive every track when they connect (see
[Protocol](protocol.md#tracks)). Every shipped track is tested: it must load, and the autopilot
must lap it drifting without touching a wall, faster than gripping (see
[Simulation](simulation.md#autopilot)).

## The Odyssey

`odyssey.ron`: 40 m from wall to wall (30 until v0.1.53), **5.28 km a lap** (5.06 until v0.1.51), its relief from 6 to 60 m, its road
missing in four places, eleven checkpoints, eight speed pads, and three shortcuts. The lap is one
big turn to the left, a rounded rectangle about 1.2 km by 0.9, with three loops hanging off its
outside: the road leaves its line to the right, winds round a tight banked loop and comes back to
the line it left, a little further on. Between them it runs fast: slaloms of 70 to 120 m radii,
a quarter turn of 140 m, straights with a gap to jump.

- **The launch**: out of the line, over the hop, 14 m missing 150 m after it.
- **The weave**: a long, fast slalom, dropping 12 m.
- **The keyhole**: the first loop, 440 m of it, the lowest part of the lap.
- **The long left**: a wide banked quarter turn, climbing.
- **The leap**: 110 m missing on the way up: the thruster's.
- **The climb**: a slalom up to the top of the lap, 60 m up.
- **The balcony**: the second loop, out over the void, winding down 26 m.
- **The sky bowl**, a banked left, and **the gate**, the top straight pinched to 18 m.
- **The dive**: down 30 m through a fast kink.
- **The rift**: 170 m missing at the bottom of the dive, a monolith standing over the void on
  either side.
- **The scree**, a slalom, and **the step**, 16 m missing on the way out of it.
- **The west corner**, climbing, and **the esses**.

Since v0.1.69 nothing stands over the Odyssey's road: its six gantries and three tunnels were taken out at the user's word "Enlève tout ce qu'il peut y avoir comme \"toit\" au dessus de la route", the car flying twice as high on its thruster. The two kinds of prop remain in the format.
- **The long way round**: the third loop, the widest.
- **The last corner**, onto the run home.

Since v0.1.54 the road is 40 m wide instead of 30, at the user's word ("Elargis les routes du
circuit"): the balcony's loop was redrawn so its two ends no longer meet on a wider road (turns of
110 degrees instead of 120 on necks of 30 m, round a turn of 52 m), and the keyhole's first turn
is banked 14 degrees instead of 22, a wider road raising its outer edge too high to fly over.
Since v0.1.52 its leap and its rift are 110 and 170 m long (45 and 55 before), the straights
under them stretched by 80 and 100 m, for a thruster that flies twice as far (the user's word:
"Agrandis les précipices, car on doit pouvoir thrust bien davantage"). With the shipped car the
autopilot laps it gripping in 83.0 s and drifting in 79.4 s with five charges of turbo fired,
both without touching a wall or falling; six ticks late, 83.4 s clean and 88.2 s with 99 ticks
against a wall; eight personalities from 75.3 to 84.0 s. It takes no
shortcut. A race is two laps by default, since a lap is two and a half times what the skyway's
was.

It was designed with a scratch tool outside the repository that integrates the clothoids, solves
two lengths so the loop closes, moves everything placed along the lap when the road before it
changes length, draws the plan colored by height, drives the autopilot round, and flies a car
straight on from any point of the road to see where it comes down.

### Shortcuts

**The walls are as high as they look** (1.4 m), and nothing stands on top of them (the user's
word: "Il ne faut pas de mur invisible mais on doit détecter quand le joueur part trop en sucette
pour le faire respawn après un petit compte à rebours"). A hop does not clear them; the thruster
does, and a car that flies over one has left the road. It keeps its own height out there, the
thruster still pushing while its tank lasts, and it is on the road again the moment it is over
road no higher than it is, whichever part of the lap that road belongs to. Meanwhile it is
counted out: 2.4 s off its course and it is brought back to its last checkpoint (see
[Simulation](simulation.md#off-the-road)).

That is all a shortcut needs, and the three loops are where they are. Each leaves the line the
road was on and comes back to it, so a car that goes **straight on where the road turns away**
crosses the void between the loop's two ends and lands on the road coming back, 300 to 340 m of
loop skipped, about five seconds:

| Loop | How | Measured |
| --- | --- | --- |
| The keyhole | Over the wall on the thruster, at top speed, from the straight before the loop (615 to 630 m, since the road is 40 m wide): later, the car flies into the banked road of the loop's first turn. The pad before the loop, on the left, is the hint. | 2.3 s in the air, 0.7 s off the road |
| The balcony | The wall is missing where the road turns away (a [breach](#breaches)), and the road coming back lies 26 m below: straight on, nothing pressed. | 1.5 s of falling |
| The long way round | The longest flight, on the thruster from top speed (until v0.1.51, when the thruster was weaker, it took the turbo). | 2.1 s in the air, 1.25 s off the road |

Nothing announces them but a pad pointing the wrong way and a wall that is not there. A failed
one costs the fall, the count and the way back from the checkpoint before the loop.

**What keeps them honest is the checkpoints.** No gate stands inside a loop, so a car that cuts
one lands between the same two gates it left, and its progress takes the cut in full. Anywhere
else, a flight from one part of the road to another ends beyond a gate the car has not passed:
there it is off its course, on the road or not, counted out and brought back (see
[Checkpoints](#checkpoints)). `the_shortcuts_of_the_odyssey_are_there` (`server/src/content.rs`)
flies all three with the shipped car, and fails if a change to the track or to the tuning closes
one or opens the last to a car at top speed.

## The circuits before it

`skyway.ron` was the one circuit the game shipped until v0.1.48, and one of two until v0.1.49,
with `rift.ron` (see [The rift](#the-rift)): 34 m from wall to wall, 1.86 km a lap, and nothing
on it is straight or level for long. It is drawn after the circuits of Rocket Racing, whose roads
turn, climb, dive and bank all at once: the relief runs from 3 m up to 47 m and back (see
[Relief](#relief)), every one of its twenty turns is banked, the steepest stretch dives at 14
degrees, and the only straight is the 68 m around the start line, which is there because the
starting grid needs somewhere to stand. It has no bottleneck, and runs through a city at night (see
[The skyway's city](#the-skyways-city)).

The lap folds back on itself like a comb: four legs, joined by three hairpins and closed by two
banked corners, so the road always has another part of itself beside it, above or below. Two legs
come within 81 m of each other, centerline to centerline, 22 m apart in height; nowhere closer. The
legs are slaloms, three turns swapping sides with no road between them, and the hairpins are
bowls: one at the bottom of the lap, 3 m up, one that climbs 12 m as it turns, and one at the top,
banked 34 degrees, 47 m up, from which the fourth leg dives 35 m in 235. That is where the drift
lives: the drifting autopilot laps it in 45.9 s without touching a wall, against 49.9 s gripping.

Width is the point. It was first a matter of room: while the drift's slide was real, from
2026-09-20 to 2026-09-27, it carried the car several meters off its line before the path came
round, and on a 16 m road that was already a wall. The drift no longer runs wide (see
[Simulation](simulation.md#drift)), and the width is now what leaves a line to choose through a
turn, room to drift with the body swung far across the road, and room for a pack. Four other
circuits lived here and are gone: `hippodrome.ron`, a 16 m oval that was the
first test track, `four-corners.ron`, a 16 m rounded rectangle, `serpentine.ron`, 18 m wide with
turns of 18 to 28 m chained without a straight between them, and `esplanade.ron`, 28 m wide, flat
but for six banked turns and pinched by three bottlenecks. The first three were drawn before the
car could really slide, and none of them left room for it. The esplanade was the circuit the drift
was drawn for, and it gave way to the skyway when the game asked for wide roads curved in more
than one direction.

## Format

```ron
(
    name: "Example",
    // Road width from wall to wall, in meters, where nothing narrows it.
    width: 16.0,
    segments: [
        Straight(length: 50.0),
        // Angles in degrees, positive to the left. Transition lengths in meters.
        Turn(angle: 180.0, radius: 40.0, transition: 20.0, banking: 30.0),
        Straight(length: 100.0),
        Turn(angle: 180.0, radius: 40.0, transition: 20.0, banking: 30.0),
        Straight(length: 50.0),
    ],
    // Optional: heights the road passes through, in meters (see Relief).
    elevation: [
        (at: 0.0, height: 10.0),
        (at: 150.0, height: 22.0),
    ],
    // Optional: where the road pinches in (see Bottlenecks).
    narrows: [
        (at: 120.0, length: 30.0, width: 10.0, blend: 20.0),
    ],
    // Optional: where the road is missing (see Gaps), and the gates of a lap (see Checkpoints).
    gaps: [
        (at: 90.0, length: 14.0),
    ],
    checkpoints: [40.0, 180.0],
    // Optional: where a wall is missing (see Breaches).
    breaches: [
        (at: 60.0, length: 30.0, side: Left),
    ],
)
```

- The track starts **on the start line**, at the origin, heading `+x`, and the segments must bring
  it back there. Putting the start line in the middle of a straight, as above, leaves room for the
  starting grid behind it.
- `Straight(length)`.
- `Turn(angle, radius, transition, banking)`: `angle` in degrees, positive to the left, along an
  arc of `radius`. `transition` and `banking` are optional.

Unknown fields are errors, so a typo such as `transtion` does not silently fall back to the
default.

## Transitions

A plain turn switches from straight to its full curvature at once: the driver has to snap the
stick to a new position at the exact entry point. Real roads avoid that with **clothoids**, curves
whose curvature grows linearly with distance, and so does this format.

With a `transition` of `L` meters, the curvature ramps from zero to `1 / radius` over `L` meters,
stays constant along the arc, and ramps back down over another `L` meters. The whole turn still
turns by `angle`. Compared with the same turn without transitions:

- the arc is `L` meters shorter, but the two clothoids add `2 L`, so the turn is `L` meters longer;
- the turn bulges slightly outward (by about `L² / (12 radius)` across the two ends), which is why
  both turns of an oval must use the same transition to close.

## Banking

`banking` tilts the road of a turn toward its inside, like a velodrome. It is the slope, in degrees,
the road reaches at the **outer** edge.

The cross-section is not a flat tilted plane but a curve: a parabola, flat at the inner edge and
steeper and steeper toward the outer edge. With a half width `w` and a banking angle `β`, the outer
edge stands `w × tan β` above the inner one, and the centerline a quarter of that. In the sky bowl,
the top hairpin of the skyway (34 m wide, 34°), the outer edge rises 11.5 m. A high line is steeper
than a low one, which gives the driver a real choice of line through the turn.

The banking follows the transitions: it rises from flat to full along the entry transition, stays
full along the arc, and settles back along the exit one. Within a transition it follows a
smoothstep, the cubic that a Catmull-Rom or Hermite spline gives between a flat key and a banked
one: the road starts and finishes rising gently, with no crease where the transition meets the
straight or the arc. That is why a banked turn needs a transition: without one, the road would
have a step.

The centerline itself stays exactly what the clothoids describe, seen from above. The banking only
adds a height to the road on each side of it, on top of the relief: the lower edge of a banked
turn lies at the height of the relief, and the rest of the road rises from there (see
[Relief](#relief)).

## Relief

`elevation` lists heights the road passes through, each placed by its distance along the lap:

```ron
elevation: [
    (at: 0.0, height: 21.0),
    (at: 55.0, height: 14.0),
    (at: 100.0, height: 16.0),
],
```

- `at`: meters along the lap from the start line, like a bottleneck or a prop rather than a
  segment, so a height keeps its place when the road around it is redrawn. Heights are listed in
  order along the lap.
- `height`: meters above the ground, which is height zero.

Between two heights the road follows a cubic whose slope at each height is the slope of the
parabola through that height and its two neighbors: each side's grade, weighted by the length of
the other side. The road passes through every height the file gives, its grade never jumps, and
the curve runs on from the last height round to the first, so the relief of a lap closes on
itself. A height above both its neighbors is a crest and one below them a dip, give or take the
little the curve overshoots when the two sides are uneven. A single height lifts the whole road to
it, and none leaves it on the ground.

The relief and the banking add up, and neither cares about the other: a turn can climb, dive, or
crest while it banks, which is what makes a hairpin a spiral. A step along the centerline is shorter
on the inside of a turn than on the outside, by the ratio of the two radii, so the inside of a turn
that climbs is steeper than its centerline: on the skyway, the dive is 14 degrees at its steepest
on the centerline and 16 at its inner edge.

The centerline may be no steeper than 25 degrees. Cars move on the plane under the road (see
[Simulation](simulation.md#slopes)), which only stays honest while the road is not too steep: at
25 degrees a car covers 10% more road than the plane says it does.

The track stays a height field, one height over each point of the plane: the road can climb over a
hill but never over itself. A bridge, a loop or a wall ride would need the simulation to move cars
along the road rather than over the plane under it, which it does not do.

## Bottlenecks

A `narrows` pinches the road in for a stretch, and is what makes a wide circuit a test of
precision: the drift needs room to run wide (see [Simulation](simulation.md#drift)), and a
bottleneck takes that room away exactly where it was being enjoyed. The skyway has none, its roads
being wide on purpose; the esplanade before it had three.

```ron
narrows: [
    (at: 712.0, length: 30.0, width: 13.0, blend: 28.0),
],
```

- `at`: meters along the lap where the narrow stretch begins, like a prop of the scenery, rather
  than which segment it falls on. It therefore keeps its place when the road is redrawn around it.
- `length`: how far the road stays narrow.
- `width`: from wall to wall along it, at least 8 m and never wider than the track.
- `blend`: how far the road takes to pinch in before the stretch and to open out after it, 24 m by
  default. It follows the same smoothstep as the banking, so the walls start and finish moving
  gently and the road has no crease. A blend of zero gives a step in the wall, which is allowed and
  looks it.

The centerline never moves: both walls come in by the same amount, and the line through a
bottleneck is the line the road already took. A bottleneck is not a corner, it is a corner with the
room taken away. Everything that reads the width reads it where it is — the walls a car is pushed
back from, the road and the walls the client builds, the props beside them, the starting grid, the
camera, the thumbnail the menus draw — so a prop standing against the wall comes in with it, and an
arch over a bottleneck is narrower than one over the open road.

The banking keeps its **slope** through a bottleneck rather than its height: on a narrower road the
same `banking` raises the outer edge less, since the edge is closer to the middle.

Narrows may overlap: the road is as wide as the narrowest of them says. Where nothing narrows it,
`width` is the width, which is why `Track::half_width` is the widest the road ever is and
`Track::narrowest_half_width` what a car has to fit through.

### Placing one

Where a bottleneck starts matters more than how narrow it is. A drift runs wide out of a turn, so
walls that close during a turn's exit meet the car while it is still out there. The esplanade's
gate was first drawn at 694 m, its blend starting inside the exit of the esses: the drifting
autopilot left them 7 m off the centerline, met the walls coming in, and scraped along them for
nine ticks, losing 12 m/s. Moved to 712 m, narrow over the end of the straight and into the fast
right after it, the same 13 m left the same drift 2.4 m to spare. The width had nothing to do
with it.

`shipped_tracks_are_drivable` is how a placement is judged: the drifting autopilot must get through
without more than a graze. How close it comes is worth knowing too: the tightest of the esplanade's
three was the funnel, where the drift out of the right-hander before it passed 0.6 m from the wall.
At 14 m it had passed 0.1 m from it, which would have made every change to the car's tuning a coin
toss in CI, so it was opened to 15.

## Speed pads

Since v0.1.43 (protocol 28, the user's word "Rajoute des pads de vitesse"), a track may lay
**speed pads** on its road: `pads: [(at: 372.0, lateral: 5.0, length: 10.0, width: 6.0)]`, each
placed by its distance along the lap and its offset from the centerline (positive to the left),
10 m long and 6 m wide unless the file says otherwise. A car on one is boosted as a drift's release
boosts it, toward `boost_top_speed`, for as long as it is on it and `pad_boost` (0.8 s) after; the
simulation checks every tick, so it is predicted like everything else. The client draws three amber
chevrons on each, brighter than white so they bloom, over a pool of the same light on the road
(`track/markings.rs`). A pad must lie on the lap and between the walls, or the track is refused
(`TrackError::InvalidPad`).

The skyway has five, each off to one side so taking one is a choice of line: the exits of the bowl
(372 m, 5 m left), of the spiral (745 m, 5 m right) and of the sink (1508 m, 6 m left), and twice
down the dive (1205 m, 4 m left; 1290 m, 6 m right). One in the middle at 1508 m put the neutral
autopilot six ticks late on a wall once a lap, which the shipped-track test refuses. The rift has
four, all in the middle: after the hop, up the climb, into the rift and out of the scree.

## Gaps

Since v0.1.49, a track may leave its road out in places, to be jumped (see
[Simulation](simulation.md#the-jump)): `gaps: [(at: 80.0, length: 14.0)]`, each placed by where it
begins along the lap and how long it is, from 4 to 200 m (90 until v0.1.51). The road, its underside and everything
painted on it stop at a gap's edges, and the piers stand back from them; the walls carry on over
it and hold a car that is no higher than they are. Since v0.1.67 (the user's word: the gaps looked
"un peu grossier") they are drawn there as what they are, the girders of a broken bridge
(`track/girders.rs`): an open Warren truss of the wall's height and outline, its top chord striped
like the wall, a solid post capping the wall at each end, so the void shows through; before, solid
wall panels hung over the void with nothing under them. Each edge is the cut face of the slab,
lined with hot magenta neon across the road at its top and along its foot, its light thrown back
along the road, and an abutment as wide as the road stands under it (`track/piers.rs`). Before the
lip, three magenta chevrons across the whole road point over it, dimmer the further back
(`track/markings.rs`), so a gap reads from far off. A car's shadow and the light under it are
not drawn over a gap, there being no road to throw them on.

A car that reaches a gap on the road falls through it and comes back at its last checkpoint (see
[Simulation](simulation.md#off-the-road)). With the shipped car a hop clears about 22 m
at top speed, less from a slower start, and the thruster about 220 m: a gap of 14 m is a tap,
and anything longer the thruster's, lit at the top of the hop. The relief carries
on under a gap, so the far side can stand lower than the lip, and a jump off a crest flies further.

A gap must keep 60 m clear behind the start line, where the grid stands, and 40 m after it, and no
pad may lie over one (`TrackError::InvalidGap`, `InvalidPad`). The autopilot presses the jump 0.2 s
before every lip (`autopilot::jumps`) and holds the jump and the thrust across every gap, and the
shipped-track test refuses a lap in which it falls.

## Checkpoints

Since v0.1.49, the user's word "Ajoute des checkpoints pour eviter les exploits":
`checkpoints: [600.0, 880.0]`, the meters along the lap of the gates a lap passes in order, the
start line being the first gate without being listed. A lap counts only once a car has passed every
one of them in order, and a car that is lost comes back at the last one it passed; how
the progress is kept honest is in [Lobbies](lobbies.md#progress-laps-and-finishing). Each is drawn
as a line of green neon across the road, its light on the road either side, with nothing standing
in the way.

A gate is a line across the road: it is passed by a car over the road, on it or flying, and not
by one going by beside it. And a car has a **course**: the road from the last gate it passed to
the next one, and 120 m behind (`Track::on_course`). Anywhere else on the road it has missed a
gate, or gone back past one: it is counted out exactly as a car off the road is, the HUD saying
MISSED CHECKPOINT, and brought back to its last gate after 2.4 s. That is what stops a flight
from one side of the lap to another (see [Shortcuts](#shortcuts)).

They must be in order along the lap, 40 m from each other and from the line, and none may lie in a
gap or less than 40 m before one, so a car brought back to it has room to get going again
(`TrackError::InvalidCheckpoint`). The Odyssey has eleven: one before and one after each loop,
one before each gap, and the rest where the lap would otherwise go 600 m without one.

## Breaches

Since v0.1.50: `breaches: [(at: 1916.8, length: 45.0, side: Left)]`, a stretch where the wall on
one side is missing, placed by where it begins along the lap, its length (4 m at least) and its
side. Nothing holds a car in there: it drives off the edge of the road and falls, onto another
part of the road if one lies below and ahead, or into the dark. The client leaves the wall out,
and where its neon would have lit the road the open edge glows amber, the color of the pads
(`track/markings.rs`). The Odyssey has one, on the balcony.

## The rift

Gone since v0.1.50, with the skyway. `rift.ron` was the second circuit (v0.1.49, the user's word "Prépare toi à faire un nouveau niveau
après pour bien correspondre a notre nouvelle mania"), drawn for the heavy, fast car of v0.1.44 and
for the jump: 30 m wide, 2.02 km a lap, its relief from 5 to 42 m, its road missing in four places.
It runs fast where the car wants to run, a 200 m launch and a long slalom of 110 m radii, and folds
into tight bowls of 44 to 46 m, banked 32 to 34 degrees, where it wants to drift.

- **The launch**: out of the line, over the hop, 14 m missing 80 m after it.
- **The weave**: a long, fast slalom east, dropping to the bottom of the lap.
- **The fold**: a tight left hairpin back into the middle of the lap, then right, north.
- **The climb**: up through a tunnel and a kink right, over a crest.
- **The lip**: 20 m missing just past the crest, under an arch, the road falling away beyond it.
- **The sky bowl**: the highest hairpin, 42 m up.
- **The rift**: 55 m missing at the top of the long dive southwest, the far side 10 m lower, under
  an arch, a monolith standing over the void on either side: the thruster's.
- **The scree**: a slalom at the bottom of the dive, then **the step**, 16 m missing.
- **The switch**, a climbing S, and **the west bowl**, onto the run home.

It was designed with a scratch tool outside the repository that integrates the clothoids as `sim`
does and solves lengths so the loop closes, draws the plan colored by height with its gaps, gates
and pads and the profile under it, and drives the autopilot round in both styles and with every
personality. With the shipped car: gripping 32.3 s, drifting 31.8 s with two charges of turbo
fired, both clean; six ticks late, gripping 32.6 s clean and drifting 35.2 s with 8 ticks against
a wall; eight personalities from 31.3 to 33.7 s drifting; no fall in any of them. Drifting pays
less here than on the skyway, the rift being faster and more open: that is for the user to judge.

## Scenery

Props stand beside the road: a `scenery` list in the same file, next to the segments that draw it.
They are decoration and nothing else. The simulation never sees them — they stand outside the
walls, where no car can reach — so adding one cannot change how a circuit drives. The format is
`Prop` in `sim/src/track/scenery.rs`; props travel to clients with the rest of the
description, and the client builds them (`client/src/scenery.rs`).

```ron
scenery: [
    Gantry(at: 0.0),
    Chevrons(at: 46.0, side: Right, offset: 1.0),
    Grandstand(at: 196.0, side: Right, offset: 7.0, length: 64.0, rows: 8),
    Tower(at: 650.0, side: Right, offset: 16.0, height: 60.0, width: 28.0, depth: 28.0),
    Skyline(at: 395.0, side: Right, length: 175.0, depth: 80.0, height: 50.0),
    Tunnel(at: 455.0, length: 70.0),
],
```

A prop is placed the way a marshal would describe a spot on a circuit, never in world coordinates:

- `at`: meters along the lap from the start line.
- `side`: `Left` or `Right`, looking along the driving direction.
- `offset`: meters from the outer face of the wall to the face of the prop that looks at the road.
  Zero leaves it leaning against the wall, and no prop ever overhangs the road.

**Heights are measured from the road too.** A prop stands at the level of the road's edge on its
side, however high the relief carries the road there, and on the raised edge of a banked turn when
that is its side: a pylon 16 m tall beside the sky bowl rises 16 m over a road 47 m up. Whatever
stands beside the road also reaches down from there into the dark, on a footing that fades to
black on its way to 40 m below the ground, the ground nobody sees: the props seem to rise out of
nothing, the way the road floats over it. A tunnel hangs from the road instead, down to the
underside of its slab.

A prop therefore keeps its place when the road around it is redrawn, and the editor to come can move
one by dragging it along the road. Every size has a default, so a file writes only what it wants
different.

| Prop | What it is | Sizes, with their defaults |
| --- | --- | --- |
| `Gantry` | An arch over the road, its legs just outside the walls. It spans the road, so it takes no `side` and no `offset`. | `clearance` 7 m over the higher wall, `depth` 2 m |
| `Pylon` | A tapered mast ringed with neon, lit on top. | `offset` 3 m, `height` 16 m |
| `Grandstand` | A stepped tribune swept along the road, so it follows the curve, a neon line along the front of every row. | `offset` 5 m, `length` 40 m, `rows` 6 |
| `Billboard` | A panel on two posts, its neon frame and stripes facing the road. | `offset` 4 m, `width` 10 m, `height` 5 m |
| `Chevrons` | A marker board of stacked arrows, facing back down the road at the cars coming. | `offset` 1.5 m, `count` 3 |
| `Monolith` | A tall slab far out in the dark, seamed with neon. | `offset` 70 m, `height` 32 m, `width` 12 m |
| `Tower` | A tower of the city, banded with lit floors: a plain block, a block with a narrower one set back on top, or a block crowned with a spire and a mast. Its `at` is the middle of its face. | `offset` 8 m, `height` 40 m, `width` 18 m along the road, `depth` 18 m |
| `Skyline` | A district: towers filling the ground beside the road, `length` along it and `depth` out, none taller than `height`. | `offset` 6 m, `length` 120 m, `depth` 60 m, `height` 40 m |
| `Facade` | A long building following the road, floors of light across its face, its roof edges lit. | `offset` 4 m, `length` 60 m, `height` 18 m, `depth` 14 m |
| `Tunnel` | A roof over the road on walls just outside the road's own, ringed inside with neon every 12 m. It spans the road, so it takes no `side` and no `offset`. | `length` 80 m, `clearance` 9 m |

Some props read the road rather than the file. A gantry measures its clearance from the higher of
the two walls, so an arch over a banked turn rises with the road instead of cutting into it. A
marker board reads the bend from the centerline a dozen meters either side of it and turns its
arrows into the turn, whichever side of the road it stands on; where the road runs straight the
arrows point away from it, which is the sign of a board in the wrong place. A tunnel's roof stands
`clearance` over the road everywhere across it, so it banks with a banked turn. No tunnel may be
lower than 7 m, the chase camera hanging 3.2 m over the road and climbing a little higher where the
road dives, and no shot of the finish camera ever rises through a roof: under a tunnel, it is held
1.5 m under it.

**A district lays itself out.** Its towers stand in rows along the road, the nearest row the
lowest, their sizes and shapes rolled from the district's own numbers, so the same file always
builds the same city on every client. A tower is kept only if it stands at least 5 m outside the
walls of every part of the road, its own and any other (less, if the file places the district
closer to its own road than that), and clear of every building already standing: the towers, slabs,
facades and grandstands the file places, and the districts before it in the file. So a district
can be given more ground than there is — the whole space between two legs of a circuit and more —
and fills what is free: round the inside of a turn its rows close up and the towers that no longer
fit are left out, across a gap between two legs its far rows stop short of the other leg. The
clearance is checked at the corners, the middles of the sides and the middle of each tower, so a
turn can come a little closer between them.

Nothing else checks a prop against the rest of the circuit: a track is built from its own file, and
a prop is only known to stand off the road where it is placed. A circuit doubles back on itself,
though, so a stand or a tower set far out on one turn can land on another part of the lap. The
shipped tracks are checked against their whole centerline by a test in `server/src/content.rs`;
districts, which keep clear of the road themselves, are checked by the client's tests instead.

**How it is drawn.** The scenery's neon goes into the same mesh as the track's, and its surfaces
into one of their own, so the scenery adds no draw call whatever it holds: the track and its city
are four meshes, the fourth being the light the walls throw on the road (see [Night](night.md)).
The skyway and its city came to 131,400 triangles before the city had windows, piers and kerbs,
and come to 155,100 with them, built in 0.14 s on entering a lobby. The scenery's mesh casts no
shadow, for two reasons: under a tunnel roof the road was in the dark, the car and the road lost
but for their neon, and the city would be drawn into the shadow maps again every frame. Nor does it
shine: a building seen along its face, as the road shows them, catches the light whatever its
color, and a city of pale gray walls read as concrete by day. Matte, and darker than the props along
the road, the city recedes into the night behind its lights, and the road stays the brightest
surface in sight.

### The skyway's city

The skyway runs through a city at night, at the user's request "Fill skyway with scenery, add
kinds of scenery, make it really packed, so you can't see the circuit too far ahead". The
point is the last part: the lap folds back on itself like a comb, and bare, the road showed the
driver most of the lap at once, across the gaps between its legs and down from its high parts. Now
the gaps are districts, their towers rising 35 to 50 m over the road beside them; the hairpins wrap
around a tower each, the spire in the middle of the spiral the tallest thing in the city, 88 m up;
and three tunnels cover the crest of the switchback, the top of the dive and the crest of the climb.
Facades hug the inside of three bends, so the road beyond them comes into sight late, and the
props of a circuit line the road: chevrons into every hairpin and corner, pylons at their apexes,
stands along the line and around the two bowls, billboards along the legs, and arches over the line
and the bowl's exit. 50 props in all, among them 15 districts of about 300 towers.

How much of the rest of the lap the driver sees was measured with a scratch tool outside the
repository, which lays the city out as the client does and casts lines of sight from the chase
camera, every 25 m round the lap, to the road more than 150 m ahead, past every tower, facade and
tunnel. Summed over the lap, the stretches of road seen fell from 6,270 to 786, and what is left is
almost all the road ahead along the same leg, never more than about 250 m of it: from the start, the
launch down to the bowl; from the top of the dive, the dive. The other legs are out of sight but for
a few glimpses between towers.

The esplanade, gone since the skyway, was dressed with the first six kinds, 83 of them: an arch
over the line and one over each of its three bottlenecks, a marker board into every turn, stands
around the turns worth watching, pylons at the apexes, panels along the fast stretches, and nine
slabs out in the dark.

## Validation

`Track::build` refuses a description, with a message pointing at the faulty segment, when:

- the width, a length or a radius is not a positive number;
- a radius is not larger than half the width, since the inner edge of the road would fold over
  itself;
- the transitions are longer than the turn itself;
- a banking is outside 0 to 60 degrees, or a banked turn has no transition;
- the angles do not add up to one full turn, left or right;
- the centerline does not come back within 0.5 m of the start line;
- the road runs over itself: two centerline points less than a road width apart, although further
  apart along the loop than `π × width / 2`. The tightest turn allowed has a radius of half the
  width, and half a turn of it is that long and brings the centerline back exactly a width apart,
  so closer points are not a turn but another part of the road;
- a height of the relief sits off the lap, is not a number, or comes no further along the lap than
  the one before it. The message names it by its place in the list;
- the centerline climbs or dives anywhere at more than 25 degrees;
- a narrows sits off the lap, is not a positive number of meters long, pinches the road below 8 m
  or wider than the track itself, has a negative blend, or is longer than the lap with both its
  blends. The message names it by its place in the list;
- a breach sits off the lap, is shorter than 4 m or as long as the lap;
- a prop of the scenery stands off the lap, over the road (a negative `offset`), has a size that
  is not a positive number of meters, or is a tunnel with its roof less than 7 m over the road. The
  message names the prop by its place in the list.

Smaller gaps than that are integration or rounding error, and are spread evenly along the loop so
the road closes exactly. Comparing every pair of points costs about half a million distances on a
drift circuit, once when the track is built.

## Built geometry

The built `Track` samples the centerline every meter or less, at even spacing, integrating
curvature exactly within each piece. The simulation and the renderer both work from those points:

- `project` finds a position's distance along the track and its lateral offset, positive on the
  left. The simulation uses it for walls, and later for lap progress.
- `point_at` returns the centerline point, direction, curvature, height of the relief, banking and
  half width at any distance; `half_width_at` is that half width alone, which is what the walls
  are read from.
- `surface` gives the height and uphill direction of the road under a position, the relief and the
  banking together, and `height_beside` the height at any lateral offset. The simulation uses the
  slope, the renderer the heights. Across the road the slope is exact. Along it, it is measured
  half a meter of centerline either side, as far from the centerline as the position, and divided
  by how much shorter or longer the road is there than on the centerline; that takes in the banking
  rising along a transition as well as the relief.
- `grid_slot` places cars on the starting grid behind the start line, alternating sides.

`project` compares against every segment of the centerline. That costs about a thousand distance
computations per car per tick on a half-kilometer track, which is negligible for now; a spatial
index or a search around the previous position can replace it if tracks grow large.
