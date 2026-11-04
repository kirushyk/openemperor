//! Placeholder units, buildings among them, and the geometry of selecting them.

use crate::battlefield::{MAP_MIDDLE_Z, RuinPiece, Terrain};
use crate::game::TeamColor;
use crate::weapons::WeaponKind;
use std::cmp::Ordering;
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

/// A point in the world: x east, y up, z south.
pub type Point = (f32, f32, f32);

pub fn distance(a: Point, b: Point) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1).hypot(a.2 - b.2)
}

/// How to divide a frame of `dt` seconds: the number of steps and how long each one is.
pub fn substeps(dt: f32) -> (usize, f32) {
    let steps = (dt / MAX_STEP).ceil().max(1.0);
    (steps as usize, dt / steps)
}
/// A vehicle must point this close to its way, in radians, before it drives.
const DRIVE_ALIGNMENT: f32 = 0.15;
/// How far ahead, beyond touching, a unit starts to steer around another.
const AVOID_MARGIN: f32 = 0.9;
/// How long a unit keeps pressing against water, lava or a ruin without getting any nearer
/// before it gives up, in seconds.
const BLOCKED_PATIENCE: f32 = 3.0;
/// Getting this much nearer counts as progress.
const PROGRESS: f32 = 0.05;
/// How far a heavy unit will swerve to run down enemy infantry.
const SQUASH_LURE: f32 = 4.0;
/// How nearly ahead that infantry must be, as the cosine of the angle off its way: it will
/// not turn back for anyone behind it.
const SQUASH_LURE_AHEAD: f32 = 0.3;
/// A unit told to attack closes to this share of its weapon's range, so that a target
/// shifting about a little does not keep it starting and stopping.
const ATTACK_REACH: f32 = 0.9;
/// An engineer this far beyond touching a building takes it over.
const CAPTURE_MARGIN: f32 = 0.3;
/// A unit after a target that covers less than this share of its usual step is wedged.
const WEDGED_SHARE: f32 = 0.25;
/// Rounds of separating overlapping units per step.
const SEPARATION_ROUNDS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitKind {
    /// Sets off in any direction at once.
    Infantry,
    /// Must turn to face its way before it drives.
    Vehicle,
    /// Stays in the air, passing over water, lava, ruins and anyone on the ground.
    Aircraft,
    /// A building: it never moves and takes no orders, and turns its two tubes by itself.
    Tower,
}

impl UnitKind {
    /// The radius of the sphere it collides with.
    pub fn radius(self) -> f32 {
        match self {
            UnitKind::Infantry => 0.25,
            UnitKind::Vehicle => 0.45,
            UnitKind::Aircraft => 0.4,
            UnitKind::Tower => 0.6,
        }
    }

    pub fn is_building(self) -> bool {
        self == UnitKind::Tower
    }

    /// The experience its killer gets.
    pub fn kill_value(self) -> u32 {
        match self {
            UnitKind::Infantry => 1,
            UnitKind::Vehicle | UnitKind::Aircraft => 2,
            UnitKind::Tower => 3,
        }
    }

    pub fn flies(self) -> bool {
        self == UnitKind::Aircraft
    }

    /// Its height above the ground. High enough for its sphere to clear everything down there.
    fn altitude(self) -> f32 {
        if self.flies() { CRUISE_ALTITUDE } else { 0.0 }
    }

    pub fn max_health(self) -> f32 {
        match self {
            UnitKind::Infantry => 100.0,
            UnitKind::Vehicle => 300.0,
            UnitKind::Aircraft => 150.0,
            UnitKind::Tower => 600.0,
        }
    }

    /// World units per second.
    fn speed(self) -> f32 {
        match self {
            UnitKind::Infantry => 2.5,
            UnitKind::Vehicle => 4.5,
            UnitKind::Aircraft => 7.0,
            UnitKind::Tower => 0.0,
        }
    }
}

/// How high aircraft fly, in world units.
const CRUISE_ALTITUDE: f32 = 4.0;
/// Experience needed for each rank above the first.
const RANK_EXPERIENCE: [u32; 3] = [2, 5, 10];
/// What each rank adds, as a share of the unit's usual health, attack and speed.
const RANK_HEALTH: f32 = 0.25;
const RANK_ATTACK: f32 = 0.2;
const RANK_SPEED: f32 = 0.1;

/// How far a unit sees, in world units, by what it is: it always sees as far as it can hit.
const INFANTRY_SIGHT: f32 = 8.0;
const VEHICLE_SIGHT: f32 = 9.0;
const AIRCRAFT_SIGHT: f32 = 10.0;
const TOWER_SIGHT: f32 = 10.0;
const SCOUT_SIGHT: f32 = 14.0;

/// What a unit is, beyond how it moves: this names it and decides what it starts with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    LightInfantry,
    /// Fast and unarmed.
    Scout,
    /// Fires slowly, but far and hard.
    Sniper,
    /// Takes enemy buildings over by reaching them.
    Engineer,
    /// Fires a pistol, or stands deployed and fires a cannon.
    KInfantry,
    Tank,
    /// Unarmed, and fixes friendly vehicles nearby.
    RepairVehicle,
    /// Lobs shells in a ballistic arc.
    Catapult,
    Aircraft,
    /// Hangs in the air and fires an automatic gun at the air and the ground alike.
    AirDefensePlatform,
    /// Flies, and fires its missiles at enemy aircraft only.
    AirDrone,
    /// Sprays fire at the ground in front of it.
    FlameInfantry,
    /// Sprays green gas at the ground in front of it.
    GasInfantry,
    Tower,
    /// A tower that throws fire.
    FlameTower,
}

impl Role {
    pub fn kind(self) -> UnitKind {
        match self {
            Role::LightInfantry
            | Role::Scout
            | Role::Sniper
            | Role::Engineer
            | Role::KInfantry
            | Role::FlameInfantry
            | Role::GasInfantry => UnitKind::Infantry,
            Role::Tank | Role::RepairVehicle | Role::Catapult => UnitKind::Vehicle,
            Role::Aircraft | Role::AirDefensePlatform | Role::AirDrone => UnitKind::Aircraft,
            Role::Tower | Role::FlameTower => UnitKind::Tower,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Role::LightInfantry => "Light Infantry",
            Role::Scout => "Scout",
            Role::Sniper => "Sniper",
            Role::Engineer => "Engineer",
            Role::KInfantry => "K Infantry",
            Role::Tank => "Tank",
            Role::RepairVehicle => "Repair Vehicle",
            Role::Catapult => "Catapult",
            Role::Aircraft => "Aircraft",
            Role::AirDefensePlatform => "Air Defense Platform",
            Role::AirDrone => "Air Drone",
            Role::FlameInfantry => "Flame Infantry",
            Role::GasInfantry => "Gas Infantry",
            Role::Tower => "Tower",
            Role::FlameTower => "Flame Tower",
        }
    }

    fn weapon(self) -> WeaponKind {
        match self {
            Role::LightInfantry => WeaponKind::Bullet,
            Role::Scout => WeaponKind::Unarmed,
            Role::Sniper => WeaponKind::Sniper,
            Role::Engineer | Role::KInfantry => WeaponKind::Pistol,
            Role::Tank => WeaponKind::Cannon,
            Role::RepairVehicle => WeaponKind::Repair,
            Role::Catapult => WeaponKind::Mortar,
            Role::Aircraft | Role::Tower => WeaponKind::Missile,
            Role::AirDefensePlatform => WeaponKind::AutoGun,
            Role::AirDrone => WeaponKind::AirMissile,
            Role::FlameInfantry | Role::FlameTower => WeaponKind::Flame,
            Role::GasInfantry => WeaponKind::Gas,
        }
    }

    fn max_health(self) -> f32 {
        match self {
            Role::Scout => 60.0,
            Role::Sniper => 70.0,
            Role::Engineer => 80.0,
            Role::KInfantry => 120.0,
            Role::RepairVehicle => 200.0,
            Role::Catapult => 180.0,
            Role::AirDefensePlatform => 200.0,
            Role::AirDrone => 80.0,
            Role::FlameTower => 500.0,
            _ => self.kind().max_health(),
        }
    }

    fn speed(self) -> f32 {
        match self {
            Role::Scout => 4.0,
            Role::Catapult => 3.5,
            Role::AirDefensePlatform => 3.0,
            Role::AirDrone => 8.0,
            _ => self.kind().speed(),
        }
    }

    fn for_kind(kind: UnitKind) -> Self {
        match kind {
            UnitKind::Infantry => Role::LightInfantry,
            UnitKind::Vehicle => Role::Tank,
            UnitKind::Aircraft => Role::Aircraft,
            UnitKind::Tower => Role::Tower,
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
/// A block of the player's this deep has a row of aircraft.
const ROWS_WITH_AIRCRAFT: usize = 4;
/// How far to the side of its group's middle the player's tower stands.
const PLAYER_TOWER_OFFSET: f32 = -3.4;
/// How far to the side of its group's middle an opponent's tower stands. Not so far that it
/// leaves the land kept clear around the ring.
const OPPONENT_TOWER_OFFSET: f32 = 3.5;
/// How high a tower's tubes are.
pub const TUBE_HEIGHT: f32 = 1.26;
/// How far ahead of a tower's middle its tubes end, and how far apart they stand from it.
const TUBE_REACH: f32 = 0.7;
pub const TUBE_SPREAD: f32 = 0.18;

/// What a unit was told to go after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// By id: an enemy to fight, or for a repair vehicle a friendly vehicle to look after.
    Unit(u32),
    /// By id: any unit at all it was forced to fire on, friendly or not.
    Forced(u32),
    /// A place on the map (x, z) to fire at until told otherwise.
    Place(f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    pub role: Role,
    /// Earned by killing enemies, and what decides its rank.
    pub experience: u32,
    /// Whether K Infantry has dug in with its cannon, unable to move.
    pub deployed: bool,
    /// Stays the same for life, so that a missile can keep chasing the same unit.
    pub id: u32,
    pub health: f32,
    pub weapon: WeaponKind,
    /// Seconds until it can fire again.
    pub cooldown: f32,
    /// Height above the ground: 0 for all but aircraft.
    pub altitude: f32,
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
    /// What its owner told it to go after. It keeps at this until a target unit dies or
    /// a new order replaces it.
    pub target: Option<Target>,
    /// A heavy unit squashes enemy infantry it drives over. Every vehicle is heavy for now.
    pub heavy: bool,
    pub alive: bool,
    /// Which of a tower's two tubes fires next.
    pub second_tube: bool,
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

    pub fn name(&self) -> &'static str {
        match (self.role, self.weapon) {
            (Role::LightInfantry, WeaponKind::Missile) => "Missile Infantry",
            (Role::LightInfantry, WeaponKind::Laser) => "Laser Infantry",
            (role, _) => role.name(),
        }
    }

    /// How far it sees along the ground.
    pub fn sight(&self) -> f32 {
        let own = match (self.role, self.kind) {
            (Role::Scout, _) => SCOUT_SIGHT,
            (_, UnitKind::Infantry) => INFANTRY_SIGHT,
            (_, UnitKind::Vehicle) => VEHICLE_SIGHT,
            (_, UnitKind::Aircraft) => AIRCRAFT_SIGHT,
            (_, UnitKind::Tower) => TOWER_SIGHT,
        };
        own.max(self.weapon.range() + 1.0)
    }

    /// From 0, with no medal, up to 3.
    pub fn rank(&self) -> u32 {
        RANK_EXPERIENCE
            .iter()
            .filter(|&&needed| self.experience >= needed)
            .count() as u32
    }

    pub fn max_health(&self) -> f32 {
        self.role.max_health() * (1.0 + RANK_HEALTH * self.rank() as f32)
    }

    /// What its damage is multiplied by.
    pub fn attack_power(&self) -> f32 {
        1.0 + RANK_ATTACK * self.rank() as f32
    }

    fn speed(&self) -> f32 {
        self.role.speed() * (1.0 + RANK_SPEED * self.rank() as f32)
    }

    /// Adds experience. Reaching a new rank adds the health that rank brings to what it has.
    pub fn gain_experience(&mut self, points: u32) {
        let before = self.max_health();
        self.experience += points;
        self.health += self.max_health() - before;
    }

    /// Whether a repair vehicle can work on it: a vehicle, or an aircraft while it is grounded.
    pub fn can_be_repaired(&self) -> bool {
        self.kind == UnitKind::Vehicle || (self.kind.flies() && self.altitude <= 0.0)
    }

    /// Forgets any target and stays where it is.
    fn stand_down(&mut self) {
        self.target = None;
        self.destination = (self.x, self.z);
    }

    fn reset_progress(&mut self) {
        self.blocked_for = 0.0;
        self.best_remaining = f32::INFINITY;
    }

    /// Whether it can be forced to fire on another unit, whoever that is.
    pub fn can_fire_on(&self, other: &Unit) -> bool {
        self.alive
            && other.alive
            && self.id != other.id
            && self.weapon.can_target(other.kind, other.altitude)
    }

    /// Whether it can be told to fire at a place on the ground.
    pub fn can_fire_at_ground(&self) -> bool {
        self.alive && self.weapon.can_target(UnitKind::Infantry, 0.0)
    }

    /// Whether it can look after another unit: a repair vehicle fixes friendly vehicles.
    pub fn can_fix(&self, other: &Unit, teams: &[usize]) -> bool {
        self.weapon == WeaponKind::Repair
            && self.alive
            && other.alive
            && self.id != other.id
            && other.can_be_repaired()
            && are_friends(teams, self.owner, other.owner)
    }

    /// Whether it could take this building over.
    pub fn can_capture(&self, other: &Unit, teams: &[usize]) -> bool {
        self.role == Role::Engineer
            && self.alive
            && other.alive
            && other.kind.is_building()
            && are_enemies(teams, self.owner, other.owner)
    }

    /// Whether its weapon can be turned on another unit.
    pub fn can_attack(&self, other: &Unit, teams: &[usize]) -> bool {
        self.alive
            && other.alive
            && are_enemies(teams, self.owner, other.owner)
            && self.weapon.can_target(other.kind, other.altitude)
    }

    /// Whether it could be at this spot. Aircraft only have to stay on the map.
    fn can_enter(&self, terrain: &Terrain, x: f32, z: f32) -> bool {
        if self.kind.flies() {
            terrain.on_map(x, z)
        } else {
            terrain.can_walk(x, z)
        }
    }

    /// Its collision sphere's center height.
    fn sphere_y(&self) -> f32 {
        self.y + self.altitude + self.kind.radius()
    }

    /// Its collision sphere's center, where shots aim.
    pub fn center(&self) -> (f32, f32, f32) {
        (self.x, self.sphere_y(), self.z)
    }

    /// Where its shots come from: a tower takes turns with the ends of its two tubes.
    pub fn muzzle(&self) -> (f32, f32, f32) {
        if self.kind.is_building() {
            let (sin, cos) = self.heading.sin_cos();
            let side = match (self.role, self.second_tube) {
                (Role::FlameTower, _) => 0.0,
                (_, true) => TUBE_SPREAD,
                (_, false) => -TUBE_SPREAD,
            };
            return (
                self.x + cos * TUBE_REACH - sin * side,
                self.y + TUBE_HEIGHT,
                self.z + sin * TUBE_REACH + cos * side,
            );
        }
        (self.x, self.y + self.altitude + HEIGHT * 0.8, self.z)
    }

    /// The height on screen that a click aims at.
    pub fn pick_height(&self) -> f32 {
        let above_feet = if self.kind.is_building() {
            TUBE_HEIGHT * 0.7
        } else {
            HEIGHT / 2.0
        };
        self.y + self.altitude + above_feet
    }

    /// Whether the player can select it, and so give it orders.
    pub fn is_selectable(&self) -> bool {
        self.owner == 0 && !self.kind.is_building()
    }

    /// A lone player's unit standing at a point, ready to fire with bullets.
    #[cfg(test)]
    pub fn for_test(x: f32, z: f32, kind: UnitKind) -> Self {
        Unit {
            role: Role::for_kind(kind),
            experience: 0,
            deployed: false,
            id: 0,
            health: kind.max_health(),
            weapon: WeaponKind::Bullet,
            cooldown: 0.0,
            altitude: kind.altitude(),
            x,
            y: 0.0,
            z,
            owner: 0,
            color: TeamColor::Blue,
            selected: false,
            kind,
            heading: -FRAC_PI_2,
            destination: (x, z),
            target: None,
            heavy: kind == UnitKind::Vehicle,
            alive: true,
            second_tube: false,
            blocked_for: 0.0,
            best_remaining: f32::INFINITY,
        }
    }
}

/// What a block's infantry carries: the player's mixes the three small arms by column, so
/// that all of them are on the field, and opponents' have bullets.
fn infantry_weapon(owner: usize, column: usize) -> WeaponKind {
    match (owner, column % 3) {
        (0, 1) => WeaponKind::Missile,
        (0, 2) => WeaponKind::Laser,
        _ => WeaponKind::Bullet,
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

/// Who stands where in a block. Only the player has vehicles, in the block's last row, so
/// that a skirmish's enemies can all be run over, and aircraft, in the first row of a block
/// deep enough to spare one.
fn kind_in_block(owner: usize, row: usize, rows: usize) -> UnitKind {
    if owner != 0 {
        UnitKind::Infantry
    } else if row + 1 == rows {
        UnitKind::Vehicle
    } else if row == 0 && rows >= ROWS_WITH_AIRCRAFT {
        UnitKind::Aircraft
    } else {
        UnitKind::Infantry
    }
}

/// Whether two owners are the same or allied. An unknown owner is nobody's friend.
pub fn are_friends(teams: &[usize], a: usize, b: usize) -> bool {
    match (teams.get(a), teams.get(b)) {
        (Some(team_a), Some(team_b)) => team_a == team_b,
        _ => false,
    }
}

/// Places a block of units centered on a world point.
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
            // A row of vehicles ends in a catapult, and the rest are tanks.
            let (role, weapon) = match kind_in_block(owner, row, rows) {
                UnitKind::Infantry => (Role::LightInfantry, infantry_weapon(owner, col)),
                UnitKind::Vehicle if col + 1 == cols => (Role::Catapult, Role::Catapult.weapon()),
                kind => (Role::for_kind(kind), Role::for_kind(kind).weapon()),
            };
            place_unit(
                units,
                terrain,
                (x, z),
                role,
                weapon,
                (owner, color),
                units.len() as u32,
            );
        }
    }
}

fn place_unit(
    units: &mut Vec<Unit>,
    terrain: &Terrain,
    (x, z): (f32, f32),
    role: Role,
    weapon: WeaponKind,
    (owner, color): (usize, TeamColor),
    id: u32,
) {
    let kind = role.kind();
    units.push(Unit {
        role,
        experience: 0,
        deployed: false,
        id,
        health: role.max_health(),
        weapon,
        cooldown: 0.0,
        altitude: kind.altitude(),
        x,
        y: terrain.height_at(x, z),
        z,
        owner,
        color,
        selected: false,
        kind,
        heading: -FRAC_PI_2,
        destination: (x, z),
        target: None,
        heavy: kind == UnitKind::Vehicle,
        alive: true,
        second_tube: false,
        blocked_for: 0.0,
        best_remaining: f32::INFINITY,
    });
}

fn place_building(
    units: &mut Vec<Unit>,
    terrain: &Terrain,
    at: (f32, f32),
    role: Role,
    side: (usize, TeamColor),
) {
    let id = units.len() as u32;
    place_unit(units, terrain, at, role, role.weapon(), side, id);
}

/// Adds a new unit at the free spot nearest to a point, if there is one. Returns whether
/// it did. Nobody stands or is headed where it appears.
pub fn produce(
    units: &mut Vec<Unit>,
    terrain: &Terrain,
    role: Role,
    near: (f32, f32),
    side: (usize, TeamColor),
    id: u32,
) -> bool {
    let taken: Vec<(f32, f32)> = units
        .iter()
        .filter(|u| u.alive && !u.kind.flies())
        .flat_map(|u| [(u.x, u.z), u.destination])
        .collect();
    let Some(&spot) = find_slots(terrain, near, 1, &taken, false).first() else {
        return false;
    };
    place_unit(units, terrain, spot, role, role.weapon(), side, id);
    true
}

/// Switches every selected K Infantry between its pistol and, standing still, its cannon.
pub fn toggle_deploy(units: &mut [Unit]) {
    for unit in units
        .iter_mut()
        .filter(|u| u.selected && u.role == Role::KInfantry)
    {
        unit.deployed = !unit.deployed;
        unit.weapon = if unit.deployed {
            WeaponKind::Cannon
        } else {
            WeaponKind::Pistol
        };
        unit.destination = (unit.x, unit.z);
    }
}

/// The player gets a block between two towers at the map's middle and another block a long
/// scroll away. Each opponent gets one block between two towers, evenly around a ring.
/// `colors` holds the player's, then the opponents'.
pub fn spawn(terrain: &Terrain, colors: &[TeamColor]) -> Vec<Unit> {
    let mut units = Vec::new();
    let player = (0, colors[0]);
    spawn_group(
        &mut units,
        terrain,
        (0.0, MAP_MIDDLE_Z),
        (4, ROWS_WITH_AIRCRAFT),
        player,
    );
    place_building(
        &mut units,
        terrain,
        (PLAYER_TOWER_OFFSET, MAP_MIDDLE_Z),
        Role::Tower,
        player,
    );
    place_building(
        &mut units,
        terrain,
        (-PLAYER_TOWER_OFFSET, MAP_MIDDLE_Z),
        Role::FlameTower,
        player,
    );
    spawn_group(
        &mut units,
        terrain,
        (SECOND_GROUP_X, MAP_MIDDLE_Z),
        (4, 2),
        player,
    );
    let opponents = colors.len() - 1;
    for (i, &color) in colors[1..].iter().enumerate() {
        // The ring starts in the north, where it is clear of the player's second group.
        let angle = i as f32 / opponents as f32 * TAU;
        let (sin, cos) = angle.sin_cos();
        let center = (OPPONENT_RING * sin, MAP_MIDDLE_Z - OPPONENT_RING * cos);
        spawn_group(&mut units, terrain, center, (3, 2), (i + 1, color));
        let tower = (
            center.0 + OPPONENT_TOWER_OFFSET * cos,
            center.1 + OPPONENT_TOWER_OFFSET * sin,
        );
        place_building(&mut units, terrain, tower, Role::Tower, (i + 1, color));
        let flame = (
            center.0 - OPPONENT_TOWER_OFFSET * cos,
            center.1 - OPPONENT_TOWER_OFFSET * sin,
        );
        place_building(&mut units, terrain, flame, Role::FlameTower, (i + 1, color));
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
/// and from every point in `taken`. Spots on the ground must be walkable, so a click on water
/// finds the nearest land. Spots in the air only have to be on the map.
fn find_slots(
    terrain: &Terrain,
    target: (f32, f32),
    count: usize,
    taken: &[(f32, f32)],
    flying: bool,
) -> Vec<(f32, f32)> {
    let mut offsets = Vec::new();
    for j in -SLOT_SEARCH_RINGS..=SLOT_SEARCH_RINGS {
        for i in -SLOT_SEARCH_RINGS..=SLOT_SEARCH_RINGS {
            offsets.push((i as f32 * SLOT_SPACING, j as f32 * SLOT_SPACING));
        }
    }
    offsets.sort_by(|a, b| a.0.hypot(a.1).total_cmp(&b.0.hypot(b.1)));
    let is_free = |spot: (f32, f32), others: &[(f32, f32)]| {
        others
            .iter()
            .all(|o| (o.0 - spot.0).hypot(o.1 - spot.1) >= SLOT_SPACING - 0.001)
    };
    let mut slots: Vec<(f32, f32)> = Vec::with_capacity(count);
    for (dx, dz) in offsets {
        if slots.len() == count {
            break;
        }
        let spot = (target.0 + dx, target.1 + dz);
        let usable = if flying {
            terrain.on_map(spot.0, spot.1)
        } else {
            let clear_of_ruins = terrain.ruins().iter().all(|piece| {
                let (near_x, near_z) = nearest_on_footprint(piece, spot.0, spot.1);
                (near_x - spot.0).hypot(near_z - spot.1) > UnitKind::Vehicle.radius()
            });
            clear_of_ruins && terrain.can_walk(spot.0, spot.1)
        };
        if usable && is_free(spot, taken) && is_free(spot, &slots) {
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
/// The air and the ground reserve their spots separately, since they cannot get in each
/// other's way.
pub fn order_move(units: &mut [Unit], target: (f32, f32), terrain: &Terrain, teams: &[usize]) {
    for flying in [false, true] {
        order_layer(units, target, terrain, teams, flying);
    }
}

fn order_layer(
    units: &mut [Unit],
    target: (f32, f32),
    terrain: &Terrain,
    teams: &[usize],
    flying: bool,
) {
    let in_layer = |u: &Unit| u.kind.flies() == flying;
    let any_heavy = units.iter().any(|u| in_layer(u) && u.selected && u.heavy);
    let taken: Vec<(f32, f32)> = units
        .iter()
        .filter(|u| {
            let can_be_run_over =
                any_heavy && are_enemies(teams, 0, u.owner) && !u.kind.is_building();
            in_layer(u) && !u.selected && u.alive && !can_be_run_over
        })
        .map(|u| u.destination)
        .collect();
    // Deployed units are dug in and stay put.
    let mut chosen: Vec<usize> = (0..units.len())
        .filter(|&i| in_layer(&units[i]) && units[i].selected && !units[i].deployed)
        .collect();
    let mut slots = find_slots(terrain, target, chosen.len(), &taken, flying);
    // Farthest first, so that those with the longest walk are not left the leftover spots.
    let distance = |unit: &Unit, spot: (f32, f32)| (unit.x - spot.0).hypot(unit.z - spot.1);
    chosen.sort_by(|&a, &b| distance(&units[b], target).total_cmp(&distance(&units[a], target)));
    for i in chosen {
        let nearest = (0..slots.len()).min_by(|&a, &b| {
            distance(&units[i], slots[a]).total_cmp(&distance(&units[i], slots[b]))
        });
        let Some(nearest) = nearest else { return };
        units[i].destination = slots.swap_remove(nearest);
        units[i].target = None;
        units[i].reset_progress();
    }
}

fn set_target(unit: &mut Unit, target: Target) {
    unit.target = Some(target);
    unit.reset_progress();
}

/// Tells every selected unit able to fight this enemy, or fix this friend, to go after it.
/// The others carry on with what they were doing.
pub fn order_attack(units: &mut [Unit], chosen: usize, teams: &[usize]) {
    let chosen = units[chosen];
    for unit in units.iter_mut().filter(|u| u.selected) {
        if unit.can_attack(&chosen, teams) || unit.can_fix(&chosen, teams) {
            set_target(unit, Target::Unit(chosen.id));
        }
    }
}

/// Like `order_attack`, but those able to fire on it do so even if it is a friend.
pub fn order_force_attack(units: &mut [Unit], chosen: usize, teams: &[usize]) {
    let chosen = units[chosen];
    for unit in units.iter_mut().filter(|u| u.selected) {
        if unit.can_attack(&chosen, teams) || unit.can_fix(&chosen, teams) {
            set_target(unit, Target::Unit(chosen.id));
        } else if unit.can_fire_on(&chosen) {
            set_target(unit, Target::Forced(chosen.id));
        }
    }
}

/// Tells every selected unit able to fire at the ground to fire at this place.
pub fn order_attack_place(units: &mut [Unit], (x, z): (f32, f32)) {
    for unit in units
        .iter_mut()
        .filter(|u| u.selected && u.can_fire_at_ground())
    {
        set_target(unit, Target::Place(x, z));
    }
}

/// Points everyone with a target at it, or stops them once it is within reach. A unit whose
/// target unit is gone, or no longer fit for its order, is free and stays where it is. An
/// engineer that reaches its target building takes it over.
fn chase_targets(units: &mut [Unit], terrain: &Terrain, teams: &[usize]) {
    for i in 0..units.len() {
        let Some(target) = units[i].target else {
            continue;
        };
        if !units[i].alive {
            continue;
        }
        let (x, z) = (units[i].x, units[i].z);
        let goal = match target {
            Target::Place(place_x, place_z) => {
                (place_x, terrain.height_at(place_x, place_z), place_z)
            }
            Target::Unit(id) | Target::Forced(id) => {
                let fit = |u: &Unit| match target {
                    Target::Forced(_) => units[i].can_fire_on(u),
                    _ => units[i].can_attack(u, teams) || units[i].can_fix(u, teams),
                };
                let Some(other_index) = units.iter().position(|u| u.alive && u.id == id && fit(u))
                else {
                    units[i].stand_down();
                    continue;
                };
                let other = units[other_index];
                if matches!(target, Target::Unit(_)) && units[i].can_capture(&other, teams) {
                    let touching = units[i].kind.radius() + other.kind.radius() + CAPTURE_MARGIN;
                    if (other.x - x).hypot(other.z - z) <= touching {
                        (units[other_index].owner, units[other_index].color) =
                            (units[i].owner, units[i].color);
                        units[i].stand_down();
                    } else {
                        units[i].destination = (other.x, other.z);
                    }
                    continue;
                }
                other.center()
            }
        };
        if units[i].deployed {
            continue;
        }
        let away = distance(goal, units[i].center());
        let in_reach = away <= units[i].weapon.range() * ATTACK_REACH;
        units[i].destination = if in_reach { (x, z) } else { (goal.0, goal.2) };
    }
}

/// Whether a moving unit would squash another by touching it.
fn can_squash(squasher: &Unit, victim: &Unit, teams: &[usize]) -> bool {
    squasher.heavy
        && squasher.is_moving()
        && victim.kind == UnitKind::Infantry
        && are_enemies(teams, squasher.owner, victim.owner)
}

/// The nearest infantry a heavy unit could run down close ahead of it, as a unit vector.
fn prey_direction(
    units: &[Unit],
    index: usize,
    teams: &[usize],
    ahead: (f32, f32),
) -> Option<(f32, f32)> {
    let unit = &units[index];
    let mut best = None;
    let mut best_distance = SQUASH_LURE;
    for other in units
        .iter()
        .filter(|o| o.alive && can_squash(unit, o, teams))
    {
        let (dx, dz) = (other.x - unit.x, other.z - unit.z);
        let distance = dx.hypot(dz).max(0.0001);
        let (dir_x, dir_z) = (dx / distance, dz / distance);
        if distance < best_distance && dir_x * ahead.0 + dir_z * ahead.1 > SQUASH_LURE_AHEAD {
            best = Some((dir_x, dir_z));
            best_distance = distance;
        }
    }
    best
}

/// Which way to head: straight for the destination, bending right around whatever stands
/// close ahead. Everyone keeps right, so two units meeting head-on pass each other. Heavy
/// units go out of their way for infantry they can squash, then carry on. The air and the
/// ground ignore each other, and aircraft pass over ruins.
fn steering(units: &[Unit], index: usize, terrain: &Terrain, teams: &[usize]) -> (f32, f32) {
    let unit = &units[index];
    let (to_x, to_z) = (unit.destination.0 - unit.x, unit.destination.1 - unit.z);
    let length = to_x.hypot(to_z).max(0.0001);
    let toward_destination = (to_x / length, to_z / length);
    let (want_x, want_z) =
        prey_direction(units, index, teams, toward_destination).unwrap_or(toward_destination);
    let (right_x, right_z) = (-want_z, want_x);
    let mut sideways = 0.0;
    for (other_index, other) in units.iter().enumerate() {
        let other_layer = other.kind.flies() != unit.kind.flies();
        let building_to_capture =
            unit.target == Some(Target::Unit(other.id)) && unit.can_capture(other, teams);
        if other_index == index
            || !other.alive
            || other_layer
            || building_to_capture
            || can_squash(unit, other, teams)
        {
            continue;
        }
        let (dx, dz) = (other.x - unit.x, other.z - unit.z);
        let distance = dx.hypot(dz);
        let range = unit.kind.radius() + other.kind.radius() + AVOID_MARGIN;
        if distance < range && dx * want_x + dz * want_z > 0.0 {
            sideways += (1.0 - distance / range) * 2.0;
        }
    }
    let ruins = if unit.kind.flies() {
        &[][..]
    } else {
        terrain.ruins()
    };
    for piece in ruins {
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
pub fn turn_between(from: f32, to: f32) -> f32 {
    (to - from + PI).rem_euclid(TAU) - PI
}

/// Moves a unit along its way, sliding along whatever it cannot enter. Returns whether
/// those got in its way.
fn walk(unit: &mut Unit, terrain: &Terrain, (dir_x, dir_z): (f32, f32), dt: f32) -> bool {
    let distance = (unit.speed() * dt).min(unit.remaining());
    let (dx, dz) = (dir_x * distance, dir_z * distance);
    let tries = [(dx, dz), (dx, 0.0), (0.0, dz)];
    match tries
        .iter()
        .find(|(tx, tz)| unit.can_enter(terrain, unit.x + tx, unit.z + tz))
    {
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
    if unit.kind.flies() || unit.kind.is_building() {
        return false;
    }
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
            let (to_x, to_z) = (
                piece.half_x - (unit.x - piece.x).abs(),
                piece.half_z - (unit.z - piece.z).abs(),
            );
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
            if a == b
                || !units[a].alive
                || !units[b].alive
                || !can_squash(&units[a], &units[b], teams)
            {
                continue;
            }
            let (dx, dz) = (units[b].x - units[a].x, units[b].z - units[a].z);
            let dy = units[b].sphere_y() - units[a].sphere_y();
            if dx.hypot(dz).hypot(dy) < units[a].kind.radius() + units[b].kind.radius() {
                units[b].alive = false;
                units[b].selected = false;
                let value = units[b].kind.kill_value();
                units[a].gain_experience(value);
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
                let (nx, nz) = if flat > 0.0001 {
                    (dx / flat, dz / flat)
                } else {
                    (1.0, 0.0)
                };
                let overlap = touching - flat.hypot(dy);
                // Infantry cannot shove something heavy: it takes the whole push itself. Nothing
                // shoves a building.
                let solid = |unit: &Unit| (unit.kind.is_building() || unit.deployed, unit.heavy);
                let (share_a, share_b) = match solid(&units[a]).cmp(&solid(&units[b])) {
                    Ordering::Greater => (0.0, 1.0),
                    Ordering::Less => (1.0, 0.0),
                    Ordering::Equal => (0.5, 0.5),
                };
                for (i, sign, share) in [(a, -1.0, share_a), (b, 1.0, share_b)] {
                    let push = overlap * share;
                    let (x, z) = (units[i].x + nx * push * sign, units[i].z + nz * push * sign);
                    if units[i].can_enter(terrain, x, z) {
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
    let started_at: Vec<(f32, f32)> = units.iter().map(|u| (u.x, u.z)).collect();
    for i in 0..units.len() {
        if !units[i].is_moving() {
            continue;
        }
        let (dir_x, dir_z) = steering(units, i, terrain, teams);
        let way = dir_z.atan2(dir_x);
        let unit = &mut units[i];
        let turn = turn_between(unit.heading, way);
        let most = TURN_SPEED * dt;
        match unit.kind {
            UnitKind::Infantry => {
                unit.heading = way;
                held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
            }
            UnitKind::Vehicle => {
                unit.heading += turn.clamp(-most, most);
                if turn.abs() <= DRIVE_ALIGNMENT {
                    held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
                }
            }
            // Nothing holds it back from setting off while its nose comes around.
            UnitKind::Aircraft => {
                unit.heading += turn.clamp(-most, most);
                held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
            }
            UnitKind::Tower => {}
        }
    }
    squash(units, teams);
    separate(units, terrain);
    for (i, unit) in units.iter_mut().enumerate() {
        if !unit.alive {
            continue;
        }
        held_back[i] |= push_out_of_ruins(unit, terrain);
        // Wedged against those who stay put, a chaser would otherwise press on forever.
        let moved = (unit.x - started_at[i].0).hypot(unit.z - started_at[i].1);
        held_back[i] |= unit.target.is_some() && moved < unit.speed() * dt * WEDGED_SHARE;
        unit.y = terrain.height_at(unit.x, unit.z);
        give_up_if_stuck(unit, held_back[i], dt);
    }
}

/// Crowds only delay a unit, so its order stands. If water, lava or a ruin keeps it from
/// getting any nearer, or it is wedged on its way to a target, there is no way through
/// without path finding: it settles for here and forgets the target.
fn give_up_if_stuck(unit: &mut Unit, held_back: bool, dt: f32) {
    if !unit.is_moving() {
        unit.reset_progress();
        return;
    }
    if unit.remaining() < unit.best_remaining - PROGRESS {
        unit.best_remaining = unit.remaining();
        unit.blocked_for = 0.0;
    } else if held_back {
        unit.blocked_for += dt;
        if unit.blocked_for > BLOCKED_PATIENCE {
            unit.stand_down();
            unit.blocked_for = 0.0;
        }
    }
}

/// Advances every unit by `dt` seconds. Units killed meanwhile are left marked as not alive.
pub fn advance(units: &mut [Unit], terrain: &Terrain, teams: &[usize], dt: f32) {
    chase_targets(units, terrain, teams);
    // Spheres only start to overlap when something moves.
    if !units.iter().any(Unit::is_moving) {
        return;
    }
    let (steps, step_seconds) = substeps(dt);
    for _ in 0..steps {
        step(units, terrain, teams, step_seconds);
        chase_targets(units, terrain, teams);
    }
}

/// Whether a point on the ground is inside a rectangle: (west, north, east, south).
pub fn in_rectangle((west, north, east, south): (f32, f32, f32, f32), x: f32, z: f32) -> bool {
    x >= west && x <= east && z >= north && z <= south
}

/// Whether a unit stands inside a rectangle on the ground.
pub fn in_selection(rectangle: (f32, f32, f32, f32), unit: &Unit) -> bool {
    in_rectangle(rectangle, unit.x, unit.z)
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
    let (nx, ny) = if length > 0.0 {
        (-dy / length, dx / length)
    } else {
        (0.0, 0.0)
    };
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

    const COLORS: [TeamColor; 4] = [
        TeamColor::Blue,
        TeamColor::Red,
        TeamColor::Cyan,
        TeamColor::Green,
    ];

    fn unit_at(x: f32, z: f32, kind: UnitKind) -> Unit {
        Unit::for_test(x, z, kind)
    }

    #[test]
    fn every_unit_has_a_weapon_and_all_of_them_are_on_the_players_side() {
        let units = spawn(terrain(0), &COLORS);
        let own: Vec<_> = units.iter().filter(|u| u.owner == 0).collect();
        for weapon in [
            WeaponKind::Bullet,
            WeaponKind::Missile,
            WeaponKind::Laser,
            WeaponKind::Mortar,
        ] {
            assert!(own.iter().any(|u| u.weapon == weapon), "{weapon:?}");
        }
        assert!(
            own.iter()
                .any(|u| u.role == Role::Tank && u.weapon == WeaponKind::Cannon)
        );
        assert!(
            own.iter()
                .any(|u| u.role == Role::Catapult && u.weapon == WeaponKind::Mortar)
        );
        assert!(
            units
                .iter()
                .filter(|u| u.kind == UnitKind::Infantry && u.owner != 0)
                .all(|u| u.weapon == WeaponKind::Bullet)
        );
    }

    #[test]
    fn every_unit_has_its_own_id_and_full_health() {
        let units = spawn(terrain(0), &COLORS);
        for (i, a) in units.iter().enumerate() {
            assert_eq!(a.health, a.max_health());
            assert!(units[i + 1..].iter().all(|b| b.id != a.id));
        }
    }

    fn enemy_at(x: f32, z: f32, kind: UnitKind) -> Unit {
        Unit {
            owner: 1,
            color: TeamColor::Red,
            ..unit_at(x, z, kind)
        }
    }

    const TWO_SIDES: [usize; 2] = [0, 1];

    fn wall(x: f32, z: f32, half_x: f32, half_z: f32, height: f32) -> RuinPiece {
        RuinPiece {
            x,
            y: 0.0,
            z,
            half_x,
            half_z,
            height,
            shade: 1.0,
        }
    }

    #[test]
    fn a_moving_heavy_vehicle_squashes_enemy_infantry_in_its_way() {
        let mut units = [
            unit_at(-3.0, 60.0, UnitKind::Vehicle),
            enemy_at(0.0, 60.0, UnitKind::Infantry),
        ];
        units[0].heading = 0.0;
        units[0].destination = (3.0, 60.0);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(!units[1].alive, "run over");
        assert!(units[0].alive);
        assert!(
            (units[0].x - 3.0).abs() < 0.2,
            "and it drove on straight: {:?}",
            units[0]
        );
        assert!(
            (units[0].z - 60.0).abs() < 0.2,
            "it did not swerve: {:?}",
            units[0]
        );
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
        let light = Unit {
            heavy: false,
            ..vehicle
        };
        let target = enemy_at(0.0, 60.0, UnitKind::Infantry);

        assert!(
            run_into(infantry, target, &TWO_SIDES),
            "infantry cannot squash"
        );
        assert!(
            run_into(light, target, &TWO_SIDES),
            "nor a vehicle that is not heavy"
        );
        assert!(
            run_into(vehicle, enemy_at(0.0, 60.0, UnitKind::Vehicle), &TWO_SIDES),
            "nor crush vehicles"
        );
        assert!(
            run_into(vehicle, unit_at(0.0, 60.0, UnitKind::Infantry), &TWO_SIDES),
            "own infantry lives"
        );
        assert!(run_into(vehicle, target, &[0, 0]), "and so do allies");
    }

    #[test]
    fn a_vehicle_goes_out_of_its_way_to_squash_enemy_infantry_then_carries_on() {
        let mut units = [
            unit_at(-6.0, 60.0, UnitKind::Vehicle),
            enemy_at(0.0, 62.5, UnitKind::Infantry),
        ];
        units[0].heading = 0.0;
        units[0].destination = (8.0, 60.0);
        for _ in 0..400 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(
            !units[1].alive,
            "it swerved and got them, though they stood off its way"
        );
        assert_eq!(units[0].destination, (8.0, 60.0), "its order stands");
        assert!(
            !units[0].is_moving(),
            "and it finished the trip: {:?}",
            units[0]
        );
    }

    #[test]
    fn a_vehicle_does_not_swerve_for_anyone_behind_it_or_too_far_or_friendly() {
        for (target, teams) in [
            (enemy_at(-9.0, 60.0, UnitKind::Infantry), TWO_SIDES),
            (enemy_at(0.0, 66.0, UnitKind::Infantry), TWO_SIDES),
            (enemy_at(0.0, 62.5, UnitKind::Infantry), [0, 0]),
        ] {
            let mut units = [unit_at(-6.0, 60.0, UnitKind::Vehicle), target];
            units[0].heading = 0.0;
            units[0].destination = (8.0, 60.0);
            for _ in 0..400 {
                advance(&mut units, open_ground(), &teams, 0.05);
            }
            assert!(units[1].alive, "{target:?}");
        }
    }

    #[test]
    fn a_vehicle_standing_still_squashes_nobody() {
        let mut units = [
            unit_at(0.0, 60.0, UnitKind::Vehicle),
            enemy_at(-3.0, 60.0, UnitKind::Infantry),
        ];
        units[1].destination = (0.0, 60.0);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(units[1].alive);
    }

    #[test]
    fn aircraft_fly_straight_over_ruins_water_and_units_on_the_ground() {
        let terrain = map_with_impassable_lower_terrain();
        let (water_x, water_z) = (-40..=40)
            .flat_map(|x| (30..=100).map(move |z| (x as f32, z as f32)))
            .find(|&(x, z)| terrain.on_map(x, z) && !terrain.can_walk(x, z))
            .expect("this map has water");
        let mut plane = unit_at(0.0, MAP_MIDDLE_Z, UnitKind::Aircraft);
        plane.destination = (water_x, water_z);
        let mut units = [plane];
        for _ in 0..400 {
            advance(&mut units, terrain, &[0], 0.05);
        }
        assert_eq!(
            units[0].destination,
            (water_x, water_z),
            "it did not give up"
        );
        assert!(
            !units[0].is_moving(),
            "it hovers over the water: {:?}",
            units[0]
        );

        let walled = Terrain::open_with_ruins(vec![wall(0.0, 60.0, 0.5, 3.0, 2.0)]);
        let tank = unit_at(2.0, 60.0, UnitKind::Vehicle);
        let mut units = [unit_at(-5.0, 60.0, UnitKind::Aircraft), tank];
        units[0].heading = 0.0;
        units[0].destination = (5.0, 60.0);
        for _ in 0..40 {
            advance(&mut units, &walled, &[0], 0.05);
            assert_eq!(units[0].z, 60.0, "never strays: {:?}", units[0]);
            assert_eq!(
                (units[1].x, units[1].z),
                (2.0, 60.0),
                "the tank is not pushed"
            );
        }
        assert!(!units[0].is_moving(), "{:?}", units[0]);
    }

    #[test]
    fn aircraft_keep_clear_of_each_other() {
        let mut units = [
            unit_at(-5.0, 60.0, UnitKind::Aircraft),
            unit_at(5.0, 60.0, UnitKind::Aircraft),
        ];
        units[0].destination = (5.0, 60.0);
        units[1].destination = (-5.0, 60.0);
        for _ in 0..200 {
            advance(&mut units, open_ground(), &[], 0.05);
            assert!(largest_overlap(&units) < 0.05);
        }
        assert!(!units[0].is_moving() && !units[1].is_moving(), "{units:?}");
    }

    #[test]
    fn nothing_on_the_ground_squashes_or_is_squashed_by_aircraft() {
        let mut units = [
            unit_at(-3.0, 60.0, UnitKind::Vehicle),
            enemy_at(0.0, 60.0, UnitKind::Aircraft),
        ];
        units[0].heading = 0.0;
        units[0].destination = (3.0, 60.0);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(units[1].alive);
        assert_eq!(
            (units[1].x, units[1].z),
            (0.0, 60.0),
            "the vehicle passed under it"
        );
    }

    #[test]
    fn the_air_and_the_ground_may_be_sent_to_the_same_spot() {
        let mut units = [
            unit_at(-3.0, 60.0, UnitKind::Infantry),
            unit_at(3.0, 60.0, UnitKind::Aircraft),
        ];
        units[0].selected = true;
        units[1].selected = true;
        order_move(&mut units, (0.0, 50.0), open_ground(), &[]);
        assert_eq!(units[0].destination, (0.0, 50.0));
        assert_eq!(units[1].destination, (0.0, 50.0));
    }

    #[test]
    fn aircraft_may_be_sent_over_ruins_and_water_but_not_off_the_map() {
        let walled = Terrain::open_with_ruins(vec![wall(0.0, 60.0, 1.0, 1.0, 1.0)]);
        let mut units = [unit_at(-8.0, 60.0, UnitKind::Aircraft)];
        units[0].selected = true;
        order_move(&mut units, (0.0, 60.0), &walled, &[0]);
        assert_eq!(units[0].destination, (0.0, 60.0), "right over the ruin");

        let terrain = map_with_impassable_lower_terrain();
        let water = (-40..=40)
            .flat_map(|x| (30..=100).map(move |z| (x as f32, z as f32)))
            .find(|&(x, z)| terrain.on_map(x, z) && !terrain.can_walk(x, z))
            .expect("this map has water");
        order_move(&mut units, water, terrain, &[0]);
        assert_eq!(units[0].destination, water, "right over the water");

        order_move(&mut units, (0.0, -30.0), terrain, &[0]);
        assert!(
            terrain.on_map(units[0].destination.0, units[0].destination.1),
            "{:?}",
            units[0].destination
        );
    }

    #[test]
    fn the_player_starts_with_aircraft_in_the_air_and_the_others_do_not() {
        let units = spawn(terrain(0), &COLORS);
        let planes: Vec<_> = units
            .iter()
            .filter(|u| u.kind == UnitKind::Aircraft)
            .collect();
        assert!(!planes.is_empty());
        assert!(
            planes
                .iter()
                .all(|u| u.owner == 0 && u.altitude == CRUISE_ALTITUDE && !u.heavy)
        );
        assert!(planes.iter().all(|u| u.weapon == WeaponKind::Missile));
        assert!(
            units
                .iter()
                .filter(|u| !u.kind.flies())
                .all(|u| u.altitude == 0.0)
        );
    }

    fn attacker_and_enemy(enemy_x: f32) -> [Unit; 2] {
        let mut units = [
            unit_at(0.0, 60.0, UnitKind::Infantry),
            enemy_at(enemy_x, 60.0, UnitKind::Infantry),
        ];
        units[1].id = 7;
        units[0].selected = true;
        units[0].weapon = WeaponKind::Bullet;
        units
    }

    #[test]
    fn a_unit_told_to_attack_closes_to_within_range_and_stops_there() {
        let mut units = attacker_and_enemy(20.0);
        order_attack(&mut units, 1, &TWO_SIDES);
        assert_eq!(units[0].target, Some(Target::Unit(7)));
        for _ in 0..400 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(!units[0].is_moving());
        let away = units[1].x - units[0].x;
        assert!(away <= WeaponKind::Bullet.range(), "in range: {away}");
        assert!(
            away > WeaponKind::Bullet.range() * 0.8,
            "and no nearer than needed: {away}"
        );
        assert_eq!(units[0].target, Some(Target::Unit(7)), "it remembers");
    }

    #[test]
    fn a_unit_follows_a_target_that_moves_away() {
        let mut units = attacker_and_enemy(5.0);
        order_attack(&mut units, 1, &TWO_SIDES);
        advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        assert!(!units[0].is_moving(), "already in range");
        units[1].x = 25.0;
        units[1].destination = (25.0, 60.0);
        for _ in 0..400 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(units[0].x > 15.0, "it went after it: {:?}", units[0]);
    }

    #[test]
    fn a_unit_stops_where_it_is_when_its_target_dies() {
        let mut units = attacker_and_enemy(20.0);
        order_attack(&mut units, 1, &TWO_SIDES);
        for _ in 0..10 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        units[1].alive = false;
        let at = units[0].x;
        advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        assert_eq!(units[0].target, None);
        assert!(!units[0].is_moving());
        assert_eq!(units[0].x, at);
    }

    #[test]
    fn a_move_order_replaces_the_target() {
        let mut units = attacker_and_enemy(20.0);
        order_attack(&mut units, 1, &TWO_SIDES);
        order_move(&mut units, (-5.0, 60.0), open_ground(), &TWO_SIDES);
        assert_eq!(units[0].target, None);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(units[0].x < -4.0, "{:?}", units[0]);
    }

    #[test]
    fn only_selected_units_able_to_fight_it_take_the_order() {
        let plane = Unit {
            id: 7,
            ..enemy_at(20.0, 60.0, UnitKind::Aircraft)
        };
        let mut mortar = unit_at(0.0, 60.0, UnitKind::Vehicle);
        mortar.selected = true;
        mortar.weapon = WeaponKind::Mortar;
        let mut idle = unit_at(0.0, 62.0, UnitKind::Infantry);
        idle.weapon = WeaponKind::Bullet;
        let mut gunner = unit_at(0.0, 64.0, UnitKind::Infantry);
        gunner.selected = true;
        gunner.weapon = WeaponKind::Bullet;
        let mut units = [mortar, idle, gunner, plane];
        order_attack(&mut units, 3, &TWO_SIDES);
        assert_eq!(units[0].target, None, "mortars cannot aim up");
        assert_eq!(units[1].target, None, "not selected");
        assert_eq!(units[2].target, Some(Target::Unit(7)));
    }

    #[test]
    fn a_unit_cut_off_from_its_target_gives_up_on_it() {
        let terrain = Terrain::open_with_ruins(vec![
            wall(0.0, 56.0, 4.0, 0.2, 2.0),
            wall(0.0, 64.0, 4.0, 0.2, 2.0),
            wall(-4.0, 60.0, 0.2, 4.0, 2.0),
            wall(4.0, 60.0, 0.2, 4.0, 2.0),
        ]);
        let mut units = attacker_and_enemy(20.0);
        order_attack(&mut units, 1, &TWO_SIDES);
        for _ in 0..1200 {
            advance(&mut units, &terrain, &TWO_SIDES, 0.05);
        }
        assert_eq!(units[0].target, None);
        assert!(!units[0].is_moving(), "{:?}", units[0]);
    }

    #[test]
    fn a_unit_wedged_behind_others_gives_up_on_its_target() {
        // A row of parked vehicles too tight to pass, across the way to the target.
        let mut units = vec![
            unit_at(0.0, 60.0, UnitKind::Infantry),
            enemy_at(20.0, 60.0, UnitKind::Infantry),
        ];
        units[1].id = 7;
        units[0].selected = true;
        units[0].weapon = WeaponKind::Bullet;
        for i in -8..=8 {
            units.push(unit_at(1.5, 60.0 + i as f32 * 0.9, UnitKind::Vehicle));
        }
        order_attack(&mut units, 1, &TWO_SIDES);
        for _ in 0..1200 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(!units[0].is_moving(), "{:?}", units[0]);
    }

    #[test]
    fn everyone_starts_with_a_flame_tower_as_well_in_their_color() {
        let units = spawn(terrain(0), &COLORS);
        for (owner, &color) in COLORS.iter().enumerate() {
            let flame: Vec<_> = units
                .iter()
                .filter(|u| u.owner == owner && u.role == Role::FlameTower)
                .collect();
            assert_eq!(flame.len(), 1);
            assert_eq!(flame[0].color, color);
            assert_eq!(flame[0].weapon, WeaponKind::Flame);
            assert!(!flame[0].is_selectable());
        }
    }

    #[test]
    fn everyone_starts_with_one_tower_of_their_color_with_missiles() {
        let units = spawn(terrain(0), &COLORS);
        for (owner, &color) in COLORS.iter().enumerate() {
            let towers: Vec<_> = units
                .iter()
                .filter(|u| u.owner == owner && u.role == Role::Tower)
                .collect();
            assert_eq!(towers.len(), 1);
            assert_eq!(towers[0].color, color);
            assert_eq!(towers[0].weapon, WeaponKind::Missile);
            assert_eq!(towers[0].health, UnitKind::Tower.max_health());
            assert!(!towers[0].is_selectable());
        }
    }

    #[test]
    fn towers_stand_on_land_on_every_map_with_any_number_of_opponents() {
        let colors = crate::game::TEAM_COLORS;
        for map in 0..crate::game::MAPS.len() {
            for count in 2..=colors.len() {
                let terrain = terrain(map);
                for unit in spawn(terrain, &colors[..count])
                    .iter()
                    .filter(|u| u.kind.is_building())
                {
                    assert!(
                        terrain.can_walk(unit.x, unit.z),
                        "map {map}, {count} sides: {unit:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn nothing_moves_a_tower_or_runs_it_over() {
        let mut units = [
            unit_at(-4.0, 60.0, UnitKind::Vehicle),
            enemy_at(0.0, 60.0, UnitKind::Tower),
        ];
        units[0].heading = 0.0;
        units[0].destination = (4.0, 60.0);
        for _ in 0..400 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(units[1].alive, "not squashed");
        assert_eq!((units[1].x, units[1].z), (0.0, 60.0), "not pushed");
        assert!(
            !units[0].is_moving(),
            "and the vehicle drove around it: {:?}",
            units[0]
        );
    }

    #[test]
    fn no_order_sends_anyone_right_onto_a_tower() {
        let mut units = [
            unit_at(-8.0, 60.0, UnitKind::Vehicle),
            enemy_at(0.0, 60.0, UnitKind::Tower),
        ];
        units[0].selected = true;
        order_move(&mut units, (0.0, 60.0), open_ground(), &TWO_SIDES);
        assert_ne!(units[0].destination, (0.0, 60.0));
    }

    #[test]
    fn a_towers_two_tubes_stand_side_by_side_ahead_of_it_where_it_is_aimed() {
        let mut tower = unit_at(0.0, 60.0, UnitKind::Tower);
        tower.heading = 0.0;
        let first = tower.muzzle();
        tower.second_tube = true;
        let second = tower.muzzle();
        assert!(first.0 > 0.0 && second.0 > 0.0, "ahead of it, to the east");
        assert_eq!(first.0, second.0);
        assert!((first.2 - second.2).abs() > 0.2, "side by side");
    }

    fn produced(role: Role) -> Unit {
        let mut units = Vec::new();
        assert!(produce(
            &mut units,
            open_ground(),
            role,
            (0.0, 60.0),
            (0, TeamColor::Blue),
            99
        ));
        units[0]
    }

    #[test]
    fn production_makes_each_role_with_its_own_equipment_and_names() {
        let expected = [
            (Role::Scout, WeaponKind::Unarmed, "Scout"),
            (Role::LightInfantry, WeaponKind::Bullet, "Light Infantry"),
            (Role::Sniper, WeaponKind::Sniper, "Sniper"),
            (Role::Engineer, WeaponKind::Pistol, "Engineer"),
            (Role::KInfantry, WeaponKind::Pistol, "K Infantry"),
            (Role::Tank, WeaponKind::Cannon, "Tank"),
            (Role::RepairVehicle, WeaponKind::Repair, "Repair Vehicle"),
            (Role::Catapult, WeaponKind::Mortar, "Catapult"),
            (
                Role::AirDefensePlatform,
                WeaponKind::AutoGun,
                "Air Defense Platform",
            ),
            (Role::AirDrone, WeaponKind::AirMissile, "Air Drone"),
            (Role::FlameInfantry, WeaponKind::Flame, "Flame Infantry"),
            (Role::GasInfantry, WeaponKind::Gas, "Gas Infantry"),
        ];
        for (role, weapon, name) in expected {
            let unit = produced(role);
            assert_eq!((unit.weapon, unit.name()), (weapon, name));
            assert_eq!(unit.health, unit.max_health());
            assert_eq!((unit.id, unit.owner), (99, 0));
        }
    }

    #[test]
    fn production_never_stacks_units_on_each_other() {
        let mut units = Vec::new();
        for id in 0..12 {
            assert!(produce(
                &mut units,
                open_ground(),
                Role::Tank,
                (0.0, 60.0),
                (0, TeamColor::Blue),
                id
            ));
        }
        assert!(
            largest_overlap(&units) <= 0.0,
            "{}",
            largest_overlap(&units)
        );
    }

    #[test]
    fn the_platform_and_the_drone_stay_in_the_air() {
        for role in [Role::AirDefensePlatform, Role::AirDrone] {
            assert_eq!(role.kind(), UnitKind::Aircraft);
            assert_eq!(produced(role).altitude, CRUISE_ALTITUDE);
        }
    }

    #[test]
    fn the_drone_can_be_told_to_attack_only_aircraft_and_the_platform_anything() {
        let (plane, tank) = (
            enemy_at(1.0, 60.0, UnitKind::Aircraft),
            enemy_at(1.0, 60.0, UnitKind::Vehicle),
        );
        let (drone, platform) = (produced(Role::AirDrone), produced(Role::AirDefensePlatform));
        assert!(drone.can_attack(&plane, &TWO_SIDES));
        assert!(!drone.can_attack(&tank, &TWO_SIDES));
        assert!(platform.can_attack(&plane, &TWO_SIDES));
        assert!(platform.can_attack(&tank, &TWO_SIDES));
    }

    #[test]
    fn flame_and_gas_infantry_can_be_told_to_attack_the_ground_but_not_aircraft() {
        let (plane, tank) = (
            enemy_at(1.0, 60.0, UnitKind::Aircraft),
            enemy_at(1.0, 60.0, UnitKind::Vehicle),
        );
        for role in [Role::FlameInfantry, Role::GasInfantry] {
            let unit = produced(role);
            assert!(unit.can_attack(&tank, &TWO_SIDES), "{role:?}");
            assert!(!unit.can_attack(&plane, &TWO_SIDES), "{role:?}");
        }
    }

    #[test]
    fn scouts_run_fast_and_can_attack_nobody() {
        let scout = produced(Role::Scout);
        assert!(scout.role.speed() > Role::LightInfantry.speed());
        let enemy = enemy_at(1.0, 60.0, UnitKind::Infantry);
        assert!(!scout.can_attack(&enemy, &TWO_SIDES));
    }

    #[test]
    fn k_infantry_deploys_into_a_cannon_stays_put_and_can_pack_up_again() {
        let mut units = [produced(Role::KInfantry)];
        units[0].selected = true;
        toggle_deploy(&mut units);
        assert!(units[0].deployed);
        assert_eq!(units[0].weapon, WeaponKind::Cannon);

        let at = (units[0].x, units[0].z);
        order_move(&mut units, (10.0, 60.0), open_ground(), &[0]);
        for _ in 0..40 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert_eq!((units[0].x, units[0].z), at, "dug in");

        toggle_deploy(&mut units);
        assert_eq!(units[0].weapon, WeaponKind::Pistol);
        order_move(&mut units, (10.0, 60.0), open_ground(), &[0]);
        advance(&mut units, open_ground(), &[0], 0.05);
        assert_ne!((units[0].x, units[0].z), at, "on the move again");
    }

    #[test]
    fn deploying_stops_a_unit_and_only_selected_k_infantry_takes_the_order() {
        let mut units = [
            produced(Role::KInfantry),
            produced(Role::Sniper),
            produced(Role::KInfantry),
        ];
        units[0].selected = true;
        units[1].selected = true;
        units[0].destination = (10.0, 60.0);
        toggle_deploy(&mut units);
        assert!(!units[0].is_moving());
        assert!(!units[1].deployed);
        assert!(!units[2].deployed);
    }

    #[test]
    fn a_deployed_unit_told_to_attack_does_not_walk_off_after_its_target() {
        let mut units = [
            produced(Role::KInfantry),
            enemy_at(25.0, 60.0, UnitKind::Infantry),
        ];
        units[1].id = 7;
        units[0].selected = true;
        toggle_deploy(&mut units);
        order_attack(&mut units, 1, &TWO_SIDES);
        for _ in 0..40 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert_eq!(units[0].target, Some(Target::Unit(7)));
        assert!(!units[0].is_moving());
    }

    fn engineer_and_tower() -> [Unit; 2] {
        let mut engineer = produced(Role::Engineer);
        engineer.selected = true;
        engineer.x = -8.0;
        engineer.destination = (-8.0, engineer.z);
        let mut tower = enemy_at(0.0, engineer.z, UnitKind::Tower);
        tower.id = 7;
        [engineer, tower]
    }

    #[test]
    fn an_engineer_that_reaches_an_enemy_building_takes_it_over_in_its_own_color() {
        let mut units = engineer_and_tower();
        order_attack(&mut units, 1, &TWO_SIDES);
        for _ in 0..200 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert_eq!(units[1].owner, 0);
        assert_eq!(units[1].color, units[0].color);
        assert_eq!(units[0].target, None, "its job is done");
        assert!(!units[0].is_moving());
        assert_eq!(units[1].health, UnitKind::Tower.max_health(), "unharmed");
    }

    #[test]
    fn a_dead_engineer_captures_nothing_and_only_engineers_capture() {
        let mut units = engineer_and_tower();
        order_attack(&mut units, 1, &TWO_SIDES);
        units[0].alive = false;
        for _ in 0..200 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert_eq!(units[1].owner, 1);

        let mut units = engineer_and_tower();
        units[0].role = Role::LightInfantry;
        units[0].weapon = WeaponKind::Bullet;
        order_attack(&mut units, 1, &TWO_SIDES);
        for _ in 0..200 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert_eq!(units[1].owner, 1);
    }

    #[test]
    fn those_attacking_a_building_stop_when_it_changes_sides() {
        let mut gunner = produced(Role::LightInfantry);
        gunner.selected = true;
        gunner.x = -20.0;
        let mut tower = enemy_at(0.0, gunner.z, UnitKind::Tower);
        tower.id = 7;
        let mut units = [gunner, tower];
        order_attack(&mut units, 1, &TWO_SIDES);
        advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        assert!(units[0].is_moving());
        (units[1].owner, units[1].color) = (0, TeamColor::Blue);
        advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        assert_eq!(units[0].target, None);
        assert!(!units[0].is_moving());
    }

    #[test]
    fn a_unit_told_to_attack_a_place_closes_to_within_range_and_stays_on_the_order() {
        let mut units = [unit_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].selected = true;
        order_attack_place(&mut units, (20.0, 60.0));
        assert_eq!(units[0].target, Some(Target::Place(20.0, 60.0)));
        for _ in 0..400 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert!(!units[0].is_moving());
        let away = 20.0 - units[0].x;
        assert!(
            away <= WeaponKind::Bullet.range() && away > 3.0,
            "it did not go all the way: {away}"
        );
        assert_eq!(
            units[0].target,
            Some(Target::Place(20.0, 60.0)),
            "the order stands"
        );

        order_move(&mut units, (0.0, 60.0), open_ground(), &[0]);
        assert_eq!(units[0].target, None, "until a move order replaces it");
    }

    #[test]
    fn a_unit_already_in_range_of_a_place_does_not_move() {
        let mut units = [unit_at(0.0, 60.0, UnitKind::Infantry)];
        units[0].selected = true;
        order_attack_place(&mut units, (3.0, 60.0));
        advance(&mut units, open_ground(), &[0], 0.05);
        assert_eq!(units[0].x, 0.0);
    }

    #[test]
    fn those_unable_to_hit_the_ground_take_no_order_to_attack_a_place() {
        let mut scout = unit_at(0.0, 60.0, UnitKind::Infantry);
        scout.weapon = WeaponKind::Unarmed;
        let mut drone = unit_at(0.0, 62.0, UnitKind::Aircraft);
        drone.weapon = WeaponKind::AirMissile;
        let mut fixer = unit_at(0.0, 64.0, UnitKind::Vehicle);
        fixer.weapon = WeaponKind::Repair;
        let mut units = [scout, drone, fixer];
        for unit in &mut units {
            unit.selected = true;
        }
        order_attack_place(&mut units, (5.0, 60.0));
        assert!(units.iter().all(|u| u.target.is_none()));
    }

    fn with_ids<const N: usize>(mut units: [Unit; N]) -> [Unit; N] {
        for (i, unit) in units.iter_mut().enumerate() {
            unit.id = i as u32 + 1;
        }
        units
    }

    #[test]
    fn a_forced_target_is_remembered_even_if_it_is_a_friend_and_its_death_frees_the_unit() {
        let mut gunner = unit_at(0.0, 60.0, UnitKind::Infantry);
        gunner.selected = true;
        let mut units = with_ids([gunner, unit_at(20.0, 60.0, UnitKind::Infantry)]);
        order_force_attack(&mut units, 1, &[0]);
        assert_eq!(units[0].target, Some(Target::Forced(2)));
        for _ in 0..400 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert!(
            units[0].x > 10.0,
            "it went after its friend: {:?}",
            units[0]
        );
        assert_eq!(units[0].target, Some(Target::Forced(2)));

        units[1].alive = false;
        advance(&mut units, open_ground(), &[0], 0.05);
        assert_eq!(units[0].target, None, "free");
        assert!(!units[0].is_moving());
    }

    #[test]
    fn no_unit_is_forced_to_target_itself() {
        let mut gunner = unit_at(0.0, 60.0, UnitKind::Infantry);
        gunner.selected = true;
        let mut units = with_ids([gunner]);
        order_force_attack(&mut units, 0, &[0]);
        assert_eq!(units[0].target, None);
    }

    #[test]
    fn forcing_an_attack_on_an_enemy_is_an_ordinary_attack() {
        let mut gunner = unit_at(0.0, 60.0, UnitKind::Infantry);
        gunner.selected = true;
        let mut units = with_ids([gunner, enemy_at(5.0, 60.0, UnitKind::Infantry)]);
        order_force_attack(&mut units, 1, &TWO_SIDES);
        assert_eq!(units[0].target, Some(Target::Unit(2)));
    }

    fn fixer_at(x: f32, z: f32) -> Unit {
        let mut fixer = unit_at(x, z, UnitKind::Vehicle);
        fixer.role = Role::RepairVehicle;
        fixer.weapon = WeaponKind::Repair;
        fixer.selected = true;
        fixer
    }

    #[test]
    fn a_repair_vehicle_goes_to_a_friendly_vehicle_it_was_sent_to_and_its_death_frees_it() {
        let mut units = with_ids([fixer_at(0.0, 60.0), unit_at(20.0, 60.0, UnitKind::Vehicle)]);
        order_attack(&mut units, 1, &[0]);
        assert_eq!(units[0].target, Some(Target::Unit(2)));
        for _ in 0..400 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        let away = units[1].x - units[0].x;
        assert!(away <= WeaponKind::Repair.range(), "within reach: {away}");
        assert!(!units[0].is_moving());

        units[1].alive = false;
        advance(&mut units, open_ground(), &[0], 0.05);
        assert_eq!(units[0].target, None);
    }

    #[test]
    fn a_repair_vehicle_can_be_sent_only_to_friendly_vehicles_other_than_itself() {
        let ally = Unit {
            owner: 1,
            ..unit_at(5.0, 60.0, UnitKind::Vehicle)
        };
        let fixer = fixer_at(0.0, 60.0);
        let id = |mut unit: Unit| {
            unit.id = 9;
            unit
        };
        assert!(fixer.can_fix(&id(unit_at(5.0, 60.0, UnitKind::Vehicle)), &[0]));
        assert!(fixer.can_fix(&id(ally), &[0, 0]), "an ally");
        assert!(!fixer.can_fix(&id(ally), &TWO_SIDES), "an enemy");
        assert!(
            !fixer.can_fix(&id(unit_at(5.0, 60.0, UnitKind::Infantry)), &[0]),
            "infantry"
        );
        assert!(!fixer.can_fix(&fixer, &[0]), "itself");
    }

    #[test]
    fn a_repair_vehicle_fixes_aircraft_only_while_they_are_grounded() {
        let fixer = fixer_at(0.0, 60.0);
        let mut plane = unit_at(5.0, 60.0, UnitKind::Aircraft);
        plane.id = 9;
        assert!(!fixer.can_fix(&plane, &[0]), "in the air");
        plane.altitude = 0.0;
        assert!(fixer.can_fix(&plane, &[0]), "on the ground");
    }

    #[test]
    fn experience_gives_four_ranks_with_more_health_power_and_speed() {
        let mut unit = unit_at(0.0, 60.0, UnitKind::Vehicle);
        let (health, power, speed) = (unit.max_health(), unit.attack_power(), unit.speed());
        assert_eq!(unit.rank(), 0);
        for (rank, needed) in (0..).zip(RANK_EXPERIENCE) {
            unit.experience = needed - 1;
            assert_eq!(unit.rank(), rank, "not yet at {needed}");
            unit.experience = needed;
            assert_eq!(unit.rank(), rank + 1);
        }
        assert_eq!(unit.rank(), 3, "the highest");
        unit.experience = 1000;
        assert_eq!(unit.rank(), 3, "and no higher");
        assert!(unit.max_health() > health);
        assert!(unit.attack_power() > power);
        assert!(unit.speed() > speed);
    }

    #[test]
    fn a_new_rank_brings_its_extra_health_at_once() {
        let mut unit = unit_at(0.0, 60.0, UnitKind::Infantry);
        unit.health = 40.0;
        unit.gain_experience(1);
        assert_eq!(unit.health, 40.0, "no new rank yet");
        unit.gain_experience(1);
        assert_eq!(unit.rank(), 1);
        assert_eq!(unit.health, 40.0 + 100.0 * RANK_HEALTH);
    }

    #[test]
    fn experienced_units_get_there_faster() {
        let run_for = |experience: u32| {
            let mut units = [unit_at(-20.0, 60.0, UnitKind::Vehicle)];
            units[0].experience = experience;
            units[0].heading = 0.0;
            units[0].destination = (20.0, 60.0);
            for _ in 0..40 {
                advance(&mut units, open_ground(), &[], 0.05);
            }
            units[0].x
        };
        assert!(run_for(10) > run_for(0));
    }

    #[test]
    fn running_over_infantry_earns_experience() {
        let mut units = [
            unit_at(-3.0, 60.0, UnitKind::Vehicle),
            enemy_at(0.0, 60.0, UnitKind::Infantry),
        ];
        units[0].heading = 0.0;
        units[0].destination = (3.0, 60.0);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &TWO_SIDES, 0.05);
        }
        assert!(!units[1].alive);
        assert_eq!(units[0].experience, UnitKind::Infantry.kill_value());
    }

    #[test]
    fn a_unit_sees_farther_than_it_can_hit_and_scouts_see_farthest_of_the_infantry() {
        for role in [
            Role::LightInfantry,
            Role::Sniper,
            Role::Tank,
            Role::Catapult,
            Role::Aircraft,
            Role::Tower,
        ] {
            let unit = produced(role);
            assert!(unit.sight() > unit.weapon.range(), "{role:?}");
        }
        assert!(produced(Role::Scout).sight() > produced(Role::LightInfantry).sight());
    }

    #[test]
    fn enemies_are_told_apart_by_team() {
        assert!(are_enemies(&[0, 1, 2], 0, 1));
        assert!(!are_enemies(&[0, 1, 0], 0, 2), "the same team is allied");
        assert!(!are_enemies(&[0, 1], 1, 1));
        assert!(!are_enemies(&[], 0, 1), "no teams, no fights");
    }

    #[test]
    fn opponents_have_only_infantry_and_towers_and_the_player_has_vehicles_that_are_heavy() {
        let units = spawn(terrain(0), &COLORS);
        assert!(
            units
                .iter()
                .filter(|u| u.owner != 0)
                .all(|u| matches!(u.kind, UnitKind::Infantry | UnitKind::Tower))
        );
        assert!(
            units
                .iter()
                .any(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
        );
        assert!(
            units
                .iter()
                .all(|u| u.heavy == (u.kind == UnitKind::Vehicle))
        );
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
                let (near_x, near_z) =
                    nearest_on_footprint(&terrain.ruins()[0], units[0].x, units[0].z);
                let gap = (near_x - units[0].x).hypot(near_z - units[0].z);
                assert!(
                    gap > kind.radius() * 0.5,
                    "{kind:?} inside the ruin: {:?}",
                    units[0]
                );
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
            assert!(
                units[0].x < 4.0 && units[0].x > -4.0,
                "stays inside: {:?}",
                units[0]
            );
        }
        assert!(!units[0].is_moving(), "gave up: {:?}", units[0]);
    }

    #[test]
    fn orders_do_not_send_anyone_into_a_ruin() {
        let terrain = Terrain::open_with_ruins(vec![wall(0.0, 60.0, 1.0, 1.0, 1.0)]);
        let mut units = [unit_at(-8.0, 60.0, UnitKind::Vehicle)];
        units[0].selected = true;
        order_move(&mut units, (0.0, 60.0), &terrain, &[0]);
        let (near_x, near_z) = nearest_on_footprint(
            &terrain.ruins()[0],
            units[0].destination.0,
            units[0].destination.1,
        );
        let gap = (near_x - units[0].destination.0).hypot(near_z - units[0].destination.1);
        assert!(
            gap > UnitKind::Vehicle.radius(),
            "{:?}",
            units[0].destination
        );
    }

    #[test]
    fn heavy_units_may_be_ordered_right_onto_an_enemy() {
        let mut units = [
            unit_at(-8.0, 60.0, UnitKind::Vehicle),
            enemy_at(0.0, 60.0, UnitKind::Infantry),
        ];
        units[0].selected = true;
        order_move(&mut units, (0.0, 60.0), open_ground(), &TWO_SIDES);
        assert_eq!(units[0].destination, (0.0, 60.0));

        let mut units = [
            unit_at(-8.0, 60.0, UnitKind::Infantry),
            enemy_at(0.0, 60.0, UnitKind::Infantry),
        ];
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
        let mut units = [
            unit_at(0.0, 60.0, UnitKind::Vehicle),
            unit_at(-3.0, 60.0, UnitKind::Infantry),
        ];
        units[1].destination = (3.0, 60.0);
        for _ in 0..200 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert_eq!(
            (units[0].x, units[0].z),
            (0.0, 60.0),
            "the vehicle stays put"
        );
        assert!(
            !units[1].is_moving(),
            "the infantry walked around it: {:?}",
            units[1]
        );
    }

    #[test]
    fn real_maps_let_units_cross_them_among_the_ruins() {
        let map = terrain(0);
        let mut units = [
            unit_at(-30.0, 40.0, UnitKind::Infantry),
            unit_at(-30.0, 90.0, UnitKind::Vehicle),
        ];
        units[0].destination = (30.0, 40.0);
        units[1].destination = (30.0, 90.0);
        for _ in 0..1000 {
            advance(&mut units, map, &[0], 0.05);
            for unit in &units {
                for piece in map.ruins() {
                    let (near_x, near_z) = nearest_on_footprint(piece, unit.x, unit.z);
                    let dy =
                        unit.sphere_y() - unit.sphere_y().clamp(piece.y, piece.y + piece.height);
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
                let distance = (a.x - b.x)
                    .hypot(a.z - b.z)
                    .hypot(a.sphere_y() - b.sphere_y());
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
            assert!(
                (moved - 2.5 * 0.05).abs() < 0.001,
                "{destination:?}: moved {moved}"
            );
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
        assert!(
            units[0].heading.sin() > 0.9,
            "facing south: {}",
            units[0].heading
        );
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
            assert!(
                from_click < SLOT_SPACING * 3.0,
                "near the click: {from_click}"
            );
            for b in &units[i + 1..] {
                let apart =
                    (a.destination.0 - b.destination.0).hypot(a.destination.1 - b.destination.1);
                assert!(apart >= SLOT_SPACING - 0.01, "too close: {apart}");
            }
        }
    }

    #[test]
    fn an_order_keeps_off_spots_others_have_reserved() {
        let mut units = vec![
            unit_at(0.0, 60.0, UnitKind::Infantry),
            unit_at(2.0, 60.0, UnitKind::Infantry),
        ];
        units[0].destination = (0.0, 50.0);
        units[1].selected = true;
        order_move(&mut units, (0.0, 50.0), open_ground(), &[]);
        assert_ne!(units[1].destination, units[0].destination);
        let apart = (units[1].destination.0).hypot(units[1].destination.1 - 50.0);
        assert!(apart >= SLOT_SPACING - 0.01);
        assert_eq!(
            units[0].destination,
            (0.0, 50.0),
            "unselected units keep their orders"
        );
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
            assert!(
                !units[0].is_moving() && !units[1].is_moving(),
                "{kind:?}: {units:?}"
            );
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
            assert!(
                largest_overlap(&units) < 0.1,
                "overlap {}",
                largest_overlap(&units)
            );
        }
        let stuck: Vec<_> = units.iter().filter(|u| u.is_moving()).collect();
        assert!(stuck.is_empty(), "still going: {stuck:?}");
    }

    #[test]
    fn a_unit_pushed_aside_remembers_where_it_was_sent_and_walks_back() {
        // The second starts touching the first and sets off through it, so it must shove it.
        let mut units = [
            unit_at(0.0, 60.0, UnitKind::Infantry),
            unit_at(-0.3, 60.0, UnitKind::Vehicle),
        ];
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
        assert!(
            !units[0].is_moving(),
            "back where it was sent: {:?}",
            units[0]
        );
        assert!(
            !units[1].is_moving(),
            "and the vehicle got past: {:?}",
            units[1]
        );
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
                if !terrain.can_walk(x as f32, z as f32)
                    && (x as f32).abs() + (z as f32 - 64.0).abs() < 50.0
                {
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
        assert!(
            !units[0].is_moving(),
            "it does not press on forever: {:?}",
            units[0]
        );
    }

    #[test]
    fn everyone_gets_units_in_their_own_color_on_the_map() {
        let units = spawn(terrain(0), &COLORS);
        for (owner, &color) in COLORS.iter().enumerate() {
            let own: Vec<_> = units.iter().filter(|u| u.owner == owner).collect();
            assert!(!own.is_empty());
            assert!(own.iter().all(|u| u.color == color));
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
                assert!(
                    (a.x - b.x).hypot(a.z - b.z) > a.kind.radius() + b.kind.radius(),
                    "{a:?} and {b:?}"
                );
            }
        }
    }

    #[test]
    fn no_unit_starts_inside_a_ruin_on_any_map() {
        let colors = crate::game::TEAM_COLORS;
        for map in 0..crate::game::MAPS.len() {
            for count in 2..=colors.len() {
                let terrain = terrain(map);
                for unit in spawn(terrain, &colors[..count]) {
                    for piece in terrain.ruins() {
                        let gap_x = (unit.x - piece.x).abs() - unit.kind.radius() - piece.half_x;
                        let gap_z = (unit.z - piece.z).abs() - unit.kind.radius() - piece.half_z;
                        assert!(
                            gap_x > 0.0 || gap_z > 0.0,
                            "map {map}: {unit:?} in {piece:?}"
                        );
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
        assert!(
            in_selection(
                rectangle,
                &Unit {
                    x: 8.0,
                    z: 50.0,
                    ..unit
                }
            ),
            "on the border"
        );
        assert!(
            !in_selection(rectangle, &Unit { x: -3.1, ..unit }),
            "west of it"
        );
        assert!(
            !in_selection(rectangle, &Unit { x: 8.1, ..unit }),
            "east of it"
        );
        assert!(
            !in_selection(rectangle, &Unit { z: 49.9, ..unit }),
            "north of it"
        );
        assert!(
            !in_selection(rectangle, &Unit { z: 70.1, ..unit }),
            "south of it"
        );
    }

    #[test]
    fn a_units_height_does_not_change_whether_it_is_selected() {
        let rectangle = (-3.0, 50.0, 8.0, 70.0);
        let low = unit_at(0.0, 60.0, UnitKind::Infantry);
        assert_eq!(
            in_selection(rectangle, &low),
            in_selection(rectangle, &Unit { y: 0.7, ..low })
        );
    }

    #[test]
    fn lightning_starts_and_ends_exactly_on_the_edge_and_stays_near_it() {
        let (a, b) = ((100.0, 200.0), (700.0, 200.0));
        let points = lightning_points(a, b, 42, 30.0, 9.0);
        assert_eq!(points[0], a);
        assert_eq!(*points.last().unwrap(), b);
        assert!(points.len() > 10);
        assert!(points.iter().all(|p| (p.1 - 200.0).abs() <= 9.0));
        assert!(
            points.iter().any(|p| (p.1 - 200.0).abs() > 1.0),
            "it is jagged"
        );
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
