//! Turns game state into a triangle list. Backends only ever see colored triangles.

use crate::battlefield::{self, MAP_CELLS};
use crate::font;
use crate::hud;
use crate::planet;
use crate::units::{self, Role, Target, Unit, UnitKind};
use crate::game::{
    Action, Ally, BuildTab, Button, CONFIRM_MESSAGE, Faction, Game, MAPS, OPPONENT_SLOTS, Outcome,
    SUPERWEAPON_CELL, STATS_ROW_STEP,
    STATS_ROW_TOP, Screen, TeamColor, buttons, checkbox_size, color_slots, map_items, minimap_rect,
    modal_rect, opponent_row, opponent_widgets, options_panel, setup_checkboxes, setup_widgets,
    small_text_px, stats_rect, ui_scale,
};
use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};
use crate::platform::Vertex;

type Color = [f32; 4];

pub const CLEAR: [f32; 3] = [0.05, 0.05, 0.08];
const INTRO_CLEAR: Color = [0.0, 0.0, 0.0, 1.0];
const CLEAR_RGBA: Color = [CLEAR[0], CLEAR[1], CLEAR[2], 1.0];
const TEXT: Color = [0.93, 0.89, 0.78, 1.0];
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
const POISON_FILL: [f32; 3] = [0.25, 0.95, 0.25];
const POISON_ALPHA: f32 = 0.32;
const LASER_GLOW: Color = [1.0, 0.25, 0.2, 0.35];
const LASER_CORE: Color = [1.0, 0.9, 0.85, 1.0];
const REPAIR_GLOW: Color = [0.3, 1.0, 0.5, 0.35];
const REPAIR_CORE: Color = [0.85, 1.0, 0.9, 1.0];
const FIRE_COLOR: [f32; 3] = [1.0, 0.5, 0.1];
const GAS_COLOR: [f32; 3] = [0.35, 0.9, 0.25];
const FLARE_GLOW: [f32; 3] = [1.0, 0.7, 0.2];
const FLARE_CORE: [f32; 3] = [1.0, 0.97, 0.8];
const BLAST_COLOR: [f32; 3] = [1.0, 0.62, 0.15];
const MISSILE_BODY: Color = [0.95, 0.95, 0.9, 1.0];
const MISSILE_FLAME: Color = [1.0, 0.6, 0.15, 0.8];
const SHELL_BODY: Color = [0.12, 0.12, 0.12, 1.0];
const SHADOW: Color = [0.0, 0.0, 0.0, 0.3];
const HEALTH_GOOD: Color = [0.3, 0.9, 0.3, 1.0];
const HEALTH_BAD: Color = [0.9, 0.2, 0.15, 1.0];
const DEFEAT_RED: Color = [0.92, 0.22, 0.18, 1.0];
const RUIN_CONCRETE: [f32; 3] = [0.56, 0.54, 0.50];
const RUIN_ON_MINIMAP: Color = [0.16, 0.15, 0.14, 1.0];
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

#[derive(Default)]
struct Canvas {
    vertices: Vec<Vertex>,
}

impl Canvas {
    fn triangle(&mut self, points: [(f32, f32); 3], c: Color) {
        for (x, y) in points {
            self.vertices.push(Vertex { x, y, r: c[0], g: c[1], b: c[2], a: c[3] });
        }
    }

    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, c: Color) {
        self.triangle([(x, y), (x + w, y), (x, y + h)], c);
        self.triangle([(x + w, y), (x + w, y + h), (x, y + h)], c);
    }

    /// A rectangle of one color, opaque along its middle and fading sideways to `end_alpha`.
    fn fading_rect(&mut self, x: f32, y: f32, w: f32, h: f32, [r, g, b]: [f32; 3], end_alpha: f32) {
        let vertex = |x: f32, y: f32, a: f32| Vertex { x, y, r, g, b, a };
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
        self.fading_rect(x, y + h - thickness, w, thickness, BANNER_LINE, BANNER_LINE_END_ALPHA);
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
    fn star(&mut self, cx: f32, cy: f32, outer: f32, inner: f32, points: usize, rotation: f32, c: Color) {
        let vertex = |i: usize| {
            let r = if i % 2 == 0 { outer } else { inner };
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
        let (nx, ny) = (-dy / length * thickness / 2.0, dx / length * thickness / 2.0);
        let (p, q) = ((a.0 + nx, a.1 + ny), (b.0 + nx, b.1 + ny));
        let (r, t) = ((b.0 - nx, b.1 - ny), (a.0 - nx, a.1 - ny));
        self.triangle([p, q, r], c);
        self.triangle([p, r, t], c);
    }

    fn polygon(&mut self, cx: f32, cy: f32, r: f32, sides: usize, rotation: f32, c: Color) {
        self.star(cx, cy, r, r * (PI / sides as f32).cos(), sides, rotation, c);
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
            [(x, y), (x, y + outline), (x + outline * 0.72, y + outline * 0.72)],
            CURSOR_EDGE,
        );
        let (fx, fy) = (x + inset * 0.6, y + inset * 1.6);
        self.triangle([(fx, fy), (fx, fy + fill), (fx + fill * 0.72, fy + fill * 0.72)], CURSOR_FILL);
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
                [at(tip - edge * 2.0, 0.0), at(base + edge, -half - edge * 1.6), at(base + edge, half + edge * 1.6)],
                CURSOR_EDGE,
            );
            self.triangle([at(tip, 0.0), at(base, -half), at(base, half)], ATTACK_CURSOR);
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

pub fn build(game: &Game) -> Vec<Vertex> {
    let mut canvas = Canvas::default();
    let s = ui_scale(game.height);
    let cx = game.width / 2.0;

    match game.screen {
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
            if matches!(game.screen, Screen::MainMenu | Screen::SinglePlayerMenu) && !game.in_battle {
                // Rising from the bottom of the screen, leaving the title on clear sky.
                let (y, radius) = (game.height * 0.66, game.height * 0.48);
                planet::draw_sandy(&mut canvas.vertices, cx, y, radius, game.menu_seconds());
                canvas.text_left(VERSION, 24.0 * s, 30.0 * s, small_text_px(game.height), TEXT);
            }
            let title = match game.screen {
                Screen::Options => Some("Main Options"),
                Screen::Credits => Some("Credits"),
                Screen::FactionSelect => Some("Choose Faction"),
                Screen::InGame | Screen::SkirmishSetup | Screen::EscapeMenu | Screen::BattleStats => None,
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
                    canvas.text_centered("Esc to continue", cx, hint_y, small_text_px(game.height), TEXT);
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
                let current = map_items(game.width, game.height)[game.skirmish.map];
                for item in map_items(game.width, game.height) {
                    draw_field(&mut canvas, game, item);
                    let color = if item.action == current.action { GOLD } else { TEXT };
                    let px = small_text_px(game.height);
                    canvas.text_left(item.label, item.x + 16.0 * s, item.y + item.h / 2.0, px, color);
                }
            }
        }
    }

    if let Some(tip) = game.tip() {
        draw_tip(&mut canvas, game, tip);
    }
    if game.cursor_means_attack() {
        canvas.attack_cursor(game.mouse, s, game.screen_elapsed());
    } else {
        canvas.cursor(game.mouse, s);
    }
    canvas.vertices
}

/// A small label below and to the right of the cursor, kept on the screen.
fn draw_tip(canvas: &mut Canvas, game: &Game, text: &str) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let pad = 6.0 * s;
    let border = (2.0 * s).round().max(1.0);
    let (w, h) = (font::text_width(text) * px + pad * 2.0, font::CAP_HEIGHT * px + pad * 2.0);
    let x = (game.mouse.0 + 26.0 * s).min(game.width - w);
    let y = (game.mouse.1 + 34.0 * s).min(game.height - h);
    canvas.rect(x - border, y - border, w + border * 2.0, h + border * 2.0, BORDER);
    canvas.rect(x, y, w, h, PANEL);
    canvas.text_left(text, x + pad, y + h / 2.0, px, TEXT);
}

fn draw_button(canvas: &mut Canvas, game: &Game, b: Button) {
    let s = ui_scale(game.height);
    let border = (2.0 * s).round().max(1.0);
    let hovered = game.hovered();
    let selected = b.action == Action::Tab(game.tab);
    let fill = if selected { PANEL } else { button_fill(b.action, hovered, game.pressed) };
    let edge = if selected { GOLD } else { BORDER };
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    if let Action::Faction(faction) = b.action {
        let ring = if hovered == Some(b.action) { GOLD } else { BORDER };
        draw_planet(canvas, game, faction, button_circle(b), ring);
        // The name goes below: the moving surface would make it hard to read on top.
        canvas.text_centered(b.label, cx, b.y + b.h + 28.0 * s, 4.0 * s, TEXT);
        return;
    }
    // The sidebar's small square buttons keep solid edges: their labels and symbols sit
    // there, over whatever the battlefield shows.
    if matches!(b.action, Action::BuildCell(_) | Action::HudSmall(_)) {
        canvas.rect(b.x, b.y, b.w, b.h, edge);
        canvas.rect(b.x + border, b.y + border, b.w - border * 2.0, b.h - border * 2.0, fill);
    } else {
        let banner = match (hovered == Some(b.action) || selected, game.pressed == Some(b.action)) {
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
    let border = (2.0 * s).round().max(1.0);
    let (x, y, w, h) = modal_rect(game.width, game.height);
    canvas.rect(0.0, 0.0, game.width, game.height, DIM);
    canvas.rect(x, y, w, h, GOLD);
    canvas.rect(x + border, y + border, w - border * 2.0, h - border * 2.0, PANEL);
    canvas.text_centered(CONFIRM_MESSAGE, x + w / 2.0, y + 90.0 * s, small_text_px(game.height), TEXT);
    for b in game.buttons() {
        draw_button(canvas, game, b);
    }
}

fn draw_options_panel(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let border = (2.0 * s).round().max(1.0);
    let (x, y, w, h) = options_panel(game.width, game.height);
    canvas.rect(x, y, w, h, BORDER);
    canvas.rect(x + border, y + border, w - border * 2.0, h - border * 2.0, PANEL);

    if let Some(text) = game.tab.placeholder() {
        canvas.text_centered(text, x + w / 2.0, y + h / 2.0, px, QUIET);
    }

    draw_sliders(canvas, game);
    draw_checkboxes(canvas, game, game.checkboxes());
}

/// A faction's planet inside a thin ring filling the given circle.
fn draw_planet(canvas: &mut Canvas, game: &Game, faction: Faction, (cx, cy, r): (f32, f32, f32), ring: Color) {
    let border = (2.0 * ui_scale(game.height)).round().max(1.0);
    canvas.circle(cx, cy, r, ring);
    planet::draw(&mut canvas.vertices, faction, cx, cy, r - border, game.screen_elapsed());
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
    battlefield::draw(
        &mut canvas.vertices,
        battlefield::terrain(game.battle_map),
        game.camera,
        game.width,
        game.height,
    );
    draw_map_objects(canvas, game);
    let view = battlefield::View::new(game.camera, game.width, game.height);
    draw_combat(canvas, game, &view);
    draw_strike_aim(canvas, game, &view);
    if !game.aiming_strike {
        draw_selection_box(canvas, game);
    }
    if let Some(faction) = game.faction {
        let label = format!("{} - Mission {}", faction.label(), game.campaign_mission + 1);
        canvas.text_centered(&label, game.width / 2.0, 40.0 * s, 4.0 * s, TEXT);
    }
    draw_energy_gauge(canvas, game);
    draw_battle_minimap(canvas, game);
    draw_money(canvas, game);
    draw_sidebar(canvas, game);
    draw_chat(canvas, game);
}

/// Units and ruins as simple boxes, farthest first, shaded per face so they read as solid.
/// Whatever flies goes over all of that, with its shadow and ring down on the ground below.
fn draw_map_objects(canvas: &mut Canvas, game: &Game) {
    enum Object<'a> {
        Unit(&'a Unit),
        Ruin(&'a battlefield::RuinPiece),
    }
    let z_of = |object: &Object| match object {
        Object::Unit(unit) => unit.z,
        Object::Ruin(piece) => piece.z,
    };
    let view = battlefield::View::new(game.camera, game.width, game.height);
    draw_poison(canvas, game, &view);
    draw_order_lines(canvas, game, &view);
    let is_marked = |unit: &Unit| unit.selected || game.in_selection_drag(unit);
    let mut flying: Vec<&Unit> = game.units.iter().filter(|u| u.kind.flies()).collect();
    flying.sort_by(|a, b| a.z.total_cmp(&b.z));
    for unit in &flying {
        draw_shadow(canvas, &view, (unit.x, unit.y, unit.z), unit.kind.radius());
        if is_marked(unit) {
            draw_selected_ring(canvas, game, &view, unit);
        }
    }
    let mut order: Vec<Object> = game.units.iter().filter(|u| !u.kind.flies()).map(Object::Unit).collect();
    order.extend(battlefield::terrain(game.battle_map).ruins().iter().map(Object::Ruin));
    order.sort_by(|a, b| z_of(a).total_cmp(&z_of(b)));
    for object in order {
        match object {
            Object::Unit(unit) => {
                if is_marked(unit) {
                    draw_selected_ring(canvas, game, &view, unit);
                }
                draw_unit(canvas, game, &view, unit);
                draw_health_bar(canvas, game, &view, unit);
            }
            Object::Ruin(piece) => {
                let color = RUIN_CONCRETE.map(|c| c * piece.shade);
                let footprint = (piece.x, piece.z, piece.half_x, piece.half_z);
                draw_box(canvas, game, &view, footprint, (piece.y, piece.y + piece.height), color);
            }
        }
    }
    for unit in flying {
        draw_unit(canvas, game, &view, unit);
        draw_health_bar(canvas, game, &view, unit);
    }
}

/// The shadow of something in the air, on the ground straight below it.
fn draw_shadow(canvas: &mut Canvas, view: &battlefield::View, below: (f32, f32, f32), radius: f32) {
    draw_ground_disc(canvas, view, below, radius, [0.0, 0.0, 0.0], (SHADOW[3], SHADOW[3]));
}

/// A flat disc on the ground, seen in perspective, fading out towards its rim if asked.
fn draw_ground_disc(
    canvas: &mut Canvas,
    view: &battlefield::View,
    (x, y, z): (f32, f32, f32),
    radius: f32,
    [r, g, b]: [f32; 3],
    (center_alpha, rim_alpha): (f32, f32),
) {
    const WEDGES: usize = 40;
    let Some(middle) = view.project(x, y, z) else { return };
    let rim = |i: usize| {
        let (sin, cos) = (i as f32 / WEDGES as f32 * TAU).sin_cos();
        view.project(x + radius * cos, y, z + radius * sin)
    };
    for i in 0..WEDGES {
        if let (Some(a), Some(b_point)) = (rim(i), rim(i + 1)) {
            let vertex = |(px, py): (f32, f32), alpha: f32| Vertex { x: px, y: py, r, g, b, a: alpha };
            canvas.vertices.extend([
                vertex(middle, center_alpha),
                vertex(a, rim_alpha),
                vertex(b_point, rim_alpha),
            ]);
        }
    }
}

/// Poisoned land glows green, breathing gently, and fades during its last seconds.
fn draw_poison(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    for zone in &game.combat.poison {
        let fade = (zone.life / 5.0).min(1.0);
        let breath = 0.85 + 0.15 * (game.battle_time() * 2.0).sin();
        let alpha = POISON_ALPHA * fade * breath;
        draw_ground_disc(
            canvas,
            view,
            (zone.x, 0.02, zone.z),
            game.strike_radius(),
            POISON_FILL,
            (alpha, alpha * 1.4),
        );
    }
}

/// Shots in the air and explosions, drawn over everything standing on the map.
fn draw_combat(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    let s = ui_scale(game.height);
    for beam in &game.combat.beams {
        let from = view.project(beam.from.0, beam.from.1, beam.from.2);
        let to = view.project(beam.to.0, beam.to.1, beam.to.2);
        if let (Some(a), Some(b)) = (from, to) {
            let (glow, core) = if beam.repair { (REPAIR_GLOW, REPAIR_CORE) } else { (LASER_GLOW, LASER_CORE) };
            let [r, g, bl, alpha] = glow;
            canvas.line(a, b, 9.0 * s, [r, g, bl, alpha * beam.life]);
            let [r, g, bl, alpha] = core;
            canvas.line(a, b, (2.5 * s).max(1.0), [r, g, bl, alpha * beam.life]);
        }
    }
    for spray in &game.combat.sprays {
        draw_spray(canvas, view, spray);
    }
    for flare in &game.combat.flares {
        let (x, y, z) = flare.at;
        if let Some((px, py)) = view.project(x, y, z) {
            let size = flare.size * s;
            let [r, g, b] = FLARE_GLOW;
            canvas.star(px, py, size, size * 0.35, 6, flare.life, [r, g, b, 0.8 * flare.life]);
            let [r, g, b] = FLARE_CORE;
            canvas.circle(px, py, size * 0.35, [r, g, b, flare.life]);
        }
    }
    for missile in &game.combat.missiles {
        let (x, y, z) = missile.position;
        let (dx, dy, dz) = missile.direction;
        let back = |by: f32| view.project(x - dx * by, y - dy * by, z - dz * by);
        if let (Some(nose), Some(tail), Some(flame)) = (back(0.0), back(0.35), back(0.8)) {
            canvas.line(tail, flame, (4.0 * s).max(1.0), MISSILE_FLAME);
            canvas.line(nose, tail, (4.0 * s).max(1.0), MISSILE_BODY);
        }
    }
    for shell in &game.combat.shells {
        let (_, ground, _) = shell.landing();
        let (x, y, z) = shell.position();
        draw_shadow(canvas, view, (x, ground, z), 0.16);
        if let Some((px, py)) = view.project(x, y, z) {
            canvas.circle(px, py, (5.0 * s).max(2.0), SHELL_BODY);
        }
    }
    for strike in &game.combat.strikes {
        let height = strike.height();
        let (top, bottom) = (view.project(strike.x, height + 3.0, strike.z), view.project(strike.x, height, strike.z));
        if let (Some(a), Some(b)) = (top, bottom) {
            let [r, g, bl] = POISON_FILL;
            canvas.line(a, b, 14.0 * s, [r, g, bl, 0.35]);
            canvas.line(a, b, 6.0 * s, MISSILE_BODY);
        }
        draw_ground_disc(canvas, view, (strike.x, 0.02, strike.z), game.strike_radius(), POISON_FILL, (0.0, 0.25));
    }
    for blast in &game.combat.blasts {
        let (x, y, z) = blast.at;
        let reach = blast.radius * (1.2 - blast.life * 0.6);
        draw_ground_disc(canvas, view, (x, y, z), reach, BLAST_COLOR, (0.9 * blast.life, 0.1 * blast.life));
    }
}

/// A cone of fire or gas, solid where it leaves the nozzle and fading towards its far edge,
/// where it comes down to about the height of whoever it reaches.
fn draw_spray(canvas: &mut Canvas, view: &battlefield::View, spray: &crate::weapons::Spray) {
    const WEDGES: usize = 10;
    const NEAR_ALPHA: f32 = 0.8;
    const FAR_ALPHA: f32 = 0.1;
    const FAR_HEIGHT: f32 = 0.4;
    let (x, y, z) = spray.from;
    let Some(nozzle) = view.project(x, y, z) else { return };
    let [r, g, b] = if spray.gas { GAS_COLOR } else { FIRE_COLOR };
    let far = |i: usize| {
        let angle = spray.heading - spray.half_angle + i as f32 / WEDGES as f32 * spray.half_angle * 2.0;
        let (sin, cos) = angle.sin_cos();
        view.project(x + cos * spray.reach, FAR_HEIGHT.min(y), z + sin * spray.reach)
    };
    let vertex = |(px, py): (f32, f32), alpha: f32| Vertex { x: px, y: py, r, g, b, a: alpha * spray.life };
    for i in 0..WEDGES {
        if let (Some(a), Some(c)) = (far(i), far(i + 1)) {
            canvas.vertices.extend([vertex(nozzle, NEAR_ALPHA), vertex(a, FAR_ALPHA), vertex(c, FAR_ALPHA)]);
        }
    }
}

/// While the superweapon waits for a click, a ring on the ground shows what it would hit.
fn draw_strike_aim(canvas: &mut Canvas, game: &Game, view: &battlefield::View) {
    const SIDES: usize = 48;
    if !game.aiming_strike {
        return;
    }
    let Some((x, z)) = view.ground_at(game.mouse.0, game.mouse.1) else { return };
    let radius = game.strike_radius();
    draw_ground_disc(canvas, view, (x, 0.02, z), radius, POISON_FILL, (0.05, 0.2));
    let thickness = (3.0 * ui_scale(game.height)).max(1.0);
    let point = |i: usize| {
        let (sin, cos) = (i as f32 / SIDES as f32 * TAU).sin_cos();
        view.project(x + radius * cos, 0.02, z + radius * sin)
    };
    let [r, g, b] = POISON_FILL;
    for i in (0..SIDES).step_by(2) {
        if let (Some(a), Some(c)) = (point(i), point(i + 1)) {
            canvas.line(a, c, thickness, [r, g, b, 1.0]);
        }
    }
}

/// A small bar over anyone who has been hurt.
fn draw_health_bar(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    if unit.health >= unit.max_health() {
        return;
    }
    let Some((x, y)) = view.project(unit.x, unit.pick_height() + 1.0, unit.z) else { return };
    let s = ui_scale(game.height);
    let (width, height) = (30.0 * s, (4.0 * s).max(2.0));
    let share = (unit.health / unit.max_health()).clamp(0.0, 1.0);
    canvas.rect(x - width / 2.0, y, width, height, HEALTH_BAD);
    canvas.rect(x - width / 2.0, y, width * share, height, HEALTH_GOOD);
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
            Some(Target::Unit(id) | Target::Forced(id)) => {
                game.units.iter().find(|u| u.alive && u.id == id).map(|u| (u.x, u.z))
            }
            None => None,
        };
        let from = (unit.x, unit.z);
        match after {
            Some(to) => draw_dashed_line(canvas, game, view, from, to, ATTACK_LINE),
            None if unit.is_moving() => draw_dashed_line(canvas, game, view, from, unit.destination, ORDER_LINE),
            None => {}
        }
    }
}

fn draw_dashed_line(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    from: (f32, f32),
    to: (f32, f32),
    color: Color,
) {
    const DASH: f32 = 0.45;
    const GAP: f32 = 0.3;
    let thickness = (2.0 * ui_scale(game.height)).max(1.0);
    let screen = (0.0, 0.0, game.width, game.height);
    let (to_x, to_z) = (to.0 - from.0, to.1 - from.1);
    let length = to_x.hypot(to_z);
    let (dir_x, dir_z) = (to_x / length, to_z / length);
    let mut along = 0.0;
    while along < length {
        let end = (along + DASH).min(length);
        let a = (from.0 + dir_x * along, from.1 + dir_z * along);
        let b = (from.0 + dir_x * end, from.1 + dir_z * end);
        let visible = view
            .project_ground_line(a, b)
            .and_then(|(a, b)| battlefield::clip_to_rect(a, b, screen));
        if let Some((a, b)) = visible {
            canvas.line(a, b, thickness, color);
        }
        along += DASH + GAP;
    }
}

/// A unit from simple boxes turned to its heading. Infantry is one tall thin block, a vehicle
/// is a hull with a turret and a barrel, and an aircraft is a body with wings and a tail, so
/// that their turning can be seen.
fn draw_unit(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    let [r, g, b, _] = team_color(unit.color);
    let color = [r, g, b];
    let (forward_x, forward_z) = (unit.heading.cos(), unit.heading.sin());
    let ahead = |by: f32| (unit.x + forward_x * by, unit.z + forward_z * by);
    match unit.kind {
        UnitKind::Infantry => {
            let barrel = |by: f32, half: f32, thick: f32| {
                let (x, z) = ahead(by);
                (x, z, half, thick, unit.heading)
            };
            let dark = color.map(|c| c * 0.5);
            let y = unit.y;
            match (unit.role, unit.deployed) {
                (Role::KInfantry, true) => {
                    let base = (unit.x, unit.z, 0.3, 0.22, unit.heading);
                    draw_turned_box(canvas, game, view, base, (y, y + 0.22), color);
                    draw_turned_box(canvas, game, view, barrel(0.45, 0.3, 0.05), (y + 0.14, y + 0.22), dark);
                }
                (Role::KInfantry, false) => {
                    let body = (unit.x, unit.z, 0.17, 0.17, unit.heading);
                    draw_turned_box(canvas, game, view, body, (y, y + 0.66), color);
                }
                (Role::Scout, _) => {
                    let body = (unit.x, unit.z, 0.1, 0.1, unit.heading);
                    draw_turned_box(canvas, game, view, body, (y, y + 0.45), color);
                }
                (Role::Sniper, _) => {
                    let body = (unit.x, unit.z, 0.1, 0.1, unit.heading);
                    draw_turned_box(canvas, game, view, body, (y, y + 0.6), color);
                    draw_turned_box(canvas, game, view, barrel(0.3, 0.26, 0.02), (y + 0.4, y + 0.44), dark);
                }
                (Role::FlameInfantry | Role::GasInfantry, _) => {
                    let body = (unit.x, unit.z, 0.13, 0.13, unit.heading);
                    draw_turned_box(canvas, game, view, body, (y, y + 0.6), color);
                    // A tank on the back and a nozzle out front, in the color of what it sprays.
                    let spray_color = if unit.role == Role::GasInfantry { GAS_COLOR } else { FIRE_COLOR };
                    draw_turned_box(canvas, game, view, barrel(-0.17, 0.05, 0.09), (y + 0.2, y + 0.5), spray_color);
                    draw_turned_box(canvas, game, view, barrel(0.22, 0.12, 0.03), (y + 0.32, y + 0.38), dark);
                }
                (Role::Engineer, _) => {
                    let body = (unit.x, unit.z, 0.13, 0.13, unit.heading);
                    draw_turned_box(canvas, game, view, body, (y, y + 0.5), color);
                    let hat = (unit.x, unit.z, 0.18, 0.18, unit.heading);
                    draw_turned_box(canvas, game, view, hat, (y + 0.5, y + 0.58), [0.95, 0.85, 0.3]);
                }
                _ => {
                    let body = (unit.x, unit.z, 0.13, 0.13, unit.heading);
                    draw_turned_box(canvas, game, view, body, (y, y + 0.6), color);
                }
            }
        }
        UnitKind::Vehicle if unit.role == Role::RepairVehicle => {
            let hull = (unit.x, unit.z, 0.42, 0.26, unit.heading);
            let top = unit.y + 0.3;
            draw_turned_box(canvas, game, view, hull, (unit.y, top), color);
            // A white cross on its roof.
            let long = (unit.x, unit.z, 0.2, 0.06, unit.heading);
            let wide = (unit.x, unit.z, 0.06, 0.2, unit.heading);
            draw_turned_box(canvas, game, view, long, (top, top + 0.03), [0.95, 0.95, 0.95]);
            draw_turned_box(canvas, game, view, wide, (top, top + 0.03), [0.95, 0.95, 0.95]);
        }
        UnitKind::Vehicle if unit.role == Role::Catapult => {
            let hull = (unit.x, unit.z, 0.42, 0.26, unit.heading);
            let top = unit.y + 0.22;
            draw_turned_box(canvas, game, view, hull, (unit.y, top), color);
            let (arm_x, arm_z) = ahead(-0.08);
            let arm = (arm_x, arm_z, 0.36, 0.04, unit.heading);
            draw_turned_box(canvas, game, view, arm, (top + 0.14, top + 0.2), color.map(|c| c * 0.6));
            let (bucket_x, bucket_z) = ahead(-0.42);
            let bucket = (bucket_x, bucket_z, 0.09, 0.09, unit.heading);
            draw_turned_box(canvas, game, view, bucket, (top + 0.1, top + 0.26), color.map(|c| c * 0.45));
        }
        UnitKind::Vehicle => {
            let hull = (unit.x, unit.z, 0.45, 0.28, unit.heading);
            let (hull_top, turret_top) = (unit.y + 0.28, unit.y + 0.46);
            draw_turned_box(canvas, game, view, hull, (unit.y, hull_top), color);
            let turret = (unit.x, unit.z, 0.17, 0.17, unit.heading);
            draw_turned_box(canvas, game, view, turret, (hull_top, turret_top), color.map(|c| c * 0.85));
            let (barrel_x, barrel_z) = ahead(0.4);
            let barrel = (barrel_x, barrel_z, 0.25, 0.04, unit.heading);
            draw_turned_box(canvas, game, view, barrel, (hull_top + 0.06, hull_top + 0.13), color.map(|c| c * 0.6));
        }
        UnitKind::Aircraft if unit.role == Role::AirDefensePlatform => {
            let base = unit.y + unit.altitude;
            let deck = (unit.x, unit.z, 0.38, 0.38, unit.heading);
            draw_turned_box(canvas, game, view, deck, (base, base + 0.1), color.map(|c| c * 0.75));
            let mount = (unit.x, unit.z, 0.14, 0.14, unit.heading);
            draw_turned_box(canvas, game, view, mount, (base + 0.1, base + 0.26), color);
            let (gun_x, gun_z) = ahead(0.3);
            let gun = (gun_x, gun_z, 0.2, 0.03, unit.heading);
            draw_turned_box(canvas, game, view, gun, (base + 0.15, base + 0.2), color.map(|c| c * 0.5));
        }
        UnitKind::Aircraft if unit.role == Role::AirDrone => {
            // Two crossed arms and a small body.
            let base = unit.y + unit.altitude;
            for turn in [FRAC_PI_4, -FRAC_PI_4] {
                let arm = (unit.x, unit.z, 0.32, 0.03, unit.heading + turn);
                draw_turned_box(canvas, game, view, arm, (base + 0.04, base + 0.07), color.map(|c| c * 0.6));
            }
            let body = (unit.x, unit.z, 0.12, 0.09, unit.heading);
            draw_turned_box(canvas, game, view, body, (base, base + 0.12), color);
        }
        UnitKind::Aircraft => {
            let base = unit.y + unit.altitude;
            let (tail_x, tail_z) = ahead(-0.36);
            let tailplane = (tail_x, tail_z, 0.07, 0.18, unit.heading);
            draw_turned_box(canvas, game, view, tailplane, (base + 0.1, base + 0.13), color.map(|c| c * 0.7));
            let wings = (unit.x, unit.z, 0.13, 0.45, unit.heading);
            draw_turned_box(canvas, game, view, wings, (base + 0.05, base + 0.09), color.map(|c| c * 0.7));
            let body = (unit.x, unit.z, 0.45, 0.08, unit.heading);
            draw_turned_box(canvas, game, view, body, (base, base + 0.16), color);
            let fin = (tail_x, tail_z, 0.07, 0.015, unit.heading);
            draw_turned_box(canvas, game, view, fin, (base + 0.16, base + 0.32), color.map(|c| c * 0.85));
        }
        UnitKind::Tower => {
            let (right_x, right_z) = (-forward_z, forward_x);
            let wall = (unit.x, unit.z, 0.5, 0.5, 0.0);
            let head_base = units::TUBE_HEIGHT - 0.2;
            draw_turned_box(canvas, game, view, wall, (unit.y, unit.y + head_base), color.map(|c| c * 0.8));
            let head = (unit.x, unit.z, 0.3, 0.3, unit.heading);
            draw_turned_box(canvas, game, view, head, (unit.y + head_base, unit.y + head_base + 0.34), color);
            let (ahead_x, ahead_z) = ahead(0.4);
            let height = (unit.y + units::TUBE_HEIGHT - 0.06, unit.y + units::TUBE_HEIGHT + 0.06);
            if unit.role == Role::FlameTower {
                // One fat nozzle, with the color of fire on its way out.
                let nozzle = (ahead_x, ahead_z, 0.3, 0.1, unit.heading);
                draw_turned_box(canvas, game, view, nozzle, height, FIRE_COLOR);
                return;
            }
            for side in [-units::TUBE_SPREAD, units::TUBE_SPREAD] {
                let tube = (ahead_x + right_x * side, ahead_z + right_z * side, 0.3, 0.06, unit.heading);
                draw_turned_box(canvas, game, view, tube, height, color.map(|c| c * 0.5));
            }
        }
    }
}

/// A box turned to any heading: (x, z, half length, half width, heading). Only the sides
/// facing the camera are drawn, farthest first.
fn draw_turned_box(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    (x, z, half_length, half_width, heading): (f32, f32, f32, f32, f32),
    (base, top): (f32, f32),
    [r, g, b]: [f32; 3],
) {
    let (forward_x, forward_z) = (heading.cos(), heading.sin());
    let (right_x, right_z) = (-forward_z, forward_x);
    let corner = |along: f32, across: f32| {
        (
            x + forward_x * along * half_length + right_x * across * half_width,
            z + forward_z * along * half_length + right_z * across * half_width,
        )
    };
    let corners = [corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0), corner(-1.0, -1.0)];
    let (camera_x, camera_z) = view.position_on_ground();
    let (width, height) = (game.width, game.height);
    let mut face = |points: [(f32, f32, f32); 4], shade: f32| {
        let mut projected = [(0.0, 0.0); 4];
        for (slot, (px, py, pz)) in projected.iter_mut().zip(points) {
            let Some(point) = view.project(px, py, pz) else { return };
            *slot = point;
        }
        let off = projected.iter().all(|p| p.0 < 0.0)
            || projected.iter().all(|p| p.0 > width)
            || projected.iter().all(|p| p.1 < 0.0)
            || projected.iter().all(|p| p.1 > height);
        if !off {
            let color = [r * shade, g * shade, b * shade, 1.0];
            canvas.triangle([projected[0], projected[1], projected[2]], color);
            canvas.triangle([projected[0], projected[2], projected[3]], color);
        }
    };
    for i in 0..corners.len() {
        let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
        let (mid_x, mid_z) = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let (normal_x, normal_z) = (mid_x - x, mid_z - z);
        let faces_camera = normal_x * (camera_x - mid_x) + normal_z * (camera_z - mid_z) > 0.0;
        if faces_camera {
            // Sides turned south catch a little more of the top's brightness.
            let southward = normal_z / normal_x.hypot(normal_z);
            face([(a.0, base, a.1), (b.0, base, b.1), (b.0, top, b.1), (a.0, top, a.1)], 0.68 + 0.14 * southward);
        }
    }
    face(corners.map(|(px, pz)| (px, top, pz)), 1.0);
}

fn draw_selected_ring(canvas: &mut Canvas, game: &Game, view: &battlefield::View, unit: &Unit) {
    const SIDES: usize = 16;
    let radius = unit.kind.radius() + 0.1;
    let thickness = (2.0 * ui_scale(game.height)).max(1.0);
    let point = |i: usize| {
        let (sin, cos) = (i as f32 / SIDES as f32 * TAU).sin_cos();
        view.project(unit.x + radius * cos, unit.y, unit.z + radius * sin)
    };
    for i in 0..SIDES {
        if let (Some(a), Some(b)) = (point(i), point(i + 1)) {
            canvas.line(a, b, thickness, SELECTED_RING);
        }
    }
}

/// A box seen from the camera: its top, its south face, and whichever side faces the camera.
fn draw_box(
    canvas: &mut Canvas,
    game: &Game,
    view: &battlefield::View,
    (x, z, half_x, half_z): (f32, f32, f32, f32),
    (base, top): (f32, f32),
    [r, g, b]: [f32; 3],
) {
    let (width, height) = (game.width, game.height);
    let corner = |dx: f32, y: f32, dz: f32| view.project(x + dx, y, z + dz);
    let side = if x > game.camera.x { -half_x } else { half_x };
    let faces = [
        ([(side, base, -half_z), (side, base, half_z), (side, top, half_z), (side, top, -half_z)], 0.6),
        ([(-half_x, base, half_z), (half_x, base, half_z), (half_x, top, half_z), (-half_x, top, half_z)], 0.78),
        ([(-half_x, top, -half_z), (half_x, top, -half_z), (half_x, top, half_z), (-half_x, top, half_z)], 1.0),
    ];
    for (points, shade) in faces {
        let mut projected = [(0.0, 0.0); 4];
        for (slot, (dx, y, dz)) in projected.iter_mut().zip(points) {
            let Some(point) = corner(dx, y, dz) else { return };
            *slot = point;
        }
        let off = projected.iter().all(|p| p.0 < 0.0)
            || projected.iter().all(|p| p.0 > width)
            || projected.iter().all(|p| p.1 < 0.0)
            || projected.iter().all(|p| p.1 > height);
        if off {
            continue;
        }
        let color = [r * shade, g * shade, b * shade, 1.0];
        canvas.triangle([projected[0], projected[1], projected[2]], color);
        canvas.triangle([projected[0], projected[2], projected[3]], color);
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
            let side = if (frame + j as u32) % 2 == 0 { 1.0 } else { -1.0 };
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let length = dx.hypot(dy).max(1.0);
            let reach = 22.0 * s;
            let end = (
                start.0 + dx / length * reach * 0.6 - dy / length * reach * side,
                start.1 + dy / length * reach * 0.6 + dx / length * reach * side,
            );
            let fork = units::lightning_points(start, end, seed.wrapping_add(j as u32), 9.0 * s, 4.0 * s);
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
        let (color_from, color_to) = if lit { (shade(from), shade(to)) } else { (GAUGE_EMPTY, GAUGE_EMPTY) };
        let vertex = |(x, y): (f32, f32), [r, g, b, a]: Color| Vertex { x, y, r, g, b, a };
        canvas.vertices.extend([
            vertex(a, color_from),
            vertex(b, color_to),
            vertex(c, color_to),
            vertex(a, color_from),
            vertex(c, color_to),
            vertex(d, color_from),
        ]);
    }
    let (x, y) = hud::energy_label_center(game.width, game.height);
    canvas.text_centered(&game.energy.to_string(), x, y, small_text_px(game.height), TEXT);
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
    for (i, cell) in hud::grid_cells(game.width, game.height).into_iter().enumerate() {
        draw_button(canvas, game, cell);
        let (cx, cy) = (cell.x + cell.w / 2.0, cell.y + cell.h / 2.0);
        if game.build_tab == BuildTab::Vehicles && i == SUPERWEAPON_CELL {
            let ready = game.superweapon_allowed() && game.strike_recharge <= 0.0;
            let color = if !ready {
                QUIET
            } else if game.aiming_strike {
                SELECTED_RING
            } else {
                GOLD
            };
            draw_radiation_symbol(canvas, cx, cy, cell.w * 0.7, color);
            if game.superweapon_allowed() && game.strike_recharge > 0.0 {
                let seconds = (game.strike_recharge.ceil() as i32).to_string();
                canvas.text_centered(&seconds, cx, cy, small_text_px(game.height), TEXT);
            }
            continue;
        }
        // Tip text gives the full name of what it makes.
        let makes = Game::build_role(game.build_tab, i);
        let symbol = if makes.is_some() { GOLD } else { GOLD_DIM };
        draw_tab_symbol(canvas, game.build_tab, cx, cy, cell.w * 0.5, symbol);
        let label = makes.map_or(format!("{letter}{}", i + 1), |role| short_name(role).to_string());
        canvas.text_left(&label, cell.x + 6.0 * s, cell.y + 14.0 * s, chat_px(game), TEXT);
    }
    for tab in hud::build_tabs(game.width, game.height) {
        let Action::BuildTab(kind) = tab.action else { continue };
        let selected = kind == game.build_tab;
        let hovered = game.hovered() == Some(tab.action);
        let border = (2.0 * s).round().max(1.0);
        canvas.rect(tab.x, tab.y, tab.w, tab.h, if selected || hovered { GOLD } else { BORDER });
        let fill = if selected { PANEL } else { button_fill(tab.action, game.hovered(), game.pressed) };
        canvas.rect(tab.x + border, tab.y + border, tab.w - border * 2.0, tab.h - border * 2.0, fill);
        let color = if selected { GOLD } else { TEXT };
        draw_tab_symbol(canvas, kind, tab.x + tab.w / 2.0, tab.y + tab.h / 2.0, tab.w * 0.7, color);
    }
}

/// A name short enough for a build menu cell.
fn short_name(role: Role) -> &'static str {
    match role {
        Role::LightInfantry => "Light",
        Role::Engineer => "Engin",
        Role::KInfantry => "K Inf",
        Role::RepairVehicle => "Repair",
        Role::Catapult => "Catap",
        Role::AirDefensePlatform => "AirDef",
        Role::AirDrone => "Drone",
        Role::FlameInfantry => "Flame",
        Role::GasInfantry => "Gas",
        _ => role.name(),
    }
}

fn chat_px(game: &Game) -> f32 {
    hud::chat_text_px(game.height)
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
            canvas.line((cx - 2.8 * u, cy + 0.2 * u), (cx + 3.6 * u, cy - 2.6 * u), 0.8 * u, c);
        }
        BuildTab::Vehicles => {
            // A tank: tracks, hull, turret and barrel.
            canvas.rect(cx - 4.6 * u, cy + 1.2 * u, 9.2 * u, 2.2 * u, c);
            canvas.rect(cx - 3.8 * u, cy - 0.6 * u, 7.6 * u, 1.4 * u, c);
            canvas.rect(cx - 2.0 * u, cy - 2.6 * u, 3.8 * u, 1.7 * u, c);
            canvas.line((cx + 1.6 * u, cy - 1.8 * u), (cx + 5.0 * u, cy - 2.6 * u), 0.7 * u, c);
        }
        BuildTab::Buildings => {
            // A factory: walls under a sawtooth roof, with a chimney.
            let (left, right, top, bottom) = (cx - 4.4 * u, cx + 4.4 * u, cy - 0.4 * u, cy + 4.0 * u);
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
        let (w, h) = (font::text_width(line) * px, font::CAP_HEIGHT * px);
        let pad = 4.0 * s;
        canvas.rect(x - pad, cy - h / 2.0 - pad, w + pad * 2.0, h + pad * 2.0, CHAT_BACKING);
        canvas.text_left(line, x, cy, px, TEXT);
    }
}

/// Everyone in this battle, in a window the battlefield shows through.
fn draw_diplomacy(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let border = (2.0 * s).round().max(1.0);
    let (x, y, w, h) = stats_rect(game.width, game.height);
    canvas.rect(x, y, w, border, BORDER);
    canvas.rect(x, y + h - border, w, border, BORDER);
    canvas.rect(x, y + border, border, h - border * 2.0, BORDER);
    canvas.rect(x + w - border, y + border, border, h - border * 2.0, BORDER);
    canvas.rect(x + border, y + border, w - border * 2.0, h - border * 2.0, WINDOW);

    canvas.text_centered("Diplomacy", x + w / 2.0, y + 40.0 * s, 5.0 * s, GOLD);
    let (swatch_x, name_x, money_x, planet_x, faction_x, ally_x) =
        (x + 30.0 * s, x + 90.0 * s, x + 390.0 * s, x + 520.0 * s, x + 570.0 * s, x + 800.0 * s);
    let header_y = y + 88.0 * s;
    canvas.text_left("Player", name_x, header_y, px, QUIET);
    canvas.text_left("Money", money_x, header_y, px, QUIET);
    canvas.text_left("Faction", faction_x, header_y, px, QUIET);
    canvas.text_left("Allies", ally_x - 22.0 * s, header_y, px, QUIET);

    for (i, row) in game.stats_rows().iter().enumerate() {
        let cy = y + (STATS_ROW_TOP + (i as f32 + 0.5) * STATS_ROW_STEP) * s;
        canvas.rect(swatch_x, cy - 18.0 * s, 36.0 * s, 36.0 * s, team_color(row.color));
        canvas.text_left(&row.name, name_x, cy, px, TEXT);
        canvas.text_left(&format!("${}", row.money), money_x, cy, px, GOLD);
        match row.faction {
            Some(faction) => {
                draw_planet(canvas, game, faction, (planet_x, cy, 26.0 * s), BORDER);
                canvas.text_left(faction.label(), faction_x, cy, px, TEXT);
            }
            None => canvas.text_left("None", faction_x, cy, px, QUIET),
        }
        for (slot, ally) in row.allies.iter().enumerate() {
            let ally_cx = ally_x + slot as f32 * 70.0 * s;
            canvas.circle(ally_cx, cy, 26.0 * s, BORDER);
            canvas.circle(ally_cx, cy, 26.0 * s - border, BUTTON);
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
        TeamColor::Cyan | TeamColor::Orange | TeamColor::Green | TeamColor::Yellow | TeamColor::LightPink => INK,
    }
}

fn draw_opponents(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let border = (2.0 * s).round().max(1.0);
    let setup = &game.skirmish;
    let count = setup.ai_players as usize;
    let hovered = game.hovered();

    let (first_x, first_y, row_w, _) = opponent_row(0, game.width, game.height);
    canvas.text_centered("Opponents", first_x + row_w / 2.0, first_y - 24.0 * s, px, TEXT);

    for slot in count..OPPONENT_SLOTS {
        let (x, y, w, h) = opponent_row(slot, game.width, game.height);
        canvas.rect(x, y, w, h, BUTTON);
        canvas.rect(x + border, y + border, w - border * 2.0, h - border * 2.0, CLEAR_RGBA);
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
                canvas.rect(b.x + border, b.y + border, b.w - border * 2.0, b.h - border * 2.0, fill);
                canvas.text_centered(opponent.kind.label(), cx, cy, px, text_on(opponent.color));
            }
            Action::CycleOpponentFaction(slot) => {
                draw_planet(canvas, game, setup.opponents[slot].faction, button_circle(b), edge);
            }
            Action::CycleOpponentAlly(slot, ally) => {
                canvas.circle(cx, cy, b.w / 2.0, edge);
                canvas.circle(cx, cy, b.w / 2.0 - border, BUTTON);
                let label_px = (px - 1.0).max(1.0);
                draw_ally(canvas, setup.opponents[slot].allies[ally], cx, cy, b.w * 0.32, label_px);
            }
            _ => {}
        }
    }
}

/// A bordered rectangle that lights up under the cursor.
fn draw_field(canvas: &mut Canvas, game: &Game, b: Button) {
    let border = (2.0 * ui_scale(game.height)).round().max(1.0);
    let lit = game.hovered() == Some(b.action);
    canvas.rect(b.x, b.y, b.w, b.h, if lit { GOLD } else { BORDER });
    canvas.rect(b.x + border, b.y + border, b.w - border * 2.0, b.h - border * 2.0, BUTTON);
}

fn draw_skirmish_setup(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let border = (2.0 * s).round().max(1.0);
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
                    let caret_x = text_x + (setup.commander.len() as f32 * crate::font::ADVANCE) * px;
                    canvas.rect(caret_x, cy - 4.0 * px, px, 8.0 * px, GOLD);
                }
            }
            Action::MapDropdown => {
                draw_field(canvas, game, b);
                canvas.text_left(MAPS[setup.map], b.x + 16.0 * s, cy, px, TEXT);
                let (ax, half) = (b.x + b.w - 28.0 * s, 10.0 * s);
                let tip = if game.dropdown_open { cy - half * 0.6 } else { cy + half * 0.6 };
                let base = if game.dropdown_open { cy + half * 0.6 } else { cy - half * 0.6 };
                canvas.triangle([(ax - half, base), (ax + half, base), (ax, tip)], TEXT);
            }
            Action::SetDifficulty(difficulty) => {
                let r = 12.0 * s;
                let dot_x = b.x + r;
                canvas.circle(dot_x, cy, r, edge);
                canvas.circle(dot_x, cy, r - border, BUTTON);
                if setup.difficulty == difficulty {
                    canvas.circle(dot_x, cy, r * 0.55, GOLD);
                }
                canvas.text_left(b.label, dot_x + r + 6.0 * s, cy, px, TEXT);
            }
            Action::ColorBar if game.color_open => {
                for slot in color_slots(&setup.available_colors(), game.width, game.height) {
                    let Action::PickColor(color) = slot.action else { continue };
                    let lit = game.hovered() == Some(slot.action);
                    let current = color == setup.color;
                    canvas.rect(slot.x, slot.y, slot.w, slot.h, if lit || current { GOLD } else { INK });
                    canvas.rect(
                        slot.x + border,
                        slot.y + border,
                        slot.w - border * 2.0,
                        slot.h - border * 2.0,
                        team_color(color),
                    );
                }
            }
            Action::ColorBar => {
                canvas.rect(b.x, b.y, b.w, b.h, edge);
                let fill = team_color(setup.color);
                canvas.rect(b.x + border, b.y + border, b.w - border * 2.0, b.h - border * 2.0, fill);
            }
            Action::StartBattle => draw_button(canvas, game, b),
            Action::CycleFaction => {
                canvas.text_centered(setup.faction.label(), cx, b.y - 24.0 * s, px, TEXT);
                draw_planet(canvas, game, setup.faction, button_circle(b), edge);
                canvas.text_centered("Click to change", cx, b.y + b.h + 32.0 * s, px, QUIET);
            }
            Action::CycleAlly(slot) => {
                canvas.text_centered(b.label, cx, b.y - 24.0 * s, px, TEXT);
                canvas.circle(cx, cy, b.w / 2.0, edge);
                canvas.circle(cx, cy, b.w / 2.0 - border, BUTTON);
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
        Ally::T => canvas.polygon(cx, cy, r, 3, -FRAC_PI_2, TEXT),
        Ally::X => canvas.polygon(cx, cy, r, 4, FRAC_PI_4, TEXT),
        Ally::G => canvas.polygon(cx, cy, r, 4, 0.0, TEXT),
        Ally::F => canvas.polygon(cx, cy, r, 6, 0.0, TEXT),
        Ally::S => canvas.star(cx, cy, r, r * 0.42, 5, -FRAC_PI_2, TEXT),
    }
}

/// The selected map's real terrain as a diamond, with its four corners named.
fn draw_minimap(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let (x, y, size) = minimap_rect(game.width, game.height);
    draw_map_diamond(canvas, game, game.skirmish.map, (x, y, size));

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
    draw_map_diamond(canvas, game, game.battle_map, rect);
    for zone in &game.combat.poison {
        let (x, y) = battlefield::world_to_minimap(rect, zone.x, zone.z);
        let radius = game.strike_radius() / battlefield::MAP_MIDDLE_Z * rect.2 / 2.0;
        let [r, g, b] = POISON_FILL;
        canvas.circle(x, y, radius, [r, g, b, 0.45]);
    }
    let dot = (3.0 * ui_scale(game.height)).round().max(2.0);
    for unit in &game.units {
        let (x, y) = battlefield::world_to_minimap(rect, unit.x, unit.z);
        let color = if unit.selected { VIEW_OUTLINE } else { team_color(unit.color) };
        let size = if unit.kind.is_building() { dot * 2.0 } else { dot };
        canvas.rect(x - size / 2.0, y - size / 2.0, size, size, color);
    }
    draw_radar_sweep(canvas, rect, game.battle_time());

    let view = battlefield::View::new(game.camera, game.width, game.height);
    let Some(corners) = view.ground_corners() else { return };
    let on_map = corners.map(|(x, z)| battlefield::world_to_minimap(rect, x, z));
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
    let [r, g, b] = SWEEP;

    let half = size / 2.0;
    let center = (x + half, y + half);
    let leading = seconds / SECONDS_PER_TURN * TAU;
    // Reaches the square's edge in every direction, so the sweep fills the whole minimap.
    let on_edge = |angle: f32| {
        let (sin, cos) = angle.sin_cos();
        let reach = half / sin.abs().max(cos.abs());
        (center.0 + cos * reach, center.1 + sin * reach)
    };
    let vertex = |(px, py): (f32, f32), a: f32| Vertex { x: px, y: py, r, g, b, a };
    for wedge in 0..WEDGES {
        let (near, far) = (wedge as f32 / WEDGES as f32, (wedge + 1) as f32 / WEDGES as f32);
        let (a_near, a_far) = (STRONGEST_ALPHA * (1.0 - near), STRONGEST_ALPHA * (1.0 - far));
        canvas.vertices.extend([
            vertex(center, a_near),
            vertex(on_edge(leading - near * TRAIL), a_near),
            vertex(on_edge(leading - far * TRAIL), a_far),
        ]);
    }
    canvas.line(center, on_edge(leading), (size / 150.0).max(1.0), [r, g, b, 0.9]);
}

fn draw_map_diamond(canvas: &mut Canvas, game: &Game, map: usize, (x, y, size): (f32, f32, f32)) {
    /// Grid steps per minimap cell.
    const STEP: usize = 4;
    let border = (2.0 * ui_scale(game.height)).round().max(1.0);
    canvas.rect(x - border, y - border, size + border * 2.0, size + border * 2.0, BORDER);
    canvas.rect(x, y, size, size, CLEAR_RGBA);

    let terrain = battlefield::terrain(map);
    let cells = MAP_CELLS as f32;
    let to_screen = |u: usize, v: usize| {
        let (u, v) = (u as f32 / cells, v as f32 / cells);
        (x + size / 2.0 + (u - v) * size / 2.0, y + (u + v) * size / 2.0)
    };
    for v in (0..MAP_CELLS).step_by(STEP) {
        for u in (0..MAP_CELLS).step_by(STEP) {
            let base = if terrain.is_ground(u + STEP / 2, v + STEP / 2) {
                terrain.upper_color()
            } else {
                terrain.lower_color()
            };
            let color = [base[0], base[1], base[2], 1.0];
            let (a, b) = (to_screen(u, v), to_screen(u + STEP, v));
            let (c, d) = (to_screen(u + STEP, v + STEP), to_screen(u, v + STEP));
            canvas.triangle([a, b, c], color);
            canvas.triangle([a, c, d], color);
        }
    }
    let dot = (2.0 * ui_scale(game.height)).round().max(1.0);
    for piece in terrain.ruins() {
        let (px, py) = battlefield::world_to_minimap((x, y, size), piece.x, piece.z);
        canvas.rect(px - dot / 2.0, py - dot / 2.0, dot, dot, RUIN_ON_MINIMAP);
    }
}

fn draw_sliders(canvas: &mut Canvas, game: &Game) {
    let s = ui_scale(game.height);
    let px = small_text_px(game.height);
    let hovered_slider = if game.pressed.is_none() { game.hovered_slider() } else { None };
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
        canvas.rect(slider.track_x, track_y, handle_x - slider.track_x, track_h, GOLD_DIM);
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
    let border = (2.0 * ui_scale(game.height)).round().max(1.0);
    let hovered = game.hovered();
    let size = checkbox_size(game.height);
    for row in rows {
        let Action::Toggle(toggle) = row.action else { continue };
        let cy = row.y + row.h / 2.0;
        canvas.text_left(row.label, row.x, cy, px, TEXT);
        let box_x = row.x + row.w - size;
        let box_y = cy - size / 2.0;
        canvas.rect(box_x, box_y, size, size, if hovered == Some(row.action) { GOLD } else { BORDER });
        canvas.rect(box_x + border, box_y + border, size - border * 2.0, size - border * 2.0, BUTTON);
        if game.toggle_value(toggle) {
            let inset = size * 0.25;
            canvas.rect(box_x + inset, box_y + inset, size - inset * 2.0, size - inset * 2.0, GOLD);
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
