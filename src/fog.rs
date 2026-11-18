//! What the player knows of the map: nothing, what was once seen, or what is seen right now.

use crate::battlefield::{
    CELL_SIDE, GRID_POINTS, MOST_CELLS, grid_index, grid_to_world, world_to_grid,
};
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

/// How far past a sight's edge, in squares, its distance is kept at each grid point: as far
/// as a corner's shading needs to cross it between two points.
const EDGE_REACH: f32 = 2.0;

/// The player's knowledge of each grid point of the map: how far into what is seen right now
/// it lies, and into what was ever seen, in squares, below 0 outside. Kept as distances, so
/// that between the points the edges can be drawn as round as the sights that make them.
pub struct Fog {
    now: Vec<f32>,
    ever: Vec<f32>,
}

impl Fog {
    pub fn new() -> Self {
        Fog {
            now: vec![-EDGE_REACH; GRID_POINTS],
            ever: vec![-EDGE_REACH; GRID_POINTS],
        }
    }

    /// Sees anew from these units, each the farther the higher it stands above the floor.
    pub fn update<'a>(&mut self, seers: impl Iterator<Item = &'a Unit>, floor: f32) {
        self.now.fill(-EDGE_REACH);
        for seer in seers {
            self.see_around(seer, floor);
        }
        for (ever, now) in self.ever.iter_mut().zip(&self.now) {
            *ever = ever.max(*now);
        }
    }

    fn see_around(&mut self, seer: &Unit, floor: f32) {
        let radius = seer.sight_over(floor);
        let (u, v) = world_to_grid(seer.x, seer.z);
        let reach = radius / CELL_SIDE + EDGE_REACH;
        let span = |center: f32| {
            let low = (center - reach).floor().max(0.0) as usize;
            let high = ((center + reach).ceil().max(0.0) as usize).min(MOST_CELLS);
            low..=high
        };
        for point_v in span(v) {
            for point_u in span(u) {
                let (x, z) = grid_to_world(point_u as f32, point_v as f32);
                let into = (radius - (x - seer.x).hypot(z - seer.z)) / CELL_SIDE;
                let now = &mut self.now[grid_index(point_u, point_v)];
                *now = now.max(into.clamp(-EDGE_REACH, EDGE_REACH));
            }
        }
    }

    /// The whole map is in view now.
    pub fn see_everything(&mut self) {
        self.now.fill(EDGE_REACH);
        self.ever.fill(EDGE_REACH);
    }

    pub fn at_grid(&self, u: usize, v: usize) -> Sight {
        let at = grid_index(u, v);
        if self.now[at] >= 0.0 {
            Sight::Visible
        } else if self.ever[at] >= 0.0 {
            Sight::Explored
        } else {
            Sight::Unexplored
        }
    }

    /// What is known at the grid point nearest a place in the world. Off the map, nothing.
    pub fn at(&self, x: f32, z: f32) -> Sight {
        let (u, v) = world_to_grid(x, z);
        let on_grid = |t: f32| {
            (t.round() >= 0.0 && t.round() <= MOST_CELLS as f32).then_some(t.round() as usize)
        };
        match (on_grid(u), on_grid(v)) {
            (Some(u), Some(v)) => self.at_grid(u, v),
            _ => Sight::Unexplored,
        }
    }

    /// How far into what is seen now and what was ever seen a place in the world lies, in
    /// squares, from the grid points around it. Off the map, outside both.
    pub fn edges_at(&self, x: f32, z: f32) -> [f32; 2] {
        let (u, v) = world_to_grid(x, z);
        let last = MOST_CELLS as f32;
        if !(0.0..=last).contains(&u) || !(0.0..=last).contains(&v) {
            return [-EDGE_REACH; 2];
        }
        let (u0, v0) = (u.floor().min(last - 1.0), v.floor().min(last - 1.0));
        let (du, dv) = (u - u0, v - v0);
        let (u0, v0) = (u0 as usize, v0 as usize);
        let between = |of: &[f32]| {
            let at = |u, v| of[grid_index(u, v)];
            let top = at(u0, v0) * (1.0 - du) + at(u0 + 1, v0) * du;
            let bottom = at(u0, v0 + 1) * (1.0 - du) + at(u0 + 1, v0 + 1) * du;
            top * (1.0 - dv) + bottom * dv
        };
        [between(&self.now), between(&self.ever)]
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
        let (x, z) = crate::battlefield::MAP_MIDDLE;
        let unit = unit_at(x, z);
        let mut fog = Fog::new();
        fog.update([unit].iter(), 0.0);
        assert_eq!(fog.at(x, z), Sight::Visible);
        for (dx, dz) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            let near = unit.sight() - 45.25;
            let far = unit.sight() + 45.25;
            let at = |by: f32| fog.at(x + dx * by, z + dz * by);
            assert_eq!(at(near), Sight::Visible, "{dx},{dz}");
            assert_eq!(at(far), Sight::Unexplored, "{dx},{dz}");
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
    fn between_the_points_the_edge_of_what_is_seen_runs_round() {
        let (x, z) = crate::battlefield::MAP_MIDDLE;
        let unit = unit_at(x, z);
        let mut fog = Fog::new();
        fog.update([unit].iter(), 0.0);
        // Its distance across the edge, wherever between the points it is, is within a
        // little of how far from the circle it is, ahead or slantwise.
        let radius = unit.sight();
        for turn in [0.0f32, 0.3, 0.785, 1.2] {
            for by in [-20.0, -7.0, 0.0, 7.0, 20.0] {
                let out = radius + by;
                let [now, ever] = fog.edges_at(x + out * turn.cos(), z + out * turn.sin());
                assert!((now + by / CELL_SIDE).abs() < 0.1, "{turn} {by}: {now}");
                assert_eq!(now, ever, "seen now is seen ever");
            }
        }
        fog.update([].iter(), 0.0);
        let [now, ever] = fog.edges_at(x, z);
        assert!(now < 0.0 && ever > 0.0, "dimmed, once nobody sees it");
        assert_eq!(fog.edges_at(-100.0, z), [-EDGE_REACH; 2], "off the map");
    }

    #[test]
    fn a_unit_near_a_corner_of_the_map_is_no_trouble_and_off_the_map_is_unknown() {
        let mut fog = Fog::new();
        fog.update([unit_at(0.0, 0.0)].iter(), 0.0);
        assert_eq!(fog.at(0.0, 0.0), Sight::Visible);
        assert_eq!(fog.at(0.0, -226.3), Sight::Unexplored);
    }
}
