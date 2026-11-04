//! Layout of the battlefield's overlay: a sidebar on the right and a chat at the top left.
//!
//! Sizes are authored for a 1080-pixel-tall screen. The sidebar's blocks all share one
//! width, that of the minimap plus the energy gauge wrapped around its left side.

use crate::game::{Action, BuildTab, Button, ui_scale};

const MARGIN: f32 = 24.0;
const PANEL_W: f32 = 300.0;
const GAP: f32 = 8.0;
const MINIMAP: f32 = 237.0;
/// The gauge clears the minimap's square corners, and its outer edge reaches the panel's.
const GAUGE_OUTER: f32 = PANEL_W - MINIMAP / 2.0;
const GAUGE_INNER: f32 = GAUGE_OUTER - 12.0;
const SMALL_ROW_Y: f32 = 372.0;
const SMALL_COUNT: usize = 5;
const WIDE_H: f32 = 44.0;
const WIDE_COUNT: usize = 2;
const TAB_W: f32 = 84.0;

pub const GRID_COLUMNS: usize = 3;
pub const GRID_ROWS: usize = 5;

/// The gauge runs clockwise from 7 o'clock to 12 o'clock, in degrees from 12 o'clock.
pub const GAUGE_START_DEGREES: f32 = 210.0;
pub const GAUGE_END_DEGREES: f32 = 360.0;
pub const MAX_ENERGY: i32 = 100;

pub const CHAT_LINES: usize = 5;
pub const CHAT_MESSAGES: [&str; 12] = [
    "Unit lost",
    "Reinforcements are approaching",
    "Base under attack",
    "Construction complete",
    "Enemy unit spotted",
    "Insufficient funds",
    "Low power",
    "Building captured",
    "New technology available",
    "Radar online",
    "Crate discovered",
    "Ally requests assistance",
];

fn panel_x(width: f32, height: f32) -> f32 {
    width - (MARGIN + PANEL_W) * ui_scale(height)
}

/// The battlefield minimap as (x, y, size).
pub fn minimap_rect(width: f32, height: f32) -> (f32, f32, f32) {
    let s = ui_scale(height);
    let top = MARGIN + GAUGE_OUTER - MINIMAP / 2.0;
    (width - (MARGIN + MINIMAP) * s, top * s, MINIMAP * s)
}

/// The gauge's center, inner radius and outer radius.
pub fn gauge(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    let (x, y, size) = minimap_rect(width, height);
    (x + size / 2.0, y + size / 2.0, GAUGE_INNER * s, GAUGE_OUTER * s)
}

/// A point on a circle, by degrees clockwise from 12 o'clock.
pub fn clock_point(cx: f32, cy: f32, radius: f32, degrees: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    (cx + radius * sin, cy - radius * cos)
}

/// Where the gauge's reading is printed: in the gap between it and the minimap, at 9 o'clock.
pub fn energy_label_center(width: f32, height: f32) -> (f32, f32) {
    let s = ui_scale(height);
    let (cx, cy, _, _) = gauge(width, height);
    (cx - (GAUGE_INNER + MINIMAP / 2.0) / 2.0 * s, cy)
}

/// The money text's right end and vertical center: above the minimap, right of the gauge's
/// end at 12 o'clock.
pub fn money_anchor(width: f32, height: f32) -> (f32, f32) {
    let s = ui_scale(height);
    (width - MARGIN * s, (MARGIN + (GAUGE_OUTER - MINIMAP / 2.0) / 2.0) * s)
}

/// The row of small square buttons under the minimap, then the row of two wide ones.
pub fn command_buttons(width: f32, height: f32) -> Vec<Button> {
    const SMALL_LABELS: [&str; SMALL_COUNT] = ["1", "2", "3", "4", "5"];
    const WIDE_LABELS: [&str; WIDE_COUNT] = ["Fake A", "Fake B"];
    let s = ui_scale(height);
    let left = panel_x(width, height);
    let small = (PANEL_W - GAP * (SMALL_COUNT - 1) as f32) / SMALL_COUNT as f32;
    let wide = (PANEL_W - GAP * (WIDE_COUNT - 1) as f32) / WIDE_COUNT as f32;
    let wide_y = SMALL_ROW_Y + small + GAP;
    let mut all = Vec::new();
    for (i, &label) in SMALL_LABELS.iter().enumerate() {
        all.push(Button {
            action: Action::HudSmall(i),
            label,
            x: left + i as f32 * (small + GAP) * s,
            y: SMALL_ROW_Y * s,
            w: small * s,
            h: small * s,
        });
    }
    for (i, &label) in WIDE_LABELS.iter().enumerate() {
        all.push(Button {
            action: Action::HudWide(i),
            label,
            x: left + i as f32 * (wide + GAP) * s,
            y: wide_y * s,
            w: wide * s,
            h: WIDE_H * s,
        });
    }
    all
}

fn grid_top() -> f32 {
    let small = (PANEL_W - GAP * (SMALL_COUNT - 1) as f32) / SMALL_COUNT as f32;
    SMALL_ROW_Y + small + GAP + WIDE_H + GAP
}

fn cell_size() -> f32 {
    (PANEL_W - GAP * (GRID_COLUMNS - 1) as f32) / GRID_COLUMNS as f32
}

/// The active tab's grid of square buttons: 5 rows of 3, numbered left to right, top down.
pub fn grid_cells(width: f32, height: f32) -> Vec<Button> {
    let s = ui_scale(height);
    let left = panel_x(width, height);
    let cell = cell_size();
    let mut all = Vec::new();
    for row in 0..GRID_ROWS {
        for col in 0..GRID_COLUMNS {
            all.push(Button {
                action: Action::BuildCell(row * GRID_COLUMNS + col),
                label: "",
                x: left + col as f32 * (cell + GAP) * s,
                y: (grid_top() + row as f32 * (cell + GAP)) * s,
                w: cell * s,
                h: cell * s,
            });
        }
    }
    all
}

/// The three tabs, standing out to the left of the grid.
pub fn build_tabs(width: f32, height: f32) -> Vec<Button> {
    const TABS: [(BuildTab, &str); 3] = [
        (BuildTab::Infantry, "Infantry"),
        (BuildTab::Vehicles, "Vehicles"),
        (BuildTab::Buildings, "Buildings"),
    ];
    let s = ui_scale(height);
    let x = panel_x(width, height) - (GAP + TAB_W) * s;
    let cell = cell_size();
    TABS.iter()
        .enumerate()
        .map(|(i, &(tab, label))| Button {
            action: Action::BuildTab(tab),
            label,
            x,
            y: (grid_top() + i as f32 * (cell + GAP)) * s,
            w: TAB_W * s,
            h: cell * s,
        })
        .collect()
}

/// Everything clickable in the sidebar.
pub fn all_buttons(width: f32, height: f32) -> Vec<Button> {
    let mut all = command_buttons(width, height);
    all.extend(grid_cells(width, height));
    all.extend(build_tabs(width, height));
    all
}

/// The chat's font pixel size: smaller than the rest of the interface.
pub fn chat_text_px(height: f32) -> f32 {
    (2.0 * ui_scale(height)).round().max(1.0)
}

/// The left end and vertical center of a chat line.
pub fn chat_line(line: usize, height: f32) -> (f32, f32) {
    let s = ui_scale(height);
    (MARGIN * s, (MARGIN + 12.0 + line as f32 * 26.0) * s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::text_width;
    use crate::game::small_text_px;

    const SCREENS: [(f32, f32); 6] = [
        (1280.0, 720.0),
        (1440.0, 900.0),
        (1920.0, 1080.0),
        (2880.0, 1800.0),
        (1024.0, 768.0),
        (5120.0, 1440.0),
    ];
    const W: f32 = 2880.0;
    const H: f32 = 1800.0;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.1
    }

    #[test]
    fn every_block_is_as_wide_as_the_minimap_with_its_gauge() {
        for (w, h) in SCREENS {
            let (mx, _, size) = minimap_rect(w, h);
            let (cx, _, _, outer) = gauge(w, h);
            let (left, right) = (cx - outer, mx + size);

            let buttons = command_buttons(w, h);
            let (small, wide) = buttons.split_at(SMALL_COUNT);
            let cells = grid_cells(w, h);
            for (name, row) in [("small", small), ("wide", wide), ("grid", &cells[..GRID_COLUMNS])] {
                let last = row.last().unwrap();
                assert!(close(row[0].x, left), "{name} starts with the gauge at {w}x{h}");
                assert!(close(last.x + last.w, right), "{name} ends with the minimap at {w}x{h}");
            }
        }
    }

    #[test]
    fn the_blocks_are_stacked_in_the_requested_order_and_fit_the_screen() {
        for (w, h) in SCREENS {
            let (mx, my, size) = minimap_rect(w, h);
            let (cx, cy, _, outer) = gauge(w, h);
            let gauge_bottom = clock_point(cx, cy, outer, GAUGE_START_DEGREES).1;
            let buttons = command_buttons(w, h);
            let cells = grid_cells(w, h);
            assert!(my + size <= buttons[0].y, "minimap above the small row");
            assert!(gauge_bottom <= buttons[0].y, "gauge above the small row at {w}x{h}");
            assert!(buttons[0].y + buttons[0].h <= buttons[SMALL_COUNT].y, "small row above wide row");
            assert!(buttons[SMALL_COUNT].y + buttons[SMALL_COUNT].h <= cells[0].y, "wide row above grid");
            let last = cells.last().unwrap();
            assert!(last.y + last.h <= h, "grid ends on screen at {w}x{h}");
            assert!(mx + size <= w);
        }
    }

    #[test]
    fn five_small_squares_two_wide_buttons_and_a_grid_of_five_rows_by_three() {
        let buttons = command_buttons(W, H);
        assert_eq!(buttons.len(), 7);
        assert!(close(buttons[0].w, buttons[0].h), "small ones are square");
        assert!(buttons[SMALL_COUNT].w > buttons[SMALL_COUNT].h, "the other two are wide");

        let cells = grid_cells(W, H);
        assert_eq!(cells.len(), 15);
        assert!(close(cells[0].w, cells[0].h), "cells are square");
        assert_eq!(cells[0].y, cells[2].y, "three in a row");
        assert_ne!(cells[0].y, cells[3].y);
        assert_eq!(cells[0].x, cells[12].x, "five in a column");
    }

    #[test]
    fn tabs_stand_to_the_left_of_the_grid() {
        let cells = grid_cells(W, H);
        let tabs = build_tabs(W, H);
        assert_eq!(tabs.len(), 3);
        for tab in &tabs {
            assert!(tab.x + tab.w <= cells[0].x);
            assert!(tab.y >= cells[0].y);
        }
    }

    #[test]
    fn the_gauge_clears_the_minimap_corners_and_runs_from_7_to_12_oclock() {
        let (cx, cy, inner, _) = gauge(W, H);
        let (x, y, _) = minimap_rect(W, H);
        assert!((x - cx).hypot(y - cy) <= inner, "top left corner stays inside the gauge");

        let start = clock_point(cx, cy, inner, GAUGE_START_DEGREES);
        assert!(start.0 < cx && start.1 > cy, "7 o'clock is lower left");
        let end = clock_point(cx, cy, inner, GAUGE_END_DEGREES);
        assert!(close(end.0, cx) && end.1 < cy, "12 o'clock is straight up");
    }

    #[test]
    fn money_and_energy_text_fit_their_places() {
        for (w, h) in SCREENS {
            let px = small_text_px(h);
            let (cx, _, inner, _) = gauge(w, h);
            let (right, y) = money_anchor(w, h);
            let (_, my, size) = minimap_rect(w, h);
            assert!(right - text_width("$30000") * px > cx, "money clears the gauge at {w}x{h}");
            assert!(y < my, "money is above the minimap");
            let (lx, _) = energy_label_center(w, h);
            let half = text_width("65") * px / 2.0;
            assert!(lx - half > cx - inner && lx + half < cx - size / 2.0, "energy fits at {w}x{h}");
        }
    }

    #[test]
    fn labels_fit_their_buttons() {
        for (w, h) in SCREENS {
            let px = small_text_px(h);
            for b in command_buttons(w, h) {
                assert!(text_width(b.label) * px < b.w, "{} at {w}x{h}", b.label);
            }
        }
    }

    #[test]
    fn nothing_in_the_sidebar_overlaps() {
        for (w, h) in SCREENS {
            let all = all_buttons(w, h);
            for (i, a) in all.iter().enumerate() {
                for b in &all[i + 1..] {
                    let apart = a.x + a.w <= b.x + 0.01
                        || b.x + b.w <= a.x + 0.01
                        || a.y + a.h <= b.y + 0.01
                        || b.y + b.h <= a.y + 0.01;
                    assert!(apart, "{:?} overlaps {:?} at {w}x{h}", a.action, b.action);
                }
            }
        }
    }

    #[test]
    fn the_chat_has_the_two_requested_messages_and_ten_more() {
        assert!(CHAT_MESSAGES.contains(&"Unit lost"));
        assert!(CHAT_MESSAGES.contains(&"Reinforcements are approaching"));
        assert_eq!(CHAT_MESSAGES.len(), 12);
    }

    #[test]
    fn the_chat_stays_clear_of_the_tabs_and_is_small() {
        for (w, h) in SCREENS {
            let px = chat_text_px(h);
            assert!(px <= small_text_px(h));
            let widest = CHAT_MESSAGES.iter().map(|m| text_width(m)).fold(0.0, f32::max) * px;
            let (x, y) = chat_line(CHAT_LINES - 1, h);
            assert!(x + widest < build_tabs(w, h)[0].x, "at {w}x{h}");
            assert!(y < h / 2.0);
        }
    }
}
