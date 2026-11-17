# Rewards

Since v0.1.69 (protocol 38), the more a player races, the more they unlock, the user's word: "Je
veux un systeme de récompenses. Plus on joue plus on gagne des trucs sympas, car je veux que les
futures joueurs, qui seront j'espere en partie des anciens joueurs de Rocket Racing, retouvent
l'esprit Fortnite. [...] choix de trois belles voitures gratuit. Et fais en sorte qu'on puisse
gagner 3 autres voitures par les récompenses. A toi de voir le système de recompenses, des
niveaux, quelque chose de peu voire pas visible, etc... trois types de sillons offerts, trois à
gagner. Ajoute des klaxons, trois offerts, trois à gagner."

## Experience and levels

Every race pays experience (XP) to each player still in the lobby who raced it
(`protocol::rewards::race_xp`): 60 for taking part, 120 more for finishing, 30 for every player
finished ahead of, and 60 for winning against at least one other. A solo race finished pays 180.

XP makes levels: the first level costs 300, each one after it 60 more (`xp_for_level`), and they go
on past the last reward. Nothing is bought, nothing is ever lost.

## What is won

Of the six cars, six trails and six horns, the first three of each are free; the others are
unlocked by level, a horn, then a trail, then a car, three times over (`REWARDS`):

| Level | Unlocks |
|---|---|
| 2 | horn LASER |
| 3 | trail ZIGZAG |
| 4 | car PHANTOM |
| 5 | horn FANFARE |
| 6 | trail HELIX |
| 8 | car TITAN |
| 9 | horn SAUCER |
| 10 | trail ARROWS |
| 12 | car NOVA |

The last car is level 12, 6,600 XP: about 37 solo races finished, fewer racing others. Paints,
neons and trail colors are all free.

## Shown little

The user asked for something "peu voire pas visible". The level shows in two places only: small,
in the garage's corner (`LEVEL 3 · 45%`), and on the results, one amber line under them, what the
race paid, the level it leaves the player at, and anything it unlocked by name (`+180 XP · LEVEL
2 · NEW HORN: LASER`). In the garage, what is not won yet is still shown and previewed, its name
dim and `LOCKED · LEVEL 8` under it; leaving the garage keeps what was chosen of what is unlocked,
anything locked going back to what it was.

## Who keeps it

The server keeps every player's XP in `progress.ron` (`--progress`), beside `ratings.ron` and
`accounts.ron` and for the same reason: outside the data directory, which deployments replace,
written whole through a temporary file after every race (`server/src/progress.rs`). The lobby
works out the XP at the end of a race, shows it in `LobbyPhase::Results::xp`, and hands it to the
directory to keep (`LobbyEvent::RaceFinished`). The player is a guest's device key or an account's
identity, so guests progress too; a guest who signs up later starts again.

`ServerMessage::Progress { xp }` is sent with every profile: after the welcome and each time the
player comes back from a lobby. On the client, `lobby::PlayerProgress` holds it, and counts a race's
pay once, as its results arrive, so the results and the garage are up to date before the server
says it again.

The server enforces it: a look in `Hello` or `SetLook` with a car, a trail or a horn the player has
not unlocked has it put back to the first free one (`rewards::allowed`), so nobody shows others
what they have not won. A first run is given a look at random among what is free
(`CarLook::random`).
