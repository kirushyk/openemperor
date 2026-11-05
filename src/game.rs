//! Game flow and menu logic. No OS or GPU code, so it runs under `cargo test`.

use crate::battlefield::{self, Camera};
use crate::fog::{Fog, Sight};
use crate::hud::{self, DEPLOY_BUTTON};
use crate::platform::{Event, Key};
use crate::units::{self, Role, Unit, UnitKind};
use crate::weapons::Combat;

pub const INTRO_SECONDS: f32 = 3.0;
pub const LOADING_SECONDS: f32 = 2.0;
pub const MOVIE_SECONDS: f32 = 3.0;
/// What the player and any opponents start with outside a skirmish, which sets its own.
const DEFAULT_BATTLE_MONEY: i32 = 10000;
/// A selection smaller than this each way, at 1080 pixels tall, counts as a click.
const CLICK_SLOP: f32 = 6.0;
/// How near a click must land to a unit to pick it, at 1080 pixels tall.
const PICK_RADIUS: f32 = 25.0;
/// A placeholder reading until buildings produce and consume energy.
const STARTING_ENERGY: i32 = 65;

/// The Vehicles menu's cell holding the superweapon.
pub const SUPERWEAPON_CELL: usize = 6;
/// What each tab's first cells produce, in order.
const INFANTRY_BUILDS: [Role; 7] = [
    Role::Scout,
    Role::LightInfantry,
    Role::Sniper,
    Role::Engineer,
    Role::KInfantry,
    Role::FlameInfantry,
    Role::GasInfantry,
];
const VEHICLE_BUILDS: [Role; 6] = [
    Role::Tank,
    Role::RepairVehicle,
    Role::Catapult,
    Role::RocketLauncher,
    Role::Strider,
    Role::HeavyStrider,
];
/// What the Vehicles menu's bottom row makes, from its left end.
const BOTTOM_VEHICLE_BUILDS: [Role; 3] =
    [Role::AirDefensePlatform, Role::AirDrone, Role::Ornithopter];
const BOTTOM_ROW_START: usize = (hud::GRID_ROWS - 1) * hud::GRID_COLUMNS;
/// The Vehicles menu's cell for the Advanced Carryall, just above the bottom row.
const CARRYALL_CELL: usize = BOTTOM_ROW_START - hud::GRID_COLUMNS;
/// Seconds of battle before the superweapon can fire again.
pub const SUPERWEAPON_RECHARGE: f32 = 60.0;

/// How long a chat line stays up, in seconds of battle.
const CHAT_SECONDS: f32 = 12.0;

/// How long the Victory or Defeat message stays up, in seconds.
pub const OUTCOME_SECONDS: f32 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleKind {
    Tutorial,
    Skirmish,
    Campaign,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Victory,
    Defeat,
}

/// The cursor's looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorLook {
    Arrow,
    Attack,
    Heal,
    /// A selected carryall would pick up or put down a vehicle here.
    Carry,
}

/// One message in the battle's chat.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatLine {
    pub text: String,
    posted_at: f32,
}

/// What a click with Control held would order.
#[derive(Clone, Copy)]
enum AttackOrder {
    Unit(usize),
    Place((f32, f32)),
}

/// One side of the battle statistics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Casualties {
    pub infantry: i32,
    pub vehicles: i32,
    pub aircraft: i32,
    pub buildings: i32,
}

impl Casualties {
    fn count(&mut self, unit: &Unit) {
        match unit.kind {
            UnitKind::Infantry => self.infantry += 1,
            UnitKind::Vehicle => self.vehicles += 1,
            UnitKind::Aircraft => self.aircraft += 1,
            UnitKind::Tower | UnitKind::Building => self.buildings += 1,
        }
    }
}

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
    /// What was killed and lost, after the Victory or Defeat message.
    BattleStats,
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
        const FACTIONS: [Faction; 3] = [Faction::H, Faction::A, Faction::O];
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
    LeaveBattleStats,
    /// The sidebar's small buttons, which do nothing yet.
    HudSmall(usize),
    /// The sidebar's wide buttons: `hud::DEPLOY_BUTTON`, and one that does nothing yet.
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
    (
        Action::SkirmishBattle,
        "Single Skirmish Battle",
        Anchor::Center,
    ),
    (Action::LoadGame, "Load Game", Anchor::Center),
    (Action::Back, "Back", Anchor::BottomLeft),
];

const BACK_ONLY: &[(Action, &str, Anchor)] = &[(Action::Back, "Back", Anchor::BottomLeft)];

const BATTLE_STATS: &[(Action, &str, Anchor)] =
    &[(Action::LeaveBattleStats, "Continue", Anchor::BottomRight)];

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
const GRAPHICS_SLIDERS: &[SliderRow] = &[(
    SliderId::GraphicsQuality,
    "Graphics Quality",
    0,
    2,
    QUALITY_STOPS,
)];
const SOUND_SLIDERS: &[SliderRow] = &[
    (SliderId::MusicVolume, "Music Volume", 0, 100, &[]),
    (SliderId::SoundVolume, "Sound Volume", 0, 100, &[]),
    (SliderId::VoiceVolume, "Voice Volume", 0, 100, &[]),
];
const GAME_TOGGLES: &[(Toggle, &str)] = &[
    (Toggle::ToolTips, "Tool Tips"),
    (Toggle::TargetLines, "Target Lines"),
];

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

/// The three factions form a triangle: A on top, H and O below.
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
    let x = if left {
        24.0 * s
    } else {
        width * 2.0 / 3.0 + 24.0 * s
    };
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
        all.push(circle(
            Action::CycleOpponentFaction(slot),
            "Faction",
            faction_left,
            FACTION_R,
        ));
        all.push(circle(
            Action::CycleOpponentAlly(slot, 0),
            "Ally",
            ally_left,
            ALLY_R,
        ));
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
        Button {
            action: Action::NameField,
            label: "Commander Name",
            x: lx,
            y: 84.0 * s,
            w: lw,
            h: 56.0 * s,
        },
        circle(Action::CycleFaction, "Faction", 0.0, FACTION_R),
        circle(Action::CycleAlly(0), "Ally", -ALLY_OFFSET, ALLY_R),
        circle(Action::CycleAlly(1), "Ally", ALLY_OFFSET, ALLY_R),
        Button {
            action: Action::ColorBar,
            label: "Color",
            x: bar_x,
            y: bar_y,
            w: bar_w,
            h: bar_h,
        },
        Button {
            action: Action::StartBattle,
            label: "Play",
            x: center - 120.0 * s,
            y: bar_y + bar_h + 16.0 * s,
            w: 240.0 * s,
            h: 72.0 * s,
        },
        Button {
            action: Action::MapDropdown,
            label: "Map",
            x: rx,
            y: 352.0 * s,
            w: rw,
            h: 52.0 * s,
        },
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

/// A rectangle of this size at 1080 pixels tall, centered on the screen, as (x, y, w, h).
fn centered_rect(w: f32, h: f32, width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    let (w, h) = (w * s, h * s);
    ((width - w) / 2.0, (height - h) / 2.0, w, h)
}

/// The diplomacy window as (x, y, w, h).
pub fn stats_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    centered_rect(1000.0, 760.0, width, height)
}

pub const CONFIRM_MESSAGE: &str = "Are you sure you will play for those guys?";

/// The confirmation message box as (x, y, w, h).
pub fn modal_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    centered_rect(900.0, 320.0, width, height)
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
        Screen::BattleStats => BATTLE_STATS,
        Screen::Options | Screen::Credits | Screen::FactionSelect | Screen::SkirmishSetup => {
            BACK_ONLY
        }
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
                Anchor::BottomLeft => (MARGIN * s, (1080.0 - MARGIN - BUTTON_H) * s, CORNER_W),
                Anchor::BottomRight => (
                    width - (MARGIN + CORNER_W) * s,
                    (1080.0 - MARGIN - BUTTON_H) * s,
                    CORNER_W,
                ),
            };
            Button {
                action,
                label,
                x: px,
                y: py,
                w: w * s,
                h: BUTTON_H * s,
            }
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
    pub battle_kind: BattleKind,
    /// The team of each owner, player first. Sharing a team is how allies will work.
    pub teams: Vec<usize>,
    /// Set once the battle is decided, and kept for the statistics.
    pub outcome: Option<Outcome>,
    outcome_elapsed: f32,
    pub killed: Casualties,
    pub lost: Casualties,
    /// Counts up from 0 through a campaign.
    pub campaign_mission: usize,
    /// Pressed on the battlefield minimap and not yet released.
    minimap_dragging: bool,
    pub units: Vec<Unit>,
    /// Vehicles carryalls hold, which are out of `units` meanwhile.
    pub carried: Vec<units::Carried>,
    pub combat: Combat,
    /// The superweapon is chosen and waiting for a click on the map.
    pub aiming_strike: bool,
    /// Seconds until the superweapon can fire again.
    pub strike_recharge: f32,
    /// Where a selection drag started, as a point on the map (x, z), so that it stays put
    /// on the ground while the camera moves.
    selection_anchor: Option<(f32, f32)>,
    /// Seconds since the game started, unbroken by going from screen to screen.
    menu_seconds: f32,
    /// Whether a Control key is down, which turns clicks into attack orders.
    control_held: bool,
    /// New units get ids never used in this battle, so a missile cannot chase the wrong one.
    next_unit_id: u32,
    pub build_tab: BuildTab,
    pub energy: i32,
    /// The player's money first, then each opponent's.
    money: Vec<i32>,
    /// Newest last, at most `hud::CHAT_LINES`.
    pub chat: Vec<ChatLine>,
    /// Which owners were already reported defeated.
    reported_defeated: Vec<bool>,
    /// What the player knows of the map, when this battle has fog of war.
    pub fog: Option<Fog>,
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
            battle_kind: BattleKind::Tutorial,
            teams: vec![0],
            outcome: None,
            outcome_elapsed: 0.0,
            killed: Casualties::default(),
            lost: Casualties::default(),
            campaign_mission: 0,
            minimap_dragging: false,
            units: Vec::new(),
            carried: Vec::new(),
            combat: Combat::default(),
            aiming_strike: false,
            strike_recharge: 0.0,
            selection_anchor: None,
            menu_seconds: 0.0,
            control_held: false,
            next_unit_id: 0,
            build_tab: BuildTab::Infantry,
            energy: STARTING_ENERGY,
            money: vec![DEFAULT_BATTLE_MONEY],
            chat: Vec::new(),
            reported_defeated: Vec::new(),
            fog: None,
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

    /// Fog of war can be turned on in a skirmish's setup; elsewhere it is always there.
    fn fog_allowed(&self) -> bool {
        self.battle_kind != BattleKind::Skirmish || self.skirmish.fog_of_war
    }

    /// What is known of a place on the map. Without fog of war, everything is in view.
    pub fn sight_at(&self, x: f32, z: f32) -> Sight {
        self.fog.as_ref().map_or(Sight::Visible, |fog| fog.at(x, z))
    }

    /// Whether the player can see this unit: their own and allies' always, others only in view.
    pub fn shows(&self, unit: &Unit) -> bool {
        units::are_friends(&self.teams, 0, unit.owner)
            || self.sight_at(unit.x, unit.z) == Sight::Visible
    }

    /// Sees the map anew from where the player's and allies' units stand now.
    fn update_fog(&mut self) {
        if let Some(fog) = &mut self.fog {
            let teams = &self.teams;
            fog.update(
                self.units
                    .iter()
                    .filter(|u| u.alive && units::are_friends(teams, 0, u.owner)),
            );
        }
    }

    /// The superweapon can be turned off in a skirmish's setup; elsewhere it is always there.
    pub fn superweapon_allowed(&self) -> bool {
        self.battle_kind != BattleKind::Skirmish || self.skirmish.superweapons
    }

    fn view(&self) -> battlefield::View {
        battlefield::View::new(self.camera, self.width, self.height)
    }

    /// The selection while dragging one: a rectangle on the ground as (west, north, east,
    /// south), between where the drag started on the map and the ground under the cursor.
    pub fn selection_ground(&self) -> Option<(f32, f32, f32, f32)> {
        let (x, z) = self.selection_anchor?;
        let (cursor_x, cursor_z) = self.view().ground_at(self.mouse.0, self.mouse.1)?;
        Some((
            x.min(cursor_x),
            z.min(cursor_z),
            x.max(cursor_x),
            z.max(cursor_z),
        ))
    }

    /// The selection's four edges as they appear on screen, with perspective, each cut down
    /// to the part that is visible.
    pub fn selection_edges(&self) -> Vec<((f32, f32), (f32, f32))> {
        let Some((west, north, east, south)) = self.selection_ground() else {
            return Vec::new();
        };
        let view = self.view();
        let screen = (0.0, 0.0, self.width, self.height);
        let corners = [(west, north), (east, north), (east, south), (west, south)];
        (0..corners.len())
            .filter_map(|i| {
                let (a, b) =
                    view.project_ground_line(corners[i], corners[(i + 1) % corners.len()])?;
                battlefield::clip_to_rect(a, b, screen)
            })
            .collect()
    }

    /// Whether the drag in progress would select this unit. An aircraft counts both by the
    /// spot below it and by the spot it appears over on screen, since a box drawn around
    /// what the player sees should catch it.
    pub fn in_selection_drag(&self, unit: &Unit) -> bool {
        let Some(rectangle) = self.selection_ground().filter(|_| unit.is_selectable()) else {
            return false;
        };
        units::in_selection(rectangle, unit) || self.appears_in_selection(rectangle, unit)
    }

    fn appears_in_selection(&self, rectangle: (f32, f32, f32, f32), unit: &Unit) -> bool {
        if !unit.kind.flies() {
            return false;
        }
        let view = self.view();
        let (x, y, z) = unit.center();
        let appears_over = view
            .project(x, y, z)
            .and_then(|(px, py)| view.ground_at(px, py));
        appears_over.is_some_and(|(x, z)| units::in_rectangle(rectangle, x, z))
    }

    /// Whether the cursor is over the battlefield itself, with nothing in the way.
    fn cursor_on_battlefield(&self) -> bool {
        let (x, y) = self.mouse;
        self.screen == Screen::InGame
            && !self.key_paused
            && !self.stats_open
            && !hud::covers(x, y, self.width, self.height)
    }

    fn begin_selection(&mut self) {
        if self.cursor_on_battlefield() {
            self.selection_anchor = self.view().ground_at(self.mouse.0, self.mouse.1);
        }
    }

    /// Whether the left button, if down on the battlefield, has stayed near where it went
    /// down on screen, which makes this a click and not a box.
    fn is_click_so_far(&self) -> bool {
        let Some((x, z)) = self.selection_anchor else {
            return true;
        };
        let start = self.view().project_clamped(x, 0.0, z);
        let slop = CLICK_SLOP * ui_scale(self.height);
        (start.0 - self.mouse.0).abs() < slop && (start.1 - self.mouse.1).abs() < slop
    }

    fn selected_units(&self) -> Vec<&Unit> {
        self.units.iter().filter(|u| u.selected).collect()
    }

    fn can_order_attack_here(&self) -> bool {
        self.cursor_on_battlefield()
            && !self.aiming_strike
            && !self.minimap_dragging
            && self.is_click_so_far()
    }

    /// Who a click would send the selected units after: an enemy they can fight, or a friendly
    /// vehicle a repair vehicle among them can fix. It must be under the cursor and no
    /// farther from it than any other of the player's own.
    fn attackable_under_cursor(&self) -> Option<usize> {
        if !self.can_order_attack_here() {
            return None;
        }
        let selected = self.selected_units();
        let able = |u: &Unit| {
            selected
                .iter()
                .any(|s| s.can_attack(u, &self.teams) || s.can_tend(u, &self.teams))
        };
        let (chosen, distance) = self.unit_near_cursor(able)?;
        let own = self.unit_near_cursor(Unit::is_selectable);
        own.is_none_or(|(own_unit, own_distance)| own_unit == chosen || distance < own_distance)
            .then_some(chosen)
    }

    /// What a cell of the build menu makes.
    pub fn build_role(tab: BuildTab, cell: usize) -> Option<Role> {
        match tab {
            BuildTab::Infantry => INFANTRY_BUILDS.get(cell).copied(),
            BuildTab::Vehicles if cell == CARRYALL_CELL => Some(Role::AdvancedCarryall),
            BuildTab::Vehicles => VEHICLE_BUILDS
                .get(cell)
                .or_else(|| BOTTOM_VEHICLE_BUILDS.get(cell.checked_sub(BOTTOM_ROW_START)?))
                .copied(),
            BuildTab::Buildings => None,
        }
    }

    /// Whether the player has the building that makes this.
    pub fn can_produce(&self, role: Role) -> bool {
        units::maker_of(&self.units, role, 0).is_some()
    }

    /// Adds a unit of the player's by the building that makes it, if they have one. There is
    /// no cost or waiting yet.
    fn produce(&mut self, role: Role) {
        let terrain = battlefield::terrain(self.battle_map);
        if units::produce(&mut self.units, terrain, role, 0, self.next_unit_id) {
            self.next_unit_id += 1;
            self.post_chat(format!("{} ready", role.name()));
        }
    }

    /// The tip text for what the cursor is over: a button making something, or a unit.
    pub fn tip(&self) -> Option<&'static str> {
        if !self.toggle_value(Toggle::ToolTips) {
            return None;
        }
        if let Some(Action::BuildCell(cell)) = self.hovered() {
            if self.build_tab == BuildTab::Vehicles && cell == SUPERWEAPON_CELL {
                return Some("Superweapon");
            }
            return Game::build_role(self.build_tab, cell).map(|role| {
                match role.made_in().filter(|_| !self.can_produce(role)) {
                    Some(Role::Barracks) => "Needs a Barracks",
                    Some(Role::Factory) => "Needs a Factory",
                    Some(_) => "Needs a Hangar",
                    None => role.name(),
                }
            });
        }
        if !self.cursor_on_battlefield() || self.selection_anchor.is_some() {
            return None;
        }
        self.unit_near_cursor(|_| true)
            .map(|(i, _)| self.units[i].name())
    }

    /// With Control held, what a click would make the selected units fire on: anyone they
    /// are able to under the cursor, or else the ground there.
    fn forced_order_under_cursor(&self) -> Option<AttackOrder> {
        if !self.control_held || !self.can_order_attack_here() {
            return None;
        }
        let selected = self.selected_units();
        let able = |u: &Unit| {
            selected.iter().any(|s| {
                s.can_fire_on(u) || s.can_attack(u, &self.teams) || s.can_tend(u, &self.teams)
            })
        };
        if let Some((unit, _)) = self.unit_near_cursor(able) {
            return Some(AttackOrder::Unit(unit));
        }
        if !selected.iter().any(|s| s.can_fire_at_ground()) {
            return None;
        }
        self.view()
            .ground_at(self.mouse.0, self.mouse.1)
            .map(AttackOrder::Place)
    }

    /// What the cursor should look like: what a click here would make the selected units do.
    pub fn cursor_look(&self) -> CursorLook {
        let selected = self.selected_units();
        let look_at = |u: &Unit| {
            let armed =
                |s: &&Unit| s.can_attack(u, &self.teams) || (self.control_held && s.can_fire_on(u));
            if selected.iter().any(armed) {
                CursorLook::Attack
            } else if selected.iter().any(|s| s.can_fix(u, &self.teams)) {
                CursorLook::Heal
            } else {
                CursorLook::Carry
            }
        };
        match self.forced_order_under_cursor() {
            Some(AttackOrder::Place(_)) => CursorLook::Attack,
            Some(AttackOrder::Unit(unit)) => look_at(&self.units[unit]),
            None => match self.attackable_under_cursor() {
                Some(unit) => look_at(&self.units[unit]),
                None if self.would_put_down_here() => CursorLook::Carry,
                None => CursorLook::Arrow,
            },
        }
    }

    /// Whether a click would send a selected carryall here to put down what it holds.
    fn would_put_down_here(&self) -> bool {
        self.can_order_attack_here()
            && self.selected_units().iter().any(|u| u.carrying)
            && self.unit_under_cursor().is_none()
            && self.view().ground_at(self.mouse.0, self.mouse.1).is_some()
    }

    #[cfg(test)]
    fn cursor_means_attack(&self) -> bool {
        self.cursor_look() == CursorLook::Attack
    }

    /// Ends the drag. A real box selects the player's units in it, replacing the old
    /// selection. Anything that stayed near its start on screen is a click: on one of the
    /// player's units it selects that unit, on an enemy they can fight it sends them after
    /// that enemy, and elsewhere it sends them there. With Control held it orders an attack
    /// on whatever is there, friend or bare ground. With `adding`, what it picks joins the
    /// selection instead, and a click that picks nothing does nothing, so that a near miss
    /// does not send everyone off.
    fn finish_selection(&mut self, adding: bool) {
        if self.selection_anchor.is_none() {
            return;
        }
        if self.aiming_strike {
            if let Some((target_x, target_z)) = self.view().ground_at(self.mouse.0, self.mouse.1) {
                self.combat.launch_strike(target_x, target_z);
                self.strike_recharge = SUPERWEAPON_RECHARGE;
                self.aiming_strike = false;
            }
            self.selection_anchor = None;
            return;
        }
        if !self.is_click_so_far() {
            let chosen: Vec<bool> = self
                .units
                .iter()
                .map(|u| self.in_selection_drag(u))
                .collect();
            for (unit, chosen) in self.units.iter_mut().zip(chosen) {
                unit.selected = chosen || (adding && unit.selected);
            }
        } else if let Some(order) = self.forced_order_under_cursor() {
            match order {
                AttackOrder::Unit(unit) => {
                    units::order_force_attack(&mut self.units, unit, &self.teams)
                }
                AttackOrder::Place(place) => units::order_attack_place(&mut self.units, place),
            }
        } else if let Some(chosen) = self.attackable_under_cursor() {
            units::order_attack(&mut self.units, chosen, &self.teams);
        } else if let Some(picked) = self.unit_under_cursor() {
            for (i, unit) in self.units.iter_mut().enumerate() {
                unit.selected = i == picked || (adding && unit.selected);
            }
        } else if adding {
            // Nothing picked, nothing to add.
        } else if let Some(target) = self.view().ground_at(self.mouse.0, self.mouse.1) {
            units::order_move(
                &mut self.units,
                target,
                battlefield::terrain(self.battle_map),
                &self.teams,
            );
        }
        self.selection_anchor = None;
    }

    /// The nearest of the player's units to the cursor, within reach of a click.
    fn unit_under_cursor(&self) -> Option<usize> {
        self.unit_near_cursor(Unit::is_selectable).map(|(i, _)| i)
    }

    /// The nearest wanted unit to the cursor within reach of a click, and how far it is.
    fn unit_near_cursor(&self, wanted: impl Fn(&Unit) -> bool) -> Option<(usize, f32)> {
        let view = self.view();
        let reach = PICK_RADIUS * ui_scale(self.height);
        let mut best = None;
        let mut best_distance = reach;
        let wanted = |unit: &Unit| self.shows(unit) && wanted(unit);
        for (i, unit) in self.units.iter().enumerate().filter(|(_, u)| wanted(u)) {
            let Some((x, y)) = view.project(unit.x, unit.pick_height(), unit.z) else {
                continue;
            };
            let distance = (x - self.mouse.0).hypot(y - self.mouse.1);
            if distance <= best_distance {
                best = Some((i, distance));
                best_distance = distance;
            }
        }
        best
    }

    pub fn player_money(&self) -> i32 {
        self.money[0]
    }

    /// Posts a message, scrolling out the oldest if the chat is full.
    fn post_chat(&mut self, text: String) {
        self.chat.push(ChatLine {
            text,
            posted_at: self.battle_time,
        });
        if self.chat.len() > hud::CHAT_LINES {
            self.chat.remove(0);
        }
    }

    /// The player, then each opponent in this battle.
    pub fn stats_rows(&self) -> Vec<StatsRow> {
        let setup = &self.skirmish;
        if self.battle_kind != BattleKind::Skirmish {
            let color = match self.faction {
                Some(Faction::H) => TeamColor::Red,
                Some(Faction::O) => TeamColor::Cyan,
                Some(Faction::A) | None => TeamColor::Blue,
            };
            let enemy_color = if color == TeamColor::Red {
                TeamColor::Blue
            } else {
                TeamColor::Red
            };
            return vec![
                StatsRow {
                    name: setup.commander.clone(),
                    faction: self.faction,
                    color,
                    allies: [Ally::None; 2],
                    money: self.player_money(),
                },
                StatsRow {
                    name: "Enemy".to_string(),
                    faction: None,
                    color: enemy_color,
                    allies: [Ally::None; 2],
                    money: self.money[1],
                },
            ];
        }
        let mut rows = vec![StatsRow {
            name: setup.commander.clone(),
            faction: Some(setup.faction),
            color: setup.color,
            allies: setup.allies,
            money: self.player_money(),
        }];
        for (i, opponent) in setup.opponents[..setup.ai_players as usize]
            .iter()
            .enumerate()
        {
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

    /// Counts the dead, clears them away, and decides the battle if that ends it: it is won
    /// when no enemy of the player is left, and lost when the player has nothing left.
    fn settle_casualties(&mut self) {
        let mut lost_names = Vec::new();
        for unit in self.units.iter().filter(|u| !u.alive) {
            if unit.owner == 0 {
                self.lost.count(unit);
                lost_names.push(unit.name());
            } else if units::are_enemies(&self.teams, 0, unit.owner) {
                self.killed.count(unit);
            }
        }
        self.units.retain(|u| u.alive);
        for name in lost_names {
            self.post_chat(format!("{name} lost"));
        }
        self.report_defeats();
        if self.outcome.is_some() {
            return;
        }
        let player_left = self.units.iter().any(|u| u.owner == 0);
        let enemies_left = self
            .units
            .iter()
            .any(|u| units::are_enemies(&self.teams, 0, u.owner));
        self.outcome = if !player_left {
            Some(Outcome::Defeat)
        } else if !enemies_left {
            Some(Outcome::Victory)
        } else {
            None
        };
        if self.outcome.is_some() {
            self.key_paused = false;
            self.aiming_strike = false;
            self.outcome_elapsed = 0.0;
        }
    }

    /// Posts a message for each side that has just lost everything.
    fn report_defeats(&mut self) {
        let rows = self.stats_rows();
        for (owner, row) in rows.iter().enumerate() {
            let beaten = !self.units.iter().any(|u| u.owner == owner);
            if beaten && !self.reported_defeated[owner] {
                self.reported_defeated[owner] = true;
                self.post_chat(format!("{} defeated", row.name));
            }
        }
    }

    fn show_battle_stats(&mut self) {
        self.in_battle = false;
        self.stats_open = false;
        self.set_screen(Screen::BattleStats);
    }

    /// Past the statistics, each kind of battle returns to where it belongs. A campaign
    /// moves on after a victory and replays the mission after a defeat.
    fn leave_battle_stats(&mut self) {
        match self.battle_kind {
            BattleKind::Skirmish => self.set_screen(Screen::SkirmishSetup),
            BattleKind::Tutorial => self.set_screen(Screen::SinglePlayerMenu),
            BattleKind::Campaign => {
                if self.outcome == Some(Outcome::Victory) {
                    self.campaign_mission += 1;
                }
                self.start_battle(
                    self.campaign_mission % MAPS.len(),
                    self.faction,
                    BattleKind::Campaign,
                );
            }
        }
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

    fn start_battle(&mut self, map: usize, faction: Option<Faction>, kind: BattleKind) {
        let skirmish = kind == BattleKind::Skirmish;
        self.battle_map = map;
        self.faction = faction;
        self.battle_kind = kind;
        self.outcome = None;
        self.outcome_elapsed = 0.0;
        self.killed = Casualties::default();
        self.lost = Casualties::default();
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
            vec![DEFAULT_BATTLE_MONEY; 2]
        };
        self.chat.clear();
        let colors: Vec<TeamColor> = self.stats_rows().iter().map(|row| row.color).collect();
        self.teams = (0..colors.len()).collect();
        self.reported_defeated = vec![false; colors.len()];
        self.units = units::spawn(battlefield::terrain(map), &colors);
        self.next_unit_id = self.units.len() as u32;
        self.fog = self.fog_allowed().then(Fog::new);
        self.update_fog();
        self.carried.clear();
        self.combat = Combat::default();
        self.aiming_strike = false;
        self.strike_recharge = 0.0;
        self.set_screen(Screen::Loading);
    }

    pub fn menu_seconds(&self) -> f32 {
        self.menu_seconds
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
            self.skirmish
                .assign_opponent_colors(old_ai_players, value as usize);
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
        let mut buttons = self.buttons().into_iter();
        if self.dropdown_open || self.color_open {
            // An open list takes all input; what lies around it stays inactive.
            return buttons.find(|b| b.contains(x, y)).map(|b| b.action);
        }
        let mut widgets = Vec::new();
        if self.screen == Screen::InGame && !self.key_paused {
            widgets = if self.outcome.is_some() {
                // The battle is over: nothing more can be built.
                hud::command_buttons(self.width, self.height)
            } else {
                hud::all_buttons(self.width, self.height)
            };
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
        self.sliders()
            .iter()
            .find(|s| s.contains(x, y))
            .map(|s| s.id)
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
            Screen::Intro
            | Screen::MainMenu
            | Screen::Loading
            | Screen::Movie(_)
            | Screen::BattleStats => None,
        }
    }

    fn go_back(&mut self) {
        if self.screen == Screen::BattleStats {
            self.leave_battle_stats();
            return;
        }
        // With the message box up, Escape answers it instead of leaving the screen.
        if self.confirming.take().is_some() {
            return;
        }
        if self.aiming_strike {
            self.aiming_strike = false;
            self.selection_anchor = None;
            return;
        }
        if self.stats_open {
            self.stats_open = false;
            return;
        }
        if self.screen == Screen::InGame && self.outcome.is_some() {
            // Past the message goes on to the statistics, not to the battlefield menu.
            self.show_battle_stats();
            return;
        }
        if self.dropdown_open || self.color_open {
            self.close_lists();
            return;
        }
        if let Some(parent) = self.parent() {
            self.set_screen(parent);
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.menu_seconds += dt;
        self.screen_elapsed += dt;
        if self.in_battle && !self.is_paused() {
            self.battle_time += dt;
            let battle_time = self.battle_time;
            self.chat
                .retain(|line| battle_time - line.posted_at < CHAT_SECONDS);
            let terrain = battlefield::terrain(self.battle_map);
            units::advance(&mut self.units, terrain, &self.teams, dt);
            self.combat
                .advance(&mut self.units, &self.teams, terrain, dt);
            units::carry(&mut self.units, &mut self.carried, terrain, &self.teams, dt);
            let was_recharging = self.strike_recharge > 0.0;
            self.strike_recharge = (self.strike_recharge - dt).max(0.0);
            if was_recharging && self.strike_recharge <= 0.0 && self.superweapon_allowed() {
                self.post_chat("Superweapon ready".to_string());
            }
            if self.screen == Screen::InGame {
                self.settle_casualties();
            }
            self.update_fog();
        }
        match self.screen {
            Screen::Intro if self.screen_elapsed >= INTRO_SECONDS => {
                self.set_screen(Screen::MainMenu)
            }
            Screen::Loading if self.screen_elapsed >= LOADING_SECONDS => {
                self.set_screen(Screen::InGame)
            }
            Screen::Movie(faction) if self.screen_elapsed >= MOVIE_SECONDS => {
                self.finish_movie(faction)
            }
            Screen::InGame
                if self.outcome.is_some() && self.outcome_elapsed + dt >= OUTCOME_SECONDS =>
            {
                self.show_battle_stats();
            }
            Screen::InGame => {
                if self.outcome.is_some() {
                    self.outcome_elapsed += dt;
                }
                self.camera.ease_zoom(dt);
                if self.focused && !self.key_paused && !self.minimap_dragging {
                    let direction =
                        battlefield::edge_direction(self.mouse, self.width, self.height);
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
                    self.control_held = false;
                }
            }
            Event::Control(held) => self.control_held = held,
            Event::RightDown => {
                if self.aiming_strike {
                    // Only calls off the superweapon; the selection stands.
                    self.aiming_strike = false;
                    self.selection_anchor = None;
                } else if self.screen == Screen::InGame && !self.key_paused {
                    self.selection_anchor = None;
                    for unit in &mut self.units {
                        unit.selected = false;
                    }
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
                self.mouse = (
                    self.mouse.0.min(self.width - 1.0),
                    self.mouse.1.min(self.height - 1.0),
                );
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
            Event::MouseUp | Event::ShiftMouseUp => {
                self.dragging = None;
                self.minimap_dragging = false;
                self.finish_selection(event == Event::ShiftMouseUp);
                let pressed = self.pressed.take();
                if self.screen == Screen::Intro {
                    self.set_screen(Screen::MainMenu);
                } else if let Screen::Movie(faction) = self.screen {
                    self.finish_movie(faction);
                } else if let Some(action) = pressed.filter(|a| Some(*a) == self.hovered()) {
                    self.activate(action);
                } else {
                    // A click on anything else closes an open list.
                    self.close_lists();
                }
            }
            Event::KeyDown(key) if self.name_focused => self.edit_name(key),
            Event::KeyDown(key) => match (self.screen, key) {
                (Screen::Intro, _) => self.set_screen(Screen::MainMenu),
                (Screen::Movie(faction), _) => self.finish_movie(faction),
                (Screen::InGame, Key::Pause) if self.pausable && self.outcome.is_none() => {
                    self.key_paused = !self.key_paused;
                    self.selection_anchor = None;
                }
                (Screen::InGame, Key::Tab) => self.stats_open = !self.stats_open,
                (Screen::InGame, Key::Char('d' | 'D')) if !self.key_paused => {
                    units::toggle_deploy(&mut self.units)
                }
                (Screen::InGame, Key::Char('a' | 'A')) if !self.key_paused => {
                    for unit in &mut self.units {
                        unit.selected = unit.is_selectable();
                    }
                }
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
            Action::Tutorial => self.start_battle(0, None, BattleKind::Tutorial),
            Action::LeaveBattleStats => self.leave_battle_stats(),
            Action::ExitGame => self.leave_battle(),
            Action::ReturnToGame => self.set_screen(Screen::InGame),
            Action::StartCampaign => self.set_screen(Screen::FactionSelect),
            Action::Faction(faction) => self.set_screen(Screen::Movie(faction)),
            Action::CancelFaction => self.confirming = None,
            Action::ConfirmFaction => {
                self.campaign_mission = 0;
                self.start_battle(0, self.confirming, BattleKind::Campaign);
            }
            Action::SkirmishBattle => self.set_screen(Screen::SkirmishSetup),
            Action::StartBattle => self.start_battle(self.skirmish.map, None, BattleKind::Skirmish),
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
            Action::BuildTab(tab) => self.build_tab = tab,
            Action::BuildCell(cell) => self.activate_build_cell(cell),
            Action::HudWide(DEPLOY_BUTTON) => units::toggle_deploy(&mut self.units),
            // Not designed yet: these buttons respond to hover but do nothing.
            Action::Multiplayer
            | Action::LoadGame
            | Action::SaveGame
            | Action::HudSmall(_)
            | Action::HudWide(_) => {}
        }
    }

    /// Makes what the cell makes, or gets the superweapon ready to aim.
    fn activate_build_cell(&mut self, cell: usize) {
        if let Some(role) = Game::build_role(self.build_tab, cell) {
            self.produce(role);
        } else if self.build_tab == BuildTab::Vehicles
            && cell == SUPERWEAPON_CELL
            && self.superweapon_allowed()
            && self.strike_recharge <= 0.0
        {
            self.aiming_strike = true;
        }
    }

    fn close_lists(&mut self) {
        self.dropdown_open = false;
        self.color_open = false;
    }

    fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.pressed = None;
        self.dragging = None;
        self.minimap_dragging = false;
        self.selection_anchor = None;
        self.confirming = None;
        self.name_focused = false;
        self.close_lists();
        self.screen_elapsed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::Vertex;

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
            [
                Action::SinglePlayer,
                Action::Multiplayer,
                Action::Exit,
                Action::Options
            ]
        );
        let b = game.buttons();
        assert!(
            (b[0].x + b[0].w / 2.0 - W / 2.0).abs() < 0.5,
            "centered horizontally"
        );
        assert!(
            b[0].y < H / 2.0 && b[1].y + b[1].h > H / 2.0,
            "centered vertically"
        );
        assert!(b[2].x < W / 2.0 && b[2].y > H / 2.0, "exit is bottom left");
        assert!(
            b[3].x > W / 2.0 && b[3].y > H / 2.0,
            "options is bottom right"
        );
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
        assert_eq!(
            (back.x, back.y),
            (exit.x, exit.y),
            "back takes exit's place"
        );

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
                "Back",
                "Game",
                "Graphics",
                "Sound",
                "Multiplayer Info",
                "Network",
                "Keyboard",
                "Sneak Peek",
                "Credits",
                "Movies"
            ]
        );
        let back = game.buttons()[0];
        assert!(back.x < W / 2.0 && back.y > H / 2.0, "back is bottom left");
    }

    #[test]
    fn panel_sits_right_of_center_with_tabs_on_its_left() {
        let game = in_options();
        let (px, _, pw, _) = options_panel(W, H);
        assert!(
            px + pw / 2.0 > W / 2.0,
            "panel center is right of screen center"
        );
        for tab in &game.buttons()[1..] {
            assert!(tab.x + tab.w <= px, "{} is left of the panel", tab.label);
        }
    }

    #[test]
    fn each_tab_shows_the_requested_controls() {
        let names = |tab| -> Vec<_> { sliders(tab, W, H).iter().map(|s| s.label).collect() };
        assert_eq!(
            names(Tab::Game),
            ["Game Speed", "Scroll Rate", "Mouse Speed"]
        );
        assert_eq!(names(Tab::Graphics), ["Graphics Quality"]);
        assert_eq!(
            names(Tab::Sound),
            ["Music Volume", "Sound Volume", "Voice Volume"]
        );
        let boxes: Vec<_> = checkboxes(Tab::Game, W, H)
            .iter()
            .map(|b| b.label)
            .collect();
        assert_eq!(boxes, ["Tool Tips", "Target Lines"]);
        assert!(checkboxes(Tab::Sound, W, H).is_empty());
        for tab in [
            Tab::MultiplayerInfo,
            Tab::Network,
            Tab::Keyboard,
            Tab::SneakPeek,
            Tab::Movies,
        ] {
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
        drag(
            &mut game,
            SliderId::GraphicsQuality,
            s.track_x + s.track_w * 0.45,
        );
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
        let row = |game: &Game, t| {
            *game
                .checkboxes()
                .iter()
                .find(|b| b.action == Action::Toggle(t))
                .unwrap()
        };
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
        assert!(
            game.buttons().is_empty(),
            "the battlefield has no Back button"
        );
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
        assert_eq!(
            labels,
            [
                "Load Game",
                "Save Game",
                "Options",
                "Exit Game",
                "Return to Game"
            ]
        );
        let middle = game.buttons()[2];
        assert!(
            (middle.x + middle.w / 2.0 - W / 2.0).abs() < 0.5,
            "centered"
        );
        assert!(
            middle.y < H / 2.0 && middle.y + middle.h > H / 2.0,
            "centered"
        );

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
        assert!(
            game.key_paused && game.is_paused(),
            "still paused after the menu"
        );

        game.handle(Event::KeyDown(Key::Pause));
        assert!(
            !game.key_paused && !game.is_paused(),
            "the same key unpauses"
        );
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
        assert!(
            !game.sliders().is_empty(),
            "the same options as the separate screen"
        );

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
    fn diplomacy_outside_a_skirmish_lists_the_player_and_one_enemy() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::O);
        click(&mut game, Action::ConfirmFaction);
        game.update(LOADING_SECONDS);
        let rows = game.stats_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].faction, Some(Faction::O));
        assert_eq!(rows[0].color, TeamColor::Cyan);
        assert_ne!(rows[1].color, rows[0].color);
    }

    #[test]
    fn a_red_player_faces_an_enemy_of_another_color() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::H);
        click(&mut game, Action::ConfirmFaction);
        let rows = game.stats_rows();
        assert_eq!(rows[0].color, TeamColor::Red);
        assert_ne!(rows[1].color, TeamColor::Red);
    }

    /// Kills every unit of the given owners and lets the game notice.
    fn kill_owners(game: &mut Game, owners: &[usize]) {
        for unit in game.units.iter_mut().filter(|u| owners.contains(&u.owner)) {
            unit.alive = false;
        }
        game.update(0.01);
    }

    fn all_enemies(game: &Game) -> Vec<usize> {
        (1..game.teams.len())
            .filter(|&o| units::are_enemies(&game.teams, 0, o))
            .collect()
    }

    fn win(game: &mut Game) {
        let enemies = all_enemies(game);
        kill_owners(game, &enemies);
        assert_eq!(game.outcome, Some(Outcome::Victory));
    }

    fn tutorial_battle() -> Game {
        let mut game = in_menu();
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game
    }

    fn campaign_battle() -> Game {
        let mut game = in_faction_select();
        pick(&mut game, Faction::A);
        click(&mut game, Action::ConfirmFaction);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game
    }

    #[test]
    fn every_kind_of_battle_starts_undecided_with_enemies_to_beat() {
        for mut game in [in_battle(), tutorial_battle(), campaign_battle()] {
            game.update(1.0);
            assert_eq!(game.outcome, None);
            assert!(!all_enemies(&game).is_empty());
        }
    }

    #[test]
    fn killing_every_enemy_wins_and_losing_everything_loses() {
        let mut game = in_battle();
        let enemies = all_enemies(&game);
        kill_owners(&mut game, &enemies[1..]);
        assert_eq!(game.outcome, None, "one is still left");
        win(&mut game);

        let mut game = in_battle();
        kill_owners(&mut game, &[0]);
        assert_eq!(game.outcome, Some(Outcome::Defeat));
    }

    #[test]
    fn allies_need_not_be_beaten() {
        let mut game = in_battle();
        // Opponent 2 is on the player's side.
        game.teams[2] = game.teams[0];
        let enemies = all_enemies(&game);
        assert!(!enemies.contains(&2));
        kill_owners(&mut game, &enemies);
        assert_eq!(game.outcome, Some(Outcome::Victory));
        assert!(
            game.units.iter().any(|u| u.owner == 2),
            "the ally is still there"
        );
    }

    #[test]
    fn statistics_count_kills_and_losses_by_kind_but_not_allies() {
        let mut game = in_battle();
        game.teams[2] = game.teams[0];
        let enemy_infantry = game
            .units
            .iter()
            .filter(|u| u.owner == 1 && u.kind == UnitKind::Infantry)
            .count() as i32;
        let lost_infantry = game
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .count() as i32;
        kill_owners(&mut game, &[1, 2]);
        assert_eq!(game.killed.infantry, enemy_infantry, "allies do not count");
        assert_eq!(game.killed.vehicles, 0);

        let vehicles = game
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
            .count() as i32;
        for unit in game
            .units
            .iter_mut()
            .filter(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
        {
            unit.alive = false;
        }
        game.update(0.01);
        assert_eq!(
            game.lost,
            Casualties {
                infantry: lost_infantry,
                ..Casualties::default()
            }
        );
        assert!(vehicles > 0);
    }

    #[test]
    fn driving_over_every_enemy_really_wins_a_skirmish() {
        let mut game = in_setup();
        let ai = slider(&game, SliderId::AiPlayers);
        drag(&mut game, SliderId::AiPlayers, ai.x_of(1));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        // Towers and aircraft cannot be run over, so this enemy has none.
        game.units
            .retain(|u| u.owner != 1 || !(u.kind.is_building() || u.kind.flies()));
        let count = game
            .units
            .iter()
            .filter(|u| u.owner == 1 && u.kind == UnitKind::Infantry)
            .count() as i32;
        for _ in 0..4000 {
            let Some(target) = game.units.iter().find(|u| u.owner == 1) else {
                break;
            };
            let target = (target.x, target.z);
            let Some(vehicle) = game.units.iter().position(|u| u.heavy) else {
                break;
            };
            game.units[vehicle].destination = target;
            game.update(0.05);
            if game.outcome.is_some() {
                break;
            }
        }
        assert_eq!(game.outcome, Some(Outcome::Victory));
        assert_eq!(game.killed.infantry, count);
    }

    #[test]
    fn the_message_lasts_ten_seconds_then_shows_the_statistics() {
        let mut game = in_battle();
        win(&mut game);
        game.update(OUTCOME_SECONDS - 0.5);
        assert_eq!(game.screen, Screen::InGame, "still up");
        game.update(1.0);
        assert_eq!(game.screen, Screen::BattleStats);
        assert!(!game.in_battle);
    }

    #[test]
    fn escape_skips_the_message_and_never_opens_the_battlefield_menu() {
        let mut game = in_battle();
        win(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::BattleStats);

        let mut game = in_battle();
        kill_owners(&mut game, &[0]);
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::BattleStats, "after a defeat too");
    }

    #[test]
    fn during_the_message_you_can_navigate_but_not_use_the_build_menu() {
        let mut game = in_battle();
        win(&mut game);

        let tab = hud::build_tabs(W, H)[1];
        click_at(&mut game, tab.x + 1.0, tab.y + 1.0);
        assert_eq!(game.build_tab, BuildTab::Infantry, "tabs no longer work");
        let cell = hud::grid_cells(W, H)[0];
        game.handle(Event::MouseMove(cell.x + 1.0, cell.y + 1.0));
        assert_eq!(game.hovered(), None, "nor does the grid");

        let start = game.camera;
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.update(0.2);
        assert_ne!(game.camera, start, "the camera still scrolls");
        game.handle(Event::Wheel(5.0));
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.update(0.5);
        assert!(game.camera.zoom > 0.0, "and zooms");
    }

    #[test]
    fn the_pause_key_does_nothing_once_the_battle_is_over() {
        let mut game = in_battle();
        win(&mut game);
        game.handle(Event::KeyDown(Key::Pause));
        assert!(!game.key_paused);
    }

    #[test]
    fn statistics_lead_to_the_setup_screen_after_a_skirmish() {
        for leave in [Event::KeyDown(Key::Escape), Event::MouseUp] {
            let mut game = in_battle();
            win(&mut game);
            game.handle(Event::KeyDown(Key::Escape));
            assert_eq!(game.buttons()[0].action, Action::LeaveBattleStats);
            if leave == Event::MouseUp {
                click(&mut game, Action::LeaveBattleStats);
            } else {
                game.handle(leave);
            }
            assert_eq!(game.screen, Screen::SkirmishSetup);
        }
    }

    #[test]
    fn statistics_lead_to_the_tutorial_button_after_a_tutorial() {
        let mut game = tutorial_battle();
        win(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        click(&mut game, Action::LeaveBattleStats);
        assert_eq!(game.screen, Screen::SinglePlayerMenu);
        assert!(actions(&game).contains(&Action::Tutorial));
    }

    #[test]
    fn a_campaign_moves_to_the_next_mission_after_a_victory_and_replays_after_a_defeat() {
        let mut game = campaign_battle();
        assert_eq!(game.campaign_mission, 0);
        win(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        click(&mut game, Action::LeaveBattleStats);
        assert_eq!(game.screen, Screen::Loading);
        assert_eq!(game.campaign_mission, 1);
        assert_eq!(game.battle_map, 1, "on another map");
        assert_eq!(game.faction, Some(Faction::A), "for the same faction");
        game.update(LOADING_SECONDS);
        assert_eq!(game.outcome, None, "a fresh start");
        assert_eq!(game.killed, Casualties::default());

        kill_owners(&mut game, &[0]);
        assert_eq!(game.outcome, Some(Outcome::Defeat));
        game.handle(Event::KeyDown(Key::Escape));
        click(&mut game, Action::LeaveBattleStats);
        assert_eq!(game.campaign_mission, 1, "the same mission again");
    }

    #[test]
    fn a_new_campaign_starts_at_the_first_mission() {
        let mut game = campaign_battle();
        win(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        click(&mut game, Action::LeaveBattleStats);
        game.update(LOADING_SECONDS);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::StartCampaign);
        pick(&mut game, Faction::O);
        click(&mut game, Action::ConfirmFaction);
        assert_eq!(game.campaign_mission, 0);
    }

    #[test]
    fn campaign_missions_keep_working_past_the_last_map() {
        let mut game = campaign_battle();
        for _ in 0..MAPS.len() + 2 {
            win(&mut game);
            game.handle(Event::KeyDown(Key::Escape));
            click(&mut game, Action::LeaveBattleStats);
            game.update(LOADING_SECONDS);
            assert_eq!(game.screen, Screen::InGame);
        }
    }

    fn pick_superweapon(game: &mut Game) {
        click_hud(game, Action::BuildTab(BuildTab::Vehicles));
        click_hud(game, Action::BuildCell(SUPERWEAPON_CELL));
    }

    #[test]
    fn the_superweapon_lives_in_the_vehicles_menu_only() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildCell(SUPERWEAPON_CELL));
        assert!(!game.aiming_strike, "not on the infantry tab");
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        click_hud(&mut game, Action::BuildCell(SUPERWEAPON_CELL));
        assert!(!game.aiming_strike, "nor on the buildings tab");
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_hud(&mut game, Action::BuildCell(SUPERWEAPON_CELL + 1));
        assert!(!game.aiming_strike, "nor in another cell");
        click_hud(&mut game, Action::BuildCell(SUPERWEAPON_CELL));
        assert!(game.aiming_strike);
    }

    #[test]
    fn once_chosen_the_next_click_launches_it_there_and_selects_nobody() {
        let mut game = in_battle();
        select_first_group(&mut game);
        pick_superweapon(&mut game);
        let point = (W / 2.0 - 500.0, H / 2.0 + 300.0);
        let expected = game.view().ground_at(point.0, point.1).unwrap();
        click_at(&mut game, point.0, point.1);
        assert!(!game.aiming_strike);
        assert_eq!(game.combat.strikes.len(), 1);
        assert_eq!(
            (game.combat.strikes[0].x, game.combat.strikes[0].z),
            expected
        );
        assert_eq!(selected_count(&game), FIRST_GROUP, "the selection stands");
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| !u.is_moving()),
            "and gets no order"
        );
    }

    #[test]
    fn aiming_can_be_called_off_with_the_right_button_or_escape() {
        let mut game = in_battle();
        select_first_group(&mut game);
        pick_superweapon(&mut game);
        game.handle(Event::RightDown);
        assert!(!game.aiming_strike);
        assert_eq!(selected_count(&game), FIRST_GROUP, "without deselecting");
        assert!(game.combat.strikes.is_empty());

        pick_superweapon(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        assert!(!game.aiming_strike);
        assert_eq!(game.screen, Screen::InGame, "without opening the menu");
    }

    #[test]
    fn the_superweapon_recharges_and_starts_ready_each_battle() {
        let mut game = in_battle();
        pick_superweapon(&mut game);
        click_at(&mut game, 400.0, H - 300.0);
        assert!(game.strike_recharge > 0.0);
        click_hud(&mut game, Action::BuildCell(SUPERWEAPON_CELL));
        assert!(!game.aiming_strike, "still recharging");
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        for _ in 0..(SUPERWEAPON_RECHARGE as usize + 1) {
            game.update(1.0);
        }
        if game.outcome.is_none() {
            click_hud(&mut game, Action::BuildCell(SUPERWEAPON_CELL));
            assert!(game.aiming_strike, "ready again");
        }

        let mut game = in_battle();
        pick_superweapon(&mut game);
        click_at(&mut game, 400.0, H - 300.0);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert_eq!(game.strike_recharge, 0.0);
        assert!(game.combat.strikes.is_empty());
    }

    #[test]
    fn the_setup_checkbox_turns_the_superweapon_off_in_a_skirmish() {
        let mut game = in_setup();
        click_widget(&mut game, Action::Toggle(Toggle::Superweapons));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        pick_superweapon(&mut game);
        assert!(!game.aiming_strike);
    }

    #[test]
    fn enemy_fire_can_now_defeat_the_player() {
        let mut game = in_battle();
        // Set down among them, big buildings would only push them about and keep them from firing.
        game.units
            .retain(|u| !(u.owner == 0 && u.kind == UnitKind::Building));
        // Everyone of the player's stands in the open beside armed enemies, unarmed.
        let enemy = game.units.iter().find(|u| u.owner == 1).copied().unwrap();
        for unit in game.units.iter_mut().filter(|u| u.owner == 0) {
            unit.x = enemy.x + 1.0;
            unit.z = enemy.z + 1.0;
            unit.destination = (unit.x, unit.z);
            unit.cooldown = 100_000.0;
            unit.health = 1.0;
        }
        for _ in 0..200 {
            game.update(0.1);
            if game.outcome.is_some() {
                break;
            }
        }
        assert_eq!(game.outcome, Some(Outcome::Defeat));
        assert!(game.lost.infantry > 0);
    }

    #[test]
    fn a_battle_can_be_won_by_gunfire_with_kills_counted() {
        let mut game = in_setup();
        let ai = slider(&game, SliderId::AiPlayers);
        drag(&mut game, SliderId::AiPlayers, ai.x_of(1));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        let enemies = game
            .units
            .iter()
            .filter(|u| u.owner == 1 && u.kind == UnitKind::Infantry)
            .count() as i32;
        let target = game
            .units
            .iter()
            .find(|u| u.owner == 1)
            .map(|u| (u.x, u.z))
            .unwrap();
        // Everyone of the player's walks into range and fires by themselves.
        select_first_group(&mut game);
        units::order_move(
            &mut game.units,
            target,
            battlefield::terrain(game.battle_map),
            &game.teams,
        );
        for _ in 0..3000 {
            game.update(0.05);
            if game.outcome.is_some() {
                break;
            }
        }
        assert_eq!(game.outcome, Some(Outcome::Victory));
        assert_eq!(game.killed.infantry, enemies);
        assert_eq!(
            game.killed.buildings, 5,
            "its towers and buildings fell too"
        );
    }

    #[test]
    fn nothing_fires_while_the_game_is_paused() {
        let mut game = in_battle();
        let enemy = game.units.iter().find(|u| u.owner == 1).copied().unwrap();
        game.units[0].x = enemy.x + 1.0;
        game.units[0].z = enemy.z + 1.0;
        game.units[0].destination = (game.units[0].x, game.units[0].z);
        let health = game.units[0].health;
        game.handle(Event::KeyDown(Key::Escape));
        game.update(2.0);
        assert_eq!(game.units[0].health, health);
    }

    #[test]
    fn combat_effects_and_poison_all_draw_without_trouble() {
        let mut game = in_battle();
        let (x, z) = (game.camera.x, game.camera.z);
        game.combat.launch_strike(x, z);
        game.combat
            .poison
            .push(crate::weapons::PoisonZone { x, z, life: 30.0 });
        game.combat.blasts.push(crate::weapons::Blast {
            at: (x, 0.0, z),
            radius: 2.0,
            life: 0.5,
        });
        game.combat.beams.push(crate::weapons::Beam {
            from: (x, 1.0, z),
            to: (x + 3.0, 1.0, z),
            life: 0.5,
            repair: false,
        });
        game.combat.beams.push(crate::weapons::Beam {
            from: (x, 1.0, z),
            to: (x + 3.0, 1.0, z),
            life: 0.5,
            repair: true,
        });
        game.combat.flares.push(crate::weapons::Flare {
            at: (x, 1.0, z),
            size: 7.0,
            life: 0.5,
        });
        game.units[0].health = 10.0;
        assert!(!crate::render::build(&game).is_empty());
        pick_superweapon(&mut game);
        assert!(game.aiming_strike);
        assert!(!crate::render::build(&game).is_empty());
    }

    #[test]
    fn every_screen_and_battle_state_draws_without_trouble() {
        let check =
            |game: &Game| assert!(!crate::render::build(game).is_empty(), "{:?}", game.screen);
        let mut game = in_menu();
        check(&game);
        click(&mut game, Action::SinglePlayer);
        check(&game);
        let mut battle = in_battle();
        check(&battle);
        battle.handle(Event::KeyDown(Key::Tab));
        check(&battle);
        battle.handle(Event::KeyDown(Key::Tab));
        battle.handle(Event::KeyDown(Key::Pause));
        check(&battle);
        battle.handle(Event::KeyDown(Key::Pause));
        battle.handle(Event::KeyDown(Key::Escape));
        check(&battle);
        battle.handle(Event::KeyDown(Key::Escape));
        win(&mut battle);
        check(&battle);
        battle.handle(Event::KeyDown(Key::Escape));
        assert_eq!(battle.screen, Screen::BattleStats);
        check(&battle);
    }

    #[test]
    fn the_diplomacy_window_fits_every_player() {
        use crate::font::text_width;
        for (w, h) in [
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            let (x, y, ww, wh) = stats_rect(w, h);
            assert!(
                x >= 0.0 && y >= 0.0 && x + ww <= w && y + wh <= h,
                "at {w}x{h}"
            );
            let s = ui_scale(h);
            assert!(text_width("AI 7 (Defensive)") * small_text_px(h) < 380.0 * s);
            const {
                assert!(
                    STATS_ROW_TOP + 8.0 * STATS_ROW_STEP <= 760.0,
                    "eight rows fit"
                )
            };
        }
    }

    fn click_hud(game: &mut Game, action: Action) {
        let b = hud::all_buttons(W, H)
            .into_iter()
            .find(|b| b.action == action)
            .unwrap();
        click_at(game, b.x + b.w / 2.0, b.y + b.h / 2.0);
    }

    #[test]
    fn build_tabs_switch_and_a_new_battle_starts_on_infantry() {
        let mut game = in_battle();
        assert_eq!(game.build_tab, BuildTab::Infantry);
        for tab in [
            BuildTab::Vehicles,
            BuildTab::Buildings,
            BuildTab::Infantry,
            BuildTab::Buildings,
        ] {
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
        for b in hud::command_buttons(W, H)
            .into_iter()
            .chain(hud::grid_cells(W, H))
        {
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

    fn chat_texts(game: &Game) -> Vec<&str> {
        game.chat.iter().map(|line| line.text.as_str()).collect()
    }

    #[test]
    fn nothing_is_posted_until_something_happens() {
        let mut game = in_battle();
        for _ in 0..20 {
            game.update(1.0);
        }
        assert!(chat_texts(&game).is_empty(), "{:?}", chat_texts(&game));
    }

    #[test]
    fn a_new_unit_reports_it_is_ready() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildCell(2));
        assert_eq!(chat_texts(&game), ["Sniper ready"]);
    }

    #[test]
    fn losing_a_unit_or_building_reports_it_but_killing_one_does_not() {
        let mut game = in_battle();
        let tower = tower_of(&game, 0);
        game.units[tower].alive = false;
        game.update(0.01);
        assert_eq!(chat_texts(&game), ["Tower lost"]);

        let mut game = in_battle();
        let enemy = game.units.iter().position(|u| u.owner == 1).unwrap();
        game.units[enemy].alive = false;
        game.update(0.01);
        assert!(chat_texts(&game).is_empty());
    }

    #[test]
    fn a_side_with_nothing_left_is_reported_defeated_once() {
        let mut game = in_battle();
        kill_owners(&mut game, &[1]);
        let name = game.stats_rows()[1].name.clone();
        assert_eq!(chat_texts(&game), [format!("{name} defeated")]);
        game.update(0.01);
        assert_eq!(chat_texts(&game).len(), 1, "only once");
    }

    #[test]
    fn the_player_is_reported_defeated_too() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildCell(0));
        game.units.retain(|u| u.owner != 0 || u.role == Role::Scout);
        game.chat.clear();
        kill_owners(&mut game, &[0]);
        let name = game.stats_rows()[0].name.clone();
        assert_eq!(
            chat_texts(&game),
            ["Scout lost".to_string(), format!("{name} defeated")]
        );
    }

    #[test]
    fn the_superweapon_reports_ready_when_it_has_recharged_and_not_before() {
        let mut game = in_battle();
        game.update(1.0);
        assert!(
            chat_texts(&game).is_empty(),
            "ready from the start needs no message"
        );

        game.strike_recharge = 2.0;
        game.update(1.0);
        assert!(chat_texts(&game).is_empty(), "still recharging");
        game.update(1.5);
        assert_eq!(chat_texts(&game), ["Superweapon ready"]);
        game.update(1.0);
        assert_eq!(chat_texts(&game).len(), 1, "only once");
    }

    #[test]
    fn a_turned_off_superweapon_never_reports_ready() {
        let mut game = in_battle();
        game.skirmish.superweapons = false;
        game.strike_recharge = 1.0;
        game.update(2.0);
        assert!(chat_texts(&game).is_empty());
    }

    #[test]
    fn chat_keeps_only_the_last_few_lines_and_they_age_away() {
        let mut game = in_battle();
        for _ in 0..hud::CHAT_LINES + 2 {
            click_hud(&mut game, Action::BuildCell(0));
        }
        assert_eq!(game.chat.len(), hud::CHAT_LINES);
        game.update(CHAT_SECONDS - 1.0);
        assert_eq!(game.chat.len(), hud::CHAT_LINES, "still there");
        game.update(2.0);
        assert!(game.chat.is_empty(), "gone");
    }

    #[test]
    fn chat_stands_still_while_paused_and_clears_for_a_new_battle() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildCell(0));
        let before = game.chat.clone();
        game.handle(Event::KeyDown(Key::Pause));
        game.update(CHAT_SECONDS * 2.0);
        assert_eq!(game.chat, before);
        game.handle(Event::KeyDown(Key::Pause));
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert!(game.chat.is_empty());
    }

    /// How many units the player's first group has.
    const FIRST_GROUP: usize = 16;

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
        (
            (W / 2.0 - 400.0, H / 2.0 - 400.0),
            (W / 2.0 + 400.0, H / 2.0 + 400.0),
        )
    }

    #[test]
    fn a_box_selects_the_players_units_inside_it() {
        let mut game = in_battle();
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);
        assert_eq!(
            selected_count(&game),
            FIRST_GROUP,
            "the first group, not the far one"
        );
        assert!(
            game.selection_ground().is_none(),
            "the box ends with the release"
        );
    }

    fn first_plane(game: &Game) -> usize {
        game.units
            .iter()
            .position(|u| u.kind == UnitKind::Aircraft)
            .unwrap()
    }

    #[test]
    fn a_box_around_an_aircraft_as_seen_selects_it_as_does_one_around_its_shadow() {
        let game = in_battle();
        let plane = game.units[first_plane(&game)];
        let view = game.view();
        let (x, y, z) = plane.center();
        let seen = view.project(x, y, z).unwrap();
        let shadow = view.project(plane.x, plane.y, plane.z).unwrap();
        assert!(seen.1 < shadow.1 - 40.0, "it appears well above its shadow");
        for around in [seen, shadow] {
            let mut game = in_battle();
            drag_select(
                &mut game,
                (around.0 - 30.0, around.1 - 30.0),
                (around.0 + 30.0, around.1 + 30.0),
            );
            assert!(game.units[first_plane(&game)].selected, "around {around:?}");
        }
    }

    #[test]
    fn clicking_where_an_aircraft_appears_selects_it_and_it_can_then_be_sent() {
        let mut game = in_battle();
        let index = first_plane(&game);
        let (x, y, z) = game.units[index].center();
        let (px, py) = game.view().project(x, y, z).unwrap();
        click_at(&mut game, px, py);
        assert!(game.units[index].selected);
        assert_eq!(selected_count(&game), 1);

        click_at(&mut game, W / 2.0 + 500.0, H / 2.0 + 200.0);
        assert!(game.units[index].is_moving(), "sent there");
    }

    #[test]
    fn aircraft_cast_a_shadow_on_the_ground() {
        let shadows = |game: &Game| {
            crate::render::build(game)
                .iter()
                .filter(|v| v.r == 0.0 && v.g == 0.0 && v.b == 0.0 && v.a == 0.3)
                .count()
        };
        let mut game = in_battle();
        let with_planes = shadows(&game);
        game.units.retain(|u| !u.kind.flies());
        assert!(with_planes > shadows(&game));
    }

    #[test]
    fn statistics_count_aircraft_on_their_own() {
        let mut game = in_battle();
        let planes = game
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.kind == UnitKind::Aircraft)
            .count() as i32;
        for unit in game
            .units
            .iter_mut()
            .filter(|u| u.owner == 0 && u.kind == UnitKind::Aircraft)
        {
            unit.alive = false;
        }
        game.update(0.01);
        assert_eq!(game.lost.aircraft, planes);
        assert_eq!(game.lost.infantry + game.lost.vehicles, 0);
    }

    fn shift_drag_select(game: &mut Game, from: (f32, f32), to: (f32, f32)) {
        game.handle(Event::MouseMove(from.0, from.1));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(to.0, to.1));
        game.handle(Event::ShiftMouseUp);
    }

    fn infantry_on_screen(game: &Game, skip: usize) -> (usize, (f32, f32)) {
        let (index, unit) = game
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.owner == 0 && u.kind == UnitKind::Infantry)
            .nth(skip)
            .unwrap();
        (
            index,
            game.view()
                .project(unit.x, unit.y + units::HEIGHT / 2.0, unit.z)
                .unwrap(),
        )
    }

    /// Puts an enemy on screen beside the first group and says where to point at it.
    fn enemy_beside_first_group(game: &mut Game) -> (usize, (f32, f32)) {
        let index = game.units.iter().position(|u| u.owner == 1).unwrap();
        let (x, z) = (game.camera.x + 6.0, game.camera.z);
        let unit = &mut game.units[index];
        (unit.x, unit.z, unit.destination) = (x, z, (x, z));
        let unit = game.units[index];
        let at = game
            .view()
            .project(unit.x, unit.y + units::HEIGHT / 2.0, unit.z)
            .unwrap();
        assert!(
            !hud::covers(at.0, at.1, W, H) && at.0 > 0.0 && at.0 < W && at.1 > 0.0 && at.1 < H,
            "{at:?}"
        );
        (index, at)
    }

    fn line_vertices(game: &Game, color: [f32; 4]) -> usize {
        let is_color = |v: &&crate::platform::Vertex| **v == Vertex::new((v.x, v.y), color);
        crate::render::build(game).iter().filter(is_color).count()
    }

    #[test]
    fn the_cursor_means_attack_only_over_an_enemy_someone_selected_can_fight() {
        let mut game = in_battle();
        let (enemy, (x, y)) = enemy_beside_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        assert!(!game.cursor_means_attack(), "nobody selected");

        select_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        assert!(game.cursor_means_attack());
        let bullseye = |g: &Game| line_vertices(g, crate::render::ATTACK_CURSOR);
        assert!(bullseye(&game) > 0, "and it shows");

        game.handle(Event::MouseMove(W / 2.0 - 800.0, H / 2.0 + 300.0));
        assert!(!game.cursor_means_attack(), "not over bare ground");
        assert_eq!(bullseye(&game), 0);

        let own = game.units[0];
        let (own_x, own_y) = game
            .view()
            .project(own.x, own.y + own.altitude + 0.25, own.z)
            .unwrap();
        game.handle(Event::MouseMove(own_x, own_y));
        assert!(!game.cursor_means_attack(), "nor over one's own");

        game.handle(Event::MouseMove(x, y));
        game.handle(Event::KeyDown(Key::Tab));
        assert!(
            !game.cursor_means_attack(),
            "nor with the diplomacy window open"
        );
        game.handle(Event::KeyDown(Key::Tab));

        // Everyone selected carries a mortar, and this enemy flies.
        for unit in &mut game.units {
            if unit.selected {
                unit.weapon = crate::weapons::WeaponKind::Mortar;
            }
        }
        assert!(game.cursor_means_attack());
        game.units[enemy].kind = UnitKind::Aircraft;
        game.units[enemy].altitude = 3.0;
        let flier = game.units[enemy];
        let (x, y) = game
            .view()
            .project(flier.x, flier.y + 3.25, flier.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert!(!game.cursor_means_attack(), "mortars cannot aim up");
    }

    #[test]
    fn the_cursor_turns_back_during_a_box_that_starts_on_an_enemy() {
        let mut game = in_battle();
        let (_, (x, y)) = enemy_beside_first_group(&mut game);
        select_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        assert!(game.cursor_means_attack(), "still a click");
        game.handle(Event::MouseMove(x + 300.0, y + 300.0));
        assert!(!game.cursor_means_attack(), "now a box");
    }

    #[test]
    fn a_click_on_an_enemy_sends_the_selected_after_it_and_they_kill_it() {
        let mut game = in_battle();
        let (enemy, (x, y)) = enemy_beside_first_group(&mut game);
        let id = game.units[enemy].id;
        select_first_group(&mut game);
        click_at(&mut game, x, y);
        assert_eq!(selected_count(&game), FIRST_GROUP, "the selection stands");
        assert!(!game.units[enemy].selected);
        let sent = game
            .units
            .iter()
            .filter(|u| u.selected && u.target == Some(crate::units::Target::Unit(id)))
            .count();
        let armed = |u: &&Unit| u.selected && u.weapon != crate::weapons::WeaponKind::Unarmed;
        assert_eq!(sent, game.units.iter().filter(armed).count());
        assert!(sent > 0);

        for _ in 0..600 {
            game.update(0.05);
            if !game.units.iter().any(|u| u.id == id) {
                break;
            }
        }
        assert!(!game.units.iter().any(|u| u.id == id), "it died");
        game.update(0.05);
        assert!(
            game.units
                .iter()
                .all(|u| u.target != Some(crate::units::Target::Unit(id))),
            "and everyone forgot it"
        );
    }

    #[test]
    fn a_click_with_shift_also_attacks() {
        let mut game = in_battle();
        let (enemy, (x, y)) = enemy_beside_first_group(&mut game);
        select_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        game.handle(Event::ShiftMouseUp);
        let id = game.units[enemy].id;
        assert!(
            game.units
                .iter()
                .filter(|u| u.selected && u.weapon != crate::weapons::WeaponKind::Unarmed)
                .all(|u| u.target == Some(crate::units::Target::Unit(id)))
        );
    }

    #[test]
    fn selected_units_show_a_red_line_to_their_target_and_green_to_their_destination() {
        let (red, green) = (crate::render::ATTACK_LINE, crate::render::ORDER_LINE);
        let mut game = in_battle();
        let (_, (x, y)) = enemy_beside_first_group(&mut game);
        select_first_group(&mut game);
        assert_eq!(
            (line_vertices(&game, red), line_vertices(&game, green)),
            (0, 0),
            "no orders yet"
        );

        click_at(&mut game, x, y);
        assert!(line_vertices(&game, red) > 0);
        assert_eq!(line_vertices(&game, green), 0, "red instead of green");

        game.handle(Event::RightDown);
        assert_eq!(line_vertices(&game, red), 0, "deselected shows nothing");
        select_first_group(&mut game);
        assert!(
            line_vertices(&game, red) > 0,
            "selected again, the target shows again"
        );

        click_at(&mut game, W / 2.0 - 800.0, H / 2.0 + 300.0);
        assert_eq!(line_vertices(&game, red), 0);
        assert!(
            line_vertices(&game, green) > 0,
            "a move order turns it green"
        );

        game.handle(Event::RightDown);
        assert_eq!(line_vertices(&game, green), 0);
        select_first_group(&mut game);
        assert!(
            line_vertices(&game, green) > 0,
            "selected again, the destination shows again"
        );
    }

    #[test]
    fn a_unit_in_range_of_its_target_still_shows_the_red_line() {
        let mut game = in_battle();
        let (enemy, (x, y)) = enemy_beside_first_group(&mut game);
        select_first_group(&mut game);
        click_at(&mut game, x, y);
        game.units[enemy].health = 100_000.0;
        for _ in 0..1200 {
            game.update(0.05);
        }
        let moving: Vec<_> = game
            .units
            .iter()
            .filter(|u| u.selected && u.is_moving())
            .collect();
        assert!(moving.is_empty(), "{moving:#?}");
        assert!(line_vertices(&game, crate::render::ATTACK_LINE) > 0);
    }

    fn tower_of(game: &Game, owner: usize) -> usize {
        game.units
            .iter()
            .position(|u| u.owner == owner && u.role == Role::Tower)
            .unwrap()
    }

    #[test]
    fn the_players_tower_cannot_be_selected_but_stands_in_the_box() {
        let mut game = in_battle();
        let tower = tower_of(&game, 0);
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);
        assert!(!game.units[tower].selected);

        game.handle(Event::RightDown);
        let unit = game.units[tower];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        click_at(&mut game, x, y);
        assert_eq!(selected_count(&game), 0);
    }

    #[test]
    fn an_enemy_tower_can_be_clicked_and_destroyed_and_counts_as_a_building() {
        let mut game = in_battle();
        let tower = tower_of(&game, 1);
        let (x, z) = (game.camera.x + 6.0, game.camera.z);
        (
            game.units[tower].x,
            game.units[tower].z,
            game.units[tower].destination,
        ) = (x, z, (x, z));
        let id = game.units[tower].id;
        select_first_group(&mut game);
        let unit = game.units[tower];
        let (px, py) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(px, py));
        assert!(game.cursor_means_attack());
        click_at(&mut game, px, py);
        assert!(
            game.units
                .iter()
                .filter(|u| u.selected && u.weapon != crate::weapons::WeaponKind::Unarmed)
                .all(|u| u.target == Some(crate::units::Target::Unit(id)))
        );

        for _ in 0..4000 {
            game.update(0.05);
            if !game.units.iter().any(|u| u.id == id) {
                break;
            }
        }
        assert!(!game.units.iter().any(|u| u.id == id), "it fell");
        assert_eq!(game.killed.buildings, 1);
    }

    #[test]
    fn a_battle_is_not_won_while_an_enemy_tower_stands_nor_lost_while_ones_own_does() {
        let mut game = in_battle();
        let enemies = all_enemies(&game);
        for unit in game
            .units
            .iter_mut()
            .filter(|u| enemies.contains(&u.owner) && !u.kind.is_building())
        {
            unit.alive = false;
        }
        game.update(0.01);
        assert_eq!(game.outcome, None, "towers still stand");

        let mut game = in_battle();
        for unit in game
            .units
            .iter_mut()
            .filter(|u| u.owner == 0 && !u.kind.is_building())
        {
            unit.alive = false;
        }
        game.update(0.01);
        assert_eq!(game.outcome, None, "the player's tower still stands");
    }

    #[test]
    fn each_infantry_button_makes_its_unit_at_once_beside_the_tower_in_the_players_color() {
        let names = [
            "Scout",
            "Light Infantry",
            "Sniper",
            "Engineer",
            "K Infantry",
        ];
        let mut game = in_battle();
        for (cell, name) in names.into_iter().enumerate() {
            let before = game.units.len();
            click_hud(&mut game, Action::BuildCell(cell));
            assert_eq!(game.units.len(), before + 1, "{name}");
            let new = game.units[before];
            assert_eq!(new.name(), name);
            assert_eq!(new.owner, 0);
            assert_eq!(new.color, game.units[0].color);
            let tower = game.units[tower_of(&game, 0)];
            assert!((new.x - tower.x).hypot(new.z - tower.z) < 8.0, "{new:?}");
            assert!(
                game.units[..before].iter().all(|u| u.id != new.id),
                "its id is its own"
            );
        }
    }

    #[test]
    fn each_vehicle_button_makes_its_vehicle_and_the_superweapon_comes_after_them() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        for (cell, name) in [
            "Tank",
            "Repair Vehicle",
            "Catapult",
            "Rocket Launcher",
            "Strider",
            "Heavy Strider",
        ]
        .into_iter()
        .enumerate()
        {
            let before = game.units.len();
            click_hud(&mut game, Action::BuildCell(cell));
            assert_eq!(game.units.len(), before + 1);
            assert_eq!(game.units[before].name(), name);
            assert_eq!(game.units[before].kind, UnitKind::Vehicle);
        }
        assert!(!game.aiming_strike);
        let before = game.units.len();
        click_hud(&mut game, Action::BuildCell(SUPERWEAPON_CELL));
        assert!(game.aiming_strike);
        assert_eq!(game.units.len(), before);
    }

    #[test]
    fn every_vehicle_shines_white_light_ahead_of_it_and_nothing_else_does() {
        let lights = |game: &Game| {
            crate::render::build(game)
                .iter()
                .filter(|v| v.a == 0.0 && v.r > 0.0)
                .inspect(|v| assert_eq!((v.r, v.g), (v.b, v.b), "white"))
                .count()
        };
        let mut game = in_battle();
        // Rocket engines are lit too, in orange.
        game.units.retain(|u| !u.kind.flies());
        let vehicles = |game: &Game| {
            let is_vehicle = |u: &&Unit| u.alive && u.kind == UnitKind::Vehicle;
            game.units.iter().filter(is_vehicle).count()
        };
        assert!(vehicles(&game) > 0);
        assert_eq!(lights(&game), vehicles(&game), "one bright corner each");
        game.units.retain(|u| u.kind != UnitKind::Vehicle);
        assert_eq!(lights(&game), 0);
    }

    #[test]
    fn an_air_defense_platform_has_two_lamps_of_two_triangles_each_and_the_drone_a_flame() {
        let lit = |game: &Game| {
            crate::render::build(game)
                .iter()
                .filter(|v| v.a == 0.0 && v.r > 0.0)
                .count()
        };
        let mut game = in_battle();
        game.units
            .retain(|u| matches!(u.kind, UnitKind::Infantry | UnitKind::Building));
        assert_eq!(lit(&game), 0);
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_hud(&mut game, Action::BuildCell(BOTTOM_ROW_START));
        assert_eq!(game.units.last().unwrap().name(), "Air Defense Platform");
        assert_eq!(
            lit(&game),
            2 * 3,
            "two near corners, shared, in each of two"
        );
        click_hud(&mut game, Action::BuildCell(BOTTOM_ROW_START + 1));
        assert_eq!(game.units.last().unwrap().name(), "Air Drone");
        assert_eq!(lit(&game), 2 * 3 + 2);
        click_hud(&mut game, Action::BuildCell(BOTTOM_ROW_START + 2));
        assert_eq!(game.units.last().unwrap().name(), "Ornithopter");
        assert_eq!(game.units.last().unwrap().altitude, 0.0);
    }

    #[test]
    fn a_selected_carryall_shows_the_carry_cursor_over_a_vehicle_and_where_it_can_put_one_down() {
        let mut game = in_battle();
        let carryall = game
            .units
            .iter()
            .position(|u| u.role == Role::AdvancedCarryall && u.owner == 0);
        let carryall = carryall.expect("the player starts with some");
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Tank)
            .unwrap();
        let unit = game.units[tank];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "nothing selected");
        game.units[carryall].selected = true;
        assert_eq!(game.cursor_look(), CursorLook::Carry);
        click_at(&mut game, x, y);
        assert_eq!(
            game.units[carryall].target,
            Some(crate::units::Target::Unit(unit.id))
        );

        let carrier = game.units[carryall];
        game.units.retain(|u| u.id == carrier.id);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "nothing in hand");
        game.units[0].carrying = true;
        assert_eq!(game.cursor_look(), CursorLook::Carry);
        assert!(!crate::render::build(&game).is_empty());
    }

    #[test]
    fn a_vehicle_carried_when_its_carryall_is_lost_is_counted_lost_too() {
        let mut game = in_battle();
        let tank = game
            .units
            .iter()
            .find(|u| u.owner == 0 && u.role == Role::Tank)
            .unwrap();
        let (tank_id, at) = (tank.id, (tank.x, tank.z));
        let carryall = game
            .units
            .iter()
            .position(|u| u.role == Role::AdvancedCarryall && u.owner == 0);
        let carryall = carryall.unwrap();
        game.units[carryall].x = at.0;
        game.units[carryall].z = at.1;
        game.units[carryall].destination = at;
        game.units[carryall].target = Some(crate::units::Target::Unit(tank_id));
        game.update(0.05);
        assert_eq!(game.carried.len(), 1);
        assert!(!game.units.iter().any(|u| u.id == tank_id));
        assert!(!crate::render::build(&game).is_empty(), "it draws, hanging");

        let carrier_id = game.carried[0].carrier;
        game.units
            .iter_mut()
            .find(|u| u.id == carrier_id)
            .unwrap()
            .alive = false;
        game.update(0.05);
        assert!(game.carried.is_empty());
        assert!(!game.units.iter().any(|u| u.id == tank_id));
        assert_eq!(game.lost.aircraft, 1);
        assert_eq!(game.lost.vehicles, 1);
    }

    #[test]
    fn without_its_building_a_button_makes_nothing_and_its_tip_says_why() {
        let mut game = in_battle();
        game.units.retain(|u| u.role != Role::Barracks);
        let before = game.units.len();
        click_hud(&mut game, Action::BuildTab(BuildTab::Infantry));
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(game.units.len(), before);
        let b = hud::grid_cells(W, H)[0];
        game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
        assert_eq!(game.tip(), Some("Needs a Barracks"));
        assert!(!game.chat.iter().any(|line| line.text == "Scout ready"));
        assert!(!crate::render::build(&game).is_empty());

        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(game.units.len(), before + 1, "the factory stands");
    }

    #[test]
    fn taking_a_factory_lets_the_player_make_vehicles_again() {
        let mut game = in_battle();
        game.units
            .retain(|u| u.role != Role::Factory || u.owner != 0);
        assert!(!game.can_produce(Role::Tank));
        let theirs = game
            .units
            .iter_mut()
            .find(|u| u.role == Role::Factory)
            .unwrap();
        theirs.owner = 0;
        assert!(game.can_produce(Role::Tank));
    }

    #[test]
    fn striders_draw_at_rest_on_the_move_and_with_their_cannons_recoiling() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_hud(&mut game, Action::BuildCell(4));
        click_hud(&mut game, Action::BuildCell(5));
        assert!(!crate::render::build(&game).is_empty());
        let strider = game.units.last_mut().unwrap();
        assert_eq!(strider.role, Role::HeavyStrider);
        strider.recoil = [1.0, 0.5, 0.0, 0.0];
        strider.stride = 1.3;
        strider.destination.0 += 5.0;
        strider.selected = true;
        assert!(!crate::render::build(&game).is_empty());
    }

    #[test]
    fn a_knee_lies_ahead_of_the_straight_line_from_hip_to_foot_and_the_limbs_keep_their_length() {
        let (thigh, shin) = (0.5, 0.5);
        let hip = (0.0, 1.0);
        let foot = (0.2, 0.2);
        let knee = crate::render::knee_of(hip, foot, thigh, shin);
        let length = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).hypot(a.1 - b.1);
        assert!((length(hip, knee) - thigh).abs() < 0.001);
        assert!((length(foot, knee) - shin).abs() < 0.001);
        assert!(knee.0 > 0.1, "it bends forward: {knee:?}");
        // Too far to reach, the leg is simply straight.
        let knee = crate::render::knee_of(hip, (0.0, -5.0), thigh, shin);
        assert!(
            (knee.0).abs() < 0.001 && (knee.1 - 0.5).abs() < 0.001,
            "{knee:?}"
        );
    }

    #[test]
    fn other_build_cells_still_make_nothing_nor_does_a_finished_battle() {
        let mut game = in_battle();
        let before = game.units.len();
        click_hud(&mut game, Action::BuildCell(7));
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(game.units.len(), before);

        let mut game = in_battle();
        let enemies = all_enemies(&game);
        kill_owners(&mut game, &enemies);
        assert!(game.outcome.is_some());
        let before = game.units.len();
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(game.units.len(), before);
    }

    #[test]
    fn new_units_can_be_selected_and_sent_at_once() {
        let mut game = in_battle();
        let before = game.units.len();
        click_hud(&mut game, Action::BuildCell(1));
        let new = game.units[before];
        let (x, y) = game
            .view()
            .project(new.x, new.pick_height(), new.z)
            .unwrap();
        click_at(&mut game, x, y);
        assert!(game.units[before].selected);
        click_at(&mut game, W / 2.0 - 800.0, H / 2.0 + 300.0);
        assert!(game.units[before].is_moving());
    }

    fn select_new(game: &mut Game, cell: usize) -> usize {
        let index = game.units.len();
        click_hud(game, Action::BuildCell(cell));
        let new = game.units[index];
        let (x, y) = game
            .view()
            .project(new.x, new.pick_height(), new.z)
            .unwrap();
        click_at(game, x, y);
        assert!(game.units[index].selected);
        index
    }

    #[test]
    fn the_deploy_button_and_the_d_key_switch_selected_k_infantry() {
        let mut game = in_battle();
        let k = select_new(&mut game, 4);
        click_hud(&mut game, Action::HudWide(DEPLOY_BUTTON));
        assert!(game.units[k].deployed);
        game.handle(Event::KeyDown(Key::Char('d')));
        assert!(!game.units[k].deployed);
    }

    #[test]
    fn a_scout_alone_never_turns_the_cursor_into_a_bullseye() {
        let mut game = in_battle();
        let (_, (x, y)) = enemy_beside_first_group(&mut game);
        select_new(&mut game, 0);
        game.handle(Event::MouseMove(x, y));
        assert!(!game.cursor_means_attack());
    }

    #[test]
    fn an_engineer_really_captures_an_enemy_tower_which_then_wins_the_battle() {
        let mut game = in_setup();
        let ai = slider(&game, SliderId::AiPlayers);
        drag(&mut game, SliderId::AiPlayers, ai.x_of(1));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.units
            .retain(|u| !(u.owner == 1 && u.kind == UnitKind::Building));
        for unit in game
            .units
            .iter_mut()
            .filter(|u| u.owner == 1 && u.kind != UnitKind::Tower)
        {
            unit.alive = false;
        }
        game.units
            .retain(|u| u.owner != 1 || u.role != Role::FlameTower);
        let tower = tower_of(&game, 1);
        // Disarmed, tough enough to outlast the player's own fire, and moved into view.
        game.units[tower].cooldown = 1e9;
        game.units[tower].health = 1e9;
        let (x, z) = (game.camera.x + 6.0, game.camera.z);
        (
            game.units[tower].x,
            game.units[tower].z,
            game.units[tower].destination,
        ) = (x, z, (x, z));
        game.update(0.01);
        let id = game.units.iter().find(|u| u.owner == 1).unwrap().id;
        let engineer = select_new(&mut game, 3);
        let engineer_id = game.units[engineer].id;
        let enemy = game.units.iter().position(|u| u.id == id).unwrap();
        // On the far side from the player's group, so that nobody stands in the way.
        let (x, z) = (game.units[engineer].x - 5.0, game.units[engineer].z);
        (
            game.units[enemy].x,
            game.units[enemy].z,
            game.units[enemy].destination,
        ) = (x, z, (x, z));
        let unit = game.units[enemy];
        let (px, py) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        click_at(&mut game, px, py);
        for _ in 0..800 {
            game.update(0.05);
            if game.outcome.is_some() {
                break;
            }
        }
        assert_eq!(game.outcome, Some(Outcome::Victory));
        let captured = game.units.iter().find(|u| u.id == id).unwrap();
        let engineer = game.units.iter().find(|u| u.id == engineer_id).unwrap();
        assert_eq!((captured.owner, captured.color), (0, engineer.color));
        assert_eq!(game.killed.buildings, 0, "taken, not destroyed");
    }

    #[test]
    fn the_bottom_of_the_vehicles_menu_makes_the_air_defense_platform_the_drone_and_the_ornithopter()
     {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        for (cell, name) in [
            (12, "Air Defense Platform"),
            (13, "Air Drone"),
            (14, "Ornithopter"),
        ] {
            let before = game.units.len();
            click_hud(&mut game, Action::BuildCell(cell));
            assert_eq!(game.units[before].name(), name);
            assert_eq!(game.units[before].kind, UnitKind::Aircraft);
        }
        assert!(!crate::render::build(&game).is_empty());
    }

    #[test]
    fn the_vehicles_menu_makes_the_advanced_carryall_above_the_bottom_row() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        let before = game.units.len();
        click_hud(&mut game, Action::BuildCell(CARRYALL_CELL));
        assert_eq!(game.units[before].name(), "Advanced Carryall");
        assert!(game.units[before].altitude > 0.0, "it hangs in the air");
    }

    #[test]
    fn the_infantry_menu_also_makes_flame_and_gas_infantry() {
        let mut game = in_battle();
        for (cell, name) in [(5, "Flame Infantry"), (6, "Gas Infantry")] {
            let before = game.units.len();
            click_hud(&mut game, Action::BuildCell(cell));
            assert_eq!(game.units[before].name(), name);
        }
    }

    #[test]
    fn fire_and_gas_draw_without_trouble() {
        let mut game = in_battle();
        let (x, z) = (game.camera.x, game.camera.z);
        for gas in [false, true] {
            game.combat.sprays.push(crate::weapons::Spray {
                from: (x, 1.0, z),
                heading: 0.5,
                reach: 4.5,
                half_angle: 0.3,
                gas,
                life: 0.5,
            });
        }
        assert!(!crate::render::build(&game).is_empty());
    }

    #[test]
    fn tip_text_names_units_under_the_cursor_and_what_buttons_make() {
        let mut game = in_battle();
        let cell = |i: usize| {
            hud::all_buttons(W, H)
                .into_iter()
                .find(|b| b.action == Action::BuildCell(i))
                .unwrap()
        };
        for (i, name) in [
            "Scout",
            "Light Infantry",
            "Sniper",
            "Engineer",
            "K Infantry",
        ]
        .into_iter()
        .enumerate()
        {
            let b = cell(i);
            game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
            assert_eq!(game.tip(), Some(name));
        }
        let b = cell(7);
        game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
        assert_eq!(game.tip(), None, "that cell makes nothing");

        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        for (i, name) in [
            "Tank",
            "Repair Vehicle",
            "Catapult",
            "Rocket Launcher",
            "Strider",
            "Heavy Strider",
            "Superweapon",
        ]
        .into_iter()
        .enumerate()
        {
            let b = cell(i);
            game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
            assert_eq!(game.tip(), Some(name));
        }

        let tower = game.units[tower_of(&game, 0)];
        let (x, y) = game
            .view()
            .project(tower.x, tower.pick_height(), tower.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.tip(), Some("Tower"), "one's own");
        let (_, (x, y)) = enemy_beside_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.tip(), Some("Light Infantry"), "and enemies");
        assert!(!crate::render::build(&game).is_empty());

        game.handle(Event::MouseMove(W / 2.0 - 800.0, H / 2.0 + 300.0));
        assert_eq!(game.tip(), None, "bare ground");
    }

    #[test]
    fn the_tool_tips_option_turns_tip_text_off() {
        let mut game = in_battle();
        let tower = game.units[tower_of(&game, 0)];
        let (x, y) = game
            .view()
            .project(tower.x, tower.pick_height(), tower.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert!(game.tip().is_some());
        game.flip(Toggle::ToolTips);
        assert_eq!(game.tip(), None);
    }

    #[test]
    fn the_a_key_selects_all_the_players_units_anywhere_on_the_map() {
        let mut game = in_battle();
        let everyone = game.units.iter().filter(|u| u.is_selectable()).count();
        assert!(everyone > FIRST_GROUP, "including the group off screen");
        game.handle(Event::KeyDown(Key::Char('a')));
        assert_eq!(selected_count(&game), everyone);
        assert!(
            game.units
                .iter()
                .filter(|u| u.selected)
                .all(|u| u.owner == 0),
            "no enemies"
        );
        assert!(
            game.units
                .iter()
                .filter(|u| u.selected)
                .all(|u| !u.kind.is_building()),
            "no towers"
        );

        game.handle(Event::RightDown);
        game.handle(Event::KeyDown(Key::Char('A')));
        assert_eq!(
            selected_count(&game),
            everyone,
            "with Shift or Caps Lock too"
        );
    }

    #[test]
    fn the_a_key_does_nothing_while_paused_in_menus_or_typing_a_name() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Pause));
        game.handle(Event::KeyDown(Key::Char('a')));
        assert_eq!(selected_count(&game), 0, "paused");

        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Escape));
        game.handle(Event::KeyDown(Key::Char('a')));
        assert_eq!(selected_count(&game), 0, "in the menu");
    }

    #[test]
    fn the_main_menu_shows_the_version_and_a_planet_that_carries_on_between_its_screens() {
        let mut game = in_menu();
        let plain_options = {
            let mut other = in_options();
            other.update(0.0);
            crate::render::build(&other).len()
        };
        let drawn = crate::render::build(&game);
        assert!(drawn.len() > plain_options + 10_000, "the planet shows");
        let top_left = |v: &&crate::platform::Vertex| v.x < 200.0 && v.y < 100.0;
        let text = [0.93, 0.89, 0.78, 1.0];
        assert!(
            drawn
                .iter()
                .filter(top_left)
                .any(|v| [v.r, v.g, v.b, v.a] == text),
            "the version shows"
        );

        game.update(5.0);
        let before = game.menu_seconds();
        click(&mut game, Action::SinglePlayer);
        assert_eq!(
            game.menu_seconds(),
            before,
            "no jump on the way to the submenu"
        );
        assert!(
            crate::render::build(&game).len() > plain_options + 10_000,
            "the planet stays"
        );
    }

    /// Whether a vertex shows this color. One fully clear shows no color at all.
    fn shows_color(v: &Vertex, color: [f32; 3]) -> bool {
        v.a > 0.0 && [v.r, v.g, v.b] == color.map(|c| c * v.a)
    }

    fn alphas_of(game: &Game, color: [f32; 3]) -> Vec<f32> {
        crate::render::build(game)
            .iter()
            .filter(|v| shows_color(v, color))
            .map(|v| v.a)
            .collect()
    }

    #[test]
    fn buttons_are_blue_fading_sideways_between_orange_lines_that_only_half_fade() {
        use crate::render::{BANNER_BLUE, BANNER_LINE, BANNER_LINE_END_ALPHA};
        let mut game = in_menu();
        game.handle(Event::MouseMove(1.0, 1.0));
        let fill = alphas_of(&game, BANNER_BLUE);
        assert!(fill.contains(&1.0), "opaque in the middle");
        let lines = alphas_of(&game, BANNER_LINE);
        assert!(lines.contains(&1.0), "lines are solid in the middle");
        assert!(
            lines.contains(&BANNER_LINE_END_ALPHA),
            "and semi-opaque at the ends"
        );
        assert!(lines.iter().all(|&alpha| alpha >= BANNER_LINE_END_ALPHA));

        let vertices = crate::render::build(&game);
        let b = buttons(game.screen, W, H)[0];
        let at = |x: f32, y: f32| {
            vertices
                .iter()
                .find(|v| {
                    (v.a == 0.0 || shows_color(v, BANNER_BLUE))
                        && (v.x - x).abs() < 0.5
                        && (v.y - y).abs() < 0.5
                })
                .map(|v| v.a)
        };
        assert_eq!(at(b.x, b.y), Some(0.0), "the left end");
        assert_eq!(at(b.x + b.w, b.y), Some(0.0), "the right end");
        assert_eq!(at(b.x + b.w / 2.0, b.y), Some(1.0), "the middle");
    }

    #[test]
    fn a_button_under_the_cursor_changes_its_band() {
        use crate::render::BANNER_BLUE;
        let mut game = in_menu();
        game.handle(Event::MouseMove(1.0, 1.0));
        let quiet = alphas_of(&game, BANNER_BLUE).len();
        let b = buttons(game.screen, W, H)[0];
        game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
        assert!(alphas_of(&game, BANNER_BLUE).len() < quiet);
    }

    #[test]
    fn the_sidebars_small_square_buttons_stay_solid() {
        use crate::render::BANNER_BLUE;
        let mut game = in_battle();
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        let drawn = crate::render::build(&game);
        let wide = hud::command_buttons(W, H)
            .into_iter()
            .filter(|b| matches!(b.action, Action::HudWide(_)))
            .count();
        let bands = drawn.iter().filter(|v| shows_color(v, BANNER_BLUE)).count();
        // Half of a band's twelve corners are its clear ends.
        assert_eq!(bands, wide * 6, "only the wide ones have a band");
    }

    #[test]
    fn the_planet_stays_off_the_escape_menu_over_a_battle() {
        let sand = |game: &Game| {
            crate::render::build(game)
                .iter()
                .filter(|v| v.a == 1.0 && v.r > v.g && v.g > v.b && v.b > 0.09 && v.r < 0.7)
                .count()
        };
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::EscapeMenu);
        assert!(sand(&in_menu()) > sand(&game), "only the main menu has it");
    }

    fn clear_spot_on_screen() -> (f32, f32) {
        (W / 2.0 - 800.0, H / 2.0 + 300.0)
    }

    #[test]
    fn control_turns_the_cursor_red_over_bare_ground_only_for_those_able_to_fire() {
        let mut game = in_battle();
        let (x, y) = clear_spot_on_screen();
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::Control(true));
        assert!(!game.cursor_means_attack(), "nobody selected");

        select_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        assert!(game.cursor_means_attack());
        game.handle(Event::Control(false));
        assert!(!game.cursor_means_attack(), "let go, and it turns back");

        game.handle(Event::Control(true));
        game.handle(Event::Focus(false));
        assert!(!game.cursor_means_attack(), "losing focus lets go of it");

        // Only a scout selected.
        let mut game = in_battle();
        select_new(&mut game, 0);
        game.handle(Event::Control(true));
        game.handle(Event::MouseMove(x, y));
        assert!(!game.cursor_means_attack(), "unarmed");
    }

    #[test]
    fn a_control_click_on_bare_ground_attacks_that_place_with_a_red_line_and_no_green() {
        let mut game = in_battle();
        select_first_group(&mut game);
        let (x, y) = clear_spot_on_screen();
        let place = game.view().ground_at(x, y).unwrap();
        game.handle(Event::Control(true));
        click_at(&mut game, x, y);
        for unit in game
            .units
            .iter()
            .filter(|u| u.selected && u.weapon != crate::weapons::WeaponKind::Unarmed)
        {
            assert_eq!(
                unit.target,
                Some(crate::units::Target::Place(place.0, place.1))
            );
        }
        assert_eq!(selected_count(&game), FIRST_GROUP);
        game.update(0.05);
        assert!(line_vertices(&game, crate::render::ATTACK_LINE) > 0);
        assert_eq!(line_vertices(&game, crate::render::ORDER_LINE), 0);

        game.handle(Event::RightDown);
        assert_eq!(line_vertices(&game, crate::render::ATTACK_LINE), 0);
        select_first_group(&mut game);
        assert!(
            line_vertices(&game, crate::render::ATTACK_LINE) > 0,
            "it shows again"
        );
    }

    #[test]
    fn a_control_click_on_a_friend_makes_it_the_target_and_its_death_frees_everyone() {
        let mut game = in_battle();
        let (friend_index, _) = infantry_on_screen(&game, 7);
        let friend = game.units[friend_index];
        let (x, y) = game
            .view()
            .project(friend.x, friend.pick_height(), friend.z)
            .unwrap();
        select_first_group(&mut game);
        game.units[friend_index].selected = false;
        game.handle(Event::Control(true));
        game.handle(Event::MouseMove(x, y));
        assert!(game.cursor_means_attack());
        click_at(&mut game, x, y);
        game.handle(Event::Control(false));
        assert!(!game.units[friend_index].selected, "not selected, targeted");
        let forced = crate::units::Target::Forced(friend.id);
        let on_it = game
            .units
            .iter()
            .filter(|u| u.target == Some(forced))
            .count();
        assert!(on_it > 0, "the selected turn on it");

        for _ in 0..1200 {
            game.update(0.05);
            if !game.units.iter().any(|u| u.id == friend.id) {
                break;
            }
        }
        assert!(!game.units.iter().any(|u| u.id == friend.id), "it dies");
        game.update(0.05);
        assert!(
            game.units.iter().all(|u| u.target != Some(forced)),
            "and everyone is free"
        );
    }

    #[test]
    fn a_selected_repair_vehicle_shows_the_heal_cursor_over_a_friendly_vehicle_and_goes_there() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Tank)
            .unwrap();
        let fixer = select_new(&mut game, 1);
        game.units[tank].health = 50.0;
        let unit = game.units[tank];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(
            game.cursor_look(),
            CursorLook::Heal,
            "not the attack cursor, and without Control"
        );
        assert!(
            line_vertices(&game, crate::render::HEAL_CURSOR) > 0,
            "and it shows"
        );
        assert_eq!(line_vertices(&game, crate::render::ATTACK_CURSOR), 0);
        click_at(&mut game, x, y);
        assert_eq!(
            game.units[fixer].target,
            Some(crate::units::Target::Unit(unit.id))
        );
        assert!(game.units[fixer].selected, "the selection stands");
        assert!(!game.units[tank].selected);
        assert!(
            line_vertices(&game, crate::render::HEAL_LINE) > 0,
            "a blue line leads there"
        );
        assert_eq!(line_vertices(&game, crate::render::ATTACK_LINE), 0);
    }

    #[test]
    fn a_selected_repair_vehicle_changes_no_cursor_over_infantry_enemies_or_itself() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        let fixer = select_new(&mut game, 1);
        let own = game.units[fixer];
        let (x, y) = game
            .view()
            .project(own.x, own.pick_height(), own.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "itself");

        let (_, (x, y)) = infantry_on_screen(&game, 7);
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "infantry");

        let (_, (x, y)) = enemy_beside_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "an enemy");
    }

    #[test]
    fn with_armed_units_selected_too_only_control_turns_a_friendly_vehicle_red() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Tank)
            .unwrap();
        let fixer = select_new(&mut game, 1);
        let gunner = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .unwrap();
        game.units[gunner].selected = true;
        assert!(game.units[fixer].selected);
        let unit = game.units[tank];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Heal);
        game.handle(Event::Control(true));
        assert_eq!(game.cursor_look(), CursorLook::Attack);
    }

    #[test]
    fn medals_show_one_v_for_each_rank_on_friend_and_foe() {
        let medals = |game: &Game| line_vertices(game, crate::render::MEDAL);
        let mut game = in_battle();
        assert_eq!(medals(&game), 0, "no medal at first");
        let (enemy, _) = enemy_beside_first_group(&mut game);
        for (experience, rank) in [(2, 1), (5, 2), (10, 3)] {
            game.units[0].experience = experience;
            assert_eq!(game.units[0].rank(), rank);
            assert_eq!(
                medals(&game),
                12 * rank as usize,
                "two strokes of two triangles each"
            );
        }
        game.units[0].experience = 0;
        game.units[enemy].experience = 2;
        assert_eq!(medals(&game), 12);
    }

    #[test]
    fn a_kill_in_a_real_battle_earns_a_medal() {
        let mut game = in_battle();
        let (enemy, _) = enemy_beside_first_group(&mut game);
        let id = game.units[enemy].id;
        game.units[enemy].health = 1.0;
        for _ in 0..200 {
            game.update(0.05);
            if !game.units.iter().any(|u| u.id == id) {
                break;
            }
        }
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .any(|u| u.experience > 0)
        );
    }

    fn in_fog_battle() -> Game {
        let mut game = in_setup();
        click_widget(&mut game, Action::Toggle(Toggle::FogOfWar));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game
    }

    fn drawn_black(game: &Game) -> usize {
        crate::render::build(game)
            .iter()
            .filter(|v| [v.r, v.g, v.b, v.a] == [0.0, 0.0, 0.0, 1.0])
            .count()
    }

    #[test]
    fn fog_follows_the_setup_checkbox_in_a_skirmish_and_is_always_on_elsewhere() {
        assert!(in_battle().fog.is_none(), "off by default");
        assert!(in_fog_battle().fog.is_some());
        let mut game = in_menu();
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert!(game.fog.is_some());
    }

    #[test]
    fn without_fog_everything_shows_and_with_it_the_unexplored_is_black() {
        let plain = in_battle();
        assert_eq!(plain.sight_at(30.0, 30.0), Sight::Visible);
        assert!(plain.units.iter().all(|u| plain.shows(u)));

        let fog = in_fog_battle();
        let tower = fog.units[tower_of(&fog, 1)];
        assert_eq!(fog.sight_at(tower.x, tower.z), Sight::Unexplored);
        assert!(!fog.shows(&tower));
        assert!(
            drawn_black(&fog) > drawn_black(&plain),
            "the unknown is drawn black"
        );
    }

    #[test]
    fn the_player_starts_seeing_around_their_own_units_and_allies_share_what_they_see() {
        let game = in_fog_battle();
        for unit in game.units.iter().filter(|u| u.owner == 0) {
            assert_eq!(game.sight_at(unit.x, unit.z), Sight::Visible, "{unit:?}");
            assert!(game.shows(unit));
        }

        let mut game = in_fog_battle();
        game.teams[1] = game.teams[0];
        game.update(0.01);
        let ally = game.units.iter().find(|u| u.owner == 1).unwrap();
        assert_eq!(game.sight_at(ally.x, ally.z), Sight::Visible);
        assert!(game.shows(ally));
    }

    #[test]
    fn hidden_enemies_are_not_drawn_named_or_targeted_until_seen() {
        let mut game = in_fog_battle();
        let enemy = game.units.iter().position(|u| u.owner == 1).unwrap();
        let drawn_with = crate::render::build(&game).len();
        let (id, unit) = (game.units[enemy].id, game.units.remove(enemy));
        assert_eq!(
            crate::render::build(&game).len(),
            drawn_with,
            "not on the map or the minimap"
        );
        game.units.insert(enemy, unit);

        select_first_group(&mut game);
        let (x, z) = (game.camera.x + 6.0, game.camera.z);
        let unit = &mut game.units[enemy];
        (unit.x, unit.z, unit.destination, unit.cooldown) = (x, z, (x, z), 1e9);
        let unit = game.units[enemy];
        assert_eq!(unit.id, id);
        let (px, py) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(px, py));
        assert!(game.shows(&unit), "in view, it can be pointed at");
        assert!(game.cursor_means_attack());
        assert_eq!(game.tip(), Some("Light Infantry"));
        assert!(
            crate::render::build(&game).len() > drawn_with,
            "and it is drawn"
        );
    }

    #[test]
    fn an_enemy_in_land_the_player_only_remembers_does_not_show_and_cannot_be_clicked() {
        let mut game = in_fog_battle();
        game.units
            .retain(|u| u.owner != 0 || u.kind == UnitKind::Vehicle);
        let keep = game.units.iter().position(|u| u.owner == 0).unwrap();
        let (x, z) = (game.camera.x, game.camera.z);
        game.units[keep].cooldown = 1e9;
        game.update(0.01);
        assert_eq!(game.sight_at(x, z), Sight::Visible);

        // Off it goes, and an enemy walks in behind it.
        let kept = game.units[keep].id;
        game.units.retain(|u| u.owner != 0 || u.id == kept);
        let keep = game.units.iter().position(|u| u.owner == 0).unwrap();
        (game.units[keep].x, game.units[keep].destination.0) = (x + 40.0, x + 40.0);
        let enemy = game.units.iter().position(|u| u.owner == 1).unwrap();
        (
            game.units[enemy].x,
            game.units[enemy].z,
            game.units[enemy].destination,
        ) = (x, z, (x, z));
        game.units[enemy].cooldown = 1e9;
        game.update(0.01);
        assert_eq!(game.sight_at(x, z), Sight::Explored);
        let unit = game.units[enemy];
        assert!(!game.shows(&unit));

        game.units[keep].selected = true;
        let (px, py) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(px, py));
        assert!(!game.cursor_means_attack());
        assert_eq!(game.tip(), None);
    }

    #[test]
    fn moving_units_uncover_more_of_the_map() {
        let mut game = in_fog_battle();
        let (far_x, far_z) = (game.camera.x - 25.0, game.camera.z);
        assert_eq!(game.sight_at(far_x, far_z), Sight::Unexplored);
        let scout = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Aircraft)
            .unwrap();
        game.units[scout].destination = (far_x, far_z);
        for _ in 0..200 {
            game.update(0.05);
        }
        assert_eq!(
            game.sight_at(far_x, far_z),
            Sight::Visible,
            "explored by flying there"
        );
    }

    #[test]
    fn fog_draws_without_trouble_with_effects_in_and_out_of_view() {
        let mut game = in_fog_battle();
        let (x, z) = (game.camera.x, game.camera.z);
        for (x, z) in [(x, z), (0.0, 5.0)] {
            game.combat
                .poison
                .push(crate::weapons::PoisonZone { x, z, life: 30.0 });
            game.combat.blasts.push(crate::weapons::Blast {
                at: (x, 0.0, z),
                radius: 2.0,
                life: 0.5,
            });
            game.combat.flares.push(crate::weapons::Flare {
                at: (x, 1.0, z),
                size: 7.0,
                life: 0.5,
            });
        }
        assert!(!crate::render::build(&game).is_empty());
    }

    #[test]
    fn a_box_with_shift_adds_to_the_selection() {
        let mut game = in_battle();
        let (first, _) = infantry_on_screen(&game, 0);
        let (last, _) = infantry_on_screen(&game, 7);
        // Boxes go by the spot on the ground where a unit stands.
        let view = game.view();
        let around = |index: usize| {
            let unit = &game.units[index];
            let (x, y) = view.project(unit.x, unit.y, unit.z).unwrap();
            ((x - 40.0, y - 40.0), (x + 40.0, y + 40.0))
        };
        let ((from, to), (next_from, next_to)) = (around(first), around(last));
        drag_select(&mut game, from, to);
        assert_eq!(selected_count(&game), 1);

        let (from, to) = (next_from, next_to);
        shift_drag_select(&mut game, from, to);
        assert!(game.units[first].selected, "the old selection stays");
        assert!(game.units[last].selected, "and the new one joins it");
        assert_eq!(selected_count(&game), 2);

        drag_select(&mut game, from, to);
        assert_eq!(
            selected_count(&game),
            1,
            "without Shift a box still replaces"
        );
    }

    #[test]
    fn a_click_on_a_unit_with_shift_adds_it_and_a_second_click_keeps_it() {
        let mut game = in_battle();
        let (first, (x, y)) = infantry_on_screen(&game, 0);
        let (second, (other_x, other_y)) = infantry_on_screen(&game, 7);
        click_at(&mut game, x, y);
        for _ in 0..2 {
            game.handle(Event::MouseMove(other_x, other_y));
            game.handle(Event::MouseDown);
            game.handle(Event::ShiftMouseUp);
            assert!(game.units[first].selected && game.units[second].selected);
            assert_eq!(selected_count(&game), 2);
        }
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| !u.is_moving()),
            "nobody was sent anywhere"
        );
    }

    #[test]
    fn with_shift_picking_nothing_neither_clears_nor_sends_anyone() {
        let mut game = in_battle();
        select_first_group(&mut game);

        // A box around nothing.
        shift_drag_select(
            &mut game,
            (W / 2.0 - 800.0, H / 2.0 + 300.0),
            (W / 2.0 - 700.0, H / 2.0 + 400.0),
        );
        assert_eq!(selected_count(&game), FIRST_GROUP);

        // A click on bare ground.
        game.handle(Event::MouseMove(W / 2.0 - 800.0, H / 2.0 + 300.0));
        game.handle(Event::MouseDown);
        game.handle(Event::ShiftMouseUp);
        assert_eq!(selected_count(&game), FIRST_GROUP);
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| !u.is_moving())
        );
    }

    #[test]
    fn shift_still_lets_go_of_buttons_and_launches_the_superweapon() {
        let mut game = in_menu();
        let (x, y) = {
            let b = game
                .buttons()
                .into_iter()
                .find(|b| b.action == Action::SinglePlayer)
                .unwrap();
            (b.x + b.w / 2.0, b.y + b.h / 2.0)
        };
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        game.handle(Event::ShiftMouseUp);
        assert_eq!(game.screen, Screen::SinglePlayerMenu);

        let mut game = in_battle();
        pick_superweapon(&mut game);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::MouseDown);
        game.handle(Event::ShiftMouseUp);
        assert_eq!(game.combat.strikes.len(), 1);
    }

    #[test]
    fn a_box_works_dragged_in_any_direction() {
        let (a, b) = around_first_group();
        for (from, to) in [
            (a, b),
            (b, a),
            ((a.0, b.1), (b.0, a.1)),
            ((b.0, a.1), (a.0, b.1)),
        ] {
            let mut game = in_battle();
            drag_select(&mut game, from, to);
            assert_eq!(selected_count(&game), FIRST_GROUP);
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
        assert_eq!(selected_count(&game), FIRST_GROUP);
    }

    #[test]
    fn a_new_box_replaces_the_selection_and_a_small_one_picks_or_clears() {
        let mut game = in_battle();
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);

        let view = game.view();
        let unit = game.units[5];
        let (x, y) = view
            .project(unit.x, unit.y + units::HEIGHT / 2.0, unit.z)
            .unwrap();
        click_at(&mut game, x, y);
        assert_eq!(selected_count(&game), 1, "a click picks one");
        assert!(game.units[5].selected);

        drag_select(&mut game, (200.0, H - 400.0), (500.0, H - 200.0));
        assert_eq!(selected_count(&game), 0, "a box around nothing clears");
    }

    fn select_first_group(game: &mut Game) {
        let (from, to) = around_first_group();
        drag_select(game, from, to);
        assert_eq!(selected_count(game), FIRST_GROUP);
    }

    #[test]
    fn a_click_sends_the_selected_units_to_separate_spots_there() {
        let mut game = in_battle();
        select_first_group(&mut game);
        let clicked = (W / 2.0 - 800.0, H / 2.0 + 300.0);
        let target = game.view().ground_at(clicked.0, clicked.1).unwrap();
        click_at(&mut game, clicked.0, clicked.1);
        assert_eq!(selected_count(&game), FIRST_GROUP, "they stay selected");

        // The air and the ground reserve their spots separately.
        for flying in [false, true] {
            let sent: Vec<(f32, f32)> = game
                .units
                .iter()
                .filter(|u| u.selected && u.kind.flies() == flying)
                .map(|u| u.destination)
                .collect();
            assert!(!sent.is_empty());
            for (i, a) in sent.iter().enumerate() {
                assert!(
                    (a.0 - target.0).hypot(a.1 - target.1) < 5.0,
                    "near the click"
                );
                for b in &sent[i + 1..] {
                    assert!((a.0 - b.0).hypot(a.1 - b.1) > 1.0, "not one point");
                }
            }
        }
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0 && !u.selected)
                .all(|u| !u.is_moving()),
            "others stay"
        );

        for _ in 0..400 {
            game.update(0.05);
        }
        for unit in game.units.iter().filter(|u| u.selected) {
            assert!(!unit.is_moving(), "arrived: {unit:?}");
        }
    }

    #[test]
    fn the_right_button_deselects_and_units_keep_their_orders() {
        let mut game = in_battle();
        select_first_group(&mut game);
        click_at(&mut game, W / 2.0 - 800.0, H / 2.0 + 300.0);
        let sent: Vec<(f32, f32)> = game.units.iter().map(|u| u.destination).collect();

        game.handle(Event::RightDown);
        assert_eq!(selected_count(&game), 0);
        let after: Vec<(f32, f32)> = game.units.iter().map(|u| u.destination).collect();
        assert_eq!(after, sent, "they still go where they were sent");

        let moving = game
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.is_moving())
            .count();
        assert!(moving > 0);
        click_at(&mut game, W / 2.0 + 800.0, H / 2.0 + 300.0);
        let later: Vec<(f32, f32)> = game.units.iter().map(|u| u.destination).collect();
        assert_eq!(later, sent, "a left click now sends nobody");
    }

    #[test]
    fn the_right_button_cancels_a_box_being_dragged() {
        let mut game = in_battle();
        let (from, to) = around_first_group();
        game.handle(Event::MouseMove(from.0, from.1));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(to.0, to.1));
        game.handle(Event::RightDown);
        assert!(game.selection_ground().is_none());
        game.handle(Event::MouseUp);
        assert_eq!(selected_count(&game), 0);
    }

    #[test]
    fn the_right_button_does_nothing_in_menus_or_while_paused() {
        let mut game = in_battle();
        select_first_group(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        game.handle(Event::RightDown);
        assert_eq!(selected_count(&game), FIRST_GROUP, "in the menu");
        game.handle(Event::KeyDown(Key::Escape));
        game.handle(Event::KeyDown(Key::Pause));
        game.handle(Event::RightDown);
        assert_eq!(selected_count(&game), FIRST_GROUP, "while paused");
    }

    #[test]
    fn a_click_with_nothing_selected_sends_nobody() {
        let mut game = in_battle();
        click_at(&mut game, W / 2.0 - 800.0, H / 2.0 + 300.0);
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| !u.is_moving())
        );
    }

    #[test]
    fn a_click_on_ones_own_unit_selects_it_rather_than_sending_anyone() {
        let mut game = in_battle();
        select_first_group(&mut game);
        let unit = *game
            .units
            .iter()
            .find(|u| u.kind == UnitKind::Infantry)
            .unwrap();
        let (x, y) = game
            .view()
            .project(unit.x, unit.y + units::HEIGHT / 2.0, unit.z)
            .unwrap();
        click_at(&mut game, x, y);
        assert_eq!(selected_count(&game), 1);
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| !u.is_moving())
        );
    }

    #[test]
    fn dragging_a_new_box_gives_no_order() {
        let mut game = in_battle();
        select_first_group(&mut game);
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| !u.is_moving())
        );
    }

    #[test]
    fn units_stand_still_while_the_game_is_paused() {
        let mut game = in_battle();
        select_first_group(&mut game);
        click_at(&mut game, W / 2.0 - 800.0, H / 2.0 + 300.0);
        game.handle(Event::KeyDown(Key::Escape));
        let before: Vec<(f32, f32)> = game.units.iter().map(|u| (u.x, u.z)).collect();
        game.update(1.0);
        let after: Vec<(f32, f32)> = game.units.iter().map(|u| (u.x, u.z)).collect();
        assert_eq!(before, after);
    }

    #[test]
    fn units_keep_going_when_the_screen_scrolls_away_or_they_are_deselected() {
        let mut game = in_battle();
        select_first_group(&mut game);
        click_at(&mut game, W / 2.0 - 800.0, H / 2.0 + 300.0);
        drag_select(&mut game, (200.0, H - 400.0), (500.0, H - 200.0));
        assert_eq!(selected_count(&game), 0);
        let walking = game
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.is_moving())
            .count();
        assert!(walking > 0);
        for _ in 0..400 {
            game.update(0.05);
        }
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0)
                .all(|u| !u.is_moving()),
            "they still arrive"
        );
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
        assert!(
            game.camera.x > Camera::centered().x,
            "the edge still scrolls during a drag"
        );
        assert_eq!(game.selection_anchor, Some(anchored), "fixed on the map");
        let (west, _, east, _) = game.selection_ground().unwrap();
        assert_eq!(
            west, anchored.0,
            "the start stays where it was on the ground"
        );
        let under_cursor = game.view().ground_at(W - 1.0, H / 2.0).unwrap();
        assert!(
            (east - under_cursor.0).abs() < 0.001,
            "the other corner stays under the cursor"
        );
        let on_screen = game.view().project(anchored.0, 0.0, anchored.1).unwrap();
        assert!(
            on_screen.0 < start.0 - 10.0,
            "on screen the start slid away: {on_screen:?}"
        );
    }

    #[test]
    fn the_selection_is_a_rectangle_on_the_ground_seen_in_perspective() {
        let mut game = in_battle();
        game.handle(Event::Wheel(10.0));
        settle_zoom(&mut game);
        assert_eq!(
            game.camera.zoom, 0.5,
            "the low position shows perspective best"
        );
        game.handle(Event::MouseMove(W / 2.0 - 300.0, H / 2.0 - 300.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(W / 2.0 + 300.0, H / 2.0 + 300.0));

        let edges = game.selection_edges();
        assert_eq!(edges.len(), 4);
        // Clockwise from the far edge: far, right, near, left.
        let (far, right, near, left) = (edges[0], edges[1], edges[2], edges[3]);
        let width = |edge: ((f32, f32), (f32, f32))| (edge.1.0 - edge.0.0).abs();
        assert!(
            width(far) < width(near) * 0.95,
            "the far edge looks shorter"
        );
        assert!(
            (far.0.1 - far.1.1).abs() < 0.01,
            "far and near edges stay level"
        );
        assert!(
            width(right) > 1.0 && width(left) > 1.0,
            "the sides lean inward"
        );
        for edge in edges {
            assert!((edge.0.0 - edge.1.0).abs() + (edge.0.1 - edge.1.1).abs() > 1.0);
        }
    }

    #[test]
    fn the_selection_decides_by_ground_position_not_screen_position() {
        let mut game = in_battle();
        game.handle(Event::Wheel(10.0));
        settle_zoom(&mut game);
        let view = game.view();
        let a = (W / 2.0 - 500.0, H / 2.0 - 400.0);
        let b = (W / 2.0 + 500.0, H / 2.0 + 400.0);
        let corner_a = view.ground_at(a.0, a.1).unwrap();
        let corner_b = view.ground_at(b.0, b.1).unwrap();
        // Just inside the far edge's width on the ground, though the near edge is what
        // would bound it in a rectangle drawn on the screen.
        let inside_x = corner_b.0 - 0.1;
        game.units[0].x = inside_x;
        game.units[0].z = corner_a.1 + 0.1;
        game.units[1].x = corner_b.0 + 0.1;
        game.units[1].z = corner_a.1 + 0.1;
        drag_select(&mut game, a, b);
        assert!(game.units[0].selected, "inside on the ground");
        assert!(!game.units[1].selected, "outside on the ground");
    }

    #[test]
    fn edges_past_the_screen_are_cut_at_its_border() {
        let mut game = in_battle();
        let (from, _) = around_first_group();
        game.handle(Event::MouseMove(from.0, from.1));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(W - 1.0, H - 1.0));
        for _ in 0..100 {
            game.update(0.05);
            for (a, b) in game.selection_edges() {
                for (x, y) in [a, b] {
                    assert!(
                        (-0.01..=W + 0.01).contains(&x) && (-0.01..=H + 0.01).contains(&y),
                        "{x}, {y}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_box_scrolled_along_the_edge_selects_across_more_than_one_screen() {
        let mut game = in_battle();
        let (from, _) = around_first_group();
        game.handle(Event::MouseMove(from.0, from.1));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(W - 1.0, H / 2.0 + 400.0));
        let far_x = game
            .units
            .iter()
            .filter(|u| u.owner == 0)
            .map(|u| u.x)
            .fold(f32::MIN, f32::max);
        for _ in 0..40 {
            game.update(0.05);
            let (sx, _) = game.view().project(far_x, 0.0, game.camera.z).unwrap();
            if sx < W - 100.0 {
                break;
            }
        }
        game.handle(Event::MouseUp);
        let players = game.units.iter().filter(|u| u.is_selectable()).count();
        assert!(players > FIRST_GROUP, "there is a second group");
        assert_eq!(
            selected_count(&game),
            players,
            "both groups, a screen apart"
        );
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
        let anchor = game
            .selection_anchor
            .expect("the drag survives the start leaving the screen");
        assert!(
            game.view().project(anchor.0, 0.0, anchor.1).is_none(),
            "its start is behind the camera"
        );
        assert!(game.selection_ground().is_some());
        assert!(
            !game.selection_edges().is_empty(),
            "the visible part still draws"
        );
        game.handle(Event::MouseUp);
        assert!(game.selection_ground().is_none());
    }

    #[test]
    fn a_selection_cannot_start_on_the_sidebar_minimap_or_while_paused() {
        let mut game = in_battle();
        let cell = hud::grid_cells(W, H)[0];
        game.handle(Event::MouseMove(cell.x + 1.0, cell.y + 1.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_ground().is_none(), "button");
        game.handle(Event::MouseUp);

        // The gap between two sidebar blocks has no button, but it is not battlefield.
        game.handle(Event::MouseMove(cell.x + 1.0, cell.y - 2.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_ground().is_none(), "gap");
        game.handle(Event::MouseUp);

        let (x, y, size) = hud::minimap_rect(W, H);
        game.handle(Event::MouseMove(x + size / 2.0, y + size / 2.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_ground().is_none(), "minimap");
        game.handle(Event::MouseUp);

        game.handle(Event::KeyDown(Key::Pause));
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::MouseDown);
        assert!(game.selection_ground().is_none(), "paused");
        game.handle(Event::MouseUp);
        game.handle(Event::KeyDown(Key::Pause));

        game.handle(Event::KeyDown(Key::Tab));
        game.handle(Event::MouseDown);
        assert!(game.selection_ground().is_none(), "diplomacy window open");
    }

    #[test]
    fn interrupting_a_drag_cancels_it_and_keeps_the_old_selection() {
        let (from, to) = around_first_group();
        let begin = |game: &mut Game| {
            game.handle(Event::MouseMove(100.0, H - 200.0));
            game.handle(Event::MouseDown);
            assert!(game.selection_ground().is_some());
        };

        let mut game = in_battle();
        drag_select(&mut game, from, to);
        begin(&mut game);
        game.handle(Event::KeyDown(Key::Escape));
        assert!(game.selection_ground().is_none(), "the menu cancels");
        assert_eq!(selected_count(&game), FIRST_GROUP);

        let mut game = in_battle();
        drag_select(&mut game, from, to);
        begin(&mut game);
        game.handle(Event::Focus(false));
        assert!(game.selection_ground().is_none(), "losing focus cancels");
        game.handle(Event::Focus(true));
        game.handle(Event::MouseUp);
        assert_eq!(selected_count(&game), FIRST_GROUP);

        let mut game = in_battle();
        drag_select(&mut game, from, to);
        begin(&mut game);
        game.handle(Event::KeyDown(Key::Pause));
        assert!(game.selection_ground().is_none(), "pausing cancels");
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
        let previewed = game
            .units
            .iter()
            .filter(|u| game.in_selection_drag(u))
            .count();
        assert_eq!(previewed, FIRST_GROUP);
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
        let find = |f| {
            *game
                .buttons()
                .iter()
                .find(|b| b.action == Action::Faction(f))
                .unwrap()
        };
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
        let a = game
            .buttons()
            .into_iter()
            .find(|b| b.action == Action::Faction(Faction::A))
            .unwrap();
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
            assert_eq!(
                game.screen,
                Screen::FactionSelect,
                "returns to the same screen"
            );
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
        for (w, h) in [
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
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
        assert!(
            !s.prebuilt_base && !s.fog_of_war,
            "these two start unchecked"
        );
        assert_eq!(s.allies, [Ally::None, Ally::None]);
    }

    #[test]
    fn setup_columns_sit_in_the_outer_thirds() {
        for (w, h) in [
            (1280.0, 720.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            for b in setup_widgets(w, h) {
                let right = matches!(b.action, Action::MapDropdown | Action::SetDifficulty(_));
                if right {
                    assert!(
                        b.x >= w * 2.0 / 3.0 - 0.5 && b.x + b.w <= w + 0.5,
                        "{}",
                        b.label
                    );
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
        let range: Vec<_> = setup_sliders(W, H)
            .iter()
            .map(|s| (s.label, s.min, s.max))
            .collect();
        assert_eq!(
            range,
            [("AI Players", 1, 7), ("Money", 0, 30000), ("Units", 0, 30)]
        );
    }

    #[test]
    fn setup_sliders_drag_and_money_snaps_to_500() {
        let mut game = in_setup();
        let money = slider(&game, SliderId::Money);
        drag(
            &mut game,
            SliderId::Money,
            money.track_x + money.track_w * 0.5,
        );
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
        for (w, h) in [
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            let items = map_items(w, h);
            assert_eq!(items.len(), MAPS.len());
            let last = items.last().unwrap();
            assert!(last.y + last.h <= h, "at {w}x{h}");
            for item in &items {
                let width = crate::font::text_width(item.label) * small_text_px(h);
                assert!(
                    width + 16.0 * ui_scale(h) < item.w,
                    "{} at {w}x{h}",
                    item.label
                );
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
        game.handle(Event::MouseMove(
            hard.x + hard.w / 2.0,
            hard.y + hard.h / 2.0,
        ));
        assert!(matches!(game.hovered(), Some(Action::SelectMap(_))));

        game.handle(Event::MouseMove(1.0, 1.0));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert!(!game.dropdown_open);
        assert_eq!(game.skirmish.difficulty, Difficulty::Normal);

        click_widget(&mut game, Action::MapDropdown);
        game.handle(Event::KeyDown(Key::Escape));
        assert!(!game.dropdown_open);
        assert_eq!(
            game.screen,
            Screen::SkirmishSetup,
            "escape only closed the list"
        );
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
        assert!(
            game.skirmish.superweapons
                && !game.skirmish.prebuilt_base
                && game.skirmish.living_world
        );
    }

    #[test]
    fn commander_name_accepts_typing_and_stops_at_the_limit() {
        let mut game = in_setup();
        type_key(&mut game, Key::Char('x'));
        assert_eq!(
            game.skirmish.commander, "Commander",
            "ignored until the field is clicked"
        );

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
        assert_eq!(
            game.screen,
            Screen::SkirmishSetup,
            "escape did not leave the screen"
        );

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
        for click_slot in [
            0, 1, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 0,
        ] {
            click_widget(&mut game, Action::CycleAlly(click_slot));
            let [left, right] = game.skirmish.allies;
            assert!(left == Ally::None || left != right, "{left:?} twice");
        }

        let mut allies = [Ally::T, Ally::None];
        cycle_ally(&mut allies, 1);
        assert_eq!(allies, [Ally::T, Ally::X], "T is skipped");

        let mut allies = [Ally::S, Ally::F];
        cycle_ally(&mut allies, 1);
        assert_eq!(
            allies,
            [Ally::S, Ally::None],
            "S is skipped, none is still allowed"
        );

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
                assert!(
                    !colors[i + 1..].contains(a),
                    "{a:?} is held twice at {count}"
                );
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
        let play = setup_widgets(W, H)
            .into_iter()
            .find(|b| b.action == Action::StartBattle)
            .unwrap();
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
        for (w, h) in [
            (1280.0, 720.0),
            (1440.0, 900.0),
            (1920.0, 1080.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            let px = small_text_px(h);
            for b in opponent_widgets(OPPONENT_SLOTS, w, h) {
                assert!(
                    b.x >= w / 3.0 && b.x + b.w <= w * 2.0 / 3.0,
                    "{} at {w}x{h}",
                    b.label
                );
                assert!(b.y + b.h < buttons(Screen::SkirmishSetup, w, h)[0].y);
                if let Action::CycleOpponentKind(_) = b.action {
                    for kind in [
                        OpponentKind::Defensive,
                        OpponentKind::Competent,
                        OpponentKind::Attacker,
                    ] {
                        assert!(
                            text_width(kind.label()) * px < b.w,
                            "{} at {h}",
                            kind.label()
                        );
                    }
                }
            }
            let all = opponent_widgets(OPPONENT_SLOTS, w, h);
            for (i, a) in all.iter().enumerate() {
                for b in &all[i + 1..] {
                    assert!(
                        a.x + a.w <= b.x + 0.01
                            || b.x + b.w <= a.x + 0.01
                            || a.y + a.h <= b.y + 0.01
                            || b.y + b.h <= a.y + 0.01,
                        "{:?} overlaps {:?} at {w}x{h}",
                        a.action,
                        b.action
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
        assert!(
            play.y - (bar.y + bar.h) < 30.0 * ui_scale(H),
            "play is just under the bar"
        );
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
        assert!(
            (game.camera.x - expected.0).abs() < 1.0,
            "went east: {}",
            game.camera.x
        );
        assert!(game.camera.x > Camera::centered().x);

        click_at(&mut game, x + size / 2.0, y + size / 2.0);
        assert!((game.camera.x - Camera::centered().x).abs() < 0.01);
        assert!(
            (game.camera.z - Camera::centered().z).abs() < 0.01,
            "the middle is the map's middle"
        );
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
        assert!(
            (game.camera.z - world(start).1).abs() < 0.01,
            "moves on press"
        );

        let next = at(0.6, 0.45);
        game.handle(Event::MouseMove(next.0, next.1));
        assert!(
            (game.camera.x - world(next).0).abs() < 0.01,
            "follows the drag"
        );
        assert!(
            (game.camera.z - world(next).1).abs() < 0.01,
            "follows the drag"
        );

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
        let (east_x, _) =
            battlefield::minimap_to_world(rect, x + size - 0.01, y + size / 2.0).unwrap();
        assert!(
            (game.camera.x - east_x).abs() < 0.01,
            "holds to the minimap's edge"
        );
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
        assert!(
            (landed.x - before.x).abs() < 0.01 && (landed.z - before.z).abs() < 0.01,
            "already on the map"
        );
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
        for (w, h) in [
            (1280.0, 720.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            let (x, y, size) = hud::minimap_rect(w, h);
            assert_eq!(
                battlefield::edge_direction((x + size - 1.0, y), w, h),
                (0.0, 0.0),
                "at {w}x{h}"
            );
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
        for (w, h) in [
            (1280.0, 720.0),
            (1440.0, 900.0),
            (1920.0, 1080.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            let px = small_text_px(h);
            let widest = "W".repeat(COMMANDER_NAME_MAX);
            let all = setup_widgets(w, h);
            let name = all.iter().find(|b| b.action == Action::NameField).unwrap();
            assert!(text_width(&widest) * px < name.w, "name at {h}");
            for b in all
                .iter()
                .filter(|b| matches!(b.action, Action::SetDifficulty(_)))
            {
                let circle = 30.0 * ui_scale(h);
                assert!(
                    circle + text_width(b.label) * px <= b.w,
                    "{} at {h}",
                    b.label
                );
            }
            for s in setup_sliders(w, h) {
                let value = text_width("30000") * px;
                assert!(
                    s.label_x + text_width(s.label) * px < s.value_right - value,
                    "{} at {h}",
                    s.label
                );
            }
            for b in setup_checkboxes(w, h) {
                assert!(
                    text_width(b.label) * px < b.w - checkbox_size(h),
                    "{} at {h}",
                    b.label
                );
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
        for (w, h) in [
            (1280.0, 720.0),
            (1440.0, 900.0),
            (1920.0, 1080.0),
            (2560.0, 1600.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            let px = small_text_px(h);
            let (panel_x, panel_y, panel_w, panel_h) = options_panel(w, h);
            assert!(panel_x + panel_w <= w);
            for tab in TABS.iter().map(|t| t.0) {
                for s in sliders(tab, w, h) {
                    assert!(
                        s.label_x + text_width(s.label) * px < s.track_x,
                        "{} at {h}",
                        s.label
                    );
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
                assert!(
                    b.y + b.h <= panel_y + panel_h + 0.5,
                    "tabs fit the panel height"
                );
            }
        }
    }

    #[test]
    fn layout_stays_on_screen_across_aspect_ratios() {
        for (w, h) in [
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (2880.0, 1800.0),
            (5120.0, 1440.0),
            (1024.0, 768.0),
        ] {
            for screen in [
                Screen::MainMenu,
                Screen::SinglePlayerMenu,
                Screen::Options,
                Screen::Credits,
                Screen::EscapeMenu,
                Screen::FactionSelect,
                Screen::SkirmishSetup,
            ] {
                let all = buttons(screen, w, h);
                for b in &all {
                    assert!(b.x >= 0.0 && b.y >= 0.0 && b.x + b.w <= w && b.y + b.h <= h);
                }
                for (i, a) in all.iter().enumerate() {
                    for b in &all[i + 1..] {
                        let apart = a.x + a.w <= b.x
                            || b.x + b.w <= a.x
                            || a.y + a.h <= b.y
                            || b.y + b.h <= a.y;
                        assert!(apart, "{} overlaps {}", a.label, b.label);
                    }
                }
            }
        }
    }
}
