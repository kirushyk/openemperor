//! The ways of killing: bullets, missiles, lasers, mortars and the superweapon.
//!
//! Everything here is plain simulation with no drawing. Units fire by themselves at the
//! nearest enemy in range, on the move or standing still.

use crate::battlefield::Terrain;
use crate::units::{Unit, UnitKind, are_enemies};

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

const BEAM_SECONDS: f32 = 0.2;
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
}

impl WeaponKind {
    pub fn range(self) -> f32 {
        match self {
            WeaponKind::Bullet => 7.0,
            WeaponKind::Missile => 12.0,
            WeaponKind::Laser => 9.0,
            WeaponKind::Mortar => 16.0,
        }
    }

    /// Mortar shells only come down on the ground. Everything else can aim up.
    pub fn can_target(self, kind: UnitKind) -> bool {
        self != WeaponKind::Mortar || !kind.flies()
    }

    fn cooldown(self) -> f32 {
        match self {
            WeaponKind::Bullet => 0.6,
            WeaponKind::Missile => 3.0,
            WeaponKind::Laser => 1.2,
            WeaponKind::Mortar => 4.0,
        }
    }

    fn damage(self) -> f32 {
        match self {
            WeaponKind::Bullet => 12.0,
            WeaponKind::Missile => 60.0,
            WeaponKind::Laser => 25.0,
            WeaponKind::Mortar => 70.0,
        }
    }

    /// Bullets barely scratch armor.
    fn damage_to(self, kind: UnitKind) -> f32 {
        match (self, kind) {
            (WeaponKind::Bullet, UnitKind::Vehicle) => self.damage() * 0.25,
            _ => self.damage(),
        }
    }
}

type Point = (f32, f32, f32);

fn distance(a: Point, b: Point) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1).hypot(a.2 - b.2)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Missile {
    pub position: Point,
    /// A unit vector.
    pub direction: Point,
    target_id: u32,
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
        self.fire(units, teams, dt);
        self.fly_missiles(units, terrain, dt);
        self.fly_shells(units, teams, dt);
        self.fall_strikes(units, dt);
        self.poison_land(units, dt);
        for beam in &mut self.beams {
            beam.life -= dt / BEAM_SECONDS;
        }
        self.beams.retain(|b| b.life > 0.0);
        for blast in &mut self.blasts {
            blast.life -= dt / BLAST_SECONDS;
        }
        self.blasts.retain(|b| b.life > 0.0);
    }

    fn fire(&mut self, units: &mut [Unit], teams: &[usize], dt: f32) {
        for i in 0..units.len() {
            if !units[i].alive {
                continue;
            }
            units[i].cooldown -= dt;
            if units[i].cooldown > 0.0 {
                continue;
            }
            let Some(target) = choose_target(units, i, teams) else {
                units[i].cooldown = RETARGET_SECONDS;
                continue;
            };
            let weapon = units[i].weapon;
            units[i].cooldown = weapon.cooldown();
            let (from, to) = (units[i].muzzle(), units[target].center());
            match weapon {
                WeaponKind::Bullet => hit(&mut units[target], weapon),
                WeaponKind::Laser => {
                    hit(&mut units[target], weapon);
                    self.beams.push(Beam { from, to, life: 1.0 });
                }
                WeaponKind::Missile => {
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
                        target_id: units[target].id,
                        fuel: MISSILE_FUEL_SECONDS,
                    });
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
                    });
                }
            }
        }
    }

    fn fly_missiles(&mut self, units: &mut [Unit], terrain: &Terrain, dt: f32) {
        let mut flying = Vec::with_capacity(self.missiles.len());
        for mut missile in std::mem::take(&mut self.missiles) {
            let target = units.iter().position(|u| u.alive && u.id == missile.target_id);
            if let Some(target) = target {
                missile.direction = turn_towards(
                    missile.direction,
                    direction_to(missile.position, units[target].center()),
                    MISSILE_TURN * dt,
                );
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
            if reached {
                let t = target.unwrap_or(0);
                hit(&mut units[t], WeaponKind::Missile);
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
            for unit in units.iter_mut() {
                if unit.alive && are_enemies(teams, shell.owner, unit.owner) {
                    let away = distance(shell.landing, unit.center());
                    let reach = SHELL_BLAST_RADIUS + unit.kind.radius();
                    if away < reach {
                        hurt(unit, WeaponKind::Mortar.damage() * (1.0 - away / reach * 0.5));
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

fn hit(unit: &mut Unit, weapon: WeaponKind) {
    let damage = weapon.damage_to(unit.kind);
    hurt(unit, damage);
}

fn hurt(unit: &mut Unit, damage: f32) {
    unit.health -= damage;
    if unit.health <= 0.0 {
        unit.alive = false;
        unit.selected = false;
    }
}

/// The enemy it was told to attack if that one is in range, otherwise whoever is nearest.
fn choose_target(units: &[Unit], shooter: usize, teams: &[usize]) -> Option<usize> {
    let unit = &units[shooter];
    let ordered = unit.target.and_then(|id| units.iter().position(|u| u.alive && u.id == id));
    ordered
        .filter(|&t| {
            unit.can_attack(&units[t], teams) && distance(unit.center(), units[t].center()) <= unit.weapon.range()
        })
        .or_else(|| nearest_enemy(units, shooter, teams))
}

/// The nearest living enemy within a unit's weapon range.
fn nearest_enemy(units: &[Unit], shooter: usize, teams: &[usize]) -> Option<usize> {
    let from = units[shooter].center();
    let owner = units[shooter].owner;
    let mut best = None;
    let weapon = units[shooter].weapon;
    let mut best_distance = weapon.range();
    for (i, unit) in units.iter().enumerate() {
        if unit.alive && are_enemies(teams, owner, unit.owner) && weapon.can_target(unit.kind) {
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
        assert_eq!(combat, Combat::default(), "nothing to see");
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
        units[0].target = Some(3);
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
        units[0].target = Some(3);
        let mut combat = Combat::default();
        combat.advance(&mut units, &TEAMS, flat(), 0.05);
        assert!(units[1].health < 100.0);
        assert_eq!(units[2].health, 100.0);
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
