# Vehicle and aircraft models

What each machine should look like after the rework, before any of it is built. Each model gets
about ten times the triangles it has now, and is mostly bare metal with the owner's color on a
few parts only.

## Throughout

**Metal and paint.** Hulls, frames, wheels, tracks, legs, barrels and engines are bare metal: a
cool grey steel, darker gunmetal for weapons and running gear, and a little bronze on joints and
hinges. The owner's color goes only where a real army would paint markings:

- a stripe or panel on each side;
- the turret roof or top hatch;
- fin tips, wing tips or tail fins on aircraft.

Seen from the battle camera, about a fifth to a third of the visible surface is team-colored, so
units stay easy to tell apart at a glance.

**How it shines.** The shader gets a shine value per vertex:

- **Bare metal** shines hard, with a sharp sun highlight and a hint of the sky in it, so it reads
  as metal even in shadow.
- **Paint** has a soft, wide sheen.
- **Rubber, glass and lamps** don't shine (glass and lamps keep the colors they have now).

All of this is done per pixel on the GPU, like the rest of the lighting.

**Shapes.**

- Boxes get bevelled edges.
- Barrels, wheels, pods, engines and joints become round: 10 to 16 sides on the big parts and 6
  to 8 on small ones.
- Fuselages are built from rings of points along the nose-to-tail line, so they taper instead of
  being boxes.

**What stays the same.** Footprint, height, heading and every animated part keep working as they
do now:

- turret and barrel direction, recoil, wing beats, leg steps, rocket racks emptying, tube pitch;
- where shots leave from (`gun_tip`, `ROCKET_SIDES`, `CANNON_SIDES` and the like), so gunfire,
  flares and rockets still start at the muzzle.

Stains, flames, shadows, headlights and selection rings work on the new models unchanged.

**Budget.** The figures below are roughly triangles per model. A battle with about 400 machines
on screen comes to about 150,000 triangles, drawn twice (once more for shadows). Units off
screen are skipped.

## Ground vehicles

### Tank (now about 30, aim about 350)

A squat main battle tank, wider than tall.

- **Hull:** a bevelled hull with a sloped front glacis and a flat engine deck at the back with two
  grille slats.
- **Tracks:** one track on each side: a long rounded belt with five road wheels showing (8 sides
  each), a drive sprocket at the back and an idler at the front. The track and wheels are dark
  gunmetal.
- **Turret:** an eight-sided turret with sloped sides, sitting a little back of the middle.
- **Gun:** a round barrel (10 sides) with a thicker mantlet where it leaves the turret and a muzzle
  brake ring at the tip.
- **Fittings:** a small commander's hatch, and a stubby antenna at the rear corner.
- **Team color:** the turret roof and a band along each hull side above the tracks.

### Repair Vehicle (now about 30, aim about 300)

A boxy six-wheeled truck, a workshop on wheels.

- **Cab:** a short cab at the front with a dark windscreen.
- **Body:** a taller box workshop behind the cab, its side panels with seams.
- **Wheels:** three round wheels a side (12 sides, dark rubber with steel hubs).
- **Crane:** a small folded crane arm on the roof: two bars and a hook.
- **Beacon:** an amber beacon on the cab.
- **Marking:** the white cross stays on the roof.
- **Team color:** the cab and a stripe along the workshop sides.

### Catapult (now about 30, aim about 300)

A tracked artillery carrier with a big throwing arm.

- **Base:** the same tracked running gear as the tank, slightly shorter, with a low open-topped
  hull.
- **Arm:** a pivot block in the middle carries a long steel throwing arm, built as two parallel
  beams with cross braces, angled back.
- **Bucket:** a round cup at the end of the arm (8-sided bowl).
- **Counterweight:** a counterweight box at the arm's front end.
- **Team color:** the hull sides and the counterweight.

### Rocket Launcher (now about 60, aim about 400)

A six-wheeled truck carrying two rocket racks.

- **Wheels and cab:** six wheels like the repair vehicle, with a low armored cab at the front.
- **Racks:** two racks behind the cab, each an open frame of thin steel bars rather than a solid
  block, so the rockets show inside it. The racks tilt up a little towards the front.
- **Rockets:** each rocket is a white round body (6 sides) with a pointed nose cone and four tiny
  fins. They vanish one by one as the salvo fires, as now (`ROCKET_SIDES`, `ROCKET_HEIGHT`).
- **Team color:** the cab and the rack side plates.

### Strider (now about 60, aim about 450)

A small walking missile carrier, like a bird-legged mech.

- **Legs:** two legs with the same knee animation as now. Each leg is a round thigh and shin
  (8 sides) with:
  - a ball joint at hip and knee (a low sphere);
  - a piston running beside the shin;
  - a flat three-toed foot.
- **Body:** a rounded cockpit body, wider at the front, with a dark visor slit.
- **Pods:** a missile pod on each side: a short round box with a face of four round tube mouths,
  dark inside.
- **Team color:** the cockpit shell's top and the pod side plates.

### Heavy Strider (now about 90, aim about 800)

The same idea twice the size: a walking gun platform.

- **Legs:** like the Strider's but thicker, with two pistons each and armored knee caps.
- **Body:** a broad bevelled body with a raised armored top. It carries a small sensor head at
  the front with two lamp lenses.
- **Cannons:** four round barrels (10 sides) on a front mantlet, each with a recoil sleeve and a
  muzzle ring. They still slide back when they fire (`CANNON_SIDES`, `CANNON_HEIGHT`,
  `CANNON_REACH`).
- **Fittings:** exhaust stacks at the back.
- **Team color:** the raised top armor and the knee caps.

## Aircraft

### Advanced Carryall (now about 50, aim about 450)

A long open lifting frame that hangs in the air.

- **Spine:** a central spine built as a lattice: two long rails with cross ribs.
- **Engines:** an engine pod on each side, a round nacelle (12 sides) with:
  - an intake ring at the front;
  - a darker exhaust at the back;
  - a short pylon tying it to the spine.
- **Underside:** a clamp frame under the spine where the cables and the carried vehicle hang.
- **Lamp:** the big round floodlight under the nose stays, as a glass disc in a steel housing.
- **Team color:** the nacelle front rings and a band across the spine's middle.

### Air Defense Platform (now about 50, aim about 400)

A hovering gun platform on two floats.

- **Floats:** two long floats, each a rounded capsule (10 sides) with a lamp at its front end,
  where the beams come from now.
- **Bridge:** a girder bridge between them.
- **Turret:** a turret on a round ring mount in the middle.
- **Gun:** a twin-barrel automatic gun, two thin round barrels with an ammo box beside them.
- **Team color:** the top of each float and the turret's sides.

### Air Drone (now 4, aim about 120)

A small fast delta-wing drone.

- **Fuselage:** a thin round fuselage from a pointed nose to a round engine nozzle at the tail.
- **Wing:** a delta wing with some thickness (top and bottom skins and a leading edge) instead of
  a flat sheet.
- **Fin:** one swept tail fin.
- **Canopy:** a dark sensor dome on the nose.
- **Exhaust:** the flame comes from the nozzle as now.
- **Team color:** the wing tips and the fin.

### Ornithopter (now about 30, aim about 350)

A slim attack machine with two pairs of flapping wings.

- **Body:** a tapered fuselage built from rings, rounded at the front and narrowing to the tail
  boom.
- **Canopy:** a dark glass canopy over the cockpit.
- **Tail:** a small tail fin and stabilizers at the end of the boom.
- **Wings:** four wings, each a thin skin over three ribs, curved slightly and wider at the root.
  They beat in turn in flight and lie still when it sits, as now.
- **Rails:** a short rocket rail under each side.
- **Team color:** the wing tips, the fin, and a stripe down the spine.

### Destroyer (now about 50, aim about 500)

A heavy armored ornithopter.

- **Body:** a broad, deep fuselage of rings with an armored nose and a dark slit canopy.
- **Tail:** a thick tail boom with twin fins.
- **Wings:** four heavier wings with thicker ribs and a reinforced leading edge. They beat slower,
  as now.
- **Pods:** two big rocket pods (12 sides), each with a face of six round tube mouths.
- **Team color:** the nose armor band, the twin fins and the wing tips.

## Not in this pass

Infantry, towers, buildings and landing pads also get more triangles. They keep their current
colors (infantry and buildings aren't metal machines), and they will get a list like this one
when we get to them.
