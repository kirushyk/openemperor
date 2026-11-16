# Not implemented yet

What the original game has that we don't do yet, or do only partly. Which units and buildings
we are missing is in [units.md](units.md). This list is about everything else.

## Models and their effects

- **Effect helpers in the models**: the parts whose names start with `?` or `#` are not
  drawn, except for lights and for faces with see-through `@` pictures (the tornado, the
  buildings' ground skirts). These include:
  - muzzle flashes (`?bigflash…`) and tracer bullets (`?mgbullet…`);
  - the windtraps' wind (`?leftwind`, `?rightwind`);
  - the palace's and the lightning gate's lightning (`?lightning…`);
  - Ordos shields (`?{SHIELD}`) and the Leech's mark (`?~~2{LEECH}`);
  - the sandworm's mouth (`?sandymouth`, `?worm02`).

  Many of these have picture slides in the model that would play once they are drawn.
- **The effects block of a model**: we read its animation runs, its picture slides (kind 7)
  and its sounds (kind 9), each at its frame. The rest is not used. That is:
  - its effect definitions: `Dlight`, the smoke (`sm`), `GFlash`, `fire`, `Engine`;
  - its other event kinds: 1–6, 8, 10–12.

  So the model's smoke, fire and flashes don't play, and its lights cast no light.
- **How the effects block is laid out** is only partly worked out. Ahead of the effects is a
  table of how many start at each frame. Those it gives the first frame are not among the
  effects that follow, and each group of effects has one more than the table counts. We
  take that one to be at the last frame, which puts the infantry's "Crush" in "Run Over"
  where it belongs, but it is a guess.
- **Animation runs not played**:
  - turning on the spot ("Turn Left", "Turn Right");
  - starting and stopping ("Move Start", "Move Stop");
  - idle variants ("Idle 0", "Idle 1");
  - infantry crouching ("Crouch", "CrouchFire");
  - "Win";
  - "Explode" (the saboteur's own blast).
- **Deaths without the disc**: our ragdoll is commented out, so without the disc's models a
  dead soldier is not shown. A wound comes from the last weapon that hurt the soldier; mines,
  the superweapon and the saw count as shots.
- **Smoke from buildings** comes from our own pipe and stack positions, not from the models'
  `#smokey…` nodes. This covers the deployed MCV, the refinery and the factory.
- **Tracks** start their picture from the beginning again whenever a machine stops. The
  original probably keeps them where they were.
- **Firing points** are taken from each model's rest pose, with its aim. The barrel's recoil
  during "Fire" is not taken into account.
- **A deployed Kobra** turns its whole body to aim, as other vehicles do. It does not turn
  only its turret.
- **Lower-detail models**: only the `_H0` models are read.
- **Invisible units**: their own side sees them painted with the disc's `hackedstealth0–7`
  shimmer, one frame per update and seen through as much as it shows. The original also draws
  two copies first, each blended with what is behind it (`DESTCOLOR`) and jittered by 1 and
  then 2 units; we don't draw those.

## Lights

- **The glow of the original's lights** is drawn: searchlight beams, the spots they throw,
  lamps and flares. They do not light anything around them. Only explosions, fires, eruptions
  and gunfire do.
- **Ground lighting**: on maps with a blue light, buildings take its hue, but the ground stays
  orange. The cause is not found yet.
- **The map's light file**: lines 4 and 5 of `test.lit` make a colour ramp that we don't use.
  What the original uses it for is not known.

## Maps

- **The sea** (faces painted `sea.tga` or `icesea.tga`) is drawn with our own water shader,
  not the original's picture. The disc's maps have no sea bed, so the sea is drawn as
  `SEA_DEPTH` deep everywhere and doesn't grow shallow near the shore. Its mirror lies at the
  lowest of it, which on a river that falls (Caladan: River Delta) is a little under parts of
  it. Only the Metal backend draws it as water.
- **Campaign map names** come from their folders, such as "Draconis Iv Hw" or "Hk Start". The
  game's text names only three of them.
- **Campaign maps with only one start** are listed, but a skirmish needs more.
- **The map dropdown** doesn't show that there are more columns to scroll to.
- **Bridges**: the land mesh shows them, but the height grid has only their decks, and what is
  under them counts as cliff.

## Units at work

- **Unreachable spice**: a harvester looks for the nearest spice it can drive onto, not the
  nearest it can get to. Spice cut off from it, such as on a rise of its own, keeps drawing
  it back.
- **Groups and the stairs**: a group with any vehicle in it is only sent to spots vehicles can
  reach, even its infantry.

## Sounds

- **What plays**: the units' answers; every unit's firing, as its model's "Fire" run has it;
  and every unit's death: a soldier's from the run for its wound, a machine's or an
  aircraft's from its "Explode", and a building's or a tower's from its ruin's (`_H3`) model.
  Only what the player sees on the screen is heard.
- **Not played yet**, though the models and tables have them:
  - engines and moving ("Move Start", "Move", "Move Stop": `TrikeMove`, `HKTrackStartMove`…)
    and footsteps;
  - deploying and packing up (`KindjalDeploy`, `MortarDeploy`, `MCVDeploy`), and the
    Kobra's and the pop-up turret's rising;
  - aircraft taking off, flying and landing, and the carryalls picking up;
  - the harvester eating and unloading spice;
  - buildings' own loops (`HKAmbient1`, `ATFactoryNoise1`), their burning once badly
    damaged (`ATBurningLarge` in the `_H2` models), and building and construction sounds;
  - hits and explosions where shots land (`ShellDetonation`, `RocketDetonation`), apart from
    those in a soldier's death;
  - the interface (`UISFX.TXT`, `InterfaceSFX.txt`) and the mentat's speech.
- **Names that lead nowhere**: many models name sounds no loaded table has, such as
  `ATMedium1`, `ATLarge1`, `HKLarge1`, `ORSmall1`, `GunHit`, `Leech` and `Explode2.wav`. Only
  `ImportedSfx.txt` lists most of them, pointing at sounds that are in neither bag. `Sfx.txt`
  doesn't include that file, so the original likely plays nothing for them either, and
  neither do we. This leaves the carryalls, the MCV, the trike, the drone, the Eye in the
  Sky, the dust scout and the Devastator with no death sound.
- **Ruins missing from the disc**: the Ordos buildings, and the Harkonnen hangar, landing pad
  and refinery, have no `_H3` model, so they go up without a sound. So does the Harkonnen
  palace, whose ruin plays none.
- **The tables' settings**: we read `Volume` and `Limit`, and pick among a section's sounds in
  turn. `Range`, `Priority`, `Type`, `MinVolume`, `Delay`, `FShift`, `VShift`, `LoopLimit` and
  `Control`'s `attack` and `decay` are not read.
- **Where a sound is**: every sound plays at its section's volume and in the middle. It
  doesn't fade with distance from the camera or pan to where it is.
- **Radio static**: an aircraft's answers are laid over static in the original (its tables'
  `Control = random attack decay`). Only the speech plays.
- **Many sounds at once**: the macOS backend plays up to 16 battlefield sounds together. Past
  that, a new one takes the place of the one that would end soonest.

## Interface

- **The Ordos landing pad's build button** shows our symbol and name: the disc has no picture
  for it, and no model either.

## Other platforms

- **Windows and Linux**: their backends (`shim/windows_d3d11.c`, `shim/linux_vulkan.c`) draw
  only the flat picture. They have no lit 3D world, so no model pictures, map wash, decals,
  glows or spice on the ground. Their picture, wash and sound calls do nothing, the
  battlefield's sounds too.
