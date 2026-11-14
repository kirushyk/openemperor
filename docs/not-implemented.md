# Not implemented yet

What the original game has that we don't do yet, or do only partly. Which units and buildings
we are missing is in [units.md](units.md). This list is about everything else.

## Models and their effects

- **Effect helpers in the models**: the parts whose names start with `?` or `#` that are not
  lights are not drawn. These include:
  - muzzle flashes (`?bigflash…`) and tracer bullets (`?mgbullet…`);
  - the windtraps' wind (`?leftwind`, `?rightwind`);
  - the palace's and the lightning gate's lightning (`?lightning…`);
  - Ordos shields (`?{SHIELD}`) and the Leech's mark (`?~~2{LEECH}`);
  - the sandworm's mouth and the tornado (`?sandymouth`, `?worm02`, `?tornado`).

  Many of these have picture slides in the model that would play once they are drawn.
- **The effects block of a model**: we read only its animation runs and its picture slides
  (kind 7). The rest is not read. That is:
  - its effect definitions: `Dlight`, the smoke (`sm`), `GFlash`, `fire`, `Engine`;
  - its other event kinds: 1–6, 8 and 9.

  So the model's smoke, fire and flashes don't play, and its lights cast no light.
- **Animation runs not played**:
  - turning on the spot ("Turn Left", "Turn Right");
  - starting and stopping ("Move Start", "Move Stop");
  - idle variants ("Idle 0", "Idle 1");
  - infantry crouching ("Crouch", "CrouchFire");
  - "Win";
  - each kind of death: "Blow Up", "Shot", "Burnt", "Gassed", "Run Over", "Deployed Death",
    "Explode".
- **Smoke from buildings** comes from our own pipe and stack positions, not from the models'
  `#smokey…` nodes. This covers the deployed MCV, the refinery and the factory.
- **Tracks** start their picture from the beginning again whenever a machine stops. The
  original probably keeps them where they were.
- **Firing points** are taken from each model's rest pose, with its aim. The barrel's recoil
  during "Fire" is not taken into account.
- **A deployed Kobra** turns its whole body to aim, as other vehicles do. It does not turn
  only its turret.
- **Lower-detail models**: only the `_H0` models are read.

## Lights

- **The glow of the original's lights** is drawn: searchlight beams, the spots they throw,
  lamps and flares. They do not light anything around them. Only explosions, fires, eruptions
  and gunfire do.
- **Ground lighting**: on maps with a blue light, buildings take its hue, but the ground stays
  orange. The cause is not found yet.
- **The map's light file**: lines 4 and 5 of `test.lit` make a colour ramp that we don't use.
  What the original uses it for is not known.

## Maps

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

## Other platforms

- **Windows and Linux**: their backends (`shim/windows_d3d11.c`, `shim/linux_vulkan.c`) draw
  only the flat picture. They have no lit 3D world, so no model pictures, map wash, decals,
  glows or spice on the ground. Their picture, wash and sound calls do nothing.
