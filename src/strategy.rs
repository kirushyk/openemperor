//! What an opponent does with the army it makes: it gathers what comes out at a rally point in
//! front of its base, holds it there until there are enough, then sends them all off together
//! against the nearest enemy base, fighting whatever they meet on the way. Meanwhile whatever
//! is still at home turns on enemies that come too close. It starts from nothing but its MCV:
//! it deploys that where it stands and builds its base around it, one building at a time,
//! paying for each bit by bit as it goes up.

use crate::battlefield::{Terrain, next_random};
use crate::game::{Faction, OpponentKind};
use crate::units::{self, Role, Unit};
use crate::weapons::WeaponKind;

/// Seconds between one look at the battle and the next.
const THINK_SECONDS: f32 = 1.0;
/// How far ahead of its base, towards the nearest enemy base, an opponent gathers its army.
const RALLY_AHEAD: f32 = 135.8;
/// How near the rally point a unit has to be to count as gathered.
const RALLY_REACH: f32 = 271.5;
/// How near its base an enemy has to come for what is at home to go after it.
const GUARD_REACH: f32 = 633.6;
/// The largest army an opponent waits for before setting off.
const WAVE_AT_MOST: usize = 30;
/// What an opponent builds from its MCV, in order. It always builds the first of these it has
/// fewer of than the list holds up to there, so it also builds again what it loses.
fn build_order(house: Faction) -> [Role; 12] {
    let [lesser, greater] = Role::towers_of(house);
    [
        Role::Barracks,
        Role::Refinery,
        Role::Factory,
        lesser,
        greater,
        Role::Hanger,
        Role::Helipad,
        lesser,
        greater,
        Role::Refinery,
        Role::Factory,
        Role::Barracks,
    ]
}
/// How far from its MCV an opponent looks for room for a building, ring by ring, and how many
/// places it tries on each ring.
const BUILD_RINGS: [f32; 8] = [153.9, 262.5, 371.1, 479.7, 588.3, 696.9, 805.5, 914.1];
const BUILD_TRIES: usize = 12;
/// How many empty squares an opponent leaves between its buildings, for its harvesters and
/// the rest of its army to drive through.
const LANE: i32 = 3;
/// The chance, each look at the battle, that a Kobra with an enemy in its deployed reach
/// deploys, and that a deployed one with nobody left in reach packs up.
const OR_KOBRA_DEPLOY_CHANCE: f32 = 0.2;
const OR_KOBRA_PACK_UP_CHANCE: f32 = 0.25;

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
    /// What its MCV is building and the seconds left, if anything.
    pub building: Option<(Role, f32)>,
    /// What its chances are drawn from.
    random: u32,
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
            building: None,
            random: owner as u32 * 2 + 1,
        }
    }

    /// Deploys its MCV where it has stopped, and works on its base from there, paying for it
    /// out of `money` as it goes: once a building is done and there is room for it near the
    /// MCV, it is returned, with this id, to go up.
    pub fn grow(
        &mut self,
        units: &mut [Unit],
        terrain: &Terrain,
        money: &mut i32,
        dt: f32,
        id: u32,
    ) -> Option<Unit> {
        let owner = self.owner;
        let mcv = units
            .iter()
            .position(|u| u.alive && u.owner == owner && u.role == Role::Mcv)?;
        if !units[mcv].deployed {
            if units[mcv].is_moving() {
                return None;
            }
            if units::can_deploy_here(&units[mcv], terrain) {
                units::start_deploying(&mut units[mcv]);
            } else if let Some(spot) = units::deploy_spot_near(
                units,
                terrain,
                units[mcv].house,
                (units[mcv].x, units[mcv].z),
            ) {
                // Off the sand first, onto higher ground.
                units[mcv].destination = spot;
            }
            return None;
        }
        let (role, left) = match self.building {
            Some(building) => building,
            None => {
                let role = self.next_building(units)?;
                (role, role.build_seconds())
            }
        };
        let left = units::work_and_pay(role, left, dt, money);
        self.building = Some((role, left));
        if left > 0.0 {
            return None;
        }
        // Done: it waits for room, if there is none just now.
        let maker = units[mcv];
        let at = room_near(units, terrain, role, maker.house, (maker.x, maker.z))?;
        self.building = None;
        let mut building = units::new_building(terrain, at, role, (owner, maker.color), id);
        building.house = maker.house;
        Some(building)
    }

    /// What its base still needs paid: the rest of the building on its way, or all of the
    /// next one it will build. It keeps that much back from its army.
    pub fn saving_for(&self, units: &[Unit]) -> i32 {
        match self.building {
            Some((role, left)) => role.cost() - units::paid_for(role, left),
            None => self.next_building(units).map_or(0, Role::cost),
        }
    }

    /// The first building of the build order it has fewer of than the order asks for so far.
    fn next_building(&self, units: &[Unit]) -> Option<Role> {
        let has = |role: Role| {
            units
                .iter()
                .filter(|u| u.alive && u.owner == self.owner && u.role == role)
                .count()
        };
        let house = units
            .iter()
            .find(|u| u.alive && u.owner == self.owner && u.role == Role::Mcv)?
            .house;
        let order = build_order(house);
        order.iter().copied().enumerate().find_map(|(i, role)| {
            let wanted = order[..=i].iter().filter(|&&r| r == role).count();
            (has(role) < wanted).then_some(role)
        })
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
            u.kind.is_building()
                && units::are_enemies(teams, self.owner, u.owner)
                && !u.hidden_from(self.owner, teams)
        })
    }

    /// Where it gathers its army: a little out from its base towards the nearest enemy base.
    pub fn rally(&self, units: &[Unit], teams: &[usize]) -> Option<(f32, f32)> {
        let base = self.base(units)?;
        let Some(enemy) = self.nearest_enemy_building(units, teams, base) else {
            return Some(base);
        };
        let (dx, dz) = (units[enemy].x - base.0, units[enemy].z - base.1);
        let away = dx.hypot(dz).max(0.04525);
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
        self.work_or_kobras(units, teams);
        let owner = self.owner;
        // What is deployed stays where it is, out of the army's orders until it packs up.
        let soldier = |u: &Unit| {
            u.alive
                && u.owner == owner
                && !u.deployed
                && !u.kind.is_building()
                && u.weapon.is_armed()
                && u.has_rockets()
        };
        let mut home: Vec<usize> = (0..units.len())
            .filter(|&i| soldier(&units[i]) && !self.attackers.contains(&units[i].id))
            .collect();

        // Those at home go after enemies that come too near, unless they are too fast to catch.
        for &i in &home {
            if units[i].target.is_some() {
                continue;
            }
            let shooter = units[i];
            let intruder = nearest(units, (shooter.x, shooter.z), |u| {
                !u.kind.is_building()
                    && shooter.can_attack(u, teams)
                    && shooter.can_catch(u)
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

    /// Now and then deploys a Kobra that has an enemy in its deployed reach, and packs up
    /// one that has nobody left in it, so that it can move on.
    fn work_or_kobras(&mut self, units: &mut [Unit], teams: &[usize]) {
        for i in 0..units.len() {
            let unit = units[i];
            if !unit.alive || unit.owner != self.owner || unit.role != Role::OrKobra {
                continue;
            }
            let mut set_up = unit;
            set_up.weapon = WeaponKind::Siege;
            let in_reach = units.iter().any(|u| {
                set_up.can_attack(u, teams)
                    && (u.x - unit.x).hypot(u.z - unit.z) <= WeaponKind::Siege.range()
            });
            let chance = match (unit.deployed, in_reach) {
                (false, true) => OR_KOBRA_DEPLOY_CHANCE,
                (true, false) => OR_KOBRA_PACK_UP_CHANCE,
                _ => continue,
            };
            if next_random(&mut self.random) < chance {
                units::switch_deployed(&mut units[i]);
            }
        }
    }

    /// Keeps an attacker going: it fights the nearest enemy it can see and catch, and otherwise
    /// heads for the nearest enemy building.
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
                && unit.can_catch(u)
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

/// The nearest place around a point with room for a building, ring by ring.
fn room_near(
    units: &[Unit],
    terrain: &Terrain,
    role: Role,
    house: Faction,
    (x, z): (f32, f32),
) -> Option<(f32, f32)> {
    BUILD_RINGS.iter().enumerate().find_map(|(ring, &reach)| {
        (0..BUILD_TRIES).find_map(|n| {
            // Each ring starts a little further round, so that rings do not line up.
            let angle = (n as f32 + ring as f32 * 0.5) / BUILD_TRIES as f32 * std::f32::consts::TAU;
            let plot = role.plot_at(house, (x + reach * angle.cos(), z + reach * angle.sin()));
            let fits = units::can_place_building(units, terrain, plot)
                && units::leaves_lanes(units, plot, LANE);
            fits.then(|| plot.center())
        })
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::UnitKind;

    const TEAMS: [usize; 2] = [0, 1];

    /// An opponent's base and Kobra in the open, and an enemy this far from the tank.
    fn or_kobra_at(enemy_away: f32) -> [Unit; 3] {
        let mut base = Unit::for_test(1358.0, 2715.0, UnitKind::Building);
        base.id = 1;
        base.owner = 1;
        let mut tank = Unit::for_test(1358.0, 2715.0, UnitKind::Vehicle);
        tank.id = 2;
        tank.owner = 1;
        tank.role = Role::OrKobra;
        tank.weapon = WeaponKind::Cannon;
        let mut enemy = Unit::for_test(1358.0 + enemy_away, 2715.0, UnitKind::Infantry);
        enemy.id = 3;
        [base, tank, enemy]
    }

    fn think_for(strategy: &mut Strategy, units: &mut [Unit], seconds: usize) {
        let terrain = crate::battlefield::terrain(0);
        for _ in 0..seconds {
            strategy.think(units, terrain, &TEAMS, THINK_SECONDS);
        }
    }

    #[test]
    fn a_kobra_deploys_sometimes_with_an_enemy_in_reach_and_packs_up_when_there_is_none() {
        let mut units = or_kobra_at(WeaponKind::Siege.range() - 45.25);
        let mut strategy = Strategy::new(1, OpponentKind::Competent);
        think_for(&mut strategy, &mut units, 60);
        assert!(units[1].deployed, "it sets up");
        assert_eq!(units[1].weapon, WeaponKind::Siege);
        units[2].alive = false;
        think_for(&mut strategy, &mut units, 60);
        assert!(!units[1].deployed, "and moves on");
    }

    #[test]
    fn those_at_home_go_after_an_intruder_they_can_catch_and_let_a_faster_one_go() {
        for (intruder, chased) in [(Role::HkDevastator, true), (Role::AtTrike, false)] {
            let mut units = or_kobra_at(452.5);
            units[1].role = Role::AtMongoose;
            units[2].kind = UnitKind::Vehicle;
            units[2].role = intruder;
            let mut strategy = Strategy::new(1, OpponentKind::Competent);
            think_for(&mut strategy, &mut units, 1);
            assert_eq!(
                units[1].target,
                chased.then_some(units::Target::Unit(3)),
                "{intruder:?}"
            );
        }
    }

    #[test]
    fn a_kobra_with_nobody_in_reach_stays_on_its_tracks() {
        let mut units = or_kobra_at(WeaponKind::Siege.range() + 45.25);
        let mut strategy = Strategy::new(1, OpponentKind::Competent);
        think_for(&mut strategy, &mut units, 60);
        assert!(!units[1].deployed);
    }

    #[test]
    fn a_deployed_kobra_takes_no_marching_orders() {
        let mut units = or_kobra_at(WeaponKind::Siege.range() - 45.25);
        units::switch_deployed(&mut units[1]);
        let stays = units[1].destination;
        let mut strategy = Strategy::new(1, OpponentKind::Competent);
        strategy.random = 2;
        strategy.think(
            &mut units,
            crate::battlefield::terrain(0),
            &TEAMS,
            THINK_SECONDS,
        );
        assert_eq!(units[1].destination, stays);
    }
}
