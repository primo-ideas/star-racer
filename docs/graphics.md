# Graphics quality

Since v1.0.0-rc4, at the user's word: the game measures the machine it runs on, silently ("je ne
veux pas déranger avec un popup d'autorisation"), and scales how fine it is drawn to it, from about
60 frames a second on an entry-level phone, ugly being fine there, to 60 frames a second in a
browser on a GTX 1050 Ti at the top; and the top was raised. Code: `client/src/quality.rs`.

## Four levels

| | LOW | MEDIUM | HIGH | ULTRA |
| --- | --- | --- | --- | --- |
| Pixels drawn on a web page, per page pixel (at most the screen's density) | 0.55 | 0.8 | 1 | 1.5 |
| The moon's shadows | none | 1024 | 2048 | 4096 |
| Glow (bloom) | off | on | on | on |
| Multisampling | off | off | off | 4× |
| The city's traffic and searchlights | off | off | on | on |

HIGH is what every machine drew until then. ULTRA is new: smooth edges and sharper shadows, and
on a dense screen more pixels than the page has. A native window always draws its own pixels: the
first row is the web's only, where a phone's dense screen is the costliest thing of all.

## Fitted to the machine

Nothing that brings up a prompt, only what the game is told anyway (`measure_machine`,
`detect`):

- the graphics adapter the renderer was given: its name, and whether it is a discrete GPU, an
  integrated one or software (a browser often hides the kind and gives the name, read for
  `GeForce`, `RTX`, `GTX`, `Radeon RX`, `Arc A`, `Intel`, `Adreno`, `Mali`, `PowerVR`, `Apple`);
- the processor's threads (natively `available_parallelism`, in a browser
  `navigator.hardwareConcurrency`);
- in a browser, the memory it admits to (`navigator.deviceMemory`, where there is one), and the
  screen's pixel density;
- whether the screen is a touchscreen.

A discrete GPU, or a strong name, holds ULTRA; an integrated one MEDIUM; software LOW; a phone's
chip LOW, MEDIUM for the big ones; four threads or fewer at most MEDIUM; 4 GB or less, LOW. The
level chosen is logged at `info` with what it was chosen from ("graphics fitted to the machine").

Then the race measures it: on AUTO, a frame rate under 48 for three seconds of a race steps the
level down one (`adapt`), at most every six seconds, never under LOW, and never back up past what
was detected. A hitch over half a second is not counted (a hidden tab, loading).

## In the settings

GRAPHICS & SOUND, a sub-menu of the settings (`ui/sound_graphics.rs`, `--sound-graphics`): AUTO,
showing the level it draws at, or LOW, MEDIUM, HIGH or ULTRA, kept with the menu choices
(`Preferences::graphics`), with what the machine was taken for written under it. A level picked
holds, races included.

## Measured

On the development machine (i5-9600K, GTX 1050 Ti), natively: detected as a discrete GTX 1050 Ti
with 6 threads, ULTRA, a drifting race at 60 frames a second with vsync. Not measured: a phone, nor
the browser on this machine.
