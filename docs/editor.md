# The level editor

Since v1.0.0-rc2, at the user's word: "Pour la v1.0.0rc2, il n'y aura qu'une chose, et un gros
morceau : fais moi un éditeur de niveau". Asked, they chose it **in the game** (natively and in the
browser, not a tool apart), its circuits **sent to the server** for every lobby to race on, and
**everything editable** from the first version: the road, what is played on it, and the scenery.

## What it is

A screen of its own, EDITOR on the first screen (not on a phone, the editor being laid out for a
mouse and a wide screen). It needs the server, to try and to publish, so the button connects first,
as LOBBIES does (`Intent::Editor`). Code: `client/src/editor/` (`mod.rs` the state, the draft and
the actions; `model.rs` the parts and their numbers, without anything shown; `map.rs` the circuit
seen from above; `panel.rs` the panel), and `sim/src/track/editing.rs`.

- **The map** is the circuit as it will be raced, built by the race's own code
  (`track::spawn_scene`, the road, walls, markings, gaps, pads, checkpoints and scenery; not the
  city's traffic and searchlights), seen from above and a little behind, so the relief reads. The
  wheel zooms, the right or middle button drags, WASD and the arrows pan; it frames the whole
  circuit on opening and on LOAD. A left click puts the **cursor** (a magenta bar across the road)
  where it falls on the road, and picks the part of the open tab nearest it: the segment under it
  on ROAD, the nearest part within 80 m on the others. The part selected is drawn white.
- **The panel**, down the left: the circuit's name (typed), the tabs ROAD, RELIEF, GAME and
  SCENERY, what can be ADDed to the open tab (at the cursor; a segment after the one selected),
  the list of its parts (the wheel scrolls it), the numbers of the part selected, each with «, ‹, ›
  and » (one step and ten), DELETE, what the editor has to say, the circuit's length and how many
  segments it added to close it, or why it does not build, LOAD with the server's circuits, and
  TRY, PUBLISH, NEW and BACK (Escape too).
- **The parts**: ROAD, the road's width and the segments (straight: length; turn: angle, radius,
  transition, banking); RELIEF, the heights; GAME, gaps, breaches, bottlenecks, pads and checkpoints;
  SCENERY, the ten kinds of prop, with their sizes (a new one takes the defaults a track file
  gets, read through the file format itself). The heights and checkpoints stay in order along the
  lap, the one moved followed. Every number keeps within bounds; a turn stepped through zero
  turns the other way. The bounds are generous: what the track file refuses (a gap too near the
  line, a checkpoint in a gap, a road that overlaps itself, a slope over 25°) is said in the status,
  in the simulation's own words.

## Closing the road

A circuit's segments must turn through one whole turn and end where they began (docs/tracks.md),
which nobody laying turns by hand hits. So the player lays the road as far as they like, and the
editor closes it: `editing::closing` finds the shortest way back to the start line, heading `+x`,
made of a turn, a straight and a turn of one radius (the shortest path of a car that can only turn
that tightly, Dubins' four words, LSL, RSR, LSR and RSL), keeping only those that turn the whole
circuit through exactly one turn. Its turns have no transition nor banking; the radius is the
editor's (70 m to begin with, CLOSING TURNS on the ROAD part, 20 to 500 m). The stretch it adds is
drawn in a pale blue line on the map. A circuit already closed, as a published one loaded back,
gets nothing added. When the road cannot be closed, or the closed circuit does not build, the
road as laid is drawn in red and the status says why.

## Trying and publishing

- **TRY** (`ClientMessage::TryTrack`): the server checks the circuit as it would a published one,
  then opens a lobby of one, one lap again and again, that the lobby list never shows
  (`Registry::trying`), its track keyed `~try` (`TRY_TRACK`): the client already holds the
  circuit, having sent it, and keeps it under that key for the lobby to draw, forgetting it on
  coming back to the menus. Leaving the lobby comes back to the editor. A circuit that does not
  build is refused with `JoinError::InvalidTrack`. `--editor --editor-try` presses it for an
  unattended run.
- **PUBLISH** (`ClientMessage::PublishTrack`, answered `ServerMessage::Published`): an account's,
  never a guest's. The key is `user-` and the name in lower case, every run of other characters a
  dash (`content::user_track_key`); another player's circuit, or one of the game's, of the same
  name (whatever its case) is `NameTaken`; the author's own is replaced. The server builds it
  (`content::check_user_track`: at most `MAX_TRACK_ITEMS`, 400, of any list, a lap of at most
  `MAX_TRACK_LENGTH`, 20 km, and road wide enough for the car everywhere), keeps it in its own file
  (`user-tracks/<key>.ron` beside the ladder, `--user-tracks`, which deployments leave alone, with
  the author's identity and name), and sends every browsing player the whole list again
  (`ServerMessage::Tracks`). `TrackInfo::author` names who published it, shown in the lobby
  creation dialog ("by Ada"). Quick play and ranked keep to the game's own circuits.
- **The draft** is kept beside the player's other choices (`editor.ron` next to `preferences.ron`),
  so it is there again on the next run; NEW starts again from a straight, a banked half turn and a
  straight; LOAD takes any of the server's circuits as a new draft, all its segments kept.

## Not done

- No undo; NEW and LOAD replace the draft without asking.
- Nothing is dragged on the map: parts are placed at the cursor and moved with their numbers.
- Not on a phone.
- A published circuit cannot be taken down from the game: its file is removed on the machine.
