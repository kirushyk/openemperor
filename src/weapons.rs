//! The ways of killing: bullets, missiles, lasers, mortars and the superweapon.
//!
//! Everything here is plain simulation with no drawing. Units fire by themselves at the
//! nearest enemy in range, most only while standing still. Whatever can only fire straight
//! ahead first turns to face its target.

use crate::battlefield::{Terrain, next_random};
use std::f32::consts::TAU;

use crate::units::{
    BURNS_PAST, CANNON_SIDES, Point, ROCKET_SIDES, Role, SMOKES_PAST, TUBE_HEIGHT, Target, Unit,
    UnitKind, are_enemies, are_friends, distance, substeps, turn_between,
};

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

/// A Heavy Strider's shells fly faster and lower than a catapult's, and go off in a tighter blast.
const BOMBARD_SPEED: f32 = 11.0;
const BOMBARD_ARC: f32 = 0.2;
const BOMBARD_BLAST_RADIUS: f32 = 1.0;
/// How long a Heavy Strider calms down once all four cannons have fired, in seconds.
const BOMBARD_CALM_SECONDS: f32 = 7.0;
/// How long a cannon takes to come forward again after it recoils.
const RECOIL_SECONDS: f32 = 0.5;
/// How much of a tracer shell's path stays visible, in seconds of its flight.
const TRACER_SECONDS: f32 = 0.2;

/// How long a rocket launcher takes to reload once its whole salvo has gone, in seconds.
const ROCKET_RELOAD_SECONDS: f32 = 8.0;

/// Radians per second whatever must face its target turns towards it.
const AIM_TURN: f32 = 2.0;
/// Radians per second a tower turns while looking out.
const TOWER_IDLE_TURN: f32 = 0.5;
/// How close to its target it must point, in radians, before it fires.
const AIM_ALIGNMENT: f32 = 0.1;
/// Radians per second a missile tower raises or lowers its tubes.
const TUBE_PITCH_SPEED: f32 = 1.2;
/// How far above its target a missile tower points, so that its missiles clear the ground.
const TUBE_LOFT: f32 = 0.3;
/// The lowest and highest its tubes go.
const TUBE_LOWEST: f32 = -0.3;
const TUBE_HIGHEST: f32 = 1.2;

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

/// How many particles an explosion throws out, for its size.
const PARTICLES_PER_RADIUS: f32 = 14.0;
const PARTICLES_AT_LEAST: usize = 8;
const PARTICLES_AT_MOST: usize = 80;
/// More than this at once and new explosions throw out no more.
const PARTICLES_ALIVE_AT_MOST: usize = 800;
const PARTICLE_SECONDS: f32 = 0.9;
const PARTICLE_GRAVITY: f32 = 7.0;
const HEAVY_BLAST_RADIUS: f32 = 1.5;
/// What is damaged leaves this many of them free for explosions.
const WEAR_PARTICLES_AT_MOST: usize = 600;
/// Eruptions leave this many of them free for explosions.
const ERUPTION_PARTICLES_AT_MOST: usize = 500;
const ERUPTION_PARTICLES_PER_SECOND: f32 = 170.0;
const ERUPTION_SECONDS: f32 = 3.5;
/// The longest a vent lies quiet.
const VENT_QUIET_SECONDS: f32 = 22.0;
const LAVA_PARTICLE_SECONDS: f32 = 2.4;

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
    /// Flies like a missile, one after another in a salvo, each weaker than a missile.
    Rockets,
    /// A destroyer's: few and slow, and they hurt what is around their target too.
    HeavyRockets,
    /// Like a mortar, but a smaller white shell that hits harder, from four cannons in turn.
    Bombard,
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
            WeaponKind::Rockets | WeaponKind::HeavyRockets => 13.0,
            WeaponKind::Bombard => 14.0,
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

    /// Mortar shells and invisible bullets only reach the ground, where aircraft that have
    /// landed are too; the automatic gun is the one bullet weapon that can aim up. Anti-air
    /// missiles only go for what is up in the air. Everything else can aim up.
    pub fn can_target(self, kind: UnitKind, altitude: f32) -> bool {
        match self {
            WeaponKind::Unarmed | WeaponKind::Repair => false,
            WeaponKind::Mortar
            | WeaponKind::Bombard
            | WeaponKind::Bullet
            | WeaponKind::Sniper
            | WeaponKind::Pistol
            | WeaponKind::Cannon => altitude <= 0.0,
            WeaponKind::Flame | WeaponKind::Gas => altitude <= SPRAY_CEILING,
            WeaponKind::AirMissile => kind.flies() && altitude > 0.0,
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

    /// Only lasers, missiles and the automatic gun can be fired on the move. Everything else
    /// must stand still.
    fn fires_on_move(self) -> bool {
        matches!(
            self,
            WeaponKind::Laser | WeaponKind::Missile | WeaponKind::AirMissile | WeaponKind::AutoGun
        )
    }

    /// Whether it can harm anyone.
    pub fn is_armed(self) -> bool {
        !matches!(self, WeaponKind::Unarmed | WeaponKind::Repair)
    }

    /// How many shots it fires in turn, and how long it then waits, if it fires salvos.
    fn salvo(self) -> Option<(usize, f32)> {
        match self {
            WeaponKind::Rockets => Some((ROCKET_SIDES.len(), ROCKET_RELOAD_SECONDS)),
            WeaponKind::HeavyRockets => Some((2, ROCKET_RELOAD_SECONDS)),
            WeaponKind::Bombard => Some((CANNON_SIDES.len(), BOMBARD_CALM_SECONDS)),
            _ => None,
        }
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
            WeaponKind::Rockets => 0.3,
            WeaponKind::HeavyRockets => 0.6,
            WeaponKind::Bombard => 0.5,
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
            WeaponKind::Rockets => 35.0,
            WeaponKind::HeavyRockets => 130.0,
            WeaponKind::Bombard => 90.0,
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
                WeaponKind::Bullet
                | WeaponKind::Pistol
                | WeaponKind::Sniper
                | WeaponKind::AutoGun
                | WeaponKind::Gas,
                UnitKind::Vehicle | UnitKind::Tower | UnitKind::Building,
            ) => self.damage() * 0.25,
            (WeaponKind::Flame, UnitKind::Vehicle | UnitKind::Tower | UnitKind::Building) => {
                self.damage() * 0.5
            }
            _ => self.damage(),
        }
    }
}

/// Who fired something, kept with shots in flight: it may rank up or die before they land.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shooter {
    id: u32,
    owner: usize,
    power: f32,
}

impl Shooter {
    fn of(unit: &Unit) -> Self {
        Shooter {
            id: unit.id,
            owner: unit.owner,
            power: unit.attack_power(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Missile {
    pub position: Point,
    /// A unit vector.
    pub direction: Point,
    weapon: WeaponKind,
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
    elapsed: f32,
    duration: f32,
    arc: f32,
    weapon: WeaponKind,
    /// Fired by order at a place or a friend, it spares nobody.
    hurts_all: bool,
    shooter: Shooter,
}

impl Shell {
    /// Where it is along its ballistic arc.
    pub fn position(&self) -> Point {
        self.position_at(self.elapsed)
    }

    /// The points of its arc it has just passed, nearest first, if it shows them.
    pub fn tracer(&self) -> Option<[Point; 4]> {
        (self.weapon == WeaponKind::Bombard).then(|| {
            std::array::from_fn(|i| {
                let behind = TRACER_SECONDS * (i as f32 + 1.0) / 4.0;
                self.position_at((self.elapsed - behind).max(0.0))
            })
        })
    }

    fn position_at(&self, elapsed: f32) -> Point {
        let t = elapsed / self.duration;
        let along = |a: f32, b: f32| a + (b - a) * t;
        (
            along(self.start.0, self.landing.0),
            along(self.start.1, self.landing.1) + 4.0 * self.arc * t * (1.0 - t),
            along(self.start.2, self.landing.2),
        )
    }

    /// The spot under it, for its shadow.
    #[cfg(test)]
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

/// Lava going up from a vent. It is only a sight: it hurts nobody.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eruption {
    pub at: Point,
    /// From 1 when it starts down to 0.
    pub life: f32,
    /// How many particles it has yet to throw, short of a whole one.
    owed: f32,
}

impl Eruption {
    /// How hard it is going, from 0 as it starts and ends up to 1 halfway through.
    pub fn strength(&self) -> f32 {
        (self.life * (1.0 - self.life) * 4.0).max(0.0)
    }
}

#[cfg(test)]
impl Eruption {
    /// One halfway through, burning its brightest.
    pub fn at_for_test(at: Point) -> Self {
        Eruption {
            at,
            life: 0.5,
            owed: 0.0,
        }
    }
}

/// A spark or a puff of smoke thrown out by an explosion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub position: Point,
    velocity: Point,
    /// How wide it is, in world units.
    pub size: f32,
    /// Smoke drifts up and darkens; a spark glows and falls.
    pub smoke: bool,
    /// A flame glows like a spark, but rises.
    flame: bool,
    /// From 1 when it is thrown out down to 0.
    pub life: f32,
    /// How much life it loses each second.
    fade: f32,
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
    pub particles: Vec<Particle>,
    pub eruptions: Vec<Eruption>,
    /// How long until each of the map's vents goes up, from when it is found to have any.
    vent_waits: Vec<f32>,
    /// Where the next explosion's scatter comes from.
    random: u32,
    pub strikes: Vec<Strike>,
    pub poison: Vec<PoisonZone>,
}

impl Combat {
    /// A flash on the ground, and sparks and smoke thrown out from a point, more and farther
    /// the bigger it is.
    pub fn explode(&mut self, at: Point, radius: f32) {
        self.blasts.push(Blast {
            at,
            radius,
            life: 1.0,
        });
        let count = ((PARTICLES_PER_RADIUS * radius) as usize + PARTICLES_AT_LEAST)
            .min(PARTICLES_AT_MOST)
            .min(PARTICLES_ALIVE_AT_MOST.saturating_sub(self.particles.len()));
        let mut random = self.random | 1;
        let mut next = || next_random(&mut random);
        for i in 0..count {
            let smoke = i % 3 == 0;
            let angle = next() * TAU;
            let speed = (0.5 + next()) * radius.min(3.0) * if smoke { 0.6 } else { 3.0 };
            let up = if smoke {
                0.5 + next()
            } else {
                1.0 + next() * 3.0
            };
            self.particles.push(Particle {
                position: at,
                velocity: (angle.cos() * speed, up, angle.sin() * speed),
                size: (0.5 + next()) * if smoke { 0.22 } else { 0.08 },
                smoke,
                flame: false,
                life: 1.0,
                fade: 1.0 / (PARTICLE_SECONDS * (0.5 + next())),
            });
        }
        self.random = random;
    }

    /// Starts vents whose time has come, and keeps those going up throwing lava and smoke.
    fn work_vents(&mut self, terrain: &Terrain, dt: f32) {
        let vents = terrain.vents();
        if vents.is_empty() {
            return;
        }
        let mut random = self.random | 1;
        if self.vent_waits.len() != vents.len() {
            self.vent_waits = vents
                .iter()
                .map(|_| next_random(&mut random) * VENT_QUIET_SECONDS)
                .collect();
        }
        for (wait, &(x, z)) in self.vent_waits.iter_mut().zip(vents) {
            *wait -= dt;
            if *wait <= 0.0 {
                *wait = ERUPTION_SECONDS + (0.3 + next_random(&mut random)) * VENT_QUIET_SECONDS;
                self.eruptions.push(Eruption {
                    at: (x, 0.0, z),
                    life: 1.0,
                    owed: 0.0,
                });
            }
        }
        let room = ERUPTION_PARTICLES_AT_MOST.saturating_sub(self.particles.len());
        let mut thrown = Vec::new();
        for eruption in &mut self.eruptions {
            eruption.life -= dt / ERUPTION_SECONDS;
            // It gets going, then dies down.
            let strength = eruption.strength();
            eruption.owed += ERUPTION_PARTICLES_PER_SECOND * strength * dt;
            while eruption.owed >= 1.0 {
                eruption.owed -= 1.0;
                if thrown.len() >= room {
                    continue;
                }
                let mut next = || next_random(&mut random);
                let smoke = next() < 0.25;
                let angle = next() * TAU;
                let out = next() * if smoke { 0.6 } else { 2.2 } * (0.4 + strength);
                let up = if smoke {
                    1.2 + next() * 1.2
                } else {
                    (5.0 + next() * 7.0) * (0.5 + 0.5 * strength)
                };
                let spot = (
                    eruption.at.0 + next() - 0.5,
                    0.0,
                    eruption.at.2 + next() - 0.5,
                );
                thrown.push(Particle {
                    position: spot,
                    velocity: (angle.cos() * out, up, angle.sin() * out),
                    size: (0.5 + next()) * if smoke { 0.7 } else { 0.22 },
                    smoke,
                    flame: false,
                    life: 1.0,
                    fade: 1.0 / (LAVA_PARTICLE_SECONDS * (0.5 + next())),
                });
            }
        }
        self.particles.append(&mut thrown);
        self.eruptions.retain(|eruption| eruption.life > 0.0);
        self.random = random;
    }

    /// Moves sparks and smoke along, stopping them at the ground, and drops the spent ones.
    fn fly_particles(&mut self, terrain: &Terrain, dt: f32) {
        for particle in &mut self.particles {
            if !particle.smoke && !particle.flame {
                particle.velocity.1 -= PARTICLE_GRAVITY * dt;
            }
            let (x, y, z) = particle.position;
            let (dx, dy, dz) = particle.velocity;
            let (x, z) = (x + dx * dt, z + dz * dt);
            let ground = terrain.height_at(x, z);
            particle.position = (x, (y + dy * dt).max(ground), z);
            particle.life -= particle.fade * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    /// Sends the radioactive missile at a point on the ground.
    pub fn launch_strike(&mut self, x: f32, z: f32) {
        self.strikes.push(Strike { x, z, elapsed: 0.0 });
    }

    /// Advances everything by `dt` seconds. Units killed are left marked as not alive.
    pub fn advance(&mut self, units: &mut [Unit], teams: &[usize], terrain: &Terrain, dt: f32) {
        let (steps, step_seconds) = substeps(dt);
        for _ in 0..steps {
            self.step(units, teams, terrain, step_seconds);
        }
        self.work_vents(terrain, dt);
        self.show_wear(units, dt);
    }

    /// What has lost enough of its health smokes, more the worse it is, and past that burns.
    fn show_wear(&mut self, units: &[Unit], dt: f32) {
        const SMOKE_PER_SECOND: f32 = 9.0;
        const FLAMES_PER_SECOND: f32 = 26.0;
        if units.iter().all(|u| u.wear() <= SMOKES_PAST) {
            return;
        }
        let mut random = self.random | 1;
        for unit in units.iter().filter(|u| u.wear() > SMOKES_PAST) {
            let wear = unit.wear();
            // Bigger things show it in more places at once.
            let size = unit.kind.radius().max(0.4);
            let smokes = SMOKE_PER_SECOND * wear * size * dt;
            let flames = if wear > BURNS_PAST {
                FLAMES_PER_SECOND * size * dt
            } else {
                0.0
            };
            for (chance, is_flame) in [(smokes, false), (flames, true)] {
                let mut left = chance;
                while left > 0.0 {
                    let mut next = || next_random(&mut random);
                    let throws = next() < left;
                    left -= 1.0;
                    if !throws || self.particles.len() >= WEAR_PARTICLES_AT_MOST {
                        continue;
                    }
                    let angle = next() * TAU;
                    let (sin, cos) = angle.sin_cos();
                    let out = next() * size * 0.5;
                    let (rise, width, lasts) = if is_flame {
                        (1.2 + next(), 0.13, 0.55)
                    } else {
                        (0.7 + next() * 0.7, 0.42, 1.9)
                    };
                    self.particles.push(Particle {
                        position: (unit.x + cos * out, unit.pick_height(), unit.z + sin * out),
                        velocity: (cos * 0.15, rise, sin * 0.15),
                        size: (0.6 + next()) * width,
                        smoke: !is_flame,
                        flame: is_flame,
                        life: 1.0,
                        fade: 1.0 / (lasts * (0.6 + next() * 0.8)),
                    });
                }
            }
        }
        self.random = random;
    }

    fn step(&mut self, units: &mut [Unit], teams: &[usize], terrain: &Terrain, dt: f32) {
        self.fire(units, teams, terrain, dt);
        self.fly_missiles(units, teams, terrain, dt);
        self.fly_shells(units, teams, dt);
        self.fall_strikes(units, dt);
        self.poison_land(units, dt);
        heal_veterans(units, dt);
        ease_recoil(units, dt);
        fade(&mut self.beams, dt / BEAM_SECONDS, |b| &mut b.life);
        fade(&mut self.flares, dt / FLARE_SECONDS, |f| &mut f.life);
        fade(&mut self.sprays, dt / SPRAY_SECONDS, |s| &mut s.life);
        fade(&mut self.blasts, dt / BLAST_SECONDS, |b| &mut b.life);
        self.fly_particles(terrain, dt);
    }

    fn fire(&mut self, units: &mut [Unit], teams: &[usize], terrain: &Terrain, dt: f32) {
        for i in 0..units.len() {
            if !units[i].alive {
                continue;
            }
            units[i].cooldown -= dt;
            let salvo_over = |unit: &Unit| {
                unit.weapon
                    .salvo()
                    .is_some_and(|(shots, _)| unit.salvo_fired == shots)
            };
            if units[i].cooldown <= 0.0 && salvo_over(&units[i]) {
                units[i].salvo_fired = 0;
            }
            if !units[i].has_rockets()
                || !units[i].can_launch()
                || (units[i].is_moving() && !units[i].weapon.fires_on_move())
            {
                continue;
            }
            let must_aim = must_turn_to_fire(&units[i]);
            if !must_aim && units[i].cooldown > 0.0 {
                continue;
            }
            let aim = choose_aim(units, i, teams, terrain);
            let aimed = !must_aim || turn_to_aim(units, i, aim, dt);
            if units[i].cooldown > 0.0 {
                continue;
            }
            let Some(aim) = aim else {
                units[i].cooldown = RETARGET_SECONDS;
                continue;
            };
            if aimed {
                self.shoot(units, i, aim, teams);
            }
        }
    }

    /// Fires a unit's weapon at its aim and starts its reload.
    fn shoot(&mut self, units: &mut [Unit], i: usize, aim: Aim, teams: &[usize]) {
        let weapon = units[i].weapon;
        units[i].cooldown = weapon.cooldown();
        if units[i].role.rocket_load().is_some() {
            units[i].rockets_fired += 1;
        }
        let from = units[i].muzzle();
        let shooter = Shooter::of(&units[i]);
        let (to, target) = match aim {
            Aim::Unit(t) => (units[t].center(), Some(t)),
            Aim::Place(place) => (place, None),
        };
        // Only firing as ordered at a place or a friend can hurt friends.
        let forced = target.is_none_or(|t| units[i].target == Some(Target::Forced(units[t].id)));
        if let Some(size) = weapon.flare() {
            self.flares.push(Flare {
                at: from,
                size,
                life: 1.0,
            });
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
                self.beams.push(Beam {
                    from,
                    to,
                    life: 1.0,
                    repair: false,
                });
            }
            WeaponKind::Repair => {
                if let Some(t) = target {
                    let fixed = &mut units[t];
                    fixed.health = (fixed.health + REPAIR_HEALTH).min(fixed.max_health());
                    self.beams.push(Beam {
                        from,
                        to,
                        life: 1.0,
                        repair: true,
                    });
                }
            }
            WeaponKind::Unarmed => {}
            WeaponKind::Missile
            | WeaponKind::AirMissile
            | WeaponKind::Rockets
            | WeaponKind::HeavyRockets => {
                // From the ground it is launched upward, so that it visibly rises and turns
                // towards its target. From the air it has a clear shot.
                let (dx, dz) = (to.0 - from.0, to.2 - from.2);
                let flat = dx.hypot(dz).max(0.0001);
                let direction = if units[i].altitude > 0.0 {
                    direction_to(from, to)
                } else if units[i].role == Role::Tower {
                    // A tower's missiles leave the way its tubes point.
                    let (rise, out) = units[i].tube_pitch.sin_cos();
                    let (sin, cos) = units[i].heading.sin_cos();
                    (cos * out, rise, sin * out)
                } else {
                    direction_to((0.0, 0.0, 0.0), (dx / flat, 0.8, dz / flat))
                };
                self.missiles.push(Missile {
                    position: from,
                    direction,
                    weapon,
                    target_id: target.map_or(NO_UNIT, |t| units[t].id),
                    place: target.is_none().then_some(to),
                    shooter,
                    fuel: MISSILE_FUEL_SECONDS,
                });
                units[i].second_tube = !units[i].second_tube;
            }
            WeaponKind::Mortar | WeaponKind::Bombard => {
                let flat = (to.0 - from.0).hypot(to.2 - from.2);
                let (speed, arc) = if weapon == WeaponKind::Bombard {
                    (BOMBARD_SPEED, BOMBARD_ARC)
                } else {
                    (SHELL_SPEED, SHELL_ARC)
                };
                self.shells.push(Shell {
                    start: from,
                    landing: to,
                    elapsed: 0.0,
                    duration: (flat / speed).max(0.2),
                    arc: flat * arc,
                    weapon,
                    hurts_all: forced,
                    shooter,
                });
            }
        }
        if let Some((shots, wait)) = weapon.salvo() {
            let fired = units[i].salvo_fired;
            if let Some(recoil) = units[i]
                .recoil
                .get_mut(fired)
                .filter(|_| weapon == WeaponKind::Bombard)
            {
                *recoil = 1.0;
            }
            units[i].salvo_fired = fired + 1;
            if fired + 1 == shots {
                units[i].cooldown = wait;
            }
        }
    }

    /// Everyone low enough in the cone in front of a unit gets hurt, if they are enemies of
    /// whoever sprayed it or it was forced to fire there.
    fn spray(&mut self, units: &mut [Unit], sprayer: usize, teams: &[usize], forced: bool) {
        let unit = units[sprayer];
        let Some(half_angle) = unit.weapon.spray_angle() else {
            return;
        };
        let reach = unit.weapon.range();
        for index in 0..units.len() {
            let other = units[index];
            let harmed = forced || are_enemies(teams, unit.owner, other.owner);
            if index == sprayer
                || !other.alive
                || !harmed
                || !unit.weapon.can_target(other.kind, other.altitude)
            {
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
            let target = units
                .iter()
                .position(|u| u.alive && u.id == missile.target_id);
            let heading_for = missile.place.or_else(|| target.map(|t| units[t].center()));
            if let Some(goal) = heading_for {
                missile.direction = turn_towards(
                    missile.direction,
                    direction_to(missile.position, goal),
                    MISSILE_TURN * dt,
                );
            }
            let (x, y, z) = missile.position;
            let (dx, dy, dz) = missile.direction;
            let step = MISSILE_SPEED * dt;
            missile.position = (x + dx * step, y + dy * step, z + dz * step);
            missile.fuel -= dt;

            let reached = target.filter(|&t| {
                distance(missile.position, units[t].center())
                    <= units[t].kind.radius() + MISSILE_FUSE
            });
            let (x, y, z) = missile.position;
            let reached_place = missile.place.is_some_and(|place| {
                distance(missile.position, place) <= MISSILE_FUSE * 2.0
                    || y <= terrain.height_at(x, z)
            });
            if reached_place {
                // Whoever stands at the place it was sent to is hurt, whoever they are.
                for index in 0..units.len() {
                    let reach = MISSILE_BLAST_RADIUS + units[index].kind.radius();
                    if units[index].alive
                        && distance(missile.position, units[index].center()) <= reach
                    {
                        hit(units, index, missile.weapon, missile.shooter, teams);
                    }
                }
                self.explode(missile.position, MISSILE_BLAST_RADIUS);
            } else if let Some(t) = reached {
                hit(units, t, missile.weapon, missile.shooter, teams);
                if missile.weapon == WeaponKind::HeavyRockets {
                    // It hurts the shooter's enemies around its target as well, by half.
                    let damage = missile.weapon.damage() * missile.shooter.power * 0.5;
                    for index in (0..units.len()).filter(|&index| index != t) {
                        let reach = HEAVY_BLAST_RADIUS + units[index].kind.radius();
                        if units[index].alive
                            && are_enemies(teams, missile.shooter.owner, units[index].owner)
                            && distance(missile.position, units[index].center()) <= reach
                        {
                            harm(units, index, damage, missile.shooter, teams);
                        }
                    }
                    self.explode(missile.position, HEAVY_BLAST_RADIUS);
                } else {
                    self.explode(missile.position, MISSILE_BLAST_RADIUS);
                }
            } else if y <= terrain.height_at(x, z) || missile.fuel <= 0.0 {
                // A miss: it hits the ground or burns out, harming nobody.
                self.explode(missile.position, MISSILE_BLAST_RADIUS * 0.6);
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
            let blast_radius = if shell.weapon == WeaponKind::Bombard {
                BOMBARD_BLAST_RADIUS
            } else {
                SHELL_BLAST_RADIUS
            };
            // Only enemies of whoever fired are hurt, so that units can fire freely.
            for index in 0..units.len() {
                let unit = units[index];
                if unit.alive
                    && (shell.hurts_all || are_enemies(teams, shell.shooter.owner, unit.owner))
                {
                    let away = distance(shell.landing, unit.center());
                    let reach = blast_radius + unit.kind.radius();
                    if away < reach {
                        let damage = shell.weapon.damage()
                            * shell.shooter.power
                            * (1.0 - away / reach * 0.5);
                        harm(units, index, damage, shell.shooter, teams);
                    }
                }
            }
            self.explode(shell.landing, blast_radius);
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
            self.explode((strike.x, 0.0, strike.z), STRIKE_RADIUS);
            self.poison.push(PoisonZone {
                x: strike.x,
                z: strike.z,
                life: POISON_SECONDS,
            });
        }
        self.strikes = falling;
    }

    fn poison_land(&mut self, units: &mut [Unit], dt: f32) {
        for zone in &mut self.poison {
            zone.life -= dt;
            for unit in units.iter_mut().filter(|u| u.alive && u.altitude <= 0.0) {
                if (unit.x - zone.x).hypot(unit.z - zone.z) < STRIKE_RADIUS {
                    hurt(unit, POISON_DAMAGE * dt);
                }
            }
        }
        self.poison.retain(|z| z.life > 0.0);
    }
}

/// Counts down each item's life and removes those it runs out for.
fn fade<T>(items: &mut Vec<T>, by: f32, life: impl Fn(&mut T) -> &mut f32) {
    items.retain_mut(|item| {
        let life = life(item);
        *life -= by;
        *life > 0.0
    });
}

/// A shot striking at once: its target, or whoever stands at the place it was fired at.
fn strike(
    units: &mut [Unit],
    firing: usize,
    target: Option<usize>,
    at: Point,
    weapon: WeaponKind,
    teams: &[usize],
) {
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
    answer_attacker(units, victim, shooter.id, teams);
}

/// Whoever is shot at by an enemy goes after it, if it can hurt it and has nothing else to do.
/// The player's units on their way somewhere stay on their way, as they were told to; an
/// opponent's are only wandering. What cannot move has only its range to go by.
fn answer_attacker(units: &mut [Unit], victim: usize, attacker_id: u32, teams: &[usize]) {
    let unit = units[victim];
    let is_free = unit.target.is_none()
        && !unit.deployed
        && !unit.kind.is_building()
        && (!unit.is_moving() || unit.owner != 0);
    if !is_free {
        return;
    }
    let can_answer = units
        .iter()
        .find(|u| u.id == attacker_id)
        .is_some_and(|attacker| unit.can_attack(attacker, teams));
    if can_answer {
        units[victim].target = Some(Target::Unit(attacker_id));
    }
}

/// Cannons that have recoiled come forward again.
fn ease_recoil(units: &mut [Unit], dt: f32) {
    for recoil in units.iter_mut().flat_map(|u| &mut u.recoil) {
        *recoil = (*recoil - dt / RECOIL_SECONDS).max(0.0);
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

/// Towers, sprayers, tanks, catapults and whatever carries rockets can only fire the way they
/// face.
fn must_turn_to_fire(unit: &Unit) -> bool {
    unit.kind == UnitKind::Tower
        || unit.weapon.spray_angle().is_some()
        || (unit.weapon != WeaponKind::Repair
            && matches!(
                unit.role,
                Role::Tank
                    | Role::Catapult
                    | Role::RocketLauncher
                    | Role::Ornithopter
                    | Role::Destroyer
                    | Role::HeavyStrider
            ))
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
fn turn_to_aim(units: &mut [Unit], index: usize, aim: Option<Aim>, dt: f32) -> bool {
    let goal = match aim {
        Some(Aim::Unit(t)) => units[t].center(),
        Some(Aim::Place(place)) => place,
        None => {
            if units[index].kind.is_building() {
                units[index].heading += TOWER_IDLE_TURN * dt;
                pitch_tubes(&mut units[index], 0.0, dt);
            }
            return false;
        }
    };
    let (dx, dz) = (goal.0 - units[index].x, goal.2 - units[index].z);
    let turn = turn_between(units[index].heading, dz.atan2(dx));
    let most = AIM_TURN * dt;
    let turned = turn.clamp(-most, most);
    units[index].heading += turned;
    let unit = &mut units[index];
    let pitched = unit.role != Role::Tower || {
        let rise = goal.1 - unit.y - TUBE_HEIGHT;
        let wanted = (rise.atan2(dx.hypot(dz)) + TUBE_LOFT).clamp(TUBE_LOWEST, TUBE_HIGHEST);
        pitch_tubes(unit, wanted, dt)
    };
    pitched && (turn - turned).abs() <= AIM_ALIGNMENT
}

/// Raises or lowers a missile tower's tubes towards a pitch. Returns whether they are there.
fn pitch_tubes(unit: &mut Unit, wanted: f32, dt: f32) -> bool {
    if unit.role != Role::Tower {
        return true;
    }
    let most = TUBE_PITCH_SPEED * dt;
    unit.tube_pitch += (wanted - unit.tube_pitch).clamp(-most, most);
    (wanted - unit.tube_pitch).abs() <= AIM_ALIGNMENT
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
            let fit = if forced {
                unit.can_fire_on(&units[t])
            } else {
                unit.can_attack(&units[t], teams)
            };
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
        Some(Target::Unit(id)) => units
            .iter()
            .position(|u| u.id == id)
            .filter(|&i| needs_it(i)),
        _ => None,
    };
    sent_to.or_else(|| {
        (0..units.len()).filter(|&i| needs_it(i)).min_by(|&a, &b| {
            distance(from, units[a].center()).total_cmp(&distance(from, units[b].center()))
        })
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
        if unit.alive
            && are_enemies(teams, owner, unit.owner)
            && weapon.can_target(unit.kind, unit.altitude)
            && unit.kind != UnitKind::Building
        {
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
    (
        (to.0 - from.0) / length,
        (to.1 - from.1) / length,
        (to.2 - from.2) / length,
    )
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
    let mut side = (
        to.0 - from.0 * cos,
        to.1 - from.1 * cos,
        to.2 - from.2 * cos,
    );
    if distance((0.0, 0.0, 0.0), side) < 0.0001 {
        side = if from.0.abs() < 0.9 {
            (1.0 - from.0 * from.0, -from.0 * from.1, -from.0 * from.2)
        } else {
            (-from.1 * from.0, 1.0 - from.1 * from.1, -from.1 * from.2)
        };
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

    /// A tank of the opponent's being shot by a rocket launcher, the one out of the other's reach.
    fn tank_under_fire(owner: usize) -> (Unit, Unit) {
        let mut launcher = shooter(WeaponKind::Rockets);
        launcher.owner = 1 - owner;
        let mut tank = target_at(12.5, 60.0);
        tank.owner = owner;
        tank.weapon = WeaponKind::Cannon;
        (launcher, tank)
    }

    #[test]
    fn a_tank_hit_from_beyond_its_reach_goes_after_whoever_did_it_and_reaches_it() {
        let (launcher, tank) = tank_under_fire(1);
        let mut units = [launcher, tank];
        let attacker = Shooter::of(&units[0]);
        assert!(units[0].weapon.range() > 12.5 && units[1].weapon.range() < 12.5);
        hit(&mut units, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(units[1].target, Some(Target::Unit(units[0].id)));
        units[0].weapon = WeaponKind::Unarmed;
        for _ in 0..600 {
            crate::units::advance(&mut units, flat(), &TEAMS, 0.05);
        }
        let away = distance(units[0].center(), units[1].center());
        assert!(
            away <= WeaponKind::Cannon.range(),
            "came within reach: {away}"
        );
    }

    #[test]
    fn the_players_unit_on_an_order_carries_on_when_shot_but_an_idle_one_answers() {
        let (launcher, tank) = tank_under_fire(0);
        let mut units = [launcher, tank];
        units[1].destination = (12.5, 90.0);
        let attacker = Shooter::of(&units[0]);
        hit(&mut units, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(units[1].target, None, "it was told to go there");

        units[1].destination = (units[1].x, units[1].z);
        hit(&mut units, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(units[1].target, Some(Target::Unit(units[0].id)));
    }

    #[test]
    fn an_opponent_on_its_way_somewhere_turns_on_whoever_shoots_it() {
        let (launcher, tank) = tank_under_fire(1);
        let mut units = [launcher, tank];
        units[1].destination = (12.5, 90.0);
        let attacker = Shooter::of(&units[0]);
        hit(&mut units, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(units[1].target, Some(Target::Unit(units[0].id)));
    }

    #[test]
    fn being_shot_does_not_change_a_target_nor_stir_those_that_cannot_answer_or_move() {
        let (launcher, tank) = tank_under_fire(1);
        let attacker = Shooter::of(&launcher);

        let mut busy = [launcher, tank];
        busy[1].target = Some(Target::Unit(99));
        hit(&mut busy, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(
            busy[1].target,
            Some(Target::Unit(99)),
            "it keeps its target"
        );

        // Cannons do not reach what is up in the air.
        let mut high = [launcher, tank];
        high[0].altitude = 2.0;
        hit(&mut high, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(high[1].target, None);

        let mut unarmed = [launcher, tank];
        unarmed[1].weapon = WeaponKind::Unarmed;
        hit(&mut unarmed, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(unarmed[1].target, None);

        let mut tower = [launcher, tank];
        tower[1].kind = UnitKind::Tower;
        hit(&mut tower, 1, WeaponKind::Rockets, attacker, &TEAMS);
        assert_eq!(tower[1].target, None);

        // A friend firing by order is not an enemy to go after.
        let mut friend = [launcher, tank];
        friend[0].owner = 1;
        let by_friend = Shooter::of(&friend[0]);
        hit(&mut friend, 1, WeaponKind::Rockets, by_friend, &TEAMS);
        assert_eq!(friend[1].target, None);
    }

    #[test]
    fn a_machine_spends_its_rockets_and_stops_firing_when_they_are_gone() {
        let mut machine = Unit::for_test(0.0, 60.0, UnitKind::Aircraft);
        machine.role = Role::Destroyer;
        machine.weapon = WeaponKind::HeavyRockets;
        machine.id = 1;
        machine.heading = 0.0;
        let mut units = [machine, target_at(5.0, 60.0)];
        units[1].health = 100_000.0;
        let mut combat = Combat::default();
        let load = Role::Destroyer.rocket_load().unwrap();
        for _ in 0..1200 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert_eq!(units[0].rockets_fired, load, "all gone and no more");
        assert_eq!(units[0].rockets_left(), Some(0));
        assert!(!units[0].has_rockets());
    }

    #[test]
    fn heavy_rockets_hit_far_harder_and_hurt_enemies_around_the_target_but_not_friends() {
        assert!(WeaponKind::HeavyRockets.damage() > WeaponKind::Rockets.damage() * 3.0);
        let mut machine = Unit::for_test(0.0, 60.0, UnitKind::Aircraft);
        machine.role = Role::Destroyer;
        machine.weapon = WeaponKind::HeavyRockets;
        machine.id = 1;
        machine.heading = 0.0;
        let mut target = target_at(4.0, 60.0);
        target.health = 1000.0;
        let mut beside = target_at(4.0, 60.6);
        beside.id = 3;
        beside.health = 1000.0;
        let mut friend = Unit::for_test(4.0, 59.4, UnitKind::Infantry);
        friend.id = 4;
        friend.cooldown = 1000.0;
        let friend_health = friend.health;
        let mut units = [machine, target, beside, friend];
        let mut combat = Combat::default();
        for _ in 0..60 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            units[0].rockets_fired = units[0].rockets_fired.min(1);
            units[0].cooldown = units[0]
                .cooldown
                .max(1000.0 * f32::from(u8::from(!combat.missiles.is_empty())));
        }
        assert!(units[1].health < 1000.0, "the target");
        assert!(units[2].health < 1000.0, "beside it");
        assert!(units[1].health < units[2].health, "the target worst");
        assert_eq!(units[3].health, friend_health, "not a friend");
    }

    #[test]
    fn what_is_damaged_smokes_past_a_point_and_burns_past_another_and_infantry_never() {
        let run_on = |kind: UnitKind, share: f32| {
            let mut unit = Unit::for_test(0.0, 60.0, kind);
            unit.weapon = WeaponKind::Unarmed;
            unit.health = unit.max_health() * share;
            let mut combat = Combat::default();
            let mut units = [unit];
            let (mut smoke, mut flames) = (false, false);
            for _ in 0..200 {
                combat.advance(&mut units, &TEAMS, flat(), 0.05);
                smoke |= combat.particles.iter().any(|p| p.smoke);
                flames |= combat.particles.iter().any(|p| p.flame);
            }
            (smoke, flames)
        };
        assert_eq!(run_on(UnitKind::Vehicle, 1.0), (false, false));
        assert_eq!(run_on(UnitKind::Vehicle, 0.5), (true, false));
        assert_eq!(run_on(UnitKind::Vehicle, 0.2), (true, true));
        assert_eq!(run_on(UnitKind::Building, 0.2), (true, true));
        assert_eq!(run_on(UnitKind::Infantry, 0.1), (false, false));
    }

    #[test]
    fn flames_rise_and_keep_room_for_explosions() {
        let mut units: Vec<Unit> = (0..60)
            .map(|i| {
                let mut unit = Unit::for_test(i as f32, 60.0, UnitKind::Building);
                unit.weapon = WeaponKind::Unarmed;
                unit.health = 1.0;
                unit
            })
            .collect();
        let mut combat = Combat::default();
        for _ in 0..200 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            assert!(combat.particles.len() <= WEAR_PARTICLES_AT_MOST);
        }
        assert!(
            combat
                .particles
                .iter()
                .any(|p| p.flame && p.velocity.1 > 0.0)
        );
        let before = combat.particles.len();
        combat.explode((0.0, 0.0, 60.0), 1.0);
        assert!(combat.particles.len() > before);
    }

    fn lava_map() -> &'static Terrain {
        let map = (0..crate::game::MAPS.len())
            .find(|&map| !crate::battlefield::terrain(map).vents().is_empty())
            .expect("a map with lava");
        crate::battlefield::terrain(map)
    }

    #[test]
    fn vents_erupt_now_and_then_with_particles_going_up_and_hurting_nobody() {
        let terrain = lava_map();
        let mut combat = Combat::default();
        let (vent_x, vent_z) = terrain.vents()[0];
        let mut units = [target_at(vent_x, vent_z)];
        units[0].kind = UnitKind::Aircraft;
        let health = units[0].health;
        let mut most = 0;
        let mut went_up = false;
        for _ in 0..(VENT_QUIET_SECONDS * 20.0) as usize {
            combat.advance(&mut units, &TEAMS, terrain, 0.05);
            most = most.max(combat.eruptions.len());
            went_up |= combat.particles.iter().any(|p| p.position.1 > 1.0);
            assert!(combat.particles.len() <= ERUPTION_PARTICLES_AT_MOST + 1);
        }
        assert!(most > 0, "some vent went up");
        assert!(went_up, "its lava flies");
        assert_eq!(units[0].health, health);
    }

    #[test]
    fn an_eruption_dies_down_and_explosions_still_find_room() {
        let terrain = lava_map();
        let mut combat = Combat::default();
        combat.eruptions.push(Eruption {
            at: (0.0, 0.0, 40.0),
            life: 1.0,
            owed: 0.0,
        });
        for _ in 0..(ERUPTION_SECONDS * 50.0) as usize + 1 {
            combat.work_vents(terrain, 0.02);
        }
        assert!(
            combat.eruptions.iter().all(|e| e.at != (0.0, 0.0, 40.0)),
            "it ended"
        );
        combat.particles.clear();
        combat.vent_waits.iter_mut().for_each(|wait| *wait = 0.0);
        for _ in 0..400 {
            combat.work_vents(terrain, 0.05);
            combat.vent_waits.iter_mut().for_each(|wait| *wait = 0.0);
        }
        assert!(combat.particles.len() <= ERUPTION_PARTICLES_AT_MOST);
        let before = combat.particles.len();
        combat.explode((0.0, 0.0, 40.0), 1.0);
        assert!(combat.particles.len() > before);
    }

    #[test]
    fn a_map_with_no_vents_has_no_eruptions() {
        let mut combat = Combat::default();
        for _ in 0..1000 {
            combat.advance(&mut [], &TEAMS, flat(), 0.05);
        }
        assert!(combat.eruptions.is_empty() && combat.particles.is_empty());
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
        assert_eq!(
            combat.flares.len(),
            1,
            "only a flare to see, where it was fired"
        );
        assert_eq!(combat.flares[0].at, units[0].muzzle());
        assert!(combat.beams.is_empty() && combat.missiles.is_empty() && combat.blasts.is_empty());
        run(&mut combat, &mut units, 0.2);
        assert!(combat.flares.is_empty(), "and it fades");
    }

    #[test]
    fn nothing_fires_beyond_its_range_or_at_friends() {
        for weapon in [
            WeaponKind::Bullet,
            WeaponKind::Missile,
            WeaponKind::Laser,
            WeaponKind::Mortar,
        ] {
            let mut units = [shooter(weapon), target_at(weapon.range() + 1.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 2.0);
            assert_eq!(units[1].health, 100.0, "{weapon:?} out of range");
            assert!(
                combat.missiles.is_empty() && combat.shells.is_empty(),
                "{weapon:?} held fire"
            );

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
        assert_eq!(
            WeaponKind::Mortar.damage_to(UnitKind::Vehicle),
            WeaponKind::Mortar.damage()
        );
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
        let angle_from_start =
            |turned: Point| (turned.0 * from.0 + turned.1 * from.1 + turned.2 * from.2).acos();
        let turned = turn_towards(from, to, 0.1);
        assert!(
            (angle_from_start(turned) - 0.1).abs() < 0.001,
            "the full allowed turn"
        );
        assert_eq!(turn_towards(from, from, 0.1), from);

        // The same turning rate holds however far off the target is, even almost behind it.
        let half_turn = std::f32::consts::FRAC_1_SQRT_2;
        for away in [
            (-1.0, 0.0, 0.0),
            (-0.9999, 0.0141, 0.0),
            (-half_turn, half_turn, 0.0),
        ] {
            let turned = turn_towards(from, away, 0.1);
            assert!(
                (angle_from_start(turned) - 0.1).abs() < 0.001,
                "{away:?} gave {turned:?}"
            );
            let length = distance((0.0, 0.0, 0.0), turned);
            assert!(
                (length - 1.0).abs() < 0.001,
                "stays a unit vector: {length}"
            );
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
            weapon: WeaponKind::Missile,
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
        let mut units = [
            shooter(WeaponKind::Missile),
            target_at(9.0, 60.0),
            target_at(9.0, 62.0),
        ];
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
        let mut units = [
            shooter(WeaponKind::Mortar),
            target_at(14.0, 60.0),
            target_at(14.5, 60.5),
            target_at(14.0, 66.0),
        ];
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
        assert!(
            middle.1 > start_y && middle.1 > landing_y,
            "it flies in an arc: {middle:?}"
        );

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
        assert_eq!(
            units[1].health,
            UnitKind::Vehicle.max_health(),
            "still falling"
        );
        assert!(combat.strikes[0].height() < STRIKE_HEIGHT);

        run(&mut combat, &mut units, 0.6);
        assert!(combat.strikes.is_empty());
        assert!(!units[0].alive, "the infantry beside it dies");
        assert!(
            units[1].health < UnitKind::Vehicle.max_health(),
            "its own side is not spared"
        );
        assert_eq!(units[3].health, 100.0, "beyond the radius");
        assert_eq!(combat.poison.len(), 1);
        assert!((combat.poison[0].life - POISON_SECONDS).abs() < 0.2);
    }

    #[test]
    fn the_land_stays_poisoned_for_a_minute_and_hurts_who_stands_there() {
        let mut combat = Combat::default();
        combat.poison.push(PoisonZone {
            x: 0.0,
            z: 60.0,
            life: POISON_SECONDS,
        });
        let mut inside = Unit::for_test(3.0, 60.0, UnitKind::Vehicle);
        inside.id = 1;
        inside.cooldown = 1000.0;
        let mut outside = Unit::for_test(10.0, 60.0, UnitKind::Infantry);
        outside.id = 2;
        outside.cooldown = 1000.0;
        let mut units = [inside, outside];

        run(&mut combat, &mut units, 10.0);
        let hurt_so_far = UnitKind::Vehicle.max_health() - units[0].health;
        assert!(
            (hurt_so_far - POISON_DAMAGE * 10.0).abs() < 1.0,
            "{hurt_so_far}"
        );
        assert_eq!(units[1].health, 100.0);

        run(&mut combat, &mut units, 49.0);
        assert_eq!(
            combat.poison.len(),
            1,
            "still there just before the minute is up"
        );
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
    fn mortars_cannot_aim_at_aircraft_but_lasers_and_missiles_can() {
        let mut units = [shooter(WeaponKind::Mortar), plane_at(5.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert!(combat.shells.is_empty(), "held fire");

        for weapon in [WeaponKind::Laser, WeaponKind::Missile] {
            let mut units = [shooter(weapon), plane_at(5.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 3.0);
            assert!(
                units[1].health < UnitKind::Aircraft.max_health(),
                "{weapon:?}"
            );
        }
    }

    #[test]
    fn a_mortar_prefers_what_it_can_reach_over_a_nearer_aircraft() {
        let mut units = [
            shooter(WeaponKind::Mortar),
            plane_at(3.0, 60.0),
            target_at(14.0, 60.0),
        ];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.shells.len(), 1);
        assert_eq!(combat.shells[0].landing(), units[2].center());
    }

    #[test]
    fn a_shell_bursting_on_the_ground_does_not_reach_aircraft_overhead() {
        let mut units = [
            shooter(WeaponKind::Mortar),
            target_at(14.0, 60.0),
            plane_at(14.0, 60.0),
        ];
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
        combat.poison.push(PoisonZone {
            x: 0.0,
            z: 60.0,
            life: POISON_SECONDS,
        });
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
        assert!(
            units[0].heading.cos() < -0.9,
            "facing west: {}",
            units[0].heading
        );
    }

    #[test]
    fn a_tower_keeps_turning_slowly_while_nobody_is_in_range() {
        let mut units = [tower(), target_at(40.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 2.0);
        assert!(combat.missiles.is_empty());
        assert!(
            (units[0].heading - TOWER_IDLE_TURN * 2.0).abs() < 0.01,
            "{}",
            units[0].heading
        );
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
        assert!(
            distance(launched_from[0], launched_from[1]) > 0.1,
            "from the other tube"
        );
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
        assert!(
            units[1].alive && units[1].health < UnitKind::Tower.max_health(),
            "it takes a while"
        );
        run(&mut combat, &mut units, 40.0);
        assert!(!units[1].alive, "but it falls");
    }

    #[test]
    fn every_gun_that_strikes_at_once_shows_a_flare_and_the_cannons_is_largest() {
        for weapon in [
            WeaponKind::Bullet,
            WeaponKind::Sniper,
            WeaponKind::Pistol,
            WeaponKind::Cannon,
        ] {
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
        assert!(
            combat.beams.iter().all(|b| b.repair) && !combat.beams.is_empty()
                || units[1].health > 50.0
        );
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
        assert!(
            units[1].health < UnitKind::Aircraft.max_health(),
            "and it hit"
        );
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
        let mut units = [
            sprayer(WeaponKind::Flame),
            target_at(3.0, 60.0),
            in_cone,
            behind,
            far,
            friend,
        ];
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
            assert_eq!(
                units[1].health,
                UnitKind::Aircraft.max_health(),
                "{weapon:?}"
            );
            assert!(combat.sprays.is_empty(), "{weapon:?} held fire");

            // Sprayed at the ground below one, it passes under.
            let mut units = [sprayer(weapon), target_at(3.0, 60.0), plane_at(3.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 1.0);
            assert!(units[1].health < 100.0);
            assert_eq!(
                units[2].health,
                UnitKind::Aircraft.max_health(),
                "{weapon:?}"
            );
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
            assert!(
                units[1].health < UnitKind::Aircraft.max_health(),
                "{weapon:?} reaches low flyers"
            );

            let mut high = target_at(2.0, 60.0);
            high.altitude = SPRAY_CEILING + 0.5;
            let mut units = [sprayer(weapon), high];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 0.5);
            assert_eq!(
                units[1].health, 100.0,
                "{weapon:?} misses whatever is above it"
            );
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
        assert!(
            units[0].heading.sin() > 0.9,
            "it faces south: {}",
            units[0].heading
        );
        let (nozzle, center) = (combat.sprays[0].from, units[0].heading);
        assert!(
            (nozzle.0 - center.cos() * 0.7).abs() < 0.01,
            "straight ahead of it"
        );
    }

    #[test]
    fn firing_at_a_place_hits_whoever_stands_there_friend_or_foe_and_nobody_else() {
        for weapon in [
            WeaponKind::Bullet,
            WeaponKind::Sniper,
            WeaponKind::Laser,
            WeaponKind::Cannon,
        ] {
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
        assert!(
            !combat.beams.is_empty() || units[0].cooldown > 0.0,
            "still firing"
        );
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
        assert!(
            units[1].health < 100.0,
            "out of range of it, the nearest enemy will do"
        );
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
    fn only_lasers_missiles_and_the_automatic_gun_can_be_fired_on_the_move() {
        for weapon in [WeaponKind::Laser, WeaponKind::Missile] {
            let mut units = [moving(shooter(weapon)), target_at(4.0, 60.0)];
            let mut combat = Combat::default();
            run(&mut combat, &mut units, 1.0);
            assert!(
                !combat.beams.is_empty() || !combat.missiles.is_empty() || units[1].health < 100.0,
                "{weapon:?}"
            );
        }
        for weapon in [
            WeaponKind::Bullet,
            WeaponKind::Sniper,
            WeaponKind::Pistol,
            WeaponKind::Cannon,
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
            assert!(
                combat != Combat::default() || units[1].health < 100.0,
                "{weapon:?} fires once stopped"
            );
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
        for (kind, health) in [
            (UnitKind::Infantry, 12.0),
            (UnitKind::Vehicle, 3.0),
            (UnitKind::Tower, 3.0),
        ] {
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
        assert_eq!(
            units[0].health,
            units[0].max_health(),
            "up to full and no further"
        );
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

    #[test]
    fn what_has_landed_is_on_the_ground_for_every_weapon_and_rockets_reach_both() {
        let landed = (UnitKind::Aircraft, 0.0);
        let flying = (UnitKind::Aircraft, 4.0);
        let can = |weapon: WeaponKind, (kind, altitude)| weapon.can_target(kind, altitude);
        assert!(can(WeaponKind::Mortar, landed) && !can(WeaponKind::Mortar, flying));
        assert!(can(WeaponKind::Flame, landed) && !can(WeaponKind::Flame, flying));
        assert!(!can(WeaponKind::AirMissile, landed) && can(WeaponKind::AirMissile, flying));
        assert!(can(WeaponKind::Rockets, flying));
        assert!(WeaponKind::Rockets.can_target(UnitKind::Vehicle, 0.0));
    }

    #[test]
    fn invisible_bullets_cannot_reach_the_air_but_the_automatic_gun_lasers_and_missiles_can() {
        for weapon in [
            WeaponKind::Bullet,
            WeaponKind::Sniper,
            WeaponKind::Pistol,
            WeaponKind::Cannon,
        ] {
            assert!(!weapon.can_target(UnitKind::Aircraft, 4.0), "{weapon:?}");
            assert!(
                weapon.can_target(UnitKind::Aircraft, 0.0),
                "{weapon:?} landed"
            );
            assert!(weapon.can_target(UnitKind::Infantry, 0.0), "{weapon:?}");
        }
        for weapon in [WeaponKind::AutoGun, WeaponKind::Laser, WeaponKind::Missile] {
            assert!(weapon.can_target(UnitKind::Aircraft, 4.0), "{weapon:?}");
        }
    }

    #[test]
    fn an_ornithopter_launches_nothing_from_the_ground_only_from_up_in_the_air() {
        let mut ornithopter = Unit::for_test(0.0, 60.0, UnitKind::Aircraft);
        ornithopter.role = Role::Ornithopter;
        ornithopter.weapon = WeaponKind::Rockets;
        ornithopter.id = 1;
        ornithopter.heading = 0.0;
        let up = ornithopter.altitude;
        ornithopter.altitude = 0.0;
        let mut units = [ornithopter, target_at(5.0, 60.0)];
        let mut combat = Combat::default();
        for _ in 0..40 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(combat.missiles.is_empty(), "sitting");
        units[0].altitude = up / 2.0;
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(combat.missiles.is_empty(), "on the way up");
        units[0].altitude = up;
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.missiles.len(), 1);
    }

    fn missile_tower_facing_east() -> Unit {
        let mut tower = Unit::for_test(0.0, 60.0, UnitKind::Tower);
        tower.id = 1;
        tower.weapon = WeaponKind::Missile;
        tower.heading = 0.0;
        tower
    }

    #[test]
    fn a_missile_tower_raises_its_tubes_higher_for_aircraft_before_firing_along_them() {
        let mut units = [missile_tower_facing_east(), plane_at(4.0, 60.0)];
        units[1].cooldown = 1000.0;
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.02);
        assert!(
            combat.missiles.is_empty(),
            "the tubes still have to come up"
        );
        let mut last = units[0].tube_pitch;
        assert!(last > 0.0);
        while combat.missiles.is_empty() {
            combat.advance(&mut units, &TEAMS, flat(), 0.02);
            assert!(units[0].tube_pitch >= last, "only up");
            last = units[0].tube_pitch;
            assert!(last < 2.0, "it should have fired");
        }
        let (dx, dy, _) = combat.missiles[0].direction;
        assert!(dy > 0.4 && dx > 0.0, "up along its tube: {dy} {dx}");
        assert!(units[0].muzzle().1 > TUBE_HEIGHT, "the tube's end rose too");
    }

    #[test]
    fn a_missile_tower_lowers_its_tubes_for_something_on_the_ground_right_below_it() {
        let mut units = [missile_tower_facing_east(), target_at(2.0, 60.0)];
        units[1].cooldown = 1000.0;
        let mut combat = Combat::default();
        let mut lowest = 0.0_f32;
        for _ in 0..100 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
            lowest = lowest.min(units[0].tube_pitch);
        }
        assert!(lowest < 0.0, "{lowest}");
        assert!(units[1].health < 100.0, "and it hits");
    }

    #[test]
    fn a_missile_tower_with_nothing_to_fire_at_levels_its_tubes_and_a_flame_tower_has_none() {
        let mut units = [missile_tower_facing_east()];
        units[0].tube_pitch = 0.8;
        let mut combat = Combat::default();
        for _ in 0..40 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert_eq!(units[0].tube_pitch, 0.0);

        let mut flame = missile_tower_facing_east();
        flame.role = Role::FlameTower;
        flame.weapon = WeaponKind::Flame;
        let mut units = [flame, target_at(2.0, 60.0)];
        for _ in 0..20 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert_eq!(units[0].tube_pitch, 0.0);
    }

    fn heavy_strider_facing_east() -> Unit {
        let mut unit = Unit::for_test(0.0, 60.0, UnitKind::Vehicle);
        unit.role = Role::HeavyStrider;
        unit.id = 1;
        unit.weapon = WeaponKind::Bombard;
        unit.heading = 0.0;
        unit
    }

    #[test]
    fn a_heavy_strider_fires_all_four_cannons_one_by_one_with_recoil_then_calms_down() {
        let mut target = target_at(10.0, 60.0);
        target.health = 100_000.0;
        let mut units = [heavy_strider_facing_east(), target];
        let mut combat = Combat::default();
        let (mut shots, mut sides, mut elapsed) = (Vec::new(), Vec::new(), 0.0);
        let mut seen = 0;
        while shots.len() < 5 && elapsed < 30.0 {
            let before = units[0].salvo_fired;
            let side = units[0].muzzle().2 - 60.0;
            combat.advance(&mut units, &TEAMS, flat(), 0.01);
            elapsed += 0.01;
            if units[0].salvo_fired != before && units[0].salvo_fired > 0 {
                seen += 1;
                shots.push(elapsed);
                sides.push(side);
                let cannon = units[0].salvo_fired - 1;
                assert!(units[0].recoil[cannon] > 0.9, "this one goes back");
                if cannon > 0 {
                    assert!(
                        units[0].recoil[cannon - 1] < 0.9,
                        "the last is coming forward"
                    );
                }
            }
        }
        assert_eq!(seen, 5, "{shots:?}");
        for pair in shots[..4].windows(2) {
            assert!(pair[1] - pair[0] < 1.0, "in turn: {shots:?}");
        }
        assert!(shots[4] - shots[3] > 6.0, "calm: {shots:?}");
        let mut distinct = sides[1..4].to_vec();
        distinct.dedup();
        assert_eq!(distinct.len(), 3, "{sides:?}");
        assert!(units[0].recoil.iter().all(|&r| r < 1.0));
    }

    #[test]
    fn its_shells_are_tracers_hitting_harder_than_a_catapults_and_leave_a_path_behind_them() {
        assert!(WeaponKind::Bombard.damage() > WeaponKind::Mortar.damage());
        assert!(!WeaponKind::Bombard.can_target(UnitKind::Aircraft, 4.0));
        assert!(!WeaponKind::Bombard.fires_on_move());
        let mut units = [heavy_strider_facing_east(), target_at(10.0, 60.0)];
        units[1].cooldown = 1000.0;
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        for _ in 0..5 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        let shell = combat.shells[0];
        let tracer = shell.tracer().expect("white shells show their path");
        assert!(tracer[0].0 < shell.position().0 && tracer[3].0 < tracer[0].0);
        for _ in 0..40 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(units[1].health < 100.0);
        assert!(
            combat
                .blasts
                .iter()
                .all(|b| b.radius <= BOMBARD_BLAST_RADIUS)
        );
    }

    #[test]
    fn a_catapults_shell_shows_no_path() {
        let mut units = [catapult_facing_east(), target_at(10.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.shells[0].tracer(), None);
    }

    #[test]
    fn a_heavy_strider_turns_to_face_its_target_and_a_strider_fires_missiles_on_the_move() {
        let mut units = [heavy_strider_facing_east(), target_at(-10.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(combat.shells.is_empty(), "the target is behind it");

        let mut strider = heavy_strider_facing_east();
        strider.role = Role::Strider;
        strider.weapon = WeaponKind::Missile;
        strider.destination = (0.0, 70.0);
        let mut units = [strider, target_at(8.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.missiles.len(), 1);
    }

    #[test]
    fn an_explosion_throws_out_sparks_and_smoke_that_stay_above_the_ground_and_die_out() {
        let mut combat = Combat::default();
        combat.explode((0.0, 1.0, 60.0), 1.0);
        let small = combat.particles.len();
        assert!(small >= PARTICLES_AT_LEAST);
        assert_eq!(combat.blasts.len(), 1);
        assert!(combat.particles.iter().any(|p| p.smoke));
        assert!(combat.particles.iter().any(|p| !p.smoke));
        let directions: Vec<_> = combat.particles.iter().map(|p| p.velocity).collect();
        assert_ne!(directions[0], directions[1], "they scatter");

        let mut big = Combat::default();
        big.explode((0.0, 1.0, 60.0), 3.0);
        assert!(big.particles.len() > small);

        for _ in 0..10 {
            combat.advance(&mut [], &TEAMS, flat(), 0.05);
        }
        assert!(
            combat
                .particles
                .iter()
                .any(|p| p.position != (0.0, 1.0, 60.0))
        );
        assert!(combat.particles.iter().all(|p| p.position.1 >= 0.0));
        for _ in 0..60 {
            combat.advance(&mut [], &TEAMS, flat(), 0.05);
        }
        assert!(combat.particles.is_empty(), "they die out");
    }

    #[test]
    fn however_many_explosions_go_off_the_particles_stay_bounded() {
        let mut combat = Combat::default();
        for _ in 0..100 {
            combat.explode((0.0, 1.0, 60.0), 7.0);
        }
        assert!(combat.particles.len() <= PARTICLES_ALIVE_AT_MOST);
    }

    #[test]
    fn the_automatic_gun_fires_on_the_move() {
        let mut gunner = shooter(WeaponKind::AutoGun);
        gunner.destination = (gunner.x, 70.0);
        let mut units = [gunner, target_at(4.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(units[1].health < 100.0);
    }

    fn enemy_barracks_at(x: f32, z: f32) -> Unit {
        let mut barracks = target_at(x, z);
        barracks.role = Role::Barracks;
        barracks.kind = UnitKind::Building;
        barracks
    }

    #[test]
    fn nobody_fires_on_a_building_that_makes_units_unless_ordered_but_towers_draw_fire() {
        let mut units = [shooter(WeaponKind::Bullet), enemy_barracks_at(3.0, 60.0)];
        let before = units[1].health;
        let mut combat = Combat::default();
        for _ in 0..40 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert_eq!(units[1].health, before, "no order");

        units[0].target = Some(Target::Unit(units[1].id));
        for _ in 0..40 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(units[1].health < before, "ordered");

        let mut tower = enemy_barracks_at(3.0, 60.0);
        tower.role = Role::Tower;
        tower.kind = UnitKind::Tower;
        let mut units = [shooter(WeaponKind::Bullet), tower];
        let before = units[1].health;
        for _ in 0..40 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(units[1].health < before, "a tower is aggressive");
    }

    fn rocket_launcher_facing_east() -> Unit {
        let mut unit = Unit::for_test(0.0, 60.0, UnitKind::Vehicle);
        unit.role = Role::RocketLauncher;
        unit.id = 1;
        unit.weapon = WeaponKind::Rockets;
        unit.heading = 0.0;
        unit
    }

    #[test]
    fn a_rocket_launcher_fires_three_from_the_left_one_by_one_then_three_from_the_right_then_reloads()
     {
        let mut target = target_at(10.0, 60.0);
        target.health = 1000.0;
        let mut units = [rocket_launcher_facing_east(), target];
        let mut combat = Combat::default();
        let (mut launches, mut sides, mut elapsed) = (Vec::new(), Vec::new(), 0.0);
        while launches.len() < 7 && elapsed < 30.0 {
            units[1].health = 1000.0;
            let flying = combat.missiles.len();
            combat.advance(&mut units, &TEAMS, flat(), 0.01);
            elapsed += 0.01;
            if combat.missiles.len() > flying {
                assert_eq!(combat.missiles.len(), flying + 1, "one by one");
                launches.push(elapsed);
                sides.push(combat.missiles[flying].position.2 - units[0].z);
            }
        }
        assert_eq!(launches.len(), 7, "{launches:?}");
        // Facing east, its left is to the north, where z is smaller.
        assert!(sides[..3].iter().all(|&side| side < 0.0), "{sides:?}");
        assert!(sides[3..6].iter().all(|&side| side > 0.0), "{sides:?}");
        assert!((sides[6] - sides[0]).abs() < 0.01, "and starts over");
        for pair in launches[..6].windows(2) {
            let gap = pair[1] - pair[0];
            assert!(gap < 0.5, "quickly one after another: {gap}");
        }
        let reload = launches[6] - launches[5];
        assert!(reload > 7.0, "a long reload: {reload}");
    }

    #[test]
    fn a_rocket_launcher_turns_to_face_its_target_and_does_not_fire_on_the_move() {
        let mut units = [rocket_launcher_facing_east(), target_at(-10.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(combat.missiles.is_empty(), "the target is behind it");
        for _ in 0..80 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert!(!combat.missiles.is_empty(), "it turned round");

        let mut units = [rocket_launcher_facing_east(), target_at(10.0, 60.0)];
        units[0].destination = (0.0, 70.0);
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(combat.missiles.is_empty());
    }

    #[test]
    fn a_rocket_hurts_less_than_a_missile() {
        assert!(WeaponKind::Rockets.damage() < WeaponKind::Missile.damage());
        let mut units = [rocket_launcher_facing_east(), target_at(5.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        units[0].cooldown = 1000.0;
        for _ in 0..60 {
            combat.advance(&mut units, &TEAMS, flat(), 0.05);
        }
        assert_eq!(units[1].health, 100.0 - WeaponKind::Rockets.damage());
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
            assert!(
                cos * dx / length + sin * dz / length > 0.98,
                "it faces {x},{z}"
            );
        }
    }

    #[test]
    fn a_tank_turns_to_face_its_target_before_it_fires_its_cannon() {
        let mut tank = Unit::for_test(0.0, 60.0, UnitKind::Vehicle);
        assert_eq!(tank.role, Role::Tank);
        tank.id = 1;
        tank.weapon = WeaponKind::Cannon;
        tank.heading = 0.0;
        let mut units = [tank, target_at(-5.0, 60.0)];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 0.3);
        assert_eq!(units[1].health, 100.0, "still turning");
        assert_ne!(units[0].heading, 0.0);
        run(&mut combat, &mut units, 2.0);
        assert!(units[1].health < 100.0, "now it fires");
        assert!(units[0].heading.cos() < -0.9, "facing west");
    }

    #[test]
    fn a_catapult_facing_its_target_fires_at_once() {
        let mut units = [catapult_facing_east(), target_at(10.0, 60.0)];
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert_eq!(combat.shells.len(), 1);
    }

    #[test]
    fn a_catapult_turns_towards_a_place_it_was_told_to_attack_and_stays_still_with_nothing_to_fire_at()
     {
        let mut units = [catapult_facing_east()];
        let mut combat = Combat::default();
        run(&mut combat, &mut units, 1.0);
        assert_eq!(
            units[0].heading, 0.0,
            "it does not look around like a tower"
        );

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
