//! What an opponent does with the army it makes: it gathers what comes out at a rally point in
//! front of its base, holds it there until there are enough, then sends them all off together
//! against the nearest enemy base, fighting whatever they meet on the way. Meanwhile whatever
//! is still at home turns on enemies that come too close.

use crate::battlefield::Terrain;
use crate::game::OpponentKind;
use crate::units::{self, Unit};

/// Seconds between one look at the battle and the next.
const THINK_SECONDS: f32 = 1.0;
/// How far ahead of its base, towards the nearest enemy base, an opponent gathers its army.
const RALLY_AHEAD: f32 = 3.0;
/// How near the rally point a unit has to be to count as gathered.
const RALLY_REACH: f32 = 6.0;
/// How near its base an enemy has to come for what is at home to go after it.
const GUARD_REACH: f32 = 14.0;
/// The largest army an opponent waits for before setting off.
const WAVE_AT_MOST: usize = 30;

/// One opponent's plan.
#[derive(Clone, Debug, PartialEq)]
pub struct Strategy {
    owner: usize,
    /// How many have to be gathered before the next attack sets off.
    pub wave_size: usize,
    /// How many more the attack after that waits for.
    growth: usize,
    /// Ids of those sent to attack: they never come back.
    pub attackers: Vec<u32>,
    /// Seconds until it looks at the battle again.
    think_in: f32,
    /// Seconds of battle before its first attack may set off, however many have gathered.
    first_attack_in: f32,
}

impl Strategy {
    pub fn new(owner: usize, kind: OpponentKind) -> Self {
        // An attacker sets off early and often; a defensive one keeps a big army home first.
        let (wave_size, growth, first_attack_in) = match kind {
            OpponentKind::Attacker => (5, 2, 45.0),
            OpponentKind::Competent => (8, 2, 90.0),
            OpponentKind::Defensive => (14, 3, 150.0),
        };
        Strategy {
            owner,
            wave_size,
            growth,
            attackers: Vec::new(),
            think_in: THINK_SECONDS,
            first_attack_in,
        }
    }

    /// Where its buildings stand, on average, if it has any.
    fn base(&self, units: &[Unit]) -> Option<(f32, f32)> {
        middle(
            units
                .iter()
                .filter(|u| u.alive && u.owner == self.owner && u.kind.is_building()),
        )
    }

    /// The enemy building nearest to a place, for this owner.
    fn nearest_enemy_building(
        &self,
        units: &[Unit],
        teams: &[usize],
        (x, z): (f32, f32),
    ) -> Option<usize> {
        nearest(units, (x, z), |u| {
            u.kind.is_building() && units::are_enemies(teams, self.owner, u.owner)
        })
    }

    /// Where it gathers its army: a little out from its base towards the nearest enemy base.
    pub fn rally(&self, units: &[Unit], teams: &[usize]) -> Option<(f32, f32)> {
        let base = self.base(units)?;
        let Some(enemy) = self.nearest_enemy_building(units, teams, base) else {
            return Some(base);
        };
        let (dx, dz) = (units[enemy].x - base.0, units[enemy].z - base.1);
        let away = dx.hypot(dz).max(0.001);
        let ahead = RALLY_AHEAD.min(away / 2.0);
        Some((base.0 + dx / away * ahead, base.1 + dz / away * ahead))
    }

    /// Gives its army its orders, once a second.
    pub fn think(&mut self, units: &mut [Unit], terrain: &Terrain, teams: &[usize], dt: f32) {
        self.think_in -= dt;
        self.first_attack_in -= dt;
        if self.think_in > 0.0 {
            return;
        }
        self.think_in += THINK_SECONDS;
        self.attackers
            .retain(|&id| units.iter().any(|u| u.alive && u.id == id));
        let (Some(base), Some(rally)) = (self.base(units), self.rally(units, teams)) else {
            return;
        };
        let owner = self.owner;
        let soldier = |u: &Unit| {
            u.alive
                && u.owner == owner
                && !u.kind.is_building()
                && u.weapon.is_armed()
                && u.has_rockets()
        };
        let mut home: Vec<usize> = (0..units.len())
            .filter(|&i| soldier(&units[i]) && !self.attackers.contains(&units[i].id))
            .collect();

        // Those at home go after enemies that come too near.
        for &i in &home {
            if units[i].target.is_some() {
                continue;
            }
            let shooter = units[i];
            let intruder = nearest(units, (shooter.x, shooter.z), |u| {
                !u.kind.is_building()
                    && shooter.can_attack(u, teams)
                    && (u.x - base.0).hypot(u.z - base.1) <= GUARD_REACH
            });
            if let Some(intruder) = intruder {
                let id = units[intruder].id;
                units[i].go_after(id);
            }
        }

        // The rest gather at the rally point.
        for &i in &home {
            let unit = &units[i];
            let off = (unit.destination.0 - rally.0).hypot(unit.destination.1 - rally.1);
            if unit.target.is_none() && off > RALLY_REACH {
                units::send_to_spot(units, i, rally, terrain);
            }
        }

        // Once enough have gathered, they set off together.
        home.retain(|&i| {
            let unit = &units[i];
            unit.target.is_none()
                && !unit.is_moving()
                && (unit.x - rally.0).hypot(unit.z - rally.1) <= RALLY_REACH * 1.5
        });
        if self.first_attack_in <= 0.0 && home.len() >= self.wave_size {
            self.attackers.extend(home.iter().map(|&i| units[i].id));
            self.wave_size = (self.wave_size + self.growth).min(WAVE_AT_MOST);
        }

        for i in 0..units.len() {
            if self.attackers.contains(&units[i].id) && soldier(&units[i]) {
                self.press_on(units, i, terrain, teams);
            }
        }
    }

    /// Keeps an attacker going: it fights the nearest enemy it can see, and otherwise heads
    /// for the nearest enemy building.
    fn press_on(&self, units: &mut [Unit], i: usize, terrain: &Terrain, teams: &[usize]) {
        let unit = units[i];
        let current = match unit.target {
            Some(units::Target::Unit(id)) => units.iter().find(|u| u.alive && u.id == id),
            _ => None,
        };
        if current.is_some_and(|u| !u.kind.is_building()) {
            return;
        }
        let in_sight = nearest(units, (unit.x, unit.z), |u| {
            !u.kind.is_building()
                && unit.can_attack(u, teams)
                && (u.x - unit.x).hypot(u.z - unit.z) <= unit.sight()
        });
        if let Some(enemy) = in_sight {
            let id = units[enemy].id;
            units[i].go_after(id);
            return;
        }
        if current.is_some() {
            return;
        }
        let building = nearest(units, (unit.x, unit.z), |u| {
            u.kind.is_building() && unit.can_attack(u, teams)
        });
        if let Some(building) = building {
            let id = units[building].id;
            units[i].go_after(id);
        } else if !unit.is_moving()
            && let Some(building) = self.nearest_enemy_building(units, teams, (unit.x, unit.z))
        {
            // What cannot harm buildings, such as an air drone, still goes along.
            let at = (units[building].x, units[building].z);
            units::send_to_spot(units, i, at, terrain);
        }
    }
}

/// The middle of some units, if there are any.
fn middle<'a>(units: impl Iterator<Item = &'a Unit>) -> Option<(f32, f32)> {
    let (count, x, z) = units.fold((0, 0.0, 0.0), |(n, x, z), u| (n + 1, x + u.x, z + u.z));
    (count > 0).then(|| (x / count as f32, z / count as f32))
}

/// The living unit nearest to a place among those that fit.
fn nearest(units: &[Unit], (x, z): (f32, f32), fit: impl Fn(&Unit) -> bool) -> Option<usize> {
    (0..units.len())
        .filter(|&i| units[i].alive && fit(&units[i]))
        .min_by(|&a, &b| {
            let away = |u: &Unit| (u.x - x).hypot(u.z - z);
            away(&units[a]).total_cmp(&away(&units[b]))
        })
}
