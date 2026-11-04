//! Placeholder units and the geometry of selecting them.

use crate::battlefield::{MAP_MIDDLE_Z, RuinPiece, Terrain};
use crate::game::TeamColor;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// About how tall a unit stands, in world units.
pub const HEIGHT: f32 = 0.5;
const SPACING: f32 = 1.2;
/// A unit this close to its destination has reached it.
const ARRIVED: f32 = 0.12;
/// Destinations in one order stand this far apart, enough for any two units side by side.
const SLOT_SPACING: f32 = 1.3;
/// How far from the clicked point to look for free destinations, in slots.
const SLOT_SEARCH_RINGS: i32 = 40;
/// The longest simulation step, in seconds; longer frames are divided up.
const MAX_STEP: f32 = 0.05;
/// A vehicle must point this close to its way, in radians, before it drives.
const DRIVE_ALIGNMENT: f32 = 0.15;
/// How far ahead, beyond touching, a unit starts to steer around another.
const AVOID_MARGIN: f32 = 0.9;
/// How long a unit keeps pressing against water, lava or a ruin without getting any nearer
/// before it gives up, in seconds.
const BLOCKED_PATIENCE: f32 = 3.0;
/// Getting this much nearer counts as progress.
const PROGRESS: f32 = 0.05;
/// Rounds of separating overlapping units per step.
const SEPARATION_ROUNDS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    /// Sets off in any direction at once.
    Infantry,
    /// Must turn to face its way before it drives.
    Vehicle,
}

impl UnitKind {
    /// The radius of the sphere it collides with.
    pub fn radius(self) -> f32 {
        match self {
            UnitKind::Infantry => 0.25,
            UnitKind::Vehicle => 0.45,
        }
    }

    /// World units per second.
    fn speed(self) -> f32 {
        match self {
            UnitKind::Infantry => 2.5,
            UnitKind::Vehicle => 4.5,
        }
    }
}

/// Radians per second a vehicle can turn.
const TURN_SPEED: f32 = 3.0;
/// The player's second group, far enough east to need scrolling to from the first.
const SECOND_GROUP_X: f32 = 35.0;
/// How far from the middle each opponent's group stands.
const OPPONENT_RING: f32 = 29.4;
/// How far map objects must stay from where groups start.
const CLEARANCE: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    pub x: f32,
    /// Height of the ground it stands on.
    pub y: f32,
    pub z: f32,
    /// 0 is the player; opponents count up from 1.
    pub owner: usize,
    pub color: TeamColor,
    pub selected: bool,
    pub kind: UnitKind,
    /// Facing, in radians: 0 points east and a quarter turn points south.
    pub heading: f32,
    /// Where its owner last sent it (x, z). This is never forgotten: being pushed aside
    /// only delays the unit, which then walks back.
    pub destination: (f32, f32),
    /// A heavy unit squashes enemy infantry it drives over. Every vehicle is heavy for now.
    pub heavy: bool,
    pub alive: bool,
    blocked_for: f32,
    /// Its nearest approach to the destination since it set off.
    best_remaining: f32,
}

impl Unit {
    /// Whether it still has somewhere to go.
    pub fn is_moving(&self) -> bool {
        self.alive && self.remaining() > ARRIVED
    }

    fn remaining(&self) -> f32 {
        (self.destination.0 - self.x).hypot(self.destination.1 - self.z)
    }

    /// Its collision sphere's center height.
    fn sphere_y(&self) -> f32 {
        self.y + self.kind.radius()
    }
}

/// Whether two owners fight each other. Everyone is on their own team unless `teams` puts
/// them together, which is how allies will work. An unknown owner fights nobody.
pub fn are_enemies(teams: &[usize], a: usize, b: usize) -> bool {
    match (teams.get(a), teams.get(b)) {
        (Some(team_a), Some(team_b)) => team_a != team_b,
        _ => false,
    }
}

/// Places a block of units centered on a world point. Only the player has vehicles, in the
/// block's last row, so that a skirmish's enemies can all be run over.
fn spawn_group(
    units: &mut Vec<Unit>,
    terrain: &Terrain,
    (cx, cz): (f32, f32),
    (cols, rows): (usize, usize),
    (owner, color): (usize, TeamColor),
) {
    for row in 0..rows {
        for col in 0..cols {
            let x = cx + (col as f32 - (cols - 1) as f32 / 2.0) * SPACING;
            let z = cz + (row as f32 - (rows - 1) as f32 / 2.0) * SPACING;
            let kind = if owner == 0 && row + 1 == rows { UnitKind::Vehicle } else { UnitKind::Infantry };
            units.push(Unit {
                x,
                y: terrain.height_at(x, z),
                z,
                owner,
                color,
                selected: false,
                kind,
                heading: -FRAC_PI_2,
                destination: (x, z),
                heavy: kind == UnitKind::Vehicle,
                alive: true,
                blocked_for: 0.0,
                best_remaining: f32::INFINITY,
            });
        }
    }
}

/// The player gets a block at the map's middle and another a long scroll away. Each opponent
/// gets one block, evenly around a ring. `colors` holds the player's, then the opponents'.
pub fn spawn(terrain: &Terrain, colors: &[TeamColor]) -> Vec<Unit> {
    let mut units = Vec::new();
    let player = (0, colors[0]);
    spawn_group(&mut units, terrain, (0.0, MAP_MIDDLE_Z), (4, 3), player);
    spawn_group(&mut units, terrain, (SECOND_GROUP_X, MAP_MIDDLE_Z), (4, 2), player);
    let opponents = colors.len() - 1;
    for (i, &color) in colors[1..].iter().enumerate() {
        // The ring starts in the north, where it is clear of the player's second group.
        let angle = i as f32 / opponents as f32 * TAU;
        let (sin, cos) = angle.sin_cos();
        let center = (OPPONENT_RING * sin, MAP_MIDDLE_Z - OPPONENT_RING * cos);
        spawn_group(&mut units, terrain, center, (3, 2), (i + 1, color));
    }
    units
}

/// Whether a point lies where a group can start, which must be land even on a planet whose
/// lower terrain is water or lava. Opponents spread around the whole ring.
pub fn needs_land(x: f32, z: f32) -> bool {
    const REACH: f32 = 5.0;
    let from_middle = x.hypot(z - MAP_MIDDLE_Z);
    let from_second_group = (x - SECOND_GROUP_X).hypot(z - MAP_MIDDLE_Z);
    from_middle < REACH || from_second_group < REACH || (from_middle - OPPONENT_RING).abs() < REACH
}

/// Whether a map object may stand here, away from every place a group can start.
pub fn keeps_clear(x: f32, z: f32) -> bool {
    let from_middle = x.hypot(z - MAP_MIDDLE_Z);
    let from_second_group = (x - SECOND_GROUP_X).hypot(z - MAP_MIDDLE_Z);
    from_middle > CLEARANCE
        && from_second_group > CLEARANCE
        && (from_middle - OPPONENT_RING).abs() > CLEARANCE * 0.6
}

/// Up to `count` free spots near a point, nearest first, each far enough from the others
/// and from every point in `taken`. Spots must be walkable, so a click on water finds the
/// nearest land.
fn find_slots(terrain: &Terrain, target: (f32, f32), count: usize, taken: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut offsets = Vec::new();
    for j in -SLOT_SEARCH_RINGS..=SLOT_SEARCH_RINGS {
        for i in -SLOT_SEARCH_RINGS..=SLOT_SEARCH_RINGS {
            offsets.push((i as f32 * SLOT_SPACING, j as f32 * SLOT_SPACING));
        }
    }
    offsets.sort_by(|a, b| a.0.hypot(a.1).total_cmp(&b.0.hypot(b.1)));
    let is_free = |spot: (f32, f32), others: &[(f32, f32)]| {
        others.iter().all(|o| (o.0 - spot.0).hypot(o.1 - spot.1) >= SLOT_SPACING - 0.001)
    };
    let mut slots: Vec<(f32, f32)> = Vec::with_capacity(count);
    for (dx, dz) in offsets {
        if slots.len() == count {
            break;
        }
        let spot = (target.0 + dx, target.1 + dz);
        let clear_of_ruins = terrain.ruins().iter().all(|piece| {
            let (near_x, near_z) = nearest_on_footprint(piece, spot.0, spot.1);
            (near_x - spot.0).hypot(near_z - spot.1) > UnitKind::Vehicle.radius()
        });
        if clear_of_ruins && terrain.can_walk(spot.0, spot.1) && is_free(spot, taken) && is_free(spot, &slots) {
            slots.push(spot);
        }
    }
    slots
}

/// The point of a ruin piece's footprint nearest to a spot on the ground.
fn nearest_on_footprint(piece: &RuinPiece, x: f32, z: f32) -> (f32, f32) {
    (
        x.clamp(piece.x - piece.half_x, piece.x + piece.half_x),
        z.clamp(piece.z - piece.half_z, piece.z + piece.half_z),
    )
}

/// Sends every selected unit to its own reserved spot near a point. Friendly destinations stay
/// reserved. Enemies' only do if nobody heavy was sent: heavy units may aim right at them.
pub fn order_move(units: &mut [Unit], target: (f32, f32), terrain: &Terrain, teams: &[usize]) {
    let any_heavy = units.iter().any(|u| u.selected && u.heavy);
    let taken: Vec<(f32, f32)> = units
        .iter()
        .filter(|u| !u.selected && u.alive && !(any_heavy && are_enemies(teams, 0, u.owner)))
        .map(|u| u.destination)
        .collect();
    let mut chosen: Vec<usize> = (0..units.len()).filter(|&i| units[i].selected).collect();
    let mut slots = find_slots(terrain, target, chosen.len(), &taken);
    // Farthest first, so that those with the longest walk are not left the leftover spots.
    let distance = |unit: &Unit, spot: (f32, f32)| (unit.x - spot.0).hypot(unit.z - spot.1);
    chosen.sort_by(|&a, &b| distance(&units[b], target).total_cmp(&distance(&units[a], target)));
    for i in chosen {
        let nearest = (0..slots.len())
            .min_by(|&a, &b| distance(&units[i], slots[a]).total_cmp(&distance(&units[i], slots[b])));
        let Some(nearest) = nearest else { return };
        units[i].destination = slots.swap_remove(nearest);
        units[i].blocked_for = 0.0;
        units[i].best_remaining = f32::INFINITY;
    }
}

/// Whether a moving unit would squash another by touching it.
fn can_squash(squasher: &Unit, victim: &Unit, teams: &[usize]) -> bool {
    squasher.heavy
        && squasher.is_moving()
        && victim.kind == UnitKind::Infantry
        && are_enemies(teams, squasher.owner, victim.owner)
}

/// Which way to head: straight for the destination, bending right around whatever stands
/// close ahead. Everyone keeps right, so two units meeting head-on pass each other. Heavy
/// units do not bend around infantry they can squash.
fn steering(units: &[Unit], index: usize, terrain: &Terrain, teams: &[usize]) -> (f32, f32) {
    let unit = &units[index];
    let (to_x, to_z) = (unit.destination.0 - unit.x, unit.destination.1 - unit.z);
    let length = to_x.hypot(to_z).max(0.0001);
    let (want_x, want_z) = (to_x / length, to_z / length);
    let (right_x, right_z) = (-want_z, want_x);
    let mut sideways = 0.0;
    for (other_index, other) in units.iter().enumerate() {
        if other_index == index || !other.alive || can_squash(unit, other, teams) {
            continue;
        }
        let (dx, dz) = (other.x - unit.x, other.z - unit.z);
        let distance = dx.hypot(dz);
        let range = unit.kind.radius() + other.kind.radius() + AVOID_MARGIN;
        if distance < range && dx * want_x + dz * want_z > 0.0 {
            sideways += (1.0 - distance / range) * 2.0;
        }
    }
    for piece in terrain.ruins() {
        let (near_x, near_z) = nearest_on_footprint(piece, unit.x, unit.z);
        let (dx, dz) = (near_x - unit.x, near_z - unit.z);
        let distance = dx.hypot(dz);
        let range = unit.kind.radius() + AVOID_MARGIN;
        if distance < range && dx * want_x + dz * want_z > 0.0 {
            sideways += (1.0 - distance / range) * 2.0;
        }
    }
    let (x, z) = (want_x + right_x * sideways, want_z + right_z * sideways);
    let length = x.hypot(z).max(0.0001);
    (x / length, z / length)
}

/// The shortest signed turn from one heading to another.
fn turn_between(from: f32, to: f32) -> f32 {
    (to - from + PI).rem_euclid(TAU) - PI
}

/// Moves a unit along its way, sliding along water or lava it cannot enter. Returns whether
/// those got in its way.
fn walk(unit: &mut Unit, terrain: &Terrain, (dir_x, dir_z): (f32, f32), dt: f32) -> bool {
    let distance = (unit.kind.speed() * dt).min(unit.remaining());
    let (dx, dz) = (dir_x * distance, dir_z * distance);
    let tries = [(dx, dz), (dx, 0.0), (0.0, dz)];
    match tries.iter().find(|(tx, tz)| terrain.can_walk(unit.x + tx, unit.z + tz)) {
        Some(&(tx, tz)) => {
            unit.x += tx;
            unit.z += tz;
            tx != dx || tz != dz
        }
        None => true,
    }
}

/// Pushes a unit's sphere out of any ruin it reaches. Returns whether it touched one.
fn push_out_of_ruins(unit: &mut Unit, terrain: &Terrain) -> bool {
    let radius = unit.kind.radius();
    let mut touched = false;
    for piece in terrain.ruins() {
        let (near_x, near_z) = nearest_on_footprint(piece, unit.x, unit.z);
        let dy = unit.sphere_y() - unit.sphere_y().clamp(piece.y, piece.y + piece.height);
        let (dx, dz) = (unit.x - near_x, unit.z - near_z);
        let flat = dx.hypot(dz);
        if flat.hypot(dy) >= radius {
            continue;
        }
        touched = true;
        // The flat distance at which the sphere just clears this piece.
        let clear = (radius * radius - dy * dy).max(0.0).sqrt();
        let (push_x, push_z) = if flat > 0.0001 {
            (dx / flat * (clear - flat), dz / flat * (clear - flat))
        } else {
            // Its center is over the footprint: leave by the nearest side.
            let (to_x, to_z) = (piece.half_x - (unit.x - piece.x).abs(), piece.half_z - (unit.z - piece.z).abs());
            if to_x < to_z {
                ((to_x + clear) * (unit.x - piece.x).signum(), 0.0)
            } else {
                (0.0, (to_z + clear) * (unit.z - piece.z).signum())
            }
        };
        if terrain.can_walk(unit.x + push_x, unit.z + push_z) {
            unit.x += push_x;
            unit.z += push_z;
        }
    }
    touched
}

/// Heavy units on the move kill enemy infantry whose spheres they touch.
fn squash(units: &mut [Unit], teams: &[usize]) {
    for a in 0..units.len() {
        for b in 0..units.len() {
            if a == b || !units[a].alive || !units[b].alive || !can_squash(&units[a], &units[b], teams) {
                continue;
            }
            let (dx, dz) = (units[b].x - units[a].x, units[b].z - units[a].z);
            let dy = units[b].sphere_y() - units[a].sphere_y();
            if dx.hypot(dz).hypot(dy) < units[a].kind.radius() + units[b].kind.radius() {
                units[b].alive = false;
                units[b].selected = false;
            }
        }
    }
}

/// Pushes overlapping spheres apart, sharing the push evenly between equals.
fn separate(units: &mut [Unit], terrain: &Terrain) {
    for _ in 0..SEPARATION_ROUNDS {
        for a in 0..units.len() {
            for b in a + 1..units.len() {
                if !units[a].alive || !units[b].alive {
                    continue;
                }
                let (dx, dz) = (units[b].x - units[a].x, units[b].z - units[a].z);
                let dy = units[b].sphere_y() - units[a].sphere_y();
                let touching = units[a].kind.radius() + units[b].kind.radius();
                let flat = dx.hypot(dz);
                if flat.hypot(dy) >= touching {
                    continue;
                }
                // Exactly on top of each other, any fixed direction will do.
                let (nx, nz) = if flat > 0.0001 { (dx / flat, dz / flat) } else { (1.0, 0.0) };
                let overlap = touching - flat.hypot(dy);
                // Infantry cannot shove something heavy: it takes the whole push itself.
                let (share_a, share_b) = match (units[a].heavy, units[b].heavy) {
                    (true, false) => (0.0, 1.0),
                    (false, true) => (1.0, 0.0),
                    _ => (0.5, 0.5),
                };
                for (i, sign, share) in [(a, -1.0, share_a), (b, 1.0, share_b)] {
                    let push = overlap * share;
                    let (x, z) = (units[i].x + nx * push * sign, units[i].z + nz * push * sign);
                    if terrain.can_walk(x, z) {
                        units[i].x = x;
                        units[i].z = z;
                    }
                }
            }
        }
    }
}

fn step(units: &mut [Unit], terrain: &Terrain, teams: &[usize], dt: f32) {
    let mut held_back = vec![false; units.len()];
    for i in 0..units.len() {
        if !units[i].is_moving() {
            continue;
        }
        let (dir_x, dir_z) = steering(units, i, terrain, teams);
        let way = dir_z.atan2(dir_x);
        let unit = &mut units[i];
        match unit.kind {
            UnitKind::Infantry => {
                unit.heading = way;
                held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
            }
            UnitKind::Vehicle => {
                let turn = turn_between(unit.heading, way);
                let most = TURN_SPEED * dt;
                unit.heading += turn.clamp(-most, most);
                if turn.abs() <= DRIVE_ALIGNMENT {
                    held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
                }
            }
        }
    }
    squash(units, teams);
    separate(units, terrain);
    for (i, unit) in units.iter_mut().enumerate() {
        if !unit.alive {
            continue;
        }
        held_back[i] |= push_out_of_ruins(unit, terrain);
        unit.y = terrain.height_at(unit.x, unit.z);
        give_up_if_stuck(unit, held_back[i], dt);
    }
}

/// Crowds only delay a unit, so its order stands. If water, lava or a ruin keeps it from
/// getting any nearer, there is no way through without path finding: it settles for here.
fn give_up_if_stuck(unit: &mut Unit, held_back: bool, dt: f32) {
    if !unit.is_moving() {
        unit.blocked_for = 0.0;
        unit.best_remaining = f32::INFINITY;
        return;
    }
    if unit.remaining() < unit.best_remaining - PROGRESS {
        unit.best_remaining = unit.remaining();
        unit.blocked_for = 0.0;
    } else if held_back {
        unit.blocked_for += dt;
        if unit.blocked_for > BLOCKED_PATIENCE {
            unit.destination = (unit.x, unit.z);
            unit.blocked_for = 0.0;
        }
    }
}

/// Advances every unit by `dt` seconds. Units killed meanwhile are left marked as not alive.
pub fn advance(units: &mut [Unit], terrain: &Terrain, teams: &[usize], dt: f32) {
    // Spheres only start to overlap when something moves.
    if !units.iter().any(Unit::is_moving) {
        return;
    }
    let steps = (dt / MAX_STEP).ceil().max(1.0);
    for _ in 0..steps as usize {
        step(units, terrain, teams, dt / steps);
    }
}

/// Whether a unit stands inside a rectangle on the ground: (west, north, east, south).
pub fn in_selection((west, north, east, south): (f32, f32, f32, f32), unit: &Unit) -> bool {
    unit.x >= west && unit.x <= east && unit.z >= north && unit.z <= south
}

fn scramble(mut value: u32) -> u32 {
    value = value.wrapping_mul(0x9E37_79B1) ^ 0x5BD1_E995;
    value ^= value >> 15;
    value = value.wrapping_mul(0x85EB_CA77);
    value ^ (value >> 13)
}

/// A jagged line from `a` to `b`, the same for the same seed. Its ends stay exactly on `a`
/// and `b`, and its points stray at most `reach` from the straight line between them.
pub fn lightning_points(
    a: (f32, f32),
    b: (f32, f32),
    seed: u32,
    spacing: f32,
    reach: f32,
) -> Vec<(f32, f32)> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = dx.hypot(dy);
    let steps = ((length / spacing) as usize).max(1);
    let (nx, ny) = if length > 0.0 { (-dy / length, dx / length) } else { (0.0, 0.0) };
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let stray = if i == 0 || i == steps {
                0.0
            } else {
                let hash = scramble(seed.wrapping_add(i as u32 * 7919));
                ((hash & 0xFFFF) as f32 / 65535.0 * 2.0 - 1.0) * reach
            };
            (a.0 + dx * t + nx * stray, a.1 + dy * t + ny * stray)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battlefield::{Camera, View, terrain};

    const COLORS: [TeamColor; 4] = [TeamColor::Blue, TeamColor::Red, TeamColor::Cyan, TeamColor::Green];

    fn unit_at(x: f32, z: f32, kind: UnitKind) -> Unit {
        Unit {
            x,
            y: 0.0,
            z,
            owner: 0,
            color: TeamColor::Blue,
            selected: false,
            kind,
            heading: -FRAC_PI_2,
            destination: (x, z),
            heavy: kind == UnitKind::Vehicle,
            alive: true,
            blocked_for: 0.0,
            best_remaining: f32::INFINITY,
        }
    }

    fn enemy_at(x: f32, z: f32, kind: UnitKind) -> Unit {
        Unit { owner: 1, color: TeamColor::Red, ..unit_at(x, z, kind) }
    }

    const TWO_SIDES: [usize; 2] = [0, 1];

    fn wall(x: f32, z: f32, half_x: f32, half_z: f32, height: f32) -> RuinPiece {
        RuinPiece { x, y: 0.0, z, half_x, half_z, height, shade: 1.0 }
    }

    #[test]
    fn a_moving_heavy_vehicle_squashes_enemy_infantry_in_its_way() {
        let mut units = [unit_at(-3.0, 60.0, UnitKind::Vehicle), enemy_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].heading = 0.0;
        units[0].destination = (3.0, 60.0);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(!units[1].alive, "run over");
        assert!(units[0].alive);
        assert!((units[0].x - 3.0).abs() < 0.2, "and it drove on straight: {:?}", units[0]);
        assert!((units[0].z - 60.0).abs() < 0.2, "it did not swerve: {:?}", units[0]);
    }

    #[test]
    fn only_heavy_moving_units_squash_and_only_enemy_infantry_dies() {
        let run_into = |driver: Unit, target: Unit, teams: &[usize]| {
            let mut units = [driver, target];
            units[0].heading = 0.0;
            units[0].destination = (3.0, 60.0);
            for _ in 0..100 {
                advance(&mut units, open_ground(), teams, 0.05);
            }
            units[1].alive
        };
        let vehicle = unit_at(-3.0, 60.0, UnitKind::Vehicle);
        let infantry = unit_at(-3.0, 60.0, UnitKind::Infantry);
        let light = Unit { heavy: false, ..vehicle };
        let target = enemy_at(0.0, 60.0, UnitKind::Infantry);

        assert!(run_into(infantry, target, &TWO_SIDES), "infantry cannot squash");
        assert!(run_into(light, target, &TWO_SIDES), "nor a vehicle that is not heavy");
        assert!(run_into(vehicle, enemy_at(0.0, 60.0, UnitKind::Vehicle), &TWO_SIDES), "nor crush vehicles");
        assert!(run_into(vehicle, unit_at(0.0, 60.0, UnitKind::Infantry), &TWO_SIDES), "own infantry lives");
        assert!(run_into(vehicle, target, &[0, 0]), "and so do allies");
    }

    #[test]
    fn a_vehicle_standing_still_squashes_nobody() {
        let mut units = [unit_at(0.0, 60.0, UnitKind::Vehicle), enemy_at(-3.0, 60.0, UnitKind::Infantry)];
        units[1].destination = (0.0, 60.0);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(units[1].alive);
    }

    #[test]
    fn enemies_are_told_apart_by_team() {
        assert!(are_enemies(&[0, 1, 2], 0, 1));
        assert!(!are_enemies(&[0, 1, 0], 0, 2), "the same team is allied");
        assert!(!are_enemies(&[0, 1], 1, 1));
        assert!(!are_enemies(&[], 0, 1), "no teams, no fights");
    }

    #[test]
    fn opponents_have_only_infantry_and_the_player_has_vehicles_that_are_heavy() {
        let units = spawn(terrain(0), &COLORS);
        assert!(units.iter().filter(|u| u.owner != 0).all(|u| u.kind == UnitKind::Infantry));
        assert!(units.iter().any(|u| u.owner == 0 && u.kind == UnitKind::Vehicle));
        assert!(units.iter().all(|u| u.heavy == (u.kind == UnitKind::Vehicle)));
    }

    #[test]
    fn a_unit_walks_around_a_ruin_in_its_way_without_entering_it() {
        for kind in [UnitKind::Infantry, UnitKind::Vehicle] {
            let terrain = Terrain::open_with_ruins(vec![wall(0.0, 60.0, 0.5, 0.5, 1.0)]);
            let mut units = [unit_at(-5.0, 60.0, kind)];
            units[0].heading = 0.0;
            units[0].destination = (5.0, 60.0);
            for _ in 0..400 {
                advance(&mut units, &terrain, &[0], 0.05);
                let (near_x, near_z) = nearest_on_footprint(&terrain.ruins()[0], units[0].x, units[0].z);
                let gap = (near_x - units[0].x).hypot(near_z - units[0].z);
                assert!(gap > kind.radius() * 0.5, "{kind:?} inside the ruin: {:?}", units[0]);
            }
            assert!(!units[0].is_moving(), "{kind:?} got there: {:?}", units[0]);
            assert_eq!(units[0].destination, (5.0, 60.0), "{kind:?} kept its order");
        }
    }

    #[test]
    fn a_sphere_clears_rubble_lower_than_it_reaches() {
        // Its center rides higher than this rubble, so the sphere only touches when close.
        let low = wall(0.0, 60.0, 0.05, 0.05, 0.02);
        let terrain = Terrain::open_with_ruins(vec![low]);
        let mut unit = unit_at(0.4, 60.0, UnitKind::Vehicle);
        let height_gap = unit.sphere_y() - 0.02;
        let reach = (0.45_f32 * 0.45 - height_gap * height_gap).max(0.0).sqrt();
        assert!(reach < 0.45, "the sphere is narrower that low down");
        let before = unit.x;
        let touched = push_out_of_ruins(&mut unit, &terrain);
        assert_eq!(touched, 0.35 < reach);
        assert_eq!(unit.x, before, "too far to touch at this distance");
    }

    #[test]
    fn a_unit_boxed_in_by_ruins_gives_up_instead_of_pressing_forever() {
        let terrain = Terrain::open_with_ruins(vec![
            wall(0.0, 56.0, 4.0, 0.2, 2.0),
            wall(0.0, 64.0, 4.0, 0.2, 2.0),
            wall(-4.0, 60.0, 0.2, 4.0, 2.0),
            wall(4.0, 60.0, 0.2, 4.0, 2.0),
        ]);
        let mut units = [unit_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].destination = (12.0, 60.0);
        for _ in 0..1200 {
            advance(&mut units, &terrain, &[0], 0.05);
            assert!(units[0].x < 4.0 && units[0].x > -4.0, "stays inside: {:?}", units[0]);
        }
        assert!(!units[0].is_moving(), "gave up: {:?}", units[0]);
    }

    #[test]
    fn orders_do_not_send_anyone_into_a_ruin() {
        let terrain = Terrain::open_with_ruins(vec![wall(0.0, 60.0, 1.0, 1.0, 1.0)]);
        let mut units = [unit_at(-8.0, 60.0, UnitKind::Vehicle)];
        units[0].selected = true;
        order_move(&mut units, (0.0, 60.0), &terrain, &[0]);
        let (near_x, near_z) = nearest_on_footprint(&terrain.ruins()[0], units[0].destination.0, units[0].destination.1);
        let gap = (near_x - units[0].destination.0).hypot(near_z - units[0].destination.1);
        assert!(gap > UnitKind::Vehicle.radius(), "{:?}", units[0].destination);
    }

    #[test]
    fn heavy_units_may_be_ordered_right_onto_an_enemy() {
        let mut units = [unit_at(-8.0, 60.0, UnitKind::Vehicle), enemy_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].selected = true;
        order_move(&mut units, (0.0, 60.0), open_ground(), &TWO_SIDES);
        assert_eq!(units[0].destination, (0.0, 60.0));

        let mut units = [unit_at(-8.0, 60.0, UnitKind::Infantry), enemy_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].selected = true;
        order_move(&mut units, (0.0, 60.0), open_ground(), &TWO_SIDES);
        assert_ne!(units[0].destination, (0.0, 60.0), "infantry keeps off it");
    }

    /// Flat sand with nothing on it.
    fn open_ground() -> &'static Terrain {
        static OPEN: std::sync::OnceLock<Terrain> = std::sync::OnceLock::new();
        OPEN.get_or_init(|| Terrain::open_with_ruins(Vec::new()))
    }

    #[test]
    fn infantry_cannot_shove_a_heavy_vehicle() {
        let mut units = [unit_at(0.0, 60.0, UnitKind::Vehicle), unit_at(-3.0, 60.0, UnitKind::Infantry)];
        units[1].destination = (3.0, 60.0);
        for _ in 0..200 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert_eq!((units[0].x, units[0].z), (0.0, 60.0), "the vehicle stays put");
        assert!(!units[1].is_moving(), "the infantry walked around it: {:?}", units[1]);
    }

    #[test]
    fn real_maps_let_units_cross_them_among_the_ruins() {
        let map = terrain(0);
        let mut units = [unit_at(-30.0, 40.0, UnitKind::Infantry), unit_at(-30.0, 90.0, UnitKind::Vehicle)];
        units[0].destination = (30.0, 40.0);
        units[1].destination = (30.0, 90.0);
        for _ in 0..1000 {
            advance(&mut units, map, &[0], 0.05);
            for unit in &units {
                for piece in map.ruins() {
                    let (near_x, near_z) = nearest_on_footprint(piece, unit.x, unit.z);
                    let dy = unit.sphere_y() - unit.sphere_y().clamp(piece.y, piece.y + piece.height);
                    let gap = (near_x - unit.x).hypot(near_z - unit.z).hypot(dy);
                    assert!(gap > unit.kind.radius() * 0.5, "{unit:?} inside {piece:?}");
                }
            }
        }
        for unit in &units {
            assert!(!unit.is_moving(), "settled, one way or another: {unit:?}");
        }
    }

    fn run(units: &mut [Unit], seconds: f32) {
        for _ in 0..(seconds / 0.05) as usize {
            advance(units, open_ground(), &[], 0.05);
        }
    }

    fn largest_overlap(units: &[Unit]) -> f32 {
        let mut largest = 0.0_f32;
        for (i, a) in units.iter().enumerate() {
            for b in &units[i + 1..] {
                let distance = (a.x - b.x).hypot(a.z - b.z).hypot(a.sphere_y() - b.sphere_y());
                largest = largest.max(a.kind.radius() + b.kind.radius() - distance);
            }
        }
        largest
    }

    #[test]
    fn infantry_sets_off_at_once_in_any_direction() {
        for destination in [(5.0, 60.0), (-5.0, 60.0), (0.0, 55.0), (0.0, 65.0)] {
            let mut units = [unit_at(0.0, 60.0, UnitKind::Infantry)];
            units[0].destination = destination;
            advance(&mut units, open_ground(), &[], 0.05);
            let moved = (units[0].x - 0.0).hypot(units[0].z - 60.0);
            assert!((moved - 2.5 * 0.05).abs() < 0.001, "{destination:?}: moved {moved}");
        }
    }

    #[test]
    fn a_vehicle_turns_first_and_only_then_drives() {
        // Facing north, sent south: a half turn away.
        let mut units = [unit_at(0.0, 60.0, UnitKind::Vehicle)];
        units[0].destination = (0.0, 66.0);
        let start_heading = units[0].heading;
        advance(&mut units, open_ground(), &[], 0.2);
        assert_eq!((units[0].x, units[0].z), (0.0, 60.0), "still in place");
        assert_ne!(units[0].heading, start_heading, "but turning");

        run(&mut units, 1.5);
        assert!(units[0].z > 60.0, "drives once it faces the way");
        assert!(units[0].heading.sin() > 0.9, "facing south: {}", units[0].heading);
    }

    #[test]
    fn a_vehicle_facing_its_way_needs_no_turn() {
        let mut units = [unit_at(0.0, 60.0, UnitKind::Vehicle)];
        units[0].destination = (0.0, 50.0);
        advance(&mut units, open_ground(), &[], 0.05);
        assert!(units[0].z < 60.0);
    }

    #[test]
    fn vehicles_turn_the_short_way() {
        assert!((turn_between(3.0, -3.0) - (TAU - 6.0)).abs() < 0.001);
        assert!((turn_between(-3.0, 3.0) + (TAU - 6.0)).abs() < 0.001);
        assert!(turn_between(1.0, 1.0).abs() < 0.001);
    }

    #[test]
    fn units_arrive_and_stop() {
        for kind in [UnitKind::Infantry, UnitKind::Vehicle] {
            let mut units = [unit_at(0.0, 60.0, kind)];
            units[0].destination = (6.0, 62.0);
            run(&mut units, 8.0);
            assert!(!units[0].is_moving(), "{kind:?} arrived");
            let settled = (units[0].x, units[0].z);
            run(&mut units, 1.0);
            assert_eq!((units[0].x, units[0].z), settled, "{kind:?} stays");
        }
    }

    #[test]
    fn units_follow_the_terrain_height_as_they_walk() {
        let mut units = [unit_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].destination = (20.0, 45.0);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &[], 0.05);
            assert_eq!(units[0].y, open_ground().height_at(units[0].x, units[0].z));
        }
    }

    #[test]
    fn an_order_reserves_separate_spots_near_the_click() {
        let mut units: Vec<Unit> = (0..12)
            .map(|i| {
                let mut unit = unit_at(i as f32 * 1.2 - 6.0, 60.0, UnitKind::Infantry);
                unit.selected = true;
                unit
            })
            .collect();
        order_move(&mut units, (0.0, 50.0), open_ground(), &[]);
        for (i, a) in units.iter().enumerate() {
            let from_click = (a.destination.0 - 0.0).hypot(a.destination.1 - 50.0);
            assert!(from_click < SLOT_SPACING * 3.0, "near the click: {from_click}");
            for b in &units[i + 1..] {
                let apart = (a.destination.0 - b.destination.0).hypot(a.destination.1 - b.destination.1);
                assert!(apart >= SLOT_SPACING - 0.01, "too close: {apart}");
            }
        }
    }

    #[test]
    fn an_order_keeps_off_spots_others_have_reserved() {
        let mut units = vec![unit_at(0.0, 60.0, UnitKind::Infantry), unit_at(2.0, 60.0, UnitKind::Infantry)];
        units[0].destination = (0.0, 50.0);
        units[1].selected = true;
        order_move(&mut units, (0.0, 50.0), open_ground(), &[]);
        assert_ne!(units[1].destination, units[0].destination);
        let apart = (units[1].destination.0).hypot(units[1].destination.1 - 50.0);
        assert!(apart >= SLOT_SPACING - 0.01);
        assert_eq!(units[0].destination, (0.0, 50.0), "unselected units keep their orders");
    }

    #[test]
    fn a_group_ordered_again_may_reuse_its_own_spots() {
        let mut units = vec![unit_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].selected = true;
        order_move(&mut units, (0.0, 50.0), open_ground(), &[]);
        let first = units[0].destination;
        order_move(&mut units, (0.0, 50.0), open_ground(), &[]);
        assert_eq!(units[0].destination, first);
    }

    #[test]
    fn head_on_units_pass_each_other_and_both_arrive() {
        for kind in [UnitKind::Infantry, UnitKind::Vehicle] {
            let mut units = [unit_at(-5.0, 60.0, kind), unit_at(5.0, 60.0, kind)];
            units[0].destination = (5.0, 60.0);
            units[1].destination = (-5.0, 60.0);
            for _ in 0..400 {
                advance(&mut units, open_ground(), &[], 0.05);
                assert!(largest_overlap(&units) < 0.05, "{kind:?} overlap");
            }
            assert!(!units[0].is_moving() && !units[1].is_moving(), "{kind:?}: {units:?}");
        }
    }

    #[test]
    fn a_large_crowd_crossing_another_never_overlaps_and_everyone_arrives() {
        let mut units = Vec::new();
        for i in 0..10 {
            for (side, kind) in [(-1.0, UnitKind::Infantry), (1.0, UnitKind::Vehicle)] {
                let mut unit = unit_at(side * 8.0, 55.0 + i as f32 * 1.3, kind);
                unit.destination = (-side * 8.0, 55.0 + i as f32 * 1.3);
                units.push(unit);
            }
        }
        for _ in 0..1600 {
            advance(&mut units, open_ground(), &[], 0.05);
            assert!(largest_overlap(&units) < 0.1, "overlap {}", largest_overlap(&units));
        }
        let stuck: Vec<_> = units.iter().filter(|u| u.is_moving()).collect();
        assert!(stuck.is_empty(), "still going: {stuck:?}");
    }

    #[test]
    fn a_unit_pushed_aside_remembers_where_it_was_sent_and_walks_back() {
        // The second starts touching the first and sets off through it, so it must shove it.
        let mut units = [unit_at(0.0, 60.0, UnitKind::Infantry), unit_at(-0.3, 60.0, UnitKind::Vehicle)];
        let home = units[0].destination;
        units[1].heading = 0.0;
        units[1].destination = (6.0, 60.0);
        let mut displaced = 0.0_f32;
        for _ in 0..400 {
            advance(&mut units, open_ground(), &[], 0.05);
            displaced = displaced.max((units[0].x - home.0).hypot(units[0].z - home.1));
        }
        assert!(displaced > 0.05, "it really was pushed: {displaced}");
        assert_eq!(units[0].destination, home, "the order is not forgotten");
        assert!(!units[0].is_moving(), "back where it was sent: {:?}", units[0]);
        assert!(!units[1].is_moving(), "and the vehicle got past: {:?}", units[1]);
    }

    #[test]
    fn a_blocked_unit_presses_on_for_a_while_without_losing_its_order() {
        // A wall of waiting units across its way only delays it.
        let mut units = vec![unit_at(0.0, 65.0, UnitKind::Infantry)];
        units[0].destination = (0.0, 55.0);
        for i in -4..=4 {
            units.push(unit_at(i as f32 * 0.6, 60.0, UnitKind::Infantry));
        }
        let sent_to = units[0].destination;
        run(&mut units, 2.0);
        assert_eq!(units[0].destination, sent_to);
    }

    fn map_with_impassable_lower_terrain() -> &'static Terrain {
        let index = crate::battlefield::MAP_PLANETS
            .iter()
            .position(|p| *p == crate::battlefield::Planet::C)
            .unwrap();
        terrain(index)
    }

    #[test]
    fn orders_on_water_go_to_the_nearest_land() {
        let terrain = map_with_impassable_lower_terrain();
        // Find a spot of open water.
        let mut water = None;
        'search: for x in -40..=40 {
            for z in 30..=100 {
                if !terrain.can_walk(x as f32, z as f32) && (x as f32).abs() + (z as f32 - 64.0).abs() < 50.0 {
                    water = Some((x as f32, z as f32));
                    break 'search;
                }
            }
        }
        let water = water.expect("this map has water");
        let mut unit = unit_at(0.0, 64.0, UnitKind::Infantry);
        unit.selected = true;
        let mut units = [unit];
        order_move(&mut units, water, terrain, &[]);
        assert!(terrain.can_walk(units[0].destination.0, units[0].destination.1));
    }

    #[test]
    fn units_never_enter_water_and_give_up_when_it_blocks_them() {
        let terrain = map_with_impassable_lower_terrain();
        let mut units = [unit_at(0.0, 64.0, UnitKind::Infantry)];
        // Straight across whatever lies to the far north, past the land ring.
        units[0].destination = (0.0, 2.0);
        for _ in 0..2000 {
            advance(&mut units, terrain, &[], 0.05);
            assert!(terrain.can_walk(units[0].x, units[0].z), "{:?}", units[0]);
        }
        assert!(!units[0].is_moving(), "it does not press on forever: {:?}", units[0]);
    }

    #[test]
    fn everyone_gets_units_in_their_own_color_on_the_map() {
        let units = spawn(terrain(0), &COLORS);
        for owner in 0..COLORS.len() {
            let own: Vec<_> = units.iter().filter(|u| u.owner == owner).collect();
            assert!(!own.is_empty());
            assert!(own.iter().all(|u| u.color == COLORS[owner]));
        }
        for unit in &units {
            let reach = unit.x.abs() + (unit.z - MAP_MIDDLE_Z).abs();
            assert!(reach < MAP_MIDDLE_Z, "off the diamond: {unit:?}");
        }
    }

    #[test]
    fn units_do_not_overlap_with_seven_opponents() {
        let mut colors = vec![TeamColor::Blue];
        colors.extend([
            TeamColor::Red,
            TeamColor::Cyan,
            TeamColor::Orange,
            TeamColor::Green,
            TeamColor::Violet,
            TeamColor::Yellow,
            TeamColor::LightPink,
        ]);
        let units = spawn(terrain(1), &colors);
        for (i, a) in units.iter().enumerate() {
            for b in &units[i + 1..] {
                assert!((a.x - b.x).hypot(a.z - b.z) > a.kind.radius() + b.kind.radius(), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn no_unit_starts_inside_a_ruin_on_any_map() {
        let colors = [
            TeamColor::Blue,
            TeamColor::Red,
            TeamColor::Cyan,
            TeamColor::Orange,
            TeamColor::Green,
            TeamColor::Violet,
            TeamColor::Yellow,
            TeamColor::LightPink,
        ];
        for map in 0..crate::game::MAPS.len() {
            for count in 2..=colors.len() {
                let terrain = terrain(map);
                for unit in spawn(terrain, &colors[..count]) {
                    for piece in terrain.ruins() {
                        let gap_x = (unit.x - piece.x).abs() - unit.kind.radius() - piece.half_x;
                        let gap_z = (unit.z - piece.z).abs() - unit.kind.radius() - piece.half_z;
                        assert!(gap_x > 0.0 || gap_z > 0.0, "map {map}: {unit:?} in {piece:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn units_stand_on_the_terrain() {
        let terrain = terrain(0);
        for unit in spawn(terrain, &COLORS) {
            assert_eq!(unit.y, terrain.height_at(unit.x, unit.z));
        }
    }

    #[test]
    fn the_players_groups_are_more_than_a_screen_apart() {
        let view = View::new(Camera::centered(), 2880.0, 1800.0);
        let units = spawn(terrain(0), &COLORS);
        let far = units.iter().find(|u| u.x >= SECOND_GROUP_X - 3.0).unwrap();
        let (x, _) = view.project(far.x, far.y, far.z).unwrap();
        assert!(x > 2880.0, "starts off screen: {x}");
    }

    #[test]
    fn selection_is_tested_on_the_ground_including_its_borders() {
        let rectangle = (-3.0, 50.0, 8.0, 70.0);
        let unit = unit_at(0.0, 60.0, UnitKind::Infantry);
        assert!(in_selection(rectangle, &unit));
        assert!(in_selection(rectangle, &Unit { x: 8.0, z: 50.0, ..unit }), "on the border");
        assert!(!in_selection(rectangle, &Unit { x: -3.1, ..unit }), "west of it");
        assert!(!in_selection(rectangle, &Unit { x: 8.1, ..unit }), "east of it");
        assert!(!in_selection(rectangle, &Unit { z: 49.9, ..unit }), "north of it");
        assert!(!in_selection(rectangle, &Unit { z: 70.1, ..unit }), "south of it");
    }

    #[test]
    fn a_units_height_does_not_change_whether_it_is_selected() {
        let rectangle = (-3.0, 50.0, 8.0, 70.0);
        let low = unit_at(0.0, 60.0, UnitKind::Infantry);
        assert_eq!(in_selection(rectangle, &low), in_selection(rectangle, &Unit { y: 0.7, ..low }));
    }

    #[test]
    fn lightning_starts_and_ends_exactly_on_the_edge_and_stays_near_it() {
        let (a, b) = ((100.0, 200.0), (700.0, 200.0));
        let points = lightning_points(a, b, 42, 30.0, 9.0);
        assert_eq!(points[0], a);
        assert_eq!(*points.last().unwrap(), b);
        assert!(points.len() > 10);
        assert!(points.iter().all(|p| (p.1 - 200.0).abs() <= 9.0));
        assert!(points.iter().any(|p| (p.1 - 200.0).abs() > 1.0), "it is jagged");
    }

    #[test]
    fn lightning_flickers_with_the_seed_but_is_stable_for_one_seed() {
        let bolt = |seed| lightning_points((0.0, 0.0), (500.0, 500.0), seed, 30.0, 9.0);
        assert_eq!(bolt(1), bolt(1));
        assert_ne!(bolt(1), bolt(2));
    }

    #[test]
    fn a_zero_length_bolt_is_harmless() {
        let points = lightning_points((5.0, 5.0), (5.0, 5.0), 1, 30.0, 9.0);
        assert_eq!(points, [(5.0, 5.0), (5.0, 5.0)]);
    }
}
