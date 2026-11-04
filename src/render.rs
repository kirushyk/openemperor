//! Turns game state into a triangle list. Backends only ever see colored triangles.

use crate::battlefield::{self, MAP_CELLS};
use crate::font;
use crate::planet;
use crate::game::{
    Action, Ally, Button, CONFIRM_MESSAGE, Faction, Game, MAPS, OPPONENT_SLOTS, Screen, TeamColor,
    buttons, checkbox_size, color_slots, map_items, minimap_rect, modal_rect, opponent_row,
    opponent_widgets, options_panel, setup_checkboxes, setup_widgets, small_text_px, ui_scale,
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
const DIM: Color = [0.0, 0.0, 0.0, 0.6];
const INK: Color = [0.05, 0.05, 0.08, 1.0];
const VIEW_OUTLINE: Color = [1.0, 1.0, 1.0, 1.0];
const CURSOR_FILL: Color = [0.98, 0.98, 0.95, 1.0];
const CURSOR_EDGE: Color = [0.0, 0.0, 0.0, 1.0];

pub const TITLE: &str = "OpenEmperor";

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
}

pub fn clear_color(game: &Game) -> [f32; 3] {
    if matches!(game.screen, Screen::Intro | Screen::Movie(_) | Screen::InGame) {
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
        | Screen::InGame => {
            let title = match game.screen {
                Screen::Options => Some("Main Options"),
                Screen::Credits => Some("Credits"),
                Screen::FactionSelect => Some("Choose Faction"),
                Screen::InGame | Screen::SkirmishSetup => None,
                _ => Some(TITLE),
            };
            if let Some(title) = title {
                canvas.text_centered(title, cx, 170.0 * s, 16.0 * s, GOLD);
            }
            if game.screen == Screen::InGame {
                battlefield::draw(
                    &mut canvas.vertices,
                    battlefield::terrain(game.battle_map),
                    game.camera,
                    game.width,
                    game.height,
                );
                if let Some(faction) = game.faction {
                    canvas.text_centered(faction.label(), cx, 40.0 * s, 4.0 * s, TEXT);
                }
                draw_battle_minimap(&mut canvas, game);
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

    canvas.cursor(game.mouse, s);
    canvas.vertices
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
        draw_planet(canvas, game, faction, b, ring);
        // The name goes below: the moving surface would make it hard to read on top.
        canvas.text_centered(b.label, cx, b.y + b.h + 28.0 * s, 4.0 * s, TEXT);
        return;
    }
    canvas.rect(b.x, b.y, b.w, b.h, edge);
    canvas.rect(b.x + border, b.y + border, b.w - border * 2.0, b.h - border * 2.0, fill);
    let px = match b.action {
        Action::Tab(_) => small_text_px(game.height),
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
fn draw_planet(canvas: &mut Canvas, game: &Game, faction: Faction, b: Button, ring: Color) {
    let border = (2.0 * ui_scale(game.height)).round().max(1.0);
    let (cx, cy, r) = (b.x + b.w / 2.0, b.y + b.h / 2.0, b.w / 2.0);
    canvas.circle(cx, cy, r, ring);
    planet::draw(&mut canvas.vertices, faction, cx, cy, r - border, game.screen_elapsed());
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
                draw_planet(canvas, game, setup.opponents[slot].faction, b, edge);
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
                draw_planet(canvas, game, setup.faction, b, edge);
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
        Ally::Triangle => canvas.polygon(cx, cy, r, 3, -FRAC_PI_2, TEXT),
        Ally::Square => canvas.polygon(cx, cy, r, 4, FRAC_PI_4, TEXT),
        Ally::Diamond => canvas.polygon(cx, cy, r, 4, 0.0, TEXT),
        Ally::Hexagon => canvas.polygon(cx, cy, r, 6, 0.0, TEXT),
        Ally::Star => canvas.star(cx, cy, r, r * 0.42, 5, -FRAC_PI_2, TEXT),
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
    let rect = battlefield::overlay_minimap_rect(game.width, game.height);
    draw_map_diamond(canvas, game, game.battle_map, rect);
    draw_radar_sweep(canvas, rect, game.screen_elapsed());

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
                battlefield::GROUND
            } else {
                battlefield::SAND
            };
            let color = [base[0], base[1], base[2], 1.0];
            let (a, b) = (to_screen(u, v), to_screen(u + STEP, v));
            let (c, d) = (to_screen(u + STEP, v + STEP), to_screen(u, v + STEP));
            canvas.triangle([a, b, c], color);
            canvas.triangle([a, c, d], color);
        }
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
