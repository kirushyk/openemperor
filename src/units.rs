//! Placeholder units and the geometry of selecting them.

use crate::battlefield::{MAP_MIDDLE_Z, Terrain};
use crate::game::TeamColor;
use std::f32::consts::TAU;

/// Half a unit's width and its full height, in world units.
pub const HALF_WIDTH: f32 = 0.3;
pub const HEIGHT: f32 = 0.5;
const SPACING: f32 = 1.2;
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
            units.push(Unit { x, y: terrain.height_at(x, z), z, owner, color, selected: false });
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
                assert!((a.x - b.x).hypot(a.z - b.z) > HALF_WIDTH * 2.0, "{a:?} and {b:?}");
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
                        let gap_x = (unit.x - piece.x).abs() - HALF_WIDTH - piece.half_x;
                        let gap_z = (unit.z - piece.z).abs() - HALF_WIDTH - piece.half_z;
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
        let unit = Unit { x: 0.0, y: 0.0, z: 60.0, owner: 0, color: TeamColor::Blue, selected: false };
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
        let low = Unit { x: 0.0, y: 0.0, z: 60.0, owner: 0, color: TeamColor::Blue, selected: false };
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
