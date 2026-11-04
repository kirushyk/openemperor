//! The ways of killing: bullets, missiles, lasers, mortars and the superweapon.
//!
//! Everything here is plain simulation with no drawing. Units fire by themselves at the
//! nearest enemy in range, most only while standing still. Whatever can only fire straight
//! ahead first turns to face its target.

use crate::battlefield::Terrain;
use crate::units::{Role, Target, Unit, UnitKind, are_enemies, are_friends, turn_between};

/// The longest simulation step, in seconds; longer frames are divided up.
const MAX_STEP: f32 = 0.05;
/// A unit with nothing in range looks again this often, in seconds.
const RETARGET_SECONDS: f32 = 0.25;

const MISSILE_SPEED: f32 = 9.0;
/// Radians per second a missile can turn: fast targets can out-turn it.
const MISSILE_TURN: f32 = 2.5;
const MISSILE_FUEL_SECONDS: f32 = 4.0;
/// How near a missile must come, beyond touching its target, to go off.
const MISSILE_FUSE: f32 = 0.2;
const MISSILE_BLAST_RADIUS: f32 = 0.8;

/// A shell flies this fast along the ground, and arcs higher the farther it goes.
const SHELL_SPEED: f32 = 7.0;
const SHELL_ARC: f32 = 0.35;
const SHELL_BLAST_RADIUS: f32 = 1.6;

/// Radians per second a tower turns its tubes: fast at a target, slowly while looking out.
const TOWER_TURN: f32 = 2.0;
const TOWER_IDLE_TURN: f32 = 0.5;
/// A tower must point this close to its target, in radians, before it fires.
const TOWER_ALIGNMENT: f32 = 0.1;

/// Fire and gas stay low: anything higher than this above the ground is out of their reach.
const SPRAY_CEILING: f32 = 2.0;
/// What a missile fired at a place carries instead of a unit's id.
const NO_UNIT: u32 = u32::MAX;
/// How far beyond their own size a shot at a place can catch someone standing there.
const PLACE_SHOT_SPREAD: f32 = 0.3;
/// How long one puff of fire or gas shows, a little longer than the time between puffs.
const SPRAY_SECONDS: f32 = 0.3;

/// Health a repair vehicle gives back with each pulse.
const REPAIR_HEALTH: f32 = 12.0;
/// Health per second that experienced units win back, for each rank.
const VETERAN_HEALING: f32 = 1.0;

const BEAM_SECONDS: f32 = 0.2;
const FLARE_SECONDS: f32 = 0.1;
const BLAST_SECONDS: f32 = 0.5;

/// How high the radioactive missile starts, and how long it takes to come down.
const STRIKE_HEIGHT: f32 = 60.0;
const STRIKE_SECONDS: f32 = 1.5;
pub const STRIKE_RADIUS: f32 = 7.0;
/// Immediate damage at the center; it falls to half of this at the edge.
const STRIKE_DAMAGE: f32 = 400.0;
pub const POISON_SECONDS: f32 = 60.0;
/// Damage per second to anyone on the ground in poisoned land.
const POISON_DAMAGE: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponKind {
    /// Invisible and immediate.
    Bullet,
    /// Flies and steers in 3D, and can miss.
    Missile,
    /// Immediate like a bullet, but visible as a straight line.
    Laser,
    /// Lobbed in a ballistic arc at where the target was, exploding where it lands.
    Mortar,
    /// Like a bullet, but slower and from farther away, and hits harder.
    Sniper,
    /// Like a bullet, but weak and slower.
    Pistol,
    /// Like a bullet, but much stronger, even against armor.
    Cannon,
    /// Like a bullet, but rapid, weak and able to reach the air.
    AutoGun,
    /// A missile that only goes for aircraft.
    AirMissile,
    /// Burns everything low enough in a short cone in front of it, once turned that way.
    Flame,
    /// Like flame, in green: reaches farther and wider, but hurts less.
    Gas,
    /// Fixes friendly vehicles nearby instead of harming anyone.
    Repair,
    /// Carries nothing.
    Unarmed,
}

impl WeaponKind {
    pub fn range(self) -> f32 {
        match self {
            WeaponKind::Bullet => 7.0,
            WeaponKind::Missile => 12.0,
            WeaponKind::Laser => 9.0,
            WeaponKind::Mortar => 16.0,
            WeaponKind::Sniper => 14.0,
            WeaponKind::Pistol => 5.0,
            WeaponKind::Cannon => 10.0,
            WeaponKind::AutoGun => 8.0,
            WeaponKind::AirMissile => 12.0,
            WeaponKind::Flame => 4.5,
            WeaponKind::Gas => 5.5,
            WeaponKind::Repair => 3.0,
            WeaponKind::Unarmed => 0.0,
        }
    }

    /// Mortar shells only come down on the ground. Everything else can aim up.
    pub fn can_target(self, kind: UnitKind, altitude: f32) -> bool {
        match self {
            WeaponKind::Unarmed | WeaponKind::Repair => false,
            WeaponKind::Mortar => !kind.flies(),
            WeaponKind::Flame | WeaponKind::Gas => altitude <= SPRAY_CEILING,
            WeaponKind::AirMissile => kind.flies(),
            _ => true,
        }
    }

    /// How large, in screen units, the flare where it is fired, if it has one. Only guns
    /// that strike at once have one: nothing else shows where their shots come from.
    pub fn flare(self) -> Option<f32> {
        match self {
            WeaponKind::Pistol => Some(5.0),
            WeaponKind::Bullet => Some(7.0),
            WeaponKind::Sniper => Some(9.0),
            WeaponKind::Cannon => Some(16.0),
            WeaponKind::AutoGun => Some(6.0),
            _ => None,
        }
    }

    /// Only lasers and missiles can be fired on the move. Everything else must stand still.
    fn fires_on_move(self) -> bool {
        matches!(self, WeaponKind::Laser | WeaponKind::Missile | WeaponKind::AirMissile)
    }

    /// How wide its cone is to each side, in radians, if it sprays one.
    fn spray_angle(self) -> Option<f32> {
        match self {
            WeaponKind::Flame => Some(0.3),
            WeaponKind::Gas => Some(0.45),
            _ => None,
        }
    }

    fn cooldown(self) -> f32 {
        match self {
            WeaponKind::Bullet => 0.6,
            WeaponKind::Missile => 3.0,
            WeaponKind::Laser => 1.2,
            WeaponKind::Mortar => 4.0,
            WeaponKind::Sniper => 3.5,
            WeaponKind::Pistol => 1.5,
            WeaponKind::Cannon => 3.0,
            WeaponKind::AutoGun => 0.2,
            WeaponKind::AirMissile => 3.0,
            WeaponKind::Flame | WeaponKind::Gas => 0.15,
            WeaponKind::Repair => 0.4,
            WeaponKind::Unarmed => RETARGET_SECONDS,
        }
    }

    fn damage(self) -> f32 {
        match self {
            WeaponKind::Bullet => 12.0,
            WeaponKind::Missile => 60.0,
            WeaponKind::Laser => 25.0,
            WeaponKind::Mortar => 70.0,
            WeaponKind::Sniper => 90.0,
            WeaponKind::Pistol => 8.0,
            WeaponKind::Cannon => 80.0,
            WeaponKind::AutoGun => 6.0,
            WeaponKind::AirMissile => 60.0,
            WeaponKind::Flame => 10.0,
            WeaponKind::Gas => 6.0,
            WeaponKind::Repair | WeaponKind::Unarmed => 0.0,
        }
    }

    /// Small arms barely scratch armor or walls.
    fn damage_to(self, kind: UnitKind) -> f32 {
        match (self, kind) {
            (
                WeaponKind::Bullet | WeaponKind::Pistol | WeaponKind::Sniper | WeaponKind::AutoGun | WeaponKind::Gas,
                UnitKind::Vehicle | UnitKind::Tower,
            ) => self.damage() * 0.25,
            (WeaponKind::Flame, UnitKind::Vehicle | UnitKind::Tower) => self.damage() * 0.5,
            _ => self.damage(),
        }
    }
}

type Point = (f32, f32, f32);

/// Who fired something, kept with shots in flight: it may rank up or die before they land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shooter {
    id: u32,
    owner: usize,
    power: f32,
}

impl Shooter {
    fn of(unit: &Unit) -> Self {
        Shooter { id: unit.id, owner: unit.owner, power: unit.attack_power() }
    }
}

fn distance(a: Point, b: Point) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1).hypot(a.2 - b.2)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Missile {
    pub position: Point,
    /// A unit vector.
    pub direction: Point,
    target_id: u32,
    /// Where it is headed if it was fired at a place and not a unit.
    place: Option<Point>,
    shooter: Shooter,
    fuel: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shell {
    start: Point,
    landing: Point,
    owner: usize,
    elapsed: f32,
    duration: f32,
    arc: f32,
    /// Fired by order at a place or a friend, it spares nobody.
    hurts_all: bool,
    shooter: Shooter,
}

impl Shell {
    /// Where it is along its ballistic arc.
    pub fn position(&self) -> Point {
        let t = self.elapsed / self.duration;
        let along = |a: f32, b: f32| a + (b - a) * t;
        (
            along(self.start.0, self.landing.0),
            along(self.start.1, self.landing.1) + 4.0 * self.arc * t * (1.0 - t),
            along(self.start.2, self.landing.2),
        )
    }

    /// The spot under it, for its shadow.
    pub fn landing(&self) -> Point {
        self.landing
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    pub from: Point,
    pub to: Point,
    pub life: f32,
    /// A repair vehicle at work, not a laser.
    pub repair: bool,
}

/// A puff of fire or gas, spreading from where it was sprayed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spray {
    pub from: Point,
    pub heading: f32,
    pub reach: f32,
    pub half_angle: f32,
    pub gas: bool,
    /// From 1 when it is sprayed down to 0.
    pub life: f32,
}

/// The burst of light where a gun is fired.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flare {
    pub at: Point,
    pub size: f32,
    /// From 1 when it is fired down to 0.
    pub life: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    pub at: Point,
    pub radius: f32,
    /// From 1 when it goes off down to 0.
    pub life: f32,
}

/// The radioactive missile on its way down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    pub x: f32,
    pub z: f32,
    elapsed: f32,
}

impl Strike {
    pub fn height(&self) -> f32 {
        STRIKE_HEIGHT * (1.0 - self.elapsed / STRIKE_SECONDS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoisonZone {
    pub x: f32,
    pub z: f32,
    /// Seconds left.
    pub life: f32,
}

/// Everything in flight or lingering on the battlefield.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Combat {
    pub missiles: Vec<Missile>,
    pub shells: Vec<Shell>,
    pub beams: Vec<Beam>,
    pub flares: Vec<Flare>,
    pub sprays: Vec<Spray>,
    pub blasts: Vec<Blast>,
    pub strikes: Vec<Strike>,
    pub poison: Vec<PoisonZone>,
}

impl Combat {
    /// Sends the radioactive missile at a point on the ground.
    pub fn launch_strike(&mut self, x: f32, z: f32) {
        self.strikes.push(Strike { x, z, elapsed: 0.0 });
    }

    /// Advances everything by `dt` seconds. Units killed are left marked as not alive.
    pub fn advance(&mut self, units: &mut [Unit], teams: &[usize], terrain: &Terrain, dt: f32) {
        let steps = (dt / MAX_STEP).ceil().max(1.0);
        for _ in 0..steps as usize {
            self.step(units, teams, terrain, dt / steps);
        }
    }

    fn step(&mut self, units: &mut [Unit], teams: &[usize], terrain: &Terrain, dt: f32) {
        self.fire(units, teams, terrain, dt);
        self.fly_missiles(units, teams, terrain, dt);
        self.fly_shells(units, teams, dt);
        self.fall_strikes(units, dt);
        self.poison_land(units, dt);
        heal_veterans(units, dt);
        for beam in &mut self.beams {
            beam.life -= dt / BEAM_SECONDS;
        }
        self.beams.retain(|b| b.life > 0.0);
        for flare in &mut self.flares {
            flare.life -= dt / FLARE_SECONDS;
        }
        self.flares.retain(|f| f.life > 0.0);
        for spray in &mut self.sprays {
            spray.life -= dt / SPRAY_SECONDS;
        }
        self.sprays.retain(|s| s.life > 0.0);
        for blast in &mut self.blasts {
            blast.life -= dt / BLAST_SECONDS;
        }
        self.blasts.retain(|b| b.life > 0.0);
    }

    fn fire(&mut self, units: &mut [Unit], teams: &[usize], terrain: &Terrain, dt: f32) {
        for i in 0..units.len() {
            if !units[i].alive {
                continue;
            }
            units[i].cooldown -= dt;
            if units[i].is_moving() && !units[i].weapon.fires_on_move() {
                continue;
            }
            let must_aim = must_turn_to_fire(&units[i]);
            if !must_aim && units[i].cooldown > 0.0 {
                continue;
            }
            let aim = choose_aim(units, i, teams, terrain);
            let aimed = !must_aim || turn_tubes(units, i, aim, dt);
            if units[i].cooldown > 0.0 {
                continue;
            }
            let Some(aim) = aim else {
                units[i].cooldown = RETARGET_SECONDS;
                continue;
            };
            if !aimed {
                continue;
            }
            let weapon = units[i].weapon;
            units[i].cooldown = weapon.cooldown();
            let from = units[i].muzzle();
            let shooter = Shooter::of(&units[i]);
            let (to, target) = match aim {
                Aim::Unit(t) => (units[t].center(), Some(t)),
                Aim::Place(place) => (place, None),
            };
            // Only firing as ordered at a place or a friend can hurt friends.
            let forced = match target {
                Some(t) => units[i].target == Some(Target::Forced(units[t].id)),
                None => true,
            };
            if let Some(size) = weapon.flare() {
                self.flares.push(Flare { at: from, size, life: 1.0 });
            }
            match weapon {
                WeaponKind::Bullet
                | WeaponKind::Sniper
                | WeaponKind::Pistol
                | WeaponKind::Cannon
                | WeaponKind::AutoGun => strike(units, i, target, to, weapon, teams),
                WeaponKind::Flame | WeaponKind::Gas => self.spray(units, i, teams, forced),
                WeaponKind::Laser => {
                    strike(units, i, target, to, weapon, teams);
                    self.beams.push(Beam { from, to, life: 1.0, repair: false });
                }
                WeaponKind::Repair => {
                    if let Some(t) = target {
                        let fixed = &mut units[t];
                        fixed.health = (fixed.health + REPAIR_HEALTH).min(fixed.max_health());
                        self.beams.push(Beam { from, to, life: 1.0, repair: true });
                    }
                }
                WeaponKind::Unarmed => {}
                WeaponKind::Missile | WeaponKind::AirMissile => {
                    // From the ground it is launched upward, so that it visibly rises and turns
                    // towards its target. From the air it has a clear shot.
                    let (dx, dz) = (to.0 - from.0, to.2 - from.2);
                    let flat = dx.hypot(dz).max(0.0001);
                    let direction = if units[i].kind.flies() {
                        direction_to(from, to)
                    } else {
                        direction_to((0.0, 0.0, 0.0), (dx / flat, 0.8, dz / flat))
                    };
                    self.missiles.push(Missile {
                        position: from,
                        direction,
                        target_id: target.map_or(NO_UNIT, |t| units[t].id),
                        place: target.is_none().then_some(to),
                        shooter,
                        fuel: MISSILE_FUEL_SECONDS,
                    });
                    units[i].second_tube = !units[i].second_tube;
                }
                WeaponKind::Mortar => {
                    let flat = (to.0 - from.0).hypot(to.2 - from.2);
                    self.shells.push(Shell {
                        start: from,
                        landing: to,
                        owner: units[i].owner,
                        elapsed: 0.0,
                        duration: (flat / SHELL_SPEED).max(0.2),
                        arc: flat * SHELL_ARC,
                        hurts_all: forced,
                        shooter,
                    });
                }
            }
        }
    }

    /// Everyone low enough in the cone in front of a unit gets hurt, if they are enemies of
    /// whoever sprayed it or it was forced to fire there.
    fn spray(&mut self, units: &mut [Unit], sprayer: usize, teams: &[usize], forced: bool) {
        let unit = units[sprayer];
        let Some(half_angle) = unit.weapon.spray_angle() else { return };
        let reach = unit.weapon.range();
        for index in 0..units.len() {
            let other = units[index];
            let harmed = forced || are_enemies(teams, unit.owner, other.owner);
            if index == sprayer || !other.alive || !harmed || !unit.weapon.can_target(other.kind, other.altitude) {
                continue;
            }
            let (dx, dz) = (other.x - unit.x, other.z - unit.z);
            let away = dx.hypot(dz);
            // A big target counts from as far around as its side reaches.
            let off_center = turn_between(unit.heading, dz.atan2(dx)).abs();
            let widened = half_angle + (other.kind.radius() / away.max(0.0001)).min(1.0).asin();
            if away <= reach + other.kind.radius() && off_center <= widened {
                hit(units, index, unit.weapon, Shooter::of(&unit), teams);
            }
        }
        self.sprays.push(Spray {
            from: unit.muzzle(),
            heading: unit.heading,
            reach,
            half_angle,
            gas: unit.weapon == WeaponKind::Gas,
            life: 1.0,
        });
    }

    fn fly_missiles(&mut self, units: &mut [Unit], teams: &[usize], terrain: &Terrain, dt: f32) {
        let mut flying = Vec::with_capacity(self.missiles.len());
        for mut missile in std::mem::take(&mut self.missiles) {
            let target = units.iter().position(|u| u.alive && u.id == missile.target_id);
            let heading_for = missile.place.or_else(|| target.map(|t| units[t].center()));
            if let Some(goal) = heading_for {
                missile.direction =
                    turn_towards(missile.direction, direction_to(missile.position, goal), MISSILE_TURN * dt);
            }
            let (x, y, z) = missile.position;
            let (dx, dy, dz) = missile.direction;
            let step = MISSILE_SPEED * dt;
            missile.position = (x + dx * step, y + dy * step, z + dz * step);
            missile.fuel -= dt;

            let reached = target.is_some_and(|t| {
                distance(missile.position, units[t].center()) <= units[t].kind.radius() + MISSILE_FUSE
            });
            let (x, y, z) = missile.position;
            let reached_place = missile.place.is_some_and(|place| {
                distance(missile.position, place) <= MISSILE_FUSE * 2.0 || y <= terrain.height_at(x, z)
            });
            if reached_place {
                // Whoever stands at the place it was sent to is hurt, whoever they are.
                for index in 0..units.len() {
                    let reach = MISSILE_BLAST_RADIUS + units[index].kind.radius();
                    if units[index].alive && distance(missile.position, units[index].center()) <= reach {
                        hit(units, index, WeaponKind::Missile, missile.shooter, teams);
                    }
                }
                self.blasts.push(Blast { at: missile.position, radius: MISSILE_BLAST_RADIUS, life: 1.0 });
            } else if reached {
                let t = target.unwrap_or(0);
                hit(units, t, WeaponKind::Missile, missile.shooter, teams);
                self.blasts.push(Blast { at: missile.position, radius: MISSILE_BLAST_RADIUS, life: 1.0 });
            } else if y <= terrain.height_at(x, z) || missile.fuel <= 0.0 {
                // A miss: it hits the ground or burns out, harming nobody.
                self.blasts.push(Blast { at: missile.position, radius: MISSILE_BLAST_RADIUS * 0.6, life: 1.0 });
            } else {
                flying.push(missile);
            }
        }
        self.missiles = flying;
    }

    fn fly_shells(&mut self, units: &mut [Unit], teams: &[usize], dt: f32) {
        let mut flying = Vec::with_capacity(self.shells.len());
        for mut shell in std::mem::take(&mut self.shells) {
            shell.elapsed += dt;
            if shell.elapsed < shell.duration {
                flying.push(shell);
                continue;
            }
            // Only enemies of whoever fired are hurt, so that units can fire freely.
            for index in 0..units.len() {
                let unit = units[index];
                if unit.alive && (shell.hurts_all || are_enemies(teams, shell.owner, unit.owner)) {
                    let away = distance(shell.landing, unit.center());
                    let reach = SHELL_BLAST_RADIUS + unit.kind.radius();
                    if away < reach {
                        let damage = WeaponKind::Mortar.damage() * shell.shooter.power * (1.0 - away / reach * 0.5);
                        harm(units, index, damage, shell.shooter, teams);
                    }
                }
            }
            self.blasts.push(Blast { at: shell.landing, radius: SHELL_BLAST_RADIUS, life: 1.0 });
        }
        self.shells = flying;
    }

    fn fall_strikes(&mut self, units: &mut [Unit], dt: f32) {
        let mut falling = Vec::with_capacity(self.strikes.len());
        for mut strike in std::mem::take(&mut self.strikes) {
            strike.elapsed += dt;
            if strike.elapsed < STRIKE_SECONDS {
                falling.push(strike);
                continue;
            }
            // Radiation does not care whose units stand there.
            for unit in units.iter_mut().filter(|u| u.alive) {
                let away = (unit.x - strike.x).hypot(unit.z - strike.z);
                if away < STRIKE_RADIUS {
                    hurt(unit, STRIKE_DAMAGE * (1.0 - away / STRIKE_RADIUS * 0.5));
                }
            }
            self.blasts.push(Blast { at: (strike.x, 0.0, strike.z), radius: STRIKE_RADIUS, life: 1.0 });
            self.poison.push(PoisonZone { x: strike.x, z: strike.z, life: POISON_SECONDS });
        }
        self.strikes = falling;
    }

    fn poison_land(&mut self, units: &mut [Unit], dt: f32) {
        for zone in &mut self.poison {
            zone.life -= dt;
            for unit in units.iter_mut().filter(|u| u.alive && !u.kind.flies()) {
                if (unit.x - zone.x).hypot(unit.z - zone.z) < STRIKE_RADIUS {
                    hurt(unit, POISON_DAMAGE * dt);
                }
            }
        }
        self.poison.retain(|z| z.life > 0.0);
    }
}

/// A shot striking at once: its target, or whoever stands at the place it was fired at.
fn strike(units: &mut [Unit], firing: usize, target: Option<usize>, at: Point, weapon: WeaponKind, teams: &[usize]) {
    let shooter = Shooter::of(&units[firing]);
    if let Some(t) = target {
        hit(units, t, weapon, shooter, teams);
        return;
    }
    for index in 0..units.len() {
        let unit = units[index];
        let touched = distance(at, unit.center()) <= unit.kind.radius() + PLACE_SHOT_SPREAD;
        if index != firing && unit.alive && touched && weapon.can_target(unit.kind, unit.altitude) {
            hit(units, index, weapon, shooter, teams);
        }
    }
}

fn hit(units: &mut [Unit], victim: usize, weapon: WeaponKind, shooter: Shooter, teams: &[usize]) {
    let damage = weapon.damage_to(units[victim].kind) * shooter.power;
    harm(units, victim, damage, shooter, teams);
}

/// Hurts a unit on someone's account. Whoever kills an enemy gains experience, if they
/// still live. Killing a friend earns nothing.
fn harm(units: &mut [Unit], victim: usize, damage: f32, shooter: Shooter, teams: &[usize]) {
    let was_alive = units[victim].alive;
    hurt(&mut units[victim], damage);
    if was_alive && !units[victim].alive && are_enemies(teams, shooter.owner, units[victim].owner) {
        let value = units[victim].kind.kill_value();
        if let Some(killer) = units.iter_mut().find(|u| u.alive && u.id == shooter.id) {
            killer.gain_experience(value);
        }
    }
}

/// Units with a medal slowly win health back by themselves.
fn heal_veterans(units: &mut [Unit], dt: f32) {
    for unit in units.iter_mut().filter(|u| u.alive && u.rank() > 0) {
        let healed = unit.health + VETERAN_HEALING * unit.rank() as f32 * dt;
        unit.health = healed.min(unit.max_health());
    }
}

fn hurt(unit: &mut Unit, damage: f32) {
    unit.health -= damage;
    if unit.health <= 0.0 {
        unit.alive = false;
        unit.selected = false;
    }
}

/// Towers, sprayers and catapults can only fire the way they face.
fn must_turn_to_fire(unit: &Unit) -> bool {
    unit.kind.is_building() || unit.weapon.spray_angle().is_some() || unit.role == Role::Catapult
}

/// What a unit fires at this moment.
#[derive(Clone, Copy)]
enum Aim {
    Unit(usize),
    Place(Point),
}

/// The place it was told to attack if that is in range, otherwise as `choose_target`.
fn choose_aim(units: &[Unit], shooter: usize, teams: &[usize], terrain: &Terrain) -> Option<Aim> {
    let unit = &units[shooter];
    if let Some(Target::Place(x, z)) = unit.target {
        let place = (x, terrain.height_at(x, z), z);
        if unit.can_fire_at_ground() && distance(unit.center(), place) <= unit.weapon.range() {
            return Some(Aim::Place(place));
        }
    }
    choose_target(units, shooter, teams).map(Aim::Unit)
}

/// Turns whatever must face its aim towards what it would fire at. A tower with nothing to fire at
/// keeps looking slowly around. Returns whether it now points at its aim.
fn turn_tubes(units: &mut [Unit], tower: usize, aim: Option<Aim>, dt: f32) -> bool {
    let goal = match aim {
        Some(Aim::Unit(t)) => (units[t].x, units[t].z),
        Some(Aim::Place((x, _, z))) => (x, z),
        None => {
            if units[tower].kind.is_building() {
                units[tower].heading += TOWER_IDLE_TURN * dt;
            }
            return false;
        }
    };
    let (dx, dz) = (goal.0 - units[tower].x, goal.1 - units[tower].z);
    let turn = turn_between(units[tower].heading, dz.atan2(dx));
    let most = TOWER_TURN * dt;
    units[tower].heading += turn.clamp(-most, most);
    (turn - turn.clamp(-most, most)).abs() <= TOWER_ALIGNMENT
}

/// The enemy it was told to attack if that one is in range, otherwise whoever is nearest.
fn choose_target(units: &[Unit], shooter: usize, teams: &[usize]) -> Option<usize> {
    let unit = &units[shooter];
    if unit.weapon == WeaponKind::Repair {
        return nearest_to_repair(units, shooter, teams);
    }
    let (id, forced) = match unit.target {
        Some(Target::Unit(id)) => (Some(id), false),
        Some(Target::Forced(id)) => (Some(id), true),
        _ => (None, false),
    };
    let ordered = id.and_then(|id| units.iter().position(|u| u.alive && u.id == id));
    ordered
        .filter(|&t| {
            let fit = if forced { unit.can_fire_on(&units[t]) } else { unit.can_attack(&units[t], teams) };
            fit && distance(unit.center(), units[t].center()) <= unit.weapon.range()
        })
        .or_else(|| nearest_enemy(units, shooter, teams))
}

/// The vehicle it was sent to if that is hurt and within reach, otherwise the nearest other
/// hurt vehicle within reach, of the same side or an ally's.
fn nearest_to_repair(units: &[Unit], repairer: usize, teams: &[usize]) -> Option<usize> {
    let from = units[repairer].center();
    let owner = units[repairer].owner;
    let reach = units[repairer].weapon.range();
    let needs_it = |i: usize| {
        let unit = &units[i];
        i != repairer
            && unit.alive
            && unit.can_be_repaired()
            && unit.health < unit.max_health()
            && are_friends(teams, owner, unit.owner)
            && distance(from, unit.center()) <= reach + unit.kind.radius()
    };
    let sent_to = match units[repairer].target {
        Some(Target::Unit(id)) => units.iter().position(|u| u.id == id).filter(|&i| needs_it(i)),
        _ => None,
    };
    sent_to.or_else(|| {
        (0..units.len())
            .filter(|&i| needs_it(i))
            .min_by(|&a, &b| distance(from, units[a].center()).total_cmp(&distance(from, units[b].center())))
    })
}

/// The nearest living enemy within a unit's weapon range.
fn nearest_enemy(units: &[Unit], shooter: usize, teams: &[usize]) -> Option<usize> {
    let from = units[shooter].center();
    let owner = units[shooter].owner;
    let mut best = None;
    let weapon = units[shooter].weapon;
    let mut best_distance = weapon.range();
    for (i, unit) in units.iter().enumerate() {
        if unit.alive && are_enemies(teams, owner, unit.owner) && weapon.can_target(unit.kind, unit.altitude) {
            let away = distance(from, unit.center());
            if away <= best_distance {
                best = Some(i);
                best_distance = away;
            }
        }
    }
    best
}

fn direction_to(from: Point, to: Point) -> Point {
    let length = distance(from, to).max(0.0001);
    ((to.0 - from.0) / length, (to.1 - from.1) / length, (to.2 - from.2) / length)
}

/// Turns a unit vector towards another by at most `most` radians, in any direction in 3D.
fn turn_towards(from: Point, to: Point, most: f32) -> Point {
    let cos = (from.0 * to.0 + from.1 * to.1 + from.2 * to.2).clamp(-1.0, 1.0);
    let angle = cos.acos();
    if angle <= most {
        return to;
    }
    // The part of `to` at right angles to `from` says which way to rotate. Facing exactly
    // away there is none, and any sideways direction will do.
    let mut side = (to.0 - from.0 * cos, to.1 - from.1 * cos, to.2 - from.2 * cos);
    if distance((0.0, 0.0, 0.0), side) < 0.0001 {
        side = if from.0.abs() < 0.9 { (1.0 - from.0 * from.0, -from.0 * from.1, -from.0 * from.2) } else { (-from.1 * from.0, 1.0 - from.1 * from.1, -from.1 * from.2) };
    }
    let side = direction_to((0.0, 0.0, 0.0), side);
    let (sin_most, cos_most) = most.sin_cos();
    (
        from.0 * cos_most + side.0 * sin_most,
        from.1 * cos_most + side.1 * sin_most,
        from.2 * cos_most + side.2 * sin_most,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::TeamColor;

    const TEAMS: [usize; 2] = [0, 1];

    fn flat() -> &'static Terrain {
        static OPEN: std::sync::OnceLock<Terrain> = std::sync::OnceLock::new();
        OPEN.get_or_init(|| Terrain::open_with_ruins(Vec::new()))
    }

    fn shooter(weapon: WeaponKind) -> Unit {
        let mut unit = Unit::for_test(0.0, 60.0, UnitKind::Infantry);
        unit.id = 1;
        unit.weapon = weapon;
        unit.cooldown = 0.0;
        unit
    }

    fn target_at(x: f32, z: f32) -> Unit {
        let mut unit = Unit::for_test(x, z, UnitKind::Infantry);
        unit.id = 2;
        unit.owner = 1;
        unit.color = TeamColor::Red;
        // Unable to shoot back during the test.
        unit.cooldown = 1000.0;
        unit
    }

    fn run(combat: &mut Combat, units: &mut [Unit], seconds: f32) {
        for _ in 0..(seconds / 0.05).round() as usize {
            combat.advance(units, &TEAMS, flat(), 0.05);
        }
    }

    #[test]
    fn bullets_strike_at_once_invisibly_within_range() {
        let mut units = [shooter(WeaponKind::Bullet), target_at(5.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(units[1].health < 100.0, "hit on the first step");
        assert_eq!(combat.flares.len(), 1, "only a flare to see, where it was fired");
        assert_eq!(combat.flares[0].at, units[0].muzzle());
        assert!(combat.beams.is_empty() && combat.missiles.is_empty() && combat.blasts.is_empty());
        run(&mut combat, &mut units, 0.2);
        assert!(combat.flares.is_empty(), "and it fades");
    }

    #[test]
    fn nothing_fires_beyond_its_range_or_at_friends() {
        for weapon in [WeaponKind::Bullet, WeaponKind::Missile, WeaponKind::Laser, WeaponKind::Mortar] {
            let mut units = [shooter(weapon), target_at(weapon.range() + 1.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 2.0);
            assert_eq!(units[1].health, 100.0, "{weapon:?} out of range");
            assert!(combat.missiles.is_empty() && combat.shells.is_empty(), "{weapon:?} held fire");

            let mut units = [shooter(weapon), target_at(3.0, 60.0)];
            let mut combat = Combat::default();
            for _ in 0..40 {
                combat.advance(&mut units, &[0, 0], flat(), 0.05);
            }
            assert_eq!(units[1].health, 100.0, "{weapon:?} at an ally");
        }
    }

    #[test]
    fn weapons_wait_for_their_cooldown() {
        let mut units = [shooter(WeaponKind::Bullet), target_at(5.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 0.1);
        let after_one = units[1].health;
        assert_eq!(after_one, 100.0 - WeaponKind::Bullet.damage());
        run(&mut combat, &mut units, 0.4);
        assert_eq!(units[1].health, after_one, "still reloading");
        run(&mut combat, &mut units, 0.3);
        assert!(units[1].health < after_one, "fires again");
    }

    #[test]
    fn repeated_hits_kill() {
        let mut units = [shooter(WeaponKind::Bullet), target_at(5.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 10.0);
        assert!(!units[1].alive);
    }

    #[test]
    fn bullets_barely_hurt_vehicles_and_other_weapons_do() {
        assert!(WeaponKind::Bullet.damage_to(UnitKind::Vehicle) < WeaponKind::Bullet.damage());
        assert_eq!(WeaponKind::Mortar.damage_to(UnitKind::Vehicle), WeaponKind::Mortar.damage());
    }

    #[test]
    fn a_laser_strikes_at_once_with_a_straight_visible_line_that_fades() {
        let mut units = [shooter(WeaponKind::Laser), target_at(5.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(units[1].health < 100.0, "immediate");
        assert_eq!(combat.beams.len(), 1, "visible");
        assert_eq!(combat.beams[0].from, units[0].muzzle());
        assert_eq!(combat.beams[0].to, units[1].center());
        run(&mut combat, &mut units, 0.3);
        assert!(combat.beams.is_empty(), "fades");
    }

    #[test]
    fn a_missile_takes_time_to_fly_then_hits_a_standing_target() {
        let mut units = [shooter(WeaponKind::Missile), target_at(9.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.missiles.len(), 1);
        assert_eq!(units[1].health, 100.0, "not yet");
        run(&mut combat, &mut units, 0.5);
        assert_eq!(units[1].health, 100.0, "still flying");
        units[0].cooldown = 1000.0;
        run(&mut combat, &mut units, 3.0);
        assert_eq!(units[1].health, 100.0 - WeaponKind::Missile.damage());
        assert!(combat.missiles.is_empty());
    }

    #[test]
    fn a_missile_can_be_outrun_and_then_harms_nobody() {
        let mut units = [shooter(WeaponKind::Missile), target_at(11.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        for _ in 0..200 {
            // Racing away faster than any missile flies.
            units[1].x += 20.0 * 0.05;
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert_eq!(units[1].health, 100.0, "it missed");
        assert!(combat.missiles.is_empty(), "and burned out");
    }

    #[test]
    fn a_missile_turns_only_so_fast() {
        let (from, to) = ((1.0, 0.0, 0.0), (0.0, 0.0, 1.0));
        let angle_from_start = |turned: Point| (turned.0 * from.0 + turned.1 * from.1 + turned.2 * from.2).acos();
        let turned = turn_towards(from, to, 0.1);
        assert!((angle_from_start(turned) - 0.1).abs() < 0.001, "the full allowed turn");
        assert_eq!(turn_towards(from, from, 0.1), from);

        // The same turning rate holds however far off the target is, even almost behind it.
        for away in [(-1.0, 0.0, 0.0), (-0.9999, 0.0141, 0.0), (-0.7071, 0.7071, 0.0)] {
            let turned = turn_towards(from, away, 0.1);
            assert!((angle_from_start(turned) - 0.1).abs() < 0.001, "{away:?} gave {turned:?}");
            let length = distance((0.0, 0.0, 0.0), turned);
            assert!((length - 1.0).abs() < 0.001, "stays a unit vector: {length}");
        }
    }

    #[test]
    fn a_missile_climbs_to_chase_a_target_in_the_air_and_can_dive_again() {
        let mut units = [shooter(WeaponKind::Missile), target_at(3.0, 60.0)];
        units[1].altitude = 10.0;
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        assert_eq!(combat.missiles.len(), 1, "it is in range");
        let mut highest = 0.0_f32;
        while let Some(missile) = combat.missiles.first() {
            highest = highest.max(missile.position.1);
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(highest > 8.0, "it climbed: {highest}");
        assert!(units[1].health < 100.0, "and hit it in the air");

        // Launched at something suddenly below it, it comes back down.
        let mut units = [shooter(WeaponKind::Missile), target_at(0.5, 60.0)];
        units[0].altitude = 15.0;
        units[0].cooldown = 1000.0;
        let mut combat = Combat::default();
        combat.missiles.push(Missile {
            position: units[0].muzzle(),
            direction: (0.0, 1.0, 0.0),
            target_id: units[1].id,
            place: None,
            shooter: Shooter::of(&units[0]),
            fuel: 4.0,
        });
        let mut lowest = f32::MAX;
        while let Some(missile) = combat.missiles.first() {
            lowest = lowest.min(missile.position.1);
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(lowest < 5.0, "it dived: {lowest}");
    }

    #[test]
    fn a_missile_whose_target_dies_flies_on_and_harms_nobody() {
        let mut units = [shooter(WeaponKind::Missile), target_at(9.0, 60.0), target_at(9.0, 62.0)];
        units[2].id = 3;
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        let aimed_at = combat.missiles[0].target_id;
        let index = units.iter().position(|u| u.id == aimed_at).unwrap();
        units[index].alive = false;
        run(&mut combat, &mut units, 5.0);
        assert!(combat.missiles.is_empty());
        let other = if index == 1 { 2 } else { 1 };
        assert_eq!(units[other].health, 100.0);
    }

    #[test]
    fn a_shell_arcs_and_lands_later_with_an_explosion_hurting_those_near() {
        let mut units = [shooter(WeaponKind::Mortar), target_at(14.0, 60.0), target_at(14.5, 60.5), target_at(14.0, 66.0)];
        units[2].id = 3;
        units[3].id = 4;
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        assert_eq!(combat.shells.len(), 1);
        assert_eq!(units[1].health, 100.0, "not yet");

        let start_y = combat.shells[0].position().1;
        let landing_y = combat.shells[0].landing().1;
        run(&mut combat, &mut units, 1.0);
        let middle = combat.shells[0].position();
        assert!(middle.1 > start_y && middle.1 > landing_y, "it flies in an arc: {middle:?}");

        run(&mut combat, &mut units, 2.0);
        assert!(combat.shells.is_empty());
        assert!(units[1].health < 100.0, "direct hit");
        assert!(units[2].health < 100.0, "the one beside it too");
        assert!(units[1].health < units[2].health, "less farther out");
        assert_eq!(units[3].health, 100.0, "outside the radius");
    }

    #[test]
    fn a_shell_misses_a_target_that_moves_away() {
        let mut units = [shooter(WeaponKind::Mortar), target_at(14.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        units[1].z = 70.0;
        run(&mut combat, &mut units, 3.0);
        assert_eq!(units[1].health, 100.0);
        assert!(combat.shells.is_empty(), "it landed all the same");
    }

    #[test]
    fn mortars_do_not_hurt_their_own_side() {
        let mut friend = Unit::for_test(14.0, 60.5, UnitKind::Infantry);
        friend.id = 9;
        friend.cooldown = 1000.0;
        let mut units = [shooter(WeaponKind::Mortar), target_at(14.0, 60.0), friend];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        run(&mut combat, &mut units, 3.0);
        assert!(units[1].health < 100.0);
        assert_eq!(units[2].health, 100.0);
    }

    #[test]
    fn a_strike_falls_then_hurts_everyone_in_its_radius_and_poisons_the_land() {
        let mut near = target_at(1.0, 60.0);
        near.cooldown = 1000.0;
        let mut own = Unit::for_test(0.0, 61.0, UnitKind::Vehicle);
        own.id = 8;
        own.cooldown = 1000.0;
        let mut edge = target_at(6.5, 60.0);
        edge.id = 3;
        let mut far = target_at(20.0, 60.0);
        far.id = 4;
        let mut units = [near, own, edge, far];
        let mut combat = Combat::default();
        combat.launch_strike(0.0, 60.0);

        run(&mut combat, &mut units, 1.0);
        assert_eq!(units[1].health, UnitKind::Vehicle.max_health(), "still falling");
        assert!(combat.strikes[0].height() < STRIKE_HEIGHT);

        run(&mut combat, &mut units, 0.6);
        assert!(combat.strikes.is_empty());
        assert!(!units[0].alive, "the infantry beside it dies");
        assert!(units[1].health < UnitKind::Vehicle.max_health(), "its own side is not spared");
        assert_eq!(units[3].health, 100.0, "beyond the radius");
        assert_eq!(combat.poison.len(), 1);
        assert!((combat.poison[0].life - POISON_SECONDS).abs() < 0.2);
    }

    #[test]
    fn the_land_stays_poisoned_for_a_minute_and_hurts_who_stands_there() {
        let mut combat = Combat::default();
        combat.poison.push(PoisonZone { x: 0.0, z: 60.0, life: POISON_SECONDS });
        let mut inside = Unit::for_test(3.0, 60.0, UnitKind::Vehicle);
        inside.id = 1;
        inside.cooldown = 1000.0;
        let mut outside = Unit::for_test(10.0, 60.0, UnitKind::Infantry);
        outside.id = 2;
        outside.cooldown = 1000.0;
        let mut units = [inside, outside];

        run(&mut combat, &mut units, 10.0);
        let hurt_so_far = UnitKind::Vehicle.max_health() - units[0].health;
        assert!((hurt_so_far - POISON_DAMAGE * 10.0).abs() < 1.0, "{hurt_so_far}");
        assert_eq!(units[1].health, 100.0);

        run(&mut combat, &mut units, 49.0);
        assert_eq!(combat.poison.len(), 1, "still there just before the minute is up");
        run(&mut combat, &mut units, 2.0);
        assert!(combat.poison.is_empty(), "gone after a minute");
    }

    fn plane_at(x: f32, z: f32) -> Unit {
        let mut unit = Unit::for_test(x, z, UnitKind::Aircraft);
        unit.id = 5;
        unit.owner = 1;
        unit.color = TeamColor::Red;
        unit.cooldown = 1000.0;
        unit
    }

    #[test]
    fn mortars_cannot_aim_at_aircraft_but_everything_else_can() {
        let mut units = [shooter(WeaponKind::Mortar), plane_at(5.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert!(combat.shells.is_empty(), "held fire");

        for weapon in [WeaponKind::Bullet, WeaponKind::Laser, WeaponKind::Missile] {
            let mut units = [shooter(weapon), plane_at(5.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 3.0);
            assert!(units[1].health < UnitKind::Aircraft.max_health(), "{weapon:?}");
        }
    }

    #[test]
    fn a_mortar_prefers_what_it_can_reach_over_a_nearer_aircraft() {
        let mut units = [shooter(WeaponKind::Mortar), plane_at(3.0, 60.0), target_at(14.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.shells.len(), 1);
        assert_eq!(combat.shells[0].landing(), units[2].center());
    }

    #[test]
    fn a_shell_bursting_on_the_ground_does_not_reach_aircraft_overhead() {
        let mut units = [shooter(WeaponKind::Mortar), target_at(14.0, 60.0), plane_at(14.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        run(&mut combat, &mut units, 3.0);
        assert!(units[1].health < 100.0, "the infantry below is hit");
        assert_eq!(units[2].health, UnitKind::Aircraft.max_health());
    }

    #[test]
    fn an_aircraft_fires_missiles_down_at_units_beneath_it() {
        let mut plane = Unit::for_test(0.0, 60.0, UnitKind::Aircraft);
        plane.id = 1;
        plane.weapon = WeaponKind::Missile;
        for target_x in [0.5, 6.0, 11.0] {
            let mut units = [plane, target_at(target_x, 60.0)];
            let mut combat = Combat::default();
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            assert!(combat.missiles[0].direction.1 < 0.0, "aimed down");
            units[0].cooldown = 1000.0;
            run(&mut combat, &mut units, 3.0);
            assert!(units[1].health < 100.0, "from {target_x} away");
        }
    }

    #[test]
    fn poisoned_land_spares_aircraft_but_the_strike_itself_does_not() {
        let mut combat = Combat::default();
        combat.poison.push(PoisonZone { x: 0.0, z: 60.0, life: POISON_SECONDS });
        let mut units = [plane_at(1.0, 60.0)];
        run(&mut combat, &mut units, 5.0);
        assert_eq!(units[0].health, UnitKind::Aircraft.max_health());

        let mut combat = Combat::default();
        combat.launch_strike(0.0, 60.0);
        run(&mut combat, &mut units, 2.0);
        assert!(!units[0].alive);
    }

    #[test]
    fn a_unit_fires_at_the_enemy_it_was_told_to_attack_rather_than_the_nearest() {
        let mut far = target_at(6.0, 60.0);
        far.id = 3;
        let mut units = [shooter(WeaponKind::Bullet), target_at(2.0, 60.0), far];
        units[0].target = Some(Target::Unit(3));
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(units[1].health, 100.0, "the nearer one is spared");
        assert!(units[2].health < 100.0);
    }

    #[test]
    fn until_its_target_is_in_range_a_unit_fires_at_whoever_is_nearest() {
        let mut far = target_at(30.0, 60.0);
        far.id = 3;
        let mut units = [shooter(WeaponKind::Bullet), target_at(2.0, 60.0), far];
        units[0].target = Some(Target::Unit(3));
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(units[1].health < 100.0);
        assert_eq!(units[2].health, 100.0);
    }

    fn tower() -> Unit {
        let mut unit = Unit::for_test(0.0, 60.0, UnitKind::Tower);
        unit.id = 1;
        unit.weapon = WeaponKind::Missile;
        unit.heading = 0.0;
        unit
    }

    #[test]
    fn a_tower_turns_its_tubes_to_its_target_before_it_fires() {
        // The target is due west, behind the tubes pointing east.
        let mut units = [tower(), target_at(-8.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 0.5);
        assert!(combat.missiles.is_empty(), "still turning");
        assert_ne!(units[0].heading, 0.0);
        run(&mut combat, &mut units, 1.5);
        assert_eq!(combat.missiles.len(), 1, "now it fires");
        assert!(units[0].heading.cos() < -0.9, "facing west: {}", units[0].heading);
    }

    #[test]
    fn a_tower_keeps_turning_slowly_while_nobody_is_in_range() {
        let mut units = [tower(), target_at(40.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert!(combat.missiles.is_empty());
        assert!((units[0].heading - TOWER_IDLE_TURN * 2.0).abs() < 0.01, "{}", units[0].heading);
    }

    #[test]
    fn a_tower_fires_from_each_tube_in_turn_and_its_missiles_hit() {
        let mut units = [tower(), target_at(8.0, 60.0)];
        let mut combat = Combat::default();
        let mut launched_from = Vec::new();
        let mut known = 0;
        for _ in 0..200 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            if combat.missiles.len() > known {
                launched_from.push(combat.missiles.last().unwrap().position);
            }
            known = combat.missiles.len();
        }
        assert!(launched_from.len() >= 2, "{launched_from:?}");
        assert!(distance(launched_from[0], launched_from[1]) > 0.1, "from the other tube");
        assert!(units[1].health < 100.0);
    }

    #[test]
    fn a_tower_shoots_at_aircraft_too() {
        let mut units = [tower(), plane_at(5.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 4.0);
        assert!(units[1].health < UnitKind::Aircraft.max_health());
    }

    #[test]
    fn a_tower_is_destroyed_by_enough_fire_and_bullets_barely_scratch_it() {
        assert!(WeaponKind::Bullet.damage_to(UnitKind::Tower) < WeaponKind::Bullet.damage());
        let mut enemy_tower = tower();
        enemy_tower.owner = 1;
        enemy_tower.cooldown = 100_000.0;
        let mut units = [shooter(WeaponKind::Laser), enemy_tower];
        units[0].x = -5.0;
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 10.0);
        assert!(units[1].alive && units[1].health < UnitKind::Tower.max_health(), "it takes a while");
        run(&mut combat, &mut units, 40.0);
        assert!(!units[1].alive, "but it falls");
    }

    #[test]
    fn every_gun_that_strikes_at_once_shows_a_flare_and_the_cannons_is_largest() {
        for weapon in [WeaponKind::Bullet, WeaponKind::Sniper, WeaponKind::Pistol, WeaponKind::Cannon] {
            let mut units = [shooter(weapon), target_at(4.0, 60.0)];
            let mut combat = Combat::default();
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            assert!(units[1].health < 100.0, "{weapon:?}");
            assert_eq!(combat.flares.len(), 1, "{weapon:?}");
            assert!(combat.beams.is_empty(), "{weapon:?} leaves no line");
        }
        let size = |w: WeaponKind| w.flare().unwrap();
        assert!(size(WeaponKind::Cannon) > size(WeaponKind::Sniper));
        assert_eq!(WeaponKind::Missile.flare(), None);
    }

    #[test]
    fn a_sniper_fires_slower_than_a_bullet_but_farther_and_harder() {
        let (sniper, bullet) = (WeaponKind::Sniper, WeaponKind::Bullet);
        assert!(sniper.cooldown() > bullet.cooldown());
        assert!(sniper.range() > bullet.range());
        assert!(sniper.damage() > bullet.damage());
    }

    #[test]
    fn a_pistol_is_weak_and_slow_and_a_cannon_much_stronger_even_against_armor() {
        let (pistol, cannon, bullet) = (WeaponKind::Pistol, WeaponKind::Cannon, WeaponKind::Bullet);
        assert!(pistol.damage() < bullet.damage() && pistol.cooldown() > bullet.cooldown());
        assert!(cannon.damage() > bullet.damage() * 4.0);
        assert_eq!(cannon.damage_to(UnitKind::Vehicle), cannon.damage());
    }

    #[test]
    fn the_unarmed_hold_fire_and_harm_nobody() {
        for weapon in [WeaponKind::Unarmed, WeaponKind::Repair] {
            let mut units = [shooter(weapon), target_at(1.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 3.0);
            assert_eq!(units[1].health, 100.0, "{weapon:?}");
        }
    }

    fn repair_scene(friend_owner: usize, friend_x: f32, kind: UnitKind) -> [Unit; 2] {
        let mut repairer = Unit::for_test(0.0, 60.0, UnitKind::Vehicle);
        repairer.id = 1;
        repairer.weapon = WeaponKind::Repair;
        let mut friend = Unit::for_test(friend_x, 60.0, kind);
        friend.id = 2;
        friend.owner = friend_owner;
        friend.cooldown = 1000.0;
        friend.health = 50.0;
        [repairer, friend]
    }

    #[test]
    fn a_repair_vehicle_fixes_nearby_friendly_vehicles_up_to_full_health() {
        let mut units = repair_scene(0, 2.0, UnitKind::Vehicle);
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert!(units[1].health > 50.0);
        assert!(combat.beams.iter().all(|b| b.repair) && !combat.beams.is_empty() || units[1].health > 50.0);
        run(&mut combat, &mut units, 30.0);
        assert_eq!(units[1].health, units[1].max_health());
    }

    #[test]
    fn a_repair_vehicle_helps_allies_but_not_enemies_infantry_or_those_far_off() {
        let run_scene = |units: &mut [Unit; 2], teams: &[usize]| {
            let mut combat = Combat::default();
            for _ in 0..40 {
                combat.advance(units, teams, flat(), 0.05);
            }
        };
        let mut ally = repair_scene(1, 2.0, UnitKind::Vehicle);
        run_scene(&mut ally, &[0, 0]);
        assert!(ally[1].health > 50.0, "an ally");

        let mut enemy = repair_scene(1, 2.0, UnitKind::Vehicle);
        run_scene(&mut enemy, &TEAMS);
        assert_eq!(enemy[1].health, 50.0, "an enemy");

        let mut infantry = repair_scene(0, 2.0, UnitKind::Infantry);
        run_scene(&mut infantry, &TEAMS);
        assert_eq!(infantry[1].health, 50.0, "not a vehicle");

        let mut far = repair_scene(0, 9.0, UnitKind::Vehicle);
        run_scene(&mut far, &TEAMS);
        assert_eq!(far[1].health, 50.0, "out of reach");
    }

    #[test]
    fn the_air_defense_gun_is_rapid_and_hits_the_air_and_the_ground_with_only_a_flare() {
        for enemy in [plane_at(4.0, 60.0), target_at(4.0, 60.0)] {
            let mut units = [shooter(WeaponKind::AutoGun), enemy];
            let start = units[1].health;
            let mut combat = Combat::default();
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            assert!(units[1].health < start, "{enemy:?}");
            assert_eq!(combat.flares.len(), 1);
            assert!(combat.beams.is_empty() && combat.missiles.is_empty());
        }
        assert!(WeaponKind::AutoGun.cooldown() < WeaponKind::Bullet.cooldown());
    }

    #[test]
    fn the_drone_fires_missiles_at_aircraft_and_ignores_the_ground() {
        let mut drone = Unit::for_test(0.0, 60.0, UnitKind::Aircraft);
        drone.id = 1;
        drone.weapon = WeaponKind::AirMissile;
        let mut units = [drone, target_at(3.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert!(combat.missiles.is_empty(), "held fire at the ground");
        assert_eq!(units[1].health, 100.0);

        let mut units = [drone, plane_at(4.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.missiles.len(), 1);
        units[0].cooldown = 1000.0;
        run(&mut combat, &mut units, 3.0);
        assert!(units[1].health < UnitKind::Aircraft.max_health(), "and it hit");
    }

    fn sprayer(weapon: WeaponKind) -> Unit {
        let mut unit = shooter(weapon);
        unit.heading = 0.0;
        unit
    }

    #[test]
    fn a_sprayer_turns_to_its_target_first_and_then_burns_it() {
        for weapon in [WeaponKind::Flame, WeaponKind::Gas] {
            // Due west, behind it.
            let mut units = [sprayer(weapon), target_at(-3.0, 60.0)];
            units[1].health = 100_000.0;
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 0.5);
            assert_eq!(units[1].health, 100_000.0, "{weapon:?} still turning");
            assert!(combat.sprays.is_empty());
            assert_ne!(units[0].heading, 0.0);
            run(&mut combat, &mut units, 2.0);
            assert!(units[1].health < 100_000.0, "{weapon:?}");
            assert!(units[0].heading.cos() < -0.9, "{weapon:?} faces west");
            assert_eq!(combat.sprays[0].gas, weapon == WeaponKind::Gas);
        }
    }

    #[test]
    fn a_spray_burns_everyone_in_its_cone_and_nobody_outside_it() {
        let mut in_cone = target_at(3.0, 60.3);
        in_cone.id = 3;
        let mut behind = target_at(-1.0, 60.0);
        behind.id = 4;
        let mut far = target_at(10.0, 60.0);
        far.id = 5;
        let mut friend = Unit::for_test(2.5, 59.8, UnitKind::Infantry);
        friend.id = 6;
        friend.cooldown = 1000.0;
        let mut units = [sprayer(WeaponKind::Flame), target_at(3.0, 60.0), in_cone, behind, far, friend];
        units[0].target = Some(Target::Unit(2));
        let mut combat = Combat::default();
        for _ in 0..4 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(units[1].health < 100.0, "the target");
        assert!(units[2].health < 100.0, "the one beside it");
        assert_eq!(units[3].health, 100.0, "behind it");
        assert_eq!(units[4].health, 100.0, "too far");
        assert_eq!(units[5].health, 100.0, "a friend");
    }

    #[test]
    fn fire_and_gas_do_not_hurt_aircraft_or_get_aimed_at_them() {
        for weapon in [WeaponKind::Flame, WeaponKind::Gas] {
            let mut units = [sprayer(weapon), plane_at(1.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 2.0);
            assert_eq!(units[1].health, UnitKind::Aircraft.max_health(), "{weapon:?}");
            assert!(combat.sprays.is_empty(), "{weapon:?} held fire");

            // Sprayed at the ground below one, it passes under.
            let mut units = [sprayer(weapon), target_at(3.0, 60.0), plane_at(3.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 1.0);
            assert!(units[1].health < 100.0);
            assert_eq!(units[2].health, UnitKind::Aircraft.max_health(), "{weapon:?}");
        }
    }

    #[test]
    fn what_keeps_fire_and_gas_off_is_height_and_not_being_an_aircraft() {
        for weapon in [WeaponKind::Flame, WeaponKind::Gas] {
            let mut low = plane_at(2.0, 60.0);
            low.altitude = SPRAY_CEILING;
            let mut units = [sprayer(weapon), low];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 0.5);
            assert!(units[1].health < UnitKind::Aircraft.max_health(), "{weapon:?} reaches low flyers");

            let mut high = target_at(2.0, 60.0);
            high.altitude = SPRAY_CEILING + 0.5;
            let mut units = [sprayer(weapon), high];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 0.5);
            assert_eq!(units[1].health, 100.0, "{weapon:?} misses whatever is above it");
        }
    }

    #[test]
    fn a_flame_tower_turns_and_throws_fire_from_its_one_nozzle() {
        let mut flame = Unit::for_test(0.0, 60.0, UnitKind::Tower);
        flame.role = crate::units::Role::FlameTower;
        flame.id = 1;
        flame.weapon = WeaponKind::Flame;
        flame.heading = 0.0;
        let mut units = [flame, target_at(0.0, 63.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert!(units[1].health < 100.0);
        assert!(units[0].heading.sin() > 0.9, "it faces south: {}", units[0].heading);
        let (nozzle, center) = (combat.sprays[0].from, units[0].heading);
        assert!((nozzle.0 - center.cos() * 0.7).abs() < 0.01, "straight ahead of it");
    }

    #[test]
    fn firing_at_a_place_hits_whoever_stands_there_friend_or_foe_and_nobody_else() {
        for weapon in [WeaponKind::Bullet, WeaponKind::Sniper, WeaponKind::Laser, WeaponKind::Cannon] {
            let mut friend = Unit::for_test(4.0, 60.0, UnitKind::Infantry);
            friend.id = 3;
            friend.cooldown = 1000.0;
            let mut beside = Unit::for_test(4.0, 63.0, UnitKind::Infantry);
            beside.id = 4;
            beside.cooldown = 1000.0;
            let mut units = [shooter(weapon), friend, beside];
            units[0].target = Some(Target::Place(4.0, 60.0));
            let mut combat = Combat::default();
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            assert!(units[1].health < 100.0, "{weapon:?} hit the friend there");
            assert_eq!(units[2].health, 100.0, "{weapon:?} spared the one nearby");
        }
    }

    #[test]
    fn firing_at_empty_ground_shows_and_hurts_nobody_and_it_keeps_on() {
        let mut units = [shooter(WeaponKind::Laser)];
        units[0].target = Some(Target::Place(4.0, 60.0));
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.beams.len(), 1, "a shot into nowhere");
        assert_eq!(combat.beams[0].to.0, 4.0);
        run(&mut combat, &mut units, 3.0);
        assert!(!combat.beams.is_empty() || units[0].cooldown > 0.0, "still firing");
        assert_eq!(units[0].target, Some(Target::Place(4.0, 60.0)));
    }

    #[test]
    fn a_unit_ordered_at_a_place_ignores_enemies_once_in_range_but_fires_on_them_on_its_way() {
        let mut units = [shooter(WeaponKind::Bullet), target_at(2.0, 60.0)];
        units[0].target = Some(Target::Place(5.0, 60.0));
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert_eq!(units[1].health, 100.0, "it keeps to the place");

        units[0].target = Some(Target::Place(30.0, 60.0));
        run(&mut combat, &mut units, 1.0);
        assert!(units[1].health < 100.0, "out of range of it, the nearest enemy will do");
    }

    #[test]
    fn a_missile_fired_at_a_place_flies_there_and_goes_off() {
        let mut friend = Unit::for_test(6.0, 60.0, UnitKind::Infantry);
        friend.id = 3;
        friend.cooldown = 1000.0;
        let mut units = [shooter(WeaponKind::Missile), friend];
        units[0].target = Some(Target::Place(6.0, 60.0));
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.missiles.len(), 1);
        units[0].cooldown = 1000.0;
        run(&mut combat, &mut units, 3.0);
        assert!(combat.missiles.is_empty());
        assert!(units[1].health < 100.0);
    }

    #[test]
    fn a_shell_fired_at_a_place_or_a_friend_hurts_friends_but_an_ordinary_one_does_not() {
        let mut friend = Unit::for_test(10.0, 60.0, UnitKind::Infantry);
        friend.id = 3;
        friend.cooldown = 1000.0;
        for target in [Target::Place(10.0, 60.0), Target::Forced(3)] {
            let mut units = [shooter(WeaponKind::Mortar), friend];
            units[0].target = Some(target);
            let mut combat = Combat::default();
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            assert_eq!(combat.shells.len(), 1, "{target:?}");
            units[0].cooldown = 1000.0;
            run(&mut combat, &mut units, 3.0);
            assert!(units[1].health < 100.0, "{target:?}");
        }
    }

    #[test]
    fn a_unit_forced_to_fire_on_a_friend_kills_it_while_nobody_else_fires_on_friends() {
        let mut friend = Unit::for_test(3.0, 60.0, UnitKind::Infantry);
        friend.id = 3;
        friend.cooldown = 1000.0;
        let mut units = [shooter(WeaponKind::Bullet), friend];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert_eq!(units[1].health, 100.0, "not without the order");

        units[0].target = Some(Target::Forced(3));
        run(&mut combat, &mut units, 10.0);
        assert!(!units[1].alive);
    }

    #[test]
    fn fire_and_gas_turn_towards_a_place_and_only_then_burn_who_stands_there() {
        let mut friend = Unit::for_test(-3.0, 60.0, UnitKind::Infantry);
        friend.id = 3;
        friend.cooldown = 1000.0;
        let mut units = [sprayer(WeaponKind::Flame), friend];
        units[0].target = Some(Target::Place(-3.0, 60.0));
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 0.5);
        assert_eq!(units[1].health, 100.0, "still turning");
        run(&mut combat, &mut units, 2.0);
        assert!(units[1].health < 100.0);
    }

    #[test]
    fn a_repair_vehicle_prefers_the_vehicle_it_was_sent_to() {
        let units = repair_scene(0, 2.0, UnitKind::Vehicle);
        let mut other = units[1];
        other.id = 3;
        other.x = 2.5;
        let mut units = [units[0], units[1], other];
        units[0].target = Some(Target::Unit(3));
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(units[1].health, 50.0, "the nearer one waits");
        assert!(units[2].health > 50.0);
    }

    fn moving(mut unit: Unit) -> Unit {
        unit.destination = (unit.x - 30.0, unit.z);
        unit
    }

    #[test]
    fn only_lasers_and_missiles_can_be_fired_on_the_move() {
        for weapon in [WeaponKind::Laser, WeaponKind::Missile] {
            let mut units = [moving(shooter(weapon)), target_at(4.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 1.0);
            assert!(!combat.beams.is_empty() || !combat.missiles.is_empty() || units[1].health < 100.0, "{weapon:?}");
        }
        for weapon in [
            WeaponKind::Bullet,
            WeaponKind::Sniper,
            WeaponKind::Pistol,
            WeaponKind::Cannon,
            WeaponKind::AutoGun,
            WeaponKind::Mortar,
            WeaponKind::Flame,
            WeaponKind::Gas,
        ] {
            let mut units = [moving(shooter(weapon)), target_at(3.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 2.0);
            assert_eq!(units[1].health, 100.0, "{weapon:?} holds fire");
            assert_eq!(combat, Combat::default(), "{weapon:?} shows nothing");

            units[0].destination = (units[0].x, units[0].z);
            run(&mut combat, &mut units, 3.0);
            assert!(combat != Combat::default() || units[1].health < 100.0, "{weapon:?} fires once stopped");
        }
    }

    #[test]
    fn a_repair_vehicle_on_the_move_stops_working() {
        let mut units = repair_scene(0, 2.0, UnitKind::Vehicle);
        units[0] = moving(units[0]);
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert_eq!(units[1].health, 50.0);
    }

    #[test]
    fn a_kill_earns_experience_by_what_was_killed_and_a_hit_alone_earns_none() {
        for (kind, health) in [(UnitKind::Infantry, 12.0), (UnitKind::Vehicle, 3.0), (UnitKind::Tower, 3.0)] {
            let mut enemy = target_at(4.0, 60.0);
            enemy.kind = kind;
            enemy.health = health * 2.0;
            let mut units = [shooter(WeaponKind::Bullet), enemy];
            let mut combat = Combat::default();
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            assert!(units[1].alive, "{kind:?}");
            assert_eq!(units[0].experience, 0, "{kind:?} only hit");
            run(&mut combat, &mut units, 1.0);
            assert!(!units[1].alive, "{kind:?}");
            assert_eq!(units[0].experience, kind.kill_value(), "{kind:?}");
        }
    }

    #[test]
    fn experience_comes_from_every_way_of_killing() {
        let kill_with = |weapon: WeaponKind, x: f32| {
            let mut enemy = target_at(x, 60.0);
            enemy.health = 1.0;
            let mut unit = shooter(weapon);
            unit.heading = 0.0;
            let mut units = [unit, enemy];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 4.0);
            assert!(!units[1].alive, "{weapon:?}");
            units[0].experience
        };
        for (weapon, x) in [
            (WeaponKind::Bullet, 4.0),
            (WeaponKind::Laser, 4.0),
            (WeaponKind::Missile, 8.0),
            (WeaponKind::Mortar, 10.0),
            (WeaponKind::Flame, 2.0),
            (WeaponKind::Gas, 2.0),
        ] {
            assert_eq!(kill_with(weapon, x), 1, "{weapon:?}");
        }
    }

    #[test]
    fn killing_a_friend_earns_nothing() {
        let mut friend = Unit::for_test(3.0, 60.0, UnitKind::Infantry);
        friend.id = 3;
        friend.health = 1.0;
        friend.cooldown = 1000.0;
        let mut units = [shooter(WeaponKind::Bullet), friend];
        units[0].target = Some(Target::Forced(3));
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert!(!units[1].alive);
        assert_eq!(units[0].experience, 0);
    }

    #[test]
    fn a_shot_still_in_flight_credits_its_shooter_unless_the_shooter_died() {
        let mut enemy = target_at(10.0, 60.0);
        enemy.health = 1.0;
        let mut units = [shooter(WeaponKind::Missile), enemy];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        units[0].alive = false;
        run(&mut combat, &mut units, 3.0);
        assert!(!units[1].alive, "the shot still lands");
        assert_eq!(units[0].experience, 0);
    }

    #[test]
    fn experienced_units_hit_harder() {
        let damage_by = |experience: u32| {
            let mut unit = shooter(WeaponKind::Bullet);
            unit.experience = experience;
            let mut units = [unit, target_at(4.0, 60.0)];
            let mut combat = Combat::default();
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            100.0 - units[1].health
        };
        assert_eq!(damage_by(0), WeaponKind::Bullet.damage());
        assert!(damage_by(2) > damage_by(0));
        assert!(damage_by(10) > damage_by(5));
    }

    #[test]
    fn units_with_a_medal_slowly_heal_and_others_do_not() {
        let hurt_for_ten_seconds = |experience: u32| {
            let mut unit = shooter(WeaponKind::Unarmed);
            unit.experience = experience;
            unit.health = 50.0;
            let mut units = [unit];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 10.0);
            units[0].health
        };
        assert_eq!(hurt_for_ten_seconds(0), 50.0);
        let (one_medal, three) = (hurt_for_ten_seconds(2), hurt_for_ten_seconds(10));
        assert!(one_medal > 50.0 && one_medal < 65.0, "slowly: {one_medal}");
        assert!(three > one_medal, "faster with more medals");

        let mut unit = shooter(WeaponKind::Unarmed);
        unit.experience = 2;
        unit.health = unit.max_health() - 0.01;
        let mut units = [unit];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert_eq!(units[0].health, units[0].max_health(), "up to full and no further");
    }

    #[test]
    fn a_grounded_aircraft_can_be_repaired_but_not_one_in_the_air() {
        let mut units = repair_scene(0, 2.0, UnitKind::Aircraft);
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert_eq!(units[1].health, 50.0, "in the air");

        units[1].altitude = 0.0;
        run(&mut combat, &mut units, 1.0);
        assert!(units[1].health > 50.0, "on the ground");
    }

    fn catapult_facing_east() -> Unit {
        let mut unit = Unit::for_test(0.0, 60.0, UnitKind::Vehicle);
        unit.role = Role::Catapult;
        unit.id = 1;
        unit.weapon = WeaponKind::Mortar;
        unit.heading = 0.0;
        unit
    }

    #[test]
    fn a_catapult_turns_to_face_a_target_to_its_side_or_behind_it_before_it_fires() {
        for (x, z) in [(-10.0, 60.0), (0.0, 70.0), (0.0, 50.0)] {
            let mut units = [catapult_facing_east(), target_at(x, z)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 0.3);
            assert!(combat.shells.is_empty(), "still turning towards {x},{z}");
            assert_ne!(units[0].heading, 0.0);
            for _ in 0..40 {
                combat.advance(&mut units, &TEAMS, flat(), 0.05);
                if !combat.shells.is_empty() {
                    break;
                }
            }
            assert_eq!(combat.shells.len(), 1, "now it fires at {x},{z}");
            let (sin, cos) = units[0].heading.sin_cos();
            let (dx, dz) = (x, z - 60.0);
            let length = dx.hypot(dz);
            assert!(cos * dx / length + sin * dz / length > 0.98, "it faces {x},{z}");
        }
    }

    #[test]
    fn a_catapult_facing_its_target_fires_at_once() {
        let mut units = [catapult_facing_east(), target_at(10.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.shells.len(), 1);
    }

    #[test]
    fn a_catapult_turns_towards_a_place_it_was_told_to_attack_and_stays_still_with_nothing_to_fire_at() {
        let mut units = [catapult_facing_east()];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert_eq!(units[0].heading, 0.0, "it does not look around like a tower");

        units[0].target = Some(Target::Place(-10.0, 60.0));
        run(&mut combat, &mut units, 0.3);
        assert!(combat.shells.is_empty());
        run(&mut combat, &mut units, 2.0);
        assert_eq!(combat.shells.len(), 1);
    }

    #[test]
    fn long_frames_behave_like_many_short_ones() {
        let mut units = [shooter(WeaponKind::Mortar), target_at(14.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 3.0);
        assert!(units[1].health < 100.0);
    }

    #[test]
    fn the_dead_neither_fire_nor_get_targeted() {
        let mut units = [shooter(WeaponKind::Bullet), target_at(3.0, 60.0)];
        units[0].alive = false;
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert_eq!(units[1].health, 100.0);

        let mut units = [shooter(WeaponKind::Laser), target_at(3.0, 60.0)];
        units[1].alive = false;
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert!(combat.beams.is_empty());
    }
}
