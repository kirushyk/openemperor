//! Placeholder units, buildings among them, and the geometry of selecting them.

#[cfg(test)]
use crate::battlefield::MAP_MIDDLE_Z;
use crate::battlefield::{
    CELL_SIDE, RuinPiece, START_SPOTS, Terrain, grid_to_world, next_random, start_spots,
    world_to_grid,
};
use crate::game::{Faction, TeamColor};
use crate::weapons::WeaponKind;
use std::cmp::Ordering;
use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, FRAC_PI_4, PI, TAU};

/// About how tall a unit stands, in world units.
pub const HEIGHT: f32 = 0.5;
#[cfg(test)]
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
const BLOCKED_PATIENCE: f32 = 8.0;
/// Getting this much nearer counts as progress.
const PROGRESS: f32 = 0.05;
/// How often a unit on its way is looked at, to see that it is getting somewhere.
const STALL_LOOK_SECONDS: f32 = 1.2;
/// How much of what it could cover between looks it must have covered.
const STALL_SHARE: f32 = 0.25;
/// How many looks in a row finding it has got nowhere before it gives up altogether.
const STALL_LOOKS_AT_MOST: u8 = 10;
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
/// How much farther something fires and sees for each world unit it stands above what it fires
/// on or the lowest ground, and the most height that counts.
const HIGH_GROUND_GAIN: f32 = 0.25;
const HIGH_GROUND_MOST: f32 = 1.6;

/// What standing this far above something multiplies how far it reaches by: nothing below.
pub fn high_ground(above: f32) -> f32 {
    1.0 + HIGH_GROUND_GAIN * above.clamp(0.0, HIGH_GROUND_MOST)
}

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
    /// A building that fights: it never moves, turns its two tubes by itself, and can be told
    /// what to fire on.
    Tower,
    /// A large building that makes units and fights no one.
    Building,
}

impl UnitKind {
    /// The radius of the sphere it collides with.
    pub fn radius(self) -> f32 {
        match self {
            UnitKind::Infantry => 0.25,
            UnitKind::Vehicle => 0.45,
            UnitKind::Aircraft => 0.4,
            UnitKind::Tower => 0.6,
            UnitKind::Building => BUILDING_RADIUS,
        }
    }

    pub fn is_building(self) -> bool {
        matches!(self, UnitKind::Tower | UnitKind::Building)
    }

    /// The experience its killer gets.
    pub fn kill_value(self) -> u32 {
        match self {
            UnitKind::Infantry => 1,
            UnitKind::Vehicle | UnitKind::Aircraft => 2,
            UnitKind::Tower | UnitKind::Building => 3,
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
            UnitKind::Building => 800.0,
        }
    }

    /// World units per second.
    fn speed(self) -> f32 {
        match self {
            UnitKind::Infantry => 2.5,
            UnitKind::Vehicle => 4.5,
            UnitKind::Aircraft => 7.0,
            UnitKind::Tower | UnitKind::Building => 0.0,
        }
    }
}

const BUILDING_RADIUS: f32 = 1.2;
/// The radius of an MCV on its tracks: far bigger than any other vehicle.
const MCV_RADIUS: f32 = 1.0;
/// How high the deck of a landing pad is, which is where a machine on it sits.
pub const PAD_TOP: f32 = 1.1;
/// How far a landing pad's deck reaches each way from its middle.
pub const PAD_DECK: (f32, f32) = (1.0, 0.68);
/// How long a landing pad takes to load one rocket, in seconds.
const RELOAD_SECONDS: f32 = 1.2;
/// How far from its pad a machine with rockets left no longer counts as using it.
const LEAVES_PAD: f32 = 1.5;
/// How close above its pad a machine has come to settle on it.
const OVER_PAD: f32 = 0.5;
/// How close to a pad a machine short of rockets gets a full one to make way, or goes to it.
const PAD_NEARBY: f32 = 10.0;
/// How far out, in rings so far apart, an MCV on the sand looks for upper ground to deploy on.
const DEPLOY_SEARCH_RINGS: usize = 60;
const DEPLOY_SEARCH_STEP: f32 = 1.2;
/// How far from its base's MCV a machine home from a mission sits down.
const HOME_BESIDE: f32 = 3.2;
/// How far out, in rings so far apart, a machine looks for higher ground to sit down on.
const HIGHER_GROUND_RINGS: usize = 10;
const HIGHER_GROUND_STEP: f32 = 1.5;
/// How far from a pad a machine waits for it to be free.
const PAD_BESIDE: f32 = 2.8;
/// How far from a pad a machine waiting for it no longer counts as beside it.
const PAD_WAITS: f32 = 4.5;
/// How far to one side of a group's middle its landing pad stands, and how far ahead of it.
#[cfg(test)]
const PAD_ALONG: f32 = -5.0;
#[cfg(test)]
const PAD_AHEAD: f32 = 2.2;
/// How far to the other side its MCV stands.
#[cfg(test)]
const MCV_ALONG: f32 = 5.4;
/// How much of its health something must have lost to smoke, to show black stains, and to burn.
pub const SMOKES_PAST: f32 = 0.25;
pub const STAINS_PAST: f32 = 0.4;
pub const BURNS_PAST: f32 = 0.7;
/// How many units a building can have waiting behind the one it is making.
const QUEUE_ROOM: usize = 30;
/// How far beyond its side the door of a building is.
const DOOR_MARGIN: f32 = 0.2;
/// How far out of a factory's door a new vehicle drives.
const DRIVE_OUT: f32 = 2.5;
/// How far behind a group's middle its buildings stand, and how far apart.
#[cfg(test)]
const BUILDINGS_BEHIND: f32 = 4.8;
#[cfg(test)]
const BUILDING_SPREAD: f32 = 4.6;
/// How high aircraft fly, in world units.
const CRUISE_ALTITUDE: f32 = 4.0;
/// World units per second that whatever lands goes up and comes down.
const CLIMB_SPEED: f32 = 2.5;
/// How far from where it was sent something that never stops circles.
const CIRCLE_RADIUS: f32 = 3.5;
/// How near right above a vehicle a carryall must be to pick it up.
const LIFT_REACH: f32 = 0.3;
/// World units per second a carried vehicle is pulled up or let down, and how high it hangs.
const WINCH_SPEED: f32 = 3.0;
const HANG_HEIGHT: f32 = CRUISE_ALTITUDE - 0.6;
/// How long an enemy vehicle fights a carryall that has hold of it before it can be lifted.
pub const STRUGGLE_SECONDS: f32 = 4.0;
/// Radians it leans into the sharpest turn, and how quickly it gets there, per second.
const MOST_BANK: f32 = 0.6;
const BANK_RATE: f32 = 4.0;
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
    /// The huge main vehicle on four tracks. It deploys into the building that lets its owner
    /// build buildings, and packs up again to drive somewhere else.
    Mcv,
    /// Lobs shells in a ballistic arc.
    Catapult,
    /// Fires a salvo of rockets from its two racks, then reloads for a long time.
    RocketLauncher,
    /// Walks on two legs and fires missiles.
    Mongoose,
    /// Walks on two big legs and fires its four cannons in turn, then has to calm down.
    Minotaurus,
    /// Hangs in the air, unarmed, and carries friendly vehicles to where it is told.
    AdvancedCarryall,
    /// Hangs in the air and fires an automatic gun at the air and the ground alike.
    AirDefensePlatform,
    /// A delta wing that can never stop: it circles where it was sent, and fires its missiles
    /// at enemy aircraft only.
    AirDrone,
    /// Sits on the ground until sent somewhere: it goes up, turns, flies there and sits again.
    Ornithopter,
    /// A heavier, slower ornithopter whose rockets do far more harm.
    Destroyer,
    /// House H's rifleman: tougher than light infantry.
    RegularInfantry,
    /// House H's soldier with a shoulder launcher firing missiles.
    RocketInfantry,
    /// House O's: unarmed, and deploying sets off a huge explosion that takes it and everyone
    /// around with it.
    SuicideInfantry,
    /// House O's: a pistol on the move, and deployed a mortar lobbing shells in an arc.
    MortarInfantry,
    /// House A's fast bike: a flat teardrop with a rifle that only really hurts infantry.
    SandBike,
    /// House H's: a huge spinning disk saw in front that cuts down infantry it reaches, and
    /// twin rifles.
    DiskSaw,
    /// House H's tank with two flamethrowers, at ten and at two o'clock.
    FlameTank,
    /// House H's huge tank with twin cannons.
    HugeWalker,
    /// House O's quick scouting machine on four long insect legs.
    Recon,
    /// House O's laser tank, gliding over the ground: it goes any way at once, turning slowly
    /// towards where it is going.
    LaserTank,
    /// House O's tank on two tracks with a cannon longer than itself. Deployed, it stands
    /// still and raises the cannon on a leg to shell what is far off.
    SiegeTank,
    /// Sprays fire at the ground in front of it.
    FlameInfantry,
    /// Sprays green gas at the ground in front of it.
    GasInfantry,
    Tower,
    /// A tower that throws fire.
    FlameTower,
    /// A tower with two machine guns side by side, firing fast at the ground and the air.
    GunTower,
    /// A short-reaching post with one machine gun, for what is on the ground.
    GunPost,
    /// A low round turret that pops up to throw cisterns of gas at infantry.
    GasTower,
    /// A low round turret nobody sees until it fires, popping up with a quick laser.
    LaserTower,
    /// Sees the whole map with its radar, and shows enemies on the minimap.
    Outpost,
    /// Makes infantry.
    Barracks,
    /// Makes ground vehicles.
    Factory,
    /// Makes aircraft.
    Hangar,
    /// A deck where one ornithopter at a time lands to load rockets.
    LandingPad,
    /// A house's own seat of power, without which its superweapon cannot fire.
    Palace(Faction),
}

/// The grid squares a building takes up, lined up with the map's diamond: `cols` squares along
/// the grid axis running south-east and `rows` along the one running south-west.
/// Which of the map's four edges the front of a building looks towards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Facing {
    #[default]
    LeftBottom,
    LeftTop,
    RightTop,
    RightBottom,
}

impl Facing {
    /// The one a drag across the screen points to.
    pub fn of_drag(dx: f32, dy: f32) -> Self {
        match (dx < 0.0, dy < 0.0) {
            (true, false) => Facing::LeftBottom,
            (true, true) => Facing::LeftTop,
            (false, true) => Facing::RightTop,
            (false, false) => Facing::RightBottom,
        }
    }

    /// How far a building's model is turned, from how it is made, to look this way.
    pub fn turn(self) -> f32 {
        let quarters = match self {
            Facing::LeftBottom => 0.0,
            Facing::LeftTop => 1.0,
            Facing::RightTop => 2.0,
            Facing::RightBottom => 3.0,
        };
        FRAC_PI_4 + quarters * FRAC_PI_2
    }

    /// The way out of the front of a building, on the ground.
    pub fn way_out(self) -> (f32, f32) {
        let (sin, cos) = self.turn().sin_cos();
        (-sin, cos)
    }

    /// Whether a building's width and depth are swapped on the grid.
    fn swaps_sides(self) -> bool {
        matches!(self, Facing::LeftTop | Facing::RightBottom)
    }
}

/// How far along getting ready to work an MCV is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Deployment {
    #[default]
    Idle,
    /// Coming round to face the map's left-bottom edge.
    Turning,
    /// Going down on its legs inside its scaffolding, boring its drill in.
    Drilling,
}

/// How an MCV's drill stands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrillPose {
    /// How far its nose is tipped down from level, in radians.
    pub tilt: f32,
    /// How far it has been driven along its length, from 0 to 1.
    pub plunge: f32,
    /// How far it has turned about its length, in radians.
    pub spin: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plot {
    pub u: i32,
    pub v: i32,
    pub cols: i32,
    pub rows: i32,
}

impl Plot {
    /// How far along a line between two grid points it first comes to these squares, as a
    /// share of the line; `None` if it never does, or starts inside them.
    pub fn entered_by(self, from: (f32, f32), to: (f32, f32)) -> Option<f32> {
        let (low_u, high_u) = (self.u as f32, (self.u + self.cols) as f32);
        let (low_v, high_v) = (self.v as f32, (self.v + self.rows) as f32);
        if (low_u..high_u).contains(&from.0) && (low_v..high_v).contains(&from.1) {
            return None;
        }
        let slab = |start: f32, way: f32, low: f32, high: f32| {
            if way.abs() < 1e-6 {
                ((low..high).contains(&start)).then_some((0.0, 1.0))
            } else {
                let (a, b) = ((low - start) / way, (high - start) / way);
                Some((a.min(b), a.max(b)))
            }
        };
        let (enter_u, leave_u) = slab(from.0, to.0 - from.0, low_u, high_u)?;
        let (enter_v, leave_v) = slab(from.1, to.1 - from.1, low_v, high_v)?;
        let (enter, leave) = (enter_u.max(enter_v).max(0.0), leave_u.min(leave_v).min(1.0));
        (enter <= leave).then_some(enter)
    }

    /// Each of its squares, by the grid corner nearest the map's north.
    pub fn cells(self) -> impl Iterator<Item = (i32, i32)> {
        (self.v..self.v + self.rows)
            .flat_map(move |v| (self.u..self.u + self.cols).map(move |u| (u, v)))
    }

    /// Where its middle is in the world.
    pub fn center(self) -> (f32, f32) {
        grid_to_world(
            self.u as f32 + self.cols as f32 / 2.0,
            self.v as f32 + self.rows as f32 / 2.0,
        )
    }

    fn contains(self, (u, v): (i32, i32)) -> bool {
        (self.u..self.u + self.cols).contains(&u) && (self.v..self.v + self.rows).contains(&v)
    }

    /// How many empty squares lie between it and another, along whichever axis has the most.
    fn gap_to(self, other: Plot) -> i32 {
        let gap = |a: i32, a_size: i32, b: i32, b_size: i32| {
            (b - (a + a_size)).max(a - (b + b_size)).max(0)
        };
        gap(self.u, self.cols, other.u, other.cols).max(gap(self.v, self.rows, other.v, other.rows))
    }
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
            | Role::GasInfantry
            | Role::RegularInfantry
            | Role::RocketInfantry
            | Role::SuicideInfantry
            | Role::MortarInfantry => UnitKind::Infantry,
            Role::Tank
            | Role::RepairVehicle
            | Role::Mcv
            | Role::SandBike
            | Role::DiskSaw
            | Role::FlameTank
            | Role::HugeWalker
            | Role::Recon
            | Role::LaserTank
            | Role::SiegeTank
            | Role::Catapult
            | Role::RocketLauncher
            | Role::Mongoose
            | Role::Minotaurus => UnitKind::Vehicle,
            Role::AdvancedCarryall
            | Role::AirDefensePlatform
            | Role::AirDrone
            | Role::Ornithopter
            | Role::Destroyer => UnitKind::Aircraft,
            Role::Tower
            | Role::FlameTower
            | Role::GunTower
            | Role::GunPost
            | Role::GasTower
            | Role::LaserTower => UnitKind::Tower,
            Role::Barracks
            | Role::Factory
            | Role::Hangar
            | Role::LandingPad
            | Role::Outpost
            | Role::Palace(_) => UnitKind::Building,
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
            Role::Mcv => "MCV",
            Role::Catapult => "Catapult",
            Role::RocketLauncher => "Rocket Launcher",
            Role::Mongoose => "Mongoose",
            Role::Minotaurus => "Minotaurus",
            Role::AdvancedCarryall => "Advanced Carryall",
            Role::AirDefensePlatform => "Air Defense Platform",
            Role::AirDrone => "Air Drone",
            Role::Ornithopter => "Ornithopter",
            Role::Destroyer => "Destroyer",
            Role::FlameInfantry => "Flame Infantry",
            Role::RegularInfantry => "Regular Infantry",
            Role::RocketInfantry => "Rocket Infantry",
            Role::SuicideInfantry => "Suicide Infantry",
            Role::MortarInfantry => "Mortar Infantry",
            Role::SandBike => "Sand Bike",
            Role::DiskSaw => "Disk Saw",
            Role::FlameTank => "Flame Tank",
            Role::HugeWalker => "Huge Walker",
            Role::Recon => "Recon",
            Role::LaserTank => "Laser Tank",
            Role::SiegeTank => "Siege Tank",
            Role::GasInfantry => "Gas Infantry",
            Role::Tower => "Rocket Tower",
            Role::FlameTower => "Flame Tower",
            Role::GunTower => "Machine Gun Tower",
            Role::GunPost => "Machine Gun Post",
            Role::GasTower => "Gas Tower",
            Role::LaserTower => "Laser Tower",
            Role::Outpost => "Outpost",
            Role::Barracks => "Barracks",
            Role::Factory => "Factory",
            Role::Hangar => "Hangar",
            Role::LandingPad => "Landing Pad",
            Role::Palace(Faction::A) => "Palace A",
            Role::Palace(Faction::H) => "Palace H",
            Role::Palace(Faction::O) => "Palace O",
        }
    }

    fn weapon(self) -> WeaponKind {
        match self {
            Role::LightInfantry
            | Role::RegularInfantry
            | Role::SandBike
            | Role::DiskSaw
            | Role::Recon => WeaponKind::Bullet,
            Role::LaserTank => WeaponKind::Laser,
            Role::SiegeTank => WeaponKind::Cannon,
            Role::RocketInfantry => WeaponKind::Missile,
            Role::MortarInfantry => WeaponKind::Pistol,
            Role::FlameTank => WeaponKind::Flame,
            Role::HugeWalker => WeaponKind::Plasma,
            Role::Scout
            | Role::Outpost
            | Role::Mcv
            | Role::SuicideInfantry
            | Role::Barracks
            | Role::Factory
            | Role::Hangar
            | Role::LandingPad
            | Role::Palace(_) => WeaponKind::Unarmed,
            Role::Sniper => WeaponKind::Sniper,
            Role::Engineer | Role::KInfantry => WeaponKind::Pistol,
            Role::Tank => WeaponKind::Cannon,
            Role::RepairVehicle => WeaponKind::Repair,
            Role::Catapult => WeaponKind::Mortar,
            Role::RocketLauncher | Role::Ornithopter => WeaponKind::Rockets,
            Role::Destroyer => WeaponKind::HeavyRockets,
            Role::Mongoose => WeaponKind::Missile,
            Role::Tower => WeaponKind::TowerRockets,
            Role::Minotaurus => WeaponKind::Bombard,
            Role::AdvancedCarryall => WeaponKind::Unarmed,
            Role::AirDefensePlatform => WeaponKind::AutoGun,
            Role::GunTower => WeaponKind::TowerGun,
            Role::GunPost => WeaponKind::PostGun,
            Role::GasTower => WeaponKind::Cistern,
            Role::LaserTower => WeaponKind::RapidLaser,
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
            Role::KInfantry | Role::RegularInfantry => 120.0,
            Role::SuicideInfantry => 80.0,
            Role::SandBike => 160.0,
            Role::DiskSaw => 340.0,
            Role::FlameTank => 320.0,
            Role::HugeWalker => 2400.0,
            Role::Recon => 150.0,
            Role::LaserTank => 260.0,
            Role::SiegeTank => 380.0,
            Role::RepairVehicle => 200.0,
            Role::Mcv => 1200.0,
            Role::Catapult | Role::RocketLauncher => 180.0,
            Role::Mongoose => 220.0,
            Role::Minotaurus => 450.0,
            Role::AirDefensePlatform => 200.0,
            Role::AirDrone => 80.0,
            Role::Destroyer => 320.0,
            Role::LandingPad => 400.0,
            Role::Palace(_) => 1200.0,
            Role::FlameTower => 500.0,
            Role::GunPost | Role::GasTower | Role::LaserTower => 450.0,
            _ => self.kind().max_health(),
        }
    }

    fn speed(self) -> f32 {
        match self {
            Role::Scout => 4.0,
            Role::Mcv => 1.8,
            Role::SandBike => 6.5,
            Role::HugeWalker => 0.9,
            Role::Recon => 6.0,
            Role::LaserTank => 5.5,
            Role::SiegeTank => 3.0,
            Role::SuicideInfantry => 3.0,
            Role::Catapult | Role::RocketLauncher => 3.5,
            Role::Mongoose => 4.0,
            Role::Minotaurus => 2.8,
            Role::AirDefensePlatform => 3.0,
            Role::AirDrone => 8.0,
            Role::Ornithopter => 5.5,
            Role::Destroyer => 3.2,
            _ => self.kind().speed(),
        }
    }

    /// How long it takes its building to make it, in seconds.
    pub fn build_seconds(self) -> f32 {
        match self {
            Role::Scout => 3.0,
            Role::LightInfantry | Role::Engineer => 4.0,
            Role::FlameInfantry
            | Role::GasInfantry
            | Role::RegularInfantry
            | Role::SuicideInfantry => 5.0,
            Role::RocketInfantry | Role::MortarInfantry => 6.0,
            Role::SandBike => 6.0,
            Role::DiskSaw | Role::FlameTank => 10.0,
            Role::HugeWalker => 45.0,
            Role::Recon => 6.0,
            Role::LaserTank => 11.0,
            Role::SiegeTank => 16.0,
            Role::Sniper | Role::KInfantry => 6.0,
            Role::RepairVehicle => 7.0,
            Role::Tank | Role::Catapult | Role::Mongoose | Role::AirDrone => 9.0,
            Role::RocketLauncher | Role::Ornithopter | Role::AirDefensePlatform => 11.0,
            Role::AdvancedCarryall => 12.0,
            Role::Minotaurus | Role::Destroyer => 15.0,
            Role::Mcv => 25.0,
            Role::LandingPad => 10.0,
            Role::Palace(_) => 30.0,
            Role::Outpost => 20.0,
            Role::Barracks => 15.0,
            Role::Factory | Role::Hangar => 20.0,
            Role::Tower | Role::GunTower | Role::LaserTower => 12.0,
            Role::GunPost | Role::FlameTower | Role::GasTower => 8.0,
        }
    }

    /// Whether it is a building that makes units.
    pub fn makes_units(self) -> bool {
        matches!(self, Role::Barracks | Role::Factory | Role::Hangar)
    }

    /// Whether it can dig in where it stands.
    pub fn can_deploy(self) -> bool {
        matches!(
            self,
            Role::KInfantry
                | Role::MortarInfantry
                | Role::SuicideInfantry
                | Role::SiegeTank
                | Role::HugeWalker
                | Role::Mcv
        )
    }

    /// Whether a building of this house can make it. Every barracks makes scouts and
    /// engineers, every factory MCVs and rocket launchers, which no house has been given yet,
    /// and every hangar the same aircraft.
    pub fn made_by(self, house: Faction) -> bool {
        match self {
            Role::GunPost | Role::Tower => house == Faction::A,
            Role::FlameTower | Role::GunTower => house == Faction::H,
            Role::GasTower | Role::LaserTower => house == Faction::O,
            Role::LightInfantry | Role::KInfantry | Role::Sniper => house == Faction::A,
            Role::RegularInfantry | Role::RocketInfantry | Role::FlameInfantry => {
                house == Faction::H
            }
            Role::GasInfantry | Role::SuicideInfantry | Role::MortarInfantry => house == Faction::O,
            Role::SandBike | Role::RepairVehicle | Role::Mongoose | Role::Minotaurus => {
                house == Faction::A
            }
            Role::Recon | Role::LaserTank | Role::SiegeTank => house == Faction::O,
            Role::Palace(of) => house == of,
            Role::Tank | Role::DiskSaw | Role::FlameTank | Role::HugeWalker | Role::Catapult => {
                house == Faction::H
            }
            _ => true,
        }
    }

    /// The armed towers a house builds, the lesser tier first.
    pub fn towers_of(house: Faction) -> [Role; 2] {
        match house {
            Faction::A => [Role::GunPost, Role::Tower],
            Faction::H => [Role::FlameTower, Role::GunTower],
            Faction::O => [Role::GasTower, Role::LaserTower],
        }
    }

    /// Whether it lies sunk in the ground until it has something to fire at.
    pub fn pops_up(self) -> bool {
        matches!(self, Role::GasTower | Role::LaserTower)
    }

    /// The radius of the sphere it collides with, as it comes out.
    pub fn radius(self) -> f32 {
        match self {
            Role::Mcv => MCV_RADIUS,
            Role::HugeWalker => 1.0,
            Role::Palace(Faction::H) => 2.6,
            Role::Outpost => 1.5,
            Role::SiegeTank => 0.55,
            Role::Barracks => 2.0,
            Role::Factory | Role::Hangar => 2.2,
            Role::Recon => 0.4,
            Role::DiskSaw => 0.5,
            Role::SandBike => 0.35,
            _ => self.kind().radius(),
        }
    }

    /// How many grid squares a building takes up along the map's two grid axes.
    pub fn plot_size(self) -> (i32, i32) {
        match self {
            Role::Barracks => (6, 4),
            Role::Factory | Role::Hangar => (6, 5),
            Role::Palace(Faction::H) => (6, 6),
            Role::Palace(_) => (4, 3),
            Role::Outpost => (4, 4),
            Role::LandingPad => (3, 2),
            Role::Mcv => (3, 3),
            _ => (2, 2),
        }
    }

    /// The squares it would take up with its middle as near this place as the grid allows,
    /// looking the usual way.
    pub fn plot_at(self, at: (f32, f32)) -> Plot {
        self.plot_facing(at, Facing::default())
    }

    /// The squares it would take up with its middle as near this place as the grid allows,
    /// looking this way.
    pub fn plot_facing(self, (x, z): (f32, f32), facing: Facing) -> Plot {
        let (cols, rows) = self.plot_size();
        let (cols, rows) = if facing.swaps_sides() {
            (rows, cols)
        } else {
            (cols, rows)
        };
        let (u, v) = world_to_grid(x, z);
        Plot {
            u: (u - cols as f32 / 2.0).round() as i32,
            v: (v - rows as f32 / 2.0).round() as i32,
            cols,
            rows,
        }
    }

    /// Whether this square of its plot is where what it makes comes out: the middle of the
    /// side its door is on.
    pub fn is_exit(self, plot: Plot, facing: Facing, (u, v): (i32, i32)) -> bool {
        let middle_u = plot.u + plot.cols / 2;
        let middle_v = plot.v + plot.rows / 2;
        let along_u = (middle_u - 1..=middle_u).contains(&u);
        let along_v = (middle_v - 1..=middle_v).contains(&v);
        self.makes_units()
            && match facing {
                Facing::LeftBottom => v == plot.v + plot.rows - 1 && along_u,
                Facing::RightTop => v == plot.v && along_u,
                Facing::RightBottom => u == plot.u + plot.cols - 1 && along_v,
                Facing::LeftTop => u == plot.u && along_v,
            }
    }

    /// How many times its basic model it is drawn.
    pub fn drawn_scale(self) -> f32 {
        match self {
            Role::Barracks | Role::Factory | Role::Hangar => 1.9,
            Role::AdvancedCarryall => 2.0,
            _ => 1.0,
        }
    }

    /// Whether it runs on wheels or tracks, and so raises dust on sand.
    pub fn rolls(self) -> bool {
        self.kind() == UnitKind::Vehicle
            && !matches!(
                self,
                Role::Mongoose
                    | Role::Minotaurus
                    | Role::HugeWalker
                    | Role::Recon
                    | Role::LaserTank
            )
    }

    /// How high a building reaches, as it is drawn.
    pub fn building_top(self) -> f32 {
        match self {
            Role::Barracks => 1.3,
            Role::Factory => 2.85,
            Role::Hangar => 1.65,
            Role::Palace(Faction::H) => 5.9,
            Role::Palace(_) => 2.2,
            Role::Outpost => 2.0,
            Role::GasTower | Role::LaserTower => 0.8,
            Role::LandingPad => PAD_TOP + 0.6,
            Role::Mcv => 2.4,
            _ => TUBE_HEIGHT + 0.4,
        }
    }

    /// Whether it sits on the ground when it has nowhere to go.
    pub fn lands(self) -> bool {
        matches!(self, Role::Ornithopter | Role::Destroyer)
    }

    /// How many rockets it can carry, if it must go back to a landing pad for more.
    pub fn rocket_load(self) -> Option<u8> {
        match self {
            Role::Ornithopter => Some(12),
            Role::Destroyer => Some(6),
            _ => None,
        }
    }

    /// Whether it must keep flying forward, unable to hang in the air.
    fn never_stops(self) -> bool {
        self == Role::AirDrone
    }

    /// Its height above the ground when it first appears.
    fn starting_altitude(self) -> f32 {
        if self.lands() {
            0.0
        } else {
            self.kind().altitude()
        }
    }

    /// The building that makes it, if anything does.
    pub fn made_in(self) -> Option<Role> {
        match self.kind() {
            UnitKind::Infantry => Some(Role::Barracks),
            UnitKind::Vehicle => Some(Role::Factory),
            UnitKind::Aircraft => Some(Role::Hangar),
            UnitKind::Tower | UnitKind::Building => None,
        }
    }

    #[cfg(test)]
    fn for_kind(kind: UnitKind) -> Self {
        match kind {
            UnitKind::Infantry => Role::LightInfantry,
            UnitKind::Vehicle => Role::Tank,
            UnitKind::Aircraft => Role::AdvancedCarryall,
            UnitKind::Tower => Role::Tower,
            UnitKind::Building => Role::Barracks,
        }
    }
}

/// Radians per second a vehicle can turn.
const TURN_SPEED: f32 = 3.0;
/// Radians per second a laser tank's nose comes round to the way it glides.
const GLIDER_TURN_SPEED: f32 = 1.0;
/// How far a recon's legs step for each radian it turns on the spot.
const LEGS_PER_TURN: f32 = 0.45;
/// Which place for a base holds the player's second group in a battle set up for tests: off to
/// the east, a long scroll from the first.
#[cfg(test)]
const SECOND_GROUP_SPOT: usize = 2;
/// A block of the player's this deep has a row of aircraft.
#[cfg(test)]
const ROWS_WITH_AIRCRAFT: usize = 4;
/// How far to the side of its group's middle the player's tower stands.
#[cfg(test)]
const PLAYER_TOWER_OFFSET: f32 = -3.4;
/// How far to the side of its group's middle an opponent's tower stands. Not so far that it
/// leaves the land kept clear around the ring.
#[cfg(test)]
const OPPONENT_TOWER_OFFSET: f32 = 3.5;
/// How high a tower's tubes are.
pub const TUBE_HEIGHT: f32 = 1.26;
/// How far ahead of a tower's middle its tubes end, and how far apart they stand from it.
const TUBE_REACH: f32 = 0.7;
/// How far ahead of a tower's middle its tubes are hinged, and how long they are from there.
pub const TUBE_HINGE: f32 = 0.1;
pub const TUBE_LENGTH: f32 = TUBE_REACH - TUBE_HINGE;
pub const TUBE_SPREAD: f32 = 0.18;
/// How far a pop-up turret's middle rises out of its base to fire.
pub const TURRET_RISE: f32 = 0.32;
/// How high a pop-up turret's gun is while it is sunk.
const TURRET_MUZZLE_UP: f32 = 0.2;
/// Where a huge walker's plasma guns end, as (ahead, up, to the side).
pub const WALKER_PLASMA_TIP: (f32, f32, f32) = (1.45, 2.05, 0.82);
/// Where its rocket tubes end, and how far to each side the three of them are.
pub const WALKER_ROCKET_AHEAD: f32 = 0.05;
pub const WALKER_ROCKET_UP: f32 = 2.82;
pub const WALKER_ROCKET_SIDES: [f32; 3] = [-0.24, 0.0, 0.24];
/// Where a factory's exhaust pipe stands on it, as (ahead, to the side), and how high it ends.
pub const FACTORY_PIPE: [f32; 2] = [0.6, -0.4];
pub const FACTORY_PIPE_TOP: f32 = 1.5;
/// Where a rocket launcher's rockets lie from its middle, in the order they are fired: the left
/// rack's three, then the right rack's.
pub const ROCKET_SIDES: [f32; 6] = [-0.21, -0.14, -0.07, 0.07, 0.14, 0.21];
/// How high a rocket launcher's rockets lie, and how far ahead of its middle they end.
pub const ROCKET_HEIGHT: f32 = 0.35;
const ROCKET_REACH: f32 = 0.3;
/// Where a Heavy Strider's cannons lie from its middle, in the order they fire.
pub const CANNON_SIDES: [f32; 4] = [-0.3, -0.14, 0.14, 0.3];
/// How high its cannons are, and how far ahead of its middle they end.
pub const CANNON_HEIGHT: f32 = 1.05;
pub const CANNON_REACH: f32 = 1.45;
/// Seconds a building takes to go up once it has been placed.
pub const CONSTRUCTION_SECONDS: f32 = 1.0;
/// The way an MCV faces to deploy: the map's left-bottom edge.
const MCV_DEPLOY_HEADING: f32 = 3.0 * FRAC_PI_4;
/// Where an MCV's drill is hinged, as [along, up].
pub const MCV_DRILL_PIVOT: [f32; 2] = [1.0, 0.6];
/// How far through its construction the scaffolding and cranes are up and work starts, and
/// how far when work is over and they start to come down.
pub const CONSTRUCTION_UP: f32 = 0.25;
pub const CONSTRUCTION_DOWN: f32 = 0.75;
/// How many empty squares may lie between a new building and the nearest of its owner's.
pub const BUILD_REACH: i32 = 2;
/// How many places on a building going up are being welded at once.
pub const WELD_POINTS: usize = 4;

/// Where a siege tank's cannon tip is, as [along, up], on its tracks and raised when deployed.
pub const SIEGE_TIP: [f32; 2] = [1.05, 0.5];
pub const SIEGE_RAISED_TIP: [f32; 2] = [0.97, 2.1];
/// How high a Strider's missiles leave from.
const STRIDER_MUZZLE_HEIGHT: f32 = 0.86;

/// What a unit was told to go after.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// By id: an enemy to fight, or for a repair vehicle a friendly vehicle to look after.
    Unit(u32),
    /// By id: any unit at all it was forced to fire on, friendly or not.
    Forced(u32),
    /// A place on the map (x, z) to fire at until told otherwise.
    Place(f32, f32),
    /// By id: a hurt friend a repair vehicle went to by itself, until it is mended.
    Rescue(u32),
    /// By id: infantry attacking a vehicle with no other answer to it, to drive over.
    RunOver(u32),
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
    /// The great house that built it, which decides what a building of it can make and how a
    /// house's own machines look.
    pub house: Faction,
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
    /// How far a missile tower's tubes are raised, in radians: below level if negative.
    pub tube_pitch: f32,
    /// What a building is making and the seconds left: one thing at a time.
    pub making: Option<(Role, f32)>,
    /// What it makes after that, in order, with the empty places at the end.
    queue: [Option<Role>; QUEUE_ROOM],
    /// Where a building that makes units sends them, if the player has said.
    pub gathering: Option<(f32, f32)>,
    /// How many shots of its salvo have gone: they stay gone until it has reloaded.
    pub salvo_fired: usize,
    /// How far back each of a Heavy Strider's cannons has recoiled, from 1 when it fires to 0.
    pub recoil: [f32; 4],
    /// How far it has walked, which is what moves a strider's legs.
    pub stride: f32,
    /// How far it leans into a turn along its length, in radians: positive to its right.
    pub bank: f32,
    /// Whether what sits on the ground has an enemy near enough to go up and fire on.
    alerted: bool,
    /// Whether a carryall holds a vehicle.
    pub carrying: bool,
    /// Whether a carryall is to put down what it holds when it gets where it is going.
    unloading: bool,
    blocked_for: f32,
    /// Its nearest approach to the destination since it set off.
    best_remaining: f32,
    /// Where it heads for first, to get around what lies between it and its destination.
    way: Option<Way>,
    /// How many of the rockets it carries are gone.
    pub rockets_fired: u8,
    /// The landing pad it is going to or sitting on.
    pad: Option<Pad>,
    /// Seconds an invisible unit stays seen after it last fired.
    pub revealed: f32,
    /// How far a pop-up turret has come up: 0 sunk in the ground, 1 ready to fire.
    pub popped: f32,
    /// Seconds left until a building just placed is finished, and works.
    pub constructing: f32,
    /// Which way a building looks.
    pub facing: Facing,
    /// How far along an MCV is with deploying.
    pub deployment: Deployment,
    /// Whether what lands has had something to go after since it was last sent anywhere: once
    /// that is over, it goes home instead of sitting down where it is.
    on_mission: bool,
    /// How far along a pad is with loading its next rocket, in seconds.
    reloading: f32,
    /// The enemy carryall that has hold of it, if one has.
    pub theft: Option<Theft>,
    /// How it is getting on with going somewhere.
    stall: Stall,
}

impl Unit {
    /// Whether it still has somewhere to go, or can never stop anyway.
    pub fn is_moving(&self) -> bool {
        self.alive && self.is_under_way()
    }

    /// Whether it was on its way somewhere, alive or just shot down.
    pub fn is_under_way(&self) -> bool {
        self.remaining() > ARRIVED || self.role.never_stops()
    }

    /// How many of this role it is making or has waiting.
    pub fn orders_for(&self, role: Role) -> usize {
        let making = self.making.iter().filter(|(making, _)| *making == role);
        let waiting = self.queue.iter().flatten().filter(|&&waits| waits == role);
        making.count() + waiting.count()
    }

    /// How many units it has waiting behind the one it is making.
    fn waiting(&self) -> usize {
        self.queue.iter().flatten().count()
    }

    /// Moves on to the next unit waiting, if there is one.
    fn start_next(&mut self) {
        self.making = self.queue[0].map(|role| (role, role.build_seconds()));
        self.queue.rotate_left(1);
        self.queue[QUEUE_ROOM - 1] = None;
    }

    /// Whether it has a fight to be up in the air for: an order, or an enemy nearby.
    fn has_a_fight(&self) -> bool {
        self.target.is_some() || self.alerted
    }

    /// Whether it is up in the air but would rather sit, having nothing to go after.
    fn is_settling(&self) -> bool {
        self.alive
            && self.role.lands()
            && self.altitude > 0.0
            && !self.is_on_pad()
            && !self.has_a_fight()
            && !self.is_moving()
    }

    /// Whether it is low down with a fight within reach, and must go up to fire.
    fn is_rising_to_fight(&self) -> bool {
        self.alive
            && self.role.lands()
            && self.altitude < CRUISE_ALTITUDE
            && self.has_a_fight()
            && !self.is_moving()
    }

    /// Whether it is high enough to fire. What lands cannot launch anything until it is up.
    pub fn can_launch(&self) -> bool {
        !self.role.lands() || self.altitude >= CRUISE_ALTITUDE
    }

    /// Whether it has no need of a spot on the ground: what lands needs one, and so does
    /// whatever has a vehicle to put down.
    fn keeps_to_the_air(&self) -> bool {
        self.kind.flies() && !self.role.lands() && !self.carrying
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
            (_, UnitKind::Tower | UnitKind::Building) => TOWER_SIGHT,
        };
        own.max(self.weapon.range() + 1.0)
    }

    /// How far it sees from where it stands: farther the higher it is above the lowest ground.
    pub fn sight_over(&self, floor: f32) -> f32 {
        self.sight() * high_ground(self.y - floor)
    }

    /// How far it can fire at what stands at this height: farther down onto lower ground.
    pub fn range_down_to(&self, height: f32) -> f32 {
        self.weapon.range() * high_ground(self.y - height)
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

    pub fn speed(&self) -> f32 {
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
        self.kind == UnitKind::Vehicle || (self.kind.flies() && self.is_grounded())
    }

    /// Whether it is down: on the ground, or for what lands, sitting on the deck of a pad.
    pub fn is_grounded(&self) -> bool {
        self.altitude <= 0.0 || (self.role.lands() && self.is_on_pad())
    }

    /// How high it counts as being for what can reach it: what is down is on the ground.
    pub fn height_as_target(&self) -> f32 {
        if self.is_grounded() {
            0.0
        } else {
            self.altitude
        }
    }

    /// Forgets any target and stays where it is.
    fn stand_down(&mut self) {
        self.target = None;
        self.destination = (self.x, self.z);
    }

    fn reset_progress(&mut self) {
        self.stall = Stall::NOT_STALLED;
        self.blocked_for = 0.0;
        self.best_remaining = f32::INFINITY;
    }

    /// The radius of the sphere it collides with: a deployed MCV's is a building's.
    pub fn radius(&self) -> f32 {
        if self.kind == self.role.kind() {
            self.role.radius()
        } else {
            self.kind.radius()
        }
    }

    /// Whether it can make this: a building makes the units made in it, and a deployed MCV
    /// makes buildings and towers.
    pub fn can_make(&self, role: Role) -> bool {
        self.alive
            && !self.is_under_construction()
            && role.made_by(self.house)
            && match role.made_in() {
                Some(maker) => self.role == maker,
                None => self.role == Role::Mcv && self.deployed,
            }
    }

    /// Sits it on a landing pad right under it.
    #[cfg(test)]
    pub fn sit_on_pad(&mut self, id: u32) {
        self.pad = Some(Pad {
            id,
            x: self.x,
            z: self.z,
            deck: FRAC_PI_4,
        });
        self.altitude = PAD_TOP;
    }

    /// Whether it is a building still going up.
    pub fn is_under_construction(&self) -> bool {
        self.constructing > 0.0
    }

    /// How far along its construction is, from 0 to 1.
    pub fn construction_done(&self) -> f32 {
        1.0 - (self.constructing / CONSTRUCTION_SECONDS).clamp(0.0, 1.0)
    }

    /// How much of its height is up, from 0 to 1: it only grows once the scaffolding and
    /// cranes are up, and is whole before they come down.
    pub fn raised(&self) -> f32 {
        // An MCV deploys whole, inside its scaffolding.
        if self.role == Role::Mcv {
            return 1.0;
        }
        let work = CONSTRUCTION_DOWN - CONSTRUCTION_UP;
        let t = ((self.construction_done() - CONSTRUCTION_UP) / work).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Where a building going up is being welded, climbing as it rises.
    pub fn weld_points(&self) -> [Point; WELD_POINTS] {
        let (cols, rows) = self.role.plot_size();
        let reach = cols.min(rows) as f32 * CELL_SIDE * 0.42;
        let rise = self.raised() * self.role.building_top();
        std::array::from_fn(|i| {
            let (sin, cos) = (self.id as f32 * 2.4 + i as f32 * TAU / 4.0).sin_cos();
            let height = 0.1 + (0.25 + 0.2 * i as f32) * rise;
            (self.x + reach * cos, self.y + height, self.z + reach * sin)
        })
    }

    /// Where the ends of its wings are, leaning with it into its turns.
    pub fn wing_tips(&self) -> [Point; 2] {
        const WING_END: f32 = 0.5;
        const WING_HEIGHT: f32 = 0.1;
        let size = self.role.drawn_scale();
        let (sin, cos) = self.heading.sin_cos();
        let (bank_sin, bank_cos) = self.bank.sin_cos();
        let height = self.y + self.altitude + WING_HEIGHT * size;
        [-1.0, 1.0].map(|side| {
            let out = side * WING_END * size;
            (
                self.x - sin * out * bank_cos,
                height - out * bank_sin,
                self.z + cos * out * bank_cos,
            )
        })
    }

    /// How an MCV's drill stands: level until it deploys, then tipped down to 45 degrees and
    /// bored in, turning all the while.
    pub fn drill_pose(&self) -> DrillPose {
        const TILTED_BY: f32 = 0.4;
        const TURNS: f32 = 5.0;
        let smooth = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let done = if self.deployed {
            1.0
        } else if self.deployment == Deployment::Drilling {
            self.construction_done()
        } else {
            0.0
        };
        DrillPose {
            tilt: smooth(done / TILTED_BY) * FRAC_PI_4,
            plunge: smooth((done - TILTED_BY) / (1.0 - TILTED_BY)),
            spin: done * TURNS * TAU,
        }
    }

    /// Whether an MCV's drill is biting into the ground now.
    pub fn drill_is_biting(&self) -> bool {
        const TOUCHES_AT: f32 = 0.2;
        self.alive
            && self.deployment == Deployment::Drilling
            && self.construction_done() >= TOUCHES_AT
    }

    /// Where an MCV's drill goes into the ground.
    pub fn drill_bite(&self) -> Point {
        let [along, up] = MCV_DRILL_PIVOT;
        let reach = along + up;
        let (sin, cos) = self.heading.sin_cos();
        (self.x + cos * reach, self.y + 0.05, self.z + sin * reach)
    }

    /// The grid squares it takes up, if it is a building.
    pub fn plot(&self) -> Option<Plot> {
        self.kind
            .is_building()
            .then(|| self.role.plot_facing((self.x, self.z), self.facing))
    }

    /// Whether it can go invisible: a scout, and a sniper with all three medals.
    pub fn can_cloak(&self) -> bool {
        self.role == Role::Scout
            || self.role == Role::LaserTower
            || (self.role == Role::Sniper && self.rank() >= 3)
    }

    /// Whether it is invisible now: it shows for a while each time it opens fire.
    pub fn is_cloaked(&self) -> bool {
        self.alive && self.can_cloak() && self.revealed <= 0.0
    }

    /// Whether this owner cannot see it at all: an invisible unit is hidden from its enemies,
    /// though it can still be caught in a blast or driven over.
    pub fn hidden_from(&self, owner: usize, teams: &[usize]) -> bool {
        self.is_cloaked() && are_enemies(teams, owner, self.owner)
    }

    /// Tells it to go after a unit by id, as a click on that unit would.
    pub fn go_after(&mut self, id: u32) {
        if self.target != Some(Target::Unit(id)) {
            set_target(self, Target::Unit(id));
        }
    }

    /// Whether it has something to fire: everything does but a machine that has run out of the
    /// rockets it carries.
    pub fn has_rockets(&self) -> bool {
        self.rockets_left().is_none_or(|left| left > 0)
    }

    /// How many rockets it has left, if it is the kind that runs out.
    pub fn rockets_left(&self) -> Option<u8> {
        let load = self.role.rocket_load()?;
        Some(load.saturating_sub(self.rockets_fired))
    }

    /// Whether it is sitting on its landing pad.
    fn is_on_pad(&self) -> bool {
        self.pad.is_some_and(|pad| {
            (pad.x - self.x).hypot(pad.z - self.z) < OVER_PAD && self.altitude <= PAD_TOP + 0.01
        })
    }

    /// How much of its health it has lost, if it is the kind to show that: a vehicle, an
    /// aircraft or a building. Otherwise 0.
    pub fn wear(&self) -> f32 {
        if self.kind == UnitKind::Infantry || !self.alive {
            0.0
        } else {
            (1.0 - self.health / self.max_health()).clamp(0.0, 1.0)
        }
    }

    /// How far past dead it was hit, as a share of all the health it can have.
    pub fn overkill(&self) -> f32 {
        (-self.health / self.max_health()).max(0.0)
    }

    /// Whether it can be forced to fire on another unit, whoever that is.
    pub fn can_fire_on(&self, other: &Unit) -> bool {
        self.has_rockets()
            && self.alive
            && other.alive
            && self.id != other.id
            && self.weapon.goes_for(other.kind)
            && self.weapon.can_target(other.height_as_target())
    }

    /// Whether it can be told to fire at a place on the ground.
    pub fn can_fire_at_ground(&self) -> bool {
        self.alive && self.weapon.can_target(0.0)
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

    /// Whether it could pick this vehicle up: a carryall with nothing in hand can take
    /// friendly vehicles.
    pub fn can_lift(&self, other: &Unit, teams: &[usize]) -> bool {
        self.role == Role::AdvancedCarryall
            && !self.carrying
            && self.alive
            && other.alive
            && other.kind == UnitKind::Vehicle
            && are_friends(teams, self.owner, other.owner)
    }

    /// Whether it would go to another unit to fix it or pick it up.
    pub fn can_tend(&self, other: &Unit, teams: &[usize]) -> bool {
        self.can_fix(other, teams) || self.can_lift(other, teams) || self.can_steal(other, teams)
    }

    /// Whether it could take hold of this enemy vehicle and carry it off.
    pub fn can_steal(&self, other: &Unit, teams: &[usize]) -> bool {
        self.role == Role::AdvancedCarryall
            && !self.carrying
            && self.alive
            && other.alive
            && other.kind == UnitKind::Vehicle
            && other.theft.is_none()
            && are_enemies(teams, self.owner, other.owner)
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
            && !other.hidden_from(self.owner, teams)
            && self.has_rockets()
            && are_enemies(teams, self.owner, other.owner)
            && self.weapon.goes_for(other.kind)
            && self.weapon.can_target(other.height_as_target())
    }

    /// Whether it opens fire on another unit in range unprompted: what it can attack, and an
    /// enemy scout too, which nobody can point out to it but which does not get past its guns.
    pub fn fires_on_sight(&self, other: &Unit, teams: &[usize]) -> bool {
        self.can_attack(other, teams)
            || (other.role == Role::Scout
                && are_enemies(teams, self.owner, other.owner)
                && self.can_fire_on(other))
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
        self.y + self.altitude + self.radius()
    }

    /// Its collision sphere's center, where shots aim.
    pub fn center(&self) -> (f32, f32, f32) {
        (self.x, self.sphere_y(), self.z)
    }

    /// Where its shots come from: a tower takes turns with the ends of its two tubes or guns,
    /// and a rocket launcher goes along its racks.
    pub fn muzzle(&self) -> (f32, f32, f32) {
        if self.role.pops_up() {
            let (sin, cos) = self.heading.sin_cos();
            let up = TURRET_MUZZLE_UP + TURRET_RISE * self.popped;
            return (self.x + cos * 0.45, self.y + up, self.z + sin * 0.45);
        }
        if self.role == Role::SiegeTank {
            let [ahead, up] = if self.deployed {
                SIEGE_RAISED_TIP
            } else {
                SIEGE_TIP
            };
            let (sin, cos) = self.heading.sin_cos();
            return (self.x + cos * ahead, self.y + up, self.z + sin * ahead);
        }
        if self.role == Role::LaserTank {
            // The crystal on top of its dome.
            return (self.x, self.y + 0.54, self.z);
        }
        if self.role == Role::Mongoose {
            return (self.x, self.y + STRIDER_MUZZLE_HEIGHT, self.z);
        }
        if self.role == Role::Minotaurus {
            let (sin, cos) = self.heading.sin_cos();
            let side = CANNON_SIDES.get(self.salvo_fired).copied().unwrap_or(0.0);
            return (
                self.x + cos * CANNON_REACH - sin * side,
                self.y + CANNON_HEIGHT,
                self.z + sin * CANNON_REACH + cos * side,
            );
        }
        if self.role == Role::RocketLauncher {
            let (sin, cos) = self.heading.sin_cos();
            let side = ROCKET_SIDES.get(self.salvo_fired).copied().unwrap_or(0.0);
            return (
                self.x + cos * ROCKET_REACH - sin * side,
                self.y + ROCKET_HEIGHT,
                self.z + sin * ROCKET_REACH + cos * side,
            );
        }
        if self.kind.is_building() {
            let (sin, cos) = self.heading.sin_cos();
            let side = match (self.role, self.second_tube) {
                (Role::FlameTower | Role::GunPost | Role::GasTower | Role::LaserTower, _) => 0.0,
                (_, true) => TUBE_SPREAD,
                (_, false) => -TUBE_SPREAD,
            };
            let (rise, out) = self.tube_pitch.sin_cos();
            let reach = TUBE_HINGE + TUBE_LENGTH * out;
            return (
                self.x + cos * reach - sin * side,
                self.y + TUBE_HEIGHT + TUBE_LENGTH * rise,
                self.z + sin * reach + cos * side,
            );
        }
        (self.x, self.y + self.altitude + HEIGHT * 0.8, self.z)
    }

    /// Where a sprayer's nozzle pointing this way from its nose is: a flame tank has one at
    /// each front corner, and anyone else sprays from its muzzle.
    pub fn nozzle(&self, way: f32) -> (f32, f32, f32) {
        if self.role != Role::FlameTank {
            return self.muzzle();
        }
        let (sin, cos) = (self.heading + way).sin_cos();
        (self.x + cos * 0.5, self.y + 0.3, self.z + sin * 0.5)
    }

    /// Where a huge walker's rockets leave its rack, one to each tube.
    pub fn walker_rocket_tubes(&self) -> [(f32, f32, f32); 3] {
        let (sin, cos) = self.heading.sin_cos();
        WALKER_ROCKET_SIDES.map(|side| {
            (
                self.x + cos * WALKER_ROCKET_AHEAD - sin * side,
                self.y + WALKER_ROCKET_UP,
                self.z + sin * WALKER_ROCKET_AHEAD + cos * side,
            )
        })
    }

    /// Where the smoke comes out of a factory's exhaust pipe, wherever it was turned.
    pub fn exhaust_tip(&self) -> (f32, f32, f32) {
        let size = self.role.drawn_scale();
        let (sin, cos) = self.facing.turn().sin_cos();
        let [ahead, across] = FACTORY_PIPE;
        (
            self.x + (ahead * cos - across * sin) * size,
            self.y + FACTORY_PIPE_TOP * size,
            self.z + (ahead * sin + across * cos) * size,
        )
    }

    /// Where gunfire comes out: at the end of the barrel, outside its body.
    pub fn gun_tip(&self) -> (f32, f32, f32) {
        if matches!(self.role, Role::GunTower | Role::GunPost | Role::SiegeTank) {
            return self.muzzle();
        }
        // Twin guns side by side, firing in turn.
        let twin = match self.role {
            Role::DiskSaw => Some((0.5, 0.36, 0.3)),
            Role::HugeWalker => Some(WALKER_PLASMA_TIP),
            _ => None,
        };
        if let Some((ahead, up, side)) = twin {
            let side = if self.second_tube { side } else { -side };
            let (sin, cos) = self.heading.sin_cos();
            return (
                self.x + cos * ahead - sin * side,
                self.y + up,
                self.z + sin * ahead + cos * side,
            );
        }
        let base = self.y + self.altitude;
        let (ahead, up) = match (self.role, self.deployed) {
            (Role::KInfantry, true) => (0.75, 0.18),
            (Role::KInfantry, false) => (0.2, 0.42),
            (Role::Sniper, _) => (0.56, 0.42),
            (Role::SandBike, _) => (0.5, 0.2),
            (Role::AirDefensePlatform, _) => (0.5, 0.175),
            _ if self.kind == UnitKind::Infantry => (0.33, 0.4),
            _ if self.kind == UnitKind::Vehicle => (0.65, 0.375),
            _ => (0.3, HEIGHT * 0.8),
        };
        let (sin, cos) = self.heading.sin_cos();
        (self.x + cos * ahead, base + up, self.z + sin * ahead)
    }

    /// The height on screen that a click aims at.
    pub fn pick_height(&self) -> f32 {
        let above_feet = if self.kind.is_building() {
            TUBE_HEIGHT * 0.7
        } else {
            match self.role {
                Role::Mongoose => 0.5,
                Role::Minotaurus => 0.9,
                Role::HugeWalker => 2.0,
                _ => HEIGHT / 2.0,
            }
        };
        self.y + self.altitude + above_feet
    }

    /// Whether the player can select it, and so give it orders.
    pub fn is_selectable(&self) -> bool {
        self.owner == 0 && self.theft.is_none()
    }

    /// Whether a box or the select-all key takes it. Buildings that make units are only ever
    /// picked by a single click of their own.
    pub fn is_selected_in_groups(&self) -> bool {
        self.is_selectable() && self.kind != UnitKind::Building
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
            house: Faction::A,
            selected: false,
            kind,
            heading: -FRAC_PI_2,
            destination: (x, z),
            target: None,
            heavy: kind == UnitKind::Vehicle,
            alive: true,
            second_tube: false,
            tube_pitch: 0.0,
            making: None,
            queue: [None; QUEUE_ROOM],
            gathering: None,
            salvo_fired: 0,
            recoil: [0.0; 4],
            stride: 0.0,
            bank: 0.0,
            alerted: false,
            carrying: false,
            unloading: false,
            blocked_for: 0.0,
            best_remaining: f32::INFINITY,
            way: None,
            rockets_fired: 0,
            pad: None,
            on_mission: false,
            revealed: 0.0,
            popped: 0.0,
            constructing: 0.0,
            facing: Facing::default(),
            deployment: Deployment::Idle,
            reloading: 0.0,
            theft: None,
            stall: Stall::NOT_STALLED,
        }
    }
}

/// What a block's infantry carries: the player's mixes the three small arms by column, so
/// that all of them are on the field, and opponents' have bullets.
#[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
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
        altitude: role.starting_altitude(),
        x,
        y: terrain.height_at(x, z),
        z,
        owner,
        color,
        house: Faction::A,
        selected: false,
        kind,
        heading: -FRAC_PI_2,
        destination: (x, z),
        target: None,
        heavy: kind == UnitKind::Vehicle,
        alive: true,
        second_tube: false,
        tube_pitch: 0.0,
        making: None,
        queue: [None; QUEUE_ROOM],
        gathering: None,
        salvo_fired: 0,
        recoil: [0.0; 4],
        stride: 0.0,
        bank: 0.0,
        alerted: false,
        carrying: false,
        unloading: false,
        blocked_for: 0.0,
        best_remaining: f32::INFINITY,
        way: None,
        rockets_fired: 0,
        pad: None,
        on_mission: false,
        revealed: 0.0,
        popped: 0.0,
        constructing: 0.0,
        facing: Facing::default(),
        deployment: Deployment::Idle,
        reloading: 0.0,
        theft: None,
        stall: Stall::NOT_STALLED,
    });
}

#[cfg(test)]
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

/// Places around a building where others must not be sent, so that nobody presses against it.
fn building_footprint(building: &Unit) -> [(f32, f32); 8] {
    std::array::from_fn(|i| {
        let (sin, cos) = (i as f32 / 8.0 * TAU).sin_cos();
        let reach = building.radius() / 2.0;
        (building.x + reach * cos, building.z + reach * sin)
    })
}

/// The building of this owner that can make this role, if they have one.
#[cfg(test)]
pub fn maker_of(units: &[Unit], role: Role, owner: usize) -> Option<&Unit> {
    role.made_in()?;
    units.iter().find(|u| u.owner == owner && u.can_make(role))
}

/// Adds a new unit for an owner, if they have the building that makes it and there is a free
/// spot by its door, on its south side. Returns whether it did. Infantry and aircraft appear
/// at that spot, and a vehicle comes out of the door and drives there.
#[cfg(test)]
fn produce(units: &mut Vec<Unit>, terrain: &Terrain, role: Role, owner: usize, id: u32) -> bool {
    let Some(&maker) = maker_of(units, role, owner) else {
        return false;
    };
    produce_at(units, terrain, role, &maker, id)
}

/// Whether an owner has a building of the right kind with nothing to do.
pub fn has_free_maker(units: &[Unit], role: Role, owner: usize) -> bool {
    role.made_in().is_some()
        && units
            .iter()
            .any(|u| u.owner == owner && u.can_make(role) && u.making.is_none())
}

/// Gives a role to the owner's building of the right kind that has the least to do: at once if
/// it is free, otherwise at the end of its queue. Returns whether one could take it.
pub fn start_making(units: &mut [Unit], role: Role, owner: usize) -> bool {
    if role.made_in().is_none() {
        return false;
    }
    let least_busy = units
        .iter_mut()
        .filter(|u| u.owner == owner && u.can_make(role))
        .min_by_key(|u| (u.making.is_some(), u.waiting()));
    let Some(building) = least_busy else {
        return false;
    };
    if building.making.is_none() {
        building.making = Some((role, role.build_seconds()));
        return true;
    }
    let Some(place) = building.queue.iter_mut().find(|place| place.is_none()) else {
        return false;
    };
    *place = Some(role);
    true
}

/// Calls off one order of a role from the owner's building with the most waiting: the last of
/// them waiting, wherever it is in the queue, or else the one being made, and then the next
/// one starts. Returns whether there was one.
pub fn cancel_making(units: &mut [Unit], role: Role, owner: usize) -> bool {
    let building = units
        .iter_mut()
        .filter(|u| u.alive && u.owner == owner && u.orders_for(role) > 0)
        .max_by_key(|u| u.orders_for(role));
    let Some(building) = building else {
        return false;
    };
    match building
        .queue
        .iter()
        .rposition(|&place| place == Some(role))
    {
        Some(place) => {
            building.queue[place..].rotate_left(1);
            building.queue[QUEUE_ROOM - 1] = None;
        }
        None => building.start_next(),
    }
    true
}

/// Counts down what every building is making. Returns the buildings done with theirs.
pub fn work_on_making(units: &mut [Unit], dt: f32) -> Vec<usize> {
    let mut done = Vec::new();
    for (i, unit) in units.iter_mut().enumerate().filter(|(_, u)| u.alive) {
        if let Some((_, left)) = &mut unit.making {
            *left = (*left - dt).max(0.0);
            if *left <= 0.0 {
                done.push(i);
            }
        }
    }
    done
}

/// Lets a building's finished unit out. Returns the new unit's index, or nothing if there is no
/// room for it yet, in which case it stays finished and tries again.
pub fn finish_making(
    units: &mut Vec<Unit>,
    terrain: &Terrain,
    index: usize,
    id: u32,
) -> Option<usize> {
    let maker = units[index];
    let (role, _) = maker.making?;
    if !produce_at(units, terrain, role, &maker, id) {
        return None;
    }
    units[index].start_next();
    Some(units.len() - 1)
}

fn produce_at(units: &mut Vec<Unit>, terrain: &Terrain, role: Role, maker: &Unit, id: u32) -> bool {
    let owner = maker.owner;
    // Out along the grid, the way the building's door faces.
    let (out_x, out_z) = maker.facing.way_out();
    let out = maker.radius() + role.radius() + DOOR_MARGIN;
    let door = (maker.x + out_x * out, maker.z + out_z * out);
    let vehicle = role.kind() == UnitKind::Vehicle;
    let near = if vehicle {
        (door.0 + out_x * DRIVE_OUT, door.1 + out_z * DRIVE_OUT)
    } else {
        door
    };
    let taken = spots_taken(units, false);
    let Some(&spot) = find_slots(terrain, near, 1, &taken, false).first() else {
        return false;
    };
    let side = (owner, maker.color);
    let appears_at = if vehicle { door } else { spot };
    // Sent on to the gathering point if there is one, to a spot there of its own.
    let flying = role.kind().flies() && !role.lands();
    let gathers_at = maker.gathering.and_then(|point| {
        let taken = spots_taken(units, flying);
        find_slots(terrain, point, 1, &taken, flying)
            .first()
            .copied()
    });
    place_unit(units, terrain, appears_at, role, role.weapon(), side, id);
    if let Some(new) = units.last_mut() {
        new.house = maker.house;
        if vehicle {
            new.heading = out_z.atan2(out_x);
        }
        if let Some(way) = gathers_at.or(vehicle.then_some(spot)) {
            new.destination = way;
        }
    }
    true
}

/// Sends a unit to its own free spot near a point, if there is one.
pub fn send_to_spot(units: &mut [Unit], index: usize, point: (f32, f32), terrain: &Terrain) {
    let flying = units[index].keeps_to_the_air();
    let taken = spots_taken(units, flying);
    if let Some(&spot) = find_slots(terrain, point, 1, &taken, flying).first() {
        units[index].destination = spot;
    }
}

/// Where a new unit must not go: where everyone in its layer stands or is headed, and around
/// the buildings.
fn spots_taken(units: &[Unit], flying: bool) -> Vec<(f32, f32)> {
    units
        .iter()
        .filter(|u| u.alive && u.keeps_to_the_air() == flying)
        .flat_map(|u| {
            let around = u.kind.is_building().then(|| building_footprint(u));
            [(u.x, u.z), u.destination]
                .into_iter()
                .chain(around.into_iter().flatten())
        })
        .collect()
}

/// Whether a building's footprint here lies all on the upper ground, which can be walked on:
/// nothing is built on the sand, or the lower ground that stands for it on other planets.
fn footprint_on_upper_ground(terrain: &Terrain, (x, z): (f32, f32)) -> bool {
    let radius = BUILDING_RADIUS;
    [
        (0.0, 0.0),
        (-radius, 0.0),
        (radius, 0.0),
        (0.0, -radius),
        (0.0, radius),
    ]
    .iter()
    .all(|(dx, dz)| terrain.can_walk(x + dx, z + dz) && terrain.on_upper_ground(x + dx, z + dz))
}

/// Whether it could deploy where it stands: an MCV only on the upper ground, as a building.
pub fn can_deploy_here(unit: &Unit, terrain: &Terrain) -> bool {
    unit.deployed || unit.role != Role::Mcv || footprint_on_upper_ground(terrain, (unit.x, unit.z))
}

/// The nearest place around a point where an MCV could deploy, ring by ring.
pub fn deploy_spot_near(terrain: &Terrain, (x, z): (f32, f32)) -> Option<(f32, f32)> {
    (0..=DEPLOY_SEARCH_RINGS).find_map(|ring| {
        let reach = ring as f32 * DEPLOY_SEARCH_STEP;
        let tries = (6 * ring).max(1);
        (0..tries).find_map(|n| {
            let angle = n as f32 / tries as f32 * TAU;
            let at = (x + reach * angle.cos(), z + reach * angle.sin());
            footprint_on_upper_ground(terrain, at).then_some(at)
        })
    })
}

/// Whether a building could stand on this grid square: on the upper ground, clear of ruins, of
/// other buildings' squares and of everyone on the ground.
pub fn cell_is_free(units: &[Unit], terrain: &Terrain, (u, v): (i32, i32)) -> bool {
    let (x, z) = grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
    let half = CELL_SIDE / std::f32::consts::SQRT_2;
    let on_land = [
        (0.0, 0.0),
        (-half, 0.0),
        (half, 0.0),
        (0.0, -half),
        (0.0, half),
    ]
    .iter()
    .all(|(dx, dz)| terrain.can_walk(x + dx, z + dz) && terrain.on_upper_ground(x + dx, z + dz));
    let clear_of_ruins = terrain.ruins().iter().all(|piece| {
        let (near_x, near_z) = nearest_on_footprint(piece, x, z);
        (near_x - x).hypot(near_z - z) > half
    });
    on_land
        && clear_of_ruins
        && units
            .iter()
            .filter(|unit| unit.alive && unit.altitude <= 0.0)
            .all(|unit| match unit.plot() {
                Some(plot) => !plot.contains((u, v)),
                None => (unit.x - x).hypot(unit.z - z) > unit.radius() + half * 0.7,
            })
}

/// Whether a new building could go up on these squares, all of them being free.
pub fn can_place_building(units: &[Unit], terrain: &Terrain, plot: Plot) -> bool {
    plot.cells().all(|cell| cell_is_free(units, terrain, cell))
}

/// Whether these squares are close enough to one of this owner's buildings to build on.
pub fn is_near_building_of(units: &[Unit], plot: Plot, owner: usize) -> bool {
    units
        .iter()
        .filter(|u| u.alive && u.owner == owner)
        .filter_map(Unit::plot)
        .any(|theirs| plot.gap_to(theirs) <= BUILD_REACH)
}

/// A new building that has not been put anywhere yet.
pub fn new_building(
    terrain: &Terrain,
    at: (f32, f32),
    role: Role,
    side: (usize, TeamColor),
    id: u32,
) -> Unit {
    let mut built = Vec::with_capacity(1);
    place_unit(&mut built, terrain, at, role, role.weapon(), side, id);
    built[0]
}

/// Puts up an owner's three buildings in a row behind a group's middle.
#[cfg(test)]
fn place_base(
    units: &mut Vec<Unit>,
    terrain: &Terrain,
    center: (f32, f32),
    behind: (f32, f32),
    side: (usize, TeamColor),
) {
    let (across_x, across_z) = (-behind.1, behind.0);
    for (role, along) in [
        (Role::Hangar, -BUILDING_SPREAD),
        (Role::Factory, 0.0),
        (Role::Barracks, BUILDING_SPREAD),
    ] {
        let at = (
            center.0 + behind.0 * BUILDINGS_BEHIND + across_x * along,
            center.1 + behind.1 * BUILDINGS_BEHIND + across_z * along,
        );
        place_building(units, terrain, at, role, side);
    }
    let pad = (
        center.0 - behind.0 * PAD_AHEAD + across_x * PAD_ALONG,
        center.1 - behind.1 * PAD_AHEAD + across_z * PAD_ALONG,
    );
    place_building(units, terrain, pad, Role::LandingPad, side);
    // The MCV stands deployed at the other end of the row: it is what makes this a base.
    let mcv = (
        center.0 + across_x * MCV_ALONG,
        center.1 + across_z * MCV_ALONG,
    );
    place_building(units, terrain, mcv, Role::Mcv, side);
    if let Some(mcv) = units.last_mut() {
        switch_deployed(mcv);
    }
}

/// Deploys every selected unit that can, or packs it up again. Returns whether an MCV could not
/// deploy where it stands.
pub fn toggle_deploy(units: &mut [Unit], terrain: &Terrain) -> bool {
    let mut refused = false;
    for unit in units
        .iter_mut()
        .filter(|u| u.selected && u.role.can_deploy())
    {
        if can_deploy_here(unit, terrain) {
            start_deploying(unit);
        } else {
            refused = true;
        }
    }
    refused
}

/// Sends the selected machines that load at landing pads to this one: the nearest of them lands
/// on it, unless another machine has it already, and the rest wait beside it.
pub fn order_land(units: &mut [Unit], pad: usize) {
    let pad_unit = units[pad];
    let pad = Pad {
        id: pad_unit.id,
        x: pad_unit.x,
        z: pad_unit.z,
        deck: pad_unit.facing.turn(),
    };
    let mut taken = units
        .iter()
        .any(|u| u.alive && !u.selected && u.pad.is_some_and(|used| used.id == pad.id));
    let away = |u: &Unit| (u.x - pad.x).hypot(u.z - pad.z);
    let mut chosen: Vec<usize> = (0..units.len())
        .filter(|&i| {
            let u = &units[i];
            u.alive && u.selected && u.owner == pad_unit.owner && u.role.lands()
        })
        .collect();
    chosen.sort_by(|&a, &b| away(&units[a]).total_cmp(&away(&units[b])));
    for i in chosen {
        let unit = &mut units[i];
        unit.target = None;
        if taken {
            unit.pad = None;
            unit.destination = beside_pad(pad, unit.id);
        } else {
            unit.pad = Some(pad);
            unit.destination = (pad.x, pad.z);
            taken = true;
        }
        unit.reset_progress();
    }
}

/// Switches one K Infantry between its pistol and, standing still, its cannon.
/// Tells a unit to deploy or pack up: an MCV on the move turns and takes its time about it,
/// and all else is done at once.
pub fn start_deploying(unit: &mut Unit) {
    match (unit.role, unit.deployed, unit.deployment) {
        (Role::Mcv, false, Deployment::Idle) => unit.deployment = Deployment::Turning,
        (Role::Mcv, false, _) => {}
        _ => switch_deployed(unit),
    }
}

pub fn switch_deployed(unit: &mut Unit) {
    unit.deployed = !unit.deployed;
    unit.destination = (unit.x, unit.z);
    if unit.role == Role::Mcv {
        // Deployed, it is a building in every way, until it packs up again.
        unit.kind = if unit.deployed {
            UnitKind::Building
        } else {
            UnitKind::Vehicle
        };
        unit.heavy = !unit.deployed;
        unit.target = None;
        return;
    }
    unit.weapon = match (unit.role, unit.deployed) {
        (Role::KInfantry, true) => WeaponKind::Cannon,
        (Role::MortarInfantry, true) => WeaponKind::Mortar,
        (Role::SiegeTank, true) => WeaponKind::Siege,
        (Role::SiegeTank, false) => WeaponKind::Cannon,
        (Role::SuicideInfantry | Role::HugeWalker, _) => WeaponKind::Unarmed,
        _ => WeaponKind::Pistol,
    };
    unit.destination = (unit.x, unit.z);
}

/// Which of a map's places for bases each side starts on, the player's first. The player's is
/// picked at random. One opponent stands opposite; two stand two places off, each side of the
/// player; three more take the other corners or edges; any other number fall anywhere.
pub fn start_slots(sides: usize, random: &mut u32) -> Vec<usize> {
    let player = (next_random(random) * START_SPOTS as f32) as usize % START_SPOTS;
    let at = |step: usize| (player + step) % START_SPOTS;
    match sides {
        2 => vec![player, at(4)],
        3 => vec![player, at(2), at(6)],
        4 => vec![player, at(2), at(4), at(6)],
        _ => {
            let mut others: Vec<usize> = (1..START_SPOTS).map(at).collect();
            for i in (1..others.len()).rev() {
                let j = (next_random(random) * (i + 1) as f32) as usize % (i + 1);
                others.swap(i, j);
            }
            std::iter::once(player)
                .chain(others)
                .take(sides.min(START_SPOTS))
                .collect()
        }
    }
}

/// Each side starts with nothing but its MCV, packed up and ready to drive, on a place for a
/// base. `colors` holds the player's, then the opponents'.
pub fn spawn(terrain: &Terrain, colors: &[TeamColor], random: &mut u32) -> Vec<Unit> {
    let mut units = Vec::new();
    let spots = start_spots(terrain.size());
    let slots = start_slots(colors.len(), random);
    for (owner, (&color, slot)) in colors.iter().zip(slots).enumerate() {
        let id = units.len() as u32;
        place_unit(
            &mut units,
            terrain,
            spots[slot],
            Role::Mcv,
            WeaponKind::Unarmed,
            (owner, color),
            id,
        );
    }
    units
}

/// A battle well under way, for tests: the player gets a block between two towers at the map's
/// middle and another block on a place for a base, a long scroll away. Each opponent gets
/// one block between two towers, on the others. `colors` holds the player's, then the opponents'.
#[cfg(test)]
pub fn spawn_bases(terrain: &Terrain, colors: &[TeamColor]) -> Vec<Unit> {
    let mut units = Vec::new();
    let player = (0, colors[0]);
    let middle_z = terrain.size().middle_z();
    let spots = start_spots(terrain.size());
    let second_group = spots[SECOND_GROUP_SPOT];
    let opposing = (0..START_SPOTS).filter(|&spot| spot != SECOND_GROUP_SPOT);
    spawn_group(
        &mut units,
        terrain,
        (0.0, middle_z),
        (4, ROWS_WITH_AIRCRAFT),
        player,
    );
    place_building(
        &mut units,
        terrain,
        (PLAYER_TOWER_OFFSET, middle_z),
        Role::Tower,
        player,
    );
    place_building(
        &mut units,
        terrain,
        (-PLAYER_TOWER_OFFSET, middle_z),
        Role::FlameTower,
        player,
    );
    place_base(&mut units, terrain, (0.0, middle_z), (0.0, 1.0), player);
    spawn_group(&mut units, terrain, second_group, (4, 2), player);
    for ((i, &color), spot) in colors[1..].iter().enumerate().zip(opposing) {
        let center = spots[spot];
        let (to_x, to_z) = (-center.0, middle_z - center.1);
        let far = to_x.hypot(to_z);
        let (sin, cos) = (-to_x / far, to_z / far);
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
        // On the side towards the middle of the map, where the ring leaves the most room.
        place_base(&mut units, terrain, center, (-sin, cos), (i + 1, color));
        // Above the block: something for the player's anti-air to go after.
        for (role, side) in [
            (Role::AdvancedCarryall, -SPACING),
            (Role::AirDrone, SPACING),
        ] {
            let at = (center.0 + side * cos, center.1 + side * sin);
            let id = units.len() as u32;
            place_unit(
                &mut units,
                terrain,
                at,
                role,
                role.weapon(),
                (i + 1, color),
                id,
            );
        }
    }
    units
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
    // Sent into a mountain or a pit, they go to the edge of it.
    let target = if flying {
        target
    } else {
        terrain.nearest_walkable(target).unwrap_or(target)
    };
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
            clear_of_ruins(terrain, spot) && terrain.can_walk(spot.0, spot.1)
        };
        if usable && is_free(spot, taken) && is_free(spot, &slots) {
            slots.push(spot);
        }
    }
    slots
}

/// Whether a spot on the ground leaves room for a vehicle beside every ruin.
fn clear_of_ruins(terrain: &Terrain, spot: (f32, f32)) -> bool {
    terrain.ruins().iter().all(|piece| {
        let (near_x, near_z) = nearest_on_footprint(piece, spot.0, spot.1);
        (near_x - spot.0).hypot(near_z - spot.1) > UnitKind::Vehicle.radius()
    })
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
/// other's way. What lands needs a spot on the ground.
pub fn order_move(units: &mut [Unit], target: (f32, f32), terrain: &Terrain, teams: &[usize]) {
    for flying in [false, true] {
        order_layer(units, target, terrain, teams, flying);
    }
    // A tower cannot go anywhere, but it does give up what it was told to fire on.
    for tower in units
        .iter_mut()
        .filter(|u| u.selected && u.kind == UnitKind::Tower)
    {
        tower.target = None;
    }
}

fn order_layer(
    units: &mut [Unit],
    target: (f32, f32),
    terrain: &Terrain,
    teams: &[usize],
    flying: bool,
) {
    let in_layer = |u: &Unit| u.keeps_to_the_air() == flying;
    let any_heavy = units.iter().any(|u| in_layer(u) && u.selected && u.heavy);
    let taken: Vec<(f32, f32)> = units
        .iter()
        .filter(|u| {
            let can_be_run_over =
                any_heavy && are_enemies(teams, 0, u.owner) && !u.kind.is_building();
            // Buildings stay in the way whether selected or not.
            in_layer(u) && (!u.selected || u.kind.is_building()) && u.alive && !can_be_run_over
        })
        .flat_map(|u| {
            let around = u.kind.is_building().then(|| building_footprint(u));
            std::iter::once(u.destination).chain(around.into_iter().flatten())
        })
        .collect();
    // Deployed units are dug in and stay put.
    let mut chosen: Vec<usize> = (0..units.len())
        .filter(|&i| {
            let unit = &units[i];
            in_layer(unit) && unit.selected && !unit.deployed && !unit.kind.is_building()
        })
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
        // Sent somewhere, it sits down there.
        units[i].on_mission = false;
        units[i].unloading = units[i].carrying;
        units[i].reset_progress();
    }
}

/// Tells every selected building that makes units where to send them.
pub fn set_gathering(units: &mut [Unit], point: (f32, f32)) {
    for unit in units.iter_mut().filter(|u| {
        u.selected
            && u.kind == UnitKind::Building
            && !matches!(u.role, Role::LandingPad | Role::Mcv | Role::Palace(_))
    }) {
        unit.gathering = Some(point);
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
        if unit.can_attack(&chosen, teams) || unit.can_tend(&chosen, teams) {
            set_target(unit, Target::Unit(chosen.id));
        }
    }
}

/// Like `order_attack`, but those able to fire on it do so even if it is a friend.
pub fn order_force_attack(units: &mut [Unit], chosen: usize, teams: &[usize]) {
    let chosen = units[chosen];
    for unit in units.iter_mut().filter(|u| u.selected) {
        if unit.can_attack(&chosen, teams) || unit.can_tend(&chosen, teams) {
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
            Target::Unit(id) | Target::Forced(id) | Target::Rescue(id) | Target::RunOver(id) => {
                let fit = |u: &Unit| match target {
                    Target::Forced(_) => units[i].can_fire_on(u),
                    Target::Rescue(_) => units[i].can_fix(u, teams) && u.health < u.max_health(),
                    Target::RunOver(_) => can_run_over(&units[i], u, teams),
                    _ => units[i].can_attack(u, teams) || units[i].can_tend(u, teams),
                };
                let Some(other_index) = units.iter().position(|u| u.alive && u.id == id && fit(u))
                else {
                    units[i].stand_down();
                    continue;
                };
                let other = units[other_index];
                if matches!(target, Target::Unit(_)) && units[i].can_capture(&other, teams) {
                    let touching = units[i].radius() + other.radius() + CAPTURE_MARGIN;
                    if (other.x - x).hypot(other.z - z) <= touching {
                        (units[other_index].owner, units[other_index].color) =
                            (units[i].owner, units[i].color);
                        units[other_index].selected = false;
                        units[other_index].gathering = None;
                        units[other_index].making = None;
                        units[other_index].queue = [None; QUEUE_ROOM];
                        units[i].stand_down();
                    } else {
                        units[i].destination = (other.x, other.z);
                    }
                    continue;
                }
                if matches!(target, Target::RunOver(_)) {
                    // Right over it: it is no use stopping short.
                    units[i].destination = (other.x, other.z);
                    continue;
                }
                other.center()
            }
        };
        if units[i].deployed || units[i].kind.is_building() {
            continue;
        }
        let away = distance(goal, units[i].center());
        let in_reach = away <= units[i].range_down_to(goal.1) * ATTACK_REACH;
        // What never stops circles its target, firing as it goes.
        units[i].destination = if in_reach && !units[i].role.never_stops() {
            (x, z)
        } else {
            (goal.0, goal.2)
        };
    }
}

/// Whether it could go after this one to drive over it: a heavy unit, enemy infantry.
pub fn can_run_over(squasher: &Unit, victim: &Unit, teams: &[usize]) -> bool {
    squasher.heavy
        && squasher.alive
        && victim.alive
        && victim.kind == UnitKind::Infantry
        && !victim.hidden_from(squasher.owner, teams)
        && are_enemies(teams, squasher.owner, victim.owner)
}

/// Sends repair vehicles with nothing to do to the nearest hurt friend close by.
fn rush_to_the_hurt(units: &mut [Unit], teams: &[usize]) {
    /// How far off a repair vehicle notices a friend that is hurt.
    const NOTICES: f32 = 12.0;
    for i in 0..units.len() {
        let unit = units[i];
        if unit.weapon != WeaponKind::Repair
            || !unit.alive
            || unit.target.is_some()
            || unit.is_moving()
        {
            continue;
        }
        let away = |u: &Unit| (u.x - unit.x).hypot(u.z - unit.z);
        let hurt = units
            .iter()
            .filter(|u| unit.can_fix(u, teams) && u.health < u.max_health() && away(u) <= NOTICES)
            .min_by(|a, b| away(a).total_cmp(&away(b)));
        if let Some(friend) = hurt {
            units[i].target = Some(Target::Rescue(friend.id));
            units[i].reset_progress();
        }
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
    let goal = unit.way.map_or(unit.destination, |way| way.through);
    let (to_x, to_z) = (goal.0 - unit.x, goal.1 - unit.z);
    let length = to_x.hypot(to_z).max(0.0001);
    let toward_destination = (to_x / length, to_z / length);
    let (want_x, want_z) =
        prey_direction(units, index, teams, toward_destination).unwrap_or(toward_destination);
    let (right_x, right_z) = (-want_z, want_x);
    let mut sideways = 0.0;
    for (other_index, other) in units.iter().enumerate() {
        let other_layer = (other.altitude > 0.0) != (unit.altitude > 0.0);
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
        let range = unit.radius() + other.radius() + AVOID_MARGIN;
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
        let range = unit.radius() + AVOID_MARGIN;
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
    let radius = unit.radius();
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

/// An enemy carryall's hold on a vehicle, which stays in the battle meanwhile: up in the air it
/// is a target like any other. It does nothing of its own until it is put down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theft {
    pub carrier: u32,
    /// How many more seconds it fights before it can be lifted.
    pub struggle: f32,
}

/// How a unit on the ground is getting on with going somewhere, looked at now and then.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stall {
    /// Where it was when last looked at.
    at: (f32, f32),
    /// How long until it is looked at again.
    wait: f32,
    /// How many looks in a row found it no nearer to getting there.
    looks: u8,
    /// Whether it has got nearer since the last look: closer to where it is going, or to the
    /// next point on a route there.
    got_nearer: bool,
    /// Whether it should find a new way around what is holding it up.
    rethink: bool,
}

impl Stall {
    const NOT_STALLED: Stall = Stall {
        at: (f32::MAX, f32::MAX),
        wait: STALL_LOOK_SECONDS,
        looks: 0,
        got_nearer: false,
        rethink: false,
    };
}

/// A landing pad, and where it stands.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pad {
    id: u32,
    x: f32,
    z: f32,
    /// The way its deck lies, as a heading.
    deck: f32,
}

/// A point to pass through, and the destination it was found for.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Way {
    through: (f32, f32),
    to: (f32, f32),
    /// Whether it is only a step to one side, with no route to go by.
    aside: bool,
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
            if dx.hypot(dz).hypot(dy) < units[a].radius() + units[b].radius() {
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
                // A carryall and the vehicle it has hold of are meant to be together.
                let held_by =
                    |held: &Unit, by: &Unit| held.theft.is_some_and(|theft| theft.carrier == by.id);
                // And so are a machine and the pad it is using.
                let using =
                    |machine: &Unit, pad: &Unit| machine.pad.is_some_and(|used| used.id == pad.id);
                if !units[a].alive
                    || !units[b].alive
                    || held_by(&units[a], &units[b])
                    || held_by(&units[b], &units[a])
                    || using(&units[a], &units[b])
                    || using(&units[b], &units[a])
                {
                    continue;
                }
                let (dx, dz) = (units[b].x - units[a].x, units[b].z - units[a].z);
                let dy = units[b].sphere_y() - units[a].sphere_y();
                let touching = units[a].radius() + units[b].radius();
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
                // shoves a building, or what is sitting or coming down.
                let solid = |unit: &Unit| {
                    let going_up_or_down = unit.role.lands() && unit.altitude < CRUISE_ALTITUDE;
                    (
                        unit.kind.is_building() || unit.deployed || going_up_or_down,
                        unit.heavy,
                    )
                };
                let (share_a, share_b) = match solid(&units[a]).cmp(&solid(&units[b])) {
                    Ordering::Greater => (0.0, 1.0),
                    Ordering::Less => (1.0, 0.0),
                    Ordering::Equal => (0.5, 0.5),
                };
                for (i, sign, share) in [(a, -1.0, share_a), (b, 1.0, share_b)] {
                    // Nothing at all moves a building, however solid the other is.
                    if units[i].kind.is_building() {
                        continue;
                    }
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

/// Finds a way around what stands between a unit on the ground and where it is going, unless
/// it has one or goes straight there. With no way at all it keeps heading straight, which ends
/// with it giving up against whatever blocks it. Returns whether it had to look for a route.
fn plan_way(unit: &mut Unit, terrain: &Terrain, may_search: bool) -> bool {
    /// Close enough to a point on its way to go on to the next.
    const THROUGH: f32 = 0.4;
    /// How far a destination may have moved for a route to it to still be good.
    const STILL_GOOD: f32 = 1.0;
    let here = (unit.x, unit.z);
    if let Some(way) = unit.way {
        let moved = (way.to.0 - unit.destination.0).hypot(way.to.1 - unit.destination.1);
        let there = (way.through.0 - unit.x).hypot(way.through.1 - unit.z) < THROUGH;
        if there {
            unit.blocked_for = 0.0;
            // A step aside gets it no nearer, for all that it got there.
            unit.stall.got_nearer |= !way.aside;
        }
        if !there && moved < STILL_GOOD {
            return false;
        }
        unit.way = None;
    }
    if !may_search || terrain.line_is_clear(here, unit.destination) {
        return false;
    }
    let through = terrain
        .next_on_route(here, unit.destination)
        .unwrap_or(unit.destination);
    unit.way = Some(Way {
        through,
        to: unit.destination,
        aside: false,
    });
    true
}

/// Sees whether a unit on the ground is getting anywhere. One that is not tries a new way, and
/// after long enough gives up: nobody is left pressing against a wall for ever.
fn watch_for_stall(unit: &mut Unit, dt: f32) {
    if !unit.is_moving() {
        unit.stall = Stall::NOT_STALLED;
        return;
    }
    unit.stall.wait -= dt;
    if unit.stall.wait > 0.0 {
        return;
    }
    let (x, z) = unit.stall.at;
    let covered = (unit.x - x).hypot(unit.z - z);
    unit.stall.at = (unit.x, unit.z);
    unit.stall.wait = STALL_LOOK_SECONDS;
    let got_nearer = std::mem::take(&mut unit.stall.got_nearer);
    unit.stall.looks = if got_nearer { 0 } else { unit.stall.looks + 1 };
    if unit.stall.looks > STALL_LOOKS_AT_MOST {
        unit.stand_down();
        unit.stall = Stall::NOT_STALLED;
    } else if covered < unit.speed() * STALL_LOOK_SECONDS * STALL_SHARE {
        unit.stall.rethink = true;
    }
}

/// Finds a new way for a stalled unit, around everything on the ground that is staying where it
/// is as well as what stands on the map. With none, it tries stepping aside, a different way
/// each time.
fn find_new_way(units: &mut [Unit], i: usize, terrain: &Terrain) {
    let unit = units[i];
    let target_id = match unit.target {
        Some(Target::Unit(id) | Target::Forced(id) | Target::Rescue(id) | Target::RunOver(id)) => {
            Some(id)
        }
        _ => None,
    };
    let obstacles: Vec<(f32, f32, f32)> = units
        .iter()
        .filter(|u| {
            u.id != unit.id
                && u.alive
                && u.altitude <= 0.0
                && Some(u.id) != target_id
                && (u.kind.is_building() || u.deployed || !u.is_moving())
        })
        .map(|u| (u.x, u.z, u.radius() + unit.radius()))
        .collect();
    let here = (unit.x, unit.z);
    let route = terrain.next_on_route_around(here, unit.destination, &obstacles);
    let through = route.unwrap_or_else(|| {
        let (sin, cos) = (unit.heading + FRAC_PI_2 * f32::from(unit.stall.looks)).sin_cos();
        (unit.x + cos * 2.0, unit.z + sin * 2.0)
    });
    units[i].way = Some(Way {
        through,
        to: unit.destination,
        aside: route.is_none(),
    });
    units[i].stall.rethink = false;
}

fn step(units: &mut [Unit], terrain: &Terrain, teams: &[usize], dt: f32) {
    /// Routes looked for per step, so that a big order is spread over a few frames.
    const SEARCHES: usize = 6;
    let mut searches = SEARCHES;
    let mut held_back = vec![false; units.len()];
    let started_at: Vec<(f32, f32)> = units.iter().map(|u| (u.x, u.z)).collect();
    for i in 0..units.len() {
        if units[i].is_settling() {
            settle(&mut units[i], terrain, dt);
        } else if units[i].is_rising_to_fight() {
            units[i].altitude = (units[i].altitude + CLIMB_SPEED * dt).min(CRUISE_ALTITUDE);
        }
        if !units[i].is_moving() {
            units[i].way = None;
            continue;
        }
        if units[i].role.never_stops() {
            fly_on(&mut units[i], terrain, dt);
            continue;
        }
        if !units[i].kind.flies() {
            if units[i].stall.rethink && searches > 0 {
                find_new_way(units, i, terrain);
                searches -= 1;
            } else if plan_way(&mut units[i], terrain, searches > 0) {
                searches -= 1;
            }
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
            // A laser tank glides any way at once, slowly turning its nose that way meanwhile.
            UnitKind::Vehicle if unit.role == Role::LaserTank => {
                let slow = GLIDER_TURN_SPEED * dt;
                unit.heading += turn.clamp(-slow, slow);
                held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
            }
            UnitKind::Vehicle => {
                let turned = turn.clamp(-most, most);
                unit.heading += turned;
                if unit.role == Role::Recon {
                    // Its legs step as it turns on the spot as well.
                    unit.stride += turned.abs() * LEGS_PER_TURN;
                }
                if turn.abs() <= DRIVE_ALIGNMENT {
                    held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
                }
            }
            // It goes straight up, then turns on the spot, then sets off.
            UnitKind::Aircraft if unit.role.lands() => {
                if unit.altitude < CRUISE_ALTITUDE {
                    unit.altitude = (unit.altitude + CLIMB_SPEED * dt).min(CRUISE_ALTITUDE);
                    unit.blocked_for = 0.0;
                } else if turn.abs() > DRIVE_ALIGNMENT {
                    unit.heading += turn.clamp(-most, most);
                    unit.blocked_for = 0.0;
                } else {
                    unit.heading += turn;
                    held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
                }
            }
            // Nothing holds it back from setting off while its nose comes around.
            UnitKind::Aircraft => {
                unit.heading += turn.clamp(-most, most);
                held_back[i] = walk(unit, terrain, (dir_x, dir_z), dt);
            }
            UnitKind::Tower | UnitKind::Building => {}
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
        unit.stride += moved;
        held_back[i] |= unit.target.is_some() && moved < unit.speed() * dt * WEDGED_SHARE;
        unit.y = terrain.height_at(unit.x, unit.z);
        if unit.kind.flies() {
            // What flies keeps above the land's level over a pit.
            unit.y = unit.y.max(0.0);
        }
        give_up_if_stuck(unit, held_back[i], dt);
        if !unit.kind.flies() {
            watch_for_stall(unit, dt);
        }
    }
}

/// Comes down to sit, if the ground right below it can take it. Otherwise it stays up.
fn settle(unit: &mut Unit, terrain: &Terrain, dt: f32) {
    let spot = (unit.x, unit.z);
    if terrain.can_walk(spot.0, spot.1) && clear_of_ruins(terrain, spot) {
        let is_over_pad = unit
            .pad
            .is_some_and(|pad| (pad.x - unit.x).hypot(pad.z - unit.z) < OVER_PAD);
        let lowest = if is_over_pad { PAD_TOP } else { 0.0 };
        unit.altitude = (unit.altitude - CLIMB_SPEED * dt).max(lowest);
        if is_over_pad {
            // Coming down, it lines itself up with the deck, whichever way round is nearer.
            let deck = unit.pad.map_or(FRAC_PI_4, |pad| pad.deck);
            let along = turn_between(unit.heading, deck);
            let turn = if along.abs() <= FRAC_PI_2 {
                along
            } else {
                turn_between(unit.heading, deck + PI)
            };
            let most = TURN_SPEED * dt;
            unit.heading += turn.clamp(-most, most);
        }
    }
}

/// Flies on along its nose, turning to come to where it was sent and then to circle it, and
/// leaning into the turn.
fn fly_on(unit: &mut Unit, terrain: &Terrain, dt: f32) {
    let (to_x, to_z) = (unit.destination.0 - unit.x, unit.destination.1 - unit.z);
    let away = to_x.hypot(to_z);
    // Straight at it from far away, along the circle on it, and outward from inside it.
    let off_its_way = if away >= CIRCLE_RADIUS {
        FRAC_PI_2 * CIRCLE_RADIUS / away
    } else {
        FRAC_PI_2 * (2.0 - away / CIRCLE_RADIUS)
    };
    let most = TURN_SPEED * dt;
    let turned = turn_between(unit.heading, to_z.atan2(to_x) + off_its_way).clamp(-most, most);
    unit.heading += turned;
    let lean = MOST_BANK * turned / most;
    unit.bank += (lean - unit.bank) * (BANK_RATE * dt).min(1.0);
    let step = unit.speed() * dt;
    let ahead = (unit.heading.cos(), unit.heading.sin());
    // At the map's edge it glides along it at full speed, the way its nose is nearer to, or
    // the way it is turning if it meets the edge head on. The edges run along the grid.
    let left = (-ahead.1, ahead.0);
    let turning = if turned >= 0.0 { 1.0 } else { -1.0 };
    let mut glides = [(1.0, 1.0), (-1.0, 1.0)].map(|(ex, ez): (f32, f32)| {
        let (ex, ez) = (ex * FRAC_1_SQRT_2, ez * FRAC_1_SQRT_2);
        let along = ahead.0 * ex + ahead.1 * ez;
        let side = left.0 * ex + left.1 * ez;
        let sign = (along + 0.1 * turning * side).signum();
        (along.abs(), (ex * sign, ez * sign))
    });
    glides.sort_by(|a, b| b.0.total_cmp(&a.0));
    // In a corner it can only glide back out along one of the edges.
    let back = glides.map(|(_, (wx, wz))| (-wx, -wz));
    let ways = std::iter::once(ahead)
        .chain(glides.map(|(_, way)| way))
        .chain(back);
    let free = ways
        .map(|(wx, wz)| (wx * step, wz * step))
        .enumerate()
        .find(|(_, (tx, tz))| unit.can_enter(terrain, unit.x + tx, unit.z + tz));
    if let Some((way, (tx, tz))) = free {
        unit.x += tx;
        unit.z += tz;
        if way > 0 {
            // Its nose comes round to the way it glides.
            unit.heading += turn_between(unit.heading, tz.atan2(tx)).clamp(-most, most);
        }
    }
}

/// Crowds only delay a unit, so its order stands. If water, lava or a ruin keeps it from
/// getting any nearer, or it is wedged on its way to a target, for all the new ways it has
/// tried meanwhile, there is no way through: it settles for here and forgets the target.
fn give_up_if_stuck(unit: &mut Unit, held_back: bool, dt: f32) {
    if !unit.is_moving() {
        unit.reset_progress();
        return;
    }
    if unit.remaining() < unit.best_remaining - PROGRESS {
        unit.best_remaining = unit.remaining();
        unit.blocked_for = 0.0;
        unit.stall.got_nearer = true;
    } else if held_back {
        unit.blocked_for += dt;
        if unit.blocked_for > BLOCKED_PATIENCE {
            unit.stand_down();
            unit.blocked_for = 0.0;
        }
    }
}

/// A vehicle on its way up or down under a carryall, or hanging there. It is out of the battle
/// meanwhile: nothing can hit it, and it does nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Carried {
    pub carrier: u32,
    /// Its `y` is how high it hangs right now, for drawing.
    pub vehicle: Unit,
    /// How far above the ground.
    height: f32,
}

/// Picks up vehicles that carryalls have reached, moves those carried along, and puts them
/// down where they were sent. A vehicle whose carryall is gone is left marked as not alive.
pub fn carry(
    units: &mut Vec<Unit>,
    carried: &mut Vec<Carried>,
    terrain: &Terrain,
    teams: &[usize],
    dt: f32,
) {
    for i in 0..units.len() {
        let Some(Target::Unit(id)) = units[i].target else {
            continue;
        };
        let carrier = units[i];
        let in_reach = |u: &Unit| (u.x - carrier.x).hypot(u.z - carrier.z) <= LIFT_REACH;
        if let Some(index) = units
            .iter()
            .position(|u| u.id == id && carrier.can_steal(u, teams) && in_reach(u))
        {
            let vehicle = &mut units[index];
            vehicle.theft = Some(Theft {
                carrier: carrier.id,
                struggle: STRUGGLE_SECONDS,
            });
            vehicle.selected = false;
            vehicle.target = None;
            vehicle.stand_down();
            units[i].carrying = true;
            units[i].stand_down();
            break;
        }
        let Some(index) = units.iter().position(|u| {
            u.id == id && carrier.can_lift(u, teams) && {
                (u.x - carrier.x).hypot(u.z - carrier.z) <= LIFT_REACH
            }
        }) else {
            continue;
        };
        let mut vehicle = units.remove(index);
        vehicle.selected = false;
        vehicle.target = None;
        carried.push(Carried {
            carrier: carrier.id,
            vehicle,
            height: 0.0,
        });
        // Everything after the removed vehicle has moved down one.
        let carrier = units
            .iter_mut()
            .find(|u| u.id == carrier.id)
            .expect("the carrier is not the vehicle");
        carrier.carrying = true;
        carrier.stand_down();
        break;
    }
    let mut still_carried = Vec::with_capacity(carried.len());
    for mut load in std::mem::take(carried) {
        let carrier = units
            .iter_mut()
            .find(|u| u.id == load.carrier)
            .filter(|u| u.alive);
        let Some(carrier) = carrier else {
            load.vehicle.alive = false;
            load.vehicle.selected = false;
            units.push(load.vehicle);
            continue;
        };
        let spot = (carrier.x, carrier.z);
        let lowering = carrier.unloading
            && !carrier.is_moving()
            && terrain.can_walk(spot.0, spot.1)
            && clear_of_ruins(terrain, spot);
        let vehicle = &mut load.vehicle;
        (vehicle.x, vehicle.z, vehicle.heading) = (spot.0, spot.1, carrier.heading);
        let winch = if lowering { -WINCH_SPEED } else { WINCH_SPEED };
        load.height = (load.height + winch * dt).clamp(0.0, HANG_HEIGHT);
        vehicle.y = carrier.y + load.height;
        if lowering && load.height <= 0.0 {
            carrier.carrying = false;
            carrier.unloading = false;
            vehicle.y = terrain.height_at(spot.0, spot.1);
            vehicle.destination = spot;
            vehicle.reset_progress();
            units.push(load.vehicle);
        } else {
            still_carried.push(load);
        }
    }
    *carried = still_carried;
    work_on_thefts(units, terrain, dt);
}

/// Holds stolen vehicles to their carryalls: fighting at first, both staying where they are,
/// then up in the air and along with it until it puts them down, when they are free again. A
/// vehicle whose carryall is gone goes with it, and a carryall whose vehicle is gone has its
/// hands free.
fn work_on_thefts(units: &mut [Unit], terrain: &Terrain, dt: f32) {
    for i in 0..units.len() {
        let Some(mut theft) = units[i].theft else {
            continue;
        };
        let held = units.iter().position(|u| u.id == theft.carrier && u.alive);
        let Some(c) = held else {
            units[i].alive = false;
            units[i].selected = false;
            units[i].theft = None;
            continue;
        };
        if !units[i].alive {
            units[c].carrying = false;
            units[c].unloading = false;
            units[i].theft = None;
            continue;
        }
        if theft.struggle > 0.0 {
            // Neither can go anywhere yet, whatever they have been told.
            theft.struggle -= dt;
            let spot = (units[i].x, units[i].z);
            (units[c].x, units[c].z) = spot;
            units[c].destination = spot;
            units[c].unloading = false;
            units[i].destination = spot;
            units[i].theft = Some(theft);
            continue;
        }
        let spot = (units[c].x, units[c].z);
        let lowering = units[c].unloading
            && !units[c].is_moving()
            && terrain.can_walk(spot.0, spot.1)
            && clear_of_ruins(terrain, spot);
        let winch = if lowering { -WINCH_SPEED } else { WINCH_SPEED };
        let height = (units[i].altitude + winch * dt).clamp(0.0, HANG_HEIGHT);
        let heading = units[c].heading;
        let vehicle = &mut units[i];
        (vehicle.x, vehicle.z, vehicle.heading) = (spot.0, spot.1, heading);
        vehicle.y = terrain.height_at(spot.0, spot.1);
        vehicle.altitude = height;
        vehicle.destination = spot;
        if lowering && height <= 0.0 {
            vehicle.theft = None;
            vehicle.reset_progress();
            units[c].carrying = false;
            units[c].unloading = false;
        }
    }
}

/// Tells whatever lands whether an enemy is within reach of where it would fire from, up at its
/// cruising height. It goes up for one and comes back down once none is left.
fn alert_landers(units: &mut [Unit], teams: &[usize]) {
    for i in 0..units.len() {
        if !units[i].role.lands() {
            continue;
        }
        let lander = units[i];
        let up_there = (
            lander.x,
            lander.y + CRUISE_ALTITUDE + lander.radius(),
            lander.z,
        );
        // What is being loaded stays down until it is done, unless told otherwise.
        let is_loading = lander.is_on_pad() && lander.rockets_fired > 0;
        units[i].alerted = !is_loading
            && units.iter().any(|other| {
                lander.fires_on_sight(other, teams)
                    && other.kind != UnitKind::Building
                    && distance(up_there, other.center()) <= lander.weapon.range()
            });
    }
}

/// Sends machines out of rockets to a free landing pad of their owner's, loads those sitting on
/// one, and lets go of a pad once they have left it with rockets on board. One out of rockets
/// that has been sent elsewhere goes there first, then back. With no pad free it waits beside
/// the nearest. A full one on a pad makes way for a friend near it that is short of rockets.
fn tend_pads(units: &mut [Unit], terrain: &Terrain, dt: f32) {
    for i in 0..units.len() {
        if !units[i].alive || !units[i].role.lands() {
            continue;
        }
        let unit = units[i];
        let out = !unit.has_rockets();
        let idle = !unit.is_moving() && unit.target.is_none();
        match unit.pad {
            Some(pad)
                if !units
                    .iter()
                    .any(|u| u.id == pad.id && is_pad_of(u, unit.owner)) =>
            {
                units[i].pad = None;
            }
            Some(_) if unit.is_on_pad() && unit.rockets_fired > 0 => {
                units[i].reloading += dt;
                if units[i].reloading >= RELOAD_SECONDS {
                    units[i].reloading = 0.0;
                    units[i].rockets_fired -= 1;
                }
            }
            Some(pad) if unit.is_on_pad() => {
                // Full, it makes way for a friend close by that is not.
                let is_short_nearby = |u: &Unit| {
                    u.id != unit.id
                        && u.alive
                        && u.owner == unit.owner
                        && u.role.lands()
                        && u.rockets_fired > 0
                        && u.pad.is_none()
                        && (u.x - pad.x).hypot(u.z - pad.z) <= PAD_NEARBY
                };
                if idle && units.iter().any(is_short_nearby) {
                    units[i].pad = None;
                    units[i].destination = beside_pad(pad, unit.id);
                    units[i].reset_progress();
                }
            }
            Some(pad) if !out => {
                // It lets go of it if it has been sent off somewhere else.
                let heading_there = unit.destination == (pad.x, pad.z);
                if !heading_there && (pad.x - unit.x).hypot(pad.z - unit.z) > LEAVES_PAD {
                    units[i].pad = None;
                }
            }
            Some(pad) => {
                if idle {
                    units[i].destination = (pad.x, pad.z);
                    units[i].reset_progress();
                }
            }
            // With rockets left it only tops up at a pad close by, with no enemy to go up for.
            None if idle && (out || (unit.rockets_fired > 0 && !unit.alerted)) => {
                let taken = |id: u32| {
                    units
                        .iter()
                        .any(|u| u.alive && u.pad.is_some_and(|pad| pad.id == id))
                };
                let away = |u: &Unit| (u.x - unit.x).hypot(u.z - unit.z);
                let as_pad = |u: &Unit| Pad {
                    id: u.id,
                    x: u.x,
                    z: u.z,
                    deck: u.facing.turn(),
                };
                let nearest_of = |free: bool| {
                    units
                        .iter()
                        .filter(|u| is_pad_of(u, unit.owner) && (!free || !taken(u.id)))
                        .min_by(|a, b| away(a).total_cmp(&away(b)))
                };
                let free = nearest_of(true).filter(|u| out || away(u) <= PAD_NEARBY);
                if let Some(pad) = free.map(as_pad) {
                    units[i].pad = Some(pad);
                    units[i].destination = (pad.x, pad.z);
                    units[i].reset_progress();
                } else if out && let Some(busy) = nearest_of(false).filter(|u| away(u) > PAD_WAITS)
                {
                    // All taken: it waits by the nearest, looking again every step.
                    units[i].destination = beside_pad(as_pad(busy), unit.id);
                    units[i].reset_progress();
                }
            }
            _ => {}
        }
        go_home_after_a_mission(units, i, terrain);
    }
}

/// Once what lands has nothing more to go after and no enemy close by, it goes to a free pad of its owner's, or
/// beside one if they are all taken, or else back to its base by the MCV, or at least to the
/// nearest higher ground off the sand, rather than sitting down wherever it is.
fn go_home_after_a_mission(units: &mut [Unit], i: usize, terrain: &Terrain) {
    let unit = units[i];
    if unit.target.is_some() {
        units[i].on_mission = true;
        return;
    }
    // With more enemies close by, its mission is not over yet.
    if !unit.on_mission
        || unit.alerted
        || unit.is_moving()
        || unit.pad.is_some()
        || !unit.has_rockets()
    {
        return;
    }
    units[i].on_mission = false;
    let away = |u: &Unit| (u.x - unit.x).hypot(u.z - unit.z);
    let taken = |id: u32| {
        units
            .iter()
            .any(|u| u.alive && u.pad.is_some_and(|pad| pad.id == id))
    };
    let pads = || units.iter().filter(|u| is_pad_of(u, unit.owner));
    let as_pad = |u: &Unit| Pad {
        id: u.id,
        x: u.x,
        z: u.z,
        deck: u.facing.turn(),
    };
    let nearest = |of: &mut dyn Iterator<Item = &Unit>| {
        of.min_by(|a, b| away(a).total_cmp(&away(b))).copied()
    };
    if let Some(pad) = nearest(&mut pads().filter(|u| !taken(u.id))) {
        units[i].pad = Some(as_pad(&pad));
        units[i].destination = (pad.x, pad.z);
    } else if let Some(pad) = nearest(&mut pads()) {
        units[i].destination = beside_pad(as_pad(&pad), unit.id);
    } else if let Some(base) = nearest(
        &mut units
            .iter()
            .filter(|u| u.alive && u.owner == unit.owner && u.role == Role::Mcv && u.deployed),
    )
    .or_else(|| {
        nearest(
            &mut units
                .iter()
                .filter(|u| u.alive && u.owner == unit.owner && u.kind.is_building()),
        )
    }) {
        let (sin, cos) = (unit.id as f32 * 2.4).sin_cos();
        units[i].destination = (base.x + cos * HOME_BESIDE, base.z + sin * HOME_BESIDE);
    } else if let Some(high) = higher_ground_near(terrain, (unit.x, unit.z)) {
        units[i].destination = high;
    } else {
        return;
    }
    units[i].reset_progress();
}

/// The nearest place around a point on the higher ground above the sand, if the sand is
/// where it would otherwise sit down.
fn higher_ground_near(terrain: &Terrain, (x, z): (f32, f32)) -> Option<(f32, f32)> {
    let high = |(x, z): (f32, f32)| terrain.can_walk(x, z) && terrain.on_upper_ground(x, z);
    if high((x, z)) {
        return None;
    }
    (1..=HIGHER_GROUND_RINGS).find_map(|ring| {
        let reach = ring as f32 * HIGHER_GROUND_STEP;
        let tries = 6 * ring;
        (0..tries).find_map(|n| {
            let angle = n as f32 / tries as f32 * TAU;
            let at = (x + reach * angle.cos(), z + reach * angle.sin());
            high(at).then_some(at)
        })
    })
}

/// Whether it is a landing pad of this owner's still standing.
fn is_pad_of(u: &Unit, owner: usize) -> bool {
    u.alive && u.role == Role::LandingPad && u.owner == owner
}

/// A spot beside a landing pad to wait or to make way at, its own for each machine.
fn beside_pad(pad: Pad, id: u32) -> (f32, f32) {
    let (sin, cos) = (id as f32 * 2.4).sin_cos();
    (pad.x + cos * PAD_BESIDE, pad.z + sin * PAD_BESIDE)
}

/// Puts it in the middle of the grid squares it stands on, as a building is.
fn settle_on_its_squares(unit: &mut Unit) {
    (unit.x, unit.z) = unit
        .role
        .plot_facing((unit.x, unit.z), unit.facing)
        .center();
}

/// Brings an MCV that was told to deploy round to the way it deploys facing, and then through
/// the second that it takes.
fn go_on_deploying(unit: &mut Unit, dt: f32) {
    match unit.deployment {
        Deployment::Idle => return,
        Deployment::Turning => {
            let turn = turn_between(unit.heading, MCV_DEPLOY_HEADING);
            let most = TURN_SPEED * dt;
            if turn.abs() <= most {
                unit.heading = MCV_DEPLOY_HEADING;
                unit.deployment = Deployment::Drilling;
                unit.constructing = CONSTRUCTION_SECONDS;
                settle_on_its_squares(unit);
            } else {
                unit.heading += turn.clamp(-most, most);
            }
        }
        Deployment::Drilling if unit.constructing <= 0.0 => {
            unit.deployment = Deployment::Idle;
            settle_on_its_squares(unit);
            switch_deployed(unit);
        }
        Deployment::Drilling => {}
    }
    // It has started: nothing it is told meanwhile moves it.
    unit.destination = (unit.x, unit.z);
    unit.target = None;
}

/// Advances every unit by `dt` seconds. Units killed meanwhile are left marked as not alive.
pub fn advance(units: &mut [Unit], terrain: &Terrain, teams: &[usize], dt: f32) {
    for unit in units.iter_mut() {
        unit.constructing = (unit.constructing - dt).max(0.0);
        go_on_deploying(unit, dt);
    }
    tend_pads(units, terrain, dt);
    alert_landers(units, teams);
    rush_to_the_hurt(units, teams);
    chase_targets(units, terrain, teams);
    // Spheres only start to overlap when something moves.
    if !units
        .iter()
        .any(|u| u.is_moving() || u.is_settling() || u.is_rising_to_fight())
    {
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
        let units = spawn_bases(terrain(0), &COLORS);
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
        let units = spawn_bases(terrain(0), &COLORS);
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
    fn everyone_starts_with_aircraft_in_the_air() {
        let units = spawn_bases(terrain(0), &COLORS);
        let planes: Vec<_> = units
            .iter()
            .filter(|u| u.kind == UnitKind::Aircraft)
            .collect();
        assert!(!planes.is_empty());
        assert!(
            planes
                .iter()
                .all(|u| u.altitude == CRUISE_ALTITUDE && !u.heavy)
        );
        assert!(
            planes
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| u.role == Role::AdvancedCarryall && u.weapon == WeaponKind::Unarmed)
        );
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
        gunner.weapon = WeaponKind::Laser;
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
        let units = spawn_bases(terrain(0), &COLORS);
        for (owner, &color) in COLORS.iter().enumerate() {
            let flame: Vec<_> = units
                .iter()
                .filter(|u| u.owner == owner && u.role == Role::FlameTower)
                .collect();
            assert_eq!(flame.len(), 1);
            assert_eq!(flame[0].color, color);
            assert_eq!(flame[0].weapon, WeaponKind::Flame);
            assert_eq!(flame[0].is_selected_in_groups(), owner == 0);
        }
    }

    #[test]
    fn everyone_starts_with_one_tower_of_their_color_with_missiles() {
        let units = spawn_bases(terrain(0), &COLORS);
        for (owner, &color) in COLORS.iter().enumerate() {
            let towers: Vec<_> = units
                .iter()
                .filter(|u| u.owner == owner && u.role == Role::Tower)
                .collect();
            assert_eq!(towers.len(), 1);
            assert_eq!(towers[0].color, color);
            assert_eq!(towers[0].weapon, WeaponKind::TowerRockets);
            assert_eq!(towers[0].health, UnitKind::Tower.max_health());
            assert_eq!(towers[0].is_selected_in_groups(), owner == 0);
        }
    }

    #[test]
    fn towers_stand_on_land_on_every_map_with_any_number_of_opponents() {
        let colors = crate::game::TEAM_COLORS;
        for map in 0..crate::game::MAPS.len() {
            for count in 2..=colors.len() {
                let terrain = terrain(map);
                for unit in spawn_bases(terrain, &colors[..count])
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
        let side = (0, TeamColor::Blue);
        place_unit(
            &mut units,
            open_ground(),
            (0.0, 60.0),
            role,
            role.weapon(),
            side,
            99,
        );
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
            (Role::Mongoose, WeaponKind::Missile, "Mongoose"),
            (Role::Minotaurus, WeaponKind::Bombard, "Minotaurus"),
            (Role::RocketLauncher, WeaponKind::Rockets, "Rocket Launcher"),
            (
                Role::AirDefensePlatform,
                WeaponKind::AutoGun,
                "Air Defense Platform",
            ),
            (
                Role::AdvancedCarryall,
                WeaponKind::Unarmed,
                "Advanced Carryall",
            ),
            (Role::AirDrone, WeaponKind::AirMissile, "Air Drone"),
            (Role::Ornithopter, WeaponKind::Rockets, "Ornithopter"),
            (Role::Destroyer, WeaponKind::HeavyRockets, "Destroyer"),
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
        let mut units = base_of(0);
        for id in 0..12 {
            assert!(produce(&mut units, open_ground(), Role::Scout, 0, 10 + id));
        }
        assert!(
            largest_overlap(&units) <= 0.0,
            "{}",
            largest_overlap(&units)
        );
    }

    #[test]
    fn vehicles_made_in_a_hurry_sort_themselves_out_beyond_the_door() {
        let mut units = base_of(0);
        for id in 0..12 {
            assert!(produce(
                &mut units,
                open_ground(),
                Role::RocketLauncher,
                0,
                10 + id
            ));
        }
        for _ in 0..200 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert!(
            largest_overlap(&units) <= 0.001,
            "{}",
            largest_overlap(&units)
        );
        assert!(units.iter().all(|u| !u.is_moving()));
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
        toggle_deploy(&mut units, open_ground());
        assert!(units[0].deployed);
        assert_eq!(units[0].weapon, WeaponKind::Cannon);

        let at = (units[0].x, units[0].z);
        order_move(&mut units, (10.0, 60.0), open_ground(), &[0]);
        for _ in 0..40 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert_eq!((units[0].x, units[0].z), at, "dug in");

        toggle_deploy(&mut units, open_ground());
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
        toggle_deploy(&mut units, open_ground());
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
        toggle_deploy(&mut units, open_ground());
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
            Role::RocketLauncher,
            Role::AdvancedCarryall,
            Role::Tower,
        ] {
            let unit = produced(role);
            assert!(unit.sight() > unit.weapon.range(), "{role:?}");
        }
        assert!(produced(Role::Scout).sight() > produced(Role::LightInfantry).sight());
    }

    fn drone_at(x: f32, z: f32) -> Unit {
        let mut drone = Unit::for_test(x, z, UnitKind::Aircraft);
        drone.role = Role::AirDrone;
        drone.weapon = WeaponKind::AirMissile;
        drone
    }

    #[test]
    fn a_drone_never_stops_it_comes_to_where_it_was_sent_and_circles_there_leaning_in() {
        let terrain = crate::battlefield::terrain(0);
        let start = (0.0, MAP_MIDDLE_Z);
        let mut units = [drone_at(start.0 + 15.0, start.1)];
        units[0].destination = start;
        for _ in 0..200 {
            advance(&mut units, terrain, &[0], 0.05);
        }
        let drone = units[0];
        assert_eq!(drone.altitude, CRUISE_ALTITUDE, "it cannot land");
        let mut least = f32::MAX;
        let mut most = 0.0_f32;
        let mut headings = 0.0;
        for _ in 0..200 {
            let before = (units[0].x, units[0].z, units[0].heading);
            advance(&mut units, terrain, &[0], 0.05);
            let moved = (units[0].x - before.0).hypot(units[0].z - before.1);
            assert!(moved > 0.3, "always on the move: {moved}");
            headings += turn_between(before.2, units[0].heading);
            let away = (units[0].x - start.0).hypot(units[0].z - start.1);
            least = least.min(away);
            most = most.max(away);
        }
        assert!(
            least > CIRCLE_RADIUS * 0.5 && most < CIRCLE_RADIUS * 1.5,
            "{least}..{most}"
        );
        assert!(headings.abs() > TAU, "round and round: {headings}");
        assert_eq!(units[0].bank.signum(), headings.signum(), "into the turn");
        assert!(units[0].bank.abs() > 0.2, "{}", units[0].bank);
    }

    #[test]
    fn a_drone_sent_to_the_edge_or_a_corner_of_the_map_glides_along_the_edge_and_never_sticks() {
        let terrain = crate::battlefield::terrain(0);
        let last = terrain.size().cells() as f32;
        for (u, v) in [
            (last - 0.5, last / 2.0),
            (last / 2.0, 0.5),
            (last - 0.5, last - 0.5),
        ] {
            let edge = crate::battlefield::grid_to_world(u, v);
            let mut units = [drone_at(0.0, MAP_MIDDLE_Z)];
            units[0].destination = edge;
            for _ in 0..800 {
                let before = (units[0].x, units[0].z);
                advance(&mut units, terrain, &[0], 0.05);
                let moved = (units[0].x - before.0).hypot(units[0].z - before.1);
                assert!(moved > 0.3, "stuck at {before:?} going to {edge:?}");
                assert!(terrain.on_map(units[0].x, units[0].z));
            }
            let away = (units[0].x - edge.0).hypot(units[0].z - edge.1);
            assert!(
                away < CIRCLE_RADIUS * 2.5,
                "still about where it was sent: {away}"
            );
        }
    }

    #[test]
    fn a_drone_ordered_to_attack_circles_its_target_and_does_not_stand_still_at_range() {
        let terrain = crate::battlefield::terrain(0);
        let mut enemy = Unit::for_test(0.0, MAP_MIDDLE_Z, UnitKind::Aircraft);
        enemy.owner = 1;
        enemy.id = 7;
        let mut drone = drone_at(9.0, MAP_MIDDLE_Z);
        drone.id = 1;
        drone.target = Some(Target::Unit(7));
        let mut units = [drone, enemy];
        for _ in 0..200 {
            advance(&mut units, terrain, &[0, 1], 0.05);
        }
        assert_eq!(units[0].destination, (0.0, MAP_MIDDLE_Z));
        assert!(units[0].x.hypot(units[0].z - MAP_MIDDLE_Z) < CIRCLE_RADIUS * 1.6);
    }

    fn ornithopter_at(x: f32, z: f32) -> Unit {
        let mut unit = Unit::for_test(x, z, UnitKind::Aircraft);
        unit.role = Role::Ornithopter;
        unit.weapon = WeaponKind::Rockets;
        unit.altitude = 0.0;
        unit.selected = true;
        unit
    }

    #[test]
    fn an_ornithopter_starts_on_the_ground_goes_up_turns_flies_and_then_sits() {
        assert_eq!(produced(Role::Ornithopter).altitude, 0.0);
        let terrain = crate::battlefield::terrain(0);
        let goal = (8.0, MAP_MIDDLE_Z);
        let mut units = [ornithopter_at(0.0, MAP_MIDDLE_Z)];
        assert!(units[0].can_be_repaired(), "while it sits");
        order_move(&mut units, goal, terrain, &[0]);
        let heading = units[0].heading;
        let at_goal = units[0].destination;
        // First only up.
        advance(&mut units, terrain, &[0], 0.5);
        assert!(units[0].altitude > 0.0);
        assert!(!units[0].can_be_repaired());
        assert_eq!((units[0].x, units[0].heading), (0.0, heading));
        while units[0].altitude < CRUISE_ALTITUDE {
            assert_eq!(units[0].x, 0.0, "not off yet");
            advance(&mut units, terrain, &[0], 0.05);
        }
        // Then it turns on the spot.
        let (mut turned, mut ticks) = (false, 0);
        while units[0].x == 0.0 && ticks < 100 {
            turned |= units[0].heading != heading;
            advance(&mut units, terrain, &[0], 0.05);
            ticks += 1;
        }
        assert!(turned, "before it sets off");
        assert_eq!(units[0].altitude, CRUISE_ALTITUDE);
        // Then there, and down.
        for _ in 0..200 {
            advance(&mut units, terrain, &[0], 0.05);
        }
        assert!((units[0].x - at_goal.0).hypot(units[0].z - at_goal.1) <= ARRIVED);
        assert_eq!(units[0].altitude, 0.0, "it sits");
        assert!(!units[0].is_moving());
    }

    #[test]
    fn an_ornithopter_that_can_only_stop_over_water_or_ruins_stays_up() {
        let terrain = crate::battlefield::terrain(0);
        let piece = terrain.ruins()[0];
        let mut units = [ornithopter_at(piece.x, piece.z)];
        units[0].altitude = CRUISE_ALTITUDE;
        advance(&mut units, terrain, &[0], 1.0);
        assert_eq!(units[0].altitude, CRUISE_ALTITUDE);
    }

    #[test]
    fn an_ornithopter_told_to_attack_goes_up_stays_up_to_fire_and_sits_again_once_it_is_over() {
        let terrain = crate::battlefield::terrain(0);
        let mut enemy = Unit::for_test(5.0, MAP_MIDDLE_Z, UnitKind::Infantry);
        enemy.owner = 1;
        enemy.id = 7;
        let mut sitting = ornithopter_at(0.0, MAP_MIDDLE_Z);
        sitting.id = 1;
        let mut units = [sitting, enemy];
        order_attack(&mut units, 1, &[0, 1]);
        assert!(!units[0].can_launch());
        for _ in 0..60 {
            advance(&mut units, terrain, &[0, 1], 0.05);
        }
        assert_eq!(units[0].altitude, CRUISE_ALTITUDE, "the target is in reach");
        assert_eq!(units[0].x, 0.0, "so it stays where it is");
        assert!(units[0].can_launch());
        units[1].alive = false;
        for _ in 0..60 {
            advance(&mut units, terrain, &[0, 1], 0.05);
        }
        assert_eq!(units[0].altitude, 0.0);
    }

    #[test]
    fn a_sitting_ornithopter_takes_off_for_an_enemy_nearby_and_sits_back_once_it_is_safe() {
        let terrain = crate::battlefield::terrain(0);
        let teams = [0, 1];
        let mut enemy = Unit::for_test(5.0, MAP_MIDDLE_Z, UnitKind::Infantry);
        enemy.owner = 1;
        enemy.id = 7;
        let mut sitting = ornithopter_at(0.0, MAP_MIDDLE_Z);
        sitting.id = 1;
        let mut units = [sitting, enemy];
        for _ in 0..60 {
            advance(&mut units, terrain, &teams, 0.05);
        }
        assert_eq!(units[0].altitude, CRUISE_ALTITUDE, "up to fight");
        assert!(units[0].can_launch());
        assert_eq!(units[0].target, None, "by itself, with no order");
        assert_eq!(units[0].x, 0.0, "and it stays where it is");

        units[1].alive = false;
        for _ in 0..60 {
            advance(&mut units, terrain, &teams, 0.05);
        }
        assert_eq!(units[0].altitude, 0.0, "safe, so it sits back");
    }

    #[test]
    fn a_sitting_ornithopter_stays_down_beside_an_enemy_building_that_makes_units() {
        let terrain = crate::battlefield::terrain(0);
        let mut barracks = Unit::for_test(4.0, MAP_MIDDLE_Z, UnitKind::Building);
        barracks.owner = 1;
        let mut units = [ornithopter_at(0.0, MAP_MIDDLE_Z), barracks];
        for _ in 0..40 {
            advance(&mut units, terrain, &[0, 1], 0.05);
        }
        assert_eq!(units[0].altitude, 0.0);
    }

    #[test]
    fn a_sitting_ornithopter_stays_down_for_far_off_enemies_and_friends() {
        let terrain = crate::battlefield::terrain(0);
        let mut far = Unit::for_test(20.0, MAP_MIDDLE_Z, UnitKind::Infantry);
        far.owner = 1;
        let mut friend = Unit::for_test(3.0, MAP_MIDDLE_Z, UnitKind::Infantry);
        friend.owner = 2;
        let mut units = [ornithopter_at(0.0, MAP_MIDDLE_Z), far, friend];
        for _ in 0..40 {
            advance(&mut units, terrain, &[0, 1, 0], 0.05);
        }
        assert_eq!(units[0].altitude, 0.0);
    }

    #[test]
    fn an_ornithopter_going_up_for_a_distant_enemy_really_has_it_in_reach_from_up_there() {
        let terrain = crate::battlefield::terrain(0);
        let reach = WeaponKind::Rockets.range();
        let mut ornithopter = ornithopter_at(0.0, MAP_MIDDLE_Z);
        ornithopter.id = 1;
        let mut enemy = Unit::for_test(reach - 2.0, MAP_MIDDLE_Z, UnitKind::Infantry);
        enemy.owner = 1;
        enemy.id = 7;
        let mut units = [ornithopter, enemy];
        for unit in &mut units {
            unit.y = terrain.height_at(unit.x, unit.z);
        }
        for _ in 0..60 {
            advance(&mut units, terrain, &[0, 1], 0.05);
        }
        let up_there = distance(units[0].center(), units[1].center());
        assert_eq!(units[0].altitude, CRUISE_ALTITUDE, "{up_there}");
        assert!(up_there <= reach, "{up_there}");
    }

    #[test]
    fn nobody_shoves_a_sitting_ornithopter() {
        let terrain = crate::battlefield::terrain(0);
        let tank_at = 0.4;
        let mut tank = Unit::for_test(tank_at, MAP_MIDDLE_Z, UnitKind::Vehicle);
        tank.destination = (5.0, MAP_MIDDLE_Z);
        let mut units = [ornithopter_at(0.0, MAP_MIDDLE_Z), tank];
        units[0].selected = false;
        advance(&mut units, terrain, &[0], 0.05);
        assert_eq!(units[0].x, 0.0);
    }

    #[test]
    fn every_opponent_has_aircraft_at_the_start_and_they_stay_up() {
        let terrain = crate::battlefield::terrain(0);
        let colors = [TeamColor::Blue, TeamColor::Red];
        let units = spawn_bases(terrain, &colors);
        let of_opponent = |role| units.iter().any(|u| u.owner == 1 && u.role == role);
        assert!(of_opponent(Role::AdvancedCarryall) && of_opponent(Role::AirDrone));
    }

    fn open_map() -> Terrain {
        Terrain::open_with_ruins(Vec::new())
    }

    fn carry_scene() -> (Vec<Unit>, Vec<Carried>) {
        let mut carryall = Unit::for_test(0.0, MAP_MIDDLE_Z, UnitKind::Aircraft);
        carryall.role = Role::AdvancedCarryall;
        carryall.weapon = WeaponKind::Unarmed;
        carryall.id = 1;
        carryall.selected = true;
        let mut tank = Unit::for_test(4.0, MAP_MIDDLE_Z, UnitKind::Vehicle);
        tank.id = 2;
        tank.health = 123.0;
        (vec![carryall, tank], Vec::new())
    }

    fn run_carry(units: &mut Vec<Unit>, carried: &mut Vec<Carried>, seconds: f32) {
        let terrain = crate::battlefield::terrain(0);
        for _ in 0..(seconds / 0.05) as usize {
            advance(units, terrain, &[0], 0.05);
            carry(units, carried, terrain, &[0], 0.05);
        }
    }

    /// An enemy tank on open ground, and a player's carryall sent to steal it.
    fn theft_scene() -> Vec<Unit> {
        let (mut units, _) = carry_scene();
        units[1].owner = 1;
        order_attack(&mut units, 1, &[0, 1]);
        units
    }

    fn run_theft(units: &mut Vec<Unit>, terrain: &Terrain, seconds: f32) {
        let mut carried = Vec::new();
        for _ in 0..(seconds / 0.05) as usize {
            advance(units, terrain, &[0, 1], 0.05);
            carry(units, &mut carried, terrain, &[0, 1], 0.05);
        }
        assert!(carried.is_empty(), "a stolen vehicle stays in the battle");
    }

    /// Runs a theft until the carryall has hold of the tank.
    fn seized(terrain: &Terrain) -> Vec<Unit> {
        let mut units = theft_scene();
        for _ in 0..200 {
            run_theft(&mut units, terrain, 0.05);
            if units[1].theft.is_some() {
                return units;
            }
        }
        panic!("it never got hold: {units:?}");
    }

    #[test]
    fn a_carryall_takes_hold_of_an_enemy_vehicle_fights_it_for_a_while_then_lifts_it() {
        let terrain = open_map();
        let units = theft_scene();
        assert_eq!(units[0].target, Some(Target::Unit(units[1].id)));
        let mut units = seized(&terrain);
        assert!(units[0].carrying);
        assert_eq!(units[1].altitude, 0.0, "still on the ground");
        let spot = (units[1].x, units[1].z);
        run_theft(&mut units, &terrain, STRUGGLE_SECONDS - 0.5);
        assert_eq!(units[1].altitude, 0.0, "still fighting it");
        assert_eq!((units[1].x, units[1].z), spot);
        run_theft(&mut units, &terrain, 2.0);
        assert!(units[1].altitude > 0.0, "up it goes");
        assert!(units[1].alive);
        assert_eq!(units[1].owner, 1, "it is still the enemy's");
    }

    #[test]
    fn what_a_carryall_has_hold_of_goes_nowhere_and_neither_does_it_until_the_fight_is_over() {
        let terrain = open_map();
        let mut units = seized(&terrain);
        let spot = (units[1].x, units[1].z);
        units[1].destination = (spot.0 + 5.0, spot.1);
        units[0].selected = true;
        order_move(&mut units, (-10.0, MAP_MIDDLE_Z), &terrain, &[0, 1]);
        run_theft(&mut units, &terrain, 1.0);
        assert_eq!((units[1].x, units[1].z), spot);
        assert!((units[0].x - spot.0).hypot(units[0].z - spot.1) < 0.001);
        assert!(!units[1].is_selectable());
    }

    #[test]
    fn a_stolen_vehicle_goes_along_and_is_put_down_where_sent_free_again_and_still_the_enemys() {
        let terrain = open_map();
        let mut units = seized(&terrain);
        run_theft(&mut units, &terrain, STRUGGLE_SECONDS + 1.0);
        units[0].selected = true;
        order_move(&mut units, (-10.0, MAP_MIDDLE_Z), &terrain, &[0, 1]);
        run_theft(&mut units, &terrain, 1.0);
        assert!(units[1].altitude > 0.0, "it hangs on the way");
        assert_eq!((units[1].x, units[1].z), (units[0].x, units[0].z));
        run_theft(&mut units, &terrain, 8.0);
        assert_eq!(units[1].theft, None);
        assert_eq!(units[1].altitude, 0.0);
        assert!(!units[0].carrying);
        assert_eq!(units[1].owner, 1);
        assert!(units[1].x < -8.0, "{:?}", units[1]);
    }

    #[test]
    fn a_stolen_vehicle_dies_with_its_carryall_whether_held_or_up_in_the_air() {
        let terrain = open_map();
        let mut units = seized(&terrain);
        units[0].alive = false;
        run_theft(&mut units, &terrain, 0.1);
        assert!(!units[1].alive, "while they fight");

        let mut units = seized(&terrain);
        run_theft(&mut units, &terrain, STRUGGLE_SECONDS + 1.0);
        units[0].alive = false;
        run_theft(&mut units, &terrain, 0.1);
        assert!(!units[1].alive, "up in the air");
    }

    #[test]
    fn a_carryall_whose_stolen_vehicle_is_destroyed_has_its_hands_free() {
        let terrain = open_map();
        let mut units = seized(&terrain);
        run_theft(&mut units, &terrain, STRUGGLE_SECONDS + 1.0);
        units[1].alive = false;
        run_theft(&mut units, &terrain, 0.1);
        assert!(units[0].alive);
        assert!(!units[0].carrying);
    }

    #[test]
    fn a_carryall_steals_only_enemy_ground_vehicles_one_at_a_time() {
        let (units, _) = carry_scene();
        let carryall = units[0];
        let enemy_tank = Unit {
            owner: 1,
            ..units[1]
        };
        let teams = [0, 1];
        assert!(carryall.can_steal(&enemy_tank, &teams));
        assert!(
            !carryall.can_steal(&units[1], &teams),
            "a friend is lifted, not stolen"
        );
        let soldier = Unit {
            kind: UnitKind::Infantry,
            ..enemy_tank
        };
        assert!(!carryall.can_steal(&soldier, &teams));
        let plane = Unit {
            kind: UnitKind::Aircraft,
            ..enemy_tank
        };
        assert!(!carryall.can_steal(&plane, &teams));
        let busy = Unit {
            carrying: true,
            ..carryall
        };
        assert!(!busy.can_steal(&enemy_tank, &teams));
        let tank_of_ours = units[1];
        assert!(!tank_of_ours.can_steal(&enemy_tank, &teams));
    }

    #[test]
    fn a_carryall_goes_to_a_friendly_vehicle_picks_it_up_and_puts_it_down_where_sent() {
        let terrain = crate::battlefield::terrain(0);
        let (mut units, mut carried) = carry_scene();
        assert_eq!(units[0].weapon, Role::AdvancedCarryall.weapon());
        order_attack(&mut units, 1, &[0]);
        assert_eq!(units[0].target, Some(Target::Unit(2)));
        run_carry(&mut units, &mut carried, 2.0);
        assert_eq!(units.len(), 1, "out of the battle");
        assert_eq!(carried.len(), 1);
        assert!(units[0].carrying && units[0].target.is_none());
        assert!(carried[0].vehicle.y > 2.0, "hauled up");
        assert_eq!(
            (carried[0].vehicle.x, carried[0].vehicle.z),
            (units[0].x, units[0].z)
        );

        // It stays up until it is told where to put it.
        run_carry(&mut units, &mut carried, 2.0);
        assert_eq!(carried.len(), 1);

        let goal = (9.0, MAP_MIDDLE_Z + 2.0);
        order_move(&mut units, goal, terrain, &[0]);
        run_carry(&mut units, &mut carried, 4.0);
        assert!(carried.is_empty());
        assert!(!units[0].carrying);
        let tank = units
            .iter()
            .find(|u| u.id == 2)
            .expect("back on the ground");
        assert!((tank.x - goal.0).hypot(tank.z - goal.1) < 0.2, "{tank:?}");
        assert_eq!(tank.y, terrain.height_at(tank.x, tank.z));
        assert_eq!(tank.health, 123.0, "as it was");
        assert!(!tank.selected);
        assert!(!tank.is_moving(), "it stays where it was put");

        // And flying on with nothing, it does not try to put anything down.
        order_move(&mut units, (0.0, MAP_MIDDLE_Z), terrain, &[0]);
        assert!(!units.iter().any(|u| u.unloading));
    }

    #[test]
    fn a_vehicle_dies_with_the_carryall_carrying_it() {
        let (mut units, mut carried) = carry_scene();
        order_attack(&mut units, 1, &[0]);
        run_carry(&mut units, &mut carried, 2.0);
        units[0].alive = false;
        run_carry(&mut units, &mut carried, 0.05);
        assert!(carried.is_empty());
        let tank = units
            .iter()
            .find(|u| u.id == 2)
            .expect("left to be counted");
        assert!(!tank.alive);
    }

    #[test]
    fn a_carryall_only_lifts_friendly_vehicles_one_at_a_time() {
        let (units, _) = carry_scene();
        let (carryall, tank) = (units[0], units[1]);
        assert!(carryall.can_lift(&tank, &[0]));
        assert!(
            carryall.can_lift(&Unit { owner: 1, ..tank }, &[0, 0]),
            "an ally"
        );
        assert!(
            !carryall.can_lift(&Unit { owner: 1, ..tank }, &[0, 1]),
            "an enemy"
        );
        let soldier = Unit::for_test(1.0, 60.0, UnitKind::Infantry);
        assert!(!carryall.can_lift(&soldier, &[0]));
        assert!(!carryall.can_lift(&carryall, &[0]));
        let full = Unit {
            carrying: true,
            ..carryall
        };
        assert!(!full.can_lift(&tank, &[0]));
        assert!(!tank.can_lift(&carryall, &[0]));
        assert!(!full.keeps_to_the_air(), "it wants a spot on the ground");
    }

    fn base_of(owner: usize) -> Vec<Unit> {
        let terrain = open_ground();
        let mut units = Vec::new();
        place_base(
            &mut units,
            terrain,
            (0.0, 60.0),
            (0.0, 1.0),
            (owner, TeamColor::Red),
        );
        units
    }

    #[test]
    fn each_building_makes_its_own_kind_of_unit_for_its_owner_only() {
        let mut units = base_of(1);
        assert!(
            !produce(&mut units, open_ground(), Role::RocketLauncher, 0, 50),
            "not theirs"
        );
        for (role, kind, maker) in [
            (Role::Scout, UnitKind::Infantry, Role::Barracks),
            (Role::RocketLauncher, UnitKind::Vehicle, Role::Factory),
            (Role::AirDrone, UnitKind::Aircraft, Role::Hangar),
        ] {
            assert_eq!(role.kind(), kind);
            assert_eq!(maker_of(&units, role, 1).map(|u| u.role), Some(maker));
            assert!(produce(&mut units, open_ground(), role, 1, 50));
            let new = units.last().unwrap();
            assert_eq!((new.role, new.owner, new.color), (role, 1, TeamColor::Red));
        }
        assert_eq!(Role::Barracks.made_in(), None);
    }

    #[test]
    fn a_building_makes_what_its_house_makes_and_every_barracks_scouts_and_engineers() {
        for (house, own, other) in [
            (
                Faction::A,
                [Role::LightInfantry, Role::SandBike],
                Role::Tank,
            ),
            (
                Faction::H,
                [Role::RocketInfantry, Role::DiskSaw],
                Role::SandBike,
            ),
            (
                Faction::O,
                [Role::SuicideInfantry, Role::SiegeTank],
                Role::KInfantry,
            ),
        ] {
            let mut units = base_of(0);
            for unit in &mut units {
                unit.house = house;
            }
            for role in own
                .into_iter()
                .chain([Role::Scout, Role::Engineer, Role::Mcv])
            {
                assert!(has_free_maker(&units, role, 0), "{house:?} {role:?}");
            }
            assert!(!has_free_maker(&units, other, 0), "{house:?} {other:?}");
            assert!(!start_making(&mut units, other, 0));
            let new = units.len();
            assert!(produce(&mut units, open_ground(), own[1], 0, 70));
            assert_eq!(units[new].house, house, "made by its house");
        }
    }

    #[test]
    fn with_its_building_gone_nothing_of_that_kind_can_be_made() {
        for (role, maker) in [
            (Role::Sniper, Role::Barracks),
            (Role::Catapult, Role::Factory),
            (Role::Ornithopter, Role::Hangar),
        ] {
            let mut units = base_of(0);
            let before = units.len();
            units.iter_mut().find(|u| u.role == maker).unwrap().alive = false;
            assert!(!produce(&mut units, open_ground(), role, 0, 50), "{role:?}");
            assert_eq!(units.len(), before);
        }
    }

    #[test]
    fn a_vehicle_comes_out_of_the_factory_door_and_drives_away_from_it() {
        let mut units = base_of(0);
        let factory = *units.iter().find(|u| u.role == Role::Factory).unwrap();
        assert!(produce(
            &mut units,
            open_ground(),
            Role::RocketLauncher,
            0,
            50
        ));
        let tank = *units.last().unwrap();
        let touching = factory.radius() + tank.radius();
        let away = (tank.x - factory.x).hypot(tank.z - factory.z);
        assert!((away - touching).abs() < 0.5, "at the door");
        assert!(
            ((tank.x - factory.x) + (tank.z - factory.z)).abs() < 0.01,
            "out along the grid"
        );
        assert!(tank.is_moving(), "on its way out");
        assert!(tank.destination.1 > tank.z);
        for _ in 0..100 {
            advance(&mut units, open_ground(), &[0], 0.05);
        }
        assert!(!units.last().unwrap().is_moving(), "it gets there");
    }

    #[test]
    fn infantry_appears_by_the_barracks_without_touching_it() {
        let mut units = base_of(0);
        let barracks = *units.iter().find(|u| u.role == Role::Barracks).unwrap();
        assert!(produce(&mut units, open_ground(), Role::Scout, 0, 50));
        let scout = units.last().unwrap();
        let away = (scout.x - barracks.x).hypot(scout.z - barracks.z);
        assert!(away > barracks.radius() + scout.radius(), "{away}");
        assert!(away < 4.0, "{away}");
        assert!(!scout.is_moving());
    }

    #[test]
    fn nobody_is_sent_to_a_spot_inside_a_building() {
        let mut units = base_of(1);
        let barracks = *units.iter().find(|u| u.role == Role::Barracks).unwrap();
        let mut tank = unit_at(barracks.x + 8.0, barracks.z, UnitKind::Vehicle);
        tank.selected = true;
        units.push(tank);
        order_move(&mut units, (barracks.x, barracks.z), open_ground(), &[0, 1]);
        let sent = units.last().unwrap().destination;
        let away = (sent.0 - barracks.x).hypot(sent.1 - barracks.z);
        assert!(
            away > barracks.radius() + UnitKind::Vehicle.radius(),
            "{away}"
        );
    }

    #[test]
    fn everyone_starts_with_all_three_buildings_on_land_and_an_engineer_can_take_them() {
        let terrain = terrain(0);
        let units = spawn_bases(terrain, &COLORS);
        for owner in 0..COLORS.len() {
            for role in [
                Role::Barracks,
                Role::Factory,
                Role::Hangar,
                Role::LandingPad,
            ] {
                let building = units.iter().find(|u| u.owner == owner && u.role == role);
                let building = building.expect("all four");
                assert!(terrain.can_walk(building.x, building.z));
                assert_eq!(building.weapon, WeaponKind::Unarmed);
                assert_eq!(building.is_selectable(), owner == 0);
                assert!(!building.is_selected_in_groups());
            }
        }
        let mut engineer = unit_at(0.0, 60.0, UnitKind::Infantry);
        engineer.role = Role::Engineer;
        let enemy = units
            .iter()
            .find(|u| u.owner == 1 && u.role == Role::Factory);
        assert!(engineer.can_capture(enemy.unwrap(), &[0, 1]));
    }

    #[test]
    fn striders_are_vehicles_made_in_a_factory_whose_stride_grows_as_they_walk() {
        for role in [Role::Mongoose, Role::Minotaurus] {
            assert_eq!(role.kind(), UnitKind::Vehicle);
            assert_eq!(role.made_in(), Some(Role::Factory));
            let mut unit = produced(role);
            unit.destination = (unit.x + 5.0, unit.z);
            unit.heading = 0.0;
            let mut units = [unit];
            advance(&mut units, open_ground(), &[0], 0.5);
            assert!(units[0].stride > 0.5, "{role:?} {}", units[0].stride);
            assert!(
                units[0].pick_height() > HEIGHT / 2.0,
                "the body is up on its legs"
            );
        }
    }

    #[test]
    fn new_units_go_on_to_their_buildings_gathering_point_each_to_its_own_spot() {
        let mut units = base_of(0);
        let point = (6.0, 52.0);
        for building in &mut units {
            building.selected = building.role == Role::Barracks;
        }
        set_gathering(&mut units, point);
        let barracks = units.iter().find(|u| u.role == Role::Barracks).unwrap();
        assert_eq!(barracks.gathering, Some(point));
        let factory = units.iter().find(|u| u.role == Role::Factory).unwrap();
        assert_eq!(factory.gathering, None, "not selected");
        assert!(produce(&mut units, open_ground(), Role::Scout, 0, 50));
        assert!(produce(&mut units, open_ground(), Role::Scout, 0, 51));
        let [.., first, second] = &units[..] else {
            panic!()
        };
        assert!(first.is_moving() && second.is_moving());
        assert_eq!(first.destination, point);
        assert_ne!(second.destination, point);
        let near = |u: &Unit| (u.destination.0 - point.0).hypot(u.destination.1 - point.1);
        assert!(near(second) < 3.0, "{second:?}");
        assert!(first.z > 60.0, "they set off from the building");
    }

    #[test]
    fn a_selected_tower_takes_no_move_order_but_fires_on_what_it_is_told() {
        let mut tower = unit_at(0.0, 60.0, UnitKind::Tower);
        tower.selected = true;
        tower.id = 1;
        tower.weapon = WeaponKind::Missile;
        let mut enemy = enemy_at(30.0, 60.0, UnitKind::Infantry);
        enemy.id = 2;
        let mut units = [tower, enemy];
        order_move(&mut units, (5.0, 60.0), open_ground(), &TWO_SIDES);
        assert_eq!(units[0].destination, (0.0, 60.0));
        order_attack(&mut units, 1, &TWO_SIDES);
        assert_eq!(units[0].target, Some(Target::Unit(2)));
        advance(&mut units, open_ground(), &TWO_SIDES, 0.5);
        assert_eq!(
            units[0].destination,
            (0.0, 60.0),
            "not even after a far target"
        );
        assert!(!units[0].is_moving());
        assert_eq!(units[0].x, 0.0);

        order_move(&mut units, (5.0, 60.0), open_ground(), &TWO_SIDES);
        assert_eq!(units[0].target, None, "a plain click calls it off");
    }

    #[test]
    fn nobody_is_sent_inside_a_building_that_happens_to_be_selected() {
        let mut units = base_of(0);
        let barracks = *units.iter().find(|u| u.role == Role::Barracks).unwrap();
        units[0].selected = true;
        units
            .iter_mut()
            .find(|u| u.role == Role::Barracks)
            .unwrap()
            .selected = true;
        let mut tank = unit_at(barracks.x + 8.0, barracks.z, UnitKind::Vehicle);
        tank.selected = true;
        units.push(tank);
        order_move(&mut units, (barracks.x, barracks.z), open_ground(), &[0]);
        let sent = units.last().unwrap().destination;
        let away = (sent.0 - barracks.x).hypot(sent.1 - barracks.z);
        assert!(
            away > barracks.radius() + UnitKind::Vehicle.radius(),
            "{away}"
        );
    }

    #[test]
    fn a_building_makes_one_thing_at_a_time_and_lets_it_out_when_it_is_done() {
        let mut units = base_of(0);
        let barracks = units.iter().position(|u| u.role == Role::Barracks).unwrap();
        let known = units.len();
        assert!(start_making(&mut units, Role::Scout, 0));
        assert!(
            start_making(&mut units, Role::Sniper, 0),
            "it waits its turn"
        );
        assert_eq!(units[barracks].orders_for(Role::Sniper), 1);
        assert!(
            start_making(&mut units, Role::RocketLauncher, 0),
            "the factory is not"
        );
        assert_eq!(units.len(), known, "nothing appears yet");
        assert_eq!(units[barracks].making.unwrap().0, Role::Scout);

        let seconds = Role::Scout.build_seconds();
        assert!(work_on_making(&mut units, seconds / 2.0).is_empty());
        assert_eq!(work_on_making(&mut units, seconds / 2.0), [barracks]);
        let new = finish_making(&mut units, open_ground(), barracks, 50).unwrap();
        assert_eq!(units[new].role, Role::Scout);
        assert_eq!(
            units[barracks].making.map(|(role, _)| role),
            Some(Role::Sniper),
            "the next one starts"
        );
        assert_eq!(units[barracks].orders_for(Role::Sniper), 1);
        assert_eq!(units[barracks].waiting(), 0);
        assert!(!has_free_maker(&units, Role::Scout, 0));
        work_on_making(&mut units, Role::Sniper.build_seconds());
        finish_making(&mut units, open_ground(), barracks, 51).unwrap();
        assert_eq!(units[barracks].making, None, "nothing left");
        assert!(has_free_maker(&units, Role::Scout, 0));
    }

    #[test]
    fn a_queue_is_kept_in_the_order_given_and_has_an_end() {
        let mut units = base_of(0);
        let barracks = units.iter().position(|u| u.role == Role::Barracks).unwrap();
        let order = [
            Role::Scout,
            Role::Sniper,
            Role::Scout,
            Role::Engineer,
            Role::Sniper,
        ];
        for role in order {
            assert!(start_making(&mut units, role, 0));
        }
        let mut made = Vec::new();
        for id in 0..order.len() as u32 {
            let role = units[barracks].making.unwrap().0;
            work_on_making(&mut units, role.build_seconds());
            let new = finish_making(&mut units, open_ground(), barracks, 50 + id).unwrap();
            made.push(units[new].role);
        }
        assert_eq!(made, order);

        assert!(start_making(&mut units, Role::Scout, 0), "making it");
        for _ in 0..QUEUE_ROOM {
            assert!(start_making(&mut units, Role::Scout, 0));
        }
        assert!(!start_making(&mut units, Role::Scout, 0), "no more room");
    }

    #[test]
    fn cancelling_takes_one_of_that_type_from_anywhere_and_leaves_the_rest_in_order() {
        let mut units = base_of(0);
        let barracks = units.iter().position(|u| u.role == Role::Barracks).unwrap();
        let order = [
            Role::Scout,
            Role::Sniper,
            Role::Engineer,
            Role::Sniper,
            Role::Engineer,
        ];
        for role in order {
            start_making(&mut units, role, 0);
        }
        work_on_making(&mut units, 1.0);
        let left = units[barracks].making.unwrap().1;

        // From the middle of the queue, with what is being made carrying on.
        assert!(cancel_making(&mut units, Role::Sniper, 0));
        assert_eq!(units[barracks].orders_for(Role::Sniper), 1);
        assert_eq!(units[barracks].making, Some((Role::Scout, left)));
        assert_eq!(
            units[barracks].queue[..4],
            [
                Some(Role::Sniper),
                Some(Role::Engineer),
                Some(Role::Engineer),
                None
            ]
        );

        // The one being made: the next starts from the beginning.
        assert!(cancel_making(&mut units, Role::Scout, 0));
        assert_eq!(
            units[barracks].making,
            Some((Role::Sniper, Role::Sniper.build_seconds()))
        );
        assert_eq!(units[barracks].orders_for(Role::Scout), 0);

        assert!(!cancel_making(&mut units, Role::Scout, 0), "none left");
        assert!(cancel_making(&mut units, Role::Sniper, 0));
        assert!(cancel_making(&mut units, Role::Engineer, 0));
        assert!(cancel_making(&mut units, Role::Engineer, 0));
        assert_eq!(units[barracks].making, None);
        assert!(has_free_maker(&units, Role::Scout, 0));
    }

    #[test]
    fn with_two_buildings_of_a_kind_the_work_goes_to_the_one_with_less_to_do() {
        let mut units = base_of(0);
        let mut second = *units.iter().find(|u| u.role == Role::Barracks).unwrap();
        second.x += 10.0;
        second.id = 40;
        units.push(second);
        for _ in 0..4 {
            assert!(start_making(&mut units, Role::Scout, 0));
        }
        let orders: Vec<usize> = units
            .iter()
            .filter(|u| u.role == Role::Barracks)
            .map(|u| u.orders_for(Role::Scout))
            .collect();
        assert_eq!(orders, [2, 2]);
    }

    #[test]
    fn every_role_a_building_makes_takes_time_and_a_heavy_one_takes_longer() {
        for role in [
            Role::Scout,
            Role::LightInfantry,
            Role::Tank,
            Role::AirDrone,
            Role::AdvancedCarryall,
        ] {
            assert!(role.build_seconds() > 0.0, "{role:?}");
        }
        assert!(Role::Minotaurus.build_seconds() > Role::Tank.build_seconds());
        assert!(Role::Tank.build_seconds() > Role::Scout.build_seconds());
    }

    #[test]
    fn a_building_snaps_to_grid_squares_jumping_a_whole_square_at_a_time() {
        let (x, z) = (10.0, 60.0);
        let plot = Role::Tower.plot_at((x, z));
        assert_eq!(plot.cells().count(), 4, "a tower takes two by two");
        assert_eq!(
            plot,
            Role::Tower.plot_at((x + 0.05, z + 0.05)),
            "no gliding"
        );
        let next = Role::Tower.plot_at((x + 0.5, z + 0.5));
        assert_eq!((next.u, next.v), (plot.u + 1, plot.v), "a whole square on");
        assert_eq!(Role::Tower.plot_at(plot.center()), plot, "and back");
        assert_eq!(Role::Factory.plot_at((x, z)).cells().count(), 30);
    }

    #[test]
    fn a_new_building_can_be_at_most_two_empty_squares_from_the_nearest_building() {
        let mut tower = Unit::for_test(0.0, 60.0, UnitKind::Tower);
        tower.role = Role::Tower;
        let mine = tower.plot().unwrap();
        let units = [tower];
        let at = |du: i32, dv: i32| Plot {
            u: mine.u + du,
            v: mine.v + dv,
            cols: 2,
            rows: 2,
        };
        let near = |plot: Plot| is_near_building_of(&units, plot, 0);
        assert!(near(at(2, 0)), "right beside it");
        assert!(near(at(4, 0)), "two empty squares between");
        assert!(!near(at(5, 0)), "three is too far");
        assert!(near(at(-4, 0)), "the other way too");
        assert!(!near(at(-5, 0)));
        assert!(
            near(at(0, 4)) && !near(at(0, 5)),
            "and along the other axis"
        );
        assert!(near(at(4, 4)), "any dimension counts, diagonally as well");
        assert!(!near(at(5, 4)));
        assert!(
            !is_near_building_of(&units, at(2, 0), 1),
            "only its owner's"
        );
    }

    #[test]
    fn a_building_goes_up_only_between_the_frames_rising_and_coming_down() {
        let mut barracks = Unit::for_test(0.0, 60.0, UnitKind::Building);
        let at = |done: f32| {
            let mut unit = barracks;
            unit.constructing = CONSTRUCTION_SECONDS * (1.0 - done);
            unit.raised()
        };
        assert_eq!(at(0.2), 0.0, "frames first");
        assert!(at(0.5) > 0.0 && at(0.5) < 1.0, "then it grows");
        assert_eq!(at(0.8), 1.0, "and stands whole as they come down");
        barracks.constructing = 0.0;
        assert_eq!(barracks.raised(), 1.0);
    }

    #[test]
    fn a_new_building_needs_clear_land() {
        assert!(
            !can_place_building(&[], open_ground(), Role::Barracks.plot_at((0.0, 60.0))),
            "not on the sand"
        );
        let upland = Terrain::open_upland();
        let terrain = &upland;
        let units = base_of(0);
        let barracks = units.iter().find(|u| u.role == Role::Barracks).unwrap();
        assert!(!can_place_building(
            &units,
            terrain,
            Role::Barracks.plot_at((barracks.x, barracks.z))
        ));
        assert!(!can_place_building(
            &units,
            terrain,
            Role::Barracks.plot_at((barracks.x + 1.5, barracks.z))
        ));
        assert!(can_place_building(
            &units,
            terrain,
            Role::Barracks.plot_at((barracks.x - 14.0, barracks.z))
        ));
        let mut plane = Unit::for_test(barracks.x + 8.0, barracks.z, UnitKind::Aircraft);
        plane.alive = true;
        assert!(
            can_place_building(
                &[plane],
                terrain,
                Role::Barracks.plot_at((plane.x, plane.z))
            ),
            "what flies is not in the way"
        );
        let sitting = Unit {
            altitude: 0.0,
            ..plane
        };
        assert!(!can_place_building(
            &[sitting],
            terrain,
            Role::Barracks.plot_at((plane.x, plane.z))
        ));
        assert!(
            !can_place_building(&[], terrain, Role::Barracks.plot_at((-1000.0, 60.0))),
            "off the map"
        );
    }

    #[test]
    fn enemies_are_told_apart_by_team() {
        assert!(are_enemies(&[0, 1, 2], 0, 1));
        assert!(!are_enemies(&[0, 1, 0], 0, 2), "the same team is allied");
        assert!(!are_enemies(&[0, 1], 1, 1));
        assert!(!are_enemies(&[], 0, 1), "no teams, no fights");
    }

    #[test]
    fn opponents_have_no_vehicles_and_the_player_has_vehicles_that_are_heavy() {
        let units = spawn_bases(terrain(0), &COLORS);
        assert!(
            units
                .iter()
                .filter(|u| u.owner != 0)
                .all(|u| u.kind != UnitKind::Vehicle)
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
    fn a_laser_tank_glides_off_sideways_at_once_turning_its_nose_slowly_meanwhile() {
        let mut glider = Unit::for_test(0.0, 60.0, UnitKind::Vehicle);
        glider.role = Role::LaserTank;
        glider.heading = 0.0;
        glider.destination = (0.0, 66.0);
        let mut tank = Unit::for_test(10.0, 60.0, UnitKind::Vehicle);
        tank.role = Role::Tank;
        tank.heading = 0.0;
        tank.destination = (10.0, 66.0);
        let mut units = [glider, tank];
        advance(&mut units, open_ground(), &[0], 0.25);
        assert!(units[0].z > 60.5, "off at once: {}", units[0].z);
        assert!(units[1].z < 60.1, "a tank turns first");
        assert!(
            units[0].heading > 0.1 && units[0].heading < 0.5,
            "{}",
            units[0].heading
        );
    }

    #[test]
    fn a_recons_legs_step_as_it_turns_on_the_spot() {
        let mut recon = Unit::for_test(0.0, 60.0, UnitKind::Vehicle);
        recon.role = Role::Recon;
        recon.heading = 0.0;
        recon.destination = (-6.0, 60.0);
        let mut units = [recon];
        advance(&mut units, open_ground(), &[0], 0.2);
        assert!(units[0].x > -0.01, "still turning around");
        assert!(units[0].stride > 0.1, "{}", units[0].stride);
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
                    assert!(gap > unit.radius() * 0.5, "{unit:?} inside {piece:?}");
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
                largest = largest.max(a.radius() + b.radius() - distance);
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
    fn units_walk_around_a_mountain_and_aircraft_go_straight_over_it() {
        let terrain = &Terrain::open_with_mountain(0.0, 40.0, 5.0);
        let mut tank = unit_at(0.0, 31.0, UnitKind::Vehicle);
        tank.destination = (0.0, 49.0);
        let mut plane = unit_at(1.0, 31.0, UnitKind::Aircraft);
        plane.altitude = 2.0;
        plane.destination = (1.0, 49.0);
        let mut units = [tank, plane];
        let mut plane_crossed = false;
        for _ in 0..4000 {
            advance(&mut units, terrain, &[0], 0.02);
            assert!(terrain.can_walk(units[0].x, units[0].z), "{:?}", units[0]);
            plane_crossed |= (units[1].z - 40.0).abs() < 0.5 && units[1].x.abs() < 1.5;
        }
        assert!(plane_crossed, "right over it: {:?}", units[1]);
        assert!(
            (units[0].x - 0.0).hypot(units[0].z - 49.0) < 0.5,
            "it got there: {:?}",
            units[0]
        );
    }

    /// A flat map with a pad at its middle and a machine out of rockets some way off.
    fn machine_and_pad(role: Role) -> [Unit; 2] {
        let mut machine = unit_at(0.0, 50.0, UnitKind::Aircraft);
        machine.role = role;
        machine.weapon = role.weapon();
        machine.id = 1;
        machine.altitude = CRUISE_ALTITUDE;
        machine.rockets_fired = role.rocket_load().unwrap();
        let mut pad = unit_at(8.0, 50.0, UnitKind::Building);
        pad.role = Role::LandingPad;
        pad.weapon = WeaponKind::Unarmed;
        pad.id = 2;
        [machine, pad]
    }

    /// A machine up in the air on a mission against an enemy that is then gone.
    fn after_a_mission(units: &mut [Unit], terrain: &Terrain) {
        units[0].rockets_fired = 0;
        units[0].target = Some(Target::Unit(999));
        for _ in 0..600 {
            advance(units, terrain, &[0], 0.05);
        }
    }

    #[test]
    fn after_a_mission_a_machine_goes_to_its_pad_however_far_off() {
        let terrain = open_map();
        let mut units = machine_and_pad(Role::Ornithopter);
        units[1].x = PAD_NEARBY + 12.0;
        after_a_mission(&mut units, &terrain);
        assert!(
            (units[0].x - units[1].x).hypot(units[0].z - 50.0) < OVER_PAD,
            "{:?}",
            units[0]
        );
        assert_eq!(units[0].altitude, PAD_TOP, "on its deck");
    }

    #[test]
    fn after_a_mission_with_no_pad_a_machine_goes_back_to_its_base_by_the_mcv() {
        let terrain = open_map();
        let mut units = machine_and_pad(Role::Destroyer);
        units[1].role = Role::Mcv;
        units[1].deployed = true;
        units[1].x = -14.0;
        after_a_mission(&mut units, &terrain);
        let away = (units[0].x - units[1].x).hypot(units[0].z - units[1].z);
        assert!(away < HOME_BESIDE + 1.0, "{away}");
        assert_eq!(units[0].altitude, 0.0, "sitting there");
    }

    #[test]
    fn after_a_mission_with_no_home_a_machine_sits_down_on_higher_ground_than_the_sand() {
        let terrain = crate::battlefield::terrain(0);
        assert_eq!(terrain.floor(), 0.0, "a map with sand");
        // Sand with higher ground close by.
        let sand = (0..200)
            .flat_map(|i| (0..200).map(move |j| (i as f32 * 0.5 - 50.0, j as f32 * 0.5)))
            .find(|&(x, z)| {
                terrain.can_walk(x, z)
                    && terrain.height_at(x, z) < 0.05
                    && higher_ground_near(terrain, (x, z))
                        .is_some_and(|(hx, hz)| (hx - x).hypot(hz - z) > 3.0)
            })
            .unwrap();
        let mut units = [machine_and_pad(Role::Ornithopter)[0]];
        (units[0].x, units[0].z) = sand;
        units[0].destination = sand;
        after_a_mission(&mut units, terrain);
        let unit = units[0];
        assert!(terrain.on_upper_ground(unit.x, unit.z), "{unit:?}");
        assert_eq!(unit.altitude, 0.0);
    }

    #[test]
    fn a_machine_sent_somewhere_after_a_mission_sits_down_there_and_not_at_home() {
        let terrain = open_map();
        let mut units = machine_and_pad(Role::Ornithopter);
        units[0].selected = true;
        units[0].rockets_fired = 0;
        units[0].target = Some(Target::Unit(999));
        advance(&mut units, &terrain, &[0], 0.05);
        order_move(&mut units, (-10.0, 50.0), &terrain, &[0]);
        for _ in 0..600 {
            advance(&mut units, &terrain, &[0], 0.05);
        }
        assert!(
            (units[0].x + 10.0).hypot(units[0].z - 50.0) < 1.5,
            "{:?}",
            units[0]
        );
        assert_eq!(units[0].altitude, 0.0);
    }

    #[test]
    fn a_machine_out_of_rockets_flies_to_a_pad_lands_on_its_deck_and_loads_up_again() {
        let terrain = open_map();
        let mut units = machine_and_pad(Role::Ornithopter);
        assert!(!units[0].has_rockets());
        for _ in 0..1000 {
            advance(&mut units, &terrain, &[0], 0.05);
            if units[0].rockets_fired == 0 {
                break;
            }
        }
        assert_eq!(units[0].rockets_fired, 0, "loaded: {:?}", units[0]);
        assert!((units[0].x - 8.0).hypot(units[0].z - 50.0) < OVER_PAD);
        assert_eq!(units[0].altitude, PAD_TOP, "sitting on the deck");
        assert!(units[0].has_rockets());
    }

    #[test]
    fn a_repair_vehicle_goes_by_itself_to_a_hurt_friend_close_by_mends_it_and_is_free_again() {
        let terrain = open_map();
        let mut fixer = unit_at(0.0, 50.0, UnitKind::Vehicle);
        fixer.role = Role::RepairVehicle;
        fixer.weapon = WeaponKind::Repair;
        fixer.id = 1;
        let mut hurt = unit_at(8.0, 50.0, UnitKind::Vehicle);
        hurt.id = 2;
        hurt.health = hurt.max_health() - 30.0;
        hurt.weapon = WeaponKind::Unarmed;
        let mut far = unit_at(-30.0, 50.0, UnitKind::Vehicle);
        far.id = 3;
        far.health = 1.0;
        far.weapon = WeaponKind::Unarmed;
        let mut units = [fixer, hurt, far];
        let mut combat = crate::weapons::Combat::default();
        advance(&mut units, &terrain, &[0], 0.05);
        assert_eq!(
            units[0].target,
            Some(Target::Rescue(2)),
            "not the one out of reach"
        );
        for _ in 0..400 {
            advance(&mut units, &terrain, &[0], 0.05);
            combat.advance(&mut units, &[0], &terrain, 0.05);
        }
        assert_eq!(units[1].health, units[1].max_health(), "mended");
        assert_eq!(units[0].target, None, "and free again");
        assert!(!units[0].is_moving());
        assert_eq!(units[2].health, 1.0);
    }

    #[test]
    fn a_repair_vehicle_on_its_way_somewhere_or_with_a_job_does_not_rush_off() {
        let terrain = open_map();
        let mut fixer = unit_at(0.0, 50.0, UnitKind::Vehicle);
        fixer.role = Role::RepairVehicle;
        fixer.weapon = WeaponKind::Repair;
        fixer.id = 1;
        fixer.destination = (-20.0, 50.0);
        let mut hurt = unit_at(5.0, 50.0, UnitKind::Vehicle);
        hurt.id = 2;
        hurt.health = 10.0;
        let mut enemy = hurt;
        enemy.id = 3;
        enemy.owner = 1;
        enemy.z = 55.0;
        let mut units = [fixer, hurt, enemy];
        advance(&mut units, &terrain, &[0, 1], 0.05);
        assert_eq!(units[0].target, None, "told to go");
        units[0].destination = (units[0].x, units[0].z);
        units[1].alive = false;
        advance(&mut units, &terrain, &[0, 1], 0.05);
        assert_eq!(units[0].target, None, "no help for an enemy");
    }

    #[test]
    fn a_machine_sitting_on_a_pad_counts_as_grounded_and_one_up_in_the_air_does_not() {
        let mut machine = unit_at(0.0, 50.0, UnitKind::Aircraft);
        machine.role = Role::Destroyer;
        machine.sit_on_pad(99);
        assert!(machine.is_grounded() && machine.can_be_repaired());
        assert_eq!(machine.height_as_target(), 0.0);
        machine.altitude = PAD_TOP + 1.0;
        assert!(!machine.is_grounded() && !machine.can_be_repaired());
        assert_eq!(machine.height_as_target(), PAD_TOP + 1.0);
        // What does not land is in the air even that low.
        let mut carryall = unit_at(0.0, 50.0, UnitKind::Aircraft);
        carryall.sit_on_pad(99);
        assert!(!carryall.is_grounded());
    }

    #[test]
    fn a_repair_vehicle_mends_a_machine_sitting_on_a_pad() {
        let terrain = open_map();
        let [mut machine, pad] = machine_and_pad(Role::Ornithopter);
        machine.rockets_fired = 0;
        machine.x = pad.x;
        machine.z = pad.z;
        machine.destination = (pad.x, pad.z);
        machine.altitude = PAD_TOP;
        machine.pad = Some(Pad {
            id: pad.id,
            x: pad.x,
            z: pad.z,
            deck: FRAC_PI_4,
        });
        machine.health = 20.0;
        let mut fixer = unit_at(pad.x - 7.0, pad.z, UnitKind::Vehicle);
        fixer.role = Role::RepairVehicle;
        fixer.weapon = WeaponKind::Repair;
        fixer.id = 3;
        let mut units = [machine, pad, fixer];
        let mut combat = crate::weapons::Combat::default();
        for _ in 0..600 {
            advance(&mut units, &terrain, &[0], 0.05);
            combat.advance(&mut units, &[0], &terrain, 0.05);
        }
        assert_eq!(units[0].health, units[0].max_health(), "{:?}", units[2]);
        assert!(units[0].is_on_pad(), "it stayed put");
    }

    /// Runs units until the first has got where it was going, or for a minute at most.
    fn go(units: &mut [Unit], terrain: &Terrain) {
        for _ in 0..1200 {
            advance(units, terrain, &[0], 0.05);
            if !units[0].is_moving() {
                break;
            }
        }
    }

    #[test]
    fn a_unit_gets_around_a_building_in_its_way() {
        let terrain = open_map();
        let mut tank = unit_at(0.0, 45.0, UnitKind::Vehicle);
        tank.id = 1;
        tank.destination = (0.0, 55.0);
        let mut building = unit_at(0.0, 50.0, UnitKind::Building);
        building.id = 2;
        let mut units = [tank, building];
        go(&mut units, &terrain);
        let left = (units[0].x).hypot(units[0].z - 55.0);
        assert!(left < 0.5, "it got there: {:?}", units[0]);
    }

    #[test]
    fn a_unit_gets_around_a_row_of_others_that_are_standing_still() {
        let terrain = open_map();
        let mut tank = unit_at(0.0, 45.0, UnitKind::Vehicle);
        tank.id = 1;
        tank.destination = (0.0, 55.0);
        let mut units = vec![tank];
        for i in 0..11 {
            let mut guard = unit_at(i as f32 - 5.0, 50.0, UnitKind::Vehicle);
            guard.id = 2 + i;
            guard.weapon = WeaponKind::Unarmed;
            units.push(guard);
        }
        go(&mut units, &terrain);
        let left = (units[0].x).hypot(units[0].z - 55.0);
        assert!(left < 1.0, "it got there: {:?}", units[0]);
    }

    #[test]
    fn a_unit_gets_around_a_ruin_wall_and_out_of_a_corner_of_one() {
        let piece = |x: f32, z: f32, half_x: f32, half_z: f32| RuinPiece {
            x,
            y: 0.0,
            z,
            half_x,
            half_z,
            height: 1.0,
            shade: 1.0,
        };
        // A wall with a return at each end, opening away from where it is going.
        let terrain = Terrain::open_with_ruins(vec![
            piece(0.0, 50.0, 5.0, 0.1),
            piece(-5.0, 47.0, 0.1, 3.0),
            piece(5.0, 47.0, 0.1, 3.0),
        ]);
        let mut soldier = unit_at(0.0, 48.0, UnitKind::Infantry);
        soldier.destination = (0.0, 56.0);
        let mut units = [soldier];
        go(&mut units, &terrain);
        let left = (units[0].x).hypot(units[0].z - 56.0);
        assert!(left < 0.5, "it got there: {:?}", units[0]);
    }

    #[test]
    fn a_unit_boxed_in_gives_up_rather_than_pressing_on_for_ever() {
        let terrain = open_map();
        let mut soldier = unit_at(0.0, 50.0, UnitKind::Infantry);
        soldier.id = 1;
        soldier.destination = (0.0, 60.0);
        let mut units = vec![soldier];
        for i in 0..12 {
            let (sin, cos) = (i as f32 / 12.0 * TAU).sin_cos();
            let mut wall = unit_at(cos * 1.7, 50.0 + sin * 1.7, UnitKind::Building);
            wall.id = 2 + i;
            units.push(wall);
        }
        for _ in 0..1200 {
            advance(&mut units, &terrain, &[0], 0.05);
        }
        assert!(!units[0].is_moving(), "{:?}", units[0]);
    }

    #[test]
    fn a_unit_getting_on_fine_is_not_looked_at_twice() {
        let terrain = open_map();
        let mut tank = unit_at(0.0, 30.0, UnitKind::Vehicle);
        tank.destination = (0.0, 90.0);
        let mut units = [tank];
        for _ in 0..200 {
            advance(&mut units, &terrain, &[], 0.05);
            assert_eq!(units[0].way, None);
            assert!(!units[0].stall.rethink);
            assert_eq!(units[0].stall.looks.min(1), units[0].stall.looks);
        }
    }

    #[test]
    fn a_machine_fires_all_it_has_then_flies_back_lands_on_the_unmoved_deck_and_loads_up() {
        let terrain = open_map();
        for role in [Role::Ornithopter, Role::Destroyer] {
            let [mut machine, pad] = machine_and_pad(role);
            machine.rockets_fired = 0;
            machine.heading = std::f32::consts::PI;
            // Far enough from the pad that it will not rise for this enemy once loaded.
            machine.x = -25.0;
            machine.destination = (-25.0, 50.0);
            let mut enemy = enemy_at(-30.0, 50.0, UnitKind::Vehicle);
            enemy.id = 9;
            enemy.health = 1e7;
            enemy.weapon = WeaponKind::Unarmed;
            let spot = (pad.x, pad.z);
            let mut units = [machine, pad, enemy];
            let mut combat = crate::weapons::Combat::default();
            let (mut ran_out, mut landed) = (false, false);
            for _ in 0..6000 {
                advance(&mut units, &terrain, &[0, 1], 0.05);
                combat.advance(&mut units, &[0, 1], &terrain, 0.05);
                assert_eq!((units[1].x, units[1].z), spot, "the pad stays put");
                ran_out |= !units[0].has_rockets();
                let over = (units[0].x - spot.0).hypot(units[0].z - spot.1);
                landed |= ran_out && over < OVER_PAD && units[0].altitude == PAD_TOP;
                if landed && units[0].rockets_fired == 0 {
                    break;
                }
            }
            assert!(ran_out, "{role:?} used them all: {:?}", units[0]);
            assert!(
                landed,
                "{role:?} came back and sat on the deck: {:?}",
                units[0]
            );
            assert_eq!(units[0].rockets_fired, 0, "{role:?} is loaded");
        }
    }

    #[test]
    fn nothing_shoves_a_building_not_even_another_building_or_a_machine_coming_down() {
        let terrain = open_map();
        let mut hangar = unit_at(0.0, 50.0, UnitKind::Building);
        hangar.id = 1;
        let mut pad = unit_at(1.0, 50.0, UnitKind::Building);
        pad.role = Role::LandingPad;
        pad.id = 2;
        let mut machine = unit_at(2.0, 50.0, UnitKind::Aircraft);
        machine.role = Role::Ornithopter;
        machine.id = 3;
        machine.altitude = 0.5;
        machine.destination = (2.0, 52.0);
        let mut units = [hangar, pad, machine];
        for _ in 0..40 {
            advance(&mut units, &terrain, &[0], 0.05);
        }
        assert_eq!((units[0].x, units[0].z), (0.0, 50.0));
        assert_eq!((units[1].x, units[1].z), (1.0, 50.0));
    }

    #[test]
    fn a_tank_and_a_soldier_go_around_a_landing_pad_on_its_tower() {
        let terrain = open_map();
        for kind in [UnitKind::Vehicle, UnitKind::Infantry] {
            let [_, mut pad] = machine_and_pad(Role::Ornithopter);
            pad.x = 0.0;
            let mut crossing = unit_at(-4.0, 50.0, kind);
            crossing.id = 5;
            crossing.destination = (4.0, 50.0);
            let mut units = [pad, crossing];
            for _ in 0..400 {
                advance(&mut units, &terrain, &[0], 0.05);
                let unit = units[1];
                let from_shaft = unit.x.hypot(unit.z - 50.0);
                assert!(from_shaft > PAD_DECK.1, "around it: {unit:?}");
                assert_eq!(unit.y, terrain.height_at(unit.x, unit.z), "on the ground");
            }
            assert_eq!((units[0].x, units[0].z), (0.0, 50.0), "the pad stays put");
            let unit = units[1];
            assert!(
                (unit.x - 4.0).hypot(unit.z - 50.0) < 0.3,
                "got past: {unit:?}"
            );
        }
    }

    #[test]
    fn machines_landing_on_a_pad_line_themselves_up_with_it() {
        let terrain = open_map();
        for role in [Role::Ornithopter, Role::Destroyer] {
            let [mut machine, pad] = machine_and_pad(role);
            machine.heading = 2.2;
            machine.selected = true;
            let mut units = [machine, pad];
            order_land(&mut units, 1);
            for _ in 0..1000 {
                advance(&mut units, &terrain, &[0], 0.05);
            }
            assert!(units[0].is_grounded(), "{:?}", units[0]);
            let off = (units[0].heading - std::f32::consts::FRAC_PI_4).sin().abs();
            assert!(off < 0.02, "{role:?} is {off} off: {:?}", units[0]);
        }
    }

    #[test]
    fn a_machine_sent_to_a_pad_lands_there_and_loads_and_the_next_waits_beside_it() {
        let terrain = open_map();
        let [mut first, pad] = machine_and_pad(Role::Ornithopter);
        first.rockets_fired = 1;
        first.selected = true;
        let mut second = first;
        second.id = 3;
        second.z = 60.0;
        let mut units = [first, pad, second];
        order_land(&mut units, 1);
        assert_eq!(units[0].destination, (8.0, 50.0), "the nearer lands");
        assert_ne!(units[2].destination, (8.0, 50.0), "the other waits beside");
        for _ in 0..1000 {
            advance(&mut units, &terrain, &[0], 0.05);
            if units[0].rockets_fired == 0 {
                break;
            }
        }
        assert_eq!(units[0].rockets_fired, 0, "loaded: {:?}", units[0]);
    }

    #[test]
    fn a_pad_takes_one_machine_and_the_other_waits_for_it_to_be_free() {
        let terrain = open_map();
        let [first, pad] = machine_and_pad(Role::Ornithopter);
        let mut second = first;
        second.id = 3;
        second.z = 53.0;
        second.destination = (0.0, 53.0);
        let mut units = [first, pad, second];
        advance(&mut units, &terrain, &[0], 0.05);
        let using = units.iter().filter(|u| u.pad.is_some()).count();
        assert_eq!(using, 1, "only one goes for it");
        let waiting = units
            .iter()
            .find(|u| u.role.lands() && u.pad.is_none())
            .unwrap();
        assert!(waiting.is_moving(), "it comes to wait beside the pad");
    }

    #[test]
    fn a_machine_with_no_free_pad_waits_beside_one_and_a_full_one_makes_way_for_it() {
        let terrain = open_map();
        let [mut full, pad] = machine_and_pad(Role::Ornithopter);
        full.rockets_fired = 0;
        full.x = pad.x;
        full.z = pad.z;
        full.destination = (pad.x, pad.z);
        full.altitude = PAD_TOP;
        full.pad = Some(Pad {
            id: pad.id,
            x: pad.x,
            z: pad.z,
            deck: FRAC_PI_4,
        });
        let spot = (pad.x, pad.z);
        // Far off at first, so it has to come and wait.
        let mut empty = machine_and_pad(Role::Destroyer)[0];
        empty.id = 3;
        empty.x = -20.0;
        empty.destination = (-20.0, 50.0);
        let mut loading = full;
        loading.rockets_fired = 11;
        let mut units = [loading, pad, empty];
        for _ in 0..200 {
            advance(&mut units, &terrain, &[0], 0.05);
        }
        assert!(units[0].is_on_pad(), "still loading");
        let waits = (units[2].x - spot.0).hypot(units[2].z - spot.1);
        assert!(waits < PAD_WAITS + 0.5, "it came to wait: {waits}");
        assert!(units[2].rockets_fired > 0);
        // The one on the pad finishes loading first, then gives way, and the other gets it.
        for _ in 0..700 {
            advance(&mut units, &terrain, &[0], 0.05);
        }
        assert_eq!(units[0].rockets_fired, 0);
        assert_eq!(units[0].pad, None, "it made way");
        assert!((units[0].x - spot.0).hypot(units[0].z - spot.1) > OVER_PAD);
        assert_eq!(units[0].altitude, 0.0, "and sat down beside it");
        assert_eq!(
            units[2].rockets_fired, 0,
            "the other loaded up: {:?}",
            units[2]
        );
        assert_eq!((units[1].x, units[1].z), spot);
    }

    #[test]
    fn a_full_machine_does_not_make_way_for_one_that_is_full_or_far_off() {
        let terrain = open_map();
        let [mut sitting, pad] = machine_and_pad(Role::Ornithopter);
        sitting.rockets_fired = 0;
        sitting.x = pad.x;
        sitting.z = pad.z;
        sitting.destination = (pad.x, pad.z);
        sitting.altitude = PAD_TOP;
        sitting.pad = Some(Pad {
            id: pad.id,
            x: pad.x,
            z: pad.z,
            deck: FRAC_PI_4,
        });
        let mut full = sitting;
        full.id = 3;
        full.pad = None;
        full.altitude = 0.0;
        full.x = pad.x + 4.0;
        full.destination = (full.x, full.z);
        let mut short_far = full;
        short_far.id = 4;
        short_far.rockets_fired = 1;
        short_far.x = pad.x + PAD_NEARBY + 8.0;
        short_far.destination = (short_far.x, short_far.z);
        let mut units = [sitting, pad, full, short_far];
        for _ in 0..40 {
            advance(&mut units, &terrain, &[0], 0.05);
        }
        assert!(units[0].is_on_pad(), "{:?}", units[0]);
    }

    #[test]
    fn a_machine_with_some_rockets_left_tops_up_at_a_free_pad_close_by() {
        let terrain = open_map();
        let [mut machine, pad] = machine_and_pad(Role::Ornithopter);
        machine.rockets_fired = 2;
        machine.x = pad.x - 5.0;
        machine.destination = (machine.x, machine.z);
        let mut units = [machine, pad];
        for _ in 0..400 {
            advance(&mut units, &terrain, &[0], 0.05);
            if units[0].rockets_fired == 0 {
                break;
            }
        }
        assert_eq!(units[0].rockets_fired, 0, "{:?}", units[0]);
    }

    #[test]
    fn a_machine_out_of_rockets_with_no_pad_sits_down_and_cannot_be_sent_to_attack() {
        let terrain = open_map();
        let [machine, _] = machine_and_pad(Role::Destroyer);
        let mut enemy = unit_at(3.0, 50.0, UnitKind::Vehicle);
        enemy.owner = 1;
        enemy.id = 9;
        let mut units = [machine, enemy];
        units[0].selected = true;
        order_attack(&mut units, 1, &[0, 1]);
        assert_eq!(units[0].target, None);
        for _ in 0..200 {
            advance(&mut units, &terrain, &[0, 1], 0.05);
        }
        assert_eq!(units[0].altitude, 0.0, "it sat down");
    }

    #[test]
    fn a_pad_of_someone_else_or_destroyed_is_no_use() {
        let terrain = open_map();
        let mut units = machine_and_pad(Role::Ornithopter);
        units[1].owner = 1;
        advance(&mut units, &terrain, &[0, 1], 0.05);
        assert_eq!(units[0].pad, None);

        let mut units = machine_and_pad(Role::Ornithopter);
        advance(&mut units, &terrain, &[0], 0.05);
        assert!(units[0].pad.is_some());
        units[1].alive = false;
        advance(&mut units, &terrain, &[0], 0.05);
        assert_eq!(units[0].pad, None);
    }

    #[test]
    fn a_machine_ordered_elsewhere_while_out_of_rockets_obeys_then_goes_back_for_more() {
        let terrain = open_map();
        let mut units = machine_and_pad(Role::Ornithopter);
        advance(&mut units, &terrain, &[0], 0.05);
        units[0].destination = (-10.0, 50.0);
        let mut went_there = false;
        for _ in 0..200 {
            advance(&mut units, &terrain, &[0], 0.05);
            went_there |= units[0].x < -9.0;
        }
        assert!(went_there, "it went where it was told first");
        for _ in 0..1000 {
            advance(&mut units, &terrain, &[0], 0.05);
        }
        assert_eq!(units[0].rockets_fired, 0, "then loaded up: {:?}", units[0]);
    }

    #[test]
    fn a_machine_that_leaves_its_pad_with_rockets_gives_it_up_and_does_not_rise_until_loaded() {
        let terrain = open_map();
        let mut units = machine_and_pad(Role::Ornithopter);
        for _ in 0..400 {
            advance(&mut units, &terrain, &[0], 0.05);
            if units[0].is_on_pad() {
                break;
            }
        }
        assert!(units[0].is_on_pad());
        let mut enemy = unit_at(9.0, 52.0, UnitKind::Vehicle);
        enemy.owner = 1;
        enemy.id = 9;
        let mut with_enemy = [units[0], units[1], enemy];
        for _ in 0..5 {
            advance(&mut with_enemy, &terrain, &[0, 1], 0.05);
        }
        assert!(with_enemy[0].is_on_pad(), "it stays to load");
        with_enemy[0].rockets_fired = 1;
        with_enemy[0].destination = (-10.0, 50.0);
        for _ in 0..200 {
            advance(&mut with_enemy, &terrain, &[0, 1], 0.05);
        }
        assert_eq!(with_enemy[0].pad, None);
    }

    #[test]
    fn a_destroyer_is_heavier_and_slower_and_carries_fewer_rockets_than_an_ornithopter() {
        let (d, o) = (Role::Destroyer, Role::Ornithopter);
        assert!(d.max_health() > o.max_health());
        assert!(d.speed() < o.speed());
        assert!(d.rocket_load() < o.rocket_load());
        assert!(d.lands());
        assert_eq!(d.made_in(), Some(Role::Hangar));
        assert_eq!(Role::Tank.rocket_load(), None);
    }

    #[test]
    fn only_what_is_not_infantry_shows_wear_and_it_grows_with_what_it_lost() {
        let mut tank = unit_at(0.0, 50.0, UnitKind::Vehicle);
        assert_eq!(tank.wear(), 0.0);
        tank.health = tank.max_health() * 0.25;
        assert!((tank.wear() - 0.75).abs() < 0.001);
        let mut soldier = unit_at(0.0, 50.0, UnitKind::Infantry);
        soldier.health = 1.0;
        assert_eq!(soldier.wear(), 0.0);
    }

    #[test]
    fn an_mcv_comes_round_to_face_the_left_bottom_edge_then_takes_a_second_to_deploy() {
        let terrain = Terrain::open_upland();
        let mut mcv = unit_at(0.0, 50.0, UnitKind::Vehicle);
        mcv.role = Role::Mcv;
        mcv.heading = 0.1;
        mcv.selected = true;
        let mut units = [mcv];
        toggle_deploy(&mut units, &terrain);
        assert!(!units[0].deployed, "not at once");
        assert_eq!(units[0].deployment, Deployment::Turning);
        for _ in 0..200 {
            advance(&mut units, &terrain, &[0], 0.02);
            if units[0].deployment == Deployment::Drilling {
                break;
            }
        }
        assert_eq!(units[0].heading, MCV_DEPLOY_HEADING);
        assert!(units[0].is_under_construction(), "in its scaffolding");
        assert_eq!(units[0].raised(), 1.0, "whole, not rising from the ground");
        let mut seconds = 0.0;
        while !units[0].deployed {
            advance(&mut units, &terrain, &[0], 0.02);
            seconds += 0.02;
            assert!(seconds < 2.0);
            // Orders make no difference to it meanwhile.
            units[0].destination = (9.0, 60.0);
        }
        assert!((seconds - CONSTRUCTION_SECONDS).abs() < 0.05, "{seconds}");
        assert_eq!(units[0].kind, UnitKind::Building);
        let plot = units[0].plot().unwrap();
        assert_eq!((units[0].x, units[0].z), plot.center(), "on the grid");
    }

    #[test]
    fn an_mcvs_drill_lies_level_then_tips_to_45_degrees_and_bores_in_as_it_turns() {
        let mut mcv = unit_at(0.0, 50.0, UnitKind::Vehicle);
        mcv.role = Role::Mcv;
        let packed = mcv.drill_pose();
        assert_eq!((packed.tilt, packed.plunge), (0.0, 0.0));
        mcv.deployment = Deployment::Drilling;
        mcv.constructing = CONSTRUCTION_SECONDS * 0.5;
        let half = mcv.drill_pose();
        assert!(
            half.tilt > 0.7 && half.spin > 1.0 && half.plunge < 1.0,
            "{half:?}"
        );
        assert!(mcv.drill_is_biting());
        mcv.deployment = Deployment::Idle;
        mcv.constructing = 0.0;
        switch_deployed(&mut mcv);
        let done = mcv.drill_pose();
        assert_eq!((done.tilt, done.plunge), (FRAC_PI_4, 1.0));
    }

    #[test]
    fn a_building_turned_to_face_another_edge_changes_its_squares_and_its_way_out() {
        let at = (0.0, 60.0);
        let plain = Role::Factory.plot_facing(at, Facing::LeftBottom);
        assert_eq!((plain.cols, plain.rows), (6, 5));
        let turned = Role::Factory.plot_facing(at, Facing::RightBottom);
        assert_eq!((turned.cols, turned.rows), (5, 6));
        assert_eq!(
            Role::Factory.plot_facing(turned.center(), Facing::RightBottom),
            turned
        );
        let exits = |facing: Facing| -> Vec<(i32, i32)> {
            let plot = Role::Factory.plot_facing(at, facing);
            plot.cells()
                .filter(|&cell| Role::Factory.is_exit(plot, facing, cell))
                .collect()
        };
        let plot = |facing| Role::Factory.plot_facing(at, facing);
        assert!(
            exits(Facing::LeftBottom)
                .iter()
                .all(|c| c.1 == plot(Facing::LeftBottom).v + 4)
        );
        assert!(
            exits(Facing::RightTop)
                .iter()
                .all(|c| c.1 == plot(Facing::RightTop).v)
        );
        assert!(
            exits(Facing::RightBottom)
                .iter()
                .all(|c| c.0 == plot(Facing::RightBottom).u + 4)
        );
        assert!(
            exits(Facing::LeftTop)
                .iter()
                .all(|c| c.0 == plot(Facing::LeftTop).u)
        );
        for facing in [
            Facing::LeftBottom,
            Facing::LeftTop,
            Facing::RightTop,
            Facing::RightBottom,
        ] {
            assert_eq!(exits(facing).len(), 2, "{facing:?}");
        }
        assert_eq!(Facing::of_drag(-5.0, 5.0), Facing::LeftBottom);
        assert_eq!(Facing::of_drag(-5.0, -5.0), Facing::LeftTop);
        assert_eq!(Facing::of_drag(5.0, -5.0), Facing::RightTop);
        assert_eq!(Facing::of_drag(5.0, 5.0), Facing::RightBottom);
        let (x, z) = Facing::RightTop.way_out();
        assert!(x > 0.5 && z < -0.5, "{x} {z}");
    }

    #[test]
    fn a_line_comes_to_a_plot_where_it_first_meets_it_and_not_from_inside() {
        let plot = Plot {
            u: 4,
            v: 0,
            cols: 2,
            rows: 2,
        };
        assert_eq!(plot.entered_by((0.0, 1.0), (8.0, 1.0)), Some(0.5));
        assert_eq!(plot.entered_by((0.0, 5.0), (8.0, 5.0)), None, "passes by");
        assert_eq!(plot.entered_by((0.0, 1.0), (2.0, 1.0)), None, "stops short");
        assert_eq!(plot.entered_by((5.0, 1.0), (9.0, 1.0)), None, "from inside");
    }

    #[test]
    fn the_player_starts_at_random_with_opponents_placed_by_how_many_there_are() {
        for seed in [1, 3, 5, 7, 11, 99, 1234] {
            let mut random = seed;
            let two = start_slots(2, &mut random);
            assert_eq!(two[1], (two[0] + 4) % 8, "opposite");
            let three = start_slots(3, &mut random);
            let p = three[0];
            assert_eq!(three[1..], [(p + 2) % 8, (p + 6) % 8], "one place between");
            let four = start_slots(4, &mut random);
            assert!(
                four.iter().all(|s| s % 2 == four[0] % 2),
                "all corners or all edges"
            );
            let mut all = four.clone();
            all.sort();
            all.dedup();
            assert_eq!(all.len(), 4);
            for sides in 5..=8 {
                let mut all = start_slots(sides, &mut random);
                assert_eq!(all.len(), sides);
                all.sort();
                all.dedup();
                assert_eq!(all.len(), sides, "each its own place");
            }
        }
        let mut random = 17;
        let firsts: Vec<usize> = (0..40).map(|_| start_slots(2, &mut random)[0]).collect();
        assert!(firsts.iter().any(|&f| f != firsts[0]), "picked at random");
    }

    #[test]
    fn everyone_starts_with_an_mcv_that_can_deploy_where_it_stands_and_nobody_in_the_middle() {
        let colors = [TeamColor::Blue; 8];
        for map in 0..crate::game::MAPS.len() {
            let terrain = crate::battlefield::terrain(map);
            for sides in 2..=8 {
                let mut random = 5 + sides as u32;
                let units = spawn(terrain, &colors[..sides], &mut random);
                assert_eq!(units.len(), sides);
                for unit in &units {
                    assert!(can_deploy_here(unit, terrain), "map {map}: {unit:?}");
                    let from_middle = unit.x.hypot(unit.z - terrain.size().middle_z());
                    assert!(from_middle > 40.0, "map {map}: {unit:?}");
                }
            }
        }
    }

    #[test]
    fn an_order_into_a_mountain_goes_to_its_edge() {
        let terrain = &Terrain::open_with_mountain(0.0, 40.0, 5.0);
        let mut tank = unit_at(0.0, 30.0, UnitKind::Vehicle);
        tank.selected = true;
        let mut units = [tank];
        order_move(&mut units, (0.0, 40.0), terrain, &[]);
        let spot = units[0].destination;
        assert!(terrain.can_walk(spot.0, spot.1), "{spot:?}");
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
        let units = spawn_bases(terrain(0), &COLORS);
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
        let units = spawn_bases(terrain(1), &colors);
        for (i, a) in units.iter().enumerate() {
            for b in units[i + 1..].iter().filter(|b| b.altitude == a.altitude) {
                assert!(
                    (a.x - b.x).hypot(a.z - b.z) > a.radius() + b.radius(),
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
                for unit in spawn_bases(terrain, &colors[..count]) {
                    for piece in terrain.ruins() {
                        let gap_x = (unit.x - piece.x).abs() - unit.radius() - piece.half_x;
                        let gap_z = (unit.z - piece.z).abs() - unit.radius() - piece.half_z;
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
        for unit in spawn_bases(terrain, &COLORS) {
            assert_eq!(unit.y, terrain.height_at(unit.x, unit.z));
        }
    }

    #[test]
    fn the_players_groups_are_more_than_a_screen_apart() {
        let view = View::new(Camera::centered(), 2880.0, 1800.0);
        let units = spawn_bases(terrain(0), &COLORS);
        let far = units
            .iter()
            .find(|u| u.owner == 0 && u.x.hypot(u.z - MAP_MIDDLE_Z) > 40.0)
            .unwrap();
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
