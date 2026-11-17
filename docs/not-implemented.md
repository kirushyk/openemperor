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
- **A deployed Kobra** swings its raised cannon round on the model's `gunpivot` at the
  original's 4° an update and raises it as other guns do. The original's limits of ±190° round
  and −90°…30° up (`ORKobraDeployedGun`) are not kept, nor its 4° an update for raising.
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

## Weapons

Health, armour and damage are the original's `Rules.txt` ones, divided by six so that its
infantry's 600 are our 100: each unit's `Health` and `Armour`, each turret's bullet `Damage`
and its warhead's share against each armour from `[WarheadTypes]`, and each turret's
`ReloadCount` at 25 updates a second. Not done yet:

- **Salvos and sprays** keep our own rhythm: the missile tank's, ornithopter's, gunship's and
  rocket turret's salvos and the Minotaurus's four cannons, and flame and gas sprays. A spray
  hit is the share of the original's shot that falls in its time.
- **Our own numbers** are still used for the gas a cistern leaves, the Tleilaxu's poison, the
  buzzsaw's saw, and the superweapons (`DeathHandBomb`, `Hawk_B` and the Ordos beam).
- **Projectiles** fly and burst as their bullets: missiles at their `Speed` and `TurnRate`
  per update, straight until their `HomingDelay` and straight for good if not `Homing` (the
  Devastator's plasma); bursts as far as `BlastRadius`, weaker outwards unless
  `ReduceDamageWithDistance` is false, and hurting friends by `FriendlyDamageAmount`. Still
  ours: how fast shells and bombs fly (their bullets have no `Speed`, or the bomb's 8 we do
  not use), the arc they take, `MinRange`, and the bursts of bullets that strike at once (the
  Kindjal's and the Kobra's 64 on its tracks), which hurt only what they hit.
- **Infantry on rock** take less damage in the original: their armour `None, 50, InfRock`
  halves it there.
- **Shields**: the laser tank's `ShieldHealth` 300 and its self-repair.
- **The repair vehicle** heals by our own rate; the original's `RepairRate` is 12.
- **By house**: the original gives the Harkonnen engineer 600 health (ours 400 for every
  house) and the Atreides outpost 100 (ours 2500). A deployed MCV keeps the MCV's 4500 rather
  than the construction yard's 4000, though it is armoured as the yard.
- **The exe**: none of this was read from `GAME.EXE`; only `Rules.txt`.

## Power and upgrades

Windtraps make 100 power and buildings use `Rules.txt`'s `PowerUsed`; the barracks, refinery,
factory and outpost need a windtrap; the turrets marked `DisableWithLowPower` stop while
their owner is short. Buildings are upgraded from an Upgrades tab for their `UpgradeCost`,
and what is marked `UpgradedPrimaryRequired` waits for it. A refinery starts with its middle
pad and gains one, with a harvester, for each of its two upgrades (`RefineryDock`). Opponents
build windtraps to stay powered and upgrade once they have a few fighters. Not known or not
done:

- **What else low power does**: ours halves the pace of building, making and upgrading, and
  puts out the outpost's radar. The original's rule is in `GAME.EXE` and was not read; its
  strings name `PowerDown`, `LowPower` and `RadarOff`.
- **How long an upgrade takes**: only the refinery pad has an `UpgradeBuildTime` (720); the
  others take as long as building the building.
- **Which building is upgraded**: the original speaks of the *primary* building of a kind.
  Ours upgrades the first one standing that is not, and each building counts for itself: one
  lost takes its upgrade with it.
- **`UpgradeTechLevel` and `TechLevel`** hold nothing back.
- **The pads' look**: a refinery is drawn with all three pads from the start; only the
  upgraded ones take harvesters.
- **The mini-houses' upgraded units** (`IXProjector`, `TLLeech`, `IMADVSardaukar`,
  `FRADVFremen`, `GUNIABTank`): their buildings can be upgraded, but we have none of these.
- **The power gauge** round the minimap shows the share of power to spare and the number;
  the original's bar is its own model (`Power%02d`).

## Walls

Walls are placed as the original's are: a click picks the first square, the route to the
cursor shows one square wide, a second click sets it, and a right click lets go of the first
square, then of the wall. The squares then go up one after another, 20 credits and 1.5
seconds each, as `Rules.txt`'s `Cost` and `BuildTime` scaled. Each square shows the disc's
piece for how it joins its owner's walls beside it. Not done yet:

- **How the original routes a wall**: its `GAME.EXE` places walls in a cursor mode
  (`WallSection`, `WallPlacement`) that was not read through. Ours takes the shortest way
  over free rock the player has seen, turning as few times as it can, and goes up to 12
  squares beyond the box round its two ends to get round what is in the way. Only its first
  square has to be near one of the player's buildings.
- **The placing cursor**: the original shows `placement\build_wall.xbf`; ours colours the
  squares as for other buildings.
- **Going up**: the original's `_HC` model shows a wall rising out of the ground. Ours goes up
  under the same crane as other buildings.
- **Sounds**: the original's `WallThud` as a square goes down.
- **Opponents** never build walls. The original's AI does, with its `MinMoneyToStartBuildingWalls`
  and gives up after a while ("AI has been building walls for %d minutes so aborting").
- **Ix walls**: the disc has `IX_Wall` pieces too, which nothing we have uses.

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
- **The volume options**: Music Volume sets the music and the movies, Voice Volume the units'
  answers, and Sound Volume the battlefield's sounds. Movies play at the music's level, and
  the options are not kept between runs.
- **Many sounds at once**: the macOS backend plays up to 16 battlefield sounds together. Past
  that, a new one takes the place of the one that would end soonest.

## Camera

- **What follows the original**: its `CAMERA.INI`. The camera stands 1290 from what it looks
  at, divided by its zoom, which goes from 0.5 out to 1.25 in. It looks 50° down, turned 45°,
  and a battle opens from 0.35 out, turning 45° on its way in over 4 seconds.
- **Not known yet**:
  - its field of view: ours is about 40° from top to bottom;
  - the keys the original turns and resets the camera with. Its commands are Rotate Left,
    Rotate Right and Reset Camera, but their default keys are not in a text file. We use `,`
    and `.` to turn by 15° and `/` to reset, and two fingers turning on a trackpad;
  - how far a wheel notch zooms. Ours moves a twentieth of the way;
  - `MinTilt`, `MaxTilt` and the inertia settings. With `InertiaAngle 0` the inertia does
    nothing, and nothing else is known to change the tilt.
- **The minimap** stays as it is when the camera turns.

## Interface

- **Build menus** list what can be made now, packed from the first cell in the order of the
  original's `[UnitTypes]` and `[BuildingTypes]`. Not done yet:
  - what we have no units for is missing from them: the Ordos AA trooper, the sonic tank
    and the deviator;
  - tech levels (`TechLevel`): nothing is held back by them;
  - the original's Next and Prev buttons, for a menu of more than 15: none of ours has that
    many;
  - the "New construction options" line when something new can be made.

- **The build menu's tabs** show the disc's symbols (the sidebar model's `infantrybody`,
  `vehiclebody` and `buildingbody`) flat, in one colour. The original draws them as glowing
  3D shapes (`!06%crimsonglow.tga`), with flares over the selected tab. The Upgrades tab
  shows its `upgradebuildings`. Its Starport tab is our wide Starport button.

## What the battlefield says

The player hears the original's spoken messages off the first disc's `DIALOG.BAG`, the lines
its `UI_Comp.txt` gives: unit ready, training, construction started, ready to place,
construction complete, unit and building lost, building sold, base and harvester under
attack, low power, insufficient funds, upgrading, guard on and off, waypoints placed and
added, retreat, superweapon ready, victory and defeat. The same line is not said again
within three seconds, an attack or low power within twenty. Not done yet:

- **The house's own voice**: the original's lines differ by house in a few places (the
  table gives Atreides their own "Structure ready to place"); most of ours are shared.
- **Priority and interrupting** (`Priority`, `Interrupt`, `TimeOut` in `Dialog.txt`): a new
  line cuts off the one playing.
- **Messages we have no event for**: wormsign, worm attack, incoming superweapons, buildings
  captured or stolen, leech and contamination, carryall under attack, reinforcements, the
  T-minus countdown, new construction options, "cannot build here", game loaded and saved.
- **Unit descriptions** (`UI-B…`) said when a unit is first shown.
- **The Mentat's mission lines** (`ScriptDialog.txt`, a house disc's `DIALOG.BAG`), which
  need the mission scripts to run.

## The campaign

Confirming a house plays its opening movie off its own disc (`A00_F00E` for the Atreides,
`O00_F00E` for the Ordos, `H01_F00E` for the Harkonnen, as `MOVIES.TXT` names them), or goes
straight on without the disc, then starts its first battle on the original's map for it
(`#u1`, `#u2`, `#u3`) with the 10 000 credits its script gives. Not done yet:

- **The mission's script** (`ATStart.tok`, see `tok-format.md`) is not run: the battle starts
  as a skirmish does, with an MCV a side, not with the script's base, waves and Mentat lines,
  and it is won by beating every enemy, not the three Harkonnen buildings.
- **Later missions** go round the skirmish maps as before.
- **Between missions**: the territory map, choosing where to attack, the new units screen
  and the question when a territory is attacked are not there (backlog task 33).
- **The end mission** with the Emperor Worm (task 34).

## The other discs

The game looks for its four discs as `/Volumes/EMPEROR1` to `4`. The one that installs it has
what every house shares; a house's own (the second is the Atreides') has that house's music,
movies and briefings. Battle music plays the long tracks of every music bag found, one bag
after another, round and round. Not done yet:

- **A house disc's menu, map and score music** (`AT_Menu1`, `AT_Map1`, `AT_Score1a`) is not
  used: the menus play the first disc's `IN_Menu`.
- **Which music a mission asks for**: every long track plays in turn, whatever the house.
- **A house disc's dialogue bag** (`DIALOG.BAG`, 1163 lines on the Atreides') is not read.
- **Discs mounted under other names**, or copied to a folder, are not found.

## Starport

A starport (1500, needing a hangar; the palace needs it) opens the Starport menu: what the
original marks `Starportable` among what we have, each at a price that moves every 60
seconds by up to 40% either way, with a stock that has a 90% chance of one more every 40
seconds. Up to six go on an order, paid at once, and the frigate brings them 100 seconds
later, as the original's `[General]` rules say. Not as the original, or not known:

- **The frigate** is not seen: what it brings appears by the starport as from a factory. The
  original's flies in (`Frigate`), lands and can be shot at; `FrigateTimeout` is unused.
- **Stock**: how much the merchants start with and the most they hold are ours (up to 3, and
  6). Units the house could not build yet are sold all the same, as we believe the original
  does.
- **Purchase and Cancel** are the menu's last two cells; the original has buttons of its own.
- **Opponents** never order from a starport, nor build one.
- **The sonic tank, the deviator, the Imperial and Guild units** on the original's list: we
  have none.

## Sidebar buttons

The five small buttons are the original's, each with its symbol from the sidebar's model:
Repair, Sell, Waypoint, Retreat and Guard. The two wide ones open the Upgrades and Starport
menus, as its tabs of those names do. Not as the original, or not known:

- **Repair** mends a building from nothing in 30 seconds for half its price. The original's
  rate (`RepairRate` 12) and price were not worked out.
- **Retreat** asks with a line of chat and gives the battle up on a second click within five
  seconds. The original asks in a pop-up (`RetreatPopUp`), and takes the units off the map in
  a campaign.
- **Guard mode** sends a unit after enemies within the original's `GuardTileRange` of 12
  tiles of where it was put on guard, and back. What else the original's does is not known.
- **Waypoints** show as flags. The original has its own markers (`Waypoints\WP_*.xbf`) and
  can close a route into a loop.
- **The buttons' look**: flat symbols on our buttons; the original's glow and spin
  (`AnimMesh`), with flares.
- **Deploy** has no button any more: the D key, or a click on the unit, as before.

## Menus

The main menu and the single player menu show the original's own scene off the disc,
`frontend\main.xbf`: its ring frame, title, Arrakis turning, glows, nebulas, horizons and the
tunnel of glyphs, drawn flat from where its camera stands, with our buttons where its own
are. Not done yet:

- **The storm clouds over Arrakis** (the scene's `cloud`) are left out: with what we read of
  them they come out as blocks.
- **The buttons** are ours, with our font. The original's are text in its own font
  (`FONT\STANDARD.MAP`) with a box and a highlight (`@button_64.tga`), and sounds
  (`NavButtonClick`, `NavButtonRollOver`).
- **Every other screen** has its own scene we do not show yet: `Options.xbf`,
  `HouseSelect.XBF`, `Wol_SetupSkirmish.XBF` (the skirmish setup), `campaign.xbf`,
  `loadsave.xbf`, `Credits.xbf`, `ingameoptions.xbf`, `popup.xbf` and the score screens.
- **The stars** (`#star…`) and whatever else its marked objects stand for.
- **How it moves between menus**: the original's buttons slide (their meshes are animated).
- **Pictures** are cut to 128-pixel tiles, at most four a side: the 512-pixel title and
  rings are whole, anything larger would be shrunk.
- **Wide screens**: the original's view is 4:3; ours fills the height and shows what the
  scene has beyond its sides.

## Other platforms

- **Windows and Linux**: their backends (`shim/windows_d3d11.c`, `shim/linux_vulkan.c`) draw
  only the flat picture. They have no lit 3D world, so no model pictures, map wash, decals,
  glows or spice on the ground. Their picture, wash and sound calls do nothing, the
  battlefield's sounds too.
