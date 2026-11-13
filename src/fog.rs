//! What the player knows of the map: nothing, what was once seen, or what is seen right now.

use crate::battlefield::{GRID_POINTS, MapSize, grid_index, grid_to_world, world_to_grid};
use crate::units::Unit;

/// How bright explored land is that nobody sees right now.
const EXPLORED_SHADE: f32 = 0.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sight {
    /// Never seen: fully black.
    Unexplored,
    /// Seen before, but nobody sees it now: dimmed, and enemies there do not show.
    Explored,
    Visible,
}

impl Sight {
    /// What colors are multiplied by here.
    pub fn shade(self) -> f32 {
        match self {
            Sight::Unexplored => 0.0,
            Sight::Explored => EXPLORED_SHADE,
            Sight::Visible => 1.0,
        }
    }
}

/// The player's knowledge of each grid point of the map.
pub struct Fog {
    sights: Vec<Sight>,
}

impl Fog {
    pub fn new() -> Self {
        Fog {
            sights: vec![Sight::Unexplored; GRID_POINTS],
        }
    }

    /// Sees the map anew from where the given units stand now.
    /// Sees anew from these units, each the farther the higher it stands above the floor.
    pub fn update<'a>(&mut self, seers: impl Iterator<Item = &'a Unit>, floor: f32) {
        for sight in &mut self.sights {
            if *sight == Sight::Visible {
                *sight = Sight::Explored;
            }
        }
        for seer in seers {
            self.see_around(seer, floor);
        }
    }

    fn see_around(&mut self, seer: &Unit, floor: f32) {
        let radius = seer.sight_over(floor);
        let (u, v) = world_to_grid(seer.x, seer.z);
        // A step along one grid axis covers 0.7 of a world unit, so this reaches far enough.
        let reach = radius * 0.03315;
        let span = |center: f32| {
            let low = (center - reach).floor().max(0.0) as usize;
            let high = ((center + reach).ceil().max(0.0) as usize).min(MapSize::L.cells());
            low..=high
        };
        for point_v in span(v) {
            for point_u in span(u) {
                let (x, z) = grid_to_world(point_u as f32, point_v as f32);
                if (x - seer.x).hypot(z - seer.z) <= radius {
                    self.sights[grid_index(point_u, point_v)] = Sight::Visible;
                }
            }
        }
    }

    /// The whole map is in view now.
    pub fn see_everything(&mut self) {
        self.sights.fill(Sight::Visible);
    }

    pub fn at_grid(&self, u: usize, v: usize) -> Sight {
        self.sights[grid_index(u, v)]
    }

    /// What is known at the grid point nearest a place in the world. Off the map, nothing.
    pub fn at(&self, x: f32, z: f32) -> Sight {
        let (u, v) = world_to_grid(x, z);
        let on_grid = |t: f32| {
            (t.round() >= 0.0 && t.round() <= MapSize::L.cells() as f32)
                .then_some(t.round() as usize)
        };
        match (on_grid(u), on_grid(v)) {
            (Some(u), Some(v)) => self.at_grid(u, v),
            _ => Sight::Unexplored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::UnitKind;

    fn unit_at(x: f32, z: f32) -> Unit {
        Unit::for_test(x, z, UnitKind::Infantry)
    }

    #[test]
    fn everything_starts_unexplored() {
        let fog = Fog::new();
        assert_eq!(fog.at(0.0, 2715.0), Sight::Unexplored);
        assert_eq!(Sight::Unexplored.shade(), 0.0, "fully black");
    }

    #[test]
    fn from_higher_ground_a_unit_sees_farther() {
        let mut high = unit_at(0.0, 2715.0);
        high.y = 54.31;
        let beyond = high.sight() + 67.88;
        let mut fog = Fog::new();
        fog.update([unit_at(0.0, 2715.0)].iter(), 0.0);
        assert_eq!(fog.at(beyond, 2715.0), Sight::Unexplored);
        fog.update([high].iter(), 0.0);
        assert_eq!(fog.at(beyond, 2715.0), Sight::Visible, "from up there");
        let mut fog = Fog::new();
        fog.update([high].iter(), 54.31);
        assert_eq!(
            fog.at(beyond, 2715.0),
            Sight::Unexplored,
            "not above the floor itself"
        );
    }

    #[test]
    fn a_unit_sees_around_itself_as_far_as_its_sight_and_no_further() {
        let unit = unit_at(0.0, 2715.0);
        let mut fog = Fog::new();
        fog.update([unit].iter(), 0.0);
        assert_eq!(fog.at(0.0, 2715.0), Sight::Visible);
        for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let near = unit.sight() - 45.25;
            let far = unit.sight() + 45.25;
            assert_eq!(
                fog.at(dx * near, 2715.0 + dz * near),
                Sight::Visible,
                "{dx},{dz}"
            );
            assert_eq!(
                fog.at(dx * far, 2715.0 + dz * far),
                Sight::Unexplored,
                "{dx},{dz}"
            );
        }
    }

    #[test]
    fn what_nobody_sees_any_more_stays_explored_and_dimmed() {
        let mut fog = Fog::new();
        fog.update([unit_at(0.0, 2715.0)].iter(), 0.0);
        fog.update([unit_at(1358.0, 2715.0)].iter(), 0.0);
        assert_eq!(fog.at(0.0, 2715.0), Sight::Explored);
        assert_eq!(fog.at(1358.0, 2715.0), Sight::Visible);
        let dim = Sight::Explored.shade();
        assert!(dim > 0.0 && dim < Sight::Visible.shade());

        fog.update([unit_at(0.0, 2715.0)].iter(), 0.0);
        assert_eq!(fog.at(0.0, 2715.0), Sight::Visible, "seen again");
    }

    #[test]
    fn nobody_left_means_nothing_is_visible_but_what_was_seen_is_remembered() {
        let mut fog = Fog::new();
        fog.update([unit_at(0.0, 2715.0)].iter(), 0.0);
        fog.update([].iter(), 0.0);
        assert_eq!(fog.at(0.0, 2715.0), Sight::Explored);
    }

    #[test]
    fn a_unit_near_a_corner_of_the_map_is_no_trouble_and_off_the_map_is_unknown() {
        let mut fog = Fog::new();
        fog.update([unit_at(0.0, 0.0)].iter(), 0.0);
        assert_eq!(fog.at(0.0, 0.0), Sight::Visible);
        assert_eq!(fog.at(0.0, -226.3), Sight::Unexplored);
    }
}
