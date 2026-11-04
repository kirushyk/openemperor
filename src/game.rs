//! Game flow and menu logic. No OS or GPU code, so it runs under `cargo test`.

use crate::battlefield::{self, Camera};
use crate::hud;
use crate::units::{self, Unit};
use crate::platform::{Event, Key};

pub const INTRO_SECONDS: f32 = 3.0;
pub const LOADING_SECONDS: f32 = 2.0;
pub const MOVIE_SECONDS: f32 = 3.0;
/// What the player and any opponents start with outside a skirmish, which sets its own.
const DEFAULT_BATTLE_MONEY: i32 = 10000;
/// A selection smaller than this each way, at 1080 pixels tall, counts as a click.
const CLICK_SLOP: f32 = 6.0;
/// How near a click must land to a unit to pick it, at 1080 pixels tall.
const PICK_RADIUS: f32 = 40.0;
/// A placeholder reading until buildings produce and consume energy.
const STARTING_ENERGY: i32 = 65;

/// The sidebar's three build menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuildTab {
    Infantry,
    Vehicles,
    Buildings,
}

/// The three playable factions: A is blue, H red, O cyan. Each is shown as its planet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faction {
    A,
    H,
    O,
}

impl Faction {
    pub fn label(self) -> &'static str {
        match self {
            Faction::A => "Faction A",
            Faction::H => "Faction H",
            Faction::O => "Faction O",
        }
    }

    fn next(self) -> Self {
        match self {
            Faction::A => Faction::H,
            Faction::H => Faction::O,
            Faction::O => Faction::A,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Intro,
    MainMenu,
    SinglePlayerMenu,
    Options,
    Credits,
    FactionSelect,
    Movie(Faction),
    SkirmishSetup,
    Loading,
    InGame,
    /// Esc on the battlefield. This and the options and credits screens opened from it
    /// draw over the battle while `Game::in_battle` holds.
    EscapeMenu,
}

/// One line of the diplomacy window.
#[derive(Clone, Debug, PartialEq)]
pub struct StatsRow {
    pub name: String,
    pub faction: Option<Faction>,
    pub color: TeamColor,
    pub allies: [Ally; 2],
    pub money: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

/// An ally slot holds one of the five mini factions, or nobody. A side can befriend at most
/// two, and never the same one twice. Each is drawn as its own shape: T is a triangle, X a
/// square, G a diamond, F a hexagon and S a star.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ally {
    None,
    T,
    X,
    G,
    F,
    S,
}

impl Ally {
    fn next(self) -> Self {
        match self {
            Ally::None => Ally::T,
            Ally::T => Ally::X,
            Ally::X => Ally::G,
            Ally::G => Ally::F,
            Ally::F => Ally::S,
            Ally::S => Ally::None,
        }
    }
}

/// Moves one ally slot to its next choice, skipping the mini faction the other slot holds.
fn cycle_ally(allies: &mut [Ally; 2], slot: usize) {
    let other = allies[1 - slot];
    let mut next = allies[slot].next();
    if next == other && next != Ally::None {
        next = next.next();
    }
    allies[slot] = next;
}

/// Eight colors for at most eight players, so every player can have their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamColor {
    Blue,
    Red,
    Cyan,
    Orange,
    Green,
    Violet,
    Yellow,
    LightPink,
}

pub const TEAM_COLORS: [TeamColor; 8] = [
    TeamColor::Blue,
    TeamColor::Red,
    TeamColor::Cyan,
    TeamColor::Orange,
    TeamColor::Green,
    TeamColor::Violet,
    TeamColor::Yellow,
    TeamColor::LightPink,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpponentKind {
    Defensive,
    Competent,
    Attacker,
}

impl OpponentKind {
    pub fn label(self) -> &'static str {
        match self {
            OpponentKind::Defensive => "Defensive",
            OpponentKind::Competent => "Competent",
            OpponentKind::Attacker => "Attacker",
        }
    }

    fn next(self) -> Self {
        match self {
            OpponentKind::Defensive => OpponentKind::Competent,
            OpponentKind::Competent => OpponentKind::Attacker,
            OpponentKind::Attacker => OpponentKind::Defensive,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opponent {
    pub kind: OpponentKind,
    pub faction: Faction,
    pub allies: [Ally; 2],
    /// Only meaningful while the slot is in use; assigned when the slot fills.
    pub color: TeamColor,
}

pub const OPPONENT_SLOTS: usize = 7;
/// Named after the planet each is on; `battlefield::MAP_PLANETS` lists those.
pub const MAPS: [&str; 10] = [
    "A - Map 1",
    "A - Map 2",
    "A - Map 3",
    "A - Map 4",
    "A - Map 5",
    "C - Map 1",
    "D - Map 1",
    "G - Map 1",
    "S - Map 1",
    "T - Map 1",
];
pub const COMMANDER_NAME_MAX: usize = 16;

/// Choices on the skirmish setup screen. Kept while the game runs, not saved between runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkirmishConfig {
    pub commander: String,
    pub faction: Faction,
    pub color: TeamColor,
    /// Left and right ally.
    pub allies: [Ally; 2],
    /// The first `ai_players` slots are in use.
    pub opponents: [Opponent; OPPONENT_SLOTS],
    /// Index into `MAPS`.
    pub map: usize,
    pub difficulty: Difficulty,
    pub ai_players: i32,
    pub money: i32,
    pub units: i32,
    pub superweapons: bool,
    pub prebuilt_base: bool,
    pub living_world: bool,
    pub crates: bool,
    pub fog_of_war: bool,
}

impl SkirmishConfig {
    /// Gives slots `from..to` the first colors nobody holds. Colors already held never
    /// change, and with eight colors for eight players one is always free.
    fn assign_opponent_colors(&mut self, from: usize, to: usize) {
        for i in from..to {
            let held: Vec<TeamColor> = self.opponents[..i].iter().map(|o| o.color).collect();
            if let Some(&free) = TEAM_COLORS
                .iter()
                .find(|&&c| c != self.color && !held.contains(&c))
            {
                self.opponents[i].color = free;
            }
        }
    }

    /// Colors the player may pick: their own plus any no opponent holds.
    pub fn available_colors(&self) -> Vec<TeamColor> {
        let opponents = &self.opponents[..self.ai_players as usize];
        TEAM_COLORS
            .iter()
            .copied()
            .filter(|&c| !opponents.iter().any(|o| o.color == c))
            .collect()
    }
}

impl Default for SkirmishConfig {
    fn default() -> Self {
        const FACTIONS: [Faction; 3] =
            [Faction::H, Faction::A, Faction::O];
        let mut opponents = [Opponent {
            kind: OpponentKind::Competent,
            faction: Faction::H,
            allies: [Ally::None; 2],
            color: TeamColor::Blue,
        }; OPPONENT_SLOTS];
        for (opponent, faction) in opponents.iter_mut().zip(FACTIONS.iter().cycle()) {
            opponent.faction = *faction;
        }
        let mut config = SkirmishConfig {
            commander: "Commander".to_string(),
            faction: Faction::A,
            color: TeamColor::Blue,
            allies: [Ally::None; 2],
            opponents,
            map: 0,
            difficulty: Difficulty::Normal,
            ai_players: 3,
            money: 10000,
            units: 10,
            superweapons: true,
            prebuilt_base: false,
            living_world: true,
            crates: true,
            fog_of_war: false,
        };
        config.assign_opponent_colors(0, config.ai_players as usize);
        config
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Game,
    Graphics,
    Sound,
    MultiplayerInfo,
    Network,
    Keyboard,
    SneakPeek,
    Credits,
    Movies,
}

const TABS: [(Tab, &str); 9] = [
    (Tab::Game, "Game"),
    (Tab::Graphics, "Graphics"),
    (Tab::Sound, "Sound"),
    (Tab::MultiplayerInfo, "Multiplayer Info"),
    (Tab::Network, "Network"),
    (Tab::Keyboard, "Keyboard"),
    (Tab::SneakPeek, "Sneak Peek"),
    (Tab::Credits, "Credits"),
    (Tab::Movies, "Movies"),
];

impl Tab {
    /// Panel text for tabs that have no content yet.
    pub fn placeholder(self) -> Option<&'static str> {
        match self {
            Tab::MultiplayerInfo => Some("Multiplayer Info placeholder"),
            Tab::Network => Some("Network placeholder"),
            Tab::Keyboard => Some("Keyboard placeholder"),
            Tab::SneakPeek => Some("Sneak Peek placeholder"),
            Tab::Movies => Some("Movies placeholder"),
            // Credits opens its own screen, so its panel is never shown.
            Tab::Game | Tab::Graphics | Tab::Sound | Tab::Credits => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderId {
    GameSpeed,
    ScrollRate,
    MouseSpeed,
    GraphicsQuality,
    MusicVolume,
    SoundVolume,
    VoiceVolume,
    AiPlayers,
    Money,
    Units,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    ToolTips,
    TargetLines,
    Superweapons,
    PrebuiltBase,
    LivingWorld,
    Crates,
    FogOfWar,
}

/// Option values. Not applied to anything yet, and not saved between runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub game_speed: i32,
    pub scroll_rate: i32,
    pub mouse_speed: i32,
    pub tool_tips: bool,
    pub target_lines: bool,
    /// Index into `QUALITY_STOPS`.
    pub graphics_quality: i32,
    pub music_volume: i32,
    pub sound_volume: i32,
    pub voice_volume: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            game_speed: 5,
            scroll_rate: 5,
            mouse_speed: 5,
            tool_tips: true,
            target_lines: true,
            graphics_quality: 2,
            music_volume: 80,
            sound_volume: 80,
            voice_volume: 80,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Tab(Tab),
    Toggle(Toggle),
    Faction(Faction),
    ConfirmFaction,
    CancelFaction,
    StartBattle,
    SaveGame,
    ExitGame,
    ReturnToGame,
    /// The sidebar's fake buttons, which do nothing yet.
    HudSmall(usize),
    HudWide(usize),
    BuildCell(usize),
    BuildTab(BuildTab),
    NameField,
    MapDropdown,
    SelectMap(usize),
    SetDifficulty(Difficulty),
    CycleFaction,
    /// 0 is the left ally, 1 the right.
    CycleAlly(usize),
    ColorBar,
    PickColor(TeamColor),
    CycleOpponentKind(usize),
    CycleOpponentFaction(usize),
    /// Opponent slot, then which of its two allies.
    CycleOpponentAlly(usize, usize),
    SinglePlayer,
    Multiplayer,
    Options,
    Exit,
    Back,
    StartCampaign,
    Tutorial,
    SkirmishBattle,
    LoadGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Center,
    BottomLeft,
    BottomRight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Button {
    pub action: Action,
    pub label: &'static str,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Button {
    /// Round buttons fill their box as a circle.
    pub fn is_round(&self) -> bool {
        matches!(
            self.action,
            Action::Faction(_)
                | Action::CycleFaction
                | Action::CycleAlly(_)
                | Action::CycleOpponentFaction(_)
                | Action::CycleOpponentAlly(..)
        )
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        if self.is_round() {
            let r = self.w / 2.0;
            let (dx, dy) = (x - (self.x + r), y - (self.y + r));
            dx * dx + dy * dy <= r * r
        } else {
            x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
        }
    }
}

const MAIN_MENU: &[(Action, &str, Anchor)] = &[
    (Action::SinglePlayer, "Single Player", Anchor::Center),
    (Action::Multiplayer, "Multiplayer", Anchor::Center),
    (Action::Exit, "Exit", Anchor::BottomLeft),
    (Action::Options, "Options", Anchor::BottomRight),
];

const SINGLE_PLAYER_MENU: &[(Action, &str, Anchor)] = &[
    (Action::StartCampaign, "Start Campaign", Anchor::Center),
    (Action::Tutorial, "Tutorial", Anchor::Center),
    (Action::SkirmishBattle, "Single Skirmish Battle", Anchor::Center),
    (Action::LoadGame, "Load Game", Anchor::Center),
    (Action::Back, "Back", Anchor::BottomLeft),
];

const BACK_ONLY: &[(Action, &str, Anchor)] = &[(Action::Back, "Back", Anchor::BottomLeft)];

const ESCAPE_MENU: &[(Action, &str, Anchor)] = &[
    (Action::LoadGame, "Load Game", Anchor::Center),
    (Action::SaveGame, "Save Game", Anchor::Center),
    (Action::Options, "Options", Anchor::Center),
    (Action::ExitGame, "Exit Game", Anchor::Center),
    (Action::ReturnToGame, "Return to Game", Anchor::Center),
];

/// Layout is authored for a 1080-pixel-tall screen and scaled to the real height.
pub fn ui_scale(height: f32) -> f32 {
    height / 1080.0
}

/// Font pixel size for tabs and option rows. Rounds down so text never outgrows its slot.
pub fn small_text_px(height: f32) -> f32 {
    (3.0 * ui_scale(height) + 0.01).floor().max(1.0)
}

const PANEL_W: f32 = 900.0;
const PANEL_H: f32 = 568.0;
const PANEL_Y: f32 = 280.0;
/// How far right of the screen center the panel's center sits.
const PANEL_OFFSET: f32 = 130.0;
const TAB_W: f32 = 320.0;
const TAB_H: f32 = 56.0;
const TAB_GAP: f32 = 8.0;
const ROW_TOP: f32 = 40.0;
const ROW_STEP: f32 = 80.0;
const ROW_H: f32 = 72.0;
const LABEL_X: f32 = 30.0;
const TRACK_X: f32 = 340.0;
const TRACK_W: f32 = 420.0;
const CHECKBOX_SIZE: f32 = 36.0;

/// The tabbed panel as (x, y, w, h).
pub fn options_panel(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    let x = width / 2.0 + (PANEL_OFFSET - PANEL_W / 2.0) * s;
    (x, PANEL_Y * s, PANEL_W * s, PANEL_H * s)
}

pub const QUALITY_STOPS: &[&str] = &["Low", "Medium", "High"];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slider {
    pub id: SliderId,
    pub label: &'static str,
    pub label_x: f32,
    pub cy: f32,
    pub track_x: f32,
    pub track_w: f32,
    pub min: i32,
    pub max: i32,
    /// Values snap to multiples of this, counted from `min`.
    pub step: i32,
    /// Named positions; when empty the number is shown instead.
    pub stops: &'static [&'static str],
    pub label_cy: f32,
    pub value_right: f32,
    pub handle_h: f32,
    hit_half_height: f32,
}

impl Slider {
    pub fn x_of(&self, value: i32) -> f32 {
        let t = (value - self.min) as f32 / (self.max - self.min) as f32;
        self.track_x + t * self.track_w
    }

    fn value_at(&self, x: f32) -> i32 {
        let t = ((x - self.track_x) / self.track_w).clamp(0.0, 1.0);
        let steps = (t * ((self.max - self.min) / self.step) as f32).round() as i32;
        self.min + steps * self.step
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        let pad = self.hit_half_height;
        x >= self.track_x - pad
            && x <= self.track_x + self.track_w + pad
            && (y - self.cy).abs() <= pad
    }
}

type SliderRow = (SliderId, &'static str, i32, i32, &'static [&'static str]);

const GAME_SLIDERS: &[SliderRow] = &[
    (SliderId::GameSpeed, "Game Speed", 1, 10, &[]),
    (SliderId::ScrollRate, "Scroll Rate", 1, 10, &[]),
    (SliderId::MouseSpeed, "Mouse Speed", 1, 10, &[]),
];
const GRAPHICS_SLIDERS: &[SliderRow] =
    &[(SliderId::GraphicsQuality, "Graphics Quality", 0, 2, QUALITY_STOPS)];
const SOUND_SLIDERS: &[SliderRow] = &[
    (SliderId::MusicVolume, "Music Volume", 0, 100, &[]),
    (SliderId::SoundVolume, "Sound Volume", 0, 100, &[]),
    (SliderId::VoiceVolume, "Voice Volume", 0, 100, &[]),
];
const GAME_TOGGLES: &[(Toggle, &str)] =
    &[(Toggle::ToolTips, "Tool Tips"), (Toggle::TargetLines, "Target Lines")];

pub fn sliders(tab: Tab, width: f32, height: f32) -> Vec<Slider> {
    let rows = match tab {
        Tab::Game => GAME_SLIDERS,
        Tab::Graphics => GRAPHICS_SLIDERS,
        Tab::Sound => SOUND_SLIDERS,
        _ => &[],
    };
    let s = ui_scale(height);
    let (px, py, _, _) = options_panel(width, height);
    rows.iter()
        .enumerate()
        .map(|(i, &(id, label, min, max, stops))| {
            let cy = py + (ROW_TOP + i as f32 * ROW_STEP + ROW_H / 2.0) * s;
            Slider {
                id,
                label,
                label_x: px + LABEL_X * s,
                cy,
                track_x: px + TRACK_X * s,
                track_w: TRACK_W * s,
                min,
                max,
                step: 1,
                stops,
                label_cy: cy,
                value_right: px + (TRACK_X + TRACK_W + 110.0) * s,
                handle_h: 44.0 * s,
                hit_half_height: ROW_H / 2.0 * s,
            }
        })
        .collect()
}

/// Checkbox rows: the whole row is clickable and the box sits at its right end.
pub fn checkboxes(tab: Tab, width: f32, height: f32) -> Vec<Button> {
    let toggles = if tab == Tab::Game { GAME_TOGGLES } else { &[] };
    let first_row = sliders(tab, width, height).len();
    let s = ui_scale(height);
    let (px, py, _, _) = options_panel(width, height);
    toggles
        .iter()
        .enumerate()
        .map(|(i, &(toggle, label))| Button {
            action: Action::Toggle(toggle),
            label,
            x: px + LABEL_X * s,
            y: py + (ROW_TOP + (first_row + i) as f32 * ROW_STEP) * s,
            w: (TRACK_X - LABEL_X + CHECKBOX_SIZE) * s,
            h: ROW_H * s,
        })
        .collect()
}

pub fn checkbox_size(height: f32) -> f32 {
    CHECKBOX_SIZE * ui_scale(height)
}

/// The three factions form a triangle: A on top, B and C below.
fn faction_buttons(width: f32, height: f32) -> Vec<Button> {
    const RADIUS: f32 = 130.0;
    const SPREAD: f32 = 200.0;
    const TOP_Y: f32 = 370.0;
    const BOTTOM_Y: f32 = 716.0;
    let s = ui_scale(height);
    [
        (Faction::A, 0.0, TOP_Y),
        (Faction::H, -SPREAD, BOTTOM_Y),
        (Faction::O, SPREAD, BOTTOM_Y),
    ]
    .into_iter()
    .map(|(faction, dx, cy)| Button {
        action: Action::Faction(faction),
        label: faction.label(),
        x: width / 2.0 + (dx - RADIUS) * s,
        y: (cy - RADIUS) * s,
        w: RADIUS * 2.0 * s,
        h: RADIUS * 2.0 * s,
    })
    .collect()
}

/// One of the setup screen's outer thirds, as (x, width), with margins.
fn setup_column(left: bool, width: f32, height: f32) -> (f32, f32) {
    let s = ui_scale(height);
    let w = (width / 3.0 - 48.0 * s).min(640.0 * s);
    let x = if left { 24.0 * s } else { width * 2.0 / 3.0 + 24.0 * s };
    (x, w)
}

/// The minimap as (x, y, size).
pub fn minimap_rect(width: f32, height: f32) -> (f32, f32, f32) {
    const SIZE: f32 = 300.0;
    let s = ui_scale(height);
    let (x, w) = setup_column(false, width, height);
    (x + (w - SIZE * s) / 2.0, 40.0 * s, SIZE * s)
}

/// The player's color bar as (x, y, w, h), under the faction circle.
pub fn color_bar_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    let (x, w) = setup_column(true, width, height);
    (x, 590.0 * s, w, 44.0 * s)
}

/// The open color bar: one slot per color the player may pick.
pub fn color_slots(colors: &[TeamColor], width: f32, height: f32) -> Vec<Button> {
    let (x, y, w, h) = color_bar_rect(width, height);
    let slot_w = w / colors.len().max(1) as f32;
    colors
        .iter()
        .enumerate()
        .map(|(i, &color)| Button {
            action: Action::PickColor(color),
            label: "",
            x: x + i as f32 * slot_w,
            y,
            w: slot_w,
            h,
        })
        .collect()
}

const OPPONENT_ROW_W: f32 = 408.0;
const OPPONENT_TOP: f32 = 160.0;
const OPPONENT_STEP: f32 = 96.0;
const OPPONENT_ROW_H: f32 = 80.0;

/// One opponent row as (x, y, w, h), in the middle third. Empty slots draw here too.
pub fn opponent_row(slot: usize, width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    let x = (width - OPPONENT_ROW_W * s) / 2.0;
    let y = (OPPONENT_TOP + slot as f32 * OPPONENT_STEP) * s;
    (x, y, OPPONENT_ROW_W * s, OPPONENT_ROW_H * s)
}

/// Each row in use: a kind box in the opponent's color, a faction circle, two ally circles.
pub fn opponent_widgets(count: usize, width: f32, height: f32) -> Vec<Button> {
    const KIND_W: f32 = 180.0;
    const KIND_H: f32 = 64.0;
    const FACTION_R: f32 = 36.0;
    const ALLY_R: f32 = 28.0;
    const GAP: f32 = 16.0;
    const ALLY_GAP: f32 = 12.0;
    let s = ui_scale(height);
    let mut all = Vec::new();
    for slot in 0..count {
        let (x, y, _, h) = opponent_row(slot, width, height);
        let cy = y + h / 2.0;
        let circle = |action, label, left: f32, r: f32| Button {
            action,
            label,
            x: x + left * s,
            y: cy - r * s,
            w: r * 2.0 * s,
            h: r * 2.0 * s,
        };
        let faction_left = KIND_W + GAP;
        let ally_left = faction_left + FACTION_R * 2.0 + GAP;
        all.push(Button {
            action: Action::CycleOpponentKind(slot),
            label: "Kind",
            x,
            y: cy - KIND_H / 2.0 * s,
            w: KIND_W * s,
            h: KIND_H * s,
        });
        all.push(circle(Action::CycleOpponentFaction(slot), "Faction", faction_left, FACTION_R));
        all.push(circle(Action::CycleOpponentAlly(slot, 0), "Ally", ally_left, ALLY_R));
        all.push(circle(
            Action::CycleOpponentAlly(slot, 1),
            "Ally",
            ally_left + ALLY_R * 2.0 + ALLY_GAP,
            ALLY_R,
        ));
    }
    all
}

/// Name field, map dropdown, difficulty radios, faction and ally circles, color bar, Play.
pub fn setup_widgets(width: f32, height: f32) -> Vec<Button> {
    const FACTION_CY: f32 = 430.0;
    const FACTION_R: f32 = 100.0;
    const ALLY_R: f32 = 52.0;
    const ALLY_OFFSET: f32 = 164.0;
    let s = ui_scale(height);
    let (lx, lw) = setup_column(true, width, height);
    let (rx, rw) = setup_column(false, width, height);
    let center = lx + lw / 2.0;
    let circle = |action, label, dx: f32, r: f32| Button {
        action,
        label,
        x: center + (dx - r) * s,
        y: (FACTION_CY - r) * s,
        w: r * 2.0 * s,
        h: r * 2.0 * s,
    };
    let radio = |i: f32, action, label| Button {
        action,
        label,
        x: rx + i * rw / 3.0,
        y: 420.0 * s,
        w: rw / 3.0,
        h: 44.0 * s,
    };
    let (bar_x, bar_y, bar_w, bar_h) = color_bar_rect(width, height);
    vec![
        Button { action: Action::NameField, label: "Commander Name", x: lx, y: 84.0 * s, w: lw, h: 56.0 * s },
        circle(Action::CycleFaction, "Faction", 0.0, FACTION_R),
        circle(Action::CycleAlly(0), "Ally", -ALLY_OFFSET, ALLY_R),
        circle(Action::CycleAlly(1), "Ally", ALLY_OFFSET, ALLY_R),
        Button { action: Action::ColorBar, label: "Color", x: bar_x, y: bar_y, w: bar_w, h: bar_h },
        Button {
            action: Action::StartBattle,
            label: "Play",
            x: center - 120.0 * s,
            y: bar_y + bar_h + 16.0 * s,
            w: 240.0 * s,
            h: 72.0 * s,
        },
        Button { action: Action::MapDropdown, label: "Map", x: rx, y: 352.0 * s, w: rw, h: 52.0 * s },
        radio(0.0, Action::SetDifficulty(Difficulty::Easy), "Easy"),
        radio(1.0, Action::SetDifficulty(Difficulty::Normal), "Normal"),
        radio(2.0, Action::SetDifficulty(Difficulty::Hard), "Hard"),
    ]
}

/// The open dropdown's entries, directly below it.
pub fn map_items(width: f32, height: f32) -> Vec<Button> {
    let s = ui_scale(height);
    let (x, w) = setup_column(false, width, height);
    MAPS.iter()
        .enumerate()
        .map(|(i, &label)| Button {
            action: Action::SelectMap(i),
            label,
            x,
            y: (404.0 + i as f32 * 52.0) * s,
            w,
            h: 52.0 * s,
        })
        .collect()
}

pub fn setup_sliders(width: f32, height: f32) -> Vec<Slider> {
    const ROWS: [(SliderId, &str, i32, i32, i32); 3] = [
        (SliderId::AiPlayers, "AI Players", 1, 7, 1),
        (SliderId::Money, "Money", 0, 30000, 500),
        (SliderId::Units, "Units", 0, 30, 1),
    ];
    let s = ui_scale(height);
    let (x, w) = setup_column(false, width, height);
    ROWS.iter()
        .enumerate()
        .map(|(i, &(id, label, min, max, step))| {
            let y = 476.0 + i as f32 * 64.0;
            Slider {
                id,
                label,
                label_x: x,
                cy: (y + 44.0) * s,
                track_x: x + 10.0 * s,
                track_w: w - 20.0 * s,
                min,
                max,
                step,
                stops: &[],
                label_cy: (y + 12.0) * s,
                value_right: x + w,
                handle_h: 28.0 * s,
                hit_half_height: 28.0 * s,
            }
        })
        .collect()
}

pub fn setup_checkboxes(width: f32, height: f32) -> Vec<Button> {
    const ROWS: [(Toggle, &str); 5] = [
        (Toggle::Superweapons, "Superweapons"),
        (Toggle::PrebuiltBase, "Prebuilt Base"),
        (Toggle::LivingWorld, "Living World"),
        (Toggle::Crates, "Crates"),
        (Toggle::FogOfWar, "Fog of War"),
    ];
    let s = ui_scale(height);
    let (x, w) = setup_column(false, width, height);
    ROWS.iter()
        .enumerate()
        .map(|(i, &(toggle, label))| Button {
            action: Action::Toggle(toggle),
            label,
            x,
            y: (676.0 + i as f32 * 46.0) * s,
            w,
            h: 44.0 * s,
        })
        .collect()
}

/// Where the diplomacy window's rows start, and their spacing, at 1080 pixels tall.
pub const STATS_ROW_TOP: f32 = 120.0;
pub const STATS_ROW_STEP: f32 = 76.0;

/// The diplomacy window as (x, y, w, h), centered on the screen.
pub fn stats_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    let (w, h) = (1000.0 * s, 760.0 * s);
    ((width - w) / 2.0, (height - h) / 2.0, w, h)
}

pub const CONFIRM_MESSAGE: &str = "Are you sure you will play for those guys?";

/// The confirmation message box as (x, y, w, h), centered on the screen.
pub fn modal_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    let (w, h) = (900.0 * s, 320.0 * s);
    ((width - w) / 2.0, (height - h) / 2.0, w, h)
}

pub fn modal_buttons(width: f32, height: f32) -> Vec<Button> {
    const W: f32 = 240.0;
    const H: f32 = 72.0;
    const GAP: f32 = 40.0;
    let s = ui_scale(height);
    let (_, my, _, mh) = modal_rect(width, height);
    let y = my + mh - (H + 40.0) * s;
    vec![
        Button {
            action: Action::ConfirmFaction,
            label: "OK",
            x: width / 2.0 - (W + GAP / 2.0) * s,
            y,
            w: W * s,
            h: H * s,
        },
        Button {
            action: Action::CancelFaction,
            label: "Cancel",
            x: width / 2.0 + GAP / 2.0 * s,
            y,
            w: W * s,
            h: H * s,
        },
    ]
}

fn tab_buttons(width: f32, height: f32) -> Vec<Button> {
    let s = ui_scale(height);
    let (px, py, _, _) = options_panel(width, height);
    TABS.iter()
        .enumerate()
        .map(|(i, &(tab, label))| Button {
            action: Action::Tab(tab),
            label,
            x: px - (TAB_GAP + TAB_W) * s,
            y: py + i as f32 * (TAB_H + TAB_GAP) * s,
            w: TAB_W * s,
            h: TAB_H * s,
        })
        .collect()
}

pub fn buttons(screen: Screen, width: f32, height: f32) -> Vec<Button> {
    const CENTER_W: f32 = 620.0;
    const CORNER_W: f32 = 240.0;
    const BUTTON_H: f32 = 72.0;
    const GAP: f32 = 20.0;
    const MARGIN: f32 = 48.0;

    let items = match screen {
        Screen::Intro | Screen::Loading | Screen::Movie(_) | Screen::InGame => return Vec::new(),
        Screen::MainMenu => MAIN_MENU,
        Screen::SinglePlayerMenu => SINGLE_PLAYER_MENU,
        Screen::EscapeMenu => ESCAPE_MENU,
        Screen::Options | Screen::Credits | Screen::FactionSelect | Screen::SkirmishSetup => BACK_ONLY,
    };
    let s = ui_scale(height);
    let centered = items.iter().filter(|i| i.2 == Anchor::Center).count() as f32;
    let stack_h = centered * BUTTON_H + (centered - 1.0) * GAP;
    let mut y = (1080.0 - stack_h) / 2.0;

    let mut all: Vec<Button> = items
        .iter()
        .map(|&(action, label, anchor)| {
            let (px, py, w) = match anchor {
                Anchor::Center => {
                    let top = y;
                    y += BUTTON_H + GAP;
                    ((width - CENTER_W * s) / 2.0, top * s, CENTER_W)
                }
                Anchor::BottomLeft => {
                    (MARGIN * s, (1080.0 - MARGIN - BUTTON_H) * s, CORNER_W)
                }
                Anchor::BottomRight => (
                    width - (MARGIN + CORNER_W) * s,
                    (1080.0 - MARGIN - BUTTON_H) * s,
                    CORNER_W,
                ),
            };
            Button { action, label, x: px, y: py, w: w * s, h: BUTTON_H * s }
        })
        .collect();
    if screen == Screen::Options {
        all.extend(tab_buttons(width, height));
    }
    if screen == Screen::FactionSelect {
        all.extend(faction_buttons(width, height));
    }
    all
}

pub struct Game {
    pub screen: Screen,
    pub width: f32,
    pub height: f32,
    pub mouse: (f32, f32),
    pub pressed: Option<Action>,
    pub quit: bool,
    pub tab: Tab,
    pub settings: Settings,
    pub dragging: Option<SliderId>,
    /// Faction awaiting a yes or no in the message box; it blocks the screen behind it.
    pub confirming: Option<Faction>,
    /// Campaign faction of the current game; `None` in a single battle.
    pub faction: Option<Faction>,
    pub skirmish: SkirmishConfig,
    pub name_focused: bool,
    pub dropdown_open: bool,
    pub color_open: bool,
    /// Index into `MAPS` of the current battle.
    pub battle_map: usize,
    pub camera: Camera,
    /// False while another app is in front; the cursor then says nothing about the game.
    pub focused: bool,
    /// A battle is under way, even while a menu is drawn over it.
    pub in_battle: bool,
    /// Network games cannot stop for one player. Every game is pausable until those exist.
    pub pausable: bool,
    /// Paused with the Pause key rather than by a menu being open.
    pub key_paused: bool,
    pub stats_open: bool,
    /// Pressed on the battlefield minimap and not yet released.
    minimap_dragging: bool,
    pub units: Vec<Unit>,
    /// Where a selection drag started, as a point on the map (x, z), so that it stays put
    /// on the ground while the camera moves.
    selection_anchor: Option<(f32, f32)>,
    pub build_tab: BuildTab,
    pub energy: i32,
    /// The player's money first, then each opponent's.
    money: Vec<i32>,
    /// Newest last, at most `hud::CHAT_LINES`.
    pub chat: Vec<&'static str>,
    next_chat_at: f32,
    chat_random: u32,
    skirmish_battle: bool,
    /// Seconds of battle played, which stand still while paused.
    battle_time: f32,
    /// Seconds since the current screen opened.
    screen_elapsed: f32,
}

impl Game {
    pub fn new(width: f32, height: f32) -> Self {
        Game {
            screen: Screen::Intro,
            width,
            height,
            mouse: (width / 2.0, height / 2.0),
            pressed: None,
            quit: false,
            tab: Tab::Game,
            settings: Settings::default(),
            dragging: None,
            confirming: None,
            faction: None,
            skirmish: SkirmishConfig::default(),
            name_focused: false,
            dropdown_open: false,
            color_open: false,
            battle_map: 0,
            camera: Camera::centered(),
            focused: true,
            in_battle: false,
            pausable: true,
            key_paused: false,
            stats_open: false,
            minimap_dragging: false,
            units: Vec::new(),
            selection_anchor: None,
            build_tab: BuildTab::Infantry,
            energy: STARTING_ENERGY,
            money: vec![DEFAULT_BATTLE_MONEY],
            chat: Vec::new(),
            next_chat_at: 0.0,
            chat_random: 0x9E37_79B9,
            skirmish_battle: false,
            battle_time: 0.0,
            screen_elapsed: 0.0,
        }
    }

    pub fn battle_time(&self) -> f32 {
        self.battle_time
    }

    /// A battle stands still with the Pause key on, or with a menu over it if it can.
    pub fn is_paused(&self) -> bool {
        self.in_battle && self.pausable && (self.key_paused || self.screen != Screen::InGame)
    }

    fn view(&self) -> battlefield::View {
        battlefield::View::new(self.camera, self.width, self.height)
    }

    /// The selection on screen as (left, top, right, bottom), while dragging one. It can
    /// reach far off screen, since its start stays where it was on the map.
    pub fn selection_rect(&self) -> Option<(f32, f32, f32, f32)> {
        let (x, z) = self.selection_anchor?;
        let a = self.view().project_clamped(x, 0.0, z);
        let b = self.mouse;
        Some((a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)))
    }

    /// Whether the drag in progress would select this unit.
    pub fn in_selection_drag(&self, unit: &Unit) -> bool {
        match self.selection_anchor {
            Some(anchor) if unit.owner == 0 => units::in_selection(&self.view(), anchor, self.mouse, unit),
            _ => false,
        }
    }

    fn begin_selection(&mut self) {
        let (x, y) = self.mouse;
        if self.screen != Screen::InGame
            || self.key_paused
            || self.stats_open
            || hud::covers(x, y, self.width, self.height)
        {
            return;
        }
        self.selection_anchor = self.view().ground_at(x, y);
    }

    /// Ends the drag: a real box selects the player's units in it, replacing the old
    /// selection, and anything smaller acts as a click.
    fn finish_selection(&mut self) {
        let Some((left, top, right, bottom)) = self.selection_rect() else { return };
        let slop = CLICK_SLOP * ui_scale(self.height);
        let is_click = right - left < slop && bottom - top < slop;
        let picked = if is_click { self.unit_under_cursor() } else { None };
        let selected: Vec<bool> = self
            .units
            .iter()
            .enumerate()
            .map(|(i, unit)| if is_click { picked == Some(i) } else { self.in_selection_drag(unit) })
            .collect();
        for (unit, selected) in self.units.iter_mut().zip(selected) {
            unit.selected = selected;
        }
        self.selection_anchor = None;
    }

    /// The nearest of the player's units to the cursor, within reach of a click.
    fn unit_under_cursor(&self) -> Option<usize> {
        let view = self.view();
        let reach = PICK_RADIUS * ui_scale(self.height);
        let mut best = None;
        let mut best_distance = reach;
        for (i, unit) in self.units.iter().enumerate().filter(|(_, u)| u.owner == 0) {
            let Some((x, y)) = view.project(unit.x, unit.y + units::HEIGHT / 2.0, unit.z) else { continue };
            let distance = (x - self.mouse.0).hypot(y - self.mouse.1);
            if distance <= best_distance {
                best = Some(i);
                best_distance = distance;
            }
        }
        best
    }

    pub fn player_money(&self) -> i32 {
        self.money[0]
    }

    fn next_random(&mut self) -> u32 {
        // Xorshift: plenty for fake messages, and it needs no dependency.
        self.chat_random ^= self.chat_random << 13;
        self.chat_random ^= self.chat_random >> 17;
        self.chat_random ^= self.chat_random << 5;
        self.chat_random
    }

    /// Posts a random message every few seconds of battle, scrolling out the oldest.
    fn update_chat(&mut self) {
        while self.battle_time >= self.next_chat_at {
            let index = self.next_random() as usize % hud::CHAT_MESSAGES.len();
            self.chat.push(hud::CHAT_MESSAGES[index]);
            if self.chat.len() > hud::CHAT_LINES {
                self.chat.remove(0);
            }
            self.next_chat_at += 3.0 + (self.next_random() % 5) as f32;
        }
    }

    /// The player, then each opponent in this battle.
    pub fn stats_rows(&self) -> Vec<StatsRow> {
        let setup = &self.skirmish;
        if !self.skirmish_battle {
            let color = match self.faction {
                Some(Faction::H) => TeamColor::Red,
                Some(Faction::O) => TeamColor::Cyan,
                Some(Faction::A) | None => TeamColor::Blue,
            };
            return vec![StatsRow {
                name: setup.commander.clone(),
                faction: self.faction,
                color,
                allies: [Ally::None; 2],
                money: self.player_money(),
            }];
        }
        let mut rows = vec![StatsRow {
            name: setup.commander.clone(),
            faction: Some(setup.faction),
            color: setup.color,
            allies: setup.allies,
            money: self.player_money(),
        }];
        for (i, opponent) in setup.opponents[..setup.ai_players as usize].iter().enumerate() {
            rows.push(StatsRow {
                name: format!("AI {} ({})", i + 1, opponent.kind.label()),
                faction: Some(opponent.faction),
                color: opponent.color,
                allies: opponent.allies,
                money: self.money[i + 1],
            });
        }
        rows
    }

    fn leave_battle(&mut self) {
        self.in_battle = false;
        self.key_paused = false;
        self.stats_open = false;
        self.set_screen(Screen::MainMenu);
    }

    fn click_battle_minimap(&mut self) {
        if self.screen != Screen::InGame || self.key_paused || self.stats_open {
            return;
        }
        let rect = hud::minimap_rect(self.width, self.height);
        if let Some((x, z)) = battlefield::minimap_to_world(rect, self.mouse.0, self.mouse.1) {
            self.camera.look_at(x, z);
            self.minimap_dragging = true;
        }
    }

    /// Follows the cursor until release, holding to the minimap's edge if it wanders off.
    fn drag_battle_minimap(&mut self) {
        if !self.minimap_dragging || self.key_paused {
            return;
        }
        let rect = hud::minimap_rect(self.width, self.height);
        let (x, y, size) = rect;
        let px = self.mouse.0.clamp(x, x + size - 0.01);
        let py = self.mouse.1.clamp(y, y + size - 0.01);
        if let Some((world_x, world_z)) = battlefield::minimap_to_world(rect, px, py) {
            self.camera.look_at(world_x, world_z);
        }
    }

    fn start_battle(&mut self, map: usize, faction: Option<Faction>, skirmish: bool) {
        self.battle_map = map;
        self.faction = faction;
        self.skirmish_battle = skirmish;
        self.camera = Camera::centered();
        self.in_battle = true;
        self.pausable = true;
        self.key_paused = false;
        self.stats_open = false;
        self.battle_time = 0.0;
        self.build_tab = BuildTab::Infantry;
        self.energy = STARTING_ENERGY;
        self.money = if skirmish {
            vec![self.skirmish.money; 1 + self.skirmish.ai_players as usize]
        } else {
            vec![DEFAULT_BATTLE_MONEY]
        };
        self.chat.clear();
        self.next_chat_at = 1.0;
        let colors: Vec<TeamColor> = self.stats_rows().iter().map(|row| row.color).collect();
        self.units = units::spawn(battlefield::terrain(map), &colors);
        self.set_screen(Screen::Loading);
    }

    /// Seconds since the current screen opened, for simple animation.
    pub fn screen_elapsed(&self) -> f32 {
        self.screen_elapsed
    }

    pub fn slider_value(&self, id: SliderId) -> i32 {
        match id {
            SliderId::GameSpeed => self.settings.game_speed,
            SliderId::ScrollRate => self.settings.scroll_rate,
            SliderId::MouseSpeed => self.settings.mouse_speed,
            SliderId::GraphicsQuality => self.settings.graphics_quality,
            SliderId::MusicVolume => self.settings.music_volume,
            SliderId::SoundVolume => self.settings.sound_volume,
            SliderId::VoiceVolume => self.settings.voice_volume,
            SliderId::AiPlayers => self.skirmish.ai_players,
            SliderId::Money => self.skirmish.money,
            SliderId::Units => self.skirmish.units,
        }
    }

    fn set_slider(&mut self, id: SliderId, value: i32) {
        let old_ai_players = self.skirmish.ai_players as usize;
        let field = match id {
            SliderId::GameSpeed => &mut self.settings.game_speed,
            SliderId::ScrollRate => &mut self.settings.scroll_rate,
            SliderId::MouseSpeed => &mut self.settings.mouse_speed,
            SliderId::GraphicsQuality => &mut self.settings.graphics_quality,
            SliderId::MusicVolume => &mut self.settings.music_volume,
            SliderId::SoundVolume => &mut self.settings.sound_volume,
            SliderId::VoiceVolume => &mut self.settings.voice_volume,
            SliderId::AiPlayers => &mut self.skirmish.ai_players,
            SliderId::Money => &mut self.skirmish.money,
            SliderId::Units => &mut self.skirmish.units,
        };
        *field = value;
        if id == SliderId::AiPlayers {
            self.skirmish.assign_opponent_colors(old_ai_players, value as usize);
        }
    }

    pub fn toggle_value(&self, toggle: Toggle) -> bool {
        match toggle {
            Toggle::ToolTips => self.settings.tool_tips,
            Toggle::TargetLines => self.settings.target_lines,
            Toggle::Superweapons => self.skirmish.superweapons,
            Toggle::PrebuiltBase => self.skirmish.prebuilt_base,
            Toggle::LivingWorld => self.skirmish.living_world,
            Toggle::Crates => self.skirmish.crates,
            Toggle::FogOfWar => self.skirmish.fog_of_war,
        }
    }

    fn flip(&mut self, toggle: Toggle) {
        let field = match toggle {
            Toggle::ToolTips => &mut self.settings.tool_tips,
            Toggle::TargetLines => &mut self.settings.target_lines,
            Toggle::Superweapons => &mut self.skirmish.superweapons,
            Toggle::PrebuiltBase => &mut self.skirmish.prebuilt_base,
            Toggle::LivingWorld => &mut self.skirmish.living_world,
            Toggle::Crates => &mut self.skirmish.crates,
            Toggle::FogOfWar => &mut self.skirmish.fog_of_war,
        };
        *field = !*field;
    }

    fn edit_name(&mut self, key: Key) {
        match key {
            Key::Char(c) if self.skirmish.commander.len() < COMMANDER_NAME_MAX => {
                self.skirmish.commander.push(c);
            }
            Key::Backspace => {
                self.skirmish.commander.pop();
            }
            Key::Enter | Key::Escape => self.name_focused = false,
            Key::Char(_) | Key::Tab | Key::Pause | Key::Other => {}
        }
    }

    /// Progress of the dummy loading screen, 0 to 1.
    pub fn loading_progress(&self) -> f32 {
        (self.screen_elapsed / LOADING_SECONDS).min(1.0)
    }

    /// The buttons that accept input right now.
    pub fn buttons(&self) -> Vec<Button> {
        if self.confirming.is_some() {
            modal_buttons(self.width, self.height)
        } else if self.dropdown_open {
            map_items(self.width, self.height)
        } else if self.color_open {
            color_slots(&self.skirmish.available_colors(), self.width, self.height)
        } else {
            buttons(self.screen, self.width, self.height)
        }
    }

    fn finish_movie(&mut self, faction: Faction) {
        self.set_screen(Screen::FactionSelect);
        self.confirming = Some(faction);
    }

    /// Sliders on screen.
    pub fn sliders(&self) -> Vec<Slider> {
        match self.screen {
            Screen::Options => sliders(self.tab, self.width, self.height),
            Screen::SkirmishSetup => setup_sliders(self.width, self.height),
            _ => Vec::new(),
        }
    }

    /// Checkbox rows on screen.
    pub fn checkboxes(&self) -> Vec<Button> {
        match self.screen {
            Screen::Options => checkboxes(self.tab, self.width, self.height),
            Screen::SkirmishSetup => setup_checkboxes(self.width, self.height),
            _ => Vec::new(),
        }
    }

    pub fn hovered(&self) -> Option<Action> {
        if self.dragging.is_some() || self.selection_anchor.is_some() {
            return None;
        }
        let (x, y) = self.mouse;
        let buttons = self.buttons().into_iter();
        if self.dropdown_open || self.color_open {
            // An open list takes all input; what lies around it stays inactive.
            return buttons.into_iter().find(|b| b.contains(x, y)).map(|b| b.action);
        }
        let mut widgets = Vec::new();
        if self.screen == Screen::InGame && !self.key_paused {
            widgets = hud::all_buttons(self.width, self.height);
        }
        if self.screen == Screen::SkirmishSetup {
            widgets = setup_widgets(self.width, self.height);
            widgets.extend(opponent_widgets(
                self.skirmish.ai_players as usize,
                self.width,
                self.height,
            ));
        }
        buttons
            .chain(self.checkboxes())
            .chain(widgets)
            .find(|b| b.contains(x, y))
            .map(|b| b.action)
    }

    pub fn hovered_slider(&self) -> Option<SliderId> {
        if self.dropdown_open || self.color_open {
            return None;
        }
        let (x, y) = self.mouse;
        self.sliders().iter().find(|s| s.contains(x, y)).map(|s| s.id)
    }

    fn drag_to(&mut self, x: f32) {
        let Some(id) = self.dragging else { return };
        if let Some(slider) = self.sliders().iter().find(|s| s.id == id) {
            self.set_slider(id, slider.value_at(x));
        }
    }

    /// Where Back and Escape lead, if anywhere.
    fn parent(&self) -> Option<Screen> {
        match self.screen {
            Screen::SinglePlayerMenu => Some(Screen::MainMenu),
            Screen::Options if self.in_battle => Some(Screen::EscapeMenu),
            Screen::Options => Some(Screen::MainMenu),
            Screen::Credits => Some(Screen::Options),
            Screen::InGame => Some(Screen::EscapeMenu),
            Screen::EscapeMenu => Some(Screen::InGame),
            Screen::FactionSelect | Screen::SkirmishSetup => Some(Screen::SinglePlayerMenu),
            Screen::Intro | Screen::MainMenu | Screen::Loading | Screen::Movie(_) => None,
        }
    }

    fn go_back(&mut self) {
        // With the message box up, Escape answers it instead of leaving the screen.
        if self.confirming.take().is_some() {
            return;
        }
        if self.stats_open {
            self.stats_open = false;
            return;
        }
        if self.dropdown_open || self.color_open {
            self.dropdown_open = false;
            self.color_open = false;
            return;
        }
        if let Some(parent) = self.parent() {
            self.set_screen(parent);
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.screen_elapsed += dt;
        if self.in_battle && !self.is_paused() {
            self.battle_time += dt;
            self.update_chat();
        }
        match self.screen {
            Screen::Intro if self.screen_elapsed >= INTRO_SECONDS => self.set_screen(Screen::MainMenu),
            Screen::Loading if self.screen_elapsed >= LOADING_SECONDS => self.set_screen(Screen::InGame),
            Screen::Movie(faction) if self.screen_elapsed >= MOVIE_SECONDS => self.finish_movie(faction),
            Screen::InGame => {
                self.camera.ease_zoom(dt);
                if self.focused && !self.key_paused && !self.minimap_dragging {
                    let direction = battlefield::edge_direction(self.mouse, self.width, self.height);
                    self.camera.scroll(direction, self.settings.scroll_rate, dt);
                }
            }
            _ => {}
        }
    }

    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Quit => self.quit = true,
            Event::Focus(focused) => {
                self.focused = focused;
                if !focused {
                    // The release may happen in another app, where the game cannot see it.
                    self.selection_anchor = None;
                }
            }
            Event::Wheel(notches) => {
                if self.screen == Screen::InGame && !self.key_paused {
                    self.camera.zoom_by(notches);
                }
            }
            Event::Resize(w, h) => {
                self.width = w as f32;
                self.height = h as f32;
                self.mouse = (self.mouse.0.min(self.width - 1.0), self.mouse.1.min(self.height - 1.0));
            }
            Event::MouseMove(x, y) => {
                self.mouse = (x, y);
                self.drag_to(x);
                self.drag_battle_minimap();
            }
            Event::MouseDown => {
                self.name_focused = false;
                self.pressed = self.hovered();
                if self.pressed.is_none() {
                    self.dragging = self.hovered_slider();
                    self.drag_to(self.mouse.0);
                    self.click_battle_minimap();
                    if !self.minimap_dragging {
                        self.begin_selection();
                    }
                }
            }
            // A button fires only if the press and the release both landed on it.
            Event::MouseUp => {
                self.dragging = None;
                self.minimap_dragging = false;
                self.finish_selection();
                let pressed = self.pressed.take();
                if self.screen == Screen::Intro {
                    self.set_screen(Screen::MainMenu);
                } else if let Screen::Movie(faction) = self.screen {
                    self.finish_movie(faction);
                } else if let Some(action) = pressed.filter(|a| Some(*a) == self.hovered()) {
                    self.activate(action);
                } else {
                    // A click on anything else closes an open list.
                    self.dropdown_open = false;
                    self.color_open = false;
                }
            }
            Event::KeyDown(key) if self.name_focused => self.edit_name(key),
            Event::KeyDown(key) => match (self.screen, key) {
                (Screen::Intro, _) => self.set_screen(Screen::MainMenu),
                (Screen::Movie(faction), _) => self.finish_movie(faction),
                (Screen::InGame, Key::Pause) if self.pausable => {
                    self.key_paused = !self.key_paused;
                    self.selection_anchor = None;
                }
                (Screen::InGame, Key::Tab) => self.stats_open = !self.stats_open,
                (_, Key::Escape) => self.go_back(),
                _ => {}
            },
        }
    }

    fn activate(&mut self, action: Action) {
        match action {
            Action::SinglePlayer => self.set_screen(Screen::SinglePlayerMenu),
            Action::Options => self.set_screen(Screen::Options),
            Action::Back => self.go_back(),
            Action::Exit => self.quit = true,
            Action::Tab(Tab::Credits) => self.set_screen(Screen::Credits),
            Action::Tab(tab) => self.tab = tab,
            Action::Toggle(toggle) => self.flip(toggle),
            Action::Tutorial => self.start_battle(0, None, false),
            Action::ExitGame => self.leave_battle(),
            Action::ReturnToGame => self.set_screen(Screen::InGame),
            Action::StartCampaign => self.set_screen(Screen::FactionSelect),
            Action::Faction(faction) => self.set_screen(Screen::Movie(faction)),
            Action::CancelFaction => self.confirming = None,
            Action::ConfirmFaction => self.start_battle(0, self.confirming, false),
            Action::SkirmishBattle => self.set_screen(Screen::SkirmishSetup),
            Action::StartBattle => self.start_battle(self.skirmish.map, None, true),
            Action::NameField => self.name_focused = true,
            Action::MapDropdown => self.dropdown_open = true,
            Action::SelectMap(map) => {
                self.skirmish.map = map;
                self.dropdown_open = false;
            }
            Action::SetDifficulty(difficulty) => self.skirmish.difficulty = difficulty,
            Action::CycleFaction => self.skirmish.faction = self.skirmish.faction.next(),
            Action::CycleAlly(slot) => cycle_ally(&mut self.skirmish.allies, slot),
            Action::ColorBar => self.color_open = true,
            Action::PickColor(color) => {
                self.skirmish.color = color;
                self.color_open = false;
            }
            Action::CycleOpponentKind(slot) => {
                let opponent = &mut self.skirmish.opponents[slot];
                opponent.kind = opponent.kind.next();
            }
            Action::CycleOpponentFaction(slot) => {
                let opponent = &mut self.skirmish.opponents[slot];
                opponent.faction = opponent.faction.next();
            }
            Action::CycleOpponentAlly(slot, ally) => {
                cycle_ally(&mut self.skirmish.opponents[slot].allies, ally);
            }
            // Not designed yet: these buttons respond to hover but do nothing.
            Action::BuildTab(tab) => self.build_tab = tab,
            Action::Multiplayer
            | Action::LoadGame
            | Action::SaveGame
            | Action::HudSmall(_)
            | Action::HudWide(_)
            | Action::BuildCell(_) => {}
        }
    }

    fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.pressed = None;
        self.dragging = None;
        self.minimap_dragging = false;
        self.selection_anchor = None;
        self.confirming = None;
        self.name_focused = false;
        self.dropdown_open = false;
        self.color_open = false;
        self.screen_elapsed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 2880.0;
    const H: f32 = 1800.0;

    fn in_menu() -> Game {
        let mut game = Game::new(W, H);
        game.update(INTRO_SECONDS);
        game
    }

    fn click(game: &mut Game, action: Action) {
        let b = *game.buttons().iter().find(|b| b.action == action).unwrap();
        game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
    }

    fn actions(game: &Game) -> Vec<Action> {
        game.buttons().iter().map(|b| b.action).collect()
    }

    #[test]
    fn intro_ends_by_timer_or_input() {
        let mut game = Game::new(W, H);
        game.update(INTRO_SECONDS - 0.1);
        assert_eq!(game.screen, Screen::Intro);
        game.update(0.2);
        assert_eq!(game.screen, Screen::MainMenu);

        let mut game = Game::new(W, H);
        game.handle(Event::KeyDown(Key::Other));
        assert_eq!(game.screen, Screen::MainMenu);

        let mut game = Game::new(W, H);
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert_eq!(game.screen, Screen::MainMenu);
    }

    #[test]
    fn skipping_the_intro_does_not_click_what_is_under_the_cursor() {
        let mut game = Game::new(W, H);
        let single = buttons(Screen::MainMenu, W, H)[0];
        game.handle(Event::MouseMove(single.x + 1.0, single.y + 1.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert_eq!(game.screen, Screen::MainMenu);
    }

    #[test]
    fn main_menu_has_the_requested_layout() {
        let game = in_menu();
        assert_eq!(
            actions(&game),
            [Action::SinglePlayer, Action::Multiplayer, Action::Exit, Action::Options]
        );
        let b = game.buttons();
        assert!((b[0].x + b[0].w / 2.0 - W / 2.0).abs() < 0.5, "centered horizontally");
        assert!(b[0].y < H / 2.0 && b[1].y + b[1].h > H / 2.0, "centered vertically");
        assert!(b[2].x < W / 2.0 && b[2].y > H / 2.0, "exit is bottom left");
        assert!(b[3].x > W / 2.0 && b[3].y > H / 2.0, "options is bottom right");
    }

    #[test]
    fn single_player_swaps_the_menu_and_back_restores_it() {
        let mut game = in_menu();
        let exit = game.buttons()[2];
        click(&mut game, Action::SinglePlayer);
        assert_eq!(game.screen, Screen::SinglePlayerMenu);
        assert_eq!(
            actions(&game),
            [
                Action::StartCampaign,
                Action::Tutorial,
                Action::SkirmishBattle,
                Action::LoadGame,
                Action::Back
            ]
        );
        let back = game.buttons()[4];
        assert_eq!((back.x, back.y), (exit.x, exit.y), "back takes exit's place");

        click(&mut game, Action::Back);
        assert_eq!(game.screen, Screen::MainMenu);
    }

    #[test]
    fn escape_goes_back_only_from_the_submenu() {
        let mut game = in_menu();
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::MainMenu);
        assert!(!game.quit);
        click(&mut game, Action::SinglePlayer);
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::MainMenu);
    }

    #[test]
    fn exit_quits() {
        let mut game = in_menu();
        click(&mut game, Action::Exit);
        assert!(game.quit);
    }

    #[test]
    fn release_outside_the_button_cancels_the_click() {
        let mut game = in_menu();
        let b = game.buttons()[0];
        game.handle(Event::MouseMove(b.x + 1.0, b.y + 1.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(0.0, 0.0));
        game.handle(Event::MouseUp);
        assert_eq!(game.screen, Screen::MainMenu);
    }

    fn in_options() -> Game {
        let mut game = in_menu();
        click(&mut game, Action::Options);
        game
    }

    fn slider(game: &Game, id: SliderId) -> Slider {
        *game.sliders().iter().find(|s| s.id == id).unwrap()
    }

    fn drag(game: &mut Game, id: SliderId, to_x: f32) {
        let s = slider(game, id);
        game.handle(Event::MouseMove(s.x_of(game.slider_value(id)), s.cy));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(to_x, s.cy));
        game.handle(Event::MouseUp);
    }

    #[test]
    fn options_opens_with_back_and_all_tabs_in_order() {
        let game = in_options();
        assert_eq!(game.screen, Screen::Options);
        assert_eq!(game.tab, Tab::Game);
        let labels: Vec<_> = game.buttons().iter().map(|b| b.label).collect();
        assert_eq!(
            labels,
            [
                "Back", "Game", "Graphics", "Sound", "Multiplayer Info", "Network", "Keyboard",
                "Sneak Peek", "Credits", "Movies"
            ]
        );
        let back = game.buttons()[0];
        assert!(back.x < W / 2.0 && back.y > H / 2.0, "back is bottom left");
    }

    #[test]
    fn panel_sits_right_of_center_with_tabs_on_its_left() {
        let game = in_options();
        let (px, _, pw, _) = options_panel(W, H);
        assert!(px + pw / 2.0 > W / 2.0, "panel center is right of screen center");
        for tab in &game.buttons()[1..] {
            assert!(tab.x + tab.w <= px, "{} is left of the panel", tab.label);
        }
    }

    #[test]
    fn each_tab_shows_the_requested_controls() {
        let names = |tab| -> Vec<_> { sliders(tab, W, H).iter().map(|s| s.label).collect() };
        assert_eq!(names(Tab::Game), ["Game Speed", "Scroll Rate", "Mouse Speed"]);
        assert_eq!(names(Tab::Graphics), ["Graphics Quality"]);
        assert_eq!(names(Tab::Sound), ["Music Volume", "Sound Volume", "Voice Volume"]);
        let boxes: Vec<_> = checkboxes(Tab::Game, W, H).iter().map(|b| b.label).collect();
        assert_eq!(boxes, ["Tool Tips", "Target Lines"]);
        assert!(checkboxes(Tab::Sound, W, H).is_empty());
        for tab in [Tab::MultiplayerInfo, Tab::Network, Tab::Keyboard, Tab::SneakPeek, Tab::Movies] {
            assert!(tab.placeholder().is_some());
            assert!(sliders(tab, W, H).is_empty());
        }
    }

    #[test]
    fn slider_ranges_match_the_request() {
        let range = |tab, i: usize| {
            let s = sliders(tab, W, H)[i];
            (s.min, s.max)
        };
        for i in 0..3 {
            assert_eq!(range(Tab::Game, i), (1, 10));
            assert_eq!(range(Tab::Sound, i), (0, 100));
        }
        assert_eq!(range(Tab::Graphics, 0), (0, 2));
        assert_eq!(QUALITY_STOPS, ["Low", "Medium", "High"]);
    }

    #[test]
    fn dragging_changes_the_value_and_clamps_past_the_ends() {
        let mut game = in_options();
        let s = slider(&game, SliderId::GameSpeed);
        drag(&mut game, SliderId::GameSpeed, s.x_of(8));
        assert_eq!(game.settings.game_speed, 8);
        drag(&mut game, SliderId::GameSpeed, -1000.0);
        assert_eq!(game.settings.game_speed, 1);
        drag(&mut game, SliderId::GameSpeed, W + 1000.0);
        assert_eq!(game.settings.game_speed, 10);
        assert_eq!(game.dragging, None);

        // After release, moving the mouse no longer changes anything.
        game.handle(Event::MouseMove(s.x_of(3), s.cy));
        assert_eq!(game.settings.game_speed, 10);
    }

    #[test]
    fn a_drag_continues_when_the_cursor_leaves_the_row() {
        let mut game = in_options();
        let s = slider(&game, SliderId::ScrollRate);
        game.handle(Event::MouseMove(s.x_of(5), s.cy));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(s.x_of(9), 0.0));
        assert_eq!(game.settings.scroll_rate, 9);
        game.handle(Event::MouseUp);
        assert_eq!(game.screen, Screen::Options);
    }

    #[test]
    fn graphics_quality_snaps_to_three_stops() {
        let mut game = in_options();
        click(&mut game, Action::Tab(Tab::Graphics));
        let s = slider(&game, SliderId::GraphicsQuality);
        drag(&mut game, SliderId::GraphicsQuality, s.track_x + s.track_w * 0.45);
        assert_eq!(game.settings.graphics_quality, 1);
        drag(&mut game, SliderId::GraphicsQuality, s.track_x);
        assert_eq!(game.settings.graphics_quality, 0);
    }

    #[test]
    fn sound_volumes_are_independent() {
        let mut game = in_options();
        click(&mut game, Action::Tab(Tab::Sound));
        let s = slider(&game, SliderId::VoiceVolume);
        drag(&mut game, SliderId::VoiceVolume, s.x_of(25));
        assert_eq!(game.settings.voice_volume, 25);
        assert_eq!(game.settings.music_volume, Settings::default().music_volume);
        assert_eq!(game.settings.sound_volume, Settings::default().sound_volume);
    }

    #[test]
    fn checkboxes_flip_on_click() {
        let mut game = in_options();
        let row = |game: &Game, t| *game.checkboxes().iter().find(|b| b.action == Action::Toggle(t)).unwrap();
        let b = row(&game, Toggle::TargetLines);
        for expected in [false, true] {
            game.handle(Event::MouseMove(b.x + 1.0, b.y + 1.0));
            game.handle(Event::MouseDown);
            game.handle(Event::MouseUp);
            assert_eq!(game.settings.target_lines, expected);
        }
        assert!(game.settings.tool_tips, "the other checkbox is untouched");
    }

    #[test]
    fn credits_tab_opens_a_screen_that_returns_to_options() {
        let mut game = in_options();
        click(&mut game, Action::Tab(Tab::Sound));
        click(&mut game, Action::Tab(Tab::Credits));
        assert_eq!(game.screen, Screen::Credits);
        click(&mut game, Action::Back);
        assert_eq!(game.screen, Screen::Options);
        assert_eq!(game.tab, Tab::Sound, "the previous tab is still selected");

        click(&mut game, Action::Tab(Tab::Credits));
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::Options);
    }

    #[test]
    fn back_and_escape_leave_options_for_the_main_menu() {
        let mut game = in_options();
        click(&mut game, Action::Back);
        assert_eq!(game.screen, Screen::MainMenu);
        click(&mut game, Action::Options);
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::MainMenu);
    }

    #[test]
    fn tutorial_loads_then_enters_the_game_and_back_returns_to_the_menu() {
        let mut game = in_menu();
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert_eq!(game.screen, Screen::Loading, "loading starts immediately");
        assert_eq!(game.loading_progress(), 0.0);
        assert!(game.buttons().is_empty());

        game.update(LOADING_SECONDS / 2.0);
        assert_eq!(game.screen, Screen::Loading);
        assert_eq!(game.loading_progress(), 0.5);

        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        assert!(game.buttons().is_empty(), "the battlefield has no Back button");
    }

    fn leave_battle(game: &mut Game) {
        game.handle(Event::KeyDown(Key::Escape));
        click(game, Action::ExitGame);
        assert_eq!(game.screen, Screen::MainMenu);
    }

    #[test]
    fn escape_opens_the_requested_menu_and_returns_to_the_game() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::EscapeMenu);
        let labels: Vec<_> = game.buttons().iter().map(|b| b.label).collect();
        assert_eq!(labels, ["Load Game", "Save Game", "Options", "Exit Game", "Return to Game"]);
        let middle = game.buttons()[2];
        assert!((middle.x + middle.w / 2.0 - W / 2.0).abs() < 0.5, "centered");
        assert!(middle.y < H / 2.0 && middle.y + middle.h > H / 2.0, "centered");

        click(&mut game, Action::ReturnToGame);
        assert_eq!(game.screen, Screen::InGame);

        game.handle(Event::KeyDown(Key::Escape));
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::InGame, "esc also returns");
    }

    #[test]
    fn the_escape_menu_pauses_a_pausable_game_only() {
        let mut game = in_battle();
        game.update(1.0);
        let played = game.battle_time();
        game.handle(Event::KeyDown(Key::Escape));
        assert!(game.is_paused());
        game.update(1.0);
        assert_eq!(game.battle_time(), played, "time stands still");
        game.handle(Event::KeyDown(Key::Escape));
        assert!(!game.is_paused());
        game.update(1.0);
        assert_eq!(game.battle_time(), played + 1.0);

        let mut game = in_battle();
        game.pausable = false;
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::EscapeMenu, "the menu still opens");
        assert!(!game.is_paused());
        game.update(1.0);
        assert_eq!(game.battle_time(), 1.0, "a network game keeps running");
    }

    #[test]
    fn the_pause_key_toggles_and_escape_does_not_unpause() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Pause));
        assert!(game.key_paused && game.is_paused());
        game.update(1.0);
        assert_eq!(game.battle_time(), 0.0);

        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::EscapeMenu);
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::InGame);
        assert!(game.key_paused && game.is_paused(), "still paused after the menu");

        game.handle(Event::KeyDown(Key::Pause));
        assert!(!game.key_paused && !game.is_paused(), "the same key unpauses");
    }

    #[test]
    fn the_pause_key_does_nothing_in_menus_or_network_games() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Escape));
        game.handle(Event::KeyDown(Key::Pause));
        assert!(!game.key_paused, "ignored in the menu");

        let mut game = in_battle();
        game.pausable = false;
        game.handle(Event::KeyDown(Key::Pause));
        assert!(!game.key_paused && !game.is_paused());

        let mut game = in_setup();
        game.handle(Event::KeyDown(Key::Pause));
        assert!(!game.key_paused);
    }

    #[test]
    fn nothing_moves_the_camera_while_key_paused() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Pause));
        let start = game.camera;
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.handle(Event::Wheel(5.0));
        game.update(0.5);
        let (x, y, size) = hud::minimap_rect(W, H);
        click_at(&mut game, x + size - 1.0, y + size / 2.0);
        game.update(0.5);
        assert_eq!(game.camera, start);
    }

    #[test]
    fn options_over_the_battle_return_to_the_escape_menu() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Escape));
        click(&mut game, Action::Options);
        assert_eq!(game.screen, Screen::Options);
        assert!(game.in_battle && game.is_paused());
        assert!(!game.sliders().is_empty(), "the same options as the separate screen");

        click(&mut game, Action::Tab(Tab::Credits));
        assert_eq!(game.screen, Screen::Credits);
        click(&mut game, Action::Back);
        assert_eq!(game.screen, Screen::Options);

        let s = slider(&game, SliderId::ScrollRate);
        drag(&mut game, SliderId::ScrollRate, s.x_of(9));
        assert_eq!(game.settings.scroll_rate, 9);

        click(&mut game, Action::Back);
        assert_eq!(game.screen, Screen::EscapeMenu);
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::InGame);
    }

    #[test]
    fn exit_game_leaves_for_the_main_menu_and_options_go_back_there_again() {
        let mut game = in_battle();
        leave_battle(&mut game);
        assert!(!game.in_battle && !game.is_paused());
        click(&mut game, Action::Options);
        click(&mut game, Action::Back);
        assert_eq!(game.screen, Screen::MainMenu);
    }

    #[test]
    fn a_new_battle_is_not_paused_and_starts_its_clock_again() {
        let mut game = in_battle();
        game.update(3.0);
        game.handle(Event::KeyDown(Key::Pause));
        leave_battle(&mut game);
        assert!(!game.key_paused);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        game.update(LOADING_SECONDS);
        assert!(!game.is_paused());
        assert_eq!(game.battle_time(), 0.0);
    }

    #[test]
    fn tab_toggles_the_diplomacy_window_on_the_battlefield_only() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Tab));
        assert!(game.stats_open);
        assert!(!game.is_paused(), "the game keeps running behind it");
        game.handle(Event::KeyDown(Key::Tab));
        assert!(!game.stats_open);

        game.handle(Event::KeyDown(Key::Tab));
        game.handle(Event::KeyDown(Key::Escape));
        assert!(!game.stats_open, "esc closes it first");
        assert_eq!(game.screen, Screen::InGame);

        game.handle(Event::KeyDown(Key::Escape));
        game.handle(Event::KeyDown(Key::Tab));
        assert!(!game.stats_open, "not from the menu");

        let mut game = in_setup();
        game.handle(Event::KeyDown(Key::Tab));
        assert!(!game.stats_open);
    }

    #[test]
    fn diplomacy_lists_the_player_and_each_opponent_of_a_skirmish() {
        let mut game = in_setup();
        let ai = slider(&game, SliderId::AiPlayers);
        drag(&mut game, SliderId::AiPlayers, ai.x_of(2));
        click_widget(&mut game, Action::CycleAlly(0));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        let rows = game.stats_rows();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].name, "Commander");
        assert_eq!(rows[0].allies, [Ally::T, Ally::None]);
        assert_eq!(rows[0].color, game.skirmish.color);
        assert_eq!(rows[1].name, "AI 1 (Competent)");
        assert_eq!(rows[2].color, game.skirmish.opponents[1].color);
    }

    #[test]
    fn diplomacy_outside_a_skirmish_lists_only_the_player() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::O);
        click(&mut game, Action::ConfirmFaction);
        game.update(LOADING_SECONDS);
        let rows = game.stats_rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].faction, Some(Faction::O));
        assert_eq!(rows[0].color, TeamColor::Cyan);
    }

    #[test]
    fn the_diplomacy_window_fits_every_player() {
        use crate::font::text_width;
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            let (x, y, ww, wh) = stats_rect(w, h);
            assert!(x >= 0.0 && y >= 0.0 && x + ww <= w && y + wh <= h, "at {w}x{h}");
            let s = ui_scale(h);
            assert!(text_width("AI 7 (Defensive)") * small_text_px(h) < 380.0 * s);
            assert!(STATS_ROW_TOP + 8.0 * STATS_ROW_STEP <= 760.0, "eight rows fit");
        }
    }

    fn click_hud(game: &mut Game, action: Action) {
        let b = hud::all_buttons(W, H).into_iter().find(|b| b.action == action).unwrap();
        click_at(game, b.x + b.w / 2.0, b.y + b.h / 2.0);
    }

    #[test]
    fn build_tabs_switch_and_a_new_battle_starts_on_infantry() {
        let mut game = in_battle();
        assert_eq!(game.build_tab, BuildTab::Infantry);
        for tab in [BuildTab::Vehicles, BuildTab::Buildings, BuildTab::Infantry, BuildTab::Buildings] {
            click_hud(&mut game, Action::BuildTab(tab));
            assert_eq!(game.build_tab, tab);
        }
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert_eq!(game.build_tab, BuildTab::Infantry);
    }

    #[test]
    fn fake_sidebar_buttons_light_up_and_change_nothing() {
        let mut game = in_battle();
        for b in hud::command_buttons(W, H).into_iter().chain(hud::grid_cells(W, H)) {
            game.handle(Event::MouseMove(b.x + 1.0, b.y + 1.0));
            assert_eq!(game.hovered(), Some(b.action));
            game.handle(Event::MouseDown);
            game.handle(Event::MouseUp);
            assert_eq!(game.screen, Screen::InGame);
        }
        assert_eq!(game.build_tab, BuildTab::Infantry);
    }

    #[test]
    fn the_sidebar_is_inactive_behind_the_menu_and_while_key_paused() {
        let tab = hud::build_tabs(W, H)[1];
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Pause));
        click_at(&mut game, tab.x + 1.0, tab.y + 1.0);
        assert_eq!(game.build_tab, BuildTab::Infantry);

        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Escape));
        click_at(&mut game, tab.x + 1.0, tab.y + 1.0);
        assert_eq!(game.build_tab, BuildTab::Infantry);
    }

    #[test]
    fn the_sidebar_and_minimap_keep_their_clicks_apart() {
        let mut game = in_battle();
        let start = game.camera;
        click_hud(&mut game, Action::HudSmall(2));
        click_hud(&mut game, Action::BuildCell(14));
        assert_eq!(game.camera, start);
    }

    #[test]
    fn skirmish_players_all_start_with_the_chosen_money() {
        let mut game = in_setup();
        let money = slider(&game, SliderId::Money);
        drag(&mut game, SliderId::Money, money.x_of(7500));
        let ai = slider(&game, SliderId::AiPlayers);
        drag(&mut game, SliderId::AiPlayers, ai.x_of(4));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        assert_eq!(game.player_money(), 7500);
        let rows = game.stats_rows();
        assert_eq!(rows.len(), 5);
        for row in rows {
            assert_eq!(row.money, 7500, "{}", row.name);
        }
    }

    #[test]
    fn tutorial_money_does_not_come_from_the_skirmish_setup() {
        let mut game = in_setup();
        let money = slider(&game, SliderId::Money);
        drag(&mut game, SliderId::Money, money.x_of(500));
        click(&mut game, Action::Back);
        click(&mut game, Action::Tutorial);
        assert_eq!(game.player_money(), DEFAULT_BATTLE_MONEY);
        assert_eq!(game.stats_rows()[0].money, DEFAULT_BATTLE_MONEY);
    }

    #[test]
    fn chat_posts_during_battle_and_keeps_only_the_last_few_lines() {
        let mut game = in_battle();
        assert!(game.chat.is_empty());
        for _ in 0..10 {
            game.update(1.0);
        }
        let posted = game.chat.len();
        assert!(posted >= 2, "only {posted} after 10 seconds");
        for _ in 0..200 {
            game.update(1.0);
            assert!(game.chat.len() <= hud::CHAT_LINES);
        }
        assert_eq!(game.chat.len(), hud::CHAT_LINES);
    }

    #[test]
    fn chat_varies_and_uses_every_message_given_time() {
        let mut game = in_battle();
        let mut seen: Vec<&str> = Vec::new();
        for _ in 0..3000 {
            game.update(1.0);
            for line in &game.chat {
                if !seen.contains(line) {
                    seen.push(line);
                }
            }
        }
        assert_eq!(seen.len(), hud::CHAT_MESSAGES.len());
    }

    #[test]
    fn chat_stops_while_paused_and_clears_for_a_new_battle() {
        let mut game = in_battle();
        for _ in 0..20 {
            game.update(1.0);
        }
        let before = game.chat.clone();
        game.handle(Event::KeyDown(Key::Pause));
        for _ in 0..20 {
            game.update(1.0);
        }
        assert_eq!(game.chat, before);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert!(game.chat.is_empty());
    }

    fn selected_count(game: &Game) -> usize {
        game.units.iter().filter(|u| u.selected).count()
    }

    fn drag_select(game: &mut Game, from: (f32, f32), to: (f32, f32)) {
        game.handle(Event::MouseMove(from.0, from.1));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(to.0, to.1));
        game.handle(Event::MouseUp);
    }

    /// A box around the player's first group, which starts at the middle of the screen.
    fn around_first_group() -> ((f32, f32), (f32, f32)) {
        ((W / 2.0 - 400.0, H / 2.0 - 400.0), (W / 2.0 + 400.0, H / 2.0 + 400.0))
    }

    #[test]
    fn a_box_selects_the_players_units_inside_it() {
        let mut game = in_battle();
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);
        assert_eq!(selected_count(&game), 12, "the first group, not the far one");
        assert!(game.selection_rect().is_none(), "the box ends with the release");
    }

    #[test]
    fn a_box_works_dragged_in_any_direction() {
        let (a, b) = around_first_group();
        for (from, to) in [(a, b), (b, a), ((a.0, b.1), (b.0, a.1)), ((b.0, a.1), (a.0, b.1))] {
            let mut game = in_battle();
            drag_select(&mut game, from, to);
            assert_eq!(selected_count(&game), 12);
        }
    }

    #[test]
    fn opponents_units_are_never_selected() {
        let mut game = in_battle();
        let first_enemy = game.units.iter().position(|u| u.owner != 0).unwrap();
        game.units[first_enemy].x = game.camera.x;
        game.units[first_enemy].z = game.camera.z;
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);
        assert!(!game.units[first_enemy].selected);
        assert_eq!(selected_count(&game), 12);
    }

    #[test]
    fn a_new_box_replaces_the_selection_and_a_small_one_picks_or_clears() {
        let mut game = in_battle();
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);

        let view = game.view();
        let unit = game.units[5];
        let (x, y) = view.project(unit.x, unit.y + units::HEIGHT / 2.0, unit.z).unwrap();
        click_at(&mut game, x, y);
        assert_eq!(selected_count(&game), 1, "a click picks one");
        assert!(game.units[5].selected);

        click_at(&mut game, 300.0, H - 300.0);
        assert_eq!(selected_count(&game), 0, "a click on empty ground clears");
    }

    #[test]
    fn the_start_of_a_box_stays_on_the_map_while_the_camera_moves() {
        let mut game = in_battle();
        let start = (W / 2.0, H / 2.0);
        game.handle(Event::MouseMove(start.0, start.1));
        game.handle(Event::MouseDown);
        let anchored = game.selection_anchor.unwrap();
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));

        game.update(0.2);
        assert!(game.camera.x > Camera::centered().x, "the edge still scrolls during a drag");
        assert_eq!(game.selection_anchor, Some(anchored), "fixed on the map");
        let (left, _, right, _) = game.selection_rect().unwrap();
        assert!(left < start.0 - 10.0, "on screen it slid away from where it began: {left}");
        assert_eq!(right, W - 1.0, "the other corner stays on the cursor");
    }

    #[test]
    fn a_box_scrolled_along_the_edge_selects_across_more_than_one_screen() {
        let mut game = in_battle();
        let (from, _) = around_first_group();
        game.handle(Event::MouseMove(from.0, from.1));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(W - 1.0, H / 2.0 + 400.0));
        let far_x = game.units.iter().filter(|u| u.owner == 0).map(|u| u.x).fold(f32::MIN, f32::max);
        for _ in 0..40 {
            game.update(0.05);
            let (sx, _) = game.view().project(far_x, 0.0, game.camera.z).unwrap();
            if sx < W - 100.0 {
                break;
            }
        }
        game.handle(Event::MouseUp);
        let players = game.units.iter().filter(|u| u.owner == 0).count();
        assert!(players > 12, "there is a second group");
        assert_eq!(selected_count(&game), players, "both groups, a screen apart");
    }

    #[test]
    fn a_box_can_scroll_upward_past_its_start_at_the_bottom_position() {
        let mut game = in_battle();
        game.handle(Event::Wheel(100.0));
        settle_zoom(&mut game);
        game.handle(Event::MouseMove(W / 2.0, H - 100.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(W / 2.0 + 200.0, 0.0));
        for _ in 0..200 {
            game.update(0.05);
        }
        let rect = game.selection_rect().expect("the drag survives the anchor leaving the screen");
        assert!(rect.3 > H, "the box reaches past the bottom of the screen: {rect:?}");
        game.handle(Event::MouseUp);
        assert!(game.selection_rect().is_none());
    }

    #[test]
    fn a_selection_cannot_start_on_the_sidebar_minimap_or_while_paused() {
        let mut game = in_battle();
        let cell = hud::grid_cells(W, H)[0];
        game.handle(Event::MouseMove(cell.x + 1.0, cell.y + 1.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_rect().is_none(), "button");
        game.handle(Event::MouseUp);

        // The gap between two sidebar blocks has no button, but it is not battlefield.
        game.handle(Event::MouseMove(cell.x + 1.0, cell.y - 2.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_rect().is_none(), "gap");
        game.handle(Event::MouseUp);

        let (x, y, size) = hud::minimap_rect(W, H);
        game.handle(Event::MouseMove(x + size / 2.0, y + size / 2.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_rect().is_none(), "minimap");
        game.handle(Event::MouseUp);

        game.handle(Event::KeyDown(Key::Pause));
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_rect().is_none(), "paused");
        game.handle(Event::MouseUp);
        game.handle(Event::KeyDown(Key::Pause));

        game.handle(Event::KeyDown(Key::Tab));
        game.handle(Event::MouseDown);
        assert!(game.selection_rect().is_none(), "diplomacy window open");
    }

    #[test]
    fn interrupting_a_drag_cancels_it_and_keeps_the_old_selection() {
        let (from, to) = around_first_group();
        let begin = |game: &mut Game| {
            game.handle(Event::MouseMove(100.0, H - 200.0));
            game.handle(Event::MouseDown);
            assert!(game.selection_rect().is_some());
        };

        let mut game = in_battle();
        drag_select(&mut game, from, to);
        begin(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        assert!(game.selection_rect().is_none(), "the menu cancels");
        assert_eq!(selected_count(&game), 12);

        let mut game = in_battle();
        drag_select(&mut game, from, to);
        begin(&mut game);
        game.handle(Event::Focus(false));
        assert!(game.selection_rect().is_none(), "losing focus cancels");
        game.handle(Event::Focus(true));
        game.handle(Event::MouseUp);
        assert_eq!(selected_count(&game), 12);

        let mut game = in_battle();
        drag_select(&mut game, from, to);
        begin(&mut game);
        game.handle(Event::KeyDown(Key::Pause));
        assert!(game.selection_rect().is_none(), "pausing cancels");
    }

    #[test]
    fn dragging_a_box_does_not_light_up_or_press_sidebar_buttons() {
        let mut game = in_battle();
        let tab = hud::build_tabs(W, H)[1];
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(tab.x + 1.0, tab.y + 1.0));
        assert_eq!(game.hovered(), None);
        game.handle(Event::MouseUp);
        assert_eq!(game.build_tab, BuildTab::Infantry);
    }

    #[test]
    fn the_drag_previews_who_it_would_select() {
        let mut game = in_battle();
        let (from, to) = around_first_group();
        game.handle(Event::MouseMove(from.0, from.1));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(to.0, to.1));
        let previewed = game.units.iter().filter(|u| game.in_selection_drag(u)).count();
        assert_eq!(previewed, 12);
        assert_eq!(selected_count(&game), 0, "not selected until release");
    }

    #[test]
    fn every_player_gets_units_when_a_battle_starts() {
        let mut game = in_setup();
        let ai = slider(&game, SliderId::AiPlayers);
        drag(&mut game, SliderId::AiPlayers, ai.x_of(3));
        click_widget(&mut game, Action::StartBattle);
        for owner in 0..=3 {
            assert!(game.units.iter().any(|u| u.owner == owner), "owner {owner}");
        }
        assert!(game.units.iter().all(|u| u.owner <= 3));
        assert!(game.units.iter().all(|u| !u.selected));
    }

    #[test]
    fn input_cannot_interrupt_loading() {
        let mut game = in_menu();
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        game.handle(Event::KeyDown(Key::Escape));
        game.handle(Event::KeyDown(Key::Other));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert_eq!(game.screen, Screen::Loading);
    }

    #[test]
    fn replaying_the_tutorial_restarts_the_loading_timer() {
        let mut game = in_menu();
        for _ in 0..2 {
            click(&mut game, Action::SinglePlayer);
            click(&mut game, Action::Tutorial);
            assert_eq!(game.loading_progress(), 0.0);
            game.update(LOADING_SECONDS);
            assert_eq!(game.screen, Screen::InGame);
            leave_battle(&mut game);
        }
    }

    fn in_faction_select() -> Game {
        let mut game = in_menu();
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::StartCampaign);
        game
    }

    /// Picks a faction and lets its movie play out, leaving the message box up.
    fn pick(game: &mut Game, faction: Faction) {
        click(game, Action::Faction(faction));
        game.update(MOVIE_SECONDS);
    }

    #[test]
    fn factions_form_a_triangle_of_circles() {
        let game = in_faction_select();
        assert_eq!(game.screen, Screen::FactionSelect);
        let find = |f| *game.buttons().iter().find(|b| b.action == Action::Faction(f)).unwrap();
        let (a, b, c) = (find(Faction::A), find(Faction::H), find(Faction::O));
        assert!(a.y < b.y && b.y == c.y, "A above, B and C level below it");
        assert!(b.x < a.x && a.x < c.x, "A between B and C");
        assert!((a.x - b.x - (c.x - a.x)).abs() < 0.5, "symmetric");
        for button in [a, b, c] {
            assert!(button.is_round());
            assert_eq!(button.w, button.h);
        }
    }

    #[test]
    fn round_buttons_ignore_clicks_in_their_corners() {
        let mut game = in_faction_select();
        let a = game.buttons().into_iter().find(|b| b.action == Action::Faction(Faction::A)).unwrap();
        game.handle(Event::MouseMove(a.x + 2.0, a.y + 2.0));
        assert_eq!(game.hovered(), None);
        game.handle(Event::MouseMove(a.x + a.w / 2.0, a.y + 2.0));
        assert_eq!(game.hovered(), Some(Action::Faction(Faction::A)));
    }

    #[test]
    fn picking_plays_that_factions_movie_then_asks() {
        for faction in [Faction::A, Faction::H, Faction::O] {
            let mut game = in_faction_select();
            click(&mut game, Action::Faction(faction));
            assert_eq!(game.screen, Screen::Movie(faction));
            assert_eq!(game.confirming, None);
            game.update(MOVIE_SECONDS);
            assert_eq!(game.screen, Screen::FactionSelect, "returns to the same screen");
            assert_eq!(game.confirming, Some(faction));
        }
    }

    #[test]
    fn movie_can_be_skipped_by_click_or_key() {
        let mut game = in_faction_select();
        click(&mut game, Action::Faction(Faction::H));
        game.handle(Event::KeyDown(Key::Other));
        assert_eq!(game.confirming, Some(Faction::H));

        let mut game = in_faction_select();
        click(&mut game, Action::Faction(Faction::H));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert_eq!(game.confirming, Some(Faction::H));
    }

    #[test]
    fn cancel_lets_the_player_pick_again() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::A);
        click(&mut game, Action::CancelFaction);
        assert_eq!(game.screen, Screen::FactionSelect);
        assert_eq!(game.confirming, None);

        pick(&mut game, Faction::O);
        assert_eq!(game.confirming, Some(Faction::O));

        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.confirming, None, "escape cancels the message box");
        assert_eq!(game.screen, Screen::FactionSelect);
    }

    #[test]
    fn ok_loads_the_fake_game_whose_only_button_is_back() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::O);
        click(&mut game, Action::ConfirmFaction);
        assert_eq!(game.screen, Screen::Loading);
        assert_eq!(game.loading_progress(), 0.0);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        assert_eq!(game.faction, Some(Faction::O));
        assert!(game.buttons().is_empty());
    }

    #[test]
    fn message_box_blocks_the_screen_behind_it() {
        let mut game = in_faction_select();
        let behind = buttons(Screen::FactionSelect, W, H);
        pick(&mut game, Faction::A);
        for b in behind {
            game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
            game.handle(Event::MouseDown);
            game.handle(Event::MouseUp);
            assert_eq!(game.screen, Screen::FactionSelect, "{} is blocked", b.label);
            assert_eq!(game.confirming, Some(Faction::A));
        }
    }

    #[test]
    fn tutorial_after_a_campaign_has_no_faction() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::H);
        click(&mut game, Action::ConfirmFaction);
        game.update(LOADING_SECONDS);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert_eq!(game.faction, None);
    }

    #[test]
    fn faction_screen_and_message_box_fit() {
        use crate::font::text_width;
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            let (mx, my, mw, mh) = modal_rect(w, h);
            assert!(text_width(CONFIRM_MESSAGE) * small_text_px(h) < mw);
            for b in modal_buttons(w, h) {
                assert!(b.x >= mx && b.x + b.w <= mx + mw && b.y >= my && b.y + b.h <= my + mh);
            }
            let (a, b) = (modal_buttons(w, h)[0], modal_buttons(w, h)[1]);
            assert!(a.x + a.w <= b.x);
            for f in faction_buttons(w, h) {
                let px = (4.0 * ui_scale(h)).round().max(1.0);
                assert!(text_width(f.label) * px < f.w, "{} fits at {h}", f.label);
            }
        }
    }

    fn in_setup() -> Game {
        let mut game = in_menu();
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::SkirmishBattle);
        game
    }

    /// Clicks a setup widget or checkbox row, which `game.buttons()` does not list.
    fn click_widget(game: &mut Game, action: Action) {
        let (w, h) = (game.width, game.height);
        let b = setup_widgets(w, h)
            .into_iter()
            .chain(setup_checkboxes(w, h))
            .chain(opponent_widgets(game.skirmish.ai_players as usize, w, h))
            .find(|b| b.action == action)
            .unwrap();
        game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
    }

    fn type_key(game: &mut Game, key: Key) {
        game.handle(Event::KeyDown(key));
    }

    #[test]
    fn skirmish_opens_setup_with_the_requested_defaults() {
        let game = in_setup();
        assert_eq!(game.screen, Screen::SkirmishSetup);
        assert_eq!(actions(&game)[0], Action::Back);
        let back = game.buttons()[0];
        assert!(back.x < W / 2.0 && back.y > H / 2.0, "back is bottom left");
        let s = &game.skirmish;
        assert!(s.superweapons && s.living_world && s.crates);
        assert!(!s.prebuilt_base && !s.fog_of_war, "these two start unchecked");
        assert_eq!(s.allies, [Ally::None, Ally::None]);
    }

    #[test]
    fn setup_columns_sit_in_the_outer_thirds() {
        for (w, h) in [(1280.0, 720.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            for b in setup_widgets(w, h) {
                let right = matches!(
                    b.action,
                    Action::MapDropdown | Action::SetDifficulty(_)
                );
                if right {
                    assert!(b.x >= w * 2.0 / 3.0 - 0.5 && b.x + b.w <= w + 0.5, "{}", b.label);
                } else {
                    assert!(b.x >= 0.0 && b.x + b.w <= w / 3.0 + 0.5, "{}", b.label);
                }
            }
            let (mx, _, size) = minimap_rect(w, h);
            assert!(mx >= w * 2.0 / 3.0 && mx + size <= w, "minimap at {w}x{h}");
        }
    }

    #[test]
    fn setup_slider_ranges_match_the_request() {
        let range: Vec<_> = setup_sliders(W, H).iter().map(|s| (s.label, s.min, s.max)).collect();
        assert_eq!(range, [("AI Players", 1, 7), ("Money", 0, 30000), ("Units", 0, 30)]);
    }

    #[test]
    fn setup_sliders_drag_and_money_snaps_to_500() {
        let mut game = in_setup();
        let money = slider(&game, SliderId::Money);
        drag(&mut game, SliderId::Money, money.track_x + money.track_w * 0.5);
        assert_eq!(game.skirmish.money, 15000);
        drag(&mut game, SliderId::Money, money.x_of(12340));
        assert_eq!(game.skirmish.money % 500, 0);
        drag(&mut game, SliderId::Money, W + 1000.0);
        assert_eq!(game.skirmish.money, 30000);
        drag(&mut game, SliderId::AiPlayers, -1000.0);
        assert_eq!(game.skirmish.ai_players, 1);
        drag(&mut game, SliderId::Units, W + 1000.0);
        assert_eq!(game.skirmish.units, 30);
    }

    #[test]
    fn map_dropdown_opens_selects_and_closes() {
        let mut game = in_setup();
        click_widget(&mut game, Action::MapDropdown);
        assert!(game.dropdown_open);
        click(&mut game, Action::SelectMap(3));
        assert_eq!(game.skirmish.map, 3);
        assert!(!game.dropdown_open);
    }

    #[test]
    fn the_whole_map_list_fits_the_screen_and_every_map_can_be_played() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            let items = map_items(w, h);
            assert_eq!(items.len(), MAPS.len());
            let last = items.last().unwrap();
            assert!(last.y + last.h <= h, "at {w}x{h}");
            for item in &items {
                let width = crate::font::text_width(item.label) * small_text_px(h);
                assert!(width + 16.0 * ui_scale(h) < item.w, "{} at {w}x{h}", item.label);
            }
        }
        for map in 0..MAPS.len() {
            let mut game = in_setup();
            click_widget(&mut game, Action::MapDropdown);
            click(&mut game, Action::SelectMap(map));
            click_widget(&mut game, Action::StartBattle);
            game.update(LOADING_SECONDS);
            assert_eq!(game.screen, Screen::InGame);
            assert_eq!(game.battle_map, map);
            assert!(!game.units.is_empty());
        }
    }

    #[test]
    fn open_dropdown_blocks_what_is_under_it_and_closes_on_outside_click() {
        let mut game = in_setup();
        click_widget(&mut game, Action::MapDropdown);
        // The list covers the difficulty radios, so this click picks a map instead.
        let hard = setup_widgets(W, H)
            .into_iter()
            .find(|b| b.action == Action::SetDifficulty(Difficulty::Hard))
            .unwrap();
        game.handle(Event::MouseMove(hard.x + hard.w / 2.0, hard.y + hard.h / 2.0));
        assert!(matches!(game.hovered(), Some(Action::SelectMap(_))));

        game.handle(Event::MouseMove(1.0, 1.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert!(!game.dropdown_open);
        assert_eq!(game.skirmish.difficulty, Difficulty::Normal);

        click_widget(&mut game, Action::MapDropdown);
        game.handle(Event::KeyDown(Key::Escape));
        assert!(!game.dropdown_open);
        assert_eq!(game.screen, Screen::SkirmishSetup, "escape only closed the list");
    }

    #[test]
    fn difficulty_radios_are_exclusive() {
        let mut game = in_setup();
        for difficulty in [Difficulty::Hard, Difficulty::Easy, Difficulty::Normal] {
            click_widget(&mut game, Action::SetDifficulty(difficulty));
            assert_eq!(game.skirmish.difficulty, difficulty);
        }
    }

    #[test]
    fn setup_checkboxes_flip_only_themselves() {
        let mut game = in_setup();
        click_widget(&mut game, Action::Toggle(Toggle::FogOfWar));
        click_widget(&mut game, Action::Toggle(Toggle::Crates));
        assert!(game.skirmish.fog_of_war);
        assert!(!game.skirmish.crates);
        assert!(game.skirmish.superweapons && !game.skirmish.prebuilt_base && game.skirmish.living_world);
    }

    #[test]
    fn commander_name_accepts_typing_and_stops_at_the_limit() {
        let mut game = in_setup();
        type_key(&mut game, Key::Char('x'));
        assert_eq!(game.skirmish.commander, "Commander", "ignored until the field is clicked");

        click_widget(&mut game, Action::NameField);
        assert!(game.name_focused);
        for _ in 0.."Commander".len() {
            type_key(&mut game, Key::Backspace);
        }
        type_key(&mut game, Key::Backspace);
        assert_eq!(game.skirmish.commander, "");
        for c in "Lana".chars() {
            type_key(&mut game, Key::Char(c));
        }
        assert_eq!(game.skirmish.commander, "Lana");
        for _ in 0..40 {
            type_key(&mut game, Key::Char('z'));
        }
        assert_eq!(game.skirmish.commander.len(), COMMANDER_NAME_MAX);
    }

    #[test]
    fn name_focus_ends_on_enter_escape_or_click_elsewhere() {
        let mut game = in_setup();
        click_widget(&mut game, Action::NameField);
        type_key(&mut game, Key::Enter);
        assert!(!game.name_focused);

        click_widget(&mut game, Action::NameField);
        type_key(&mut game, Key::Escape);
        assert!(!game.name_focused);
        assert_eq!(game.screen, Screen::SkirmishSetup, "escape did not leave the screen");

        click_widget(&mut game, Action::NameField);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert!(!game.name_focused);
    }

    #[test]
    fn faction_planet_cycles_a_h_o() {
        let mut game = in_setup();
        assert_eq!(game.skirmish.faction, Faction::A);
        for expected in [Faction::H, Faction::O, Faction::A] {
            click_widget(&mut game, Action::CycleFaction);
            assert_eq!(game.skirmish.faction, expected);
        }
    }

    #[test]
    fn a_side_cannot_befriend_the_same_mini_faction_twice() {
        let mut game = in_setup();
        // Every combination reachable by clicking either slot stays free of duplicates.
        for click_slot in [0, 1, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 0] {
            click_widget(&mut game, Action::CycleAlly(click_slot));
            let [left, right] = game.skirmish.allies;
            assert!(left == Ally::None || left != right, "{left:?} twice");
        }

        let mut allies = [Ally::T, Ally::None];
        cycle_ally(&mut allies, 1);
        assert_eq!(allies, [Ally::T, Ally::X], "T is skipped");

        let mut allies = [Ally::S, Ally::F];
        cycle_ally(&mut allies, 1);
        assert_eq!(allies, [Ally::S, Ally::None], "S is skipped, none is still allowed");

        let mut allies = [Ally::None, Ally::None];
        cycle_ally(&mut allies, 0);
        cycle_ally(&mut allies, 0);
        cycle_ally(&mut allies, 1);
        assert_eq!(allies, [Ally::X, Ally::T], "both slots can be filled");
    }

    #[test]
    fn opponents_cannot_befriend_the_same_mini_faction_twice() {
        let mut game = in_setup();
        click_widget(&mut game, Action::CycleOpponentAlly(0, 0));
        click_widget(&mut game, Action::CycleOpponentAlly(0, 1));
        assert_eq!(game.skirmish.opponents[0].allies, [Ally::T, Ally::X]);
    }

    #[test]
    fn each_ally_cycles_through_five_shapes_and_none_on_its_own() {
        let mut game = in_setup();
        let mut seen = Vec::new();
        for _ in 0..6 {
            click_widget(&mut game, Action::CycleAlly(1));
            seen.push(game.skirmish.allies[1]);
        }
        assert_eq!(
            seen,
            [Ally::T, Ally::X, Ally::G, Ally::F, Ally::S, Ally::None]
        );
        assert_eq!(game.skirmish.allies[0], Ally::None);
        click_widget(&mut game, Action::CycleAlly(0));
        assert_eq!(game.skirmish.allies, [Ally::T, Ally::None]);
    }

    fn set_ai_players(game: &mut Game, count: i32) {
        let s = slider(game, SliderId::AiPlayers);
        drag(game, SliderId::AiPlayers, s.x_of(count));
        assert_eq!(game.skirmish.ai_players, count);
    }

    fn active_colors(game: &Game) -> Vec<TeamColor> {
        game.skirmish.opponents[..game.skirmish.ai_players as usize]
            .iter()
            .map(|o| o.color)
            .collect()
    }

    #[test]
    fn ai_players_slider_fills_and_empties_the_opponent_slots() {
        let mut game = in_setup();
        let row_of = |game: &Game, slot| {
            let (x, y, _, h) = opponent_row(slot, game.width, game.height);
            (x + 1.0, y + h / 2.0)
        };
        for count in [1, 7, 2] {
            set_ai_players(&mut game, count);
            for slot in 0..OPPONENT_SLOTS {
                let (x, y) = row_of(&game, slot);
                game.handle(Event::MouseMove(x, y));
                let expected = (slot < count as usize).then_some(Action::CycleOpponentKind(slot));
                assert_eq!(game.hovered(), expected, "slot {slot} with {count} players");
            }
        }
    }

    #[test]
    fn every_player_holds_a_different_color_at_any_count() {
        let mut game = in_setup();
        for count in [1, 7, 3, 5, 7] {
            set_ai_players(&mut game, count);
            let mut colors = active_colors(&game);
            colors.push(game.skirmish.color);
            let total = colors.len();
            colors.dedup_by(|a, b| a == b);
            for (i, a) in colors.iter().enumerate() {
                assert!(!colors[i + 1..].contains(a), "{a:?} is held twice at {count}");
            }
            assert_eq!(colors.len(), total);
        }
    }

    #[test]
    fn colors_held_by_opponents_are_not_offered_to_the_player() {
        let mut game = in_setup();
        for count in 1..=7 {
            set_ai_players(&mut game, count);
            let offered = game.skirmish.available_colors();
            assert_eq!(offered.len(), 8 - count as usize);
            assert!(offered.contains(&game.skirmish.color), "own color stays");
            for held in active_colors(&game) {
                assert!(!offered.contains(&held));
            }
        }
    }

    #[test]
    fn color_bar_opens_into_slots_and_closes_on_pick() {
        let mut game = in_setup();
        set_ai_players(&mut game, 3);
        let free = game.skirmish.available_colors();
        let pick = *free.last().unwrap();
        click_widget(&mut game, Action::ColorBar);
        assert!(game.color_open);
        assert_eq!(game.buttons().len(), free.len());
        click(&mut game, Action::PickColor(pick));
        assert_eq!(game.skirmish.color, pick);
        assert!(!game.color_open);
    }

    #[test]
    fn open_color_bar_blocks_other_controls_and_escape_closes_it() {
        let mut game = in_setup();
        click_widget(&mut game, Action::ColorBar);
        let play = setup_widgets(W, H).into_iter().find(|b| b.action == Action::StartBattle).unwrap();
        game.handle(Event::MouseMove(play.x + 1.0, play.y + 1.0));
        assert_eq!(game.hovered(), None);
        let ai = slider(&game, SliderId::AiPlayers);
        game.handle(Event::MouseMove(ai.x_of(7), ai.cy));
        assert_eq!(game.hovered_slider(), None);
        game.handle(Event::KeyDown(Key::Escape));
        assert!(!game.color_open);
        assert_eq!(game.screen, Screen::SkirmishSetup);
    }

    #[test]
    fn picking_a_color_leaves_opponents_alone_and_new_opponents_avoid_it() {
        let mut game = in_setup();
        set_ai_players(&mut game, 3);
        let before = active_colors(&game);
        let pick = *game.skirmish.available_colors().last().unwrap();
        click_widget(&mut game, Action::ColorBar);
        click(&mut game, Action::PickColor(pick));
        assert_eq!(active_colors(&game), before);

        set_ai_players(&mut game, 7);
        assert!(!active_colors(&game).contains(&pick));
        assert_eq!(&active_colors(&game)[..3], &before[..]);
    }

    #[test]
    fn removing_an_opponent_frees_its_color() {
        let mut game = in_setup();
        set_ai_players(&mut game, 7);
        let last = active_colors(&game)[6];
        assert!(!game.skirmish.available_colors().contains(&last));
        set_ai_players(&mut game, 6);
        assert!(game.skirmish.available_colors().contains(&last));
    }

    #[test]
    fn opponent_controls_cycle_independently() {
        let mut game = in_setup();
        set_ai_players(&mut game, 2);
        let mut kinds = Vec::new();
        for _ in 0..3 {
            click_widget(&mut game, Action::CycleOpponentKind(1));
            kinds.push(game.skirmish.opponents[1].kind.label());
        }
        assert_eq!(kinds, ["Attacker", "Defensive", "Competent"]);

        let before = game.skirmish.opponents[1].faction;
        click_widget(&mut game, Action::CycleOpponentFaction(1));
        assert_eq!(game.skirmish.opponents[1].faction, before.next());
        for _ in 0..2 {
            click_widget(&mut game, Action::CycleOpponentAlly(1, 1));
        }
        assert_eq!(game.skirmish.opponents[1].allies, [Ally::None, Ally::X]);
        assert_eq!(game.skirmish.opponents[0].allies, [Ally::None, Ally::None]);
    }

    #[test]
    fn opponent_rows_fit_the_middle_third_and_above_nothing() {
        use crate::font::text_width;
        for (w, h) in [(1280.0, 720.0), (1440.0, 900.0), (1920.0, 1080.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            let px = small_text_px(h);
            for b in opponent_widgets(OPPONENT_SLOTS, w, h) {
                assert!(b.x >= w / 3.0 && b.x + b.w <= w * 2.0 / 3.0, "{} at {w}x{h}", b.label);
                assert!(b.y + b.h < buttons(Screen::SkirmishSetup, w, h)[0].y);
                if let Action::CycleOpponentKind(_) = b.action {
                    for kind in [OpponentKind::Defensive, OpponentKind::Competent, OpponentKind::Attacker] {
                        assert!(text_width(kind.label()) * px < b.w, "{} at {h}", kind.label());
                    }
                }
            }
            let all = opponent_widgets(OPPONENT_SLOTS, w, h);
            for (i, a) in all.iter().enumerate() {
                for b in &all[i + 1..] {
                    assert!(
                        a.x + a.w <= b.x + 0.01 || b.x + b.w <= a.x + 0.01 || a.y + a.h <= b.y + 0.01 || b.y + b.h <= a.y + 0.01,
                        "{:?} overlaps {:?} at {w}x{h}", a.action, b.action
                    );
                }
            }
        }
    }

    #[test]
    fn color_bar_sits_under_the_faction_and_play_just_under_it() {
        let all = setup_widgets(W, H);
        let find = |action| *all.iter().find(|b| b.action == action).unwrap();
        let (faction, bar, play) = (
            find(Action::CycleFaction),
            find(Action::ColorBar),
            find(Action::StartBattle),
        );
        assert!(faction.y + faction.h < bar.y);
        assert!(bar.y + bar.h <= play.y);
        assert!(play.y - (bar.y + bar.h) < 30.0 * ui_scale(H), "play is just under the bar");
    }

    fn in_battle() -> Game {
        let mut game = in_setup();
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game
    }

    #[test]
    fn play_uses_the_selected_map_and_a_centered_camera() {
        let mut game = in_setup();
        click_widget(&mut game, Action::MapDropdown);
        click(&mut game, Action::SelectMap(2));
        click_widget(&mut game, Action::StartBattle);
        assert_eq!(game.battle_map, 2);
        assert_eq!(game.camera, Camera::centered());
    }

    #[test]
    fn the_cursor_at_an_edge_scrolls_only_on_the_battlefield() {
        let mut game = in_battle();
        let start = game.camera;
        game.update(0.5);
        assert_eq!(game.camera, start, "cursor in the middle");

        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.update(0.5);
        assert!(game.camera.x > start.x, "right edge moves east");
        assert_eq!(game.camera.z, start.z);

        let before = game.camera;
        game.handle(Event::MouseMove(W / 2.0, 0.0));
        game.update(0.5);
        assert!(game.camera.z < before.z, "top edge moves north");

        let before = game.camera;
        leave_battle(&mut game);
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.update(0.5);
        assert_eq!(game.camera, before, "menus do not scroll");
    }

    #[test]
    fn scroll_rate_option_sets_the_speed() {
        let moved = |rate: i32| {
            let mut game = in_battle();
            game.settings.scroll_rate = rate;
            game.handle(Event::MouseMove(W - 1.0, H / 2.0));
            game.update(0.1);
            game.camera.x - Camera::centered().x
        };
        assert!((moved(10) - moved(5) * 2.0).abs() < 1e-3);
    }

    #[test]
    fn scrolling_stops_while_another_app_is_in_front() {
        let mut game = in_battle();
        let start = game.camera;
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.handle(Event::Focus(false));
        game.update(0.5);
        assert_eq!(game.camera, start);
        game.handle(Event::Focus(true));
        game.update(0.5);
        assert!(game.camera.x > start.x);
    }

    fn settle_zoom(game: &mut Game) {
        // Parked mid-screen so that edge scrolling stays out of the way.
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        for _ in 0..200 {
            game.update(0.016);
        }
    }

    #[test]
    fn the_wheel_zooms_only_on_the_battlefield() {
        let mut game = in_battle();
        game.handle(Event::Wheel(100.0));
        settle_zoom(&mut game);
        assert_eq!(game.camera.zoom, 1.0, "wheel away from the player descends");
        game.handle(Event::Wheel(-100.0));
        settle_zoom(&mut game);
        assert_eq!(game.camera.zoom, 0.0);

        let mut game = in_setup();
        game.handle(Event::Wheel(100.0));
        for _ in 0..50 {
            game.update(0.016);
        }
        assert_eq!(game.camera.zoom, 0.0, "menus ignore it");
    }

    #[test]
    fn a_new_battle_starts_at_the_top_position() {
        let mut game = in_battle();
        game.handle(Event::Wheel(5.0));
        settle_zoom(&mut game);
        assert!(game.camera.zoom > 0.0);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        game.update(LOADING_SECONDS);
        settle_zoom(&mut game);
        assert_eq!(game.camera.zoom, 0.0);
    }

    fn click_at(game: &mut Game, x: f32, y: f32) {
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
    }

    #[test]
    fn clicking_the_minimap_sends_the_camera_there() {
        let mut game = in_battle();
        let rect = hud::minimap_rect(W, H);
        let (x, y, size) = rect;
        assert!(x + size <= W && x > W / 2.0 && y < H / 2.0, "top right");

        // The east corner of the diamond.
        click_at(&mut game, x + size - 1.0, y + size / 2.0);
        let expected = battlefield::minimap_to_world(rect, x + size - 1.0, y + size / 2.0).unwrap();
        assert!((game.camera.x - expected.0).abs() < 1.0, "went east: {}", game.camera.x);
        assert!(game.camera.x > Camera::centered().x);

        click_at(&mut game, x + size / 2.0, y + size / 2.0);
        assert!((game.camera.x - Camera::centered().x).abs() < 0.01);
        assert!((game.camera.z - Camera::centered().z).abs() < 0.01, "the middle is the map's middle");
    }

    #[test]
    fn dragging_on_the_minimap_moves_the_camera_until_release() {
        let mut game = in_battle();
        let rect = hud::minimap_rect(W, H);
        let (x, y, size) = rect;
        let at = |fx: f32, fy: f32| (x + size * fx, y + size * fy);
        let world = |p: (f32, f32)| battlefield::minimap_to_world(rect, p.0, p.1).unwrap();

        let start = at(0.5, 0.3);
        game.handle(Event::MouseMove(start.0, start.1));
        game.handle(Event::MouseDown);
        assert!((game.camera.z - world(start).1).abs() < 0.01, "moves on press");

        let next = at(0.6, 0.45);
        game.handle(Event::MouseMove(next.0, next.1));
        assert!((game.camera.x - world(next).0).abs() < 0.01, "follows the drag");
        assert!((game.camera.z - world(next).1).abs() < 0.01, "follows the drag");

        game.handle(Event::MouseUp);
        let released = game.camera;
        let later = at(0.4, 0.5);
        game.handle(Event::MouseMove(later.0, later.1));
        assert_eq!(game.camera, released, "release ends it");
    }

    #[test]
    fn a_minimap_drag_can_leave_the_minimap_without_scrolling_the_screen() {
        let mut game = in_battle();
        let rect = hud::minimap_rect(W, H);
        let (x, y, size) = rect;
        game.handle(Event::MouseMove(x + size / 2.0, y + size / 2.0));
        game.handle(Event::MouseDown);

        // Off the right edge of the screen, past the minimap's edge.
        game.handle(Event::MouseMove(W - 1.0, y + size / 2.0));
        let (east_x, _) = battlefield::minimap_to_world(rect, x + size - 0.01, y + size / 2.0).unwrap();
        assert!((game.camera.x - east_x).abs() < 0.01, "holds to the minimap's edge");
        let held = game.camera;
        game.update(0.5);
        assert_eq!(game.camera, held, "edge scrolling waits for the release");

        game.handle(Event::MouseUp);
        game.update(0.5);
        assert_ne!(game.camera, held, "edge scrolling resumes");
    }

    #[test]
    fn a_press_outside_the_minimap_does_not_start_a_drag() {
        let mut game = in_battle();
        let (x, y, size) = hud::minimap_rect(W, H);
        let start = game.camera;
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(x + size / 2.0, y + size / 2.0));
        assert_eq!(game.camera, start);
    }

    #[test]
    fn opening_the_menu_ends_a_minimap_drag() {
        let mut game = in_battle();
        let (x, y, size) = hud::minimap_rect(W, H);
        game.handle(Event::MouseMove(x + size / 2.0, y + size / 2.0));
        game.handle(Event::MouseDown);
        game.handle(Event::KeyDown(Key::Escape));
        let held = game.camera;
        game.handle(Event::MouseMove(x + size * 0.7, y + size * 0.7));
        assert_eq!(game.camera, held);
    }

    #[test]
    fn a_minimap_click_outside_the_diamond_stays_on_the_map() {
        let mut game = in_battle();
        let (x, y, _) = hud::minimap_rect(W, H);
        click_at(&mut game, x + 1.0, y + 1.0);
        let before = game.camera;
        let mut landed = game.camera;
        landed.look_at(before.x, before.z);
        assert!((landed.x - before.x).abs() < 0.01 && (landed.z - before.z).abs() < 0.01, "already on the map");
    }

    #[test]
    fn clicks_elsewhere_and_on_other_screens_leave_the_camera_alone() {
        let mut game = in_battle();
        let start = game.camera;
        click_at(&mut game, W / 2.0, H / 2.0);
        assert_eq!(game.camera, start);

        let (x, y, size) = hud::minimap_rect(W, H);
        let mut game = in_setup();
        let start = game.camera;
        click_at(&mut game, x + size - 1.0, y + size / 2.0);
        assert_eq!(game.camera, start);
    }

    #[test]
    fn the_battle_minimap_does_not_touch_the_scroll_edges() {
        for (w, h) in [(1280.0, 720.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            let (x, y, size) = hud::minimap_rect(w, h);
            assert_eq!(battlefield::edge_direction((x + size - 1.0, y), w, h), (0.0, 0.0), "at {w}x{h}");
        }
    }

    #[test]
    fn a_new_battle_recenters_the_camera() {
        let mut game = in_battle();
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.update(0.5);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        game.update(LOADING_SECONDS);
        assert_eq!(game.camera, Camera::centered());
    }

    #[test]
    fn play_runs_loading_then_a_single_battle_and_keeps_the_setup() {
        let mut game = in_setup();
        click_widget(&mut game, Action::SetDifficulty(Difficulty::Hard));
        click_widget(&mut game, Action::StartBattle);
        assert_eq!(game.screen, Screen::Loading);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        assert_eq!(game.faction, None);
        assert!(game.buttons().is_empty());
        leave_battle(&mut game);
        assert_eq!(game.skirmish.difficulty, Difficulty::Hard);
    }

    #[test]
    fn setup_content_fits_and_does_not_collide() {
        use crate::font::text_width;
        for (w, h) in [(1280.0, 720.0), (1440.0, 900.0), (1920.0, 1080.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            let px = small_text_px(h);
            let widest = "W".repeat(COMMANDER_NAME_MAX);
            let all = setup_widgets(w, h);
            let name = all.iter().find(|b| b.action == Action::NameField).unwrap();
            assert!(text_width(&widest) * px < name.w, "name at {h}");
            for b in all.iter().filter(|b| matches!(b.action, Action::SetDifficulty(_))) {
                let circle = 30.0 * ui_scale(h);
                assert!(circle + text_width(b.label) * px <= b.w, "{} at {h}", b.label);
            }
            for s in setup_sliders(w, h) {
                let value = text_width("30000") * px;
                assert!(s.label_x + text_width(s.label) * px < s.value_right - value, "{} at {h}", s.label);
            }
            for b in setup_checkboxes(w, h) {
                assert!(text_width(b.label) * px < b.w - checkbox_size(h), "{} at {h}", b.label);
            }
            let mut rows: Vec<_> = all.clone();
            rows.extend(setup_checkboxes(w, h));
            rows.extend(buttons(Screen::SkirmishSetup, w, h));
            for (i, a) in rows.iter().enumerate() {
                for b in &rows[i + 1..] {
                    let apart = a.x + a.w <= b.x + 0.01
                        || b.x + b.w <= a.x + 0.01
                        || a.y + a.h <= b.y + 0.01
                        || b.y + b.h <= a.y + 0.01;
                    assert!(apart, "{} overlaps {} at {w}x{h}", a.label, b.label);
                }
            }
        }
    }

    #[test]
    fn options_content_fits_at_common_resolutions() {
        use crate::font::text_width;
        for (w, h) in [(1280.0, 720.0), (1440.0, 900.0), (1920.0, 1080.0), (2560.0, 1600.0), (2880.0, 1800.0), (1024.0, 768.0), (5120.0, 1440.0)] {
            let px = small_text_px(h);
            let (panel_x, panel_y, panel_w, panel_h) = options_panel(w, h);
            assert!(panel_x + panel_w <= w);
            for tab in TABS.iter().map(|t| t.0) {
                for s in sliders(tab, w, h) {
                    assert!(s.label_x + text_width(s.label) * px < s.track_x, "{} at {h}", s.label);
                    assert!(s.track_x + s.track_w < panel_x + panel_w);
                    assert!(s.cy < panel_y + panel_h);
                }
                for b in checkboxes(tab, w, h) {
                    assert!(text_width(b.label) * px < b.w - checkbox_size(h));
                    assert!(b.y + b.h <= panel_y + panel_h);
                }
                if let Some(text) = tab.placeholder() {
                    assert!(text_width(text) * px < panel_w);
                }
            }
            for b in tab_buttons(w, h) {
                assert!(text_width(b.label) * px < b.w, "{} at {h}", b.label);
                assert!(b.y + b.h <= panel_y + panel_h + 0.5, "tabs fit the panel height");
            }
        }
    }

    #[test]
    fn layout_stays_on_screen_across_aspect_ratios() {
        for (w, h) in [(1280.0, 720.0), (1920.0, 1080.0), (2880.0, 1800.0), (5120.0, 1440.0), (1024.0, 768.0)] {
            for screen in [Screen::MainMenu, Screen::SinglePlayerMenu, Screen::Options, Screen::Credits, Screen::EscapeMenu, Screen::FactionSelect, Screen::SkirmishSetup] {
                let all = buttons(screen, w, h);
                for b in &all {
                    assert!(b.x >= 0.0 && b.y >= 0.0 && b.x + b.w <= w && b.y + b.h <= h);
                }
                for (i, a) in all.iter().enumerate() {
                    for b in &all[i + 1..] {
                        let apart = a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
                        assert!(apart, "{} overlaps {}", a.label, b.label);
                    }
                }
            }
        }
    }
}
