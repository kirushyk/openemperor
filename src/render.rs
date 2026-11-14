//! Turns game state into a triangle list. Backends only ever see colored triangles.

use crate::battlefield;
use crate::fog::{Fog, Sight};
use crate::font;
use crate::game::{
    Action, Ally, BuildTab, Button, CONFIRM_MESSAGE, CursorLook, Faction, Game, OPPONENT_SLOTS,
    Outcome, PlaceSquare, STATS_ROW_STEP, STATS_ROW_TOP, SUPERWEAPON_CELL, Screen, TeamColor,
    buttons, checkbox_size, color_slots, map_items, minimap_rect, modal_rect, opponent_row,
    opponent_widgets, options_panel, setup_checkboxes, setup_widgets, small_text_px, stats_rect,
    ui_scale,
};
use crate::hud;
use crate::planet;
use crate::platform::{EffectVertex, Frame, MAX_LIGHTS, PointLight, Vertex, WorldVertex};
use crate::units::{self, Role, Target, Unit, UnitKind};
use crate::weapons::STRIKE_RADIUS;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};
use std::sync::OnceLock;
use std::thread;

mod backdrop;
mod models;
mod original;

pub use original::{
    PICTURE_SIDE, load as load_original_models, muzzles as original_muzzles,
    pictures as original_pictures, wash as original_wash,
};

type Color = [f32; 4];

pub const CLEAR: [f32; 3] = [0.05, 0.05, 0.08];
const INTRO_CLEAR: Color = [0.0, 0.0, 0.0, 1.0];
const CLEAR_RGBA: Color = [CLEAR[0], CLEAR[1], CLEAR[2], 1.0];
const TEXT: Color = [0.93, 0.89, 0.78, 1.0];
/// How bright a headlight is where it leaves the vehicle.
const HEADLIGHT: f32 = 0.35;
/// How bright the flame of a rocket engine is at its nozzle.
const ROCKET_FLAME: [f32; 3] = [1.0, 0.6, 0.15];
const BUILDING_DOOR: [f32; 3] = [0.05, 0.05, 0.06];
const SMOKE: [f32; 3] = [0.16, 0.15, 0.14];
pub const MAKING_DIM: Color = [0.0, 0.0, 0.0, 0.62];
const PLACE_OK: [f32; 3] = [0.3, 1.0, 0.4];
const PLACE_NOT: [f32; 3] = [1.0, 0.25, 0.2];
const PLACE_EXIT: [f32; 3] = [1.0, 0.85, 0.15];
const CABLE: [f32; 3] = [0.1, 0.1, 0.1];
const GATHERING_FLAG: [f32; 3] = [0.95, 0.1, 0.08];
const ROCKET_PIP: Color = [0.95, 0.8, 0.25, 1.0];
const ROCKET_PIP_GONE: Color = [0.1, 0.1, 0.1, 0.8];
const QUIET: Color = [0.55, 0.53, 0.48, 1.0];
const GOLD: Color = [0.95, 0.76, 0.32, 1.0];
const GOLD_DIM: Color = [0.62, 0.50, 0.24, 1.0];
const PANEL: Color = [0.09, 0.09, 0.13, 1.0];
const BUTTON: Color = [0.13, 0.13, 0.18, 1.0];
const BUTTON_HOVER: Color = [0.22, 0.21, 0.27, 1.0];
const BUTTON_PRESSED: Color = [0.32, 0.27, 0.18, 1.0];
const BORDER: Color = [0.38, 0.35, 0.28, 1.0];
pub const BANNER_BLUE: [f32; 3] = [0.10, 0.20, 0.44];
const BANNER_HOVER: [f32; 3] = [0.18, 0.33, 0.66];
const BANNER_PRESSED: [f32; 3] = [0.07, 0.13, 0.30];
pub const BANNER_LINE: [f32; 3] = [0.98, 0.70, 0.20];
/// How opaque a button's lines are at their two ends; they are solid in the middle.
pub const BANNER_LINE_END_ALPHA: f32 = 0.5;
const DIM: Color = [0.0, 0.0, 0.0, 0.6];
const INK: Color = [0.05, 0.05, 0.08, 1.0];
const WINDOW: Color = [0.07, 0.07, 0.10, 0.78];
const CHAT_BACKING: Color = [0.0, 0.0, 0.0, 0.45];
const SELECTED_RING: Color = [0.35, 1.0, 0.45, 1.0];
pub const ORDER_LINE: Color = [0.35, 1.0, 0.45, 0.85];
pub const ATTACK_LINE: Color = [1.0, 0.2, 0.15, 0.85];
pub const ATTACK_CURSOR: Color = [0.95, 0.12, 0.1, 1.0];
pub const MOVE_CURSOR: Color = [0.2, 0.92, 0.32, 1.0];
/// Times per second the arrows of a green cursor draw in and out.
const CURSOR_PULSE: f32 = 1.2;
pub const HEAL_LINE: Color = [0.3, 0.6, 1.0, 0.85];
pub const HEAL_CURSOR: Color = [0.25, 0.6, 1.0, 1.0];
pub const MEDAL: Color = [1.0, 0.82, 0.25, 1.0];
const POISON_FILL: [f32; 3] = [0.25, 0.95, 0.25];
const POISON_ALPHA: f32 = 0.32;
const LASER_GLOW: Color = [1.0, 0.25, 0.2, 0.35];
const LASER_CORE: Color = [1.0, 0.9, 0.85, 1.0];
const REPAIR_GLOW: Color = [0.3, 1.0, 0.5, 0.35];
const REPAIR_CORE: Color = [0.85, 1.0, 0.9, 1.0];
const FIRE_COLOR: [f32; 3] = [1.0, 0.5, 0.1];
const SANDBAGS: [f32; 3] = [0.62, 0.55, 0.4];
const PLASMA_TRAIL: Color = [0.35, 0.55, 1.0, 0.55];

/// The walls and roofs a house builds, whatever side they are on: its side's color is only
/// for the little touches on them.
fn house_paint(house: Faction) -> ([f32; 3], [f32; 3]) {
    match house {
        Faction::Atreides => ([0.56, 0.51, 0.43], [0.66, 0.61, 0.52]),
        Faction::Harkonnen => ([0.17, 0.17, 0.19], [0.25, 0.25, 0.27]),
        Faction::Ordos => ([0.66, 0.76, 0.78], [0.76, 0.85, 0.87]),
    }
}
const GAS_COLOR: [f32; 3] = [0.35, 0.9, 0.25];
const FLARE_GLOW: [f32; 3] = [1.0, 0.7, 0.2];
const GUN_STREAK: Color = [1.0, 0.93, 0.6, 1.0];
const GUN_TRAIL: Color = [1.0, 0.8, 0.4, 0.45];
const FLARE_CORE: [f32; 3] = [1.0, 0.97, 0.8];
const BLAST_COLOR: [f32; 3] = [1.0, 0.62, 0.15];
const MISSILE_BODY: Color = [0.95, 0.95, 0.9, 1.0];
const MISSILE_FLAME: Color = [1.0, 0.6, 0.15, 0.8];
const SHELL_BODY: Color = [0.12, 0.12, 0.12, 1.0];
const HEALTH_GOOD: Color = [0.3, 0.9, 0.3, 1.0];
const HEALTH_BAD: Color = [0.9, 0.2, 0.15, 1.0];
const DEFEAT_RED: Color = [0.92, 0.22, 0.18, 1.0];
const BOLT_GLOW: Color = [0.35, 0.55, 1.0, 0.25];
const BOLT_HALO: Color = [0.55, 0.75, 1.0, 0.65];
const BOLT_CORE: Color = [0.95, 0.98, 1.0, 1.0];
const ENERGY_LOW: [f32; 3] = [0.90, 0.15, 0.12];
const ENERGY_HIGH: [f32; 3] = [0.20, 0.85, 0.25];
const GAUGE_EMPTY: Color = [0.10, 0.10, 0.13, 1.0];
const VIEW_OUTLINE: Color = [1.0, 1.0, 1.0, 1.0];
const CURSOR_FILL: Color = [0.98, 0.98, 0.95, 1.0];
const CURSOR_EDGE: Color = [0.0, 0.0, 0.0, 1.0];

pub const TITLE: &str = "OpenEmperor";
pub const VERSION: &str = "v2.00";

/// A color with this opacity.
fn with_alpha([r, g, b]: [f32; 3], alpha: f32) -> Color {
    [r, g, b, alpha]
}

/// A color with its opacity scaled down by `life`, as it fades out.
fn faded([r, g, b, a]: Color, life: f32) -> Color {
    [r, g, b, a * life]
}

/// The thin border most of the interface is drawn with.
fn border_px(height: f32) -> f32 {
    (2.0 * ui_scale(height)).round().max(1.0)
}

#[derive(Default)]
struct Canvas {
    /// What goes on the screen over the whole world.
    vertices: Vec<Vertex>,
    ground: Vec<WorldVertex>,
    /// What lies on the ground, see-through.
    decals: Vec<WorldVertex>,
    /// All that stands on the map, solid.
    objects: Vec<WorldVertex>,
    /// What casts a shadow without being drawn as solid.
    casters: Vec<WorldVertex>,
    /// What is see-through, hidden by whatever solid is in front of it.
    effects: Vec<EffectVertex>,
    /// What looks at the models being made, so that faces none of it sees are left out; none
    /// keeps them all.
    watchers: Option<models::Watchers>,
    /// How much of the model being made is worth making.
    detail: models::Detail,
}

/// A point in the world, as (x, height, z).
type Point = (f32, f32, f32);

impl Canvas {
    /// A solid face, with its first two corners at its foot and the others at its head.
    fn solid_quad(
        &mut self,
        [a, b, c, d]: [Point; 4],
        normal: [f32; 3],
        (foot, head): ([f32; 3], [f32; 3]),
    ) {
        let corner = |point: Point, color: [f32; 3]| WorldVertex::solid(point, normal, color);
        self.objects.extend([
            corner(a, foot),
            corner(b, foot),
            corner(c, head),
            corner(a, foot),
            corner(c, head),
            corner(d, head),
        ]);
    }

    /// A long square bar between two points, its flat sides along `side`, a unit vector across
    /// the way it runs.
    fn solid_bar(&mut self, a: Point, b: Point, half: f32, side: [f32; 3], color: [f32; 3]) {
        let way = unit_vector([b.0 - a.0, b.1 - a.1, b.2 - a.2]);
        let up = unit_vector(cross(way, side));
        let at = |point: Point, s: f32, u: f32| {
            (
                point.0 + (side[0] * s + up[0] * u) * half,
                point.1 + (side[1] * s + up[1] * u) * half,
                point.2 + (side[2] * s + up[2] * u) * half,
            )
        };
        for (s0, u0, s1, u1, normal) in [
            (1.0, -1.0, 1.0, 1.0, side),
            (-1.0, 1.0, -1.0, -1.0, side.map(|c| -c)),
            (1.0, 1.0, -1.0, 1.0, up),
            (-1.0, -1.0, 1.0, -1.0, up.map(|c| -c)),
        ] {
            self.solid_quad(
                [at(a, s0, u0), at(a, s1, u1), at(b, s1, u1), at(b, s0, u0)],
                normal,
                (color, color),
            );
        }
    }

    /// Moves what was drawn on the screen since `mark` among the effects, all at one depth.
    fn deepen(&mut self, mark: usize, depth: f32) {
        self.effects.extend(
            self.vertices
                .drain(mark..)
                .map(|vertex| EffectVertex::of(vertex, depth)),
        );
    }

    /// A line in the world of a thickness in pixels, hidden by whatever solid is in front of it.
    fn deep_line(
        &mut self,
        view: &battlefield::View,
        a: Point,
        b: Point,
        thickness: f32,
        c: Color,
    ) {
        let (Some(from), Some(to)) = (
            view.project_deep(a.0, a.1, a.2),
            view.project_deep(b.0, b.1, b.2),
        ) else {
            return;
        };
        let (dx, dy) = (to.0 - from.0, to.1 - from.1);
        let length = dx.hypot(dy);
        if length == 0.0 {
            return;
        }
        let (nx, ny) = (
            -dy / length * thickness / 2.0,
            dx / length * thickness / 2.0,
        );
        let corner = |(x, y, depth): (f32, f32, f32), sign: f32| {
            EffectVertex::of(Vertex::new((x + nx * sign, y + ny * sign), c), depth)
        };
        let (p, q) = (corner(from, 1.0), corner(to, 1.0));
        let (r, t) = (corner(to, -1.0), corner(from, -1.0));
        self.effects.extend([p, q, r, p, r, t]);
    }

    /// A see-through triangle in the world, each corner of its own color.
    fn deep_fill(&mut self, view: &battlefield::View, corners: [(Point, Color); 3]) {
        let placed = corners.map(|((x, y, z), color)| {
            view.project_deep(x, y, z)
                .map(|(px, py, depth)| EffectVertex::of(Vertex::new((px, py), color), depth))
        });
        if let [Some(a), Some(b), Some(c)] = placed {
            self.effects.extend([a, b, c]);
        }
    }

    /// A triangle of light in the world, each corner of its own brightness.
    fn deep_light(&mut self, view: &battlefield::View, corners: [(Point, [f32; 3]); 3]) {
        let placed = corners.map(|((x, y, z), color)| {
            view.project_deep(x, y, z)
                .map(|(px, py, depth)| EffectVertex::of(Vertex::light((px, py), color), depth))
        });
        if let [Some(a), Some(b), Some(c)] = placed {
            self.effects.extend([a, b, c]);
        }
    }

    fn triangle(&mut self, points: [(f32, f32); 3], c: Color) {
        self.vertices
            .extend(points.map(|point| Vertex::new(point, c)));
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.triangle([(x, y), (x + w, y), (x, y + h)], c);
        self.triangle([(x + w, y), (x + w, y + h), (x, y + h)], c);
    }

    /// A rectangle with a border of another color inside its edges.
    fn bordered_rect(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        border: f32,
        edge: Color,
        fill: Color,
    ) {
        self.rect(x, y, w, h, edge);
        self.rect(
            x + border,
            y + border,
            w - border * 2.0,
            h - border * 2.0,
            fill,
        );
    }

    /// A circle with a border of another color inside its edge.
    fn bordered_circle(
        &mut self,
        (cx, cy): (f32, f32),
        r: f32,
        border: f32,
        edge: Color,
        fill: Color,
    ) {
        self.circle(cx, cy, r, edge);
        self.circle(cx, cy, r - border, fill);
    }

    /// A rectangle of one color, opaque along its middle and fading sideways to `end_alpha`.
    fn fading_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 3], end_alpha: f32) {
        let vertex = |x: f32, y: f32, a: f32| Vertex::new((x, y), with_alpha(color, a));
        let middle = x + w / 2.0;
        for (from, to) in [(x, middle), (x + w, middle)] {
            self.vertices.extend([
                vertex(from, y, end_alpha),
                vertex(to, y, 1.0),
                vertex(to, y + h, 1.0),
                vertex(from, y, end_alpha),
                vertex(to, y + h, 1.0),
                vertex(from, y + h, end_alpha),
            ]);
        }
    }

    /// A blue band fading out to both sides, between two orange lines.
    fn banner(&mut self, x: f32, y: f32, w: f32, h: f32, thickness: f32, fill: [f32; 3]) {
        self.fading_rect(x, y, w, h, fill, 0.0);
        self.fading_rect(x, y, w, thickness, BANNER_LINE, BANNER_LINE_END_ALPHA);
        self.fading_rect(
            x,
            y + h - thickness,
            w,
            thickness,
            BANNER_LINE,
            BANNER_LINE_END_ALPHA,
        );
    }

    fn circle(&mut self, cx: f32, cy: f32, r: f32, c: Color) {
        const SEGMENTS: usize = 96;
        let point = |i: usize| {
            let angle = i as f32 / SEGMENTS as f32 * TAU;
            (cx + r * angle.cos(), cy + r * angle.sin())
        };
        for i in 0..SEGMENTS {
            self.triangle([(cx, cy), point(i), point(i + 1)], c);
        }
    }

    /// A fan around the center alternating between two radii: a star, or with
    /// `inner = outer * cos(PI / points)` a regular polygon.
    fn star(
        &mut self,
        (cx, cy): (f32, f32),
        (outer, inner): (f32, f32),
        points: usize,
        rotation: f32,
        c: Color,
    ) {
        let vertex = |i: usize| {
            let r = if i.is_multiple_of(2) { outer } else { inner };
            let angle = rotation + i as f32 * PI / points as f32;
            (cx + r * angle.cos(), cy + r * angle.sin())
        };
        for i in 0..points * 2 {
            self.triangle([(cx, cy), vertex(i), vertex(i + 1)], c);
        }
    }

    fn line(&mut self, a: (f32, f32), b: (f32, f32), thickness: f32, c: Color) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx.hypot(dy);
        if length == 0.0 {
            return;
        }
        let (nx, ny) = (
            -dy / length * thickness / 2.0,
            dx / length * thickness / 2.0,
        );
        let (p, q) = ((a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny));
        let (r, t) = ((b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny));
        self.triangle([p, q, r], c);
        self.triangle([p, r, t], c);
    }

    fn polygon(&mut self, cx: f32, cy: f32, r: f32, sides: usize, rotation: f32, c: Color) {
        self.star(
            (cx, cy),
            (r, r * (PI / sides as f32).cos()),
            sides,
            rotation,
            c,
        );
    }

    /// Draws `text` with its cap height centered on `cy`, one font pixel = `px` screen pixels.
    fn text_centered(&mut self, text: &str, cx: f32, cy: f32, px: f32, c: Color) {
        // Snap to whole pixels so the font stays crisp at any UI scale.
        let px = px.round().max(1.0);
        self.text_left(text, cx - font::text_width(text) * px / 2.0, cy, px, c);
    }

    /// Same as `text_centered`, starting at `x`. `px` must already be a whole number.
    fn text_left(&mut self, text: &str, x: f32, cy: f32, px: f32, c: Color) {
        let mut x = x.round();
        let y = (cy - font::CAP_HEIGHT * px / 2.0).round();
        for ch in text.chars() {
            for (col, bits) in font::glyph(ch).into_iter().enumerate() {
                for row in 0..8 {
                    if bits & (1 << row) != 0 {
                        self.rect(x + col as f32 * px, y + row as f32 * px, px, px, c);
                    }
                }
            }
            x += font::ADVANCE * px;
        }
    }

    fn cursor(&mut self, (x, y): (f32, f32), s: f32) {
        let outline = 26.0 * s;
        let fill = 20.0 * s;
        let inset = 2.5 * s;
        self.triangle(
            [
                (x, y),
                (x, y + outline),
                (x + outline * 0.72, y + outline * 0.72),
            ],
            CURSOR_EDGE,
        );
        let (fx, fy) = (x + inset * 0.6, y + inset * 1.6);
        self.triangle(
            [
                (fx, fy),
                (fx, fy + fill),
                (fx + fill * 0.72, fy + fill * 0.72),
            ],
            CURSOR_FILL,
        );
    }

    /// A blue ring around a blue cross, centered on the cursor.
    fn heal_cursor(&mut self, (x, y): (f32, f32), s: f32) {
        let (radius, arm, wide) = (20.0 * s, 11.0 * s, 7.0 * s);
        let edge = (2.0 * s).max(1.0);
        let thickness = (4.0 * s).max(1.0);
        self.ring((x, y), radius, thickness + edge * 2.0, CURSOR_EDGE);
        self.ring((x, y), radius, thickness, HEAL_CURSOR);
        for (half_w, half_h, grow, color) in [
            (arm, wide / 2.0, edge, CURSOR_EDGE),
            (arm, wide / 2.0, 0.0, HEAL_CURSOR),
        ] {
            let (w, h) = (half_w + grow, half_h + grow);
            self.rect(x - w, y - h, w * 2.0, h * 2.0, color);
            self.rect(x - h, y - w, h * 2.0, w * 2.0, color);
        }
    }

    /// A blue ring holding a hook.
    fn carry_cursor(&mut self, (x, y): (f32, f32), s: f32) {
        let edge = (2.0 * s).max(1.0);
        let thickness = (4.0 * s).max(1.0);
        let hook = [
            (x, y - 12.0 * s),
            (x, y + 1.0 * s),
            (x + 6.0 * s, y + 6.0 * s),
            (x, y + 12.0 * s),
            (x - 6.0 * s, y + 6.0 * s),
        ];
        for (grow, color) in [(edge * 2.0, CURSOR_EDGE), (0.0, HEAL_CURSOR)] {
            self.ring((x, y), 20.0 * s, thickness + grow, color);
            for piece in hook.windows(2) {
                self.line(piece[0], piece[1], thickness + grow, color);
            }
        }
    }

    /// A ring of short straight pieces.
    fn ring(&mut self, (cx, cy): (f32, f32), r: f32, thickness: f32, c: Color) {
        const SIDES: usize = 32;
        let point = |i: usize| {
            let (sin, cos) = (i as f32 / SIDES as f32 * TAU).sin_cos();
            (cx + r * cos, cy + r * sin)
        };
        for i in 0..SIDES {
            self.line(point(i), point(i + 1), thickness, c);
        }
    }

    /// A red bullseye centered on the cursor, with four arrows around it pointing in and
    /// slowly turning. A dark edge keeps it readable over red units and red ground.
    /// A green ring with four arrows inside it, drawing in on the point it would send them to.
    fn move_cursor(&mut self, at: (f32, f32), s: f32, seconds: f32) {
        self.green_ring(at, 26.0 * s, s);
        let pulse = 3.0 * s * (seconds * CURSOR_PULSE * TAU).sin();
        self.four_arrows(at, (7.0 * s + pulse, 18.0 * s + pulse), s);
    }

    /// The move cursor around a small wedge of five dots, a group keeping its shape on the way.
    fn march_cursor(&mut self, (x, y): (f32, f32), s: f32, seconds: f32) {
        const DOTS: [(f32, f32); 5] = [
            (0.0, -6.0),
            (-6.0, 0.0),
            (6.0, 0.0),
            (-12.0, 6.0),
            (12.0, 6.0),
        ];
        self.move_cursor((x, y), s, seconds);
        let edge = (2.0 * s).max(1.0);
        for (dx, dy) in DOTS {
            let (dx, dy) = (x + dx * s * 0.6, y + dy * s * 0.6);
            self.circle(dx, dy, 2.0 * s + edge, CURSOR_EDGE);
            self.circle(dx, dy, 2.0 * s, MOVE_CURSOR);
        }
    }

    /// A gold dollar sign in a gold ring, beating, for selling what is under it.
    fn sell_cursor(&mut self, (x, y): (f32, f32), s: f32, seconds: f32) {
        let thickness = (4.0 * s).max(1.0);
        let edge = (2.0 * s).max(1.0);
        let radius = 20.0 * s + 2.0 * s * (seconds * CURSOR_PULSE * TAU).sin();
        self.ring((x, y), radius, thickness + edge * 2.0, CURSOR_EDGE);
        self.ring((x, y), radius, thickness, GOLD);
        let px = (3.0 * s).round().max(1.0);
        for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
            self.text_centered("$", x + dx * edge, y + dy * edge, px, CURSOR_EDGE);
        }
        self.text_centered("$", x, y, px, GOLD);
    }

    /// A green ring with four arrows outside it, pushing away from it as a unit spreads out to
    /// dig in.
    fn deploy_cursor(&mut self, at: (f32, f32), s: f32, seconds: f32) {
        self.green_ring(at, 14.0 * s, s);
        let pulse = 3.0 * s * (seconds * CURSOR_PULSE * TAU).sin();
        self.four_arrows(at, (35.0 * s + pulse, 20.0 * s + pulse), s);
    }

    /// A green ring with four arrows inside it around a dot, for a unit drawing back in.
    fn undeploy_cursor(&mut self, (x, y): (f32, f32), s: f32, seconds: f32) {
        self.green_ring((x, y), 34.0 * s, s);
        let pulse = 3.0 * s * (seconds * CURSOR_PULSE * TAU).sin();
        self.four_arrows((x, y), (13.0 * s + pulse, 26.0 * s + pulse), s);
        let edge = (2.0 * s).max(1.0);
        self.circle(x, y, 4.0 * s + edge, CURSOR_EDGE);
        self.circle(x, y, 4.0 * s, MOVE_CURSOR);
    }

    /// A green ring around a green H, as on a landing pad, with an arrow over it coming down
    /// onto it again and again.
    fn land_cursor(&mut self, (x, y): (f32, f32), s: f32, seconds: f32) {
        self.green_ring((x, y), 24.0 * s, s);
        let edge = (2.0 * s).max(1.0);
        let (half_h, apart, bar) = (10.0 * s, 6.0 * s, 2.5 * s);
        for (grow, color) in [(edge, CURSOR_EDGE), (0.0, MOVE_CURSOR)] {
            for side in [-1.0, 1.0] {
                let left = x + side * apart - bar - grow;
                self.rect(
                    left,
                    y - half_h - grow,
                    (bar + grow) * 2.0,
                    (half_h + grow) * 2.0,
                    color,
                );
            }
            self.rect(
                x - apart,
                y - bar - grow,
                apart * 2.0,
                (bar + grow) * 2.0,
                color,
            );
        }
        // The arrow drops from high over the ring down onto it, then starts again.
        self.dropping_arrow((x, y - 46.0 * s), y - 28.0 * s, s, seconds);
    }

    /// A green ring around an open box, with an arrow dropping into it again and again, for
    /// soldiers getting into an APC.
    fn board_cursor(&mut self, (x, y): (f32, f32), s: f32, seconds: f32) {
        self.green_ring((x, y), 26.0 * s, s);
        let edge = (2.0 * s).max(1.0);
        let thickness = (4.0 * s).max(1.0);
        let (half, top, bottom) = (12.0 * s, y - 2.0 * s, y + 12.0 * s);
        let open_box = [
            (x - half, top),
            (x - half, bottom),
            (x + half, bottom),
            (x + half, top),
        ];
        for (grow, color) in [(edge * 2.0, CURSOR_EDGE), (0.0, MOVE_CURSOR)] {
            for piece in open_box.windows(2) {
                self.line(piece[0], piece[1], thickness + grow, color);
            }
        }
        self.dropping_arrow((x, y - 20.0 * s), y + 8.0 * s, s, seconds);
    }

    /// A green arrow pointing down, falling with its tip from one point down to a height, then
    /// starting again.
    fn dropping_arrow(&mut self, (x, from): (f32, f32), to: f32, s: f32, seconds: f32) {
        let edge = (2.0 * s).max(1.0);
        let fall = (seconds * CURSOR_PULSE).fract();
        let tip = from + (to - from) * fall;
        let (half, tall) = (8.0 * s, 11.0 * s);
        self.triangle(
            [
                (x, tip + edge * 1.8),
                (x - half - edge * 1.6, tip - tall - edge),
                (x + half + edge * 1.6, tip - tall - edge),
            ],
            CURSOR_EDGE,
        );
        self.triangle(
            [(x, tip), (x - half, tip - tall), (x + half, tip - tall)],
            MOVE_CURSOR,
        );
    }

    fn green_ring(&mut self, at: (f32, f32), radius: f32, s: f32) {
        let thickness = (4.0 * s).max(1.0);
        let edge = (2.0 * s).max(1.0);
        self.ring(at, radius, thickness + edge * 2.0, CURSOR_EDGE);
        self.ring(at, radius, thickness, MOVE_CURSOR);
    }

    /// Four green arrows around a point, each given by how far out its tip and its base are:
    /// the tip nearer for arrows pointing in, farther for arrows pointing out.
    fn four_arrows(&mut self, (x, y): (f32, f32), (tip, base): (f32, f32), s: f32) {
        let edge = (2.0 * s).max(1.0);
        let half = 6.5 * s;
        let beyond = edge * 1.8 * (tip - base).signum();
        for arrow in 0..4 {
            let (sin, cos) = (arrow as f32 * FRAC_PI_2).sin_cos();
            // Measured out from the center and sideways from the arrow's middle.
            let at = |out: f32, side: f32| (x + cos * out - sin * side, y + sin * out + cos * side);
            self.triangle(
                [
                    at(tip + beyond, 0.0),
                    at(base - beyond * 0.6, -half - edge * 1.6),
                    at(base - beyond * 0.6, half + edge * 1.6),
                ],
                CURSOR_EDGE,
            );
            self.triangle([at(tip, 0.0), at(base, -half), at(base, half)], MOVE_CURSOR);
        }
    }

    fn attack_cursor(&mut self, (x, y): (f32, f32), s: f32, seconds: f32) {
        /// Radians per second the arrows turn.
        const SPIN: f32 = 1.5;
        let (outer, inner) = (20.0 * s, 10.0 * s);
        let thickness = (4.0 * s).max(1.0);
        let edge = (2.0 * s).max(1.0);
        for r in [outer, inner] {
            self.ring((x, y), r, thickness + edge * 2.0, CURSOR_EDGE);
            self.ring((x, y), r, thickness, ATTACK_CURSOR);
        }
        self.circle(x, y, 3.0 * s + edge, CURSOR_EDGE);
        self.circle(x, y, 3.0 * s, ATTACK_CURSOR);
        for arrow in 0..4 {
            let angle = seconds * SPIN + arrow as f32 * FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            // Measured out from the center and sideways from the arrow's middle.
            let at = |out: f32, side: f32| (x + cos * out - sin * side, y + sin * out + cos * side);
            let (tip, base, half) = (outer + 5.0 * s, outer + 19.0 * s, 8.0 * s);
            self.triangle(
                [
                    at(tip - edge * 2.0, 0.0),
                    at(base + edge, -half - edge * 1.6),
                    at(base + edge, half + edge * 1.6),
                ],
                CURSOR_EDGE,
            );
            self.triangle(
                [at(tip, 0.0), at(base, -half), at(base, half)],
                ATTACK_CURSOR,
            );
        }
    }
}

pub fn clear_color(game: &Game) -> [f32; 3] {
    if game.in_battle || matches!(game.screen, Screen::Intro | Screen::Movie(_)) {
        [INTRO_CLEAR[0], INTRO_CLEAR[1], INTRO_CLEAR[2]]
    } else {
        CLEAR
    }
}

/// All that is drawn on the screen and as effects, with the solid world left out.
#[cfg(test)]
pub fn build(game: &Game) -> Vec<Vertex> {
    let frame = build_frame(game);
    let effects = frame.effects.iter().map(|v| Vertex {
        x: v.x,
        y: v.y,
        r: v.r,
        g: v.g,
        b: v.b,
        a: v.a,
    });
    effects.chain(frame.over.iter().copied()).collect()
}

/// The way to the sun of the map being fought on.
fn sun_of(game: &Game) -> [f32; 3] {
    battlefield::terrain(game.battle_map).light().sun
}

/// The whole frame: the battlefield, if it shows, and what goes over it.
pub fn build_frame(game: &Game) -> Frame {
    let terrain = battlefield::terrain(game.battle_map);
    let light = terrain.light();
    let canvas = canvas_of(game);
    let view = battlefield::View::new(game.camera, game.width, game.height);
    let focus = (game.camera.x, game.camera.z);
    Frame {
        lights: if canvas.ground.is_empty() {
            Vec::new()
        } else {
            lights_of(game)
        },
        ground: canvas.ground,
        decals: canvas.decals,
        material: terrain.material(),
        objects: canvas.objects,
        casters: canvas.casters,
        effects: canvas.effects,
        over: canvas.vertices,
        time: game.battle_time(),
        view: view.matrix(),
        mirror: view.mirror_matrix(),
        sun_view: view.sun_matrix(focus, light.sun, game.width, game.height),
        camera: view.position(),
        sun: light.sun,
        ambient: light.ambient,
        sunlight: light.sunlight,
        wash: original::wash_reach(game.battle_map),
        size: (game.width, game.height),
    }
}

/// What burns on the map right now and lights what is around it: eruptions, flickering, then
/// explosions, the newest first.
fn lights_of(game: &Game) -> Vec<PointLight> {
    let seen = |(x, _, z): (f32, f32, f32)| game.sight_at(x, z) == Sight::Visible;
    let eruptions = game
        .combat
        .eruptions
        .iter()
        .filter(|eruption| seen(eruption.at))
        .map(|eruption| {
            let flicker = 0.85 + 0.15 * (game.battle_time() * 23.0 + eruption.at.0 * 0.0221).sin();
            let light = eruption.strength() * 1.8 * flicker;
            PointLight {
                place: [eruption.at.0, eruption.at.1 + 90.51, eruption.at.2],
                radius: 497.8,
                color: [light, light * 0.4, light * 0.1],
                unused: 0.0,
            }
        });
    let blasts = game
        .combat
        .blasts
        .iter()
        .rev()
        .filter(|blast| seen(blast.at))
        .map(|blast| {
            let light = blast.life * blast.life * 1.3;
            PointLight {
                place: [blast.at.0, blast.at.1 + 27.15, blast.at.2],
                radius: blast.radius * 3.0 + 67.88,
                color: [light, light * 0.55, light * 0.2],
                unused: 0.0,
            }
        });
    let fires = game
        .units
        .iter()
        .filter(|unit| unit.wear() > units::BURNS_PAST && game.shows(unit))
        .map(|unit| {
            let flicker = 0.8 + 0.2 * (game.battle_time() * 17.0 + unit.id as f32).sin();
            PointLight {
                place: [unit.x, unit.pick_height() + 22.63, unit.z],
                radius: 135.8 + unit.radius() * 2.0,
                color: [0.9 * flicker, 0.42 * flicker, 0.1 * flicker],
                unused: 0.0,
            }
        });
    // Gunfire goes last: it is many and brief, and must not put out what burns on.
    let shots = game
        .combat
        .flares
        .iter()
        .rev()
        .filter(|flare| seen(flare.at))
        .map(|flare| {
            let light = flare.life * flare.size * 0.07;
            PointLight {
                place: [flare.at.0, flare.at.1 + 9.051, flare.at.2],
                radius: 54.31 + flare.size * 6.336,
                color: [light, light * 0.68, light * 0.3],
                unused: 0.0,
            }
        });
    eruptions
        .chain(blasts)
        .chain(fires)
        .chain(shots)
        .take(MAX_LIGHTS)
        .collect()
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit_vector(a: [f32; 3]) -> [f32; 3] {
    let length = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-6);
    [a[0] / length, a[1] / length, a[2] / length]
}

/// A point this far along a way from another. Spelled out, as it is worked out for corners of
/// models by the thousand every frame.
fn offset(from: [f32; 3], way: [f32; 3], by: f32) -> [f32; 3] {
    [
        from[0] + way[0] * by,
        from[1] + way[1] * by,
        from[2] + way[2] * by,
    ]
}

fn canvas_of(game: &Game) -> Canvas {
    let mut canvas = Canvas::default();
    let s = ui_scale(game.height);
    let cx = game.width / 2.0;

    match game.screen {
        Screen::Intro | Screen::Movie(_) if game.is_playing_movie() => {}
        Screen::Intro => {
            canvas.text_centered("INTRO PLACEHOLDER", cx, game.height / 2.0, 6.0 * s, QUIET);
        }
        Screen::Movie(faction) => {
            let text = format!("{} movie placeholder", faction.label());
            canvas.text_centered(&text, cx, game.height / 2.0, 6.0 * s, QUIET);
        }
        Screen::Loading => {
            let (bar_w, bar_h) = (800.0 * s, 32.0 * s);
            let (bar_x, bar_y) = (cx - bar_w / 2.0, game.height / 2.0 + 40.0 * s);
            canvas.text_centered("Loading", cx, game.height / 2.0 - 30.0 * s, 6.0 * s, TEXT);
            canvas.rect(bar_x, bar_y, bar_w, bar_h, BUTTON);
            canvas.rect(bar_x, bar_y, bar_w * game.loading_progress(), bar_h, GOLD);
        }
        Screen::MainMenu
        | Screen::SinglePlayerMenu
        | Screen::Options
        | Screen::Credits
        | Screen::FactionSelect
        | Screen::SkirmishSetup
        | Screen::EscapeMenu
        | Screen::BattleStats
        | Screen::InGame => {
            if game.in_battle {
                draw_battle(&mut canvas, game);
                if game.screen != Screen::InGame {
                    canvas.rect(0.0, 0.0, game.width, game.height, DIM);
                }
            }
            if matches!(game.screen, Screen::MainMenu | Screen::SinglePlayerMenu) && !game.in_battle
            {
                // Rising from the bottom of the screen, leaving the title on clear sky.
                let (y, radius) = (game.height * 0.66, game.height * 0.48);
                backdrop::draw(
                    &mut canvas,
                    (game.width, game.height),
                    (cx, y, radius),
                    game.menu_seconds(),
                );
                planet::draw_sandy(&mut canvas.vertices, cx, y, radius, game.menu_seconds());
                canvas.text_left(
                    VERSION,
                    24.0 * s,
                    30.0 * s,
                    small_text_px(game.height),
                    TEXT,
                );
            }
            let title = match game.screen {
                Screen::Options => Some("Main Options"),
                Screen::Credits => Some("Credits"),
                Screen::FactionSelect => Some("Choose Faction"),
                Screen::InGame
                | Screen::SkirmishSetup
                | Screen::EscapeMenu
                | Screen::BattleStats => None,
                _ => Some(TITLE),
            };
            if let Some(title) = title {
                canvas.text_centered(title, cx, 170.0 * s, 16.0 * s, GOLD);
            }
            if game.screen == Screen::InGame {
                if game.key_paused {
                    canvas.rect(0.0, 0.0, game.width, game.height, DIM);
                    canvas.text_centered("Paused", cx, game.height / 2.0, 12.0 * s, GOLD);
                }
                if game.stats_open {
                    draw_diplomacy(&mut canvas, game);
                }
                if let Some(outcome) = game.outcome {
                    let (text, color) = outcome_look(outcome);
                    canvas.text_centered(text, cx, game.height * 0.4, 16.0 * s, color);
                    let hint_y = game.height * 0.4 + 110.0 * s;
                    canvas.text_centered(
                        "Esc to continue",
                        cx,
                        hint_y,
                        small_text_px(game.height),
                        TEXT,
                    );
                }
            }
            if game.screen == Screen::BattleStats {
                draw_battle_stats(&mut canvas, game);
            }
            if game.screen == Screen::SkirmishSetup {
                draw_skirmish_setup(&mut canvas, game);
            }
            if game.screen == Screen::Options {
                draw_options_panel(&mut canvas, game);
            }
            if game.screen == Screen::Credits {
                canvas.text_centered("Credits placeholder", cx, game.height / 2.0, 4.0 * s, QUIET);
            }
            for b in buttons(game.screen, game.width, game.height) {
                draw_button(&mut canvas, game, b);
            }
            if game.confirming.is_some() {
                draw_message_box(&mut canvas, game);
            }
            if game.dropdown_open {
                // Last, so the list covers the controls below the dropdown.
                for item in map_items(game.map_folder, game.map_scroll, game.width, game.height) {
                    draw_field(&mut canvas, game, item);
                    let chosen = [
                        Action::SelectMap(game.skirmish.map),
                        Action::MapFolder(game.map_folder),
                    ];
                    let color = if chosen.contains(&item.action) {
                        GOLD
                    } else {
                        TEXT
                    };
                    let px = small_text_px(game.height);
                    canvas.text_left(
                        item.label,
                        item.x + 16.0 * s,
                        item.y + item.h / 2.0,
                        px,
                        color,
                    );
                }
            }
        }
    }

    if let Some(tip) = game.tip() {
        draw_tip(&mut canvas, game, tip);
    }
    let look = game.cursor_look();
    let cursor_drawn = canvas.vertices.len();
    match look {
        CursorLook::Attack | CursorLook::Harvest => {
            canvas.attack_cursor(game.mouse, s, game.screen_elapsed())
        }
        CursorLook::Heal => canvas.heal_cursor(game.mouse, s),
        CursorLook::Carry => canvas.carry_cursor(game.mouse, s),
        CursorLook::Move => canvas.move_cursor(game.mouse, s, game.screen_elapsed()),
        CursorLook::Deploy => canvas.deploy_cursor(game.mouse, s, game.screen_elapsed()),
        CursorLook::Undeploy => canvas.undeploy_cursor(game.mouse, s, game.screen_elapsed()),
        CursorLook::Land => canvas.land_cursor(game.mouse, s, game.screen_elapsed()),
        CursorLook::March => canvas.march_cursor(game.mouse, s, game.screen_elapsed()),
        CursorLook::Board => canvas.board_cursor(game.mouse, s, game.screen_elapsed()),
        CursorLook::Sell => canvas.sell_cursor(game.mouse, s, game.screen_elapsed()),
        CursorLook::Arrow => canvas.cursor(game.mouse, s),
    }
    let lies_on_map = matches!(
        look,
        CursorLook::Attack
            | CursorLook::Harvest
            | CursorLook::Carry
            | CursorLook::Move
            | CursorLook::March
            | CursorLook::Deploy
            | CursorLook::Undeploy
    );
    if lies_on_map {
        lay_cursor_on_map(&mut canvas, game, cursor_drawn);
    }
    canvas
}

/// Lays what was just drawn of a cursor flat on the land under it, as wide as it was drawn.
fn lay_cursor_on_map(canvas: &mut Canvas, game: &Game, drawn: usize) {
    let terrain = battlefield::terrain(game.battle_map);
    let view = game.view();
    let Some((x, z)) = game.land_under_cursor() else {
        return;
    };
    let y = terrain.height_at(x, z);
    let (Some(middle), Some(east)) = (view.project(x, y, z), view.project(x + 45.25, y, z)) else {
        return;
    };
    let world_per_pixel = 45.25 / (east.0 - middle.0).abs().max(1.0);
    for vertex in &mut canvas.vertices[drawn..] {
        let (dx, dz) = (vertex.x - game.mouse.0, vertex.y - game.mouse.1);
        let (px, pz) = (x + dx * world_per_pixel, z + dz * world_per_pixel);
        if let Some((screen_x, screen_y)) = view.project(px, y, pz) {
            (vertex.x, vertex.y) = (screen_x, screen_y);
        }
    }
}

/// A small label below and to the right of the cursor, kept on the screen.
fn draw_tip(canvas: &mut Canvas, game: &Game, text: &str) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let pad = 6.0 * s;
    let border = border_px(game.height);
    let (w, h) = (
        font::text_width(text) * px + pad * 2.0,
        font::CAP_HEIGHT * px + pad * 2.0,
    );
    let x = (game.mouse.0 + 26.0 * s).min(game.width - w);
    let y = (game.mouse.1 + 34.0 * s).min(game.height - h);
    canvas.rect(
        x - border,
        y - border,
        w + border * 2.0,
        h + border * 2.0,
        BORDER,
    );
    canvas.rect(x, y, w, h, PANEL);
    canvas.text_left(text, x + pad, y + h / 2.0, px, TEXT);
}

fn draw_button(canvas: &mut Canvas, game: &Game, b: Button) {
    let s = ui_scale(game.height);
    let border = border_px(game.height);
    let hovered = game.hovered();
    let selected = b.action == Action::Tab(game.tab);
    let fill = if selected {
        PANEL
    } else {
        button_fill(b.action, hovered, game.pressed)
    };
    let edge = if selected { GOLD } else { BORDER };
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    if let Action::Faction(faction) = b.action {
        let ring = if hovered == Some(b.action) {
            GOLD
        } else {
            BORDER
        };
        draw_planet(canvas, game, faction, button_circle(b), ring);
        // The name goes below: the moving surface would make it hard to read on top.
        canvas.text_centered(b.label, cx, b.y + b.h + 28.0 * s, 4.0 * s, TEXT);
        return;
    }
    // The sidebar's small square buttons keep solid edges: their labels and symbols sit
    // there, over whatever the battlefield shows.
    if matches!(b.action, Action::BuildCell(_) | Action::HudSmall(_)) {
        canvas.bordered_rect((b.x, b.y, b.w, b.h), border, edge, fill);
    } else {
        let banner = match (
            hovered == Some(b.action) || selected,
            game.pressed == Some(b.action),
        ) {
            (true, true) => BANNER_PRESSED,
            (true, false) => BANNER_HOVER,
            _ => BANNER_BLUE,
        };
        canvas.banner(b.x, b.y, b.w, b.h, border, banner);
    }
    let px = match b.action {
        Action::Tab(_) | Action::HudSmall(_) | Action::HudWide(_) => small_text_px(game.height),
        _ => 4.0 * s,
    };
    canvas.text_centered(b.label, cx, cy, px, TEXT);
}

fn draw_message_box(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let border = border_px(game.height);
    let (x, y, w, h) = modal_rect(game.width, game.height);
    canvas.rect(0.0, 0.0, game.width, game.height, DIM);
    canvas.bordered_rect((x, y, w, h), border, GOLD, PANEL);
    canvas.text_centered(
        CONFIRM_MESSAGE,
        x + w / 2.0,
        y + 90.0 * s,
        small_text_px(game.height),
        TEXT,
    );
    for b in game.buttons() {
        draw_button(canvas, game, b);
    }
}

fn draw_options_panel(canvas: &mut Canvas, game: &Game) {
    let px = small_text_px(game.height);
    let border = border_px(game.height);
    let (x, y, w, h) = options_panel(game.width, game.height);
    canvas.bordered_rect((x, y, w, h), border, BORDER, PANEL);

    if let Some(text) = game.tab.placeholder() {
        canvas.text_centered(text, x + w / 2.0, y + h / 2.0, px, QUIET);
    }

    draw_sliders(canvas, game);
    draw_checkboxes(canvas, game, game.checkboxes());
}

/// A faction's planet inside a thin ring filling the given circle.
fn draw_planet(
    canvas: &mut Canvas,
    game: &Game,
    faction: Faction,
    (cx, cy, r): (f32, f32, f32),
    ring: Color,
) {
    let border = border_px(game.height);
    canvas.circle(cx, cy, r, ring);
    planet::draw(
        &mut canvas.vertices,
        faction,
        cx,
        cy,
        r - border,
        game.screen_elapsed(),
    );
}

fn outcome_look(outcome: Outcome) -> (&'static str, Color) {
    match outcome {
        Outcome::Victory => ("Victory", GOLD),
        Outcome::Defeat => ("Defeat", DEFEAT_RED),
    }
}

/// What was killed and lost in the battle just ended, by kind.
fn draw_battle_stats(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = 4.0 * s;
    let cx = game.width / 2.0;
    canvas.text_centered("Battle Statistics", cx, 150.0 * s, 8.0 * s, GOLD);
    if let Some(outcome) = game.outcome {
        let (text, color) = outcome_look(outcome);
        canvas.text_centered(text, cx, 280.0 * s, 6.0 * s, color);
    }
    let (kind_x, killed_x, lost_x) = (cx - 330.0 * s, cx + 130.0 * s, cx + 340.0 * s);
    let header_y = 420.0 * s;
    canvas.text_centered("Killed", killed_x, header_y, px, QUIET);
    canvas.text_centered("Lost", lost_x, header_y, px, QUIET);
    let rows = [
        ("Infantry", game.killed.infantry, game.lost.infantry),
        ("Vehicles", game.killed.vehicles, game.lost.vehicles),
        ("Aircraft", game.killed.aircraft, game.lost.aircraft),
        ("Buildings", game.killed.buildings, game.lost.buildings),
    ];
    for (i, (name, killed, lost)) in rows.into_iter().enumerate() {
        let y = header_y + (i + 1) as f32 * 90.0 * s;
        canvas.text_left(name, kind_x, y, px.round().max(1.0), TEXT);
        canvas.text_centered(&killed.to_string(), killed_x, y, px, TEXT);
        canvas.text_centered(&lost.to_string(), lost_x, y, px, TEXT);
    }
}

fn button_circle(b: Button) -> (f32, f32, f32) {
    (b.x + b.w / 2.0, b.y + b.h / 2.0, b.w / 2.0)
}

/// The terrain, the faction name in a campaign, and the minimap.
fn draw_battle(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let view = battlefield::View::new(game.camera, game.width, game.height);
    let fog = game.fog.as_ref();
    if !original::draw_land(
        &mut canvas.ground,
        game.battle_map,
        fog,
        &view,
        game.width,
        game.height,
    ) {
        draw_plain_land(
            &mut canvas.ground,
            battlefield::terrain(game.battle_map),
            fog,
        );
    }
    original::draw_spice(
        &mut canvas.decals,
        battlefield::terrain(game.battle_map),
        fog,
        &view,
        game.width,
        game.height,
    );
    draw_map_objects(canvas, game);
    draw_combat(canvas, game, &view);
    draw_tower_ranges(canvas, game, &view);
    draw_strike_aim(canvas, game, &view);
    if !game.aiming_strike {
        draw_selection_box(canvas, game);
    }
    if let Some(faction) = game.faction {
        let label = format!(
            "{} - Mission {}",
            faction.label(),
            game.campaign_mission + 1
        );
        canvas.text_centered(&label, game.width / 2.0, 40.0 * s, 4.0 * s, TEXT);
    }
    draw_energy_gauge(canvas, game);
    draw_battle_minimap(canvas, game);
    draw_money(canvas, game);
    draw_sidebar(canvas, game);
    draw_chat(canvas, game);
}

/// A map's ground when the disc's shape of it cannot be drawn: its cells, each in its color on
/// the radar, at their heights.
fn draw_plain_land(
    ground: &mut Vec<WorldVertex>,
    terrain: &battlefield::Terrain,
    fog: Option<&Fog>,
) {
    /// How many cells across each square drawn is.
    const STEP: usize = 4;
    let (width, height) = terrain.cells();
    for v in (0..height).step_by(STEP) {
        for u in (0..width).step_by(STEP) {
            let color = terrain.radar_at(u, v);
            let corner = |du: usize, dv: usize| {
                let (x, z) = battlefield::grid_to_world(
                    (u + du).min(width) as f32,
                    (v + dv).min(height) as f32,
                );
                let mut corner =
                    WorldVertex::solid((x, terrain.height_at(x, z), z), [0.0, 1.0, 0.0], color);
                corner.seen = battlefield::seen_at(fog, x, z);
                corner
            };
            let (a, b, c, d) = (
                corner(0, 0),
                corner(STEP, 0),
                corner(STEP, STEP),
                corner(0, STEP),
            );
            ground.extend([a, b, c, a, c, d]);
        }
    }
}

/// Units as simple boxes, farthest first, for a backend with no depth to go by. Whatever
/// flies goes last, with its ring down on the ground below.
fn draw_map_objects(canvas: &mut Canvas, game: &Game) {
    let view = battlefield::View::new(game.camera, game.width, game.height);
    draw_poison(canvas, game, &view);
    draw_order_lines(canvas, game, &view);
    canvas.watchers = Some(models::Watchers {
        eye: view.position(),
        sun: sun_of(game),
        camera: true,
        mirrored: false,
    });
    let is_marked = |unit: &Unit| unit.selected || game.in_selection_drag(unit);
    let mut flying: Vec<&Unit> = game
        .units
        .iter()
        .filter(|u| u.altitude > 0.0 && game.shows(u))
        .collect();
    flying.sort_by(|a, b| (a.x + a.z).total_cmp(&(b.x + b.z)));
    for unit in &flying {
        if is_marked(unit) {
            draw_selected_ring(canvas, game, &view, unit);
        }
    }
    // Farthest first: the screen's bottom lies where x and z both grow.
    let mut order: Vec<&Unit> = game
        .units
        .iter()
        .filter(|u| u.altitude <= 0.0 && game.shows(u))
        .collect();
    order.sort_by(|a, b| (a.x + a.z).total_cmp(&(b.x + b.z)));
    draw_on_threads(canvas, &order, |canvas, &unit| {
        if is_marked(unit) {
            draw_selected_ring(canvas, game, &view, unit);
        }
        draw_unit(canvas, game, &view, unit);
        draw_health_bar(canvas, game, &view, unit);
        draw_pips(canvas, game, &view, unit);
        draw_medals(canvas, game, &view, unit);
        draw_group_number(canvas, game, &view, unit);
    });
    for wreck in game
        .combat
        .wrecks
        .iter()
        .filter(|w| game.sight_at(w.unit.x, w.unit.z) == Sight::Visible)
    {
        let seen = seen_by(game, &view, &wreck.unit);
        draw_seen_model(canvas, game, &wreck.unit, seen);
        if matches!(seen, Seen::Near | Seen::Far) {
            draw_flames(canvas, game, &view, &wreck.unit);
        }
    }
    let terrain = battlefield::terrain(game.battle_map);
    for found in game
        .crates
        .iter()
        .filter(|c| game.sight_at(c.x, c.z) == Sight::Visible)
    {
        models::draw_crate(
            canvas,
            (found.x, terrain.height_at(found.x, found.z), found.z),
        );
    }
    // A body reaches no farther than this from its first joint.
    const RAGDOLL_REACH: f32 = 67.88;
    let ragdolls: Vec<_> = game
        .combat
        .ragdolls
        .iter()
        .filter(|r| {
            game.sight_at(r.joints[0].0, r.joints[0].2) == Sight::Visible
                && view.ball_on_screen(r.joints[0], RAGDOLL_REACH).is_some()
        })
        .collect();
    draw_on_threads(canvas, &ragdolls, |canvas, ragdoll| {
        let [r, g, b, _] = team_color(ragdoll.unit.color);
        models::draw_ragdoll(canvas, ragdoll, [r, g, b]);
    });
    for shatter in game
        .combat
        .shatters
        .iter()
        .filter(|s| game.sight_at(s.unit.x, s.unit.z) == Sight::Visible)
    {
        draw_shatter(canvas, game, &view, shatter);
    }
    draw_placing_squares(canvas, game, &view);
    draw_placing_arrow(canvas, game, &view);
    draw_on_threads(canvas, &flying, |canvas, &unit| {
        draw_carried(canvas, game, &view, unit);
        draw_unit(canvas, game, &view, unit);
        draw_health_bar(canvas, game, &view, unit);
        draw_pips(canvas, game, &view, unit);
        draw_medals(canvas, game, &view, unit);
        draw_group_number(canvas, game, &view, unit);
    });
    canvas.watchers = None;
}

/// Draws these things in their order, sharing them out over the CPU's cores once there are
/// enough of them to be worth it. Each share is drawn on a canvas of its own, and the shares
/// are put together in order, so what comes out is the same as drawing them one by one.
fn draw_on_threads<T: Sync>(
    canvas: &mut Canvas,
    things: &[T],
    draw: impl Fn(&mut Canvas, &T) + Sync,
) {
    /// How many things a thread must have to be worth starting.
    const THINGS_PER_THREAD: usize = 12;
    const THREADS_AT_MOST: usize = 8;
    static CORES: OnceLock<usize> = OnceLock::new();
    let cores = *CORES.get_or_init(|| thread::available_parallelism().map_or(1, usize::from));
    let threads = cores
        .min(THREADS_AT_MOST)
        .min(things.len() / THINGS_PER_THREAD);
    if threads < 2 {
        things.iter().for_each(|thing| draw(canvas, thing));
        return;
    }
    let (watchers, detail) = (canvas.watchers, canvas.detail);
    let draw = &draw;
    let shares: Vec<Canvas> = thread::scope(|scope| {
        let workers: Vec<_> = things
            .chunks(things.len().div_ceil(threads))
            .map(|share| {
                scope.spawn(move || {
                    let mut own = Canvas {
                        watchers,
                        detail,
                        ..Canvas::default()
                    };
                    share.iter().for_each(|thing| draw(&mut own, thing));
                    own
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().expect("a drawing thread panicked"))
            .collect()
    });
    canvas
        .objects
        .reserve(shares.iter().map(|s| s.objects.len()).sum());
    for mut share in shares {
        canvas.objects.append(&mut share.objects);
        canvas.casters.append(&mut share.casters);
        canvas.effects.append(&mut share.effects);
        canvas.vertices.append(&mut share.vertices);
    }
}

/// Something small in the air that shows only by its shadow on the ground.
fn cast_only(canvas: &mut Canvas, (x, y, z): Point, half: f32) {
    let drawn = canvas.objects.len();
    draw_turned_box(
        canvas,
        (x, z, half, half, 0.0),
        (y - half, y + half),
        [0.0; 3],
    );
    let moved: Vec<WorldVertex> = canvas.objects.drain(drawn..).collect();
    canvas.casters.extend(moved);
}

/// How far over the ground what lies on it goes, to show over it in the depth test.
const OVER_GROUND: f32 = 1.81;

/// A flat disc at one height, seen in perspective, fading out towards its rim if asked.
fn draw_flat_disc(
    canvas: &mut Canvas,
    view: &battlefield::View,
    (x, y, z): Point,
    radius: f32,
    color: [f32; 3],
    alphas: (f32, f32),
) {
    draw_disc(canvas, view, (x, z), radius, color, alphas, |_, _| y);
}

/// A disc lying on the ground, following the terrain under it.
/// The little squares a building being placed would take, each half as wide as its grid cell
/// and in its middle, lying on the ground: green where it can go, yellow for the way out of
/// what makes units, and red where it cannot go.
fn draw_placing_squares(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    /// How far in from its grid lines each square stops.
    const INSET: f32 = 0.25;
    const ALPHA: f32 = 0.6;
    let terrain = battlefield::terrain(game.battle_map);
    for ((u, v), square) in game.placing_squares() {
        let color = match square {
            PlaceSquare::Free => PLACE_OK,
            PlaceSquare::Exit => PLACE_EXIT,
            PlaceSquare::Blocked => PLACE_NOT,
        };
        let color = with_alpha(color, ALPHA);
        let corner = |along: f32, across: f32| {
            let (x, z) = battlefield::grid_to_world(u as f32 + along, v as f32 + across);
            (
                (x, terrain.height_at(x, z).max(0.0) + OVER_GROUND, z),
                color,
            )
        };
        let (near, far) = (INSET, 1.0 - INSET);
        let [a, b, c, d] = [
            corner(near, near),
            corner(far, near),
            corner(far, far),
            corner(near, far),
        ];
        canvas.deep_fill(view, [a, b, c]);
        canvas.deep_fill(view, [a, c, d]);
    }
}

/// While a drag turns the building being placed, a small arrow lying on the ground just out
/// from the edge of its squares, pointing the way it will look.
fn draw_placing_arrow(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    /// How long it is, about a square and a half, how long and wide its head is, and how wide
    /// its shaft.
    const LENGTH: f32 = 1.5 * battlefield::CELL_SIDE;
    const HEAD: (f32, f32) = (0.45, 0.25);
    const SHAFT: f32 = 0.08;
    const COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.8];
    let Some((plot, (out_x, out_z))) = game.placing_arrow() else {
        return;
    };
    let terrain = battlefield::terrain(game.battle_map);
    let (x, z) = plot.center();
    let corners = [
        (0, 0),
        (plot.cols, 0),
        (plot.cols, plot.rows),
        (0, plot.rows),
    ];
    let edge = corners
        .map(|(u, v)| {
            let (cx, cz) = battlefield::grid_to_world((plot.u + u) as f32, (plot.v + v) as f32);
            (cx - x) * out_x + (cz - z) * out_z
        })
        .into_iter()
        .fold(0.0, f32::max);
    let point = |along: f32, across: f32| {
        let (px, pz) = (
            x + out_x * along - out_z * across,
            z + out_z * along + out_x * across,
        );
        (
            (
                px,
                terrain.height_at(px, pz).max(0.0) + OVER_GROUND * 2.0,
                pz,
            ),
            COLOR,
        )
    };
    let tip = edge + LENGTH;
    let neck = tip - HEAD.0;
    let [a, b, c, d] = [
        point(edge, -SHAFT),
        point(neck, -SHAFT),
        point(neck, SHAFT),
        point(edge, SHAFT),
    ];
    canvas.deep_fill(view, [a, b, c]);
    canvas.deep_fill(view, [a, c, d]);
    canvas.deep_fill(
        view,
        [point(neck, -HEAD.1), point(tip, 0.0), point(neck, HEAD.1)],
    );
}

fn draw_draped_disc(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    center: (f32, f32),
    radius: f32,
    color: [f32; 3],
    alphas: (f32, f32),
) {
    let terrain = battlefield::terrain(game.battle_map);
    draw_disc(canvas, view, center, radius, color, alphas, |x, z| {
        terrain.height_at(x, z).max(0.0) + OVER_GROUND
    });
}

fn draw_disc(
    canvas: &mut Canvas,
    view: &battlefield::View,
    (x, z): (f32, f32),
    radius: f32,
    color: [f32; 3],
    (center_alpha, rim_alpha): (f32, f32),
    height: impl Fn(f32, f32) -> f32,
) {
    const WEDGES: usize = 40;
    let rim = |i: usize| {
        let (sin, cos) = (i as f32 / WEDGES as f32 * TAU).sin_cos();
        let (px, pz) = (x + radius * cos, z + radius * sin);
        (px, height(px, pz), pz)
    };
    let (middle, rim_color) = (
        with_alpha(color, center_alpha),
        with_alpha(color, rim_alpha),
    );
    for i in 0..WEDGES {
        canvas.deep_fill(
            view,
            [
                ((x, height(x, z), z), middle),
                (rim(i), rim_color),
                (rim(i + 1), rim_color),
            ],
        );
    }
}

/// Poisoned land glows green, breathing gently, and fades during its last seconds.
fn draw_poison(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    for zone in game
        .combat
        .poison
        .iter()
        .filter(|z| game.sight_at(z.x, z.z) == Sight::Visible)
    {
        let fade = (zone.life / 5.0).min(1.0);
        let breath = 0.85 + 0.15 * (game.battle_time() * 2.0).sin();
        let alpha = POISON_ALPHA * fade * breath;
        draw_draped_disc(
            canvas,
            game,
            view,
            (zone.x, zone.z),
            STRIKE_RADIUS,
            POISON_FILL,
            (alpha, alpha * 1.4),
        );
        draw_radiation_motes(canvas, game, view, zone, fade);
    }
}

/// Faint green specks of radiation drifting up off poisoned land, each rising and fading away
/// before it starts again from the ground.
fn draw_radiation_motes(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    zone: &crate::weapons::PoisonZone,
    fade: f32,
) {
    const MOTES: usize = 24;
    const CLIMB: f32 = 113.1;
    let terrain = battlefield::terrain(game.battle_map);
    let time = game.battle_time();
    for i in 0..MOTES {
        let seed = i as f32 * 7.31 + zone.x * 0.002873 + zone.z * 0.003757;
        let angle = seed.sin() * TAU;
        let out = STRIKE_RADIUS * (0.5 + 0.5 * (seed * 3.7).sin()).sqrt();
        let (mx, mz) = (zone.x + angle.cos() * out, zone.z + angle.sin() * out);
        let phase = (time * 0.35 + (seed * 1.9).sin() * 0.5 + 0.5).fract();
        let y = terrain.height_at(mx, mz) + phase * CLIMB;
        let glow = POISON_FILL.map(|c| c * 0.35 * fade * (1.0 - phase) * phase * 4.0);
        draw_glare(canvas, view, (mx, y, mz), 11.31, glow);
    }
}

/// Shots in the air and explosions, hidden by what is solid in front of them.
fn draw_combat(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    let s = ui_scale(game.height);
    // What happens where the player cannot see stays hidden.
    let seen = |(x, _, z): (f32, f32, f32)| game.sight_at(x, z) == Sight::Visible;
    for beam in game
        .combat
        .beams
        .iter()
        .filter(|b| seen(b.from) || seen(b.to))
    {
        let (glow, core) = if beam.repair {
            (REPAIR_GLOW, REPAIR_CORE)
        } else {
            (LASER_GLOW, LASER_CORE)
        };
        canvas.deep_line(view, beam.from, beam.to, 9.0 * s, faded(glow, beam.life));
        let thin = (2.5 * s).max(1.0);
        canvas.deep_line(view, beam.from, beam.to, thin, faded(core, beam.life));
    }
    for spray in game.combat.sprays.iter().filter(|s| seen(s.from)) {
        draw_spray(canvas, view, spray);
    }
    for flamelet in game.combat.flamelets.iter().filter(|f| seen(f.position)) {
        draw_flamelet(canvas, view, flamelet);
    }
    for streak in game.combat.streaks.iter().filter(|s| seen(s.from)) {
        // A short bright dash racing from the gun to what it hit, with a fainter line behind.
        let at = |t: f32| {
            (
                streak.from.0 + (streak.to.0 - streak.from.0) * t,
                streak.from.1 + (streak.to.1 - streak.from.1) * t,
                streak.from.2 + (streak.to.2 - streak.from.2) * t,
            )
        };
        let head = (1.0 - streak.life) * 1.3;
        let (front, back) = (head.min(1.0), (head - 0.4).clamp(0.0, 1.0));
        let thick = (3.0 * s).max(1.5);
        canvas.deep_line(
            view,
            at(0.0),
            at(front),
            thick * 0.5,
            faded(GUN_TRAIL, streak.life),
        );
        canvas.deep_line(view, at(back), at(front), thick, GUN_STREAK);
    }
    for flare in game.combat.flares.iter().filter(|f| seen(f.at)) {
        draw_flare(canvas, view, flare, s);
    }
    for missile in game.combat.missiles.iter().filter(|m| seen(m.position)) {
        let (x, y, z) = missile.position;
        let (dx, dy, dz) = missile.direction;
        let back = |by: f32| (x - dx * by, y - dy * by, z - dz * by);
        if missile.is_plasma() {
            canvas.deep_line(
                view,
                back(0.0),
                back(49.78),
                (5.0 * s).max(2.0),
                PLASMA_TRAIL,
            );
            draw_glare(canvas, view, missile.position, 24.89, [0.25, 0.45, 1.0]);
            draw_glare(canvas, view, missile.position, 9.051, [0.85, 0.92, 1.0]);
            cast_only(canvas, missile.position, 3.62);
            continue;
        }
        let wide = (4.0 * s).max(1.0);
        canvas.deep_line(view, back(15.84), back(36.2), wide, MISSILE_FLAME);
        canvas.deep_line(view, back(0.0), back(15.84), wide, MISSILE_BODY);
        cast_only(canvas, back(9.051), 2.263);
    }
    for shell in game.combat.shells.iter().filter(|s| seen(s.position())) {
        let (x, y, z) = shell.position();
        if let Some(turn) = shell.cistern_turn() {
            let (sin, cos) = turn.sin_cos();
            let end = |by: f32| (x + cos * by, y + sin * by, z);
            cast_only(canvas, (x, y, z), 3.62);
            canvas.deep_line(
                view,
                end(-7.241),
                end(7.241),
                (7.0 * s).max(3.0),
                SHELL_BODY,
            );
            let band = with_alpha(GAS_COLOR, 1.0);
            canvas.deep_line(view, end(-2.263), end(2.263), (8.0 * s).max(3.5), band);
            continue;
        }
        if shell.is_bomb() {
            // A fat dark body nose first along its fall, with its fins behind.
            let (bx, by, bz) = shell.a_moment_ago();
            let (dx, dy, dz) = (x - bx, y - by, z - bz);
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            let (dx, dy, dz) = if length > 0.004525 {
                (dx / length, dy / length, dz / length)
            } else {
                (0.0, -1.0, 0.0)
            };
            let behind = |by: f32| (x - dx * by, y - dy * by, z - dz * by);
            cast_only(canvas, (x, y, z), 5.431);
            canvas.deep_line(
                view,
                (x, y, z),
                behind(18.1),
                (9.0 * s).max(3.0),
                SHELL_BODY,
            );
            canvas.deep_line(
                view,
                behind(14.48),
                behind(20.36),
                (13.0 * s).max(4.0),
                SHELL_BODY,
            );
            continue;
        }
        if let Some(tracer) = shell.tracer() {
            cast_only(canvas, (x, y, z), 2.263);
            let path = std::iter::once((x, y, z)).chain(tracer);
            let next = tracer.into_iter();
            for (i, (from, to)) in path.zip(next).enumerate() {
                let fade = 0.5 * (1.0 - i as f32 / tracer.len() as f32);
                let thin = (2.0 * s).max(1.0);
                canvas.deep_line(view, from, to, thin, with_alpha([1.0; 3], fade));
            }
            if let Some((px, py, depth)) = view.project_deep(x, y, z) {
                let mark = canvas.vertices.len();
                canvas.circle(px, py, (3.0 * s).max(1.5), with_alpha([1.0; 3], 1.0));
                canvas.deepen(mark, depth);
            }
            continue;
        }
        cast_only(canvas, (x, y, z), 4.525);
        if let Some((px, py, depth)) = view.project_deep(x, y, z) {
            let mark = canvas.vertices.len();
            canvas.circle(px, py, (5.0 * s).max(2.0), SHELL_BODY);
            canvas.deepen(mark, depth);
        }
    }
    for strike in &game.combat.strikes {
        let height = strike.height();
        let (top, bottom) = (
            (strike.x, height + 135.8, strike.z),
            (strike.x, height, strike.z),
        );
        canvas.deep_line(view, top, bottom, 14.0 * s, with_alpha(POISON_FILL, 0.35));
        canvas.deep_line(view, top, bottom, 6.0 * s, MISSILE_BODY);
        cast_only(canvas, (strike.x, height + 67.88, strike.z), 6.788);
        draw_draped_disc(
            canvas,
            game,
            view,
            (strike.x, strike.z),
            STRIKE_RADIUS,
            POISON_FILL,
            (0.0, 0.25),
        );
    }
    for blast in game.combat.blasts.iter().filter(|b| seen(b.at)) {
        let reach = blast.radius * (1.2 - blast.life * 0.6);
        draw_flat_disc(
            canvas,
            view,
            blast.at,
            reach,
            BLAST_COLOR,
            (0.9 * blast.life, 0.1 * blast.life),
        );
        draw_blast_glow(canvas, view, blast);
    }
    for eruption in game.combat.eruptions.iter().filter(|e| seen(e.at)) {
        draw_eruption(canvas, game, view, eruption);
    }
    for mushroom in game.combat.mushrooms.iter().filter(|m| seen(m.at)) {
        draw_mushroom(canvas, view, mushroom);
    }
    for particle in game.combat.particles.iter().filter(|p| seen(p.position)) {
        draw_particle(canvas, view, particle);
    }
    for wisp in game.combat.wisps.iter().filter(|w| seen(w.position)) {
        draw_wisp(canvas, view, wisp);
    }
    for tornado in game.combat.tornados.iter().filter(|t| seen(t.at)) {
        draw_tornado(canvas, game, view, tornado);
    }
}

/// Gunfire leaving a barrel: a burst of light at its end, and flames licking out the way the
/// shot went, a long one with a shorter one to each side.
fn draw_flare(
    canvas: &mut Canvas,
    view: &battlefield::View,
    flare: &crate::weapons::Flare,
    scale: f32,
) {
    /// How far its middle flame reaches, in world units, for a flare of size 1.
    const FLAME_REACH: f32 = 3.168;
    const SIDE_ANGLE: f32 = 0.5;
    let (x, y, z) = flare.at;
    let Some((px, py, depth)) = view.project_deep(x, y, z) else {
        return;
    };
    let (dx, dy, dz) = flare.direction;
    // It shoots out fast and thins as it goes.
    let reach = flare.size * FLAME_REACH * (0.55 + 0.45 * flare.life);
    let (sin, cos) = SIDE_ANGLE.sin_cos();
    for (way_x, way_z, length) in [
        (dx, dz, 1.0),
        (dx * cos - dz * sin, dx * sin + dz * cos, 0.55),
        (dx * cos + dz * sin, dz * cos - dx * sin, 0.55),
    ] {
        let end = (
            x + way_x * reach * length,
            y + dy * reach * length,
            z + way_z * reach * length,
        );
        let Some((ex, ey, end_depth)) = view.project_deep(end.0, end.1, end.2) else {
            continue;
        };
        let (along_x, along_y) = (ex - px, ey - py);
        let width = 0.22 * along_x.hypot(along_y).max(2.0);
        let across = along_x.hypot(along_y).max(0.001);
        let (nx, ny) = (-along_y / across * width, along_x / across * width);
        let hot = [1.0, 0.72, 0.28].map(|c: f32| c * flare.life);
        // Widest a little way along, closing to a point at its end.
        let (mid_x, mid_y) = (px + along_x * 0.35, py + along_y * 0.35);
        let mid_depth = depth + (end_depth - depth) * 0.35;
        let at = |point: (f32, f32), color: [f32; 3], at_depth: f32| {
            EffectVertex::of(Vertex::light(point, color), at_depth)
        };
        let warm = hot.map(|c| c * 0.7);
        let (left, right) = ((mid_x + nx, mid_y + ny), (mid_x - nx, mid_y - ny));
        canvas.effects.extend([
            at((px, py), hot, depth),
            at(left, warm, mid_depth),
            at(right, warm, mid_depth),
            at(left, warm, mid_depth),
            at(right, warm, mid_depth),
            at((ex, ey), [0.0; 3], end_depth),
        ]);
    }
    let size = flare.size * scale;
    let mark = canvas.vertices.len();
    canvas.star(
        (px, py),
        (size, size * 0.35),
        6,
        flare.life,
        with_alpha(FLARE_GLOW, 0.8 * flare.life),
    );
    canvas.circle(px, py, size * 0.35, with_alpha(FLARE_CORE, flare.life));
    canvas.deepen(mark, depth);
}

/// Lava bursting up from a vent: a glowing pool where it comes out and a fountain of light
/// standing on it, both flickering, that the particles fly out of.
fn draw_eruption(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    eruption: &crate::weapons::Eruption,
) {
    const POOL_WEDGES: usize = 24;
    let (x, ground, z) = eruption.at;
    let surface = ground + OVER_GROUND;
    let strength = eruption.strength();
    let flicker = 0.8 + 0.2 * (game.battle_time() * 31.0 + x * 0.0221).sin();
    let height = 271.5 * strength * flicker;
    let hot = [1.0, 0.55, 0.12].map(|c: f32| c * strength * flicker);
    let pool_radius = 144.8 * strength;
    let pool = |i: usize| {
        let (sin, cos) = (i as f32 / POOL_WEDGES as f32 * TAU).sin_cos();
        (x + pool_radius * cos, surface, z + pool_radius * sin)
    };
    for i in 0..POOL_WEDGES {
        canvas.deep_light(
            view,
            [
                ((x, surface, z), hot),
                (pool(i), [0.0; 3]),
                (pool(i + 1), [0.0; 3]),
            ],
        );
    }
    // Two blades across each other, so that it has width from any side.
    for (dx, dz) in [(24.89, 0.0), (0.0, 24.89)] {
        let (left, right) = ((x - dx, ground, z - dz), (x + dx, ground, z + dz));
        let (tip_x, tip_z) = (x + 9.051 * (game.battle_time() * 7.0).sin(), z);
        canvas.deep_light(
            view,
            [
                (left, hot),
                (right, hot),
                ((tip_x, ground + height, tip_z), [0.0; 3]),
            ],
        );
    }
}

/// A nuclear mushroom: a stem of smoke climbing from a skirt of dust, under a cap rolling over
/// on itself as it swells. It burns inside at first, then darkens and thins away.
fn draw_mushroom(
    canvas: &mut Canvas,
    view: &battlefield::View,
    mushroom: &crate::weapons::Mushroom,
) {
    const STEM_LEVELS: usize = 14;
    const STEM_AROUND: usize = 3;
    const CAP_AROUND: usize = 16;
    const CAP_TUBE: usize = 5;
    const SKIRT: usize = 20;
    const ASH: [f32; 3] = [0.36, 0.31, 0.27];
    const FIRE: [f32; 3] = [0.95, 0.45, 0.12];
    let (x, ground, z) = mushroom.at;
    let radius = mushroom.radius;
    let age = 1.0 - mushroom.life;
    // It shoots up fast at first, then slows as it reaches its height.
    let rise = 1.0 - (1.0 - (age * 2.5).min(1.0)).powi(2);
    let fire = (1.0 - age * 4.0).max(0.0);
    let thin = (mushroom.life * 4.0).min(1.0) * 0.9;
    let top = ground + radius * (0.6 + 1.4 * rise);
    let cap_radius = radius * (0.25 + 0.45 * rise);
    let tube = radius * (0.18 + 0.12 * rise);
    let tint = |hot: f32| {
        let hot = (fire * hot).min(1.0);
        [0, 1, 2].map(|i| ASH[i] + (FIRE[i] - ASH[i]) * hot)
    };
    // Steady scatter, so that the cloud keeps its shape from frame to frame.
    let jitter = |seed: usize| (seed as f32 * 12.9898).sin() * 0.5;
    for i in 0..SKIRT {
        let angle = i as f32 / SKIRT as f32 * TAU + jitter(i);
        let out = radius * (0.4 + 0.8 * rise);
        let at = (x + angle.cos() * out, ground + 13.58, z + angle.sin() * out);
        draw_puff(canvas, view, at, radius * 0.14, thin * 0.6, Some(tint(0.3)));
    }
    for level in 0..STEM_LEVELS {
        let up = level as f32 / STEM_LEVELS as f32;
        let width = tube * 0.55 * (1.0 + 0.6 * (1.0 - up));
        for i in 0..STEM_AROUND {
            let angle = i as f32 / STEM_AROUND as f32 * TAU + level as f32 * 1.3;
            let at = (
                x + angle.cos() * width * 0.4,
                ground + (top - ground - tube * 0.5) * up,
                z + angle.sin() * width * 0.4,
            );
            draw_puff(canvas, view, at, width * 0.9, thin, Some(tint(0.6 + up)));
        }
    }
    for i in 0..CAP_AROUND {
        let angle = i as f32 / CAP_AROUND as f32 * TAU + jitter(i) * 0.2;
        for k in 0..CAP_TUBE {
            // Rolling out over the top and back in underneath.
            let roll = k as f32 / CAP_TUBE as f32 * TAU - age * 6.0;
            let out = cap_radius + tube * roll.cos();
            let at = (
                x + angle.cos() * out,
                top + tube * 0.7 * roll.sin(),
                z + angle.sin() * out,
            );
            let size = tube * (0.75 + 0.15 * jitter(i * CAP_TUBE + k));
            draw_puff(canvas, view, at, size, thin, Some(tint(1.2)));
        }
    }
    draw_puff(
        canvas,
        view,
        (x, top + tube * 0.6, z),
        cap_radius * 0.9,
        thin,
        Some(tint(1.0)),
    );
    if fire > 0.0 {
        let glow = FIRE.map(|c| c * fire);
        draw_glare(canvas, view, (x, top, z), cap_radius * 1.6, glow);
        let stem = glow.map(|c| c * 0.6);
        draw_glare(canvas, view, (x, (ground + top) / 2.0, z), tube * 1.5, stem);
    }
}

/// A soft round puff: a spark adds its light and shrinks as it cools, and smoke covers what is
/// behind it and spreads as it fades.
fn draw_particle(
    canvas: &mut Canvas,
    view: &battlefield::View,
    particle: &crate::weapons::Particle,
) {
    let life = particle.life;
    let grown = if particle.smoke {
        particle.size * (2.0 - life)
    } else {
        particle.size * (0.4 + 0.6 * life)
    };
    let tint = particle.smoke.then_some(SMOKE);
    draw_puff(canvas, view, particle.position, grown, life, tint);
}

/// A funnel of dusty wind: rings of see-through polygons, widening as they rise, streaked with
/// dust that spirals up around them. Where it touches the ground it throws up wisps of its own.
fn draw_tornado(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    tornado: &crate::weapons::Tornado,
) {
    const RINGS: usize = 9;
    const AROUND: usize = 14;
    const HEIGHT: f32 = 271.5;
    const FOOT: f32 = 6.788;
    const MOUTH: f32 = 81.46;
    let dust = crate::weapons::dust_color();
    let time = game.battle_time() + tornado.phase;
    let (x, y, z) = tornado.at;
    // It spins up out of nothing, and dies away again.
    let strength = (tornado.life * 6.0).min(1.0) * ((1.0 - tornado.life) * 12.0).min(1.0);
    let corner = |ring: usize, i: usize| -> (Point, Color) {
        let up = ring as f32 / RINGS as f32;
        let radius = FOOT + (MOUTH - FOOT) * up.powf(1.7);
        let sway = (time * 0.9 + up * 3.0).sin() * 22.63 * up;
        let angle = i as f32 / AROUND as f32 * TAU;
        let (sin, cos) = (angle + time * 5.0 + up * 5.0).sin_cos();
        let streak = 0.5 + 0.5 * (angle * 3.0 + up * 11.0 - time * 6.0).sin();
        let ends = (up * 5.0).min(1.0) * ((1.0 - up) * 2.5).min(1.0);
        let at = (x + sway + radius * cos, y + up * HEIGHT, z + radius * sin);
        (
            at,
            with_alpha(dust, (0.12 + 0.5 * streak) * ends * strength),
        )
    };
    for ring in 0..RINGS {
        for i in 0..AROUND {
            let (a, b) = (corner(ring, i), corner(ring, i + 1));
            let (c, d) = (corner(ring + 1, i + 1), corner(ring + 1, i));
            canvas.deep_fill(view, [a, b, c]);
            canvas.deep_fill(view, [a, c, d]);
        }
    }
}

/// Soft balls of light added over an explosion, facing the camera: a hot core inside a wide
/// orange glow, and a few more bursting around them.
fn draw_blast_glow(canvas: &mut Canvas, view: &battlefield::View, blast: &crate::weapons::Blast) {
    const BURSTS: usize = 5;
    let (x, y, z) = blast.at;
    let (life, radius) = (blast.life, blast.radius);
    let at = (x, y + radius * 0.35, z);
    let grown = radius * (1.5 - life * 0.6);
    draw_glare(
        canvas,
        view,
        at,
        grown,
        [0.75 * life, 0.32 * life, 0.08 * life],
    );
    draw_glare(
        canvas,
        view,
        at,
        grown * 0.5,
        [life, 0.8 * life, 0.45 * life],
    );
    // Scattered by where it went off, so that no two explosions look alike.
    let turn = x * 0.1613 + z * 0.0685;
    for burst in 0..BURSTS {
        let (sin, cos) = (turn + burst as f32 * TAU / BURSTS as f32).sin_cos();
        let out = radius * (1.1 - life * 0.7);
        let lifted = y + radius * (0.2 + 0.25 * (burst % 3) as f32);
        draw_glare(
            canvas,
            view,
            (x + cos * out, lifted, z + sin * out),
            grown * 0.4,
            [0.6 * life, 0.28 * life, 0.05 * life],
        );
    }
}

/// A wisp of vapor or dust, spreading and thinning as it goes.
fn draw_wisp(canvas: &mut Canvas, view: &battlefield::View, wisp: &crate::weapons::Wisp) {
    let grown = wisp.size * (2.0 - wisp.life);
    let opacity = 0.7 * wisp.life;
    draw_puff(
        canvas,
        view,
        wisp.position,
        grown,
        opacity,
        Some(wisp.color),
    );
}

/// A puff this big at a point: smoke of a tint covering what is behind it, or without one a
/// spark adding its light, as strong as its life left.
fn draw_puff(
    canvas: &mut Canvas,
    view: &battlefield::View,
    (x, y, z): Point,
    grown: f32,
    life: f32,
    tint: Option<[f32; 3]>,
) {
    let (Some((mx, my, depth)), Some(above)) =
        (view.project_deep(x, y, z), view.project(x, y + grown, z))
    else {
        return;
    };
    const WEDGES: usize = 8;
    let reach = (my - above.1).abs().max(1.0);
    let rim = |i: usize| {
        let (sin, cos) = (i as f32 / WEDGES as f32 * TAU).sin_cos();
        (mx + reach * cos, my + reach * sin)
    };
    for i in 0..WEDGES {
        let corners = [(mx, my), rim(i), rim(i + 1)];
        let (middle, edge) = if let Some(tint) = tint {
            let dark = Vertex::new(corners[0], with_alpha(tint, 0.85 * life));
            (dark, with_alpha(tint, 0.0))
        } else {
            let glow = [life, life * (0.3 + 0.6 * life), 0.3 * life * life];
            (Vertex::light(corners[0], glow), [0.0; 4])
        };
        canvas.effects.extend([
            EffectVertex::of(middle, depth),
            EffectVertex::of(Vertex::new(corners[1], edge), depth),
            EffectVertex::of(Vertex::new(corners[2], edge), depth),
        ]);
    }
}

/// A cone of fire or gas, solid where it leaves the nozzle and fading towards its far edge,
/// where it comes down to about the height of whoever it reaches.
fn draw_spray(canvas: &mut Canvas, view: &battlefield::View, spray: &crate::weapons::Spray) {
    const WEDGES: usize = 10;
    const NEAR_ALPHA: f32 = 0.8;
    const FAR_ALPHA: f32 = 0.1;
    const FAR_HEIGHT: f32 = 18.1;
    let (x, y, z) = spray.from;
    let color = if spray.gas { GAS_COLOR } else { FIRE_COLOR };
    let far = |i: usize| {
        let angle =
            spray.heading - spray.half_angle + i as f32 / WEDGES as f32 * spray.half_angle * 2.0;
        let (sin, cos) = angle.sin_cos();
        (
            x + cos * spray.reach,
            FAR_HEIGHT.min(y),
            z + sin * spray.reach,
        )
    };
    let shade = |alpha: f32| with_alpha(color, alpha * spray.life);
    for i in 0..WEDGES {
        canvas.deep_fill(
            view,
            [
                (spray.from, shade(NEAR_ALPHA)),
                (far(i), shade(FAR_ALPHA)),
                (far(i + 1), shade(FAR_ALPHA)),
            ],
        );
    }
}

/// A flame turret's tongue of fire: yellow down the middle of the blast, then orange, and red
/// out at its edge and as it dies away.
fn draw_flamelet(
    canvas: &mut Canvas,
    view: &battlefield::View,
    flamelet: &crate::weapons::Flamelet,
) {
    const CORE: [f32; 3] = [1.0, 0.85, 0.25];
    const MIDDLE: [f32; 3] = [1.0, 0.42, 0.06];
    const RIM: [f32; 3] = [0.85, 0.1, 0.03];
    let mix = |from: [f32; 3], to: [f32; 3], by: f32| -> [f32; 3] {
        std::array::from_fn(|i| from[i] + (to[i] - from[i]) * by)
    };
    let heat = flamelet.heat * flamelet.life.sqrt();
    let color = if heat > 0.5 {
        mix(MIDDLE, CORE, (heat - 0.5) * 2.0)
    } else {
        mix(RIM, MIDDLE, heat * 2.0)
    };
    let size = flamelet.size * (1.8 - flamelet.life * 0.8);
    let glow = color.map(|c| c * flamelet.life.min(0.5) * 1.2);
    draw_glare(canvas, view, flamelet.position, size, glow);
}

/// Around each selected armed tower and air defense platform, a low see-through green wall
/// as far out as it reaches.
fn draw_tower_ranges(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    const SIDES: usize = 96;
    const WALL_HEIGHT: f32 = 20.36;
    const FOOT_ALPHA: f32 = 0.38;
    const TOP_ALPHA: f32 = 0.08;
    const WALL_GREEN: [f32; 3] = [0.3, 1.0, 0.4];
    let terrain = battlefield::terrain(game.battle_map);
    for tower in game.units.iter().filter(|u| {
        u.selected && (u.kind == UnitKind::Tower || u.role == Role::HkAdp) && u.weapon.is_armed()
    }) {
        let radius = tower.range();
        let foot = |i: usize| {
            let (sin, cos) = (i as f32 / SIDES as f32 * TAU).sin_cos();
            let (x, z) = (tower.x + radius * cos, tower.z + radius * sin);
            (x, terrain.height_at(x, z).max(0.0) + OVER_GROUND, z)
        };
        let top = |(x, y, z): Point| (x, y + WALL_HEIGHT, z);
        let (low, high) = (
            with_alpha(WALL_GREEN, FOOT_ALPHA),
            with_alpha(WALL_GREEN, TOP_ALPHA),
        );
        for i in 0..SIDES {
            let (a, b) = (foot(i), foot(i + 1));
            canvas.deep_fill(view, [(a, low), (b, low), (top(b), high)]);
            canvas.deep_fill(view, [(a, low), (top(b), high), (top(a), high)]);
        }
    }
}

/// While the superweapon waits for a click, a ring on the ground shows what it would hit.
fn draw_strike_aim(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    const SIDES: usize = 48;
    if !game.aiming_strike {
        return;
    }
    let Some((x, z)) = game.land_under_cursor() else {
        return;
    };
    let radius = STRIKE_RADIUS;
    draw_draped_disc(canvas, game, view, (x, z), radius, POISON_FILL, (0.05, 0.2));
    let thickness = (3.0 * ui_scale(game.height)).max(1.0);
    let point = |i: usize| {
        let (sin, cos) = (i as f32 / SIDES as f32 * TAU).sin_cos();
        view.project(x + radius * cos, 0.9051, z + radius * sin)
    };
    for i in (0..SIDES).step_by(2) {
        if let (Some(a), Some(c)) = (point(i), point(i + 1)) {
            canvas.line(a, c, thickness, with_alpha(POISON_FILL, 1.0));
        }
    }
}

/// A small bar over anyone who has been hurt.
/// The number key a selected unit is under, beside it.
fn draw_group_number(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    if !unit.selected {
        return;
    }
    let Some(number) = game.group_number(unit.id) else {
        return;
    };
    let Some((x, y)) = view.project(unit.x, unit.pick_height() + 45.25, unit.z) else {
        return;
    };
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    canvas.text_centered(&number.to_string(), x + 22.0 * s, y, px, TEXT);
}

fn draw_health_bar(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    if unit.health >= unit.max_health() {
        return;
    }
    let Some((x, y)) = view.project(unit.x, unit.pick_height() + 45.25, unit.z) else {
        return;
    };
    let s = ui_scale(game.height);
    let (width, height) = (30.0 * s, (4.0 * s).max(2.0));
    let share = (unit.health / unit.max_health()).clamp(0.0, 1.0);
    canvas.rect(x - width / 2.0, y, width, height, HEALTH_BAD);
    canvas.rect(x - width / 2.0, y, width * share, height, HEALTH_GOOD);
}

/// A row of pips under where a health bar goes: for a machine short of rockets, lit for those
/// it has left and dark for those it has fired; and for an APC of the player's with anyone on
/// board, lit for the seats taken.
fn draw_pips(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    let pips = match (unit.rockets_left(), unit.role.rocket_load()) {
        (Some(left), Some(load)) if left < load => Some((left, load)),
        _ if unit.owner == 0 && unit.role.carries_infantry() => {
            let aboard = unit.passengers.iter().flatten().count() as u8;
            (aboard > 0).then_some((aboard, units::APC_ROOM as u8))
        }
        _ => None,
    };
    let Some((left, load)) = pips else {
        return;
    };
    let Some((x, y)) = view.project(unit.x, unit.pick_height() + 45.25, unit.z) else {
        return;
    };
    let s = ui_scale(game.height);
    let (pip, gap) = ((3.0 * s).max(2.0), (1.5 * s).max(1.0));
    let width = f32::from(load) * (pip + gap) - gap;
    let top = y + (7.0 * s).max(4.0);
    for i in 0..load {
        let color = if i < left {
            ROCKET_PIP
        } else {
            ROCKET_PIP_GONE
        };
        let px = x - width / 2.0 + f32::from(i) * (pip + gap);
        canvas.rect(px, top, pip, pip, color);
    }
}

/// One gold V for each rank above the first, stacked above where a health bar goes.
fn draw_medals(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    let rank = unit.rank();
    if rank == 0 {
        return;
    }
    let Some((x, y)) = view.project(unit.x, unit.pick_height() + 45.25, unit.z) else {
        return;
    };
    let s = ui_scale(game.height);
    let (half_w, tall, step) = (7.0 * s, 6.0 * s, 8.0 * s);
    let thickness = (2.5 * s).max(1.0);
    for i in 0..rank {
        let top = y - 6.0 * s - tall - i as f32 * step;
        let (left, tip, right) = ((x - half_w, top), (x, top + tall), (x + half_w, top));
        canvas.line(left, tip, thickness, MEDAL);
        canvas.line(tip, right, thickness, MEDAL);
    }
}

/// The superweapon's symbol: three blades around a dot.
fn draw_radiation_symbol(canvas: &mut Canvas, cx: f32, cy: f32, size: f32, c: Color) {
    const POINTS: usize = 8;
    let (inner, outer) = (size * 0.14, size * 0.45);
    for blade in 0..3 {
        let middle = blade as f32 / 3.0 * TAU - FRAC_PI_2;
        let spread = 0.5;
        for i in 0..POINTS {
            let at = |t: f32, r: f32| {
                let angle = middle - spread + t * spread * 2.0;
                (cx + r * angle.cos(), cy + r * angle.sin())
            };
            let (t0, t1) = (i as f32 / POINTS as f32, (i + 1) as f32 / POINTS as f32);
            canvas.triangle([at(t0, inner), at(t0, outer), at(t1, outer)], c);
            canvas.triangle([at(t0, inner), at(t1, outer), at(t1, inner)], c);
        }
    }
    canvas.circle(cx, cy, size * 0.08, c);
}

/// For each selected unit, a dashed line along the ground to what it was told to do: red to
/// the unit or place it is after, for as long as it is after one, or green to where it is
/// going.
fn draw_order_lines(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    for unit in game.units.iter().filter(|u| u.selected) {
        let after = match unit.target {
            Some(Target::Place(x, z)) => Some((x, z)),
            Some(
                Target::Unit(id) | Target::Forced(id) | Target::Rescue(id) | Target::RunOver(id),
            ) => game
                .units
                .iter()
                .find(|u| u.alive && u.id == id && game.shows(u))
                .map(|u| (u.x, u.z)),
            None => None,
        };
        let from = (unit.x, unit.z);
        match after {
            Some(to)
                if matches!(unit.role, Role::AtRepairUnit | Role::AdvCarryall)
                    && !matches!(unit.target, Some(Target::RunOver(_))) =>
            {
                draw_dashed_line(canvas, game, view, from, to, HEAL_LINE)
            }
            Some(to) => draw_dashed_line(canvas, game, view, from, to, ATTACK_LINE),
            None if unit.is_moving() => {
                draw_dashed_line(canvas, game, view, from, unit.destination, ORDER_LINE)
            }
            None => {
                if let Some(point) = unit.gathering {
                    draw_dashed_line(canvas, game, view, from, point, ORDER_LINE);
                    draw_gathering_flag(canvas, game, point);
                }
            }
        }
    }
}

/// A red triangular flag on a pole, stirring a little, where a building sends what it makes. It
/// leans towards the camera, or from straight above it would hardly show.
fn draw_gathering_flag(canvas: &mut Canvas, game: &Game, (x, z): (f32, f32)) {
    const POLE_HEIGHT: f32 = 67.88;
    const FLAG_TALL: f32 = 22.63;
    const FLAG_LEANS: f32 = 31.68;
    const FLAG_LONG: f32 = 45.25;
    let ground = battlefield::terrain(game.battle_map)
        .height_at(x, z)
        .max(0.0);
    let top = ground + POLE_HEIGHT;
    draw_turned_box(
        canvas,
        (x, z, 1.584, 1.584, 0.0),
        (ground, top),
        [0.82, 0.82, 0.8],
    );
    let stir = (game.screen_elapsed() * 3.0 + x * 0.0221).sin() * 3.168;
    let normal = unit_vector([0.0, FLAG_LEANS, FLAG_TALL]);
    canvas.objects.extend(
        [
            (x, top, z),
            (x, top - FLAG_TALL, z + FLAG_LEANS),
            (
                x + FLAG_LONG,
                top - FLAG_TALL / 2.0 + stir,
                z + FLAG_LEANS / 2.0,
            ),
        ]
        .map(|point| WorldVertex::solid(point, normal, GATHERING_FLAG)),
    );
}

fn draw_dashed_line(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    from: (f32, f32),
    to: (f32, f32),
    color: Color,
) {
    const DASH: f32 = 20.36;
    const GAP: f32 = 13.58;
    let thickness = (2.0 * ui_scale(game.height)).max(1.0);
    let screen = (0.0, 0.0, game.width, game.height);
    let terrain = battlefield::terrain(game.battle_map);
    let (to_x, to_z) = (to.0 - from.0, to.1 - from.1);
    let length = to_x.hypot(to_z);
    let (dir_x, dir_z) = (to_x / length, to_z / length);
    let mut along = 0.0;
    while along < length {
        let end = (along + DASH).min(length);
        let a = (from.0 + dir_x * along, from.1 + dir_z * along);
        let b = (from.0 + dir_x * end, from.1 + dir_z * end);
        // Along the land itself, so that it meets what stands at its end.
        let on_land = |(x, z): (f32, f32)| view.project(x, terrain.height_at(x, z).max(0.0), z);
        let visible = on_land(a)
            .zip(on_land(b))
            .and_then(|(a, b)| battlefield::clip_to_rect(a, b, screen));
        if let Some((a, b)) = visible {
            canvas.line(a, b, thickness, color);
        }
        along += DASH + GAP;
    }
}

/// A vehicle hanging under its carryall on two cables.
fn draw_carried(canvas: &mut Canvas, game: &Game, view: &battlefield::View, carrier: &Unit) {
    let cables = |canvas: &mut Canvas, bottom_y: f32| {
        let (sin, cos) = carrier.heading.sin_cos();
        for side in [-6.788, 6.788] {
            let (x, z) = (carrier.x - sin * side, carrier.z + cos * side);
            let top = (x, carrier.y + carrier.altitude, z);
            let bottom = (x, bottom_y + 11.31, z);
            canvas.solid_bar(top, bottom, 0.5431, [cos, 0.0, sin], CABLE);
        }
    };
    if let Some(stolen) = stolen_by(game, carrier) {
        // It draws itself, being in the battle still.
        cables(canvas, stolen.y + stolen.altitude);
    }
    if let Some(load) = game.carried.iter().find(|load| load.carrier == carrier.id) {
        cables(canvas, load.vehicle.y);
        draw_seen_model(canvas, game, &load.vehicle, seen_by(game, view, carrier));
    }
}

/// The enemy vehicle this carryall has hold of, if any.
fn stolen_by<'a>(game: &'a Game, carrier: &Unit) -> Option<&'a Unit> {
    game.units
        .iter()
        .find(|u| u.alive && u.theft.is_some_and(|theft| theft.carrier == carrier.id))
}

/// Which way, and how far, the fight throws something that is in one right now, as it is seen.
fn shaken(game: &Game, unit: &Unit, seed: f32) -> Unit {
    let time = game.battle_time() * 37.0 + seed;
    let mut shown = *unit;
    shown.x += 3.168 * time.sin();
    shown.z += 2.263 * (time * 1.3).cos();
    shown.heading += 0.12 * (time * 0.7).sin();
    shown
}

/// How many lamps a machine of this role has.
#[cfg(test)]
pub fn lamps_on(role: Role) -> usize {
    models::lamp_spots(role).len()
}

/// How much of a unit is worth drawing this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    /// On the screen, big enough to show all its detail.
    Near,
    /// On the screen, but small.
    Far,
    /// Off the screen, with its shadow reaching onto it.
    ShadowOnly,
    /// Neither it nor its shadow is on the screen.
    Unseen,
}

/// Below how many pixels across a unit's bounds on the screen it is drawn with less detail.
const FAR_PIXELS: f32 = 40.0;

/// A ball around all of a unit that is drawn: its middle and its radius.
fn bounds(unit: &Unit) -> (Point, f32) {
    let height = match (unit.kind, unit.role) {
        (UnitKind::Infantry, _) => 31.68,
        (_, Role::AtMinotaurus) => 61.09,
        (_, Role::AtMongoose) => 42.99,
        (_, Role::HkDevastator) => 144.8,
        // Its pipe stands high once it is deployed.
        // Its drill sticks out a long way in front.
        (_, Role::Mcv) => 135.8,
        // Its cannon stands high once it is deployed.
        (_, Role::OrKobra) => 104.1,
        // It may have a vehicle hanging under it.
        (_, Role::AdvCarryall | Role::Carryall) => 99.56,
        (_, Role::Harvester) => 63.36,
        (UnitKind::Vehicle | UnitKind::Aircraft, _) => 22.63,
        (UnitKind::Tower, _) => (units::TUBE_HEIGHT + 0.4) * unit.stretch()[1],
        (UnitKind::Building, role) => role.building_top(unit.house),
    };
    let middle = (unit.x, unit.y + unit.altitude + height / 2.0, unit.z);
    (middle, unit.radius() * 1.5 + height / 2.0)
}

/// How much of a unit the camera can see: of it, or of its shadow on the ground.
fn seen_by(game: &Game, view: &battlefield::View, unit: &Unit) -> Seen {
    let (middle, radius) = bounds(unit);
    if let Some(pixels) = view.ball_on_screen(middle, radius) {
        return if pixels < FAR_PIXELS {
            Seen::Far
        } else {
            Seen::Near
        };
    }
    // The shadow lies between its foot and where the sun throws its top, on the ground.
    let top = middle.1 + radius - unit.y;
    let [sx, sy, sz] = sun_of(game);
    let thrown = (-sx / sy * top, -sz / sy * top);
    let shadow = (unit.x + thrown.0 / 2.0, unit.y, unit.z + thrown.1 / 2.0);
    let reach = radius + thrown.0.hypot(thrown.1) / 2.0;
    if view.ball_on_screen(shadow, reach).is_some() {
        Seen::ShadowOnly
    } else {
        Seen::Unseen
    }
}

fn draw_unit(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    let seen = seen_by(game, view, unit);
    if let Some(theft) = unit.theft {
        // A vehicle's model stands on its y, however high it has been lifted.
        let mut lifted = *unit;
        lifted.y += unit.altitude;
        lifted.altitude = 0.0;
        let shown = if theft.struggle > 0.0 {
            shaken(game, &lifted, 0.0)
        } else {
            lifted
        };
        draw_seen_model(canvas, game, &shown, seen);
        return;
    }
    let is_fighting = unit.role == Role::AdvCarryall
        && stolen_by(game, unit)
            .is_some_and(|stolen| stolen.theft.is_some_and(|theft| theft.struggle > 0.0));
    let shaken_unit = is_fighting.then(|| shaken(game, unit, 2.0));
    let unit = shaken_unit.as_ref().unwrap_or(unit);
    // The original's models have their own lights, just where it has them; ours have these.
    let lit = !original::has_model(unit) && !unit.is_under_construction();
    // Beams reach far past what they come from, so they show even when it does not.
    if lit {
        draw_beams(canvas, game, view, unit);
    }
    if unit.role == Role::Helipad && lit {
        draw_searchlight(canvas, game, view, unit);
    }
    if unit.role == Role::Mcv {
        draw_mcv_lights(canvas, game, view, unit);
    }
    if unit.role == Role::Palace(Faction::Harkonnen) && lit {
        draw_palace_lights(canvas, game, view, unit);
    }
    if unit.role == Role::Refinery && lit {
        draw_red_glares(canvas, view, &stack_lamps(unit), game.battle_time());
    }
    if unit.role == Role::HkAdp && lit {
        let glares = models::air_defense_glares(unit);
        draw_red_glares(canvas, view, &glares, game.battle_time());
    }
    draw_seen_model(canvas, game, unit, seen);
    if matches!(seen, Seen::Near | Seen::Far) {
        if unit.role == Role::AtAdp {
            draw_jet_flame(canvas, game, view, unit, models::DRONE_NOZZLE);
        }
        if unit.role == Role::Carryall {
            let [a, u, c] = units::CARRYALL_JET_NOZZLE;
            for c in [-c, c] {
                draw_jet_flame(canvas, game, view, unit, [a, u, c]);
            }
        }
        if unit.wear() > units::BURNS_PAST {
            draw_flames(canvas, game, view, unit);
        }
    }
}

/// A unit's model with as much of it as can be seen, stained as it is: on the screen with its
/// detail for how big it shows there, or off it only as a rough shape that casts its shadow.
fn draw_seen_model(canvas: &mut Canvas, game: &Game, unit: &Unit, seen: Seen) {
    let drawn = canvas.objects.len();
    match seen {
        Seen::Near | Seen::Far => {
            if seen == Seen::Far {
                canvas.detail = models::Detail::Low;
            }
            draw_standing_model(canvas, game, unit);
            stain_model(canvas, drawn, unit);
            if unit.is_cloaked() {
                cloak_model(canvas, drawn);
            }
        }
        Seen::ShadowOnly => {
            let watchers = canvas.watchers;
            canvas.detail = models::Detail::Low;
            canvas.watchers = watchers.map(|w| models::Watchers { camera: false, ..w });
            draw_standing_model(canvas, game, unit);
            canvas.watchers = watchers;
            let shadow: Vec<WorldVertex> = canvas.objects.drain(drawn..).collect();
            canvas.casters.extend(shadow);
        }
        Seen::Unseen => {}
    }
    canvas.detail = models::Detail::Full;
}

/// A destroyed vehicle's model, every face of it, its triangles flying apart.
fn draw_shatter(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    shatter: &crate::weapons::Shatter,
) {
    let seen = seen_by(game, view, &shatter.unit);
    if !matches!(seen, Seen::Near | Seen::Far) {
        return;
    }
    if seen == Seen::Far {
        canvas.detail = models::Detail::Low;
    }
    let drawn = canvas.objects.len();
    // Once they tumble, any face of it can come to be seen.
    let watchers = canvas.watchers.take();
    draw_unit_model(canvas, game, &shatter.unit);
    canvas.watchers = watchers;
    canvas.detail = models::Detail::Full;
    let terrain = battlefield::terrain(game.battle_map);
    break_apart(&mut canvas.objects[drawn..], shatter, terrain);
}

/// Pulls a model's triangles in for a moment, then throws each its own way, spinning, while
/// they cool from a glow to char, fall to the ground and shrink away.
fn break_apart(
    triangles: &mut [WorldVertex],
    shatter: &crate::weapons::Shatter,
    terrain: &battlefield::Terrain,
) {
    const IMPLODE_SECONDS: f32 = 0.1;
    const PULLED_IN: f32 = 0.4;
    const GRAVITY: f32 = 407.3;
    const DRAG: f32 = 2.2;
    const SHRINKS_FROM: f32 = 0.35;
    const GLOW_SECONDS: f32 = 0.3;
    const CHAR_SECONDS: f32 = 0.9;
    const CHAR: f32 = 0.05;
    let (unit, age) = (&shatter.unit, shatter.age);
    let (cx, cy, cz) = unit.center();
    let center = [cx, cy, cz];
    let pull = (age / IMPLODE_SECONDS).min(1.0);
    let flight = (age - IMPLODE_SECONDS).max(0.0);
    let scale = ((1.0 - age / crate::weapons::SHATTER_SECONDS) / SHRINKS_FROM).clamp(0.0, 1.0);
    let glow = (1.0 - age / GLOW_SECONDS).max(0.0);
    let charred = (age / CHAR_SECONDS).min(1.0);
    for (n, triangle) in triangles.chunks_exact_mut(3).enumerate() {
        let mut seed =
            (n as u32 + 1).wrapping_mul(0x9E37_79B1) ^ unit.id.wrapping_mul(0x85EB_CA77) | 1;
        let mut next = || battlefield::next_random(&mut seed);
        let middle = [
            (triangle[0].x + triangle[1].x + triangle[2].x) / 3.0,
            (triangle[0].y + triangle[1].y + triangle[2].y) / 3.0,
            (triangle[0].z + triangle[1].z + triangle[2].z) / 3.0,
        ];
        let from = [0, 1, 2].map(|i| middle[i] - center[i]);
        let way = unit_vector([
            from[0] + (next() - 0.5) * 36.2,
            from[1] + 11.31 + next() * 31.68,
            from[2] + (next() - 0.5) * 36.2,
        ]);
        let speed = 135.8 + next() * 407.3;
        let thrown = speed * (1.0 - (-DRAG * flight).exp()) / DRAG;
        let axis = unit_vector([next() - 0.5, next() - 0.5, next() - 0.5]);
        let (sin, cos) = ((6.0 + next() * 22.0) * flight).sin_cos();
        let turned = |v: [f32; 3]| {
            let across = cross(axis, v);
            let along = axis[0] * v[0] + axis[1] * v[1] + axis[2] * v[2];
            [0, 1, 2].map(|i| v[i] * cos + across[i] * sin + axis[i] * along * (1.0 - cos))
        };
        let mut at =
            [0, 1, 2].map(|i| center[i] + from[i] * (1.0 - PULLED_IN * pull) + way[i] * thrown);
        at[1] -= 0.5 * GRAVITY * flight * flight;
        at[1] = at[1].max(terrain.height_at(at[0], at[2]) + 0.9051);
        for vertex in triangle {
            let [x, y, z] = turned([
                vertex.x - middle[0],
                vertex.y - middle[1],
                vertex.z - middle[2],
            ]);
            vertex.x = at[0] + x * scale;
            vertex.y = at[1] + y * scale;
            vertex.z = at[2] + z * scale;
            vertex.normal = turned(vertex.normal);
            vertex.r += (CHAR - vertex.r) * charred + 1.3 * glow;
            vertex.g += (CHAR - vertex.g) * charred + 0.5 * glow;
            vertex.b += (CHAR - vertex.b) * charred;
        }
    }
}

/// A unit's model as it stands now: a building going up is only as tall as it has got, with
/// the cranes and scaffolding that are putting it up.
fn draw_standing_model(canvas: &mut Canvas, game: &Game, unit: &Unit) {
    let drawn = canvas.objects.len();
    draw_unit_model(canvas, game, unit);
    if unit.is_under_construction() {
        let raised = unit.raised();
        for vertex in &mut canvas.objects[drawn..] {
            vertex.y = unit.y + (vertex.y - unit.y) * raised;
        }
        models::draw_construction(canvas, unit);
    }
}

/// The flame burning out of a jet's nozzle, given as [along, up, across] on an aircraft's
/// model, flickering.
fn draw_jet_flame(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    unit: &Unit,
    [a, u, c]: [f32; 3],
) {
    let place = models::Place::in_air(unit);
    let reach = 0.45 + 0.12 * (game.battle_time() * 40.0 + c).sin();
    let at = |p: [f32; 3]| {
        let [x, y, z] = place.point(p);
        (x, y, z)
    };
    canvas.deep_light(
        view,
        [
            (at([a, u, c - 0.03]), ROCKET_FLAME),
            (at([a, u, c + 0.03]), ROCKET_FLAME),
            (at([a - reach, u, c]), [0.0; 3]),
        ],
    );
    canvas.deep_light(
        view,
        [
            (at([a, u - 0.03, c]), ROCKET_FLAME),
            (at([a, u + 0.03, c]), ROCKET_FLAME),
            (at([a - reach, u, c]), [0.0; 3]),
        ],
    );
}

/// Each of a machine's lamps' beams: an additive white trapezium from its lens, narrow there
/// and brightest, to the ground ahead, wide there and fading to nothing.
fn draw_beams(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    for beam in models::beams(unit) {
        draw_beam(canvas, game, view, &beam, unit);
    }
}

/// What lights an MCV: its sweeping searchlight, and once deployed the smoke from its pipe and
/// the red glares that blink in turn either side of it. One drawn with the original's model has
/// that model's lights instead, and only the smoke.
fn draw_mcv_lights(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    const PUFFS: usize = 7;
    let faction = unit.house;
    let time = game.battle_time();
    let lit = !original::has_model(unit);
    if lit {
        draw_beam(
            canvas,
            game,
            view,
            &models::mcv_searchlight(unit, faction, time),
            unit,
        );
    }
    if !unit.deployed {
        return;
    }
    let (x, y, z) = models::mcv_pipe_top(unit, faction);
    for i in 0..PUFFS {
        // Each puff rises, drifts with the wind and spreads as it fades.
        let age = (time * 0.3 + i as f32 / PUFFS as f32 + unit.id as f32 * 0.13).fract();
        let at = (x + age * 49.78, y + 4.525 + age * 99.56, z + age * 18.1);
        draw_puff(
            canvas,
            view,
            at,
            8.146 + age * 24.89,
            1.0 - age,
            Some(SMOKE),
        );
    }
    if lit {
        draw_red_glares(canvas, view, &models::mcv_glares(unit, faction), time);
    }
}

/// Red glares that blink in turn.
fn draw_red_glares(canvas: &mut Canvas, view: &battlefield::View, glares: &[Point], time: f32) {
    for (i, &glare) in glares.iter().enumerate() {
        let blink = (time * 2.5 + i as f32 * PI).sin().max(0.0);
        draw_glare(
            canvas,
            view,
            glare,
            2.715 + 4.525 * blink,
            [0.9 * blink, 0.05 * blink, 0.02],
        );
    }
}

/// Bright white floodlights on the steps of a House Harkonnen palace, and red warning lights high up.
fn draw_palace_lights(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    const WHITE: [[f32; 3]; 8] = [
        [1.95, 1.08, 1.95],
        [-1.95, 1.08, 1.95],
        [1.95, 1.08, -1.95],
        [-1.95, 1.08, -1.95],
        [1.45, 2.28, 1.45],
        [-1.45, 2.28, 1.45],
        [1.45, 2.28, -1.45],
        [-1.45, 2.28, -1.45],
    ];
    const RED: [[f32; 3]; 4] = [
        [0.62, 4.75, 0.62],
        [-0.62, 4.75, -0.62],
        [1.0, 3.5, -1.0],
        [-1.0, 3.5, 1.0],
    ];
    let [stretch_x, stretch_y, stretch_z] = unit.stretch();
    let (sin, cos) = unit.facing.turn().sin_cos();
    let at = |[x, y, z]: [f32; 3]| {
        let (x, y, z) = (x * stretch_x, y * stretch_y, z * stretch_z);
        (
            unit.x + x * cos - z * sin,
            unit.y + y,
            unit.z + x * sin + z * cos,
        )
    };
    for spot in WHITE {
        draw_glare(canvas, view, at(spot), 14.48, [1.0, 1.0, 0.95]);
    }
    draw_red_glares(canvas, view, &RED.map(at), game.battle_time());
}

/// A round glow of light, brightest in its middle.
fn draw_glare(canvas: &mut Canvas, view: &battlefield::View, at: Point, size: f32, glow: [f32; 3]) {
    const WEDGES: usize = 8;
    let (x, y, z) = at;
    let (Some((mx, my, depth)), Some(above)) =
        (view.project_deep(x, y, z), view.project(x, y + size, z))
    else {
        return;
    };
    let reach = (my - above.1).abs().max(1.0);
    let rim = |i: usize| {
        let (sin, cos) = (i as f32 / WEDGES as f32 * TAU).sin_cos();
        (mx + reach * cos, my + reach * sin)
    };
    for i in 0..WEDGES {
        canvas.effects.extend([
            EffectVertex::of(Vertex::light((mx, my), glow), depth),
            EffectVertex::of(Vertex::new(rim(i), [0.0; 4]), depth),
            EffectVertex::of(Vertex::new(rim(i + 1), [0.0; 4]), depth),
        ]);
    }
}

/// One lamp's beam: a trapezium of light from its lens to the ground ahead, fading as it goes,
/// turned to face the camera.
fn draw_beam(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    beam: &models::Beam,
    from: &Unit,
) {
    let terrain = battlefield::terrain(game.battle_map);
    let (x, y, z) = beam.lens;
    let (fx, fz) = (
        x + beam.ahead[0] * beam.reach,
        z + beam.ahead[2] * beam.reach,
    );
    let far_y = terrain.height_at(fx, fz).max(0.0) + OVER_GROUND;
    // It turns about the line from its lens to the ground ahead to face the camera, so that it
    // shows as wide as it can, whichever way it points.
    let axis = [fx - x, far_y - y, fz - z];
    let eye = view.position();
    let middle = [(x + fx) / 2.0, (y + far_y) / 2.0, (z + fz) / 2.0];
    let to_eye = [0, 1, 2].map(|i| eye[i] - middle[i]);
    let side = cross(axis, to_eye);
    let across = if side.iter().map(|c| c * c).sum::<f32>() > 1e-8 {
        unit_vector(side)
    } else {
        beam.right
    };
    // A building in the way stops it there, as bright as it is by then.
    let open = open_share_of_beam(game, (x, z), (fx, fz), from);
    let spread = beam.spread * open;
    let ended = [HEADLIGHT * (1.0 - open) * 0.6; 3];
    let (fx, far_y, fz) = (
        x + (fx - x) * open,
        y + (far_y - y) * open,
        z + (fz - z) * open,
    );
    // Raised so that the lower of its far corners stays on the ground.
    let far = (fx, far_y + spread * across[1].abs(), fz);
    let corner = |(cx, cy, cz): Point, by: f32| {
        (
            cx + across[0] * by,
            cy + across[1] * by,
            cz + across[2] * by,
        )
    };
    let (a, b) = (corner(beam.lens, -beam.half), corner(beam.lens, beam.half));
    let (c, d) = (corner(far, spread), corner(far, -spread));
    let bright = [HEADLIGHT; 3];
    canvas.deep_light(view, [(a, bright), (b, bright), (c, ended)]);
    canvas.deep_light(view, [(a, bright), (c, ended), (d, ended)]);
}

/// How much of the way along a beam on the ground it gets before a building other than the
/// one it comes from stands in the way. What flies shines down over buildings.
fn open_share_of_beam(game: &Game, lens: (f32, f32), far: (f32, f32), from: &Unit) -> f32 {
    if from.altitude > 0.0 {
        return 1.0;
    }
    let (start, end) = (
        battlefield::world_to_grid(lens.0, lens.1),
        battlefield::world_to_grid(far.0, far.1),
    );
    game.units
        .iter()
        .filter(|u| u.alive && u.id != from.id && game.shows(u))
        .filter_map(Unit::plot)
        .filter_map(|plot| plot.entered_by(start, end))
        .fold(1.0, f32::min)
}

/// Marks what was just drawn of an invisible unit for the backend to draw as a shimmer of grey,
/// blue and pink, every pixel its own and changing fast. Only its own side ever sees it.
fn cloak_model(canvas: &mut Canvas, drawn: usize) {
    for vertex in &mut canvas.objects[drawn..] {
        vertex.stain = CLOAKED;
    }
}

/// What an invisible unit's stains are set to, for the backend to know it by.
pub const CLOAKED: [f32; 3] = [0.0, 0.0, -1.0];

/// Marks what was just drawn of a unit with how black it has gone, and where on the unit each
/// corner is, so that the stains stay put on it as it moves and turns.
fn stain_model(canvas: &mut Canvas, drawn: usize, unit: &Unit) {
    let amount = (unit.wear() - units::STAINS_PAST) / (1.0 - units::STAINS_PAST);
    if amount <= 0.0 {
        return;
    }
    let (sin, cos) = unit.heading.sin_cos();
    // Each unit has its own stains, from a small number that keeps the noise precise.
    let own = (unit.id % 23) as f32 * 3.7;
    for vertex in &mut canvas.objects[drawn..] {
        let (dx, dy, dz) = (vertex.x - unit.x, vertex.y - unit.y, vertex.z - unit.z);
        vertex.stain = [
            (dx * cos + dz * sin + dy * 0.7) * 0.0221 + own,
            (dz * cos - dx * sin - dy * 0.6) * 0.0221,
            amount,
        ];
    }
}

/// Flames standing on what is badly damaged: blades of light licking up from it, more of them
/// on what is bigger.
fn draw_flames(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    let size = unit.radius().max(18.1);
    let count = (size * 0.08839) as usize + 2;
    let time = game.battle_time();
    let top = unit.pick_height();
    for i in 0..count {
        // Spread out over the unit by the golden angle, each flickering in its own time.
        let seed = i as f32 * 2.4 + unit.id as f32;
        let (sin, cos) = seed.sin_cos();
        let out = size * 0.55 * ((i as f32 + 0.5) / count as f32).sqrt();
        let (x, z) = (unit.x + cos * out, unit.z + sin * out);
        let tall = (0.35 + 0.25 * (time * 11.0 + seed).sin()) * (31.68 + size * 0.5);
        let half = 4.073 + 0.05 * size;
        let tip = (x + 2.715 * (time * 7.0 + seed).sin(), top + tall, z);
        let hot = [0.75, 0.32, 0.05];
        canvas.deep_light(
            view,
            [
                ((x - half, top - 4.525, z), hot),
                ((x + half, top - 4.525, z), hot),
                (tip, [0.0; 3]),
            ],
        );
        canvas.deep_light(
            view,
            [
                ((x, top - 4.525, z - half), hot),
                ((x, top - 4.525, z + half), hot),
                (tip, [0.0; 3]),
            ],
        );
    }
}

/// The steel case of a tower's searchlight, and the glass at its front.
const SEARCHLIGHT_HOUSING: [f32; 3] = [0.3, 0.31, 0.33];
const SEARCHLIGHT_LENS: [f32; 3] = [1.0, 0.98, 0.88];
/// A machine gun's dark steel.
const GUNMETAL: [f32; 3] = [0.22, 0.23, 0.25];

/// Where a landing pad's searchlight is, relative to the pad's middle, and how high.
const SEARCHLIGHT: (f32, f32, f32) = (0.52, units::PAD_TOP + 0.45, 0.84);

/// A beam going up into the sky from a landing pad's mast, slowly sweeping around.
fn draw_searchlight(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    const HEIGHT: f32 = 407.3;
    const SWEEP: f32 = 158.4;
    // Where the mast ends up once the pad is laid along the grid.
    let [stretch_x, stretch_y, stretch_z] = unit.stretch();
    let (dx, dy, dz) = SEARCHLIGHT;
    let (dx, dy, dz) = (dx * stretch_x, dy * stretch_y, dz * stretch_z);
    let (sin, cos) = unit.facing.turn().sin_cos();
    let lamp = (
        unit.x + dx * cos - dz * sin,
        unit.y + dy,
        unit.z + dx * sin + dz * cos,
    );
    let angle = game.battle_time() * 0.5 + unit.id as f32;
    let (sin, cos) = angle.sin_cos();
    let far = (lamp.0 + cos * SWEEP, lamp.1 + HEIGHT, lamp.2 + sin * SWEEP);
    // Across the way it points, as the camera sees it, so that it shows as a broad beam.
    let (across_x, across_z) = (33.94, 0.0);
    let near_half = 0.08;
    let bright = [0.55, 0.55, 0.5];
    let corner = |at: Point, by: f32| (at.0 + across_x * by, at.1, at.2 + across_z * by);
    let (a, b) = (corner(lamp, -near_half), corner(lamp, near_half));
    let (c, d) = (corner(far, 1.0), corner(far, -1.0));
    canvas.deep_light(view, [(a, bright), (b, bright), (c, [0.0; 3])]);
    canvas.deep_light(view, [(a, bright), (c, [0.0; 3]), (d, [0.0; 3])]);
}

/// A low deck with a landing mark, just big enough for one machine, and a mast with a
/// searchlight at one corner.
fn draw_helipad(canvas: &mut Canvas, unit: &Unit, color: [f32; 3]) {
    const DECK: [f32; 3] = [0.24, 0.24, 0.26];
    const MARK: [f32; 3] = [0.92, 0.92, 0.88];
    const RING: [f32; 3] = [0.9, 0.72, 0.16];
    const RING_SIDES: usize = 16;
    let (x, z, y) = (unit.x, unit.z, unit.y);
    let top = y + units::PAD_TOP;
    let (half_x, half_z) = units::PAD_DECK;
    let flat = |canvas: &mut Canvas, footprint: (f32, f32, f32, f32), heights, color| {
        let (px, pz, half_x, half_z) = footprint;
        draw_turned_box(canvas, (px, pz, half_x, half_z, 0.0), heights, color);
    };
    // The tower: a broad shaft, and a leg under each corner of the deck.
    let legs = house_paint(unit.house).0;
    flat(
        canvas,
        (x, z, 0.3, 0.36),
        (y, top - 0.14),
        color.map(|c| c * 0.75),
    );
    for (side_x, side_z) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let leg = (
            x + side_x * (half_x - 0.14),
            z + side_z * (half_z - 0.12),
            0.06,
            0.06,
        );
        flat(canvas, leg, (y, top - 0.14), legs);
    }
    flat(
        canvas,
        (x, z, half_x, half_z),
        (top - 0.14, top - 0.07),
        color.map(|c| c * 0.6),
    );
    flat(
        canvas,
        (x, z, half_x - 0.1, half_z - 0.08),
        (top - 0.07, top),
        DECK,
    );
    // The H, thin on the deck.
    let mark = (top, top + 0.012);
    flat(canvas, (x, z - 0.22, 0.32, 0.06), mark, MARK);
    flat(canvas, (x, z + 0.22, 0.32, 0.06), mark, MARK);
    flat(canvas, (x, z, 0.05, 0.2), mark, MARK);
    // The ring around it.
    let ring = |i: usize| {
        let (sin, cos) = (i as f32 / RING_SIDES as f32 * TAU).sin_cos();
        (x + 0.52 * cos, top + 0.012, z + 0.52 * sin)
    };
    for i in 0..RING_SIDES {
        let (a, b) = (ring(i), ring(i + 1));
        let way = unit_vector([b.0 - a.0, 0.0, b.2 - a.2]);
        canvas.solid_bar(a, b, 0.018, [-way[2], 0.0, way[0]], RING);
    }
    // The mast at a corner of the deck, with the lamp on top of it tilted up.
    let (dx, dy, dz) = SEARCHLIGHT;
    let (lamp_x, lamp_z) = (x + dx, z + dz);
    flat(
        canvas,
        (lamp_x, lamp_z, 0.05, 0.05),
        (top, y + dy - 0.1),
        legs,
    );
    flat(
        canvas,
        (lamp_x, lamp_z, 0.11, 0.11),
        (y + dy - 0.1, y + dy),
        [0.3, 0.3, 0.32],
    );
    flat(
        canvas,
        (lamp_x, lamp_z, 0.07, 0.07),
        (y + dy, y + dy + 0.02),
        [1.0, 0.98, 0.8],
    );
}

/// The red lamps either side of a refinery's exhaust stacks, a little under their tops.
fn stack_lamps(unit: &Unit) -> [Point; 4] {
    let [(ax, ay, az), (bx, by, bz)] = unit.stack_tops();
    let [across, up, _] = unit.stretch();
    let (sin, cos) = unit.facing.turn().sin_cos();
    let side = |(x, y, z): Point, way: f32| {
        let way = way * across;
        (x + way * cos, y - 0.2 * up, z + way * sin)
    };
    [
        side((ax, ay, az), -STACK_HALF - 0.03),
        side((ax, ay, az), STACK_HALF + 0.03),
        side((bx, by, bz), -STACK_HALF - 0.03),
        side((bx, by, bz), STACK_HALF + 0.03),
    ]
}

/// How far each way a refinery's exhaust stack reaches from its middle.
const STACK_HALF: f32 = 0.13;

/// A refinery, with its front on its south side: a long hall at the back with a tank on it
/// and two tall exhaust stacks with red lamps near their tops, and in front of it three pads
/// for harvesters to back onto, fanned out with the side ones at 45 degrees to the middle one,
/// each with a chute into the hall.
fn draw_refinery(canvas: &mut Canvas, unit: &Unit, color: [f32; 3]) {
    const DECK: [f32; 3] = [0.24, 0.24, 0.26];
    const STRIPE: [f32; 3] = [0.9, 0.72, 0.16];
    const RED_LAMP: [f32; 3] = [0.7, 0.06, 0.04];
    let (x, z, y) = (unit.x, unit.z, unit.y);
    let (walls, roof) = house_paint(unit.house);
    let mut turned_part = |(px, pz, half_x, half_z, heading), heights, color| {
        draw_turned_box(
            canvas,
            (x + px, z + pz, half_x, half_z, heading),
            heights,
            color,
        );
    };
    let mut part = |(px, pz, half_x, half_z): (f32, f32, f32, f32), heights, color| {
        turned_part((px, pz, half_x, half_z, 0.0), heights, color);
    };
    // The hall, its roof, and the owner's band along its front.
    part((0.0, -1.95, 2.6, 0.8), (y, y + 1.4), walls);
    part((0.0, -1.95, 2.5, 0.7), (y + 1.4, y + 1.52), roof);
    part((0.0, -1.15, 2.6, 0.012), (y + 1.05, y + 1.18), color);
    // The tank on its roof, banded.
    part((0.0, -2.0, 0.65, 0.6), (y + 1.52, y + 2.4), roof);
    part((0.0, -2.0, 0.66, 0.61), (y + 2.1, y + 2.22), color);
    // The stacks: a broad foot, a tall shaft, a band and a dark rim at the top, and red lamps.
    let top = units::REFINERY_STACK_TOP;
    for (sx, sz) in units::REFINERY_STACKS {
        part((sx, sz, 0.22, 0.22), (y + 1.4, y + 1.9), walls);
        part((sx, sz, STACK_HALF, STACK_HALF), (y + 1.9, y + top), roof);
        part(
            (sx, sz, STACK_HALF + 0.01, STACK_HALF + 0.01),
            (y + top - 0.55, y + top - 0.45),
            color,
        );
        part(
            (sx, sz, STACK_HALF + 0.02, STACK_HALF + 0.02),
            (y + top - 0.06, y + top),
            BUILDING_DOOR,
        );
        for side in [-1.0, 1.0] {
            let lamp = (sx + side * (STACK_HALF + 0.03), sz, 0.03, 0.03);
            part(lamp, (y + top - 0.23, y + top - 0.17), RED_LAMP);
        }
    }
    // The pads, striped round their edges, each with a chute into the hall at its back: each
    // turned along its own way out.
    let (half_x, half_z) = units::REFINERY_PAD_HALF;
    let (hub_x, hub_z) = units::REFINERY_PAD_HUB;
    for pad in 0..units::REFINERY_PAD_TURNS.len() {
        let ((px, pz), (out_x, out_z)) = units::refinery_pad(pad);
        let heading = out_z.atan2(out_x);
        turned_part((px, pz, half_z, half_x, heading), (y, y + 0.05), STRIPE);
        turned_part(
            (px, pz, half_z - 0.06, half_x - 0.06, heading),
            (y + 0.05, y + 0.06),
            DECK,
        );
        let chute = (hub_x + out_x, hub_z + out_z);
        let chute_part = |half_width| (chute.0, chute.1, 0.5, half_width, heading);
        turned_part(chute_part(0.32), (y, y + 0.7), BUILDING_DOOR);
        turned_part(chute_part(0.36), (y + 0.7, y + 0.78), color);
    }
}

fn draw_unit_model(canvas: &mut Canvas, game: &Game, unit: &Unit) {
    let [r, g, b, _] = team_color(unit.color);
    let color = [r, g, b];
    if original::draw(canvas, unit, color, game.battle_time()) {
        return;
    }
    if unit.role == Role::Mcv {
        models::draw_mcv(canvas, unit, unit.house, game.battle_time(), color);
        return;
    }
    match unit.kind {
        UnitKind::Infantry => models::draw_infantry(canvas, unit, game.battle_time(), color),
        UnitKind::Vehicle | UnitKind::Aircraft => {
            models::draw_machine(canvas, unit, game.battle_time(), color);
        }
        UnitKind::Building => {
            let drawn = canvas.objects.len();
            if unit.role == Role::Helipad {
                draw_helipad(canvas, unit, color);
            } else if unit.role == Role::Refinery {
                draw_refinery(canvas, unit, color);
            } else {
                let open = game.launcher_open(unit);
                draw_building(canvas, unit, color, game.battle_time(), open);
            }
            lay_on_grid(canvas, drawn, unit);
        }
        UnitKind::Tower if unit.role.pops_up() => {
            models::draw_or_pop_up_turret(canvas, unit, color, house_paint(unit.house).0)
        }
        UnitKind::Tower => {
            let drawn = canvas.objects.len();
            draw_tower(canvas, unit, color);
            stretch_drawn(canvas, drawn, unit, 0.0);
        }
    }
}

/// A tower made in its own size, its head turned to its heading.
fn draw_tower(canvas: &mut Canvas, unit: &Unit, color: [f32; 3]) {
    let (forward_x, forward_z) = (unit.heading.cos(), unit.heading.sin());
    let ahead = |by: f32| (unit.x + forward_x * by, unit.z + forward_z * by);
    let mut part = |footprint, heights, color| draw_turned_box(canvas, footprint, heights, color);
    let wall_of = |half: f32| (unit.x, unit.z, half, half, 0.0);
    let wall = wall_of(0.5);
    let head_base = units::TUBE_HEIGHT - 0.2;
    let (wall_color, head_color) = house_paint(unit.house);
    part(wall, (unit.y, unit.y + head_base), wall_color);
    let head = (unit.x, unit.z, 0.3, 0.3, unit.heading);
    let top_of_head = (unit.y + head_base, unit.y + head_base + 0.34);
    part(head, top_of_head, head_color);
    let band = (unit.x, unit.z, 0.31, 0.31, unit.heading);
    part(band, (top_of_head.0, top_of_head.0 + 0.06), color);
    let (ahead_x, ahead_z) = ahead(0.4);
    let height = (
        unit.y + units::TUBE_HEIGHT - 0.06,
        unit.y + units::TUBE_HEIGHT + 0.06,
    );
    if unit.role == Role::HkFlameTurret {
        // One fat nozzle, with the color of fire on its way out.
        let nozzle = (ahead_x, ahead_z, 0.3, 0.1, unit.heading);
        part(nozzle, height, FIRE_COLOR);
        return;
    }
    if unit.role == Role::AtPillbox {
        // One machine gun on a low parapet of sandbags.
        part(wall_of(0.62), (unit.y, unit.y + 0.3), SANDBAGS);
        let gun = (ahead_x, ahead_z, 0.32, 0.035, unit.heading);
        part(gun, height, GUNMETAL);
        return;
    }
    // A searchlight on the roof, turning with the head.
    let roof = unit.y + head_base + 0.34;
    let (lamp_x, lamp_z) = ahead(0.05);
    part(
        (lamp_x, lamp_z, 0.08, 0.07, unit.heading),
        (roof, roof + 0.12),
        SEARCHLIGHT_HOUSING,
    );
    let (lens_x, lens_z) = ahead(0.135);
    part(
        (lens_x, lens_z, 0.006, 0.055, unit.heading),
        (roof + 0.015, roof + 0.105),
        SEARCHLIGHT_LENS,
    );
    if unit.role == Role::HkGunTurret {
        // Two machine guns side by side on a breech block.
        let right = (-forward_z, forward_x);
        let (breech_x, breech_z) = ahead(0.14);
        let breech = (
            breech_x,
            breech_z,
            0.12,
            units::TUBE_SPREAD + 0.06,
            unit.heading,
        );
        let gun_height = unit.y + units::TUBE_HEIGHT;
        part(breech, (gun_height - 0.08, gun_height + 0.07), GUNMETAL);
        for side in [-units::TUBE_SPREAD, units::TUBE_SPREAD] {
            let (x, z) = ahead(0.45);
            let barrel = (
                x + right.0 * side,
                z + right.1 * side,
                0.25,
                0.03,
                unit.heading,
            );
            part(barrel, (gun_height - 0.03, gun_height + 0.03), GUNMETAL);
        }
        return;
    }
    for side in [-units::TUBE_SPREAD, units::TUBE_SPREAD] {
        draw_tube(canvas, unit, side, color.map(|c| c * 0.5));
    }
}

/// A square tube of a rocket turret, hinged at its back end and raised to its pitch.
fn draw_tube(canvas: &mut Canvas, unit: &Unit, side: f32, color: [f32; 3]) {
    const HALF: f32 = 0.11;
    /// How far behind its hinge a launcher tube reaches.
    const BACK: f32 = 0.3;
    let (sin, cos) = unit.heading.sin_cos();
    let (rise, out) = unit.tube_pitch.sin_cos();
    // A point given along the tube, across it to the right, and up from its middle.
    let point = |along: f32, across: f32, up: f32| {
        let ahead = units::TUBE_HINGE + along * out - up * rise;
        let right = side + across;
        (
            unit.x + cos * ahead - sin * right,
            unit.y + units::TUBE_HEIGHT + along * rise + up * out,
            unit.z + sin * ahead + cos * right,
        )
    };
    let length = units::TUBE_LENGTH;
    let way = [cos * out, rise, sin * out];
    let across = [-sin, 0.0, cos];
    let up = [-cos * rise, out, -sin * rise];
    let flip = |v: [f32; 3]| v.map(|c| -c);
    let faces = [
        (
            (-BACK, HALF, -HALF),
            (length, HALF, -HALF),
            (length, HALF, HALF),
            (-BACK, HALF, HALF),
            across,
            color,
        ),
        (
            (-BACK, -HALF, HALF),
            (length, -HALF, HALF),
            (length, -HALF, -HALF),
            (-BACK, -HALF, -HALF),
            flip(across),
            color,
        ),
        (
            (-BACK, -HALF, HALF),
            (length, -HALF, HALF),
            (length, HALF, HALF),
            (-BACK, HALF, HALF),
            up,
            color,
        ),
        (
            (-BACK, -HALF, -HALF),
            (length, -HALF, -HALF),
            (length, HALF, -HALF),
            (-BACK, HALF, -HALF),
            flip(up),
            color,
        ),
        // The open end, dark inside.
        (
            (length, -HALF, -HALF),
            (length, HALF, -HALF),
            (length, HALF, HALF),
            (length, -HALF, HALF),
            way,
            color.map(|c| c * 0.25),
        ),
        (
            (-BACK, -HALF, -HALF),
            (-BACK, HALF, -HALF),
            (-BACK, HALF, HALF),
            (-BACK, -HALF, HALF),
            flip(way),
            color,
        ),
    ];
    for (a, b, c, d, normal, color) in faces {
        let at = |(along, side, height): (f32, f32, f32)| point(along, side, height);
        canvas.solid_quad([at(a), at(b), at(c), at(d)], normal, (color, color));
    }
}

/// Where a knee lies between a hip and a foot, given as (ahead, up) with the leg bending
/// forward. Out of reach, the leg goes straight towards the foot.
pub fn knee_of(hip: (f32, f32), foot: (f32, f32), thigh: f32, shin: f32) -> (f32, f32) {
    let (dx, dy) = (foot.0 - hip.0, foot.1 - hip.1);
    let span = dx.hypot(dy).clamp(0.0001, thigh + shin);
    let length = dx.hypot(dy).max(0.0001);
    let (along_x, along_y) = (dx / length, dy / length);
    let along = (thigh * thigh - shin * shin + span * span) / (2.0 * span);
    let out = (thigh * thigh - along * along).max(0.0).sqrt();
    // A leg going down has its forward side to the left of that way.
    (
        hip.0 + along_x * along - along_y * out,
        hip.1 + along_y * along + along_x * out,
    )
}

/// Stretches what was just drawn of a building to the original's size, and turns it to line up
/// with the map's grid squares.
fn lay_on_grid(canvas: &mut Canvas, drawn: usize, unit: &Unit) {
    stretch_drawn(canvas, drawn, unit, unit.facing.turn());
}

/// Stretches what was just drawn of a unit to the original's size about where it stands, and
/// turns it this far.
fn stretch_drawn(canvas: &mut Canvas, drawn: usize, unit: &Unit, turn: f32) {
    let [stretch_x, stretch_y, stretch_z] = unit.stretch();
    let (sin, cos) = turn.sin_cos();
    for vertex in &mut canvas.objects[drawn..] {
        let (dx, dz) = (
            (vertex.x - unit.x) * stretch_x,
            (vertex.z - unit.z) * stretch_z,
        );
        vertex.x = unit.x + dx * cos - dz * sin;
        vertex.z = unit.z + dx * sin + dz * cos;
        vertex.y = unit.y + (vertex.y - unit.y) * stretch_y;
        let [nx, ny, nz] = vertex.normal;
        let [nx, ny, nz] = unit_vector([nx / stretch_x, ny / stretch_y, nz / stretch_z]);
        vertex.normal = [nx * cos - nz * sin, ny, nx * sin + nz * cos];
    }
}

/// A barracks, factory, hangar, palace or outpost, with its door on its south side, in its
/// house's colors with touches of its side's. `open` is how far a palace's launcher is.
fn draw_building(canvas: &mut Canvas, unit: &Unit, color: [f32; 3], time: f32, open: f32) {
    let (x, z, y) = (unit.x, unit.z, unit.y);
    let (walls, roof) = house_paint(unit.house);
    let mut part = |footprint: (f32, f32, f32, f32), heights, color| {
        let (x, z, half_x, half_z) = footprint;
        let turned = (x, z, half_x, half_z, 0.0);
        draw_turned_box(canvas, turned, heights, color);
    };
    let (half_z, door_half, door_top) = match unit.role {
        Role::Barracks => {
            part((x, z, 1.0, 0.7), (y, y + 0.5), walls);
            part((x, z, 1.0, 0.35), (y + 0.5, y + 0.68), roof);
            (0.7, 0.15, 0.35)
        }
        Role::Factory => {
            let [pipe_x, pipe_z] = units::FACTORY_PIPE;
            let top = units::FACTORY_PIPE_TOP;
            part((x + pipe_x, z + pipe_z, 0.13, 0.13), (y, y + top), roof);
            let band = (x + pipe_x, z + pipe_z, 0.14, 0.14);
            part(band, (y + top - 0.25, y + top - 0.15), color);
            part((x, z, 1.05, 0.85), (y, y + 0.85), walls);
            (0.85, 0.45, 0.55)
        }
        // A keep between four corner turrets.
        Role::Palace(Faction::Atreides) => {
            part((x, z, 1.15, 0.95), (y, y + 0.7), walls);
            part((x, z, 0.6, 0.5), (y + 0.7, y + 1.5), roof);
            for (side_x, side_z) in [(-1.0, -0.8), (1.0, -0.8), (-1.0, 0.8), (1.0, 0.8)] {
                part((x + side_x, z + side_z, 0.2, 0.2), (y, y + 1.1), walls);
            }
            (0.95, 0.3, 0.45)
        }
        // A stepped pyramid.
        Role::Palace(Faction::Harkonnen) => {
            for (half, from, to, paint) in [
                (1.95, 0.0, 1.0, walls),
                (1.45, 1.0, 2.2, roof),
                (1.0, 2.2, 3.4, walls),
                (0.62, 3.4, 4.6, roof),
            ] {
                part((x, z, half, half), (y + from, y + to), paint);
                part(
                    (x, z, half + 0.01, half + 0.01),
                    (y + to - 0.09, y + to),
                    color,
                );
            }
            // A rocket that rises out of the top as the hatches over it slide apart.
            let rise = open * 1.2;
            part((x, z, 0.12, 0.12), (y + 3.6 + rise, y + 4.3 + rise), HATCH);
            part((x, z, 0.07, 0.07), (y + 4.3 + rise, y + 4.55 + rise), color);
            for side in [-1.0, 1.0] {
                let (rail_x, rail_z) = (x + side * 0.2, z);
                part(
                    (rail_x, rail_z, 0.03, 0.03),
                    (y + 3.8 + rise, y + 4.5 + rise),
                    HATCH,
                );
                let slide = 0.31 + 0.3 * open;
                part((x, z + side * slide, 0.6, 0.3), (y + 4.6, y + 4.7), HATCH);
            }
            (1.95, 0.4, 0.7)
        }
        // A squat block under a mast with a dish going round.
        Role::Outpost => {
            part((x, z, 1.15, 1.15), (y, y + 0.7), walls);
            part((x, z, 0.5, 0.5), (y + 0.7, y + 1.0), roof);
            part((x, z, 0.08, 0.08), (y + 1.0, y + 1.65), HATCH);
            (1.15, 0.25, 0.4)
        }
        // A long hall under two thin spires.
        Role::Palace(Faction::Ordos) => {
            part((x, z, 1.3, 0.7), (y, y + 0.8), walls);
            part((x, z, 1.1, 0.45), (y + 0.8, y + 1.0), roof);
            for side in [-0.8, 0.8] {
                part((x + side, z, 0.1, 0.1), (y + 1.0, y + 2.2), walls);
            }
            (0.7, 0.35, 0.5)
        }
        _ => {
            part((x, z, 1.1, 0.9), (y, y + 0.6), walls);
            part((x, z, 0.9, 0.9), (y + 0.6, y + 0.76), roof);
            part((x, z, 0.55, 0.9), (y + 0.76, y + 0.86), walls);
            (0.9, 0.75, 0.5)
        }
    };
    let door = (x, z + half_z + 0.01, door_half, 0.01);
    part(door, (y, y + door_top), BUILDING_DOOR);
    let lintel = (x, z + half_z + 0.012, door_half + 0.06, 0.012);
    part(lintel, (y + door_top, y + door_top + 0.07), color);
    let banner = (x, z + half_z + 0.012, 0.09, 0.012);
    part(banner, (y + door_top + 0.1, y + door_top + 0.34), color);
    if unit.role == Role::Outpost {
        // The radar dish, turning. It stands above anything the door's side can hide.
        let spin = time * RADAR_TURNS;
        let beam = (x, z, 0.62, 0.05, spin);
        draw_turned_box(canvas, beam, (y + 1.65, y + 1.76), HATCH);
        let dish = (
            x + spin.cos() * 0.58,
            z + spin.sin() * 0.58,
            0.05,
            0.3,
            spin,
        );
        draw_turned_box(canvas, dish, (y + 1.55, y + 1.95), color);
    }
}

/// How fast an outpost's radar goes round, in radians per second.
const RADAR_TURNS: f32 = 2.0;
/// The cold steel of a palace's rocket and an outpost's radar.
const HATCH: [f32; 3] = [0.55, 0.56, 0.6];

/// A box turned to any heading: (x, z, half length, half width, heading). The air reaches its
/// foot less, so its sides darken towards it.
fn draw_turned_box(
    canvas: &mut Canvas,
    (x, z, half_length, half_width, heading): (f32, f32, f32, f32, f32),
    (base, top): (f32, f32),
    color: [f32; 3],
) {
    const FOOT: f32 = 0.72;
    let (forward_x, forward_z) = (heading.cos(), heading.sin());
    let (right_x, right_z) = (-forward_z, forward_x);
    let corner = |along: f32, across: f32| {
        (
            x + forward_x * along * half_length + right_x * across * half_width,
            z + forward_z * along * half_length + right_z * across * half_width,
        )
    };
    let corners = [
        corner(1.0, -1.0),
        corner(1.0, 1.0),
        corner(-1.0, 1.0),
        corner(-1.0, -1.0),
    ];
    let foot = color.map(|channel| channel * FOOT);
    for i in 0..corners.len() {
        let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
        let (mid_x, mid_z) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let normal = unit_vector([mid_x - x, 0.0, mid_z - z]);
        canvas.solid_quad(
            [
                (a.0, base, a.1),
                (b.0, base, b.1),
                (b.0, top, b.1),
                (a.0, top, a.1),
            ],
            normal,
            (foot, color),
        );
    }
    let [a, b, c, d] = corners;
    let flat = |y: f32| [(a.0, y, a.1), (b.0, y, b.1), (c.0, y, c.1), (d.0, y, d.1)];
    canvas.solid_quad(flat(top), [0.0, 1.0, 0.0], (color, color));
    canvas.solid_quad(flat(base), [0.0, -1.0, 0.0], (foot, foot));
}

fn draw_selected_ring(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    const SIDES: usize = 16;
    let radius = unit.radius() + 4.525;
    let thickness = (2.0 * ui_scale(game.height)).max(1.0);
    let point = |i: usize| {
        let (sin, cos) = (i as f32 / SIDES as f32 * TAU).sin_cos();
        (
            unit.x + radius * cos,
            unit.y + OVER_GROUND,
            unit.z + radius * sin,
        )
    };
    for i in 0..SIDES {
        canvas.deep_line(view, point(i), point(i + 1), thickness, SELECTED_RING);
    }
}

/// The selection's four edges as flickering lightning, where they lie on the ground.
fn draw_selection_box(canvas: &mut Canvas, game: &Game) {
    /// New bolts per second.
    const FLICKER: f32 = 24.0;
    let s = ui_scale(game.height);
    let frame = (game.screen_elapsed() * FLICKER) as u32;
    for (i, (a, b)) in game.selection_edges().into_iter().enumerate() {
        let seed = frame.wrapping_mul(31).wrapping_add(i as u32 * 1009);
        let bolt = units::lightning_points(a, b, seed, 26.0 * s, 9.0 * s);
        for layer in [(9.0, BOLT_GLOW), (4.0, BOLT_HALO), (1.6, BOLT_CORE)] {
            for pair in bolt.windows(2) {
                canvas.line(pair[0], pair[1], (layer.0 * s).max(1.0), layer.1);
            }
        }
        // Thin forks spitting off every few points.
        for (j, &start) in bolt.iter().enumerate().skip(2).step_by(4) {
            let side = if (frame + j as u32).is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = dx.hypot(dy).max(1.0);
            let reach = 22.0 * s;
            let end = (
                start.0 + dx / length * reach * 0.6 - dy / length * reach * side,
                start.1 + dy / length * reach * 0.6 + dx / length * reach * side,
            );
            let fork =
                units::lightning_points(start, end, seed.wrapping_add(j as u32), 9.0 * s, 4.0 * s);
            for pair in fork.windows(2) {
                canvas.line(pair[0], pair[1], (1.2 * s).max(1.0), BOLT_HALO);
            }
        }
    }
}

fn draw_money(canvas: &mut Canvas, game: &Game) {
    let px = small_text_px(game.height);
    let text = format!("${}", game.player_money());
    let (right, cy) = hud::money_anchor(game.width, game.height);
    canvas.text_left(&text, right - font::text_width(&text) * px, cy, px, GOLD);
}

/// An arc from 7 o'clock to 12 o'clock around the minimap. The filled part shows the energy,
/// shading from red at its start towards green at the far end of the scale.
fn draw_energy_gauge(canvas: &mut Canvas, game: &Game) {
    const SEGMENTS: usize = 48;
    let (cx, cy, inner, outer) = hud::gauge(game.width, game.height);
    let span = hud::GAUGE_END_DEGREES - hud::GAUGE_START_DEGREES;
    let filled = game.energy as f32 / hud::MAX_ENERGY as f32;
    let shade = |t: f32| -> Color {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        [
            mix(ENERGY_LOW[0], ENERGY_HIGH[0]),
            mix(ENERGY_LOW[1], ENERGY_HIGH[1]),
            mix(ENERGY_LOW[2], ENERGY_HIGH[2]),
            1.0,
        ]
    };
    for i in 0..SEGMENTS {
        let (from, to) = (i as f32 / SEGMENTS as f32, (i + 1) as f32 / SEGMENTS as f32);
        let at = |t: f32, r: f32| hud::clock_point(cx, cy, r, hud::GAUGE_START_DEGREES + t * span);
        let (a, b) = (at(from, inner), at(to, inner));
        let (c, d) = (at(to, outer), at(from, outer));
        let lit = from < filled;
        let (color_from, color_to) = if lit {
            (shade(from), shade(to))
        } else {
            (GAUGE_EMPTY, GAUGE_EMPTY)
        };
        canvas.vertices.extend(
            [
                (a, color_from),
                (b, color_to),
                (c, color_to),
                (a, color_from),
                (c, color_to),
                (d, color_from),
            ]
            .map(|(point, color)| Vertex::new(point, color)),
        );
    }
    let (x, y) = hud::energy_label_center(game.width, game.height);
    canvas.text_centered(
        &game.energy.to_string(),
        x,
        y,
        small_text_px(game.height),
        TEXT,
    );
}

/// The fake buttons, the tabs with their symbols, and the active tab's grid.
fn draw_sidebar(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    for b in hud::command_buttons(game.width, game.height) {
        draw_button(canvas, game, b);
    }
    let letter = match game.build_tab {
        BuildTab::Infantry => 'I',
        BuildTab::Vehicles => 'V',
        BuildTab::Buildings => 'B',
    };
    for (i, cell) in hud::grid_cells(game.width, game.height)
        .into_iter()
        .enumerate()
    {
        draw_button(canvas, game, cell);
        let (cx, cy) = (cell.x + cell.w / 2.0, cell.y + cell.h / 2.0);
        if game.build_tab == BuildTab::Vehicles && i == SUPERWEAPON_CELL {
            let armed = game.superweapon().is_some();
            let ready = armed && game.strike_recharge <= 0.0;
            let color = if !ready {
                QUIET
            } else if game.aiming_strike {
                SELECTED_RING
            } else {
                GOLD
            };
            draw_radiation_symbol(canvas, cx, cy, cell.w * 0.7, color);
            if armed && game.strike_recharge > 0.0 {
                let seconds = (game.strike_recharge.ceil() as i32).to_string();
                canvas.text_centered(&seconds, cx, cy, small_text_px(game.height), TEXT);
            }
            continue;
        }
        // Tip text gives the full name of what it makes.
        let makes = game.build_role(game.build_tab, i);
        let can_make =
            makes.is_some_and(|role| game.can_produce(role) && !game.is_held_up_by_site(role));
        let symbol = if can_make { GOLD } else { GOLD_DIM };
        draw_tab_symbol(canvas, game.build_tab, cx, cy, cell.w * 0.5, symbol);
        let label = makes.map_or(format!("{letter}{}", i + 1), |role| {
            short_name(role).to_string()
        });
        canvas.text_left(
            &label,
            cell.x + 6.0 * s,
            cell.y + 14.0 * s,
            chat_px(game),
            if can_make || makes.is_none() {
                TEXT
            } else {
                QUIET
            },
        );
        if let Some(progress) = makes.and_then(|role| game.making_progress(role)) {
            for wedge in making_dim_wedges((cell.x, cell.y, cell.w, cell.h), progress) {
                canvas.triangle(wedge, MAKING_DIM);
            }
        }
        if makes.is_some_and(|role| game.is_ready_to_place(role)) {
            canvas.text_centered("Ready", cx, cell.y + cell.h - 12.0 * s, chat_px(game), GOLD);
        } else if let Some(role) = makes {
            // Its price, dimmed while there is not that much money.
            let price = format!("${}", role.cost());
            let affordable = role.cost() <= game.player_money();
            let color = if can_make && affordable { TEXT } else { QUIET };
            let y = cell.y + cell.h - 12.0 * s;
            canvas.text_left(&price, cell.x + 6.0 * s, y, chat_px(game), color);
        }
        if let Some(count) = makes.and_then(|role| game.queue_badge(role)) {
            let px = small_text_px(game.height);
            let (x, y) = (cell.x + cell.w - 12.0 * s, cell.y + cell.h - 12.0 * s);
            canvas.text_centered(&count.to_string(), x, y, px, GOLD);
        }
    }
    for tab in hud::build_tabs(game.width, game.height) {
        let Action::BuildTab(kind) = tab.action else {
            continue;
        };
        let selected = kind == game.build_tab;
        // A tab with nothing to make it greys out and does not light up.
        let enabled = game.tab_enabled(kind);
        let hovered = enabled && game.hovered() == Some(tab.action);
        let border = border_px(game.height);
        canvas.rect(
            tab.x,
            tab.y,
            tab.w,
            tab.h,
            if selected || hovered { GOLD } else { BORDER },
        );
        let fill = if selected {
            PANEL
        } else {
            button_fill(tab.action, game.hovered(), game.pressed)
        };
        canvas.rect(
            tab.x + border,
            tab.y + border,
            tab.w - border * 2.0,
            tab.h - border * 2.0,
            fill,
        );
        let color = if !enabled {
            QUIET
        } else if selected {
            GOLD
        } else {
            TEXT
        };
        draw_tab_symbol(
            canvas,
            kind,
            tab.x + tab.w / 2.0,
            tab.y + tab.h / 2.0,
            tab.w * 0.7,
            color,
        );
    }
}

/// A name short enough for a build menu cell.
fn short_name(role: Role) -> &'static str {
    match role {
        Role::AtInfantry => "Inf",
        Role::Engineer => "Engin",
        Role::AtKindjal => "Kindjal",
        Role::AtRepairUnit => "Repair",
        Role::HkInkVine => "Inkvine",
        Role::HkMissile => "MslTank",
        Role::AtMongoose => "Mongse",
        Role::AtMinotaurus => "Minotr",
        Role::AdvCarryall => "AdvCarry",
        Role::Carryall => "Carry",
        Role::HkAdp => "AirDef",
        Role::AtAdp => "Drone",
        Role::AtOrni => "Ornith",
        Role::HkGunship => "Gunship",
        Role::Helipad => "Pad",
        Role::Refinery => "Refnry",
        Role::Harvester => "Harvest",
        Role::Palace(_) => "Palace",
        Role::AtRocketTurret => "R Turret",
        Role::HkGunTurret => "Gun Trt",
        Role::AtPillbox => "MG Post",
        Role::Outpost => "Outpost",
        Role::OrGasTurret => "Gas Trt",
        Role::OrPopUpTurret => "Pop-up",
        Role::HkFlamer => "Flame",
        Role::OrChemical => "Chem",
        Role::HkLightInf => "Light",
        Role::HkTrooper => "Trooper",
        Role::OrSaboteur => "Sabotr",
        Role::OrMortar => "Mortar",
        Role::AtTrike => "Bike",
        Role::HkBuzzsaw => "Buzzsaw",
        Role::HkFlame => "FlmTank",
        Role::HkDevastator => "Devastator",
        Role::OrLaserTank => "Laser",
        Role::OrKobra => "Kobra",
        Role::OrApc | Role::AtApc => "APC",
        Role::OrAdp => "AA Mine",
        Role::OrEits => "Eye",
        Role::HkAssault => "Assault",
        _ => role.name(),
    }
}

fn chat_px(game: &Game) -> f32 {
    hud::chat_text_px(game.height)
}

/// The part of a button still dimmed while its unit is made: all of it at first, then less as
/// a clear sector grows clockwise from twelve o'clock.
pub fn making_dim_wedges(
    (x, y, w, h): (f32, f32, f32, f32),
    progress: f32,
) -> Vec<[(f32, f32); 3]> {
    const WEDGES: usize = 48;
    let middle = (x + w / 2.0, y + h / 2.0);
    // The button's edge straight out from its middle, clockwise from straight up.
    let edge = |turn: f32| {
        let (sin, cos) = (turn * TAU).sin_cos();
        let reach = (w / 2.0 / sin.abs().max(0.0001)).min(h / 2.0 / cos.abs().max(0.0001));
        (middle.0 + sin * reach, middle.1 - cos * reach)
    };
    let start = progress.clamp(0.0, 1.0);
    if start >= 1.0 {
        return Vec::new();
    }
    let step = (1.0 - start) / WEDGES as f32;
    (0..WEDGES)
        .map(|i| {
            let from = start + step * i as f32;
            [middle, edge(from), edge(from + step)]
        })
        .collect()
}

/// Small vector pictures built from the same triangles as everything else.
fn draw_tab_symbol(canvas: &mut Canvas, tab: BuildTab, cx: f32, cy: f32, size: f32, c: Color) {
    let u = size / 10.0;
    match tab {
        BuildTab::Infantry => {
            // A soldier: head, body, legs, and a rifle held across.
            canvas.circle(cx, cy - 3.6 * u, 1.2 * u, c);
            canvas.line((cx, cy - 2.4 * u), (cx, cy + 1.0 * u), 1.6 * u, c);
            canvas.line((cx, cy + 0.8 * u), (cx - 1.6 * u, cy + 4.6 * u), 1.1 * u, c);
            canvas.line((cx, cy + 0.8 * u), (cx + 1.6 * u, cy + 4.6 * u), 1.1 * u, c);
            canvas.line(
                (cx - 2.8 * u, cy + 0.2 * u),
                (cx + 3.6 * u, cy - 2.6 * u),
                0.8 * u,
                c,
            );
        }
        BuildTab::Vehicles => {
            // A tank: tracks, hull, turret and barrel.
            canvas.rect(cx - 4.6 * u, cy + 1.2 * u, 9.2 * u, 2.2 * u, c);
            canvas.rect(cx - 3.8 * u, cy - 0.6 * u, 7.6 * u, 1.4 * u, c);
            canvas.rect(cx - 2.0 * u, cy - 2.6 * u, 3.8 * u, 1.7 * u, c);
            canvas.line(
                (cx + 1.6 * u, cy - 1.8 * u),
                (cx + 5.0 * u, cy - 2.6 * u),
                0.7 * u,
                c,
            );
        }
        BuildTab::Buildings => {
            // A factory: walls under a sawtooth roof, with a chimney.
            let (left, right, top, bottom) =
                (cx - 4.4 * u, cx + 4.4 * u, cy - 0.4 * u, cy + 4.0 * u);
            canvas.rect(left, top, right - left, bottom - top, c);
            let tooth = (right - left) / 3.0;
            for i in 0..3 {
                let x = left + i as f32 * tooth;
                canvas.triangle([(x, top), (x + tooth, top), (x + tooth, top - 2.2 * u)], c);
            }
            canvas.rect(left + 0.6 * u, top - 4.0 * u, 1.3 * u, 4.0 * u, c);
        }
    }
}

/// The last few messages, on small dark strips so they read over any terrain.
fn draw_chat(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = chat_px(game);
    for (i, line) in game.chat.iter().enumerate() {
        let (x, cy) = hud::chat_line(i, game.height);
        let (w, h) = (font::text_width(&line.text) * px, font::CAP_HEIGHT * px);
        let pad = 4.0 * s;
        canvas.rect(
            x - pad,
            cy - h / 2.0 - pad,
            w + pad * 2.0,
            h + pad * 2.0,
            CHAT_BACKING,
        );
        canvas.text_left(&line.text, x, cy, px, TEXT);
    }
}

/// Everyone in this battle, in a window the battlefield shows through.
fn draw_diplomacy(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let border = border_px(game.height);
    let (x, y, w, h) = stats_rect(game.width, game.height);
    canvas.rect(x, y, w, border, BORDER);
    canvas.rect(x, y + h - border, w, border, BORDER);
    canvas.rect(x, y + border, border, h - border * 2.0, BORDER);
    canvas.rect(x + w - border, y + border, border, h - border * 2.0, BORDER);
    canvas.rect(
        x + border,
        y + border,
        w - border * 2.0,
        h - border * 2.0,
        WINDOW,
    );

    canvas.text_centered("Diplomacy", x + w / 2.0, y + 40.0 * s, 5.0 * s, GOLD);
    let (swatch_x, name_x, money_x, planet_x, faction_x, ally_x) = (
        x + 30.0 * s,
        x + 90.0 * s,
        x + 390.0 * s,
        x + 520.0 * s,
        x + 570.0 * s,
        x + 800.0 * s,
    );
    let header_y = y + 88.0 * s;
    canvas.text_left("Player", name_x, header_y, px, QUIET);
    canvas.text_left("Money", money_x, header_y, px, QUIET);
    canvas.text_left("Faction", faction_x, header_y, px, QUIET);
    canvas.text_left("Allies", ally_x - 22.0 * s, header_y, px, QUIET);

    for (i, row) in game.stats_rows().iter().enumerate() {
        let cy = y + (STATS_ROW_TOP + (i as f32 + 0.5) * STATS_ROW_STEP) * s;
        canvas.rect(
            swatch_x,
            cy - 18.0 * s,
            36.0 * s,
            36.0 * s,
            team_color(row.color),
        );
        let defeated = game.is_defeated(i);
        let (name_color, money_color) = if defeated {
            (QUIET, QUIET)
        } else {
            (TEXT, GOLD)
        };
        canvas.text_left(&row.name, name_x, cy, px, name_color);
        canvas.text_left(&format!("${}", row.money), money_x, cy, px, money_color);
        match row.faction {
            _ if defeated => canvas.text_left("Defeated", faction_x, cy, px, DEFEAT_RED),
            Some(faction) => {
                draw_planet(canvas, game, faction, (planet_x, cy, 26.0 * s), BORDER);
                canvas.text_left(faction.label(), faction_x, cy, px, TEXT);
            }
            None => canvas.text_left("None", faction_x, cy, px, QUIET),
        }
        for (slot, ally) in row.allies.iter().enumerate() {
            let ally_cx = ally_x + slot as f32 * 70.0 * s;
            canvas.bordered_circle((ally_cx, cy), 26.0 * s, border, BORDER, BUTTON);
            draw_ally(canvas, *ally, ally_cx, cy, 17.0 * s, (px - 1.0).max(1.0));
        }
    }
}

fn team_color(color: TeamColor) -> Color {
    match color {
        TeamColor::Blue => [0.20, 0.40, 0.90, 1.0],
        TeamColor::Red => [0.85, 0.18, 0.18, 1.0],
        TeamColor::Cyan => [0.20, 0.80, 0.85, 1.0],
        TeamColor::Orange => [0.95, 0.55, 0.15, 1.0],
        TeamColor::Green => [0.25, 0.70, 0.30, 1.0],
        TeamColor::Violet => [0.55, 0.30, 0.85, 1.0],
        TeamColor::Yellow => [0.95, 0.85, 0.25, 1.0],
        TeamColor::LightPink => [1.00, 0.72, 0.80, 1.0],
    }
}

/// Text color that stays readable on top of `color`.
fn text_on(color: TeamColor) -> Color {
    match color {
        TeamColor::Blue | TeamColor::Red | TeamColor::Violet => TEXT,
        TeamColor::Cyan
        | TeamColor::Orange
        | TeamColor::Green
        | TeamColor::Yellow
        | TeamColor::LightPink => INK,
    }
}

fn draw_opponents(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let border = border_px(game.height);
    let setup = &game.skirmish;
    let count = setup.ai_players as usize;
    let hovered = game.hovered();

    let (first_x, first_y, row_w, _) = opponent_row(0, game.width, game.height);
    canvas.text_centered(
        "Opponents",
        first_x + row_w / 2.0,
        first_y - 24.0 * s,
        px,
        TEXT,
    );

    for slot in count..OPPONENT_SLOTS {
        let (x, y, w, h) = opponent_row(slot, game.width, game.height);
        canvas.bordered_rect((x, y, w, h), border, BUTTON, CLEAR_RGBA);
        canvas.text_centered("Empty", x + w / 2.0, y + h / 2.0, px, QUIET);
    }

    for b in opponent_widgets(count, game.width, game.height) {
        let lit = hovered == Some(b.action);
        let edge = if lit { GOLD } else { BORDER };
        let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
        match b.action {
            Action::CycleOpponentKind(slot) => {
                let opponent = setup.opponents[slot];
                canvas.rect(b.x, b.y, b.w, b.h, edge);
                let fill = team_color(opponent.color);
                canvas.rect(
                    b.x + border,
                    b.y + border,
                    b.w - border * 2.0,
                    b.h - border * 2.0,
                    fill,
                );
                canvas.text_centered(opponent.kind.label(), cx, cy, px, text_on(opponent.color));
            }
            Action::CycleOpponentFaction(slot) => {
                draw_planet(
                    canvas,
                    game,
                    setup.opponents[slot].faction,
                    button_circle(b),
                    edge,
                );
            }
            Action::CycleOpponentAlly(slot, ally) => {
                canvas.bordered_circle((cx, cy), b.w / 2.0, border, edge, BUTTON);
                let label_px = (px - 1.0).max(1.0);
                draw_ally(
                    canvas,
                    setup.opponents[slot].allies[ally],
                    cx,
                    cy,
                    b.w * 0.32,
                    label_px,
                );
            }
            _ => {}
        }
    }
}

/// A bordered rectangle that lights up under the cursor.
fn draw_field(canvas: &mut Canvas, game: &Game, b: Button) {
    let border = border_px(game.height);
    let lit = game.hovered() == Some(b.action);
    canvas.bordered_rect(
        (b.x, b.y, b.w, b.h),
        border,
        if lit { GOLD } else { BORDER },
        BUTTON,
    );
}

fn draw_skirmish_setup(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let border = border_px(game.height);
    let setup = &game.skirmish;

    canvas.text_centered("Game Setup", game.width / 2.0, 80.0 * s, 6.0 * s, GOLD);
    draw_minimap(canvas, game);
    draw_opponents(canvas, game);

    for b in setup_widgets(game.width, game.height) {
        let lit = game.hovered() == Some(b.action);
        let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
        let edge = if lit { GOLD } else { BORDER };
        match b.action {
            Action::NameField => {
                canvas.text_left(b.label, b.x, b.y - 24.0 * s, px, TEXT);
                draw_field(canvas, game, b);
                let text_x = b.x + 16.0 * s;
                canvas.text_left(&setup.commander, text_x, cy, px, TEXT);
                let blink_on = game.screen_elapsed().fract() < 0.5;
                if game.name_focused && blink_on {
                    let caret_x =
                        text_x + (setup.commander.len() as f32 * crate::font::ADVANCE) * px;
                    canvas.rect(caret_x, cy - 4.0 * px, px, 8.0 * px, GOLD);
                }
            }
            Action::MapDropdown => {
                draw_field(canvas, game, b);
                if let Some(map) = crate::maps::list().get(setup.map) {
                    canvas.text_left(&map.name, b.x + 16.0 * s, cy, px, TEXT);
                }
                let (ax, half) = (b.x + b.w - 28.0 * s, 10.0 * s);
                let tip = if game.dropdown_open {
                    cy - half * 0.6
                } else {
                    cy + half * 0.6
                };
                let base = if game.dropdown_open {
                    cy + half * 0.6
                } else {
                    cy - half * 0.6
                };
                canvas.triangle([(ax - half, base), (ax + half, base), (ax, tip)], TEXT);
            }
            Action::SetDifficulty(difficulty) => {
                let r = 12.0 * s;
                let dot_x = b.x + r;
                canvas.bordered_circle((dot_x, cy), r, border, edge, BUTTON);
                if setup.difficulty == difficulty {
                    canvas.circle(dot_x, cy, r * 0.55, GOLD);
                }
                canvas.text_left(b.label, dot_x + r + 6.0 * s, cy, px, TEXT);
            }
            Action::ColorBar if game.color_open => {
                for slot in color_slots(&setup.available_colors(), game.width, game.height) {
                    let Action::PickColor(color) = slot.action else {
                        continue;
                    };
                    let lit = game.hovered() == Some(slot.action);
                    let current = color == setup.color;
                    canvas.bordered_rect(
                        (slot.x, slot.y, slot.w, slot.h),
                        border,
                        if lit || current { GOLD } else { INK },
                        team_color(color),
                    );
                }
            }
            Action::ColorBar => {
                canvas.rect(b.x, b.y, b.w, b.h, edge);
                let fill = team_color(setup.color);
                canvas.rect(
                    b.x + border,
                    b.y + border,
                    b.w - border * 2.0,
                    b.h - border * 2.0,
                    fill,
                );
            }
            Action::StartBattle => draw_button(canvas, game, b),
            Action::CycleFaction => {
                canvas.text_centered(setup.faction.label(), cx, b.y - 24.0 * s, px, TEXT);
                draw_planet(canvas, game, setup.faction, button_circle(b), edge);
                canvas.text_centered("Click to change", cx, b.y + b.h + 32.0 * s, px, QUIET);
            }
            Action::CycleAlly(slot) => {
                canvas.text_centered(b.label, cx, b.y - 24.0 * s, px, TEXT);
                canvas.bordered_circle((cx, cy), b.w / 2.0, border, edge, BUTTON);
                draw_ally(canvas, setup.allies[slot], cx, cy, b.w * 0.32, px);
            }
            _ => {}
        }
    }

    draw_sliders(canvas, game);
    draw_checkboxes(canvas, game, setup_checkboxes(game.width, game.height));
}

fn draw_ally(canvas: &mut Canvas, ally: Ally, cx: f32, cy: f32, r: f32, px: f32) {
    match ally {
        Ally::None => canvas.text_centered("None", cx, cy, px, QUIET),
        Ally::Tleilaxu => canvas.polygon(cx, cy, r, 3, -FRAC_PI_2, TEXT),
        Ally::Ix => canvas.polygon(cx, cy, r, 4, FRAC_PI_4, TEXT),
        Ally::Guild => canvas.polygon(cx, cy, r, 4, 0.0, TEXT),
        Ally::Fremen => canvas.polygon(cx, cy, r, 6, 0.0, TEXT),
        Ally::Sardaukar => canvas.star((cx, cy), (r, r * 0.42), 5, -FRAC_PI_2, TEXT),
    }
}

/// The selected map's real terrain as a diamond, with its four corners named.
fn draw_minimap(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let (x, y, size) = minimap_rect(game.width, game.height);
    draw_map_diamond(canvas, game, game.skirmish.map, None, (x, y, size));

    let (mid_x, mid_y) = (x + size / 2.0, y + size / 2.0);
    let inset = 16.0 * s;
    canvas.text_centered("N", mid_x + inset * 1.5, y + inset, px, TEXT);
    canvas.text_centered("S", mid_x + inset * 1.5, y + size - inset, px, TEXT);
    canvas.text_centered("E", x + size - inset, mid_y - inset * 1.5, px, TEXT);
    canvas.text_centered("W", x + inset, mid_y - inset * 1.5, px, TEXT);
}

/// The battlefield's minimap: terrain, a radar sweep, then the outline of the view.
fn draw_battle_minimap(canvas: &mut Canvas, game: &Game) {
    let rect = hud::minimap_rect(game.width, game.height);
    draw_map_diamond(canvas, game, game.battle_map, game.fog.as_ref(), rect);
    for zone in game
        .combat
        .poison
        .iter()
        .filter(|z| game.sight_at(z.x, z.z) == Sight::Visible)
    {
        let (x, y) = battlefield::world_to_minimap(rect, game.map_extent(), zone.x, zone.z);
        let radius = STRIKE_RADIUS / battlefield::minimap_span(game.map_extent()) * rect.2;
        canvas.circle(x, y, radius, with_alpha(POISON_FILL, 0.45));
    }
    let dot = (3.0 * ui_scale(game.height)).round().max(2.0);
    let enemies_on_radar = game.has_outpost();
    // A carryall bringing a harvester shows once it is over the map.
    let terrain = battlefield::terrain(game.battle_map);
    for unit in game.units.iter().filter(|u| {
        game.shows(u)
            && terrain.on_map(u.x, u.z)
            && (enemies_on_radar || !game.is_enemy_of_player(u))
    }) {
        let (x, y) = battlefield::world_to_minimap(rect, game.map_extent(), unit.x, unit.z);
        let color = if unit.selected {
            VIEW_OUTLINE
        } else {
            team_color(unit.color)
        };
        let size = if unit.kind.is_building() {
            dot * 2.0
        } else {
            dot
        };
        canvas.rect(x - size / 2.0, y - size / 2.0, size, size, color);
    }
    draw_radar_sweep(canvas, rect, game.battle_time());

    let view = battlefield::View::new(game.camera, game.width, game.height);
    let Some(corners) = view.ground_corners() else {
        return;
    };
    let on_map = corners.map(|(x, z)| battlefield::world_to_minimap(rect, game.map_extent(), x, z));
    let thickness = (2.0 * ui_scale(game.height)).max(1.0);
    for i in 0..on_map.len() {
        let (a, b) = (on_map[i], on_map[(i + 1) % on_map.len()]);
        if let Some((a, b)) = battlefield::clip_to_square(a, b, rect) {
            canvas.line(a, b, thickness, VIEW_OUTLINE);
        }
    }
}

/// A line turning around the minimap's center, fading out over the area it just crossed.
fn draw_radar_sweep(canvas: &mut Canvas, (x, y, size): (f32, f32, f32), seconds: f32) {
    const SECONDS_PER_TURN: f32 = 4.0;
    const TRAIL: f32 = 1.2;
    const WEDGES: usize = 40;
    const STRONGEST_ALPHA: f32 = 0.45;
    const SWEEP: [f32; 3] = [0.35, 1.0, 0.45];

    let half = size / 2.0;
    let center = (x + half, y + half);
    let leading = seconds / SECONDS_PER_TURN * TAU;
    // Reaches the square's edge in every direction, so the sweep fills the whole minimap.
    let on_edge = |angle: f32| {
        let (sin, cos) = angle.sin_cos();
        let reach = half / sin.abs().max(cos.abs());
        (center.0 + cos * reach, center.1 + sin * reach)
    };
    let vertex = |point: (f32, f32), a: f32| Vertex::new(point, with_alpha(SWEEP, a));
    for wedge in 0..WEDGES {
        let (near, far) = (
            wedge as f32 / WEDGES as f32,
            (wedge + 1) as f32 / WEDGES as f32,
        );
        let (a_near, a_far) = (
            STRONGEST_ALPHA * (1.0 - near),
            STRONGEST_ALPHA * (1.0 - far),
        );
        canvas.vertices.extend([
            vertex(center, a_near),
            vertex(on_edge(leading - near * TRAIL), a_near),
            vertex(on_edge(leading - far * TRAIL), a_far),
        ]);
    }
    canvas.line(
        center,
        on_edge(leading),
        (size / 150.0).max(1.0),
        with_alpha(SWEEP, 0.9),
    );
}

fn draw_map_diamond(
    canvas: &mut Canvas,
    game: &Game,
    map: usize,
    fog: Option<&Fog>,
    (x, y, size): (f32, f32, f32),
) {
    /// The most blocks of cells the minimap shows along a side of the map.
    const MOST_BLOCKS: usize = 96;
    let border = border_px(game.height);
    canvas.rect(
        x - border,
        y - border,
        size + border * 2.0,
        size + border * 2.0,
        BORDER,
    );
    canvas.rect(x, y, size, size, CLEAR_RGBA);

    let terrain = battlefield::terrain(map);
    let (width, height) = terrain.cells();
    let extent = terrain.extent();
    let step = width.max(height).div_ceil(MOST_BLOCKS);
    let to_screen = |u: usize, v: usize| {
        let (wx, wz) = battlefield::grid_to_world(u.min(width) as f32, v.min(height) as f32);
        battlefield::world_to_minimap((x, y, size), extent, wx, wz)
    };
    for v in (0..height).step_by(step) {
        for u in (0..width).step_by(step) {
            let (mid_u, mid_v) = (
                (u + step / 2).min(width - 1),
                (v + step / 2).min(height - 1),
            );
            let shade = fog.map_or(1.0, |fog| fog.at_grid(mid_u, mid_v).shade());
            let base = terrain.radar_at(mid_u, mid_v);
            let color = with_alpha(base.map(|channel| channel * shade), 1.0);
            let (a, b) = (to_screen(u, v), to_screen(u + step, v));
            let (c, d) = (to_screen(u + step, v + step), to_screen(u, v + step));
            canvas.triangle([a, b, c], color);
            canvas.triangle([a, c, d], color);
        }
    }
}

fn draw_sliders(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let hovered_slider = if game.pressed.is_none() {
        game.hovered_slider()
    } else {
        None
    };
    for slider in game.sliders() {
        let value = game.slider_value(slider.id);
        let active = game.dragging == Some(slider.id)
            || (game.dragging.is_none() && hovered_slider == Some(slider.id));
        let track_h = (8.0 * s).round().max(2.0);
        let track_y = slider.cy - track_h / 2.0;
        let handle_x = slider.x_of(value);
        let handle_w = 20.0 * s;

        canvas.text_left(slider.label, slider.label_x, slider.label_cy, px, TEXT);
        canvas.rect(slider.track_x, track_y, slider.track_w, track_h, BORDER);
        canvas.rect(
            slider.track_x,
            track_y,
            handle_x - slider.track_x,
            track_h,
            GOLD_DIM,
        );
        canvas.rect(
            handle_x - handle_w / 2.0,
            slider.cy - slider.handle_h / 2.0,
            handle_w,
            slider.handle_h,
            if active { GOLD } else { TEXT },
        );

        if slider.stops.is_empty() {
            let text = value.to_string();
            let text_x = slider.value_right - crate::font::text_width(&text) * px;
            canvas.text_left(&text, text_x, slider.label_cy, px, TEXT);
        } else {
            for (i, stop) in slider.stops.iter().enumerate() {
                let color = if i as i32 == value { GOLD } else { QUIET };
                let stop_x = slider.x_of(slider.min + i as i32);
                canvas.text_centered(stop, stop_x, slider.cy + 44.0 * s, px, color);
            }
        }
    }
}

fn draw_checkboxes(canvas: &mut Canvas, game: &Game, rows: Vec<Button>) {
    let px = small_text_px(game.height);
    let border = border_px(game.height);
    let hovered = game.hovered();
    let size = checkbox_size(game.height);
    for row in rows {
        let Action::Toggle(toggle) = row.action else {
            continue;
        };
        let cy = row.y + row.h / 2.0;
        canvas.text_left(row.label, row.x, cy, px, TEXT);
        let box_x = row.x + row.w - size;
        let box_y = cy - size / 2.0;
        canvas.bordered_rect(
            (box_x, box_y, size, size),
            border,
            if hovered == Some(row.action) {
                GOLD
            } else {
                BORDER
            },
            BUTTON,
        );
        if game.toggle_value(toggle) {
            let inset = size * 0.25;
            canvas.rect(
                box_x + inset,
                box_y + inset,
                size - inset * 2.0,
                size - inset * 2.0,
                GOLD,
            );
        }
    }
}

fn button_fill(action: Action, hovered: Option<Action>, pressed: Option<Action>) -> Color {
    match (hovered == Some(action), pressed == Some(action)) {
        (true, true) => BUTTON_PRESSED,
        (true, false) => BUTTON_HOVER,
        _ => BUTTON,
    }
}

#[cfg(test)]
mod thread_tests {
    use super::*;

    #[test]
    fn what_is_drawn_on_threads_comes_out_in_the_order_given() {
        let things: Vec<f32> = (0..500).map(|i| i as f32).collect();
        let mut canvas = Canvas::default();
        draw_on_threads(&mut canvas, &things, |canvas, &thing| {
            canvas.vertices.push(Vertex::new((thing, 0.0), [1.0; 4]));
        });
        let drawn: Vec<f32> = canvas.vertices.iter().map(|v| v.x).collect();
        assert_eq!(drawn, things);
    }
}

#[cfg(test)]
mod size_tests {
    use super::*;
    use crate::units::Facing;

    /// How big what was drawn of a unit comes out, as (along, up, across) in its own frame:
    /// along its heading for a machine, and across and back from its front for a building.
    fn drawn_size(canvas: &Canvas, unit: &Unit, turn: f32) -> [f32; 3] {
        let (sin, cos) = turn.sin_cos();
        let local: Vec<[f32; 3]> = canvas
            .objects
            .iter()
            .map(|v| {
                let (dx, dz) = (v.x - unit.x, v.z - unit.z);
                [dx * cos + dz * sin, v.y - unit.y, dz * cos - dx * sin]
            })
            .collect();
        let span = |axis: usize| {
            let lo = local.iter().map(|p| p[axis]).fold(f32::MAX, f32::min);
            let hi = local.iter().map(|p| p[axis]).fold(f32::MIN, f32::max);
            (lo, hi)
        };
        let ((x_lo, x_hi), (y_lo, y_hi), (z_lo, z_hi)) = (span(0), span(1), span(2));
        // What stands on the ground is as tall as its top; what flies, as thick as it is.
        let up = if unit.kind.flies() { y_hi - y_lo } else { y_hi };
        [x_hi - x_lo, up, z_hi - z_lo]
    }

    fn assert_original_size(role: Role, house: Faction, drawn: [f32; 3]) {
        let wanted = role.original_size(house);
        for (axis, (drawn, wanted)) in drawn.into_iter().zip(wanted).enumerate() {
            assert!(
                (drawn / wanted - 1.0).abs() < 0.03,
                "{role:?} of {house:?}, axis {axis}: drawn {drawn}, the original's {wanted}"
            );
        }
    }

    #[test]
    fn machines_come_out_as_big_as_the_originals() {
        let roles = [
            Role::HkAssault,
            Role::AtRepairUnit,
            Role::Mcv,
            Role::Harvester,
            Role::AtTrike,
            Role::HkBuzzsaw,
            Role::HkFlame,
            Role::HkDevastator,
            Role::OrDustScout,
            Role::OrLaserTank,
            Role::OrKobra,
            Role::OrApc,
            Role::AtApc,
            Role::HkInkVine,
            Role::HkMissile,
            Role::AtMongoose,
            Role::AtMinotaurus,
            Role::AdvCarryall,
            Role::Carryall,
            Role::HkAdp,
            Role::AtAdp,
            Role::AtOrni,
            Role::HkGunship,
            Role::OrAdp,
            Role::OrEits,
        ];
        for role in roles {
            let mut unit = Unit::for_test(0.0, 0.0, role.kind());
            (unit.role, unit.kind, unit.altitude, unit.heading) = (role, role.kind(), 0.0, 0.0);
            let mut canvas = Canvas::default();
            if role == Role::Mcv {
                models::draw_mcv(&mut canvas, &unit, unit.house, 0.0, [1.0; 3]);
            } else {
                models::draw_machine(&mut canvas, &unit, 0.0, [1.0; 3]);
            }
            assert_original_size(role, unit.house, drawn_size(&canvas, &unit, 0.0));
        }
    }

    #[test]
    fn towers_the_pad_and_a_deployed_mcv_come_out_as_big_as_the_originals() {
        for role in [
            Role::AtRocketTurret,
            Role::HkFlameTurret,
            Role::HkGunTurret,
            Role::AtPillbox,
            Role::OrGasTurret,
            Role::OrPopUpTurret,
        ] {
            let mut unit = Unit::for_test(0.0, 0.0, role.kind());
            (unit.role, unit.kind, unit.heading, unit.popped) = (role, role.kind(), 0.0, 1.0);
            let mut canvas = Canvas::default();
            if role.pops_up() {
                models::draw_or_pop_up_turret(&mut canvas, &unit, [1.0; 3], [1.0; 3]);
            } else {
                draw_tower(&mut canvas, &unit, [1.0; 3]);
                stretch_drawn(&mut canvas, 0, &unit, 0.0);
            }
            // Square on the grid, which lies along the world's axes.
            let turn = 0.0;
            let [_, up, _] = drawn_size(&canvas, &unit, turn);
            // Its squares are what it stands on: its guns may reach out past them.
            canvas.objects.retain(|v| v.y < 1.0);
            let [across, _, ahead] = drawn_size(&canvas, &unit, turn);
            assert_original_size(role, unit.house, [across, up, ahead]);
        }
        let mut pad = Unit::for_test(0.0, 0.0, UnitKind::Building);
        pad.role = Role::Helipad;
        let mut canvas = Canvas::default();
        draw_helipad(&mut canvas, &pad, [1.0; 3]);
        lay_on_grid(&mut canvas, 0, &pad);
        let drawn = drawn_size(&canvas, &pad, pad.facing.turn());
        assert_original_size(Role::Helipad, pad.house, drawn);
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let mut mcv = Unit::for_test(0.0, 0.0, UnitKind::Building);
            (mcv.role, mcv.house, mcv.deployed, mcv.heading) = (Role::Mcv, house, true, 0.0);
            let mut canvas = Canvas::default();
            models::draw_mcv(&mut canvas, &mcv, house, 0.0, [1.0; 3]);
            let (cols, rows) = Role::Mcv.plot_size(house);
            let yard = [
                rows as f32 * battlefield::CELL_SIDE,
                107.0,
                cols as f32 * battlefield::CELL_SIDE,
            ];
            for (drawn, wanted) in drawn_size(&canvas, &mcv, 0.0).into_iter().zip(yard) {
                assert!(
                    (drawn / wanted - 1.0).abs() < 0.03,
                    "{house:?}: {drawn} {wanted}"
                );
            }
        }
    }

    #[test]
    fn buildings_come_out_as_big_as_the_originals_on_their_grid_squares() {
        let houses = [Faction::Atreides, Faction::Harkonnen, Faction::Ordos];
        for house in houses {
            for role in [
                Role::Barracks,
                Role::Factory,
                Role::Hanger,
                Role::Refinery,
                Role::Outpost,
                Role::Palace(house),
            ] {
                for facing in [Facing::LeftBottom, Facing::RightTop] {
                    let mut unit = Unit::for_test(0.0, 0.0, UnitKind::Building);
                    (unit.role, unit.house, unit.facing) = (role, house, facing);
                    let mut canvas = Canvas::default();
                    if role == Role::Refinery {
                        draw_refinery(&mut canvas, &unit, [1.0; 3]);
                    } else {
                        draw_building(&mut canvas, &unit, [1.0; 3], 0.0, 0.0);
                    }
                    lay_on_grid(&mut canvas, 0, &unit);
                    let drawn = drawn_size(&canvas, &unit, facing.turn());
                    assert_original_size(role, house, drawn);
                }
            }
        }
    }
}
