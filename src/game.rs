//! Game flow and menu logic. No OS or GPU code, so it runs under `cargo test`.

use crate::battlefield::{self, Camera};
use crate::campaign::{self, Campaign, Mission, Next, Side};
use crate::fog::{Fog, Sight};
use crate::hud::{
    self, GUARD_BUTTON, REPAIR_BUTTON, RETREAT_BUTTON, SELL_BUTTON, STARPORT_BUTTON,
    UPGRADE_BUTTON, WAYPOINT_BUTTON,
};
use crate::maps;
use crate::mission::{self, Script, Token};
use crate::platform::{Event, Key, Modifier};
use crate::strategy::Strategy;
use crate::units::{self, Facing, Role, Unit, UnitKind};
use crate::weapons::OVERKILL_BLOWS_APART;
use crate::weapons::{Combat, Made, Noise, Superweapon};

pub const INTRO_SECONDS: f32 = 3.0;
pub const LOADING_SECONDS: f32 = 2.0;
pub const MOVIE_SECONDS: f32 = 3.0;
/// How long the opening movie's screen waits for its movie to start before going on without
/// it.
const OPENING_WAIT_SECONDS: f32 = 0.2;
/// What the player starts a campaign with, as the original's first missions give.
const CAMPAIGN_START_MONEY: i32 = 10_000;
/// What the player and any opponents start with outside a skirmish, which sets its own.
const DEFAULT_BATTLE_MONEY: i32 = 10000;
/// A selection smaller than this each way, at 1080 pixels tall, counts as a click.
const CLICK_SLOP: f32 = 6.0;
/// How near a click must land to a unit to pick it, at 1080 pixels tall.
const PICK_RADIUS: f32 = 25.0;

/// What a cell of a build menu holds: something to make, or the palace's superweapon to fire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Make(Role),
    Superweapon,
    /// An upgrade of the player's building of this role.
    Upgrade(Role),
    /// One more of this role on the order to the starport.
    Order(Role),
    /// Pays for the order, and sends the frigate for it.
    Purchase,
    /// Empties the order.
    CancelOrder,
}

/// What each house can order through its starport, as the original's `Starportable` among
/// what we have, in the order of its `[UnitTypes]`.
fn starportable(house: Faction) -> &'static [Role] {
    match house {
        Faction::Atreides => &[
            Role::AtTrike,
            Role::AtApc,
            Role::AtRepairUnit,
            Role::AtMongoose,
            Role::AtMinotaurus,
            Role::AtOrni,
            Role::AdvCarryall,
            Role::Harvester,
            Role::Mcv,
            Role::Carryall,
        ],
        Faction::Harkonnen => &[
            Role::HkBuzzsaw,
            Role::HkAssault,
            Role::HkFlame,
            Role::HkInkVine,
            Role::HkMissile,
            Role::HkDevastator,
            Role::HkGunship,
            Role::AdvCarryall,
            Role::Harvester,
            Role::Mcv,
            Role::Carryall,
        ],
        Faction::Ordos => &[
            Role::OrDustScout,
            Role::OrLaserTank,
            Role::OrApc,
            Role::OrKobra,
            Role::OrEits,
            Role::AdvCarryall,
            Role::Harvester,
            Role::Mcv,
            Role::Carryall,
        ],
    }
}

/// The original's starport, from its `[General]` rules at 25 updates a second: prices change
/// every `StarportCostUpdateDelay` by up to `StarportCostVariationPercent` either way; every
/// `StarportStockIncreaseDelay` each kind has a `StarportStockIncreaseProb` chance of one more
/// in stock; a delivery is of `StarportMaxDeliverySingle` at most; and the frigate comes
/// `FrigateCountdown` after the order is paid.
const STARPORT_PRICE_SECONDS: f32 = 60.0;
const STARPORT_PRICE_VARIATION: f32 = 0.4;
const STARPORT_STOCK_SECONDS: f32 = 40.0;
const STARPORT_STOCK_CHANCE: f32 = 0.9;
const STARPORT_MOST_IN_A_DELIVERY: usize = 6;
const FRIGATE_SECONDS: f32 = 100.0;
/// The most of one kind the starport's merchants ever have, and the most they start with.
const STARPORT_MOST_STOCK: u8 = 6;
const STARPORT_STARTING_STOCK: u8 = 3;

/// One kind the starport's merchants sell: what one costs now, and how many they have.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Offer {
    pub role: Role,
    pub price: i32,
    pub stock: u8,
}

/// The starport's market, the player's order from it, and the frigate bringing what was paid
/// for with the seconds until it lands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Market {
    pub offers: Vec<Offer>,
    pub order: Vec<Role>,
    pub frigate: Option<(Vec<Role>, f32)>,
    price_in: f32,
    stock_in: f32,
    random: u32,
}

impl Market {
    /// The market as a battle starts, for a house.
    fn open(house: Faction, random: u32) -> Self {
        let mut market = Market {
            offers: starportable(house)
                .iter()
                .map(|&role| Offer {
                    role,
                    price: role.cost(),
                    stock: 0,
                })
                .collect(),
            price_in: STARPORT_PRICE_SECONDS,
            stock_in: STARPORT_STOCK_SECONDS,
            random: random | 1,
            ..Market::default()
        };
        for offer in &mut market.offers {
            let share = battlefield::next_random(&mut market.random);
            offer.stock = (share * f32::from(STARPORT_STARTING_STOCK + 1)) as u8;
        }
        market.reprice();
        market
    }

    /// Gives each kind a new price, up to the variation either side of what it costs to make.
    fn reprice(&mut self) {
        for offer in &mut self.offers {
            let swing = battlefield::next_random(&mut self.random) * 2.0 - 1.0;
            let price = offer.role.cost() as f32 * (1.0 + swing * STARPORT_PRICE_VARIATION);
            offer.price = (price / 10.0).round() as i32 * 10;
        }
    }

    /// Lets the market's time go by: its prices change and its stock grows.
    fn tick(&mut self, dt: f32) {
        self.price_in -= dt;
        if self.price_in <= 0.0 {
            self.price_in += STARPORT_PRICE_SECONDS;
            self.reprice();
        }
        self.stock_in -= dt;
        if self.stock_in <= 0.0 {
            self.stock_in += STARPORT_STOCK_SECONDS;
            for offer in &mut self.offers {
                let comes = battlefield::next_random(&mut self.random) < STARPORT_STOCK_CHANCE;
                if comes && offer.stock < STARPORT_MOST_STOCK {
                    offer.stock += 1;
                }
            }
        }
    }

    /// How many of a kind are on the order.
    pub fn ordered(&self, role: Role) -> usize {
        self.order.iter().filter(|&&r| r == role).count()
    }

    /// What the order costs at the prices now.
    pub fn total(&self) -> i32 {
        self.order
            .iter()
            .filter_map(|&role| self.offers.iter().find(|o| o.role == role))
            .map(|offer| offer.price)
            .sum()
    }

    /// Adds one of a kind to the order, if the merchants have another, the delivery has
    /// room, and no frigate is on its way.
    fn add(&mut self, role: Role) {
        let in_stock = self
            .offers
            .iter()
            .any(|o| o.role == role && usize::from(o.stock) > self.ordered(role));
        if in_stock && self.order.len() < STARPORT_MOST_IN_A_DELIVERY && self.frigate.is_none() {
            self.order.push(role);
        }
    }

    /// Takes one of a kind off the order.
    fn take_off(&mut self, role: Role) {
        if let Some(at) = self.order.iter().rposition(|&r| r == role) {
            self.order.remove(at);
        }
    }

    /// Pays for the order out of `money` and sends the frigate for it, if there is one and
    /// the money for all of it.
    fn purchase(&mut self, money: &mut i32) -> bool {
        let total = self.total();
        if self.order.is_empty() || total > *money || self.frigate.is_some() {
            return false;
        }
        *money -= total;
        for role in &self.order {
            if let Some(offer) = self.offers.iter_mut().find(|o| o.role == *role) {
                offer.stock = offer.stock.saturating_sub(1);
            }
        }
        self.frigate = Some((std::mem::take(&mut self.order), FRIGATE_SECONDS));
        true
    }
}

/// What the Upgrades menu lists before the allies' buildings, in the order of the original's
/// `[BuildingTypes]`: the construction yard, which a deployed MCV is, comes last there.
const UPGRADE_ORDER: [Role; 5] = [
    Role::Barracks,
    Role::Refinery,
    Role::Factory,
    Role::Hanger,
    Role::Mcv,
];

/// What each house's Infantry menu lists, in the order of the original's `[UnitTypes]` in
/// `Rules.txt`. Ordos AA troopers are left out: we have none yet.
fn infantry_order(house: Faction) -> &'static [Role] {
    match house {
        Faction::Atreides => &[
            Role::Scout,
            Role::AtInfantry,
            Role::AtSniper,
            Role::Engineer,
            Role::AtKindjal,
        ],
        Faction::Harkonnen => &[
            Role::Scout,
            Role::HkLightInf,
            Role::HkTrooper,
            Role::Engineer,
            Role::HkFlamer,
        ],
        Faction::Ordos => &[
            Role::Scout,
            Role::OrChemical,
            Role::Engineer,
            Role::OrMortar,
            Role::OrSaboteur,
        ],
    }
}

/// What each house's Vehicles menu lists, in the order of the original's `[UnitTypes]`: what
/// its factory makes, then its hangar with its palace's superweapon among it, then every
/// house's harvester, MCV and carryall. The sonic tank and the deviator are left out: we have
/// none yet.
fn vehicle_order(house: Faction) -> &'static [Entry] {
    use Entry::{Make, Superweapon};
    match house {
        Faction::Atreides => &[
            Make(Role::AtTrike),
            Make(Role::AtMongoose),
            Make(Role::AtApc),
            Make(Role::AtRepairUnit),
            Make(Role::AtMinotaurus),
            Make(Role::AtOrni),
            Make(Role::AdvCarryall),
            Superweapon,
            Make(Role::AtAdp),
            Make(Role::Harvester),
            Make(Role::Mcv),
            Make(Role::Carryall),
        ],
        Faction::Harkonnen => &[
            Make(Role::HkBuzzsaw),
            Make(Role::HkAssault),
            Make(Role::HkFlame),
            Make(Role::HkInkVine),
            Make(Role::HkMissile),
            Make(Role::HkDevastator),
            Make(Role::HkGunship),
            Make(Role::AdvCarryall),
            Superweapon,
            Make(Role::HkAdp),
            Make(Role::Harvester),
            Make(Role::Mcv),
            Make(Role::Carryall),
        ],
        Faction::Ordos => &[
            Make(Role::OrDustScout),
            Make(Role::OrLaserTank),
            Make(Role::OrApc),
            Make(Role::OrKobra),
            Make(Role::OrEits),
            Make(Role::AdvCarryall),
            Superweapon,
            Make(Role::OrAdp),
            Make(Role::Harvester),
            Make(Role::Mcv),
            Make(Role::Carryall),
        ],
    }
}

/// What each house's Buildings menu lists, in the order of the original's `[BuildingTypes]`.
/// Ordos have no landing pad. A refinery's further pads are among the upgrades.
fn building_order(house: Faction) -> impl Iterator<Item = Role> {
    let [lesser, greater] = Role::towers_of(house);
    [
        Role::Windtrap,
        Role::Barracks,
        Role::Wall,
        Role::Refinery,
        Role::Factory,
        Role::Outpost,
        lesser,
        greater,
        Role::Hanger,
        Role::Helipad,
        Role::Starport,
        Role::Palace(house),
    ]
    .into_iter()
    .filter(move |role| role.made_by(house))
}
/// Seconds of battle before the superweapon can fire again.
pub const SUPERWEAPON_RECHARGE: f32 = 60.0;
/// How long a side has to have fewer harvesters than refineries before it is given one.
const HARVESTER_GIFT_SECONDS: f32 = 60.0;
/// With crates on, the most seconds between one crate dropping somewhere and the next, how long
/// one lies before it is gone, the most lying about at once, and the most and least money in
/// one, in steps of the least.
const CRATE_SECONDS: f32 = 40.0;
const CRATE_LASTS: f32 = 60.0;
const CRATES_AT_MOST: usize = 6;
const CRATE_MONEY: (i32, i32) = (250, 1500);

/// What opponents make before anything else, while they have fewer than this many of each for
/// each of their refineries: harvesters to bring in money, and carryalls to help them.
const OPPONENT_ECONOMY: [(Role, usize); 2] = [(Role::Harvester, 2), (Role::Carryall, 1)];
/// What opponents build, one at random whenever they get to. They leave out what only helps
/// others, since they have no plan for it yet.
const OPPONENT_BUILDS: [Role; 28] = [
    Role::AtInfantry,
    Role::AtSniper,
    Role::AtKindjal,
    Role::HkLightInf,
    Role::HkTrooper,
    Role::HkFlamer,
    Role::OrChemical,
    Role::OrMortar,
    Role::OrSaboteur,
    Role::AtTrike,
    Role::AtMongoose,
    Role::AtMinotaurus,
    Role::HkAssault,
    Role::HkInkVine,
    Role::HkBuzzsaw,
    Role::HkFlame,
    Role::HkDevastator,
    Role::OrDustScout,
    Role::OrLaserTank,
    Role::OrKobra,
    Role::OrApc,
    Role::AtApc,
    Role::HkMissile,
    Role::HkAdp,
    Role::AtAdp,
    Role::AtOrni,
    Role::HkGunship,
    Role::OrAdp,
];
/// How long before the superweapon is ready the launcher on top of a palace starts to open.
const LAUNCHER_OPENS_SECONDS: f32 = 3.0;
/// How much an opponent has standing at most before it stops making more, so that a long battle
/// does not bury the game in units.
const OPPONENT_UNITS_AT_MOST: usize = 80;

/// What the chat says when an MCV is told to deploy on the sand or lower ground.
const CANNOT_DEPLOY: &str = "MCV needs higher ground to deploy";

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
/// A press of the button on the map while something is being placed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PlacingPress {
    from: (f32, f32),
    /// Whether it has been dragged far enough to turn what is being placed.
    turned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorLook {
    Arrow,
    Attack,
    Heal,
    /// A selected carryall would pick up or put down a vehicle here.
    Carry,
    /// The selected would go here, or a selected building would send what it makes here.
    Move,
    /// The selected unit under the cursor would dig in.
    Deploy,
    /// The selected unit under the cursor would stop being dug in.
    Undeploy,
    /// A selected ornithopter or gunship would land on the landing pad under the cursor, or a
    /// selected harvester empty its load at the refinery there.
    Land,
    /// The selected harvesters would go and harvest the sand here.
    Harvest,
    /// The selected would march here together.
    March,
    /// The selected soldiers would get into the APC under the cursor.
    Board,
    /// A click would sell what is here, or anything of the player's it is put on.
    Sell,
    /// A click would start or stop mending the player's building here.
    Repair,
}

/// What the next click on the map does instead of what it usually would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickMode {
    /// Sends the selected marching there together, after Z.
    March,
    /// Sells the player's building there for half what it cost, with the sell button down,
    /// until it is let go.
    Sell,
    /// Starts mending the player's building there, or stops, with the repair button down.
    Repair,
    /// Adds a waypoint there to the route the selected are sent along, with the waypoint
    /// button down.
    Route,
}

/// Units sent along a route of waypoints: who they are, and the waypoints still ahead of
/// the one they are on their way to.
#[derive(Clone, Debug, PartialEq)]
struct Route {
    ids: Vec<u32>,
    ahead: std::collections::VecDeque<(f32, f32)>,
}

/// What the battlefield tells the player out loud, as the original's spoken messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Speech {
    UnitReady,
    Training,
    ConstructionStarted,
    ReadyToPlace,
    ConstructionComplete,
    UnitLost,
    BuildingLost,
    BuildingSold,
    BaseUnderAttack,
    HarvesterUnderAttack,
    LowPower,
    InsufficientFunds,
    Upgrading,
    GuardOn,
    GuardOff,
    WaypointPlaced,
    WaypointAdded,
    Retreat,
    SuperweaponReady,
    Victory,
    Defeat,
}

impl Speech {
    /// How long after it is said it is not said again, in seconds: a warning of an attack
    /// longer than anything else, so that a fight does not keep it saying so.
    fn quiet_for(self) -> f32 {
        match self {
            Speech::BaseUnderAttack | Speech::HarvesterUnderAttack | Speech::LowPower => 20.0,
            _ => 3.0,
        }
    }
}

/// How long the retreat button waits for its second click, in seconds.
const RETREAT_CONFIRM_SECONDS: f32 = 5.0;

/// The new building the player has on the way: one at a time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Site {
    Idle,
    /// Being built, with the seconds left.
    Building(Role, f32),
    /// Done, and waiting for its button to be clicked.
    Ready(Role),
    /// Its button was clicked, and the next click on the map puts it there.
    Placing(Role),
}

/// One message in the battle's chat.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatLine {
    pub text: String,
    posted_at: f32,
}

/// What one of the little squares of a building being placed shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceSquare {
    /// The building can go here.
    Free,
    /// It can go here, and this is where what it makes comes out, or where a refinery's
    /// harvesters come in.
    Exit,
    /// Sand, a cliff, something standing there, or too far from any building.
    Blocked,
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
        if !unit.role.counts_for_stats() {
            return;
        }
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
    Upgrades,
    Starport,
}

/// What the player's units answer, out loud, when they are told something.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Said {
    /// They have just been selected.
    Selected,
    /// They have been sent somewhere.
    Moving,
    /// They have been told to attack.
    Attacking,
}

/// The three playable factions: A is blue, H red, O cyan. Each is shown as its planet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faction {
    Atreides,
    Harkonnen,
    Ordos,
}

impl Faction {
    pub fn label(self) -> &'static str {
        match self {
            Faction::Atreides => "House Atreides",
            Faction::Harkonnen => "House Harkonnen",
            Faction::Ordos => "House Ordos",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Faction::Atreides => Faction::Harkonnen,
            Faction::Harkonnen => Faction::Ordos,
            Faction::Ordos => Faction::Atreides,
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
    /// The movie a house's campaign opens with, before its first battle.
    Opening(Faction),
    SkirmishSetup,
    Loading,
    InGame,
    /// Esc on the battlefield. This and the options and credits screens opened from it
    /// draw over the battle while `Game::in_battle` holds.
    EscapeMenu,
    /// What was killed and lost, after the Victory or Defeat message.
    BattleStats,
    /// The campaign's map of Arrakis between its battles, the original's strategic battle
    /// map: where the player picks what to attack, and is asked to defend.
    CampaignMap,
    /// The Mentat's briefing before a campaign battle.
    Briefing,
    /// The units a new tech level brings.
    NewUnits,
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
    Tleilaxu,
    Ix,
    Guild,
    Fremen,
    Sardaukar,
}

impl Ally {
    /// The main building of the mini-house, which a house allied with it can put up.
    pub fn main_building(self) -> Option<Role> {
        match self {
            Ally::None => None,
            Ally::Tleilaxu => Some(Role::TlFleshVat),
            Ally::Ix => Some(Role::IxResCentre),
            Ally::Guild => Some(Role::GuPalace),
            Ally::Fremen => Some(Role::FrCamp),
            Ally::Sardaukar => Some(Role::ImBarracks),
        }
    }

    /// What a house allied with the mini-house can have its main building make.
    pub fn unit(self) -> Option<Role> {
        match self {
            Ally::None => None,
            Ally::Tleilaxu => Some(Role::TlContaminator),
            Ally::Ix => Some(Role::IxInfiltrator),
            Ally::Guild => Some(Role::GuMaker),
            Ally::Fremen => Some(Role::FrFremen),
            Ally::Sardaukar => Some(Role::ImSardaukar),
        }
    }

    pub fn next(self) -> Self {
        match self {
            Ally::None => Ally::Tleilaxu,
            Ally::Tleilaxu => Ally::Ix,
            Ally::Ix => Ally::Guild,
            Ally::Guild => Ally::Fremen,
            Ally::Fremen => Ally::Sardaukar,
            Ally::Sardaukar => Ally::None,
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

    pub fn next(self) -> Self {
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
    /// Index into `maps::list()`.
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
        const FACTIONS: [Faction; 3] = [Faction::Harkonnen, Faction::Atreides, Faction::Ordos];
        let mut opponents = [Opponent {
            kind: OpponentKind::Competent,
            faction: Faction::Harkonnen,
            allies: [Ally::None; 2],
            color: TeamColor::Blue,
        }; OPPONENT_SLOTS];
        for (opponent, faction) in opponents.iter_mut().zip(FACTIONS.iter().cycle()) {
            opponent.faction = *faction;
        }
        let mut config = SkirmishConfig {
            commander: "Commander".to_string(),
            faction: Faction::Atreides,
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

/// Option values, not saved between runs. The game speed and the volumes are applied so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// From 1 to `UPDATES_PER_SECOND.len()`, as on the original's slider.
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

/// How many times a second the original game updates at each step of its Game Speed slider,
/// from the slowest. The battle runs this much faster than at `units::UPDATES_PER_SECOND`.
pub const UPDATES_PER_SECOND: [f32; 7] = [8.0, 10.0, 12.0, 15.0, 20.0, 25.0, 30.0];

impl Settings {
    /// How many seconds of battle go by for each second that passes.
    pub fn battle_pace(&self) -> f32 {
        let step = (self.game_speed - 1).clamp(0, UPDATES_PER_SECOND.len() as i32 - 1);
        UPDATES_PER_SECOND[step as usize] / units::UPDATES_PER_SECOND
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            game_speed: 6,
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
    /// Shows the maps of this folder in the open dropdown.
    MapFolder(MapFolder),
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
    /// A territory on the campaign's map, by its place.
    Territory(usize),
    CampaignAttack,
    /// Leaves the campaign for the main menu.
    CampaignMenu,
    /// The answers to the question when a territory is attacked.
    Defend,
    GiveUp,
    TakeSide(Side),
    AcceptBriefing,
    ShowNewUnits,
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

/// How far down the screen the middle of each centered button of the main menu and of the
/// single player menu is, as a share of its height: where the original's `main.xbf` has its
/// `single` and `Mplayer`, and its `Campaign`, `Tutorial`, `Skirmish` and `Load`.
const MAIN_MENU_ROWS: [f32; 2] = [0.684, 0.792];
const SINGLE_PLAYER_ROWS: [f32; 4] = [0.576, 0.684, 0.792, 0.919];

const BACK_ONLY: &[(Action, &str, Anchor)] = &[(Action::Back, "Back", Anchor::BottomLeft)];

const BATTLE_STATS: &[(Action, &str, Anchor)] =
    &[(Action::LeaveBattleStats, "Continue", Anchor::BottomRight)];

const CAMPAIGN_MAP: &[(Action, &str, Anchor)] = &[
    (Action::CampaignMenu, "Main Menu", Anchor::BottomLeft),
    (Action::CampaignAttack, "Attack", Anchor::BottomRight),
];

const BRIEFING: &[(Action, &str, Anchor)] = &[
    (Action::Back, "Back", Anchor::BottomLeft),
    (Action::AcceptBriefing, "Accept", Anchor::BottomRight),
];

const NEW_UNITS: &[(Action, &str, Anchor)] = &[(Action::Back, "Continue", Anchor::BottomRight)];

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
    (
        SliderId::GameSpeed,
        "Game Speed",
        1,
        UPDATES_PER_SECOND.len() as i32,
        &[],
    ),
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
        (Faction::Atreides, 0.0, TOP_Y),
        (Faction::Harkonnen, -SPREAD, BOTTOM_Y),
        (Faction::Ordos, SPREAD, BOTTOM_Y),
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

/// The maps of one kind, which the open dropdown shows together.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapFolder {
    #[default]
    Skirmish,
    Campaign,
}

impl MapFolder {
    const ALL: [(MapFolder, &'static str); 2] = [
        (MapFolder::Skirmish, "Skirmish"),
        (MapFolder::Campaign, "Campaign"),
    ];

    fn of(map: &maps::Map) -> Self {
        if map.campaign {
            MapFolder::Campaign
        } else {
            MapFolder::Skirmish
        }
    }
}

/// The open dropdown's entries, directly below it: a tab for each folder if there is more
/// than one, then the maps of the folder shown, scrolled on by `scroll` columns.
pub fn map_items(folder: MapFolder, scroll: usize, width: f32, height: f32) -> Vec<Button> {
    let folders = maps::list().iter().any(|map| map.campaign && !map.hidden);
    let entries = folder_entries(folder);
    let mut items = Vec::new();
    if folders {
        let s = ui_scale(height);
        let (x, w) = setup_column(false, width, height);
        let wide = w / MapFolder::ALL.len() as f32;
        items.extend(
            MapFolder::ALL
                .iter()
                .enumerate()
                .map(|(n, &(folder, label))| Button {
                    action: Action::MapFolder(folder),
                    label,
                    x: x + n as f32 * wide,
                    y: LIST_TOP * s,
                    w: wide,
                    h: LIST_ROW * s,
                }),
        );
    }
    items.extend(list_items(entries, folders, scroll, width, height));
    items
}

/// The maps of a folder, each by its number and its name.
fn folder_entries(folder: MapFolder) -> impl Iterator<Item = (usize, &'static str)> + Clone {
    maps::list()
        .iter()
        .enumerate()
        .filter(move |(_, map)| !map.hidden && MapFolder::of(map) == folder)
        .map(|(i, map)| (i, map.name.as_str()))
}

/// What the map dropdown says beside a map's name: how many players it takes.
pub fn players_note(players: usize) -> String {
    format!("{players} players")
}

/// The longest of `players_note`, which every column leaves room for.
const WIDEST_PLAYERS_NOTE: &str = "8 players";

/// How far down the dropdown's list starts, and how tall each of its rows is, unscaled.
const LIST_TOP: f32 = 404.0;
const LIST_ROW: f32 = 52.0;

/// How the dropdown lays out this many entries: where its first column is and how wide each
/// is, how many rows it has from where, how many columns all of them take, and how many
/// show at once.
struct ListLayout {
    left: f32,
    wide: f32,
    top: f32,
    rows: usize,
    columns: usize,
    shown: usize,
}

impl ListLayout {
    /// Columns as tall as the screen leaves room for under the dropdown, and under its
    /// folders' tabs if it has them, each as wide as the widest label and its players need: from under the
    /// dropdown on, or as far to its left as keeps them on the screen. More columns than fit
    /// are scrolled to.
    fn new(
        labels: impl Iterator<Item = &'static str>,
        tabs: bool,
        width: f32,
        height: f32,
    ) -> Self {
        let s = ui_scale(height);
        let (x, w) = setup_column(false, width, height);
        let (tall, margin) = (LIST_ROW * s, 8.0 * s);
        let top = (LIST_TOP + if tabs { LIST_ROW } else { 0.0 }) * s;
        let rows = (((height - margin - top) / tall) as usize).max(1);
        let (count, widest) = labels.fold((0_usize, 0.0_f32), |(count, widest), label| {
            let width = crate::font::text_width(label) * small_text_px(height);
            (count + 1, widest.max(width))
        });
        let columns = count.div_ceil(rows).max(1);
        let note = crate::font::text_width(WIDEST_PLAYERS_NOTE) * small_text_px(height);
        let wide = (widest + note + 48.0 * s)
            .max(w / 2.0)
            .min(width - 2.0 * margin);
        let shown = columns.min((((width - 2.0 * margin) / wide) as usize).max(1));
        let left = x.min(width - margin - shown as f32 * wide).max(margin);
        ListLayout {
            left,
            wide,
            top,
            rows,
            columns,
            shown,
        }
    }

    /// How many columns the list can be scrolled on by.
    fn most_scroll(&self) -> usize {
        self.columns - self.shown
    }
}

/// Entries for these maps, each by its number and label, laid out as `ListLayout` says, those
/// of the columns scrolled to.
fn list_items(
    entries: impl Iterator<Item = (usize, &'static str)> + Clone,
    tabs: bool,
    scroll: usize,
    width: f32,
    height: f32,
) -> Vec<Button> {
    let layout = ListLayout::new(entries.clone().map(|(_, label)| label), tabs, width, height);
    let scroll = scroll.min(layout.most_scroll());
    let tall = LIST_ROW * ui_scale(height);
    entries
        .enumerate()
        .filter(|(n, _)| (scroll..scroll + layout.shown).contains(&(n / layout.rows)))
        .map(|(n, (map, label))| Button {
            action: Action::SelectMap(map),
            label,
            x: layout.left + (n / layout.rows - scroll) as f32 * layout.wide,
            y: layout.top + (n % layout.rows) as f32 * tall,
            w: layout.wide,
            h: tall,
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
    modal_pair(
        (Action::ConfirmFaction, "OK"),
        (Action::CancelFaction, "Cancel"),
        width,
        height,
    )
}

/// The answers to a question of the campaign's map, in the message box.
fn question_buttons(question: Next, width: f32, height: f32) -> Vec<Button> {
    match question {
        Next::Side => modal_pair(
            (Action::TakeSide(Side::Copec), "Copec"),
            (Action::TakeSide(Side::Gunseng), "Gunseng"),
            width,
            height,
        ),
        _ => modal_pair(
            (Action::Defend, "Defend"),
            (Action::GiveUp, "Give Up"),
            width,
            height,
        ),
    }
}

/// Two buttons side by side at the foot of the message box.
fn modal_pair(
    (first, first_label): (Action, &'static str),
    (second, second_label): (Action, &'static str),
    width: f32,
    height: f32,
) -> Vec<Button> {
    const W: f32 = 240.0;
    const H: f32 = 72.0;
    const GAP: f32 = 40.0;
    let s = ui_scale(height);
    let (_, my, _, mh) = modal_rect(width, height);
    let y = my + mh - (H + 40.0) * s;
    vec![
        Button {
            action: first,
            label: first_label,
            x: width / 2.0 - (W + GAP / 2.0) * s,
            y,
            w: W * s,
            h: H * s,
        },
        Button {
            action: second,
            label: second_label,
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
        Screen::Intro
        | Screen::Loading
        | Screen::Movie(_)
        | Screen::Opening(_)
        | Screen::InGame => return Vec::new(),
        Screen::MainMenu => MAIN_MENU,
        Screen::SinglePlayerMenu => SINGLE_PLAYER_MENU,
        Screen::EscapeMenu => ESCAPE_MENU,
        Screen::BattleStats => BATTLE_STATS,
        Screen::CampaignMap => CAMPAIGN_MAP,
        Screen::Briefing => BRIEFING,
        Screen::NewUnits => NEW_UNITS,
        Screen::Options | Screen::Credits | Screen::FactionSelect | Screen::SkirmishSetup => {
            BACK_ONLY
        }
    };
    let s = ui_scale(height);
    let centered = items.iter().filter(|i| i.2 == Anchor::Center).count() as f32;
    let stack_h = centered * BUTTON_H + (centered - 1.0) * GAP;
    let mut y = (1080.0 - stack_h) / 2.0;
    // The main menus' stand where the original's scene has them, under its planet.
    let mut rows = match screen {
        Screen::MainMenu => MAIN_MENU_ROWS.iter(),
        Screen::SinglePlayerMenu => SINGLE_PLAYER_ROWS.iter(),
        _ => [].iter(),
    };

    let mut all: Vec<Button> = items
        .iter()
        .map(|&(action, label, anchor)| {
            let (px, py, w) = match anchor {
                Anchor::Center => {
                    let top = match rows.next() {
                        Some(middle) => middle * 1080.0 - BUTTON_H / 2.0,
                        None => y,
                    };
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
    if screen == Screen::CampaignMap {
        all.extend(territory_buttons(width, height));
    }
    if screen == Screen::Briefing {
        let s = ui_scale(height);
        all.push(Button {
            action: Action::ShowNewUnits,
            label: "New Units",
            x: (width - CORNER_W * s) / 2.0,
            y: (1080.0 - MARGIN - BUTTON_H) * s,
            w: CORNER_W * s,
            h: BUTTON_H * s,
        });
    }
    all
}

/// Where the campaign's map of Arrakis is drawn, as (x, y, w, h): on the left, with the
/// panel of what is picked on it to its right.
pub fn campaign_map_rect(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    (width / 2.0 - 920.0 * s, 150.0 * s, 1220.0 * s, 780.0 * s)
}

pub fn campaign_panel(width: f32, height: f32) -> (f32, f32, f32, f32) {
    let s = ui_scale(height);
    (width / 2.0 + 360.0 * s, 150.0 * s, 560.0 * s, 780.0 * s)
}

/// Where a point of the original's map, up being up, lands on the screen: the whole map
/// fitted into its box, the same way across as down.
pub fn campaign_place(width: f32, height: f32) -> impl Fn((f32, f32)) -> (f32, f32) {
    let (x, y, w, h) = campaign_map_rect(width, height);
    let ((left, bottom), (right, top)) = campaign::data()
        .and_then(campaign::Data::bounds)
        .unwrap_or(((0.0, 0.0), (1.0, 1.0)));
    let (across, down) = ((right - left).max(1.0), (top - bottom).max(1.0));
    let scale = (w / across).min(h / down);
    let (from_x, from_y) = (x + (w - across * scale) / 2.0, y + (h - down * scale) / 2.0);
    move |(px, py)| (from_x + (px - left) * scale, from_y + (top - py) * scale)
}

/// How wide the mark in the middle of each territory is, unscaled, and how tall a row of
/// the panel's homeworlds.
const TOKEN: f32 = 44.0;
const HOMEWORLD_ROW: f32 = 64.0;
/// How far down the panel its homeworlds start.
const HOMEWORLDS_TOP: f32 = 500.0;

/// A mark in the middle of each territory of Arrakis, and a row in the panel for each
/// homeworld, off the map.
fn territory_buttons(width: f32, height: f32) -> Vec<Button> {
    let Some(data) = campaign::data() else {
        return Vec::new();
    };
    let s = ui_scale(height);
    let place = campaign_place(width, height);
    let (px, py, pw, _) = campaign_panel(width, height);
    let mut homeworlds = 0.0;
    data.territories
        .iter()
        .enumerate()
        .map(|(at, territory)| match territory.centre {
            Some(centre) => {
                let (cx, cy) = place(centre);
                Button {
                    action: Action::Territory(at),
                    label: "",
                    x: cx - TOKEN * s / 2.0,
                    y: cy - TOKEN * s / 2.0,
                    w: TOKEN * s,
                    h: TOKEN * s,
                }
            }
            None => {
                let y = py + (HOMEWORLDS_TOP + homeworlds * HOMEWORLD_ROW) * s;
                homeworlds += 1.0;
                Button {
                    action: Action::Territory(at),
                    label: &territory.name,
                    x: px,
                    y,
                    w: pw,
                    h: (HOMEWORLD_ROW - 8.0) * s,
                }
            }
        })
        .collect()
}

/// How many of a mission script's updates go by in a second of battle: a guess, which puts
/// the Atreides' first reinforcements in at five seconds and the Mentat's call to prepare at
/// forty-three.
const SCRIPT_UPDATES_PER_SECOND: f32 = 10.0;
/// The points of a map where the player's base and the enemy's main base stand.
const PLAYER_BASE_POINT: &str = "Default";
const ENEMY_BASE_POINT: &str = "Primary";
/// What the rules' name of a construction yard ends with: an MCV dug in.
const CONSTRUCTION_YARD: &str = "ConYard";

/// A battle as its mission's script sees it: its sides are 0 for the player, then each one it
/// makes, all of them the player's enemy.
impl mission::Battle for Game {
    fn now(&self) -> i32 {
        self.script_updates
    }

    fn new_side(&mut self) -> i32 {
        self.script_sides += 1;
        self.script_sides
    }

    fn base(&self, side: i32) -> (f32, f32) {
        self.map_point(if side == 0 {
            PLAYER_BASE_POINT
        } else {
            ENEMY_BASE_POINT
        })
    }

    fn enemy_base(&self) -> (f32, f32) {
        self.map_point(ENEMY_BASE_POINT)
    }

    fn script_point(&self, number: i32) -> (f32, f32) {
        self.map_point(&format!("Script{number}"))
    }

    fn place(&mut self, side: i32, kind: u16, at: (f32, f32), facing: i32) -> Option<u32> {
        self.put_scripted(side, kind, at, Some(Facing::numbered(facing)))
    }

    fn bring(&mut self, side: i32, kind: u16, at: (f32, f32)) -> Option<u32> {
        self.put_scripted(side, kind, at, None)
    }

    fn left(&self, side: i32) -> i32 {
        let of_side = |unit: &Unit| {
            if side == 0 {
                unit.owner == 0
            } else {
                self.script_side_of.contains(&(unit.id, side))
            }
        };
        let left = self
            .units
            .iter()
            .filter(|u| u.alive && !u.kind.is_building() && of_side(u));
        left.count() as i32
    }

    fn destroyed(&self, thing: Option<u32>) -> bool {
        thing.is_some_and(|id| !self.units.iter().any(|u| u.id == id && u.alive))
    }

    fn say(&mut self, line: u16) {
        self.mission_lines.push(line);
    }

    fn pay(&mut self, side: i32, money: i32) {
        if let Some(held) = self.money.get_mut(usize::from(side != 0)) {
            *held = money;
        }
    }

    fn win(&mut self) {
        self.decide(Outcome::Victory);
    }

    fn lose(&mut self) {
        self.decide(Outcome::Defeat);
    }

    /// A side's are what the script put down for it, and the someone's also what the map has
    /// of nobody the script made and not nobody's; a construction yard is an MCV dug in, and an
    /// MCV one that is not.
    fn count(&self, side: i32, kind: u16) -> i32 {
        let Some(name) = campaign::kind_name(kind) else {
            return 0;
        };
        let Some((role, _)) = Role::named(name) else {
            return 0;
        };
        let yard = name.ends_with(CONSTRUCTION_YARD);
        let of_side = |unit: &Unit| match side {
            0 => unit.owner == 0,
            _ => {
                self.script_side_of.contains(&(unit.id, side))
                    || side == mission::SOMEONE_SIDE
                        && self.script_side(unit).is_none()
                        && unit.owner != units::NEUTRAL
            }
        };
        let counted = self.units.iter().filter(|u| {
            u.alive && u.role == role && (role != Role::Mcv || u.deployed == yard) && of_side(u)
        });
        counted.count() as i32
    }

    fn scene(&mut self, playing: bool) {
        self.scene_playing = playing;
    }

    fn camera(&mut self, shot: mission::Shot) {
        let seconds = |updates: i32| updates as f32 / SCRIPT_UPDATES_PER_SECOND;
        match shot {
            mission::Shot::Keep => self.camera.keep(),
            mission::Shot::LookAt((x, z)) => self.camera.look_at(x, z),
            mission::Shot::GlideTo(to, updates) => self.camera.glide_to(to, seconds(updates)),
            mission::Shot::Circle(degrees, way) => {
                let way = if way == 2 { -1.0 } else { 1.0 };
                let per_second = degrees as f32 * SCRIPT_UPDATES_PER_SECOND;
                self.camera.circle(way * per_second.to_radians());
            }
            mission::Shot::StopCircling => self.camera.circle(0.0),
            mission::Shot::GlideBack(updates) => self.camera.glide_back(seconds(updates)),
        }
    }

    fn place_of(&self, thing: Option<u32>) -> Option<(f32, f32)> {
        let unit = self.units.iter().find(|u| Some(u.id) == thing && u.alive)?;
        Some((unit.x, unit.z))
    }

    fn sees(&self, seen: i32, by: i32) -> bool {
        let of = |side: i32| {
            self.units
                .iter()
                .filter(move |u| u.alive && self.script_side(u) == Some(side))
        };
        of(by).any(|looking| {
            of(seen).any(|u| (u.x - looking.x).hypot(u.z - looking.z) <= looking.sight())
        })
    }

    fn set_health(&mut self, thing: Option<u32>, health: i32) {
        if let Some(unit) = self.units.iter_mut().find(|u| Some(u.id) == thing) {
            unit.health = (health as f32).min(unit.max_health());
        }
    }

    fn turn_into(&mut self, thing: Option<u32>, kind: u16, side: i32) -> Option<u32> {
        let (role, _) = Role::named(campaign::kind_name(kind)?)?;
        let id = thing?;
        let (owner, color) = self.side_owner(side)?;
        let unit = self.units.iter_mut().find(|u| u.id == id && u.alive)?;
        units::turn_into(unit, role);
        (unit.owner, unit.color) = (owner, color);
        self.script_side_of.retain(|&(of, _)| of != id);
        self.script_side_of.push((id, side));
        Some(id)
    }

    fn owner_side(&self, thing: Option<u32>) -> Option<i32> {
        let unit = self.units.iter().find(|u| Some(u.id) == thing && u.alive)?;
        // What has changed hands is its new owner's: a captured outpost the player's.
        if unit.owner == 0 {
            return Some(0);
        }
        self.script_side(unit)
    }

    fn hurt_by(&self, thing: Option<u32>, side: i32) -> bool {
        // Who hurt it is not kept: what the player's enemies hold is hurt by the player.
        let Some(unit) = self.units.iter().find(|u| Some(u.id) == thing) else {
            return false;
        };
        let hurt = !unit.alive || unit.health < unit.max_health();
        hurt && (side == 0) == (unit.owner != 0)
    }

    fn relate(&mut self, a: i32, b: i32, enemies: bool) {
        let pair = (a.min(b), a.max(b));
        self.script_enemies.retain(|&made| made != pair);
        if enemies {
            self.script_enemies.push(pair);
        }
        // What each of the script's things is follows from whom its side is now the enemy of.
        let sides = self.script_side_of.clone();
        for (id, side) in sides {
            let Some((owner, color)) = self.side_owner(side) else {
                continue;
            };
            if let Some(unit) = self.units.iter_mut().find(|u| u.id == id) {
                (unit.owner, unit.color) = (owner, color);
            }
        }
    }
}

impl Game {
    /// Whether the script has made these two of its sides enemies.
    fn script_enemies_of(&self, a: i32, b: i32) -> bool {
        self.script_enemies.contains(&(a.min(b), a.max(b)))
    }

    /// Whose a script's side's things are, and in what color: the player's own for the
    /// player's side, and for a side the player's enemies are the enemies of; the enemy's for
    /// one the player's are; and nobody's for the rest, as the prisoners of a camp, whom
    /// neither side fires at.
    fn side_owner(&self, side: i32) -> Option<(usize, TeamColor)> {
        let fights_the_enemy = || {
            (1..=self.script_sides).any(|other| {
                self.script_enemies_of(side, other) && self.script_enemies_of(other, 0)
            })
        };
        let owner = if side == 0 {
            0
        } else if self.script_enemies_of(side, 0) {
            1
        } else if fights_the_enemy() {
            0
        } else {
            units::NEUTRAL
        };
        let color = match self.stats_rows().get(owner) {
            Some(row) => row.color,
            None => TeamColor::Yellow,
        };
        Some((owner, color))
    }

    /// The script's side a thing is on, if the script put it down.
    fn script_side(&self, unit: &Unit) -> Option<i32> {
        self.script_side_of
            .iter()
            .find(|&&(id, _)| id == unit.id)
            .map(|&(_, side)| side)
            .or((unit.owner == 0).then_some(0))
    }
}

/// Whether a point is inside a triangle, whichever way round its corners go.
fn is_in_triangle([a, b, c]: [(f32, f32); 3], (x, y): (f32, f32)) -> bool {
    let side =
        |(ax, ay): (f32, f32), (bx, by): (f32, f32)| (bx - ax) * (y - ay) - (by - ay) * (x - ax);
    let sides = [side(a, b), side(b, c), side(c, a)];
    sides.iter().all(|&s| s >= 0.0) || sides.iter().all(|&s| s <= 0.0)
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
    /// The folder of maps the open dropdown shows, and how many columns on it is scrolled.
    pub map_folder: MapFolder,
    pub map_scroll: usize,
    pub color_open: bool,
    /// Index into `maps::list()` of the current battle.
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
    /// The campaign under way, and the battle of it being fought or briefed.
    pub campaign: Option<Campaign>,
    pub mission: Option<Mission>,
    pub briefing: Option<Mission>,
    /// The territory picked on the campaign's map.
    pub picked: Option<usize>,
    /// The tech level whose new units were last shown.
    pub units_shown: u32,
    /// The mission's script the battle runs, if it has one; how many of its updates have gone
    /// by and how far into the next; how many sides it has made; the script's side of each
    /// thing it made, by id; and the lines of speech it has asked for, not yet played.
    script: Option<Script>,
    script_updates: i32,
    /// Whether a scene of the mission's script is playing, the camera its own.
    scene_playing: bool,
    script_clock: f32,
    script_sides: i32,
    script_side_of: Vec<(u32, i32)>,
    /// The pairs of the script's sides it has made enemies.
    script_enemies: Vec<(i32, i32)>,
    mission_lines: Vec<u16>,
    /// Pressed on the battlefield minimap and not yet released.
    minimap_dragging: bool,
    pub units: Vec<Unit>,
    /// Vehicles carryalls hold, which are out of `units` meanwhile.
    pub carried: Vec<units::Carried>,
    /// The id of the unit the camera stays on, if any.
    following: Option<u32>,
    /// What the player's units have just answered, and who answered, until it is heard.
    reply: Option<(Said, Vec<(Role, Faction)>)>,
    pub site: Site,
    /// Seconds until opponents next build something.
    opponent_random: u32,
    /// Which way what is being placed will look.
    placing_facing: units::Facing,
    /// The press of the button on the map while placing something, until it is let go.
    placing_press: Option<PlacingPress>,
    /// The square a wall being placed starts from, once it has been clicked.
    wall_start: Option<(i32, i32)>,
    /// The squares of wall still to go up, in the order they do.
    wall_plan: std::collections::VecDeque<(i32, i32)>,
    /// Where the next battle's sides are placed from.
    start_random: u32,
    /// The place the player's base started.
    home: (f32, f32),
    /// Each opponent's plan for its base and its army, in owner order from 1.
    strategies: Vec<Strategy>,
    pub combat: Combat,
    /// The superweapon is chosen and waiting for a click on the map.
    pub aiming_strike: bool,
    /// What the next click on the map does instead, if anything.
    pub click_mode: Option<ClickMode>,
    /// The routes of waypoints the player's units are on.
    routes: Vec<Route>,
    /// The starport's market, and the player's order from it.
    market: Market,
    /// What is to be said out loud, waiting to be taken; and when each was last said.
    speeches: Vec<Speech>,
    last_said: Vec<(Speech, f32)>,
    /// How hurt each of the player's units was at the last look, by id, to hear of attacks;
    /// and whether the player was short of power then.
    health_seen: std::collections::HashMap<u32, f32>,
    was_short_of_power: bool,
    /// The player's buildings that were going up at the last look.
    constructing_seen: Vec<u32>,
    /// Seconds the retreat button still waits for its second click.
    retreat_armed: f32,
    /// Seconds until the superweapon can fire again.
    pub strike_recharge: f32,
    /// Where a selection drag started, as a point on the map (x, z), so that it stays put
    /// on the ground while the camera moves.
    selection_anchor: Option<(f32, f32)>,
    /// Where on screen the left button went down, and whether the edge has scrolled the
    /// camera since: only that, or moving the cursor away, makes a press a box.
    press_point: (f32, f32),
    scrolled_since_press: bool,
    /// Seconds since the game started, unbroken by going from screen to screen.
    menu_seconds: f32,
    /// Whether a movie is playing on the intro's or a faction's movie screen, which then lasts
    /// until it ends. Going to another screen clears it.
    movie_playing: bool,
    /// Whether a Control key is down, which turns clicks into attack orders.
    control_held: bool,
    /// The ids of the units assigned to each of the number keys 1 to 9.
    control_groups: [Vec<u32>; 9],
    /// New units get ids never used in this battle, so a missile cannot chase the wrong one.
    next_unit_id: u32,
    pub build_tab: BuildTab,
    /// The player's money first, then each opponent's.
    money: Vec<i32>,
    /// The refineries whose owners have had the harvester each new one brings, by id.
    refineries_served: Vec<u32>,
    /// How long each side, the player first, has had fewer harvesters than refineries.
    short_of_harvesters: Vec<f32>,
    /// Money lying about the battlefield, with crates on.
    pub crates: Vec<units::Crate>,
    /// Seconds until the next crate drops.
    crate_wait: f32,
    /// Where crates' places and money come from.
    crate_random: u32,
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
    /// Gives this run its own placing of the sides at the start of its battles.
    pub fn with_start_seed(mut self, seed: u32) -> Self {
        self.start_random = seed | 1;
        self
    }

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
            map_folder: MapFolder::Skirmish,
            map_scroll: 0,
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
            campaign: None,
            mission: None,
            briefing: None,
            picked: None,
            units_shown: 0,
            script: None,
            script_updates: 0,
            scene_playing: false,
            script_clock: 0.0,
            script_sides: 0,
            script_side_of: Vec::new(),
            script_enemies: Vec::new(),
            mission_lines: Vec::new(),
            minimap_dragging: false,
            units: Vec::new(),
            carried: Vec::new(),
            following: None,
            reply: None,
            site: Site::Idle,
            opponent_random: 1,
            start_random: 1,
            placing_facing: units::Facing::default(),
            wall_start: None,
            wall_plan: std::collections::VecDeque::new(),
            placing_press: None,
            home: (0.0, 0.0),
            strategies: Vec::new(),
            combat: Combat::default(),
            aiming_strike: false,
            click_mode: None,
            routes: Vec::new(),
            market: Market::default(),
            speeches: Vec::new(),
            last_said: Vec::new(),
            health_seen: std::collections::HashMap::new(),
            was_short_of_power: false,
            constructing_seen: Vec::new(),
            retreat_armed: 0.0,
            strike_recharge: 0.0,
            selection_anchor: None,
            press_point: (0.0, 0.0),
            scrolled_since_press: false,
            menu_seconds: 0.0,
            movie_playing: false,
            control_held: false,
            control_groups: Default::default(),
            next_unit_id: 0,
            build_tab: BuildTab::Infantry,
            money: vec![DEFAULT_BATTLE_MONEY],
            refineries_served: Vec::new(),
            short_of_harvesters: Vec::new(),
            crates: Vec::new(),
            crate_wait: 0.0,
            crate_random: 1,
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
        !unit.hidden_from(0, &self.teams)
            && (units::are_friends(&self.teams, 0, unit.owner)
                || self.sight_at(unit.x, unit.z) == Sight::Visible)
    }

    /// Whether a unit is against the player.
    pub fn is_enemy_of_player(&self, unit: &Unit) -> bool {
        units::are_enemies(&self.teams, 0, unit.owner)
    }

    /// Whether the player has a finished outpost, whose radar sees the whole map and puts
    /// enemies on the minimap.
    pub fn has_outpost(&self) -> bool {
        // Its radar goes out while its owner is short of power.
        !units::is_short_of_power(&self.units, 0)
            && self.units.iter().any(|u| {
                u.alive && u.owner == 0 && u.role == Role::Outpost && !u.is_under_construction()
            })
    }

    /// How far the launcher on top of this palace is open, from 0 to 1: it opens as the
    /// player's superweapon comes ready.
    pub fn launcher_open(&self, palace: &Unit) -> f32 {
        if palace.owner != 0 || palace.is_under_construction() || !self.superweapon_allowed() {
            return 0.0;
        }
        1.0 - (self.strike_recharge / LAUNCHER_OPENS_SECONDS).min(1.0)
    }

    /// Sees the map anew from where the player's and allies' units stand now.
    fn update_fog(&mut self) {
        let outpost = self.has_outpost();
        if let Some(fog) = &mut self.fog {
            if outpost {
                fog.see_everything();
                return;
            }
            let teams = &self.teams;
            fog.update(
                self.units
                    .iter()
                    .filter(|u| u.alive && units::are_friends(teams, 0, u.owner)),
                battlefield::terrain(self.battle_map).floor(),
            );
        }
    }

    /// The superweapon can be turned off in a skirmish's setup; elsewhere it is always there.
    pub fn superweapon_allowed(&self) -> bool {
        self.battle_kind != BattleKind::Skirmish || self.skirmish.superweapons
    }

    pub fn view(&self) -> battlefield::View {
        battlefield::View::new(self.camera, self.width, self.height)
    }

    /// The selection while dragging one: a rectangle on the ground lined up with the screen,
    /// as (left, top, right, bottom) in `Camera::across_and_down`, between where the drag
    /// started on the map and the ground under the cursor.
    /// There is none while a press is placing a building: a drag then only turns it.
    pub fn selection_ground(&self) -> Option<(f32, f32, f32, f32)> {
        if self.placing_press.is_some() {
            return None;
        }
        let (x, z) = self.selection_anchor?;
        let (cursor_x, cursor_z) = self.view().ground_at(self.mouse.0, self.mouse.1)?;
        let (x, z) = self.camera.across_and_down(x, z);
        let (cursor_x, cursor_z) = self.camera.across_and_down(cursor_x, cursor_z);
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
        let corners = [(west, north), (east, north), (east, south), (west, south)]
            .map(|(across, down)| self.camera.from_across_and_down(across, down));
        (0..corners.len())
            .filter_map(|i| {
                let (a, b) =
                    view.project_ground_line(corners[i], corners[(i + 1) % corners.len()])?;
                battlefield::clip_to_rect(a, b, screen)
            })
            .collect()
    }

    /// Whether the drag in progress would select this unit: by where its feet appear on
    /// screen, the box lying on the sand plane as drawn, so that one up on a plateau or the
    /// Heighliner's deck is caught where it shows. An aircraft counts both by the spot below
    /// it and by the spot it appears over, since a box drawn around what the player sees
    /// should catch it.
    pub fn in_selection_drag(&self, unit: &Unit) -> bool {
        let Some(rectangle) = self
            .selection_ground()
            .filter(|_| unit.is_selected_in_groups())
        else {
            return false;
        };
        let terrain = battlefield::terrain(self.battle_map);
        let feet = (unit.x, terrain.height_at(unit.x, unit.z), unit.z);
        self.appears_in_selection(rectangle, feet)
            || (unit.kind.flies()
                && (units::in_rectangle(rectangle, self.camera.across_and_down(unit.x, unit.z))
                    || self.appears_in_selection(rectangle, unit.center())))
    }

    /// Whether a point appears on screen over the sand plane inside the selection.
    fn appears_in_selection(
        &self,
        rectangle: (f32, f32, f32, f32),
        (x, y, z): (f32, f32, f32),
    ) -> bool {
        let view = self.view();
        let appears_over = view
            .project(x, y, z)
            .and_then(|(px, py)| view.ground_at(px, py));
        appears_over
            .is_some_and(|(x, z)| units::in_rectangle(rectangle, self.camera.across_and_down(x, z)))
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
            self.press_point = self.mouse;
            self.scrolled_since_press = false;
        }
    }

    /// Whether the left button, if down on the battlefield, has stayed near where it went
    /// down on screen, which makes this a click and not a box. The camera easing its zoom
    /// or following someone does not turn a click into a box; scrolling at the edge does.
    fn is_click_so_far(&self) -> bool {
        let Some((x, z)) = self.selection_anchor else {
            return true;
        };
        let start = if self.scrolled_since_press {
            self.view().project_clamped(x, 0.0, z)
        } else {
            self.press_point
        };
        let slop = CLICK_SLOP * ui_scale(self.height);
        (start.0 - self.mouse.0).abs() < slop && (start.1 - self.mouse.1).abs() < slop
    }

    fn selected_units(&self) -> Vec<&Unit> {
        self.units.iter().filter(|u| u.selected).collect()
    }

    fn can_order_attack_here(&self) -> bool {
        self.cursor_on_battlefield()
            && !self.aiming_strike
            && !matches!(self.site, Site::Placing(_))
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
                .any(|s| s.can_attack_when_told(u, &self.teams) || s.can_tend(u, &self.teams))
        };
        let (chosen, distance) = self.unit_near_cursor(able)?;
        let own = self.unit_near_cursor(Unit::is_selectable);
        own.is_none_or(|(own_unit, own_distance)| own_unit == chosen || distance < own_distance)
            .then_some(chosen)
    }

    /// What a build menu shows, as the original's does: of what the player's house and their
    /// allies make there, in its order, what can be made now or is being made, one to a cell.
    pub fn menu(&self, tab: BuildTab) -> Vec<Entry> {
        let house = self.faction_of(0);
        let allies = self.player_allies();
        let listed: Vec<Entry> = match tab {
            BuildTab::Infantry => infantry_order(house)
                .iter()
                .copied()
                .chain(allies.iter().filter_map(|ally| ally.unit()))
                .map(Entry::Make)
                .collect(),
            BuildTab::Vehicles => vehicle_order(house).to_vec(),
            BuildTab::Buildings => building_order(house)
                .filter(|role| !matches!(role, Role::Palace(_)) || self.superweapon_allowed())
                .chain(allies.iter().filter_map(|ally| ally.main_building()))
                .map(Entry::Make)
                .collect(),
            BuildTab::Upgrades => UPGRADE_ORDER
                .into_iter()
                .chain(allies.iter().filter_map(|ally| ally.main_building()))
                .map(Entry::Upgrade)
                .collect(),
            BuildTab::Starport => self
                .market
                .offers
                .iter()
                .map(|offer| Entry::Order(offer.role))
                .chain([Entry::Purchase, Entry::CancelOrder])
                .collect(),
        };
        listed
            .into_iter()
            .filter(|&entry| match entry {
                Entry::Superweapon => self.superweapon().is_some(),
                Entry::Upgrade(role) => {
                    self.can_upgrade(role) || self.upgrade_progress(role).is_some()
                }
                Entry::Order(_) | Entry::Purchase | Entry::CancelOrder => self.has_starport(),
                Entry::Make(role) => {
                    self.can_produce(role)
                        || self.making_progress(role).is_some()
                        || self.queue_badge(role).is_some()
                        || self.is_ready_to_place(role)
                }
            })
            .collect()
    }

    /// What a cell of a build menu holds.
    pub fn build_entry(&self, tab: BuildTab, cell: usize) -> Option<Entry> {
        self.menu(tab).get(cell).copied()
    }

    /// What a cell of a build menu makes.
    pub fn build_role(&self, tab: BuildTab, cell: usize) -> Option<Role> {
        match self.build_entry(tab, cell)? {
            Entry::Make(role) => Some(role),
            _ => None,
        }
    }

    /// Whether the player has a starport up and standing.
    pub fn has_starport(&self) -> bool {
        self.has_standing(Role::Starport)
    }

    /// The starport's market and the player's order from it.
    pub fn market(&self) -> &Market {
        &self.market
    }

    /// Lets the market's time go by, and lands the frigate when its time comes: what it
    /// brings is set down by the player's starport, or paid back if there is none left.
    fn run_market(&mut self, dt: f32) {
        self.market.tick(dt);
        let Some((cargo, left)) = &mut self.market.frigate else {
            return;
        };
        *left -= dt;
        if *left > 0.0 {
            return;
        }
        let cargo = std::mem::take(cargo);
        self.market.frigate = None;
        let terrain = battlefield::terrain(self.battle_map);
        for role in cargo {
            let starport = self.units.iter().position(|u| {
                u.alive && u.owner == 0 && u.role == Role::Starport && !u.is_under_construction()
            });
            let landed = starport.is_some_and(|starport| {
                units::deliver_at(&mut self.units, terrain, role, starport, self.next_unit_id)
            });
            if landed {
                self.next_unit_id += 1;
            } else {
                self.money[0] += role.cost();
            }
        }
        self.post_chat("Starport delivery arrived".to_string());
    }

    /// Whether the player has a building of this role that could be upgraded now.
    pub fn can_upgrade(&self, role: Role) -> bool {
        self.units
            .iter()
            .any(|u| u.owner == 0 && u.role == role && u.can_be_upgraded())
    }

    /// How far along the upgrade of the player's building of this role is, from 0 to 1, if
    /// one is being upgraded.
    pub fn upgrade_progress(&self, role: Role) -> Option<f32> {
        self.units
            .iter()
            .filter(|u| u.alive && u.owner == 0 && u.role == role)
            .find_map(|u| u.upgrading)
            .map(|left| 1.0 - left / role.upgrade_seconds().max(0.001))
    }

    /// The power the player's buildings make and use.
    pub fn player_power(&self) -> (i32, i32) {
        units::power(&self.units, 0)
    }

    /// The share of the power the player's buildings make that is to spare, from 0 with none
    /// or too little to 1 with none of it used.
    pub fn power_to_spare(&self) -> f32 {
        let (made, used) = self.player_power();
        if made <= 0 {
            0.0
        } else {
            (1.0 - used as f32 / made as f32).clamp(0.0, 1.0)
        }
    }

    /// Moves along every building's upgrade. A refinery's new pad brings its harvester.
    fn work_on_upgrades(&mut self, dt: f32) {
        let terrain = battlefield::terrain(self.battle_map);
        for building in units::work_on_upgrades(&mut self.units, &mut self.money, dt) {
            let upgraded = self.units[building];
            if upgraded.owner == 0 {
                self.post_chat(format!("{} upgraded", upgraded.name()));
            }
            if upgraded.role == Role::Refinery {
                let ids = (self.next_unit_id, self.next_unit_id + 1);
                self.next_unit_id += 2;
                units::deliver_harvester(&mut self.units, terrain, (building, false), ids);
            }
        }
    }

    /// The Vehicles menu's cell holding the superweapon, while the player has one.
    pub fn superweapon_cell(&self) -> Option<usize> {
        self.menu(BuildTab::Vehicles)
            .iter()
            .position(|&entry| entry == Entry::Superweapon)
    }

    /// What the player's palace can fire, if they have one and superweapons are allowed. A
    /// captured palace brings the superweapon of the house that built it.
    pub fn superweapon(&self) -> Option<Superweapon> {
        self.units
            .iter()
            .filter(|u| {
                self.superweapon_allowed() && u.alive && u.owner == 0 && !u.is_under_construction()
            })
            .find_map(|u| match u.role {
                Role::Palace(house) => Some(Superweapon::of(house)),
                _ => None,
            })
    }

    /// Each opponent keeps its buildings busy with what it can pay for: once a building is
    /// free, it starts harvesters and carryalls while it is short of them, or else something at
    /// random. It keeps back what its base still needs and what it owes for what is being made
    /// already. What comes out joins its army at the rally point, where `Strategy` takes over.
    fn run_opponents(&mut self) {
        let terrain = battlefield::terrain(self.battle_map);
        for owner in 1..self.teams.len() {
            let has = |units: &[Unit], role: Role| {
                units
                    .iter()
                    .filter(|u| u.alive && u.owner == owner)
                    .map(|u| usize::from(u.role == role) + u.orders_for(role))
                    .sum::<usize>()
            };
            let mut standing = self
                .units
                .iter()
                .filter(|u| u.alive && u.owner == owner)
                .count();
            let saving = self
                .strategies
                .get(owner - 1)
                .map_or(0, |plan| plan.saving_for(&self.units));
            let money = self.money.get(owner).copied().unwrap_or(0);
            let mut spare = money - saving - units::still_owed(&self.units, owner);
            while standing < OPPONENT_UNITS_AT_MOST {
                let affordable = |role: Role| {
                    role.cost() <= spare && units::has_free_maker(&self.units, role, owner)
                };
                let refineries = has(&self.units, Role::Refinery);
                let short = OPPONENT_ECONOMY.into_iter().find(|&(role, each)| {
                    affordable(role) && has(&self.units, role) < each * refineries
                });
                let role = match short {
                    Some((role, _)) => role,
                    None => {
                        let can_build: Vec<Role> = OPPONENT_BUILDS
                            .into_iter()
                            .filter(|&r| affordable(r))
                            .collect();
                        if can_build.is_empty() {
                            break;
                        }
                        let (x, _) = terrain.random_point(&mut self.opponent_random);
                        can_build[(x.abs() * 22.1) as usize % can_build.len()]
                    }
                };
                units::start_making(&mut self.units, role, owner);
                spare -= role.cost();
                standing += 1;
            }
        }
    }

    /// Moves along what every building is making, and lets out what is done.
    fn work_on_making(&mut self, dt: f32) {
        let terrain = battlefield::terrain(self.battle_map);
        for maker in units::work_on_making(&mut self.units, &mut self.money, dt) {
            let id = self.next_unit_id;
            let Some(new) = units::finish_making(&mut self.units, terrain, maker, id) else {
                continue;
            };
            self.next_unit_id += 1;
            if self.units[new].owner == 0 {
                self.post_chat(format!("{} ready", self.units[new].name()));
                self.announce(Speech::UnitReady);
            } else {
                let owner = self.units[new].owner;
                let rally = self
                    .strategies
                    .get(owner - 1)
                    .and_then(|plan| plan.rally(&self.units, &self.teams));
                let point =
                    rally.unwrap_or_else(|| terrain.random_point(&mut self.opponent_random));
                units::send_to_spot(&mut self.units, new, point, terrain);
            }
        }
    }

    /// Sends each refinery just finished the harvester it brings its owner, the first time it
    /// is finished only: one taken over brings none. Its carryall stays, the owner's. And each
    /// side that has had fewer harvesters than refineries for a while is given one more, its
    /// carryall flying off again.
    fn deliver_harvesters(&mut self, dt: f32) {
        let terrain = battlefield::terrain(self.battle_map);
        for i in 0..self.units.len() {
            let refinery = self.units[i];
            if refinery.role != Role::Refinery
                || !refinery.alive
                || refinery.is_under_construction()
                || self.refineries_served.contains(&refinery.id)
            {
                continue;
            }
            self.refineries_served.push(refinery.id);
            let ids = (self.next_unit_id, self.next_unit_id + 1);
            self.next_unit_id += 2;
            units::deliver_harvester(&mut self.units, terrain, (i, true), ids);
            if refinery.owner == 0 {
                self.post_chat("Harvester on its way".to_string());
            }
        }
        for owner in 0..self.short_of_harvesters.len() {
            let short = &mut self.short_of_harvesters[owner];
            let Some(refinery) = units::short_of_harvesters(&self.units, owner) else {
                *short = 0.0;
                continue;
            };
            *short += dt;
            if *short < HARVESTER_GIFT_SECONDS {
                continue;
            }
            *short = 0.0;
            let ids = (self.next_unit_id, self.next_unit_id + 1);
            self.next_unit_id += 2;
            units::deliver_harvester(&mut self.units, terrain, (refinery, false), ids);
            if owner == 0 {
                self.post_chat("Harvester on its way".to_string());
            }
        }
    }

    /// With crates on in a skirmish, drops one now and then somewhere on the ground nobody
    /// stands on; and in any battle pays whoever reaches one what is in it.
    fn drop_crates(&mut self, dt: f32) {
        if self.battle_kind == BattleKind::Skirmish && self.skirmish.crates {
            self.drop_crate(dt);
        }
        let opened = units::open_crates(&self.units, &mut self.crates, &mut self.money, dt);
        for (owner, found) in opened {
            let what = match found.gift {
                units::Gift::Money(money) => format!("${money}"),
                units::Gift::Unit(role) => {
                    self.let_out(owner, role, (found.x, found.z));
                    role.name().to_string()
                }
            };
            if owner == 0 {
                self.post_chat(format!("Crate: {what}"));
            }
        }
    }

    /// Lets a unit out of a crate for its opener, at the free spot nearest it.
    fn let_out(&mut self, owner: usize, role: Role, at: (f32, f32)) {
        let Some(color) = self.stats_rows().get(owner).map(|row| row.color) else {
            return;
        };
        let house = self.faction_of(owner);
        let terrain = battlefield::terrain(self.battle_map);
        let id = self.next_unit_id;
        let side = (owner, color);
        if units::put(
            &mut self.units,
            terrain,
            at,
            (role, house),
            side,
            (false, None, units::Anchor::Corner),
            id,
        )
        .is_some()
        {
            self.next_unit_id += 1;
        }
    }

    /// Drops a crate now and then somewhere on the ground nobody stands on.
    fn drop_crate(&mut self, dt: f32) {
        const CLEAR: f32 = 113.1;
        let terrain = battlefield::terrain(self.battle_map);
        self.crate_wait -= dt;
        if self.crate_wait <= 0.0 {
            let random = &mut self.crate_random;
            self.crate_wait = CRATE_SECONDS * (0.3 + 0.7 * battlefield::next_random(random));
            let (x, z) = terrain.random_point(random);
            let (least, most) = CRATE_MONEY;
            let steps = (battlefield::next_random(random) * (most / least) as f32) as i32;
            let clear = self
                .units
                .iter()
                .all(|u| (u.x - x).hypot(u.z - z) > CLEAR + u.radius());
            if self.crates.len() < CRATES_AT_MOST && terrain.can_walk(x, z) && clear {
                self.crates.push(units::Crate {
                    x,
                    z,
                    kind: units::CrateKind::Money,
                    gift: units::Gift::Money(least * (1 + steps)),
                    left: CRATE_LASTS,
                });
            }
        }
    }

    /// How much of this role the player's building has made so far, from 0 to 1, if it is
    /// making one.
    pub fn making_progress(&self, role: Role) -> Option<f32> {
        if let Site::Building(on_site, left) = self.site
            && on_site == role
        {
            return Some(1.0 - left / role.build_seconds());
        }
        self.units
            .iter()
            .filter(|u| u.alive && u.owner == 0)
            .find_map(|u| match u.making {
                Some((making, left)) if making == role => {
                    Some(1.0 - left / role.build_seconds().max(0.001))
                }
                _ => None,
            })
    }

    /// The number to show on this role's button: how many are ordered, once there is more than
    /// the one its dimming already shows.
    pub fn queue_badge(&self, role: Role) -> Option<usize> {
        // Walls show how many squares are still to go up, the one on its way among them.
        if role == Role::Wall && self.wall_plan.len() > 1 {
            return Some(self.wall_plan.len());
        }
        let orders: usize = self
            .units
            .iter()
            .filter(|u| u.alive && u.owner == 0)
            .map(|u| u.orders_for(role))
            .sum();
        let shown_by_dimming = usize::from(self.making_progress(role).is_some());
        (orders > shown_by_dimming).then_some(orders)
    }

    /// Lets everything being made come out at once.
    #[cfg(test)]
    fn finish_all_making(&mut self) {
        for unit in &mut self.units {
            if let Some((_, left)) = &mut unit.making {
                *left = 0.0;
            }
        }
        self.work_on_making(0.0);
    }

    /// Keeps the camera on the unit it follows, and lets go once that is no longer there, or
    /// has become a building, which there is no following.
    fn follow_with_camera(&mut self) {
        let Some(id) = self.following else {
            return;
        };
        match self.units.iter().find(|u| u.alive && u.id == id) {
            Some(unit) if !unit.kind.is_building() => self.camera.look_at(unit.x, unit.z),
            _ => self.following = None,
        }
    }

    /// Selects those of the player's wanted units that show on screen. If they are already
    /// selected, or none are on screen, selects them all across the map.
    fn select_among(&mut self, wanted: impl Fn(&Unit) -> bool) {
        let wanted = |u: &Unit| u.is_selectable() && wanted(u);
        let view = self.view();
        let (width, height) = (self.width, self.height);
        let on_screen = |u: &Unit| {
            view.project(u.x, u.pick_height(), u.z)
                .is_some_and(|(x, y)| {
                    (0.0..width).contains(&x)
                        && (0.0..height).contains(&y)
                        && !hud::covers(x, y, width, height)
                })
        };
        let whole_map = self
            .units
            .iter()
            .filter(|u| wanted(u) && on_screen(u))
            .all(|u| u.selected);
        for unit in &mut self.units {
            unit.selected = wanted(unit) && (whole_map || on_screen(unit));
        }
    }

    /// The number key a unit is under, the lowest one if it is under several.
    pub fn group_number(&self, id: u32) -> Option<usize> {
        self.control_groups
            .iter()
            .position(|group| group.contains(&id))
            .map(|group| group + 1)
    }

    /// With Control assigns the selected to a number key. Otherwise selects those still left
    /// on it, wherever they are, on top of the present selection with Shift. A key with none
    /// left changes nothing.
    fn use_control_group(&mut self, number: usize, modifier: Option<Modifier>) {
        if modifier == Some(Modifier::Control) {
            let selected: Vec<u32> = self.selected_units().iter().map(|u| u.id).collect();
            if !selected.is_empty() {
                self.control_groups[number - 1] = selected;
            }
            return;
        }
        let group = &self.control_groups[number - 1];
        let in_group = |u: &Unit| u.alive && u.is_selectable() && group.contains(&u.id);
        if !self.units.iter().any(in_group) {
            return;
        }
        let adding = modifier == Some(Modifier::Shift);
        for unit in &mut self.units {
            unit.selected = in_group(unit) || (adding && unit.selected);
        }
    }

    /// Whether the player has what makes this: the building it is made in, or for a building a
    /// deployed MCV, and what else it needs standing.
    pub fn can_produce(&self, role: Role) -> bool {
        self.units.iter().any(|u| u.owner == 0 && u.can_make(role))
            && role.needs().is_none_or(|needed| self.has_standing(needed))
    }

    /// Whether the player has a building of this role up and standing.
    fn has_standing(&self, role: Role) -> bool {
        self.units
            .iter()
            .any(|u| u.owner == 0 && u.alive && u.role == role && !u.is_under_construction())
    }

    /// The mini-houses the player is allied with: in a skirmish, as it was set up; otherwise
    /// none.
    pub fn player_allies(&self) -> [Ally; 2] {
        if self.battle_kind == BattleKind::Skirmish {
            self.skirmish.allies
        } else {
            [Ally::None; 2]
        }
    }

    /// Whether a build tab can be used: the player has something that makes any of what is on
    /// it, or for the Vehicles tab a palace, as the superweapon is there. Otherwise it shows
    /// greyed out.
    pub fn tab_enabled(&self, tab: BuildTab) -> bool {
        !self.menu(tab).is_empty()
    }

    /// Deploys the selected units that can, or packs them up, saying so if an MCV cannot deploy
    /// where it stands.
    fn toggle_deploy(&mut self) {
        let terrain = battlefield::terrain(self.battle_map);
        if units::toggle_deploy(&mut self.units, terrain) {
            self.post_chat(CANNOT_DEPLOY.to_string());
        }
    }

    /// Moves along the building on its way, while the player has a deployed MCV to build it and
    /// money to pay for it as it goes.
    fn work_on_site(&mut self, dt: f32) {
        if let Site::Building(role, left) = self.site
            && self.can_produce(role)
        {
            let dt = dt * units::work_pace(&self.units, 0);
            let left = units::work_and_pay(role, left, dt, &mut self.money[0]);
            self.site = if left > 0.0 {
                Site::Building(role, left)
            } else if role == Role::Wall {
                self.put_up_wall();
                if self.wall_plan.is_empty() {
                    Site::Idle
                } else {
                    Site::Building(role, role.build_seconds())
                }
            } else {
                self.post_chat(format!("{} ready to place", role.name()));
                self.announce(Speech::ReadyToPlace);
                Site::Ready(role)
            };
        }
    }

    /// Puts up the next square of the wall on its way, done and paid for, where it was planned:
    /// if something has come to stand there since, that square is left out and paid back.
    fn put_up_wall(&mut self) {
        let Some((u, v)) = self.wall_plan.pop_front() else {
            return;
        };
        let terrain = battlefield::terrain(self.battle_map);
        if !units::cell_is_free(&self.units, terrain, (u, v)) {
            self.money[0] += Role::Wall.cost();
            return;
        }
        let side = (0, self.player_color());
        let at = battlefield::grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
        let mut wall = units::new_building(terrain, at, Role::Wall, side, self.next_unit_id);
        wall.house = self.faction_of(0);
        wall.constructing = units::CONSTRUCTION_SECONDS;
        self.units.push(wall);
        self.next_unit_id += 1;
        units::join_walls(&mut self.units);
    }

    /// The color the player's side is drawn in.
    fn player_color(&self) -> TeamColor {
        self.stats_rows()
            .first()
            .map_or(TeamColor::Blue, |row| row.color)
    }

    /// Whether a square of wall could go up there for the player: free rock they have seen.
    fn can_wall(&self, (u, v): (i32, i32)) -> bool {
        let terrain = battlefield::terrain(self.battle_map);
        let (x, z) = battlefield::grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
        units::cell_is_free(&self.units, terrain, (u, v))
            && self.sight_at(x, z) != Sight::Unexplored
    }

    /// The square under the cursor, on the map's grid.
    fn square_under_cursor(&self) -> Option<(i32, i32)> {
        let (x, z) = self.land_under_cursor()?;
        let (u, v) = battlefield::world_to_grid(x, z);
        Some((u.floor() as i32, v.floor() as i32))
    }

    /// Whether a wall could start on this square: one it could go up on, near enough to one
    /// of the player's buildings.
    fn can_start_wall(&self, (u, v): (i32, i32)) -> bool {
        let plot = units::Plot {
            u,
            v,
            cols: 1,
            rows: 1,
        };
        self.can_produce(Role::Wall)
            && self.can_wall((u, v))
            && units::is_near_building_of(&self.units, plot, 0)
    }

    /// The squares the wall being placed would take, from where it starts to the cursor, once
    /// its start is picked.
    fn wall_route(&self) -> Option<Vec<(i32, i32)>> {
        let from = self.wall_start?;
        units::wall_route(from, self.square_under_cursor()?, |cell| {
            self.can_wall(cell)
        })
    }

    /// A click on the map while placing a wall: the first picks where it starts, the second
    /// where it ends, and the squares between go up one after another. Neither does anything
    /// where the wall cannot go.
    fn click_wall(&mut self) {
        let Some(cell) = self.square_under_cursor() else {
            return;
        };
        if self.wall_start.is_none() {
            if self.can_start_wall(cell) {
                self.wall_start = Some(cell);
            }
            return;
        }
        if let Some(route) = self.wall_route() {
            self.wall_plan = route.into();
            self.wall_start = None;
            self.site = Site::Building(Role::Wall, Role::Wall.build_seconds());
        }
    }

    /// Stops placing a wall: back to picking its start once that is picked, or not placing it
    /// at all.
    fn call_off_wall(&mut self) {
        if self.wall_start.take().is_none() {
            self.site = Site::Idle;
        }
    }

    /// A click on a building's button: starts it, or takes it up to place once it is ready.
    /// A wall is placed first, and then built a square at a time.
    fn click_building(&mut self, role: Role) {
        match self.site {
            Site::Idle if role == Role::Wall => {
                self.aiming_strike = false;
                self.site = Site::Placing(role);
                self.wall_start = None;
            }
            Site::Idle => {
                self.site = Site::Building(role, role.build_seconds());
                self.announce(Speech::ConstructionStarted);
            }
            Site::Ready(ready) if ready == role => {
                self.aiming_strike = false;
                self.site = Site::Placing(role);
                self.placing_facing = units::Facing::default();
                self.placing_press = None;
            }
            _ => {}
        }
    }

    /// Whether this building has to wait for another one on its way: only one goes up at a
    /// time, and there is no queue.
    pub fn is_held_up_by_site(&self, role: Role) -> bool {
        role.made_in().is_none() && self.site != Site::Idle && !self.site_of(role)
    }

    /// The building on its way, if it is of this role.
    fn site_of(&self, role: Role) -> bool {
        match self.site {
            Site::Idle => false,
            Site::Building(on_site, _) | Site::Ready(on_site) | Site::Placing(on_site) => {
                on_site == role
            }
        }
    }

    /// Whether this building is done and waiting to be placed.
    pub fn is_ready_to_place(&self, role: Role) -> bool {
        self.site == Site::Ready(role) || self.site == Site::Placing(role)
    }

    /// The building waiting to be placed and the grid squares it would take with the cursor
    /// where it is.
    /// With the button held down, a drag far enough towards one of the screen's corners turns
    /// what is being placed to look that way.
    fn turn_what_is_being_placed(&mut self) {
        const DRAG_TO_TURN: f32 = 24.0;
        let far_enough = DRAG_TO_TURN * ui_scale(self.height);
        let mouse = self.mouse;
        let Some(press) = &mut self.placing_press else {
            return;
        };
        let (dx, dy) = (mouse.0 - press.from.0, mouse.1 - press.from.1);
        if dx.hypot(dy) >= far_enough {
            press.turned = true;
            self.placing_facing = units::Facing::of_drag(dx, dy);
        }
    }

    /// Once a drag has turned the building being placed, its squares and the way it now looks,
    /// for an arrow pointing that way until the button comes up.
    pub fn placing_arrow(&self) -> Option<(units::Plot, (f32, f32))> {
        self.placing_press.filter(|press| press.turned)?;
        let (_, plot) = self.placing_plot()?;
        Some((plot, self.placing_facing.way_out()))
    }

    /// The place on the land itself under the cursor, up on what stands high too.
    pub fn land_under_cursor(&self) -> Option<(f32, f32)> {
        let terrain = battlefield::terrain(self.battle_map);
        self.view().land_at(terrain, self.mouse.0, self.mouse.1)
    }

    fn placing_plot(&self) -> Option<(Role, units::Plot)> {
        let Site::Placing(role) = self.site else {
            return None;
        };
        if role == Role::Wall {
            return None;
        }
        // Held down and dragged, the squares stay where they were pressed and only turn.
        let (x, y) = self.placing_press.map_or(self.mouse, |press| press.from);
        let terrain = battlefield::terrain(self.battle_map);
        let at = self.view().land_at(terrain, x, y)?;
        Some((
            role,
            role.plot_facing(self.faction_of(0), at, self.placing_facing),
        ))
    }

    /// The grid squares of the building being placed and what each shows: blocked unless it
    /// is free land the player has seen, near enough to one of their buildings.
    pub fn placing_squares(&self) -> Vec<((i32, i32), PlaceSquare)> {
        if self.site == Site::Placing(Role::Wall) {
            return self.wall_squares();
        }
        let Some((role, plot)) = self.placing_plot() else {
            return Vec::new();
        };
        let terrain = battlefield::terrain(self.battle_map);
        let reachable = self.can_produce(role) && units::is_near_building_of(&self.units, plot, 0);
        plot.cells()
            .map(|(u, v)| {
                let (x, z) = battlefield::grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
                let can = reachable
                    && units::cell_is_free(&self.units, terrain, (u, v))
                    && self.sight_at(x, z) != Sight::Unexplored;
                let square = if !can {
                    PlaceSquare::Blocked
                } else if role.is_exit(plot, self.placing_facing, (u, v)) {
                    PlaceSquare::Exit
                } else {
                    PlaceSquare::Free
                };
                ((u, v), square)
            })
            .collect()
    }

    /// What a wall being placed shows: the one square under the cursor until its start is
    /// picked, then the squares it would run along to the cursor, or where it starts and the
    /// cursor's square blocked if it cannot get there.
    fn wall_squares(&self) -> Vec<((i32, i32), PlaceSquare)> {
        let Some(cursor) = self.square_under_cursor() else {
            return Vec::new();
        };
        let Some(start) = self.wall_start else {
            let square = if self.can_start_wall(cursor) {
                PlaceSquare::Free
            } else {
                PlaceSquare::Blocked
            };
            return vec![(cursor, square)];
        };
        match self.wall_route() {
            Some(route) => route
                .into_iter()
                .map(|cell| (cell, PlaceSquare::Free))
                .collect(),
            None if cursor == start => vec![(start, PlaceSquare::Free)],
            None => vec![(start, PlaceSquare::Free), (cursor, PlaceSquare::Blocked)],
        }
    }

    /// The building about to be placed, on its squares, and whether it can go there.
    pub fn placing_preview(&self) -> Option<(Unit, bool)> {
        let (role, plot) = self.placing_plot()?;
        let terrain = battlefield::terrain(self.battle_map);
        let side = (0, self.stats_rows().first()?.color);
        let mut building =
            units::new_building(terrain, plot.center(), role, side, self.next_unit_id);
        building.house = self.faction_of(0);
        building.facing = self.placing_facing;
        let can = self
            .placing_squares()
            .iter()
            .all(|&(_, square)| square != PlaceSquare::Blocked);
        Some((building, can))
    }

    /// The tip text for what the cursor is over: a button making something, or a unit.
    pub fn tip(&self) -> Option<&'static str> {
        if !self.toggle_value(Toggle::ToolTips) {
            return None;
        }
        if let Some(Action::BuildCell(cell)) = self.hovered() {
            if self.build_entry(self.build_tab, cell) == Some(Entry::Superweapon) {
                return self.superweapon().map(Superweapon::name);
            }
            if let Some(Entry::Upgrade(role)) = self.build_entry(self.build_tab, cell) {
                return Some(match role {
                    Role::Refinery => "Refinery Pad",
                    Role::Mcv => "Upgrade Construction Yard",
                    _ => "Upgrade",
                });
            }
            return self.build_role(self.build_tab, cell).map(|role| {
                if self.can_produce(role) {
                    return role.name();
                }
                // What makes it stands, and has what it needs, but is not upgraded.
                let unmade = |u: &Unit| {
                    let mut upgraded = *u;
                    upgraded.upgrades = 1;
                    u.owner == 0 && !u.can_make(role) && upgraded.can_make(role)
                };
                if role.needs_upgraded_maker() && self.units.iter().any(unmade) {
                    return "Needs an upgrade";
                }
                match role.made_in().or_else(|| {
                    role.needs()
                        .filter(|_| self.units.iter().any(|u| u.owner == 0 && u.can_make(role)))
                }) {
                    Some(Role::Windtrap) => "Needs a Windtrap",
                    Some(Role::Barracks) => "Needs a Barracks",
                    Some(Role::Factory) => "Needs a Factory",
                    Some(_) => "Needs a Hangar",
                    None => "Needs a deployed MCV",
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
                s.can_fire_on(u)
                    || s.can_attack_when_told(u, &self.teams)
                    || s.can_tend(u, &self.teams)
            })
        };
        if let Some((unit, _)) = self.unit_near_cursor(able) {
            return Some(AttackOrder::Unit(unit));
        }
        if !selected.iter().any(|s| s.can_fire_at_ground()) {
            return None;
        }
        self.land_under_cursor().map(AttackOrder::Place)
    }

    /// What the cursor should look like: what a click here would make the selected units do.
    pub fn cursor_look(&self) -> CursorLook {
        let selected = self.selected_units();
        let look_at = |u: &Unit| {
            let armed = |s: &&Unit| {
                s.can_attack_when_told(u, &self.teams) || (self.control_held && s.can_fire_on(u))
            };
            if selected.iter().any(armed) {
                CursorLook::Attack
            } else if selected.iter().any(|s| s.can_fix(u, &self.teams)) {
                CursorLook::Heal
            } else {
                CursorLook::Carry
            }
        };
        match self.click_mode {
            Some(ClickMode::Sell) => return CursorLook::Sell,
            Some(ClickMode::Repair) => return CursorLook::Repair,
            Some(ClickMode::Route) if self.would_go_here() => return CursorLook::March,
            Some(ClickMode::March) if self.would_go_here() => return CursorLook::March,
            _ => {}
        }
        match self.forced_order_under_cursor() {
            Some(AttackOrder::Place(_)) => CursorLook::Attack,
            Some(AttackOrder::Unit(unit)) => look_at(&self.units[unit]),
            None => match self.attackable_under_cursor() {
                Some(unit) => look_at(&self.units[unit]),
                None if self.boardable_under_cursor().is_some() => CursorLook::Board,
                None if self.unloadable_under_cursor().is_some() => CursorLook::Deploy,
                None if self.deployable_under_cursor().is_some() => {
                    match self
                        .deployable_under_cursor()
                        .map(|u| self.units[u].deployed)
                    {
                        Some(true) => CursorLook::Undeploy,
                        _ => CursorLook::Deploy,
                    }
                }
                None if self.pad_under_cursor().is_some() => CursorLook::Land,
                None if self.refinery_under_cursor().is_some() => CursorLook::Land,
                None if self.harvest_here().is_some() => CursorLook::Harvest,
                None if self.would_put_down_here() => CursorLook::Carry,
                None if self.would_go_here() => CursorLook::Move,
                None => CursorLook::Arrow,
            },
        }
    }

    /// The player's building under the cursor, finished or not, that a click would sell.
    fn sellable_under_cursor(&self) -> Option<usize> {
        self.unit_under_cursor()
            .filter(|&i| self.units[i].owner == 0 && self.units[i].kind.is_building())
    }

    /// Puts a button's click mode on, or off if it is on.
    fn toggle_mode(&mut self, mode: ClickMode) {
        self.click_mode = (self.click_mode != Some(mode)).then_some(mode);
    }

    /// Whether a sidebar button shows lit: its click mode is on, the retreat is waiting for
    /// its second click, or everyone selected who can is in guard mode.
    pub fn hud_small_lit(&self, button: usize) -> bool {
        match button {
            SELL_BUTTON => self.click_mode == Some(ClickMode::Sell),
            REPAIR_BUTTON => self.click_mode == Some(ClickMode::Repair),
            WAYPOINT_BUTTON => self.click_mode == Some(ClickMode::Route),
            RETREAT_BUTTON => self.retreat_armed > 0.0,
            GUARD_BUTTON => {
                let mut guards = self
                    .units
                    .iter()
                    .filter(|u| u.selected && !u.kind.is_building() && u.weapon.is_armed())
                    .peekable();
                guards.peek().is_some() && guards.all(|u| u.guard.is_some())
            }
            _ => false,
        }
    }

    /// The retreat button: the first click asks, and a second one soon after gives the
    /// battle up.
    fn retreat(&mut self) {
        if self.retreat_armed > 0.0 {
            self.retreat_armed = 0.0;
            self.outcome = Some(Outcome::Defeat);
            self.announce(Speech::Retreat);
            self.key_paused = false;
            self.aiming_strike = false;
            self.outcome_elapsed = 0.0;
        } else {
            self.retreat_armed = RETREAT_CONFIRM_SECONDS;
            self.post_chat("Retreat all units from the map? Click again to retreat".to_string());
        }
    }

    /// A click on the map with the repair button down: the player's building there starts
    /// being mended if it is hurt, or stops.
    fn click_repair(&mut self) {
        if let Some(building) = self.sellable_under_cursor() {
            let unit = &mut self.units[building];
            unit.repairing = !unit.repairing && unit.health < unit.max_health();
        }
    }

    /// A click on the map with the waypoint button down: the selected who can move go there
    /// if they are on no route yet, and otherwise once they have been to the waypoints before
    /// it.
    fn click_waypoint(&mut self) {
        let Some(point) = self.land_under_cursor() else {
            return;
        };
        let ids: Vec<u32> = self
            .units
            .iter()
            .filter(|u| u.selected && u.owner == 0 && !u.kind.is_building())
            .map(|u| u.id)
            .collect();
        if ids.is_empty() {
            return;
        }
        match self.routes.iter_mut().find(|route| route.ids == ids) {
            Some(route) => {
                route.ahead.push_back(point);
                self.speeches.push(Speech::WaypointAdded);
            }
            None => {
                // Whoever of them was on another route leaves it.
                self.leave_routes(&ids);
                let terrain = battlefield::terrain(self.battle_map);
                units::order_move(&mut self.units, point, terrain, &self.teams);
                self.routes.push(Route {
                    ids,
                    ahead: std::collections::VecDeque::new(),
                });
                self.announce(Speech::WaypointPlaced);
                self.answer(Said::Moving);
            }
        }
    }

    /// Takes these units off whatever routes they are on.
    fn leave_routes(&mut self, ids: &[u32]) {
        for route in &mut self.routes {
            route.ids.retain(|id| !ids.contains(id));
        }
        self.routes.retain(|route| !route.ids.is_empty());
    }

    /// Sends the units of each route on to its next waypoint once they have all come to the
    /// one before, and forgets routes that are done or whose units are gone.
    fn follow_routes(&mut self) {
        let terrain = battlefield::terrain(self.battle_map);
        for at in 0..self.routes.len() {
            let ids = self.routes[at].ids.clone();
            let on_route = |u: &Unit| u.alive && ids.contains(&u.id);
            if self.units.iter().any(|u| on_route(u) && u.is_moving()) {
                continue;
            }
            let Some(next) = self.routes[at].ahead.pop_front() else {
                self.routes[at].ids.clear();
                continue;
            };
            // Sent as a click with only them selected would send them.
            let selected: Vec<bool> = self.units.iter().map(|u| u.selected).collect();
            for unit in &mut self.units {
                unit.selected = on_route(unit);
            }
            units::order_move(&mut self.units, next, terrain, &self.teams);
            for (unit, was) in self.units.iter_mut().zip(selected) {
                unit.selected = was;
            }
        }
        self.routes.retain(|route| {
            !route.ids.is_empty()
                && self
                    .units
                    .iter()
                    .any(|u| u.alive && route.ids.contains(&u.id))
        });
    }

    /// The waypoints ahead of the selected units' routes, for drawing.
    pub fn waypoints(&self) -> Vec<(f32, f32)> {
        self.routes
            .iter()
            .filter(|route| {
                self.units
                    .iter()
                    .any(|u| u.selected && route.ids.contains(&u.id))
            })
            .flat_map(|route| route.ahead.iter().copied())
            .collect()
    }

    /// Sells the player's building for half what it cost: it is gone, and the money theirs.
    fn sell(&mut self, building: usize) {
        let sold = self.units.remove(building);
        let money = sold.role.cost() / 2;
        self.money[0] += money;
        self.post_chat(format!("{} sold: ${money}", sold.role.name()));
        self.announce(Speech::BuildingSold);
    }

    /// The selected unit under the cursor that a click would deploy or bring back from that.
    fn deployable_under_cursor(&self) -> Option<usize> {
        if !self.can_order_attack_here() {
            return None;
        }
        // An MCV on the lower ground cannot deploy there, so it offers no deploy cursor.
        let terrain = battlefield::terrain(self.battle_map);
        self.unit_under_cursor().filter(|&unit| {
            let unit = &self.units[unit];
            unit.selected && unit.role.can_deploy() && units::can_deploy_here(unit, terrain)
        })
    }

    /// The APC under the cursor that a click would send the selected soldiers into.
    fn boardable_under_cursor(&self) -> Option<usize> {
        if !self.can_order_attack_here() {
            return None;
        }
        let selected = self.selected_units();
        self.unit_under_cursor().filter(|&apc| {
            let apc = &self.units[apc];
            selected.iter().any(|s| units::can_board(s, apc))
        })
    }

    /// The selected APC under the cursor that a click would let everyone on board out of.
    fn unloadable_under_cursor(&self) -> Option<usize> {
        if !self.can_order_attack_here() {
            return None;
        }
        self.unit_under_cursor().filter(|&apc| {
            let apc = &self.units[apc];
            apc.selected && apc.passengers.iter().any(Option::is_some)
        })
    }

    /// The player's landing pad under the cursor, if a click would land a selected machine on it.
    fn pad_under_cursor(&self) -> Option<usize> {
        if !self.can_order_attack_here() || !self.selected_units().iter().any(|u| u.role.lands()) {
            return None;
        }
        self.unit_under_cursor()
            .filter(|&unit| self.units[unit].role == Role::Helipad)
    }

    /// The player's refinery under the cursor, if a click would send a selected harvester to
    /// empty its load there.
    fn refinery_under_cursor(&self) -> Option<usize> {
        let loaded = |u: &&Unit| u.role == Role::Harvester && u.load > 0.0;
        if !self.can_order_attack_here() || !self.selected_units().iter().any(loaded) {
            return None;
        }
        self.unit_under_cursor().filter(|&unit| {
            let refinery = &self.units[unit];
            refinery.role == Role::Refinery && !refinery.is_under_construction()
        })
    }

    /// The sand under the cursor, if what is selected is harvesters that a click would send to
    /// harvest it.
    fn harvest_here(&self) -> Option<(f32, f32)> {
        let selected = self.selected_units();
        let movers = || selected.iter().filter(|u| !u.kind.is_building());
        if !self.can_order_attack_here()
            || movers().next().is_none()
            || movers().any(|u| u.role != Role::Harvester)
            || self.unit_under_cursor().is_some()
        {
            return None;
        }
        let terrain = battlefield::terrain(self.battle_map);
        self.land_under_cursor()
            .filter(|&at| units::has_substance(terrain, at))
    }

    /// Whether a click here would send somewhere any of what is selected: whatever can move, or
    /// a building that makes units, whose gathering point it would be.
    fn would_go_here(&self) -> bool {
        let goes = |u: &&Unit| {
            !u.deployed
                && match u.kind {
                    UnitKind::Tower => false,
                    UnitKind::Building => u.role.takes_gathering_point(),
                    _ => u.role.goes_where_told(),
                }
        };
        self.can_order_attack_here()
            && self.selected_units().iter().any(goes)
            && self.unit_under_cursor().is_none()
            && self.view().ground_at(self.mouse.0, self.mouse.1).is_some()
    }

    /// Whether a click would send a selected carryall here to put down what it holds.
    fn would_put_down_here(&self) -> bool {
        self.can_order_attack_here()
            && self
                .selected_units()
                .iter()
                .any(|u| u.carrying && u.role.goes_where_told())
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
        // A drag has only turned what is being placed; the click to put it down is still to come.
        if self.placing_press.take().is_some_and(|press| press.turned) {
            self.selection_anchor = None;
            return;
        }
        if self.site == Site::Placing(Role::Wall) {
            self.click_wall();
            self.selection_anchor = None;
            return;
        }
        if let Some((mut building, true)) = self.placing_preview() {
            building.constructing = units::CONSTRUCTION_SECONDS;
            self.units.push(building);
            self.next_unit_id += 1;
            self.site = Site::Idle;
            self.selection_anchor = None;
            return;
        }
        if matches!(self.site, Site::Placing(_)) {
            // Not there. It stays up until a good click, or a right click.
            self.selection_anchor = None;
            return;
        }
        if self.click_mode == Some(ClickMode::Sell) {
            // It stays down for the next sale until it is let go.
            if let Some(building) = self.sellable_under_cursor() {
                self.sell(building);
            }
            self.selection_anchor = None;
            return;
        }
        if self.click_mode == Some(ClickMode::Repair) {
            self.click_repair();
            self.selection_anchor = None;
            return;
        }
        if self.click_mode == Some(ClickMode::Route) {
            self.click_waypoint();
            self.selection_anchor = None;
            return;
        }
        if self.aiming_strike {
            let target = self.land_under_cursor();
            if let (Some(weapon), Some((target_x, target_z))) = (self.superweapon(), target) {
                self.combat.launch_superweapon(weapon, target_x, target_z);
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
            self.answer(Said::Attacking);
        } else if let Some(chosen) = self.attackable_under_cursor() {
            units::order_attack(&mut self.units, chosen, &self.teams);
            self.answer(Said::Attacking);
        } else if let Some(apc) = self.boardable_under_cursor().filter(|_| !adding) {
            units::order_board(&mut self.units, apc);
        } else if let Some(apc) = self.unloadable_under_cursor().filter(|_| !adding) {
            let terrain = battlefield::terrain(self.battle_map);
            units::unload(&mut self.units, terrain, apc);
        } else if let Some(unit) = self.deployable_under_cursor().filter(|_| !adding) {
            // So two clicks on one that is not selected yet select it, then switch it.
            units::start_deploying(&mut self.units[unit]);
        } else if let Some(pad) = self.pad_under_cursor().filter(|_| !adding) {
            units::order_land(&mut self.units, pad);
        } else if let Some(refinery) = self.refinery_under_cursor().filter(|_| !adding) {
            units::order_dump(&mut self.units, refinery);
        } else if let Some(sand) = self.harvest_here().filter(|_| !adding) {
            let terrain = battlefield::terrain(self.battle_map);
            units::order_harvest(&mut self.units, sand, terrain, &self.teams);
            self.answer(Said::Moving);
        } else if let Some(picked) = self.unit_under_cursor() {
            // A building, tower or not, is always picked on its own.
            let adding = adding && !self.units[picked].kind.is_building();
            for (i, unit) in self.units.iter_mut().enumerate() {
                let stays = adding && unit.selected && !unit.kind.is_building();
                unit.selected = i == picked || stays;
            }
        } else if adding {
            // Nothing picked, nothing to add.
        } else if let Some(target) = self.land_under_cursor() {
            let terrain = battlefield::terrain(self.battle_map);
            // Sent somewhere else, they leave the route and the post they were on.
            let sent: Vec<u32> = self
                .units
                .iter()
                .filter(|u| u.selected)
                .map(|u| u.id)
                .collect();
            self.leave_routes(&sent);
            for unit in self.units.iter_mut().filter(|u| u.selected) {
                unit.guard = None;
            }
            if self.click_mode == Some(ClickMode::March) {
                units::order_march(&mut self.units, target, terrain, &self.teams);
            } else {
                units::order_move(&mut self.units, target, terrain, &self.teams);
            }
            units::set_gathering(&mut self.units, target);
            self.answer(Said::Moving);
        }
        if self.click_mode == Some(ClickMode::March) {
            self.click_mode = None;
        }
        self.selection_anchor = None;
    }

    /// The nearest of the player's units to the cursor, within reach of a click.
    fn unit_under_cursor(&self) -> Option<usize> {
        self.unit_near_cursor(Unit::is_selectable).map(|(i, _)| i)
    }

    /// The wanted unit the cursor picks, and how far its middle is. What has its middle within reach comes first, then a machine or a
    /// soldier the cursor is on, then a building; of those the one with its middle nearest.
    fn unit_near_cursor(&self, wanted: impl Fn(&Unit) -> bool) -> Option<(usize, f32)> {
        let view = self.view();
        let reach = PICK_RADIUS * ui_scale(self.height);
        let wanted = |unit: &Unit| self.shows(unit) && wanted(unit);
        self.units
            .iter()
            .enumerate()
            .filter(|(_, unit)| wanted(unit))
            .filter_map(|(i, unit)| {
                let (x, y) = view.project(unit.x, unit.pick_height(), unit.z)?;
                let distance = (x - self.mouse.0).hypot(y - self.mouse.1);
                let on = if unit.kind.is_building() {
                    self.cursor_is_on_building(&view, unit)
                } else {
                    self.cursor_is_on_box(&view, unit)
                };
                let rank = match (distance <= reach, on) {
                    (true, _) => 0,
                    (false, true) if !unit.kind.is_building() => 1,
                    (false, true) => 2,
                    (false, false) => return None,
                };
                Some((rank, i, distance))
            })
            .min_by(|a, b| a.0.cmp(&b.0).then(a.2.total_cmp(&b.2)))
            .map(|(_, i, distance)| (i, distance))
    }

    /// Whether the cursor is anywhere on a machine or a soldier as it shows: in the box the
    /// original selects it by.
    fn cursor_is_on_box(&self, view: &battlefield::View, unit: &Unit) -> bool {
        let corners: Option<Vec<(f32, f32)>> = unit
            .selection_box()
            .iter()
            .map(|&(x, y, z)| view.project(x, y, z))
            .collect();
        corners.is_some_and(|corners| {
            is_on_box(
                std::array::from_fn(|i| corners[i]),
                std::array::from_fn(|i| corners[i + 4]),
                self.mouse,
            )
        })
    }

    /// Whether the cursor is anywhere on a building as it shows: on its base, its roof or the
    /// walls between them.
    fn cursor_is_on_building(&self, view: &battlefield::View, unit: &Unit) -> bool {
        const WALLS_SHARE_OF_TOP: f32 = 0.7;
        let plot = unit
            .role
            .plot_facing(unit.house, (unit.x, unit.z), unit.facing);
        let (low_u, low_v) = (plot.u as f32, plot.v as f32);
        let (high_u, high_v) = (low_u + plot.cols as f32, low_v + plot.rows as f32);
        let ground = [
            (low_u, low_v),
            (high_u, low_v),
            (high_u, high_v),
            (low_u, high_v),
        ]
        .map(|(u, v)| battlefield::grid_to_world(u, v));
        let top = unit.y + unit.role.building_top(unit.house) * WALLS_SHARE_OF_TOP;
        let corners_at = |y: f32| -> Option<Vec<(f32, f32)>> {
            ground.iter().map(|&(x, z)| view.project(x, y, z)).collect()
        };
        let (Some(base), Some(roof)) = (corners_at(unit.y), corners_at(top)) else {
            return false;
        };
        is_on_box(
            std::array::from_fn(|i| base[i]),
            std::array::from_fn(|i| roof[i]),
            self.mouse,
        )
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

    /// The great house an owner's machines are built by. Outside a skirmish the enemy is of
    /// the house after the player's.
    pub fn faction_of(&self, owner: usize) -> Faction {
        let player = match self.battle_kind {
            BattleKind::Skirmish => self.skirmish.faction,
            _ => self.faction.unwrap_or(Faction::Atreides),
        };
        let campaign_enemy = || {
            let mission = self.mission?;
            Some(self.campaign.as_ref()?.enemy(mission))
        };
        match (self.battle_kind, owner) {
            (_, 0) => player,
            (BattleKind::Skirmish, _) => self.skirmish.opponents[owner - 1].faction,
            (BattleKind::Campaign, _) => campaign_enemy().unwrap_or(player.next()),
            _ => player.next(),
        }
    }

    /// The player, then each opponent in this battle.
    pub fn stats_rows(&self) -> Vec<StatsRow> {
        let setup = &self.skirmish;
        if self.battle_kind != BattleKind::Skirmish {
            let color = match self.faction {
                Some(Faction::Harkonnen) => TeamColor::Red,
                Some(Faction::Ordos) => TeamColor::Cyan,
                Some(Faction::Atreides) | None => TeamColor::Blue,
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
        let mut lost_speeches = Vec::new();
        for unit in self.units.iter().filter(|u| !u.alive) {
            // Up in the air and not hit hard, it comes down in one piece. Otherwise it goes up
            // here, the harder it was hit past dead the more so.
            let overkill = unit.overkill();
            if unit.kind == UnitKind::Infantry {
                self.combat.drop_body(unit);
            } else if unit.kind == UnitKind::Vehicle {
                self.combat.blow_up_vehicle(unit);
            } else if unit.kind.flies() && !unit.is_grounded() && overkill < OVERKILL_BLOWS_APART {
                self.combat.shoot_down(unit);
            } else {
                self.combat
                    .explode_destroyed(unit.center(), unit.radius() * 2.0, overkill);
                let made = Made::Died {
                    wound: unit.wound,
                    second: false,
                };
                self.combat.hear(unit, made);
            }
            // Whoever it has on board dies with it.
            let aboard = unit.passengers.iter().flatten().count() as i32;
            if unit.owner == 0 {
                self.lost.count(unit);
                self.lost.infantry += aboard;
                if unit.role.counts_for_stats() {
                    lost_names.push(unit.name());
                    lost_speeches.push(if unit.kind.is_building() {
                        Speech::BuildingLost
                    } else {
                        Speech::UnitLost
                    });
                }
            } else if units::are_enemies(&self.teams, 0, unit.owner) {
                self.killed.count(unit);
                self.killed.infantry += aboard;
            }
        }
        self.units.retain(|u| u.alive);
        units::join_walls(&mut self.units);
        for name in lost_names {
            self.post_chat(format!("{name} lost"));
        }
        for speech in lost_speeches {
            self.announce(speech);
        }
        self.report_defeats();
        if self.outcome.is_some() {
            return;
        }
        let standing = || self.units.iter().filter(|u| u.role.counts_for_losing());
        let player_left = standing().any(|u| u.owner == 0);
        let enemies_left = standing().any(|u| units::are_enemies(&self.teams, 0, u.owner));
        if !player_left {
            self.decide(Outcome::Defeat);
        } else if !enemies_left {
            self.decide(Outcome::Victory);
        }
    }

    /// The battle is decided, unless it already was.
    fn decide(&mut self, outcome: Outcome) {
        if self.outcome.is_some() {
            return;
        }
        self.outcome = Some(outcome);
        // The side that has won cheers: the player's on a victory, their enemies' on a defeat.
        let teams = &self.teams;
        units::cheer(&mut self.units, |owner| {
            let enemy = units::are_enemies(teams, 0, owner);
            teams.get(owner).is_some() && enemy == (outcome == Outcome::Defeat)
        });
        self.announce(match outcome {
            Outcome::Victory => Speech::Victory,
            Outcome::Defeat => Speech::Defeat,
        });
        self.key_paused = false;
        self.aiming_strike = false;
        self.outcome_elapsed = 0.0;
    }

    /// Whether this side has lost everything, the player being owner 0.
    pub fn is_defeated(&self, owner: usize) -> bool {
        self.reported_defeated.get(owner) == Some(&true)
    }

    /// Posts a message for each side that has just lost everything.
    fn report_defeats(&mut self) {
        let rows = self.stats_rows();
        for (owner, row) in rows.iter().enumerate() {
            let beaten = !self
                .units
                .iter()
                .any(|u| u.owner == owner && u.role.counts_for_losing());
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

    /// Past the statistics, each kind of battle returns to where it belongs. A campaign goes
    /// back to its map with what the battle won or lost, or straight into its first battle
    /// again if that was lost.
    fn leave_battle_stats(&mut self) {
        match self.battle_kind {
            BattleKind::Skirmish => self.set_screen(Screen::SkirmishSetup),
            BattleKind::Tutorial => self.set_screen(Screen::SinglePlayerMenu),
            BattleKind::Campaign => {
                let won = self.outcome == Some(Outcome::Victory);
                if let (Some(campaign), Some(mission)) = (&mut self.campaign, self.mission.take()) {
                    campaign.finish(mission, won);
                }
                self.picked = None;
                match self.campaign.as_ref().map(Campaign::next) {
                    Some(Next::Fight(Mission::Start)) => self.fight(Mission::Start),
                    _ => self.set_screen(Screen::CampaignMap),
                }
            }
        }
    }

    /// The question the campaign's map asks now, if any: whether to defend a territory, or
    /// which side to take.
    pub fn question(&self) -> Option<Next> {
        (self.screen == Screen::CampaignMap).then_some(())?;
        let next = self.campaign.as_ref()?.next();
        matches!(next, Next::Defend(..) | Next::Side).then_some(next)
    }

    /// What the question says.
    pub fn question_text(&self) -> Option<String> {
        let data = campaign::data();
        match self.question()? {
            Next::Defend(at, by) => {
                let name = data?.territories.get(at)?.name.as_str();
                Some(format!("{} attacks {name}. Defend it?", by.label()))
            }
            // The original's own words for it.
            _ => Some("Choose Side".to_string()),
        }
    }

    /// The territory of the campaign's map under this point of the screen, by its shape.
    fn territory_at(&self, point: (f32, f32)) -> Option<usize> {
        let place = campaign_place(self.width, self.height);
        campaign::data()?.territories.iter().position(|territory| {
            territory
                .fill
                .iter()
                .any(|&triangle| is_in_triangle(triangle.map(&place), point))
        })
    }

    /// The battle that follows the campaign map's Attack: the territory picked, if it can be
    /// attacked, or the mission the campaign has come to.
    pub fn next_battle(&self) -> Option<Mission> {
        let campaign = self.campaign.as_ref()?;
        match campaign.next() {
            Next::Pick => campaign.attack(self.picked?),
            Next::Fight(mission) => Some(mission),
            _ => None,
        }
    }

    /// Opens the briefing of a battle, after the units of a tech level not yet shown.
    fn brief(&mut self, mission: Mission) {
        self.briefing = Some(mission);
        self.set_screen(Screen::Briefing);
        if self
            .campaign
            .as_ref()
            .is_some_and(|c| c.tech > self.units_shown)
        {
            self.show_new_units();
        }
    }

    fn show_new_units(&mut self) {
        if let Some(campaign) = &self.campaign {
            self.units_shown = campaign.tech;
            self.set_screen(Screen::NewUnits);
        }
    }

    /// Fights a battle of the campaign, on the original's map for it if the disc has it.
    fn fight(&mut self, mission: Mission) {
        let Some(campaign) = &self.campaign else {
            return;
        };
        let (house, phase) = (campaign.house, campaign.phase);
        let map = maps::by_id(&campaign.map_id(mission)).unwrap_or(0);
        self.mission = Some(mission);
        self.briefing = None;
        self.start_battle(map, Some(house), BattleKind::Campaign);
        // The original's first mission gives the player 10 000 credits.
        if mission == Mission::Start {
            self.money[0] = CAMPAIGN_START_MONEY;
            if let Some(lines) = campaign::start_script(house) {
                self.start_script(lines);
            }
        }
        // A battle for a territory runs its own script, where the disc has one for the phase,
        // and one past the territories its own.
        let script = match mission {
            Mission::Attack(at) => campaign::mission_script(house, phase, false, at),
            Mission::Defend(at, _) => campaign::mission_script(house, phase, true, at),
            Mission::Start => None,
            _ => campaign::battle_script(house, mission),
        };
        if let Some(lines) = script {
            self.start_script(lines);
        }
    }

    /// Starts the battle as its mission's script has it, in place of a base a side: the
    /// script puts down what each side has.
    fn start_script(&mut self, lines: &[Vec<Token>]) {
        self.units.clear();
        self.next_unit_id = 0;
        self.script_updates = 0;
        self.script_clock = 0.0;
        self.script_sides = 0;
        self.script_side_of.clear();
        self.script_enemies.clear();
        // What the map has standing is there before the script first looks: its palace, say,
        // which a script wins on once it is gone.
        self.place_map_objects();
        let mut script = Script::new(lines);
        script.run(self);
        self.script = Some(script);
        units::join_walls(&mut self.units);
        if let Some(&Unit { x, z, .. }) = self.units.iter().find(|u| u.owner == 0) {
            self.home = (x, z);
            self.camera.look_at(x, z);
        }
        self.update_fog();
    }

    /// Puts down what the battle's map has standing on it from the start, of what we know: its
    /// walls, buildings and units, each the player's if of the player's house and the enemy's
    /// if not; its barrels, wrecks and such, which are nobody's; and its crates.
    fn place_map_objects(&mut self) {
        let terrain = battlefield::terrain(self.battle_map);
        let player = self.faction_of(0);
        for object in terrain.objects() {
            let (column, row) = object.cell;
            let at = battlefield::grid_to_world(column as f32 + 0.5, row as f32 + 0.5);
            if let Some(kind) = units::CrateKind::named(&object.name) {
                self.crates.push(units::Crate {
                    x: at.0,
                    z: at.1,
                    kind,
                    gift: kind.gift(),
                    left: f32::INFINITY,
                });
                continue;
            }
            let Some((role, house)) = Role::named(&object.name) else {
                continue;
            };
            // A sub-house's, of no great house, is the enemy's.
            let owner = match role {
                _ if role.is_incidental() => units::NEUTRAL,
                Role::Extra(extra) if extra.house().is_none() => 1,
                _ => usize::from(house != player),
            };
            let color = match self.stats_rows().get(owner) {
                Some(row) => row.color,
                None if owner == units::NEUTRAL => TeamColor::Yellow,
                None => continue,
            };
            let dug_in = object.name.ends_with(CONSTRUCTION_YARD);
            let facing = Some(Facing::numbered(object.facing));
            let id = self.next_unit_id;
            let placed = units::put(
                &mut self.units,
                terrain,
                at,
                (role, house),
                (owner, color),
                (dug_in, facing, units::Anchor::Corner),
                id,
            );
            if placed.is_some() {
                self.next_unit_id += 1;
            }
        }
    }

    /// Runs the battle's script for each of its updates `dt` seconds bring, until the battle
    /// is decided.
    fn run_script(&mut self, dt: f32) {
        let Some(mut script) = self.script.take() else {
            return;
        };
        self.script_clock += dt * SCRIPT_UPDATES_PER_SECOND;
        while self.script_clock >= 1.0 && self.outcome.is_none() {
            self.script_clock -= 1.0;
            self.script_updates += 1;
            script.run(self);
        }
        self.script = Some(script);
    }

    /// The lines of speech the battle's script has asked for since last asked.
    pub fn take_mission_lines(&mut self) -> Vec<u16> {
        std::mem::take(&mut self.mission_lines)
    }

    /// The point of the battle's map of this name, or where it would be on a map without it:
    /// a base at a start, anything else in the middle.
    fn map_point(&self, name: &str) -> (f32, f32) {
        let terrain = battlefield::terrain(self.battle_map);
        let start = |n: usize| terrain.starts().get(n).copied();
        let (right, bottom) = terrain.extent();
        terrain
            .point(name)
            .or(match name {
                PLAYER_BASE_POINT => start(0),
                ENEMY_BASE_POINT => start(1),
                _ => None,
            })
            .unwrap_or((right / 2.0, bottom / 2.0))
    }

    /// Puts what the script asks for on the map for one of its sides: the player's own, any
    /// other the enemy's. Exactly where, or else at the free spot nearest.
    /// Puts what a script names on the map: placed exactly, facing a way, or brought in.
    fn put_scripted(
        &mut self,
        side: i32,
        kind: u16,
        at: (f32, f32),
        facing: Option<Facing>,
    ) -> Option<u32> {
        let name = campaign::kind_name(kind)?;
        let thing = Role::named(name)?;
        let (owner, color) = self.side_owner(side)?;
        let dug_in = facing.is_some() && name.ends_with(CONSTRUCTION_YARD);
        let anchor = if dug_in {
            units::Anchor::Yard
        } else {
            units::Anchor::Corner
        };
        let terrain = battlefield::terrain(self.battle_map);
        let id = self.next_unit_id;
        let placed = units::put(
            &mut self.units,
            terrain,
            at,
            thing,
            (owner, color),
            (dug_in, facing, anchor),
            id,
        )?;
        self.next_unit_id += 1;
        self.script_side_of.push((placed, side));
        Some(placed)
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
        if let Some((x, z)) =
            battlefield::minimap_to_world(rect, self.map_extent(), self.mouse.0, self.mouse.1)
        {
            self.minimap_dragging = true;
            self.camera.look_at(x, z);
            self.following = None;
        }
    }

    /// Follows the cursor until release, holding to the minimap's edge if it wanders off.
    fn drag_battle_minimap(&mut self) {
        if !self.minimap_dragging || self.key_paused {
            return;
        }
        self.following = None;
        let rect = hud::minimap_rect(self.width, self.height);
        let (x, y, size) = rect;
        let px = self.mouse.0.clamp(x, x + size - 0.01);
        let py = self.mouse.1.clamp(y, y + size - 0.01);
        if let Some((world_x, world_z)) =
            battlefield::minimap_to_world(rect, self.map_extent(), px, py)
        {
            self.camera.look_at(world_x, world_z);
        }
    }

    /// Where the H key sends the selected: the place the player's base started.
    fn home(&self) -> (f32, f32) {
        self.home
    }

    /// How far the battle's map reaches along x and along z.
    pub fn map_extent(&self) -> (f32, f32) {
        battlefield::terrain(self.battle_map).extent()
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
        self.camera = Camera::centered_on(battlefield::terrain(map).extent());
        self.scene_playing = false;
        self.camera.open();
        self.in_battle = true;
        self.pausable = true;
        self.key_paused = false;
        self.stats_open = false;
        self.battle_time = 0.0;
        self.build_tab = BuildTab::Infantry;
        self.money = if skirmish {
            vec![self.skirmish.money; 1 + self.skirmish.ai_players as usize]
        } else {
            vec![DEFAULT_BATTLE_MONEY; 2]
        };
        self.refineries_served.clear();
        self.short_of_harvesters = vec![0.0; self.money.len()];
        self.crates.clear();
        self.crate_wait = CRATE_SECONDS / 2.0;
        self.crate_random = map as u32 * 2 + 7;
        self.chat.clear();
        let colors: Vec<TeamColor> = self.stats_rows().iter().map(|row| row.color).collect();
        self.teams = (0..colors.len()).collect();
        self.reported_defeated = vec![false; colors.len()];
        self.units = units::spawn(battlefield::terrain(map), &colors, &mut self.start_random);
        if let Some(&Unit { x, z, .. }) = self.units.first() {
            self.home = (x, z);
            self.camera.look_at(x, z);
        }
        let houses: Vec<Faction> = (0..colors.len()).map(|o| self.faction_of(o)).collect();
        for unit in &mut self.units {
            unit.house = houses[unit.owner];
        }
        self.next_unit_id = self.units.len() as u32;
        self.fog = self.fog_allowed().then(Fog::new);
        self.update_fog();
        self.carried.clear();
        self.control_groups = Default::default();
        self.following = None;
        self.site = Site::Idle;
        self.wall_start = None;
        self.wall_plan.clear();
        self.opponent_random = map as u32 * 2 + 1;
        self.strategies = (1..colors.len())
            .map(|owner| {
                let kind = match self.battle_kind {
                    BattleKind::Skirmish => self.skirmish.opponents[owner - 1].kind,
                    // The tutorial's enemy takes its time.
                    BattleKind::Tutorial => OpponentKind::Defensive,
                    BattleKind::Campaign => OpponentKind::Competent,
                };
                Strategy::new(owner, kind)
            })
            .collect();
        self.combat = Combat::default();
        self.aiming_strike = false;
        self.click_mode = None;
        self.routes.clear();
        self.market = Market::open(self.faction_of(0), self.start_random);
        self.speeches.clear();
        self.last_said.clear();
        self.health_seen.clear();
        self.was_short_of_power = false;
        self.constructing_seen.clear();
        self.retreat_armed = 0.0;
        self.strike_recharge = 0.0;
        self.script = None;
        self.mission_lines.clear();
        self.set_screen(Screen::Loading);
    }

    pub fn menu_seconds(&self) -> f32 {
        self.menu_seconds
    }

    /// Keeps the intro's or a faction's movie screen up for as long as the movie now starting
    /// on it plays.
    pub fn start_movie(&mut self) {
        self.movie_playing = matches!(
            self.screen,
            Screen::Intro | Screen::Movie(_) | Screen::Opening(_)
        );
    }

    /// Starts a house's campaign with its first battle.
    fn start_campaign(&mut self, house: Faction) {
        self.campaign = Some(Campaign::new(house, self.start_random));
        self.units_shown = 0;
        self.picked = None;
        self.fight(Mission::Start);
    }

    /// Goes straight into a territory's battle by its script's name on the disc, `ATP1M7FR`,
    /// in a campaign set up as it would be by then. Whether the disc has such a battle.
    pub fn play_mission(&mut self, name: &str) -> bool {
        let Some((house, phase, mission)) = campaign::named_mission(name) else {
            return false;
        };
        self.campaign = Some(Campaign::at(house, phase, mission, self.start_random));
        self.units_shown = 0;
        self.picked = None;
        self.fight(mission);
        true
    }

    /// The movie has ended: on to what follows its screen, unless it was skipped already.
    pub fn end_movie(&mut self) {
        if std::mem::take(&mut self.movie_playing) {
            match self.screen {
                Screen::Intro => self.set_screen(Screen::MainMenu),
                Screen::Movie(faction) => self.finish_movie(faction),
                Screen::Opening(house) => self.start_campaign(house),
                _ => {}
            }
        }
    }

    /// Whether one of the menus shows now, out of battle: not the intro, a movie or loading.
    pub fn is_in_menus(&self) -> bool {
        !self.in_battle
            && !matches!(
                self.screen,
                Screen::Intro
                    | Screen::Movie(_)
                    | Screen::Opening(_)
                    | Screen::Loading
                    | Screen::InGame
            )
    }

    /// Whether a movie is what shows now.
    pub fn is_playing_movie(&self) -> bool {
        self.movie_playing
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

    /// As many opponents as the chosen map has places for besides the player's.
    fn most_ai_players(&self) -> i32 {
        maps::list()
            .get(self.skirmish.map)
            .map_or(OPPONENT_SLOTS, |map| map.players.saturating_sub(1))
            .clamp(1, OPPONENT_SLOTS) as i32
    }

    fn set_slider(&mut self, id: SliderId, value: i32) {
        let old_ai_players = self.skirmish.ai_players as usize;
        let value = if id == SliderId::AiPlayers {
            value.min(self.most_ai_players())
        } else {
            value
        };
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
        } else if let Some(question) = self.question() {
            question_buttons(question, self.width, self.height)
        } else if self.dropdown_open {
            map_items(self.map_folder, self.map_scroll, self.width, self.height)
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
        let found = buttons
            .chain(self.checkboxes())
            .chain(widgets)
            .find(|b| b.contains(x, y))
            .map(|b| b.action);
        if found.is_none() && self.screen == Screen::CampaignMap && self.question().is_none() {
            return self.territory_at((x, y)).map(Action::Territory);
        }
        found
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
            Screen::Briefing => Some(Screen::CampaignMap),
            Screen::NewUnits => Some(Screen::Briefing),
            Screen::Intro
            | Screen::MainMenu
            | Screen::Loading
            | Screen::Movie(_)
            | Screen::Opening(_)
            | Screen::BattleStats
            | Screen::CampaignMap => None,
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
        if self.aiming_strike || self.click_mode.take().is_some() {
            self.aiming_strike = false;
            self.selection_anchor = None;
            return;
        }
        if self.site == Site::Placing(Role::Wall) {
            self.call_off_wall();
            self.selection_anchor = None;
            return;
        }
        if let Site::Placing(role) = self.site {
            self.site = Site::Ready(role);
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
            let dt = dt * self.settings.battle_pace();
            self.battle_time += dt;
            self.run_script(dt);
            let battle_time = self.battle_time;
            self.chat
                .retain(|line| battle_time - line.posted_at < CHAT_SECONDS);
            let terrain = battlefield::terrain(self.battle_map);
            units::advance(&mut self.units, terrain, &self.teams, dt);
            units::tend_harvesters(&mut self.units, terrain, &mut self.money, dt);
            units::tend_carryalls(&mut self.units, terrain);
            units::board_apcs(&mut self.units);
            self.combat
                .advance(&mut self.units, &self.teams, terrain, dt);
            for eye in units::release_eyes(&mut self.units, terrain, &mut self.next_unit_id) {
                self.combat.drop_bombs(&eye, terrain);
            }
            self.combat.stir_tornados(terrain, dt);
            units::carry(&mut self.units, &mut self.carried, terrain, &self.teams, dt);
            if self.superweapon().is_some() {
                let was_recharging = self.strike_recharge > 0.0;
                self.strike_recharge = (self.strike_recharge - dt).max(0.0);
                if was_recharging && self.strike_recharge <= 0.0 {
                    self.post_chat("Superweapon ready".to_string());
                    self.announce(Speech::SuperweaponReady);
                }
            } else {
                self.aiming_strike = false;
            }
            if self.screen == Screen::InGame {
                self.settle_casualties();
                if self.outcome.is_none() {
                    self.run_opponents();
                    for (owner, plan) in (1..).zip(&mut self.strategies) {
                        let money = &mut self.money[owner];
                        if let Some(building) =
                            plan.grow(&mut self.units, terrain, money, dt, self.next_unit_id)
                        {
                            self.units.push(building);
                            self.next_unit_id += 1;
                        }
                        plan.think(&mut self.units, terrain, &self.teams, dt);
                    }
                    units::settle_power(&mut self.units);
                    self.listen_to_the_base();
                    units::work_on_repairs(&mut self.units, &mut self.money, dt);
                    self.run_market(dt);
                    self.follow_routes();
                    self.retreat_armed = (self.retreat_armed - dt).max(0.0);
                    self.work_on_making(dt);
                    self.work_on_upgrades(dt);
                    self.work_on_site(dt);
                    self.deliver_harvesters(dt);
                    self.drop_crates(dt);
                }
            }
            self.update_fog();
        }
        match self.screen {
            Screen::Intro if !self.movie_playing && self.screen_elapsed >= INTRO_SECONDS => {
                self.set_screen(Screen::MainMenu)
            }
            Screen::Loading if self.screen_elapsed >= LOADING_SECONDS => {
                self.set_screen(Screen::InGame)
            }
            Screen::Movie(faction)
                if !self.movie_playing && self.screen_elapsed >= MOVIE_SECONDS =>
            {
                self.finish_movie(faction)
            }
            // Without the house's disc there is no movie to wait for.
            Screen::Opening(house)
                if !self.movie_playing && self.screen_elapsed >= OPENING_WAIT_SECONDS =>
            {
                self.start_campaign(house)
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
                self.camera.play(dt);
                self.follow_with_camera();
                // A scene a mission's script plays is watched, not scrolled away from.
                if self.following.is_none()
                    && !self.scene_playing
                    && self.focused
                    && !self.key_paused
                    && !self.minimap_dragging
                {
                    let direction =
                        battlefield::edge_direction(self.mouse, self.width, self.height);
                    let before = (self.camera.x, self.camera.z);
                    self.camera.scroll(direction, self.settings.scroll_rate, dt);
                    self.scrolled_since_press |= before != (self.camera.x, self.camera.z);
                }
            }
            _ => {}
        }
    }

    /// Takes in what the player did. Units the player has just selected answer.
    pub fn handle(&mut self, event: Event) {
        let before: Vec<u32> = self.selected_units().iter().map(|u| u.id).collect();
        let earlier = self.reply.take();
        self.handle_event(event);
        if self.reply.is_none()
            && self
                .units
                .iter()
                .any(|u| u.selected && !before.contains(&u.id))
        {
            self.answer(Said::Selected);
        }
        if self.reply.is_none() {
            self.reply = earlier;
        }
    }

    /// What the player's units have answered since this was last asked, and who answered.
    /// What has been heard on the battlefield since this was last asked, of what the player
    /// sees on the screen: units firing and dying.
    pub fn take_noises(&mut self) -> Vec<Noise> {
        let noises = std::mem::take(&mut self.combat.noises);
        let view = self.view();
        noises
            .into_iter()
            .filter(|noise| {
                let (x, y, z) = noise.unit.center();
                self.sight_at(x, z) == Sight::Visible
                    && view.project(x, y, z).is_some_and(|(px, py)| {
                        (0.0..=self.width).contains(&px) && (0.0..=self.height).contains(&py)
                    })
            })
            .collect()
    }

    pub fn take_reply(&mut self) -> Option<(Said, Vec<(Role, Faction)>)> {
        self.reply.take()
    }

    /// What is to be said out loud now, for the player's house.
    pub fn take_speeches(&mut self) -> Vec<Speech> {
        std::mem::take(&mut self.speeches)
    }

    /// Has this said out loud, unless it was said only a moment ago.
    fn announce(&mut self, speech: Speech) {
        let now = self.battle_time;
        let recent = self
            .last_said
            .iter()
            .any(|&(said, at)| said == speech && now - at < speech.quiet_for());
        if !recent {
            self.last_said.retain(|&(said, _)| said != speech);
            self.last_said.push((speech, now));
            self.speeches.push(speech);
        }
    }

    /// Hears what has happened to the player's side since the last look: an attack on its
    /// base or a harvester, a building finished, power running short.
    fn listen_to_the_base(&mut self) {
        let mut attacked = (false, false);
        let mut finished = false;
        for unit in self.units.iter().filter(|u| u.owner == 0 && u.alive) {
            let Some(&before) = self.health_seen.get(&unit.id) else {
                continue;
            };
            if unit.health < before - 0.01 {
                if unit.role == Role::Harvester {
                    attacked.1 = true;
                } else if unit.kind.is_building() && !unit.repairing {
                    attacked.0 = true;
                }
            }
        }
        let built: Vec<u32> = self
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.kind.is_building() && u.is_under_construction())
            .map(|u| u.id)
            .collect();
        for unit in self
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.kind.is_building())
        {
            let was_going_up = self.health_seen.get(&unit.id).is_some()
                && self.constructing_seen.contains(&unit.id);
            if was_going_up && !unit.is_under_construction() && unit.role != Role::Wall {
                finished = true;
            }
        }
        self.constructing_seen = built;
        self.health_seen = self
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.alive)
            .map(|u| (u.id, u.health))
            .collect();
        if attacked.0 {
            self.announce(Speech::BaseUnderAttack);
        }
        if attacked.1 {
            self.announce(Speech::HarvesterUnderAttack);
        }
        if finished {
            self.announce(Speech::ConstructionComplete);
        }
        let short = units::is_short_of_power(&self.units, 0);
        if short && !self.was_short_of_power {
            self.announce(Speech::LowPower);
        }
        self.was_short_of_power = short;
    }

    /// Has the selected units answer this.
    fn answer(&mut self, said: Said) {
        let speakers: Vec<(Role, Faction)> = self
            .selected_units()
            .iter()
            .filter(|u| !u.kind.is_building())
            .map(|u| (u.role, u.house))
            .collect();
        if !speakers.is_empty() {
            self.reply = Some((said, speakers));
        }
    }

    fn handle_event(&mut self, event: Event) {
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
            Event::NumberKey(number, modifier) => {
                if self.screen == Screen::InGame && !self.key_paused {
                    self.use_control_group(number, modifier);
                }
            }
            Event::RightDown => {
                if self.aiming_strike || self.click_mode.is_some() {
                    // Only calls off the superweapon, marching or selling; the selection stands.
                    self.aiming_strike = false;
                    self.click_mode = None;
                    self.selection_anchor = None;
                } else if self.screen == Screen::InGame
                    && !self.key_paused
                    && let Some(Action::BuildCell(cell)) = self.hovered()
                {
                    // On a build button it calls off one of those, and the selection stands.
                    // What was paid for it so far comes back.
                    if let Some(Entry::Upgrade(role)) = self.build_entry(self.build_tab, cell) {
                        units::cancel_upgrade(&mut self.units, 0, role, &mut self.money[0]);
                    }
                    if let Some(Entry::Order(role)) = self.build_entry(self.build_tab, cell) {
                        self.market.take_off(role);
                    }
                    if let Some(role) = self.build_role(self.build_tab, cell) {
                        if self.site == Site::Placing(Role::Wall) && role == Role::Wall {
                            // Nothing is paid for a wall until it is placed.
                            self.call_off_wall();
                        } else if self.site_of(role) {
                            self.money[0] += match self.site {
                                Site::Building(_, left) => units::paid_for(role, left),
                                _ => role.cost(),
                            };
                            self.site = Site::Idle;
                            if role == Role::Wall {
                                self.wall_plan.clear();
                            }
                        } else {
                            units::cancel_making(&mut self.units, role, 0, &mut self.money[0]);
                        }
                    }
                } else if self.site == Site::Placing(Role::Wall) {
                    self.call_off_wall();
                    self.selection_anchor = None;
                } else if let Site::Placing(role) = self.site {
                    // Elsewhere it only stops placing; the building stays ready.
                    self.site = Site::Ready(role);
                    self.selection_anchor = None;
                } else if self.screen == Screen::InGame && !self.key_paused {
                    self.selection_anchor = None;
                    for unit in &mut self.units {
                        unit.selected = false;
                    }
                }
            }
            Event::Wheel(notches) => {
                if self.dropdown_open {
                    self.scroll_maps(notches);
                } else if self.screen == Screen::InGame && !self.key_paused {
                    self.camera.zoom_by(notches);
                }
            }
            Event::Magnify(by) if self.screen == Screen::InGame && !self.key_paused => {
                self.camera.magnify(1.0 + by);
            }
            Event::Rotate(degrees) if self.screen == Screen::InGame && !self.key_paused => {
                self.camera.turn(degrees.to_radians());
            }
            Event::Magnify(_) | Event::Rotate(_) => {}
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
                self.turn_what_is_being_placed();
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
                        if matches!(self.site, Site::Placing(role) if role != Role::Wall)
                            && self.selection_anchor.is_some()
                        {
                            self.placing_press = Some(PlacingPress {
                                from: self.mouse,
                                turned: false,
                            });
                        }
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
                } else if let Screen::Opening(house) = self.screen {
                    self.start_campaign(house);
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
                (Screen::Opening(house), _) => self.start_campaign(house),
                (Screen::InGame, Key::Pause) if self.pausable && self.outcome.is_none() => {
                    self.key_paused = !self.key_paused;
                    self.selection_anchor = None;
                }
                (Screen::InGame, Key::Tab) => self.stats_open = !self.stats_open,
                (Screen::InGame, Key::Char('d' | 'D')) if !self.key_paused => self.toggle_deploy(),
                // The camera turns left or right about what it looks at, and back as it was.
                (Screen::InGame, Key::Char(',' | '<')) if !self.key_paused => {
                    self.camera.turn_step(-1.0);
                }
                (Screen::InGame, Key::Char('.' | '>')) if !self.key_paused => {
                    self.camera.turn_step(1.0);
                }
                (Screen::InGame, Key::Char('/' | '?')) if !self.key_paused => self.camera.reset(),
                (Screen::InGame, Key::Char('s' | 'S')) if !self.key_paused => {
                    units::order_stop(&mut self.units);
                }
                (Screen::InGame, Key::Char('z' | 'Z')) if !self.key_paused => {
                    let any = self.units.iter().any(|u| u.selected && u.owner == 0);
                    self.click_mode = any.then_some(ClickMode::March);
                }
                (Screen::InGame, Key::Char('a' | 'A')) if !self.key_paused => {
                    self.select_among(|u| !u.kind.is_building());
                }
                (Screen::InGame, Key::Char('p' | 'P')) if !self.key_paused => {
                    self.select_among(|u| !u.kind.is_building() && u.weapon.is_armed());
                }
                (Screen::InGame, Key::Char('t' | 'T')) if !self.key_paused => {
                    let kept: Vec<(u32, Role)> = self
                        .selected_units()
                        .iter()
                        .filter(|u| u.is_selected_in_groups())
                        .map(|u| (u.id, u.role))
                        .collect();
                    if !kept.is_empty() {
                        self.select_among(|u| {
                            u.is_selected_in_groups()
                                && kept.iter().any(|&(_, role)| role == u.role)
                        });
                        // Those it went by stay, off screen too, so the next press finds all
                        // their types.
                        for unit in &mut self.units {
                            unit.selected |= kept.iter().any(|&(id, _)| id == unit.id);
                        }
                    }
                }
                (Screen::InGame, Key::Char('f' | 'F')) if !self.key_paused => {
                    self.following = match self.following {
                        Some(_) => None,
                        None => self.units.iter().find(|u| u.selected).map(|u| u.id),
                    };
                    self.follow_with_camera();
                }
                (Screen::InGame, Key::Char('h' | 'H')) if !self.key_paused => {
                    let home = self.home();
                    units::order_move(
                        &mut self.units,
                        home,
                        battlefield::terrain(self.battle_map),
                        &self.teams,
                    );
                    self.answer(Said::Moving);
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
                if let Some(house) = self.confirming.take() {
                    self.set_screen(Screen::Opening(house));
                }
            }
            Action::Territory(at) => {
                if self.campaign.as_ref().map(Campaign::next) == Some(Next::Pick) {
                    self.picked = Some(at);
                }
            }
            Action::CampaignAttack => {
                if let Some(mission) = self.next_battle() {
                    self.brief(mission);
                }
            }
            Action::CampaignMenu => {
                self.campaign = None;
                self.set_screen(Screen::MainMenu);
            }
            Action::Defend => {
                if let Some(Next::Defend(at, by)) = self.question() {
                    self.brief(Mission::Defend(at, by));
                }
            }
            Action::GiveUp => {
                if let Some(campaign) = &mut self.campaign {
                    campaign.give_up();
                }
            }
            Action::TakeSide(side) => {
                if let Some(campaign) = &mut self.campaign {
                    campaign.take_side(side);
                }
            }
            Action::AcceptBriefing => {
                if let Some(mission) = self.briefing {
                    self.fight(mission);
                }
            }
            Action::ShowNewUnits => self.show_new_units(),
            Action::SkirmishBattle => self.set_screen(Screen::SkirmishSetup),
            Action::StartBattle => self.start_battle(self.skirmish.map, None, BattleKind::Skirmish),
            Action::NameField => self.name_focused = true,
            Action::MapDropdown => {
                self.dropdown_open = true;
                self.map_folder = maps::list()
                    .get(self.skirmish.map)
                    .map_or_else(MapFolder::default, MapFolder::of);
                self.map_scroll = 0;
            }
            Action::MapFolder(folder) => {
                self.map_folder = folder;
                self.map_scroll = 0;
            }
            Action::SelectMap(map) => {
                self.skirmish.map = map;
                let most = self.most_ai_players();
                if self.skirmish.ai_players > most {
                    self.set_slider(SliderId::AiPlayers, most);
                }
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
            Action::BuildTab(tab) if self.tab_enabled(tab) => self.build_tab = tab,
            Action::BuildTab(_) => {}
            Action::BuildCell(cell) => self.activate_build_cell(cell),
            Action::HudWide(UPGRADE_BUTTON) if self.tab_enabled(BuildTab::Upgrades) => {
                self.build_tab = BuildTab::Upgrades;
            }
            Action::HudWide(STARPORT_BUTTON) if self.tab_enabled(BuildTab::Starport) => {
                self.build_tab = BuildTab::Starport;
            }
            Action::HudSmall(SELL_BUTTON) if self.in_battle => self.toggle_mode(ClickMode::Sell),
            Action::HudSmall(REPAIR_BUTTON) if self.in_battle => {
                self.toggle_mode(ClickMode::Repair);
            }
            Action::HudSmall(WAYPOINT_BUTTON) if self.in_battle => {
                self.toggle_mode(ClickMode::Route);
            }
            Action::HudSmall(GUARD_BUTTON) if self.in_battle => {
                units::toggle_guard(&mut self.units);
                let on = self.hud_small_lit(GUARD_BUTTON);
                self.announce(if on {
                    Speech::GuardOn
                } else {
                    Speech::GuardOff
                });
            }
            Action::HudSmall(RETREAT_BUTTON) if self.in_battle && self.outcome.is_none() => {
                self.retreat();
            }
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
        if let Some(role) = self.build_role(self.build_tab, cell) {
            if role.kind().is_building() {
                if self.can_produce(role) {
                    self.click_building(role);
                }
                return;
            }
            // It is paid for while it is made. A busy building queues it.
            if self.player_money() <= 0 {
                self.announce(Speech::InsufficientFunds);
            }
            if units::start_making(&mut self.units, role, 0) {
                self.announce(Speech::Training);
            }
        } else if let Some(Entry::Upgrade(role)) = self.build_entry(self.build_tab, cell) {
            // Paid for while it goes on, as what is made is.
            if units::start_upgrade(&mut self.units, 0, role) {
                self.announce(Speech::Upgrading);
            }
        } else if let Some(Entry::Order(role)) = self.build_entry(self.build_tab, cell) {
            self.market.add(role);
        } else if self.build_entry(self.build_tab, cell) == Some(Entry::Purchase) {
            if self.market.purchase(&mut self.money[0]) {
                self.post_chat("Starport order placed".to_string());
            }
        } else if self.build_entry(self.build_tab, cell) == Some(Entry::CancelOrder) {
            self.market.order.clear();
        } else if self.build_entry(self.build_tab, cell) == Some(Entry::Superweapon)
            && self.strike_recharge <= 0.0
        {
            self.aiming_strike = true;
        }
    }

    /// Scrolls the open dropdown's maps a column back for each notch the wheel turns up, on
    /// for each down, as far as there are columns.
    fn scroll_maps(&mut self, notches: f32) {
        let tabs = maps::list().iter().any(|map| map.campaign && !map.hidden);
        let labels = folder_entries(self.map_folder).map(|(_, label)| label);
        let most = ListLayout::new(labels, tabs, self.width, self.height).most_scroll();
        let columns = notches.abs().ceil() as usize;
        self.map_scroll = if notches > 0.0 {
            self.map_scroll.saturating_sub(columns)
        } else {
            (self.map_scroll + columns).min(most)
        };
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
        self.movie_playing = false;
    }
}

/// Whether a point is inside a convex shape, whichever way round its corners go.
/// Whether a point on screen is on a box as it shows, from the corners of its bottom and of
/// its top, each going round it the same way: on one of its sides, its bottom or its top.
fn is_on_box(base: [(f32, f32); 4], roof: [(f32, f32); 4], point: (f32, f32)) -> bool {
    (0..4)
        .map(|i| {
            let j = (i + 1) % 4;
            [base[i], base[j], roof[j], roof[i]]
        })
        .chain([base, roof])
        .any(|face| is_in_convex(face, point))
}

fn is_in_convex(corners: [(f32, f32); 4], (x, y): (f32, f32)) -> bool {
    let sides = (0..4).map(|i| {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0)
    });
    sides.clone().all(|side| side >= 0.0) || sides.clone().all(|side| side <= 0.0)
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
        // Under the planet, where the original's scene has them.
        let middle = |b: &Button| (b.y + b.h / 2.0) / H;
        assert!((middle(&b[0]) - 0.684).abs() < 0.001, "{}", middle(&b[0]));
        assert!((middle(&b[1]) - 0.792).abs() < 0.001, "{}", middle(&b[1]));
        assert!(b[0].y + b[0].h < b[1].y, "one under the other");
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
        assert_eq!(range(Tab::Game, 0), (1, UPDATES_PER_SECOND.len() as i32));
        for i in 1..3 {
            assert_eq!(range(Tab::Game, i), (1, 10));
        }
        for i in 0..3 {
            assert_eq!(range(Tab::Sound, i), (0, 100));
        }
        assert_eq!(range(Tab::Graphics, 0), (0, 2));
        assert_eq!(QUALITY_STOPS, ["Low", "Medium", "High"]);
    }

    #[test]
    fn dragging_changes_the_value_and_clamps_past_the_ends() {
        let mut game = in_options();
        let s = slider(&game, SliderId::GameSpeed);
        drag(&mut game, SliderId::GameSpeed, s.x_of(4));
        assert_eq!(game.settings.game_speed, 4);
        drag(&mut game, SliderId::GameSpeed, -1000.0);
        assert_eq!(game.settings.game_speed, 1);
        drag(&mut game, SliderId::GameSpeed, W + 1000.0);
        assert_eq!(game.settings.game_speed, 7);
        assert_eq!(game.dragging, None);

        // After release, moving the mouse no longer changes anything.
        game.handle(Event::MouseMove(s.x_of(3), s.cy));
        assert_eq!(game.settings.game_speed, 7);
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
        assert_eq!(rows[0].allies, [Ally::Tleilaxu, Ally::None]);
        assert_eq!(rows[0].color, game.skirmish.color);
        assert_eq!(rows[1].name, "AI 1 (Competent)");
        assert_eq!(rows[2].color, game.skirmish.opponents[1].color);
    }

    #[test]
    fn diplomacy_outside_a_skirmish_lists_the_player_and_one_enemy() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::Ordos);
        click(&mut game, Action::ConfirmFaction);
        game.update(OPENING_WAIT_SECONDS + 0.05);
        game.update(LOADING_SECONDS);
        let rows = game.stats_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].faction, Some(Faction::Ordos));
        assert_eq!(rows[0].color, TeamColor::Cyan);
        assert_ne!(rows[1].color, rows[0].color);
    }

    #[test]
    fn a_red_player_faces_an_enemy_of_another_color() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::Harkonnen);
        click(&mut game, Action::ConfirmFaction);
        game.update(OPENING_WAIT_SECONDS + 0.05);
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
        pick(&mut game, Faction::Atreides);
        click(&mut game, Action::ConfirmFaction);
        game.update(OPENING_WAIT_SECONDS + 0.05);
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
    fn a_sardaukar_steps_out_of_his_crate_for_whoever_opens_it() {
        let mut game = in_battle();
        let finder = game
            .units
            .iter()
            .find(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
            .copied()
            .unwrap();
        let kind = units::CrateKind::named("SardaukarCrate").unwrap();
        game.crates.push(units::Crate {
            x: finder.x,
            z: finder.z,
            kind,
            gift: kind.gift(),
            left: f32::INFINITY,
        });
        let sardaukar = |game: &Game| {
            game.units
                .iter()
                .filter(|u| u.owner == 0 && u.role == Role::ImSardaukar)
                .count()
        };
        let before = sardaukar(&game);
        game.update(0.1);
        assert!(game.crates.is_empty());
        assert_eq!(sardaukar(&game), before + 1);
    }

    #[test]
    fn a_scripts_sides_are_the_enemys_the_players_or_nobodys_by_whom_they_fight() {
        use mission::Battle;
        let mut game = in_battle();
        game.script_sides = 2;
        let owner = |game: &Game, side: i32| game.side_owner(side).map(|(owner, _)| owner);
        assert_eq!(owner(&game, 0), Some(0));
        assert_eq!(owner(&game, 1), Some(units::NEUTRAL), "fighting nobody yet");
        game.relate(2, 0, true);
        assert_eq!(owner(&game, 2), Some(1), "the player's enemy");
        assert_eq!(owner(&game, 1), Some(units::NEUTRAL), "a prisoner, say");
        game.relate(1, 2, true);
        assert_eq!(owner(&game, 1), Some(0), "fighting the player's enemy");
    }

    #[test]
    fn the_winners_soldiers_stop_and_cheer() {
        let mut game = in_battle();
        win(&mut game);
        let soldiers = |owner: usize| {
            game.units
                .iter()
                .filter(move |u| u.alive && u.owner == owner && u.kind == UnitKind::Infantry)
        };
        assert!(soldiers(0).next().is_some());
        assert!(soldiers(0).all(|u| u.cheering && !u.is_moving()));

        let mut game = in_battle();
        kill_owners(&mut game, &[0]);
        let enemy = all_enemies(&game)[0];
        let theirs: Vec<_> = game
            .units
            .iter()
            .filter(|u| u.alive && u.owner == enemy && u.kind == UnitKind::Infantry)
            .collect();
        assert!(!theirs.is_empty());
        assert!(
            theirs.iter().all(|u| u.cheering),
            "on a defeat the enemy does"
        );
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
        with_bases(&mut game);
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

    /// Wins the battle and goes past its statistics.
    fn win_and_leave(game: &mut Game) {
        win(game);
        game.handle(Event::KeyDown(Key::Escape));
        click(game, Action::LeaveBattleStats);
    }

    #[test]
    fn a_territorys_battle_can_be_gone_straight_into_by_its_scripts_name() {
        let mut game = Game::new(1280.0, 800.0);
        assert!(!game.play_mission("ORP3D30TLFail"));
        assert!(!game.play_mission("Nothing"));
        assert_eq!(game.campaign, None);
        assert!(game.play_mission("ORP3D30TL"));
        let defence = Mission::Defend(29, Faction::Atreides);
        assert_eq!(game.mission, Some(defence));
        assert_eq!(game.screen, Screen::Loading);
        let campaign = game.campaign.as_ref().unwrap();
        assert_eq!((campaign.house, campaign.phase), (Faction::Ordos, 3));
        assert_eq!(campaign.next(), Next::Defend(29, Faction::Atreides));
    }

    #[test]
    fn a_campaign_opens_its_map_after_its_first_victory_and_replays_it_after_a_defeat() {
        let mut game = campaign_battle();
        assert_eq!(game.mission, Some(Mission::Start));
        kill_owners(&mut game, &[0]);
        game.handle(Event::KeyDown(Key::Escape));
        click(&mut game, Action::LeaveBattleStats);
        assert_eq!(game.screen, Screen::Loading, "the first battle again");
        assert_eq!(game.mission, Some(Mission::Start));
        game.update(LOADING_SECONDS);
        win_and_leave(&mut game);
        assert_eq!(game.screen, Screen::CampaignMap);
        assert_eq!(game.campaign.as_ref().unwrap().next(), Next::Pick);
    }

    #[test]
    fn a_houses_first_battle_starts_as_its_script_has_it_and_goes_on_by_it() {
        let mut game = campaign_battle();
        let yard = game.units.iter().find(|u| u.owner == 0).unwrap();
        assert_eq!(
            (yard.role, yard.deployed),
            (Role::Mcv, true),
            "a construction yard"
        );
        let palace = game.units.iter().find(|u| u.owner == 1).unwrap();
        assert_eq!(palace.role, Role::Palace(Faction::Harkonnen));
        assert_eq!(game.units.len(), 2, "nothing but what the script puts down");
        assert_eq!(game.player_money(), 10_000);
        assert_eq!(game.take_mission_lines(), [642], "the Mentat's welcome");
        assert!(game.take_mission_lines().is_empty(), "once");
        // Its third update brings an MCV in for the player.
        game.update(0.35);
        assert!(
            game.units
                .iter()
                .any(|u| u.owner == 0 && u.role == Role::Mcv && !u.deployed)
        );
        // The palace destroyed, the script declares the battle won.
        let palace = game.units.iter().position(|u| u.owner == 1).unwrap();
        game.units[palace].alive = false;
        game.units.remove(palace);
        game.update(0.15);
        assert_eq!(game.outcome, Some(Outcome::Victory));
    }

    #[test]
    fn a_territory_picked_on_the_map_is_briefed_after_the_new_units_and_then_fought() {
        let mut game = campaign_battle();
        game.update(LOADING_SECONDS);
        win_and_leave(&mut game);
        click(&mut game, Action::CampaignAttack);
        assert_eq!(game.screen, Screen::CampaignMap, "nothing picked yet");
        let target = game.campaign.as_ref().unwrap().targets()[0];
        click(&mut game, Action::Territory(target));
        assert_eq!(game.picked, Some(target));
        click(&mut game, Action::CampaignAttack);
        assert_eq!(
            game.screen,
            Screen::NewUnits,
            "the first tech level's units first"
        );
        assert_eq!(game.units_shown, 1);
        click(&mut game, Action::Back);
        assert_eq!(game.screen, Screen::Briefing);
        assert_eq!(game.briefing, Some(Mission::Attack(target)));
        click(&mut game, Action::ShowNewUnits);
        assert_eq!(game.screen, Screen::NewUnits, "again, if asked for");
        click(&mut game, Action::Back);
        click(&mut game, Action::AcceptBriefing);
        assert_eq!(game.screen, Screen::Loading);
        assert_eq!(game.mission, Some(Mission::Attack(target)));
        let owner = game.campaign.as_ref().unwrap().owners[target];
        assert_eq!(game.faction_of(1), owner, "against who holds it");
        game.update(LOADING_SECONDS);
        win_and_leave(&mut game);
        let campaign = game.campaign.as_ref().unwrap();
        assert_eq!(campaign.owners[target], Faction::Atreides, "taken");
        assert_eq!(game.screen, Screen::CampaignMap);
    }

    #[test]
    fn an_attacked_territory_is_defended_in_battle_or_given_up() {
        let mut game = campaign_battle();
        game.update(LOADING_SECONDS);
        win_and_leave(&mut game);
        // Attacks until another house attacks back.
        for _ in 0..30 {
            if game.question().is_some() {
                break;
            }
            let target = game.campaign.as_ref().unwrap().targets()[0];
            click(&mut game, Action::Territory(target));
            click(&mut game, Action::CampaignAttack);
            while game.screen != Screen::Briefing {
                click(&mut game, Action::Back);
            }
            click(&mut game, Action::AcceptBriefing);
            game.update(LOADING_SECONDS);
            kill_owners(&mut game, &[0]);
            game.handle(Event::KeyDown(Key::Escape));
            click(&mut game, Action::LeaveBattleStats);
            if game.campaign.as_ref().unwrap().next() == Next::Fight(Mission::Heighliner) {
                game.campaign
                    .as_mut()
                    .unwrap()
                    .finish(Mission::Heighliner, true);
            }
        }
        let Some(Next::Defend(at, by)) = game.question() else {
            panic!("never attacked");
        };
        assert!(game.question_text().unwrap().contains("Defend it?"));
        assert!(actions(&game).contains(&Action::GiveUp));
        click(&mut game, Action::Defend);
        assert_eq!(game.briefing, Some(Mission::Defend(at, by)));
        click(&mut game, Action::Back);
        assert_eq!(game.question(), Some(Next::Defend(at, by)), "asked again");
        click(&mut game, Action::GiveUp);
        assert_eq!(game.campaign.as_ref().unwrap().owners[at], by);
    }

    #[test]
    fn the_harkonnen_are_asked_which_side_to_take() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::Harkonnen);
        click(&mut game, Action::ConfirmFaction);
        game.update(OPENING_WAIT_SECONDS + 0.05);
        let mut campaign = crate::campaign::tests::on_the_map(Faction::Harkonnen);
        campaign.finish(Mission::Attack(campaign.targets()[0]), true);
        while let Next::Defend(..) = campaign.next() {
            campaign.give_up();
        }
        campaign.finish(Mission::Attack(campaign.targets()[0]), true);
        campaign.finish(Mission::Heighliner, true);
        for _ in 0..2 {
            while let Next::Defend(..) = campaign.next() {
                campaign.give_up();
            }
            campaign.finish(Mission::Attack(campaign.targets()[0]), true);
        }
        game.campaign = Some(campaign);
        game.set_screen(Screen::CampaignMap);
        assert_eq!(game.question(), Some(Next::Side));
        assert_eq!(game.question_text().as_deref(), Some("Choose Side"));
        click(&mut game, Action::TakeSide(Side::Copec));
        assert_eq!(game.question(), None);
        click(&mut game, Action::CampaignAttack);
        while game.screen != Screen::Briefing {
            click(&mut game, Action::Back);
        }
        assert_eq!(game.briefing, Some(Mission::CivilWar(Side::Copec)));
    }

    #[test]
    fn a_territory_is_picked_by_its_shape_on_the_map_too() {
        let mut game = campaign_battle();
        game.update(LOADING_SECONDS);
        win_and_leave(&mut game);
        let b = *game
            .buttons()
            .iter()
            .find(|b| b.action == Action::Territory(3))
            .unwrap();
        game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
        assert_eq!(game.hovered(), Some(Action::Territory(3)));
        click(&mut game, Action::CampaignMenu);
        assert_eq!(game.screen, Screen::MainMenu);
        assert!(game.campaign.is_none());
    }

    /// Gives the player a palace of their house.
    fn give_palace(game: &mut Game) {
        let mut palace = *game.units.iter().find(|u| u.owner == 0).unwrap();
        palace.role = Role::Palace(game.faction_of(0));
        palace.kind = UnitKind::Building;
        palace.selected = false;
        palace.id = game.next_unit_id;
        game.next_unit_id += 1;
        game.units.push(palace);
    }

    #[test]
    fn each_house_builds_its_own_palace_which_brings_its_own_superweapon() {
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let mut game = in_battle_as(house);
            assert!(offers(&game, BuildTab::Buildings, Role::Palace(house)));
            assert!(Role::Palace(house).made_by(house));
            assert_eq!(game.superweapon(), None, "none without a palace");
            give_palace(&mut game);
            assert_eq!(game.superweapon(), Some(Superweapon::of(house)));
        }
    }

    #[test]
    fn a_house_cannot_build_another_houses_palace() {
        assert!(!Role::Palace(Faction::Atreides).made_by(Faction::Harkonnen));
        assert!(!Role::Palace(Faction::Harkonnen).made_by(Faction::Ordos));
        assert!(!Role::Palace(Faction::Ordos).made_by(Faction::Atreides));
    }

    #[test]
    fn the_superweapon_cannot_be_aimed_without_a_palace() {
        let mut game = in_battle();
        assert_eq!(game.superweapon_cell(), None);
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        for cell in 0..hud::GRID_ROWS * hud::GRID_COLUMNS {
            click_hud(&mut game, Action::BuildCell(cell));
        }
        assert!(!game.aiming_strike);
    }

    #[test]
    fn losing_the_palace_calls_off_the_aiming() {
        let mut game = in_battle();
        pick_superweapon(&mut game);
        assert!(game.aiming_strike);
        game.units.retain(|u| !matches!(u.role, Role::Palace(_)));
        game.update(0.01);
        assert!(!game.aiming_strike);
    }

    #[test]
    fn the_superweapon_only_recharges_while_there_is_a_palace() {
        let mut game = in_battle();
        game.strike_recharge = 10.0;
        game.update(1.0);
        assert_eq!(game.strike_recharge, 10.0, "no palace, no countdown");
        give_palace(&mut game);
        game.update(1.0);
        assert_eq!(game.strike_recharge, 9.0);
    }

    #[test]
    fn a_palace_opens_the_vehicles_tab_without_a_factory_or_hangar() {
        let mut game = in_battle();
        game.units
            .retain(|u| u.owner != 0 || !matches!(u.role, Role::Factory | Role::Hanger));
        assert!(!game.tab_enabled(BuildTab::Vehicles));
        give_palace(&mut game);
        assert!(game.tab_enabled(BuildTab::Vehicles));
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        assert_eq!(game.build_tab, BuildTab::Vehicles);
        assert_eq!(game.superweapon_cell(), Some(0), "all there is");
        click_hud(&mut game, Action::BuildCell(0));
        assert!(game.aiming_strike);
    }

    #[test]
    fn no_palace_is_on_offer_with_superweapons_turned_off() {
        let mut game = in_battle();
        game.skirmish.superweapons = false;
        let house = game.faction_of(0);
        assert!(!offers(&game, BuildTab::Buildings, Role::Palace(house)));
    }

    #[test]
    fn a_building_placed_takes_a_second_to_go_up_and_only_then_works() {
        let mut game = in_battle();
        game.site = Site::Placing(Role::Barracks);
        let (x, y) = buildable_spot_on_screen(&mut game);
        click_at(&mut game, x, y);
        assert_eq!(game.site, Site::Idle);
        let barracks = game
            .units
            .iter()
            .rposition(|u| u.owner == 0 && u.role == Role::Barracks)
            .unwrap();
        assert!(game.units[barracks].is_under_construction());
        assert!(!game.units[barracks].can_make(Role::Scout));
        let objects = |game: &Game| crate::render::build_frame(game).objects.len();
        game.update(0.5);
        let with_cranes = objects(&game);
        let left = std::mem::take(&mut game.units[barracks].constructing);
        assert!(with_cranes > objects(&game), "cranes are up");
        game.units[barracks].constructing = left;
        assert!(
            game.combat.particles.iter().any(|p| !p.smoke),
            "and it is being welded"
        );
        game.update(0.6);
        assert!(!game.units[barracks].is_under_construction());
        assert!(game.units[barracks].can_make(Role::Scout));
    }

    #[test]
    fn a_building_on_its_way_is_paid_for_as_it_goes_up_and_comes_back_if_called_off() {
        let mut game = in_battle();
        game.skirmish.crates = false;
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        let before = game.player_money();
        let refinery = cell_of(&game, BuildTab::Buildings, Role::Refinery);
        click_hud(&mut game, Action::BuildCell(refinery));
        assert!(matches!(game.site, Site::Building(Role::Refinery, _)));
        game.update(Role::Refinery.build_seconds() / 2.0);
        let paid = before - game.player_money();
        assert!((paid - Role::Refinery.cost() / 2).abs() <= 2, "{paid}");
        let cell = hud::all_buttons(W, H)
            .into_iter()
            .find(|b| b.action == Action::BuildCell(refinery))
            .unwrap();
        game.handle(Event::MouseMove(
            cell.x + cell.w / 2.0,
            cell.y + cell.h / 2.0,
        ));
        game.handle(Event::RightDown);
        assert_eq!(game.site, Site::Idle);
        assert_eq!(game.player_money(), before, "it all comes back");
    }

    #[test]
    fn without_the_money_for_it_nothing_gets_made_until_there_is() {
        let mut game = in_battle();
        game.skirmish.crates = false;
        game.money[0] = 0;
        units::start_making(&mut game.units, Role::AtTrike, 0);
        game.update(Role::AtTrike.build_seconds());
        assert_eq!(game.making_progress(Role::AtTrike), Some(0.0));
        game.money[0] = Role::AtTrike.cost();
        game.update(Role::AtTrike.build_seconds() / 2.0);
        let progress = game.making_progress(Role::AtTrike).unwrap();
        assert!((progress - 0.5).abs() < 0.01, "{progress}");
    }

    #[test]
    fn each_new_refinery_brings_one_harvester_under_a_carryall_which_then_stays() {
        let mut game = in_battle();
        game.skirmish.crates = false;
        let mine = |game: &Game, role: Role| {
            game.units
                .iter()
                .filter(|u| u.owner == 0 && u.role == role)
                .count()
        };
        let harvesters = mine(&game, Role::Harvester);
        game.site = Site::Placing(Role::Refinery);
        let (x, y) = buildable_spot_on_screen(&mut game);
        click_at(&mut game, x, y);
        game.update(units::CONSTRUCTION_SECONDS + 0.1);
        assert_eq!(mine(&game, Role::Harvester), harvesters + 1);
        assert_eq!(mine(&game, Role::Carryall), 1, "it comes under a carryall");
        assert!(
            game.chat
                .iter()
                .any(|line| line.text == "Harvester on its way")
        );
        let brought = game
            .units
            .iter()
            .find(|u| u.owner == 0 && u.role == Role::Harvester && u.theft.is_some())
            .map(|u| u.id);
        // Put down, it may well be full and lifted home again by the end.
        let mut down = false;
        for _ in 0..90 * 10 {
            game.update(0.1);
            down |= game
                .units
                .iter()
                .any(|u| Some(u.id) == brought && u.theft.is_none() && u.load > 0.0);
        }
        assert_eq!(mine(&game, Role::Harvester), harvesters + 1, "only the one");
        assert_eq!(
            mine(&game, Role::Carryall),
            1,
            "and the carryall is the player's"
        );
        assert!(down, "it is put down and harvests");
    }

    #[test]
    fn a_side_short_of_harvesters_for_a_minute_is_given_one_under_a_carryall_that_leaves() {
        let mut game = in_battle();
        game.skirmish.crates = false;
        let mine = |game: &Game, role: Role| {
            game.units
                .iter()
                .filter(|u| u.alive && u.owner == 0 && u.role == role)
                .count()
        };
        game.site = Site::Placing(Role::Refinery);
        let (x, y) = buildable_spot_on_screen(&mut game);
        click_at(&mut game, x, y);
        game.update(units::CONSTRUCTION_SECONDS + 0.1);
        let refineries = mine(&game, Role::Refinery);
        assert_eq!(refineries, 1);
        game.units
            .retain(|u| u.owner != 0 || !matches!(u.role, Role::Harvester | Role::Carryall));
        game.update(HARVESTER_GIFT_SECONDS - 1.0);
        assert_eq!(mine(&game, Role::Harvester), 0, "not yet");
        game.update(1.5);
        assert_eq!(mine(&game, Role::Harvester), 1);
        assert_eq!(mine(&game, Role::Carryall), 1);
        for _ in 0..90 * 10 {
            game.update(0.1);
        }
        assert_eq!(mine(&game, Role::Carryall), 0, "it flies off again");
        assert!(
            mine(&game, Role::Harvester) <= refineries,
            "no more than it needs"
        );
    }

    #[test]
    fn the_dollar_button_sells_the_players_buildings_for_half_until_it_is_let_go() {
        let mut game = in_battle();
        game.fog = None;
        assert_eq!(
            hud::command_buttons(game.width, game.height)[SELL_BUTTON].label,
            "$"
        );
        let pad = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Helipad)
            .unwrap();
        let at = game.units[pad];
        game.camera.look_at(at.x, at.z);
        let (x, y) = game.view().project(at.x, at.pick_height(), at.z).unwrap();
        click_hud(&mut game, Action::HudSmall(SELL_BUTTON));
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Sell);
        let before = game.player_money();
        click_at(&mut game, x, y);
        assert!(game.units.iter().all(|u| u.id != at.id), "sold");
        assert_eq!(game.player_money(), before + Role::Helipad.cost() / 2);
        assert_eq!(game.click_mode, Some(ClickMode::Sell), "still selling");
        game.handle(Event::RightDown);
        assert_eq!(game.click_mode, None);
    }

    #[test]
    fn s_stops_the_selected_where_they_are() {
        let mut game = in_battle();
        let (first, second) = two_soldiers(&game);
        select_ids(&mut game, &[first, second]);
        let terrain = battlefield::terrain(game.battle_map);
        let far = game
            .units
            .iter()
            .find(|u| u.id == first)
            .map(|u| beside((u.x, u.z), 905.1))
            .unwrap();
        units::order_move(&mut game.units, far, terrain, &game.teams);
        game.update(0.5);
        game.handle(Event::KeyDown(Key::Char('s')));
        assert!(
            game.units
                .iter()
                .filter(|u| u.selected)
                .all(|u| u.destination == (u.x, u.z) && u.target.is_none())
        );
    }

    #[test]
    fn z_then_a_click_marches_the_selected_keeping_their_shape_at_the_slowest_pace() {
        let mut game = in_battle();
        game.fog = None;
        let mut ground = game
            .units
            .iter()
            .filter(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .map(|u| u.id);
        let (first, second) = (ground.next().unwrap(), ground.next().unwrap());
        select_ids(&mut game, &[first, second]);
        let spot = |game: &Game, id: u32| {
            let u = game.units.iter().find(|u| u.id == id).unwrap();
            (u.x, u.z)
        };
        let (x, y) = buildable_spot_on_screen(&mut game);
        let (a, b) = (spot(&game, first), spot(&game, second));
        game.handle(Event::KeyDown(Key::Char('z')));
        assert_eq!(game.click_mode, Some(ClickMode::March));
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::March);
        click_at(&mut game, x, y);
        assert_eq!(game.click_mode, None, "for one click only");
        let to = |id: u32| game.units.iter().find(|u| u.id == id).unwrap().destination;
        let (to_a, to_b) = (to(first), to(second));
        let apart = |p: (f32, f32), q: (f32, f32)| (p.0 - q.0, p.1 - q.1);
        let (was, now) = (apart(a, b), apart(to_a, to_b));
        assert!(
            (was.0 - now.0).hypot(was.1 - now.1) < 0.4525,
            "the same shape: {was:?} {now:?}"
        );
        let paces: Vec<f32> = [first, second]
            .map(|id| game.units.iter().find(|u| u.id == id).unwrap().speed())
            .into();
        assert_eq!(paces[0], paces[1], "at one pace");
    }

    #[test]
    fn with_crates_on_they_drop_now_and_then_and_pay_whoever_reaches_one() {
        let mut game = in_battle();
        assert!(game.skirmish.crates);
        for _ in 0..(CRATE_SECONDS * 4.0) as usize {
            game.update(1.0);
        }
        assert!(!game.crates.is_empty());
        let found = game.crates[0];
        let terrain = battlefield::terrain(game.battle_map);
        assert!(terrain.can_walk(found.x, found.z));
        let units::Gift::Money(money) = found.gift else {
            panic!("a dropped crate holds money");
        };
        assert!((CRATE_MONEY.0..=CRATE_MONEY.1).contains(&money));
        let vehicle = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
            .unwrap();
        let before = game.player_money();
        (game.units[vehicle].x, game.units[vehicle].z) = (found.x, found.z);
        game.units[vehicle].destination = (found.x, found.z);
        game.update(0.01);
        assert!(game.crates.iter().all(|c| (c.x, c.z) != (found.x, found.z)));
        assert!(game.player_money() >= before + money);
        let line = format!("Crate: ${money}");
        assert!(game.chat.iter().any(|l| l.text == line));

        let lying = game.crates.clone();
        for _ in 0..CRATE_LASTS as usize {
            game.update(1.0);
        }
        assert!(
            lying
                .iter()
                .all(|old| game.crates.iter().all(|c| (c.x, c.z) != (old.x, old.z))),
            "those nobody reached are gone"
        );

        let mut game = in_battle();
        game.skirmish.crates = false;
        for _ in 0..(CRATE_SECONDS * 4.0) as usize {
            game.update(1.0);
        }
        assert!(game.crates.is_empty(), "none with crates off");
    }

    #[test]
    fn opponents_pay_their_way_never_running_into_debt_and_put_up_a_refinery() {
        let mut game = in_setup();
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        game.skirmish.crates = false;
        for _ in 0..180 * 10 {
            game.update(0.1);
            assert!(
                game.money.iter().all(|&money| money >= 0),
                "{:?}",
                game.money
            );
        }
        let theirs = |role: Role| {
            game.units
                .iter()
                .filter(|u| u.owner == 1 && u.role == role)
                .count()
        };
        assert!(theirs(Role::Refinery) >= 1);
        assert!(
            theirs(Role::Harvester) >= 2,
            "the one it brought and one made"
        );
        assert!(game.money[1] < DEFAULT_BATTLE_MONEY, "it spends");
    }

    #[test]
    fn a_destroyed_vehicle_goes_up_in_a_big_blast_and_its_pieces_fly_then_are_gone() {
        let mut game = in_battle();
        let vehicle = game
            .units
            .iter()
            .position(|u| u.kind == UnitKind::Vehicle && u.role != Role::Mcv)
            .unwrap();
        let building = game
            .units
            .iter()
            .position(|u| u.kind == UnitKind::Building)
            .unwrap();
        let mut plain = Combat::default();
        let radius = game.units[building].radius();
        plain.explode_destroyed(game.units[building].center(), radius * 2.0, 0.0);

        let (x, z) = (game.camera.x, game.camera.z);
        game.units[vehicle].x = x;
        game.units[vehicle].z = z;
        game.units[vehicle].alive = false;
        game.update(0.01);
        assert_eq!(game.combat.shatters.len(), 1);
        assert!(game.combat.particles.len() > plain.particles.len() * 2);

        game.update(0.3);
        let objects = |game: &Game| crate::render::build_frame(game).objects.len();
        let with_pieces = objects(&game);
        let pieces = std::mem::take(&mut game.combat.shatters);
        assert!(with_pieces > objects(&game), "its triangles still show");
        game.combat.shatters = pieces;

        game.update(crate::weapons::SHATTER_SECONDS);
        assert!(game.combat.shatters.is_empty());
    }

    #[test]
    fn placing_shows_little_squares_on_the_grid_and_not_the_building() {
        let mut game = in_battle();
        game.site = Site::Placing(Role::Factory);
        let (x, y) = buildable_spot_on_screen(&mut game);
        game.handle(Event::MouseMove(x, y));
        let squares = game.placing_squares();
        let (cols, rows) = Role::Factory.plot_size(game.faction_of(0));
        assert_eq!(squares.len(), (cols * rows) as usize);
        assert!(squares.iter().all(|&(_, s)| s != PlaceSquare::Blocked));
        let exits: Vec<_> = squares
            .iter()
            .filter(|&&(_, s)| s == PlaceSquare::Exit)
            .map(|&(cell, _)| cell)
            .collect();
        assert_eq!(exits.len(), 2, "the factory's way out shows");
        let last_row = squares.iter().map(|&((_, v), _)| v).max().unwrap();
        assert!(exits.iter().all(|&(_, v)| v == last_row));
        let objects = crate::render::build_frame(&game).objects.len();
        game.site = Site::Idle;
        assert!(game.placing_squares().is_empty());
        assert_eq!(
            crate::render::build_frame(&game).objects.len(),
            objects,
            "no model of it"
        );
    }

    #[test]
    fn nothing_can_be_placed_far_from_every_building() {
        let mut game = in_battle();
        game.units.retain(|u| !u.kind.is_building() || u.owner != 0);
        game.site = Site::Placing(Role::AtRocketTurret);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        assert!(!game.placing_squares().is_empty());
        assert!(
            game.placing_squares()
                .iter()
                .all(|&(_, s)| s == PlaceSquare::Blocked)
        );
        assert!(!game.placing_preview().unwrap().1);
    }

    #[test]
    fn dead_infantry_leave_a_body_and_no_explosion_of_their_own() {
        let mut game = in_battle();
        let soldier = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .unwrap();
        game.units[soldier].alive = false;
        let flashes = game.combat.blasts.len();
        game.update(0.01);
        assert_eq!(game.combat.bodies.len(), 1);
        assert_eq!(game.combat.blasts.len(), flashes);
    }

    #[test]
    fn every_soldier_draws_in_every_stance_and_its_body_when_it_falls() {
        let mut game = in_battle();
        let soldier = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .unwrap();
        for role in [
            Role::AtInfantry,
            Role::Scout,
            Role::AtSniper,
            Role::Engineer,
            Role::AtKindjal,
            Role::HkLightInf,
            Role::HkTrooper,
            Role::HkFlamer,
            Role::OrChemical,
            Role::OrSaboteur,
            Role::OrMortar,
        ] {
            for deployed in [false, true] {
                game.units[soldier].role = role;
                game.units[soldier].deployed = deployed;
                assert!(!crate::render::build(&game).is_empty(), "{role:?}");
            }
        }
        game.units[soldier].deployed = false;
        let mut body = game.units[soldier];
        (body.x, body.z) = (game.camera.x, game.camera.z);
        game.combat.drop_body(&body);
        // Without the disc there is no model to show it dying with, but it still lies there.
        assert_eq!(game.combat.bodies.len(), 1);
        assert!(!crate::render::build(&game).is_empty());
    }

    fn pick_superweapon(game: &mut Game) {
        if game.superweapon().is_none() {
            give_palace(game);
        }
        click_hud(game, Action::BuildTab(BuildTab::Vehicles));
        if let Some(cell) = game.superweapon_cell() {
            click_hud(game, Action::BuildCell(cell));
        }
    }

    /// The cell of a build menu that shows what makes this now.
    fn cell_of(game: &Game, tab: BuildTab, role: Role) -> usize {
        game.menu(tab)
            .iter()
            .position(|&entry| entry == Entry::Make(role))
            .unwrap_or_else(|| panic!("{role:?} is on offer"))
    }

    /// Clicks the cell of a build menu that shows what makes this now.
    fn click_role(game: &mut Game, tab: BuildTab, role: Role) {
        let cell = cell_of(game, tab, role);
        click_hud(game, Action::BuildCell(cell));
    }

    /// Whether a build menu shows what makes this now.
    fn offers(game: &Game, tab: BuildTab, role: Role) -> bool {
        game.menu(tab).contains(&Entry::Make(role))
    }

    #[test]
    fn the_superweapon_lives_in_the_vehicles_menu_only() {
        let mut game = in_battle();
        give_palace(&mut game);
        let cell = game.superweapon_cell().unwrap();
        for tab in [BuildTab::Infantry, BuildTab::Buildings] {
            assert!(!game.menu(tab).contains(&Entry::Superweapon), "{tab:?}");
        }
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_hud(&mut game, Action::BuildCell(cell + 1));
        assert!(!game.aiming_strike, "nor in another cell");
        click_hud(&mut game, Action::BuildCell(cell));
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
        let cell = game.superweapon_cell().unwrap();
        click_hud(&mut game, Action::BuildCell(cell));
        assert!(!game.aiming_strike, "still recharging");
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        for _ in 0..(SUPERWEAPON_RECHARGE as usize + 1) {
            game.update(1.0);
        }
        if game.outcome.is_none() {
            click_hud(&mut game, Action::BuildCell(cell));
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
            unit.x = enemy.x + 45.25;
            unit.z = enemy.z + 45.25;
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
        with_bases(&mut game);
        // Nobody fires on these without an order, and the drone circles out of reach.
        game.units
            .retain(|u| u.owner != 1 || !(u.kind == UnitKind::Building || u.role == Role::AtAdp));
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
        // Everyone of the player's marches into range together and fires by themselves.
        select_first_group(&mut game);
        units::order_march(
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
        assert_eq!(game.killed.buildings, 2, "its towers fell too");
    }

    #[test]
    fn nothing_fires_while_the_game_is_paused() {
        let mut game = in_battle();
        let enemy = game.units.iter().find(|u| u.owner == 1).copied().unwrap();
        game.units[0].x = enemy.x + 45.25;
        game.units[0].z = enemy.z + 45.25;
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
            radius: 90.51,
            life: 0.5,
        });
        game.combat.beams.push(crate::weapons::Beam {
            from: (x, 45.25, z),
            to: (x + 135.8, 45.25, z),
            life: 0.5,
            repair: false,
        });
        game.combat.beams.push(crate::weapons::Beam {
            from: (x, 45.25, z),
            to: (x + 135.8, 45.25, z),
            life: 0.5,
            repair: true,
        });
        game.combat.flares.push(crate::weapons::Flare {
            at: (x, 45.25, z),
            direction: (1.0, 0.0, 0.0),
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
        if matches!(action, Action::BuildCell(_)) {
            game.finish_all_making();
        }
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
    fn every_sidebar_button_lights_up_takes_a_click_and_stays_in_the_battle() {
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
        assert_eq!(
            game.build_tab,
            BuildTab::Starport,
            "the last wide one clicked"
        );
        assert_eq!(game.outcome, None, "one click on Retreat only asks");
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
        assert_eq!(chat_texts(&game), ["Rocket Turret lost"]);

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
    fn diplomacy_knows_who_is_defeated_and_that_stays_so() {
        let mut game = in_battle();
        assert!(!game.is_defeated(1), "nobody has lost anything yet");
        kill_owners(&mut game, &[1]);
        assert!(game.is_defeated(1));
        assert!(!game.is_defeated(0));
        game.update(0.01);
        assert!(game.is_defeated(1));
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
        give_palace(&mut game);
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

    /// How many units the player's first group has, not counting its towers, which a box
    /// leaves out.
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
            (W / 2.0 - 480.0, H / 2.0 - 480.0),
            (W / 2.0 + 480.0, H / 2.0 + 480.0),
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

        let (x, y) = clear_spot_on_screen();
        click_at(&mut game, x, y);
        assert!(game.units[index].is_moving(), "sent there");
    }

    #[test]
    fn the_world_goes_to_the_backend_in_3d_with_normals_and_effects_with_depth() {
        let mut game = in_battle();
        let frame = crate::render::build_frame(&game);
        assert!(!frame.ground.is_empty() && !frame.objects.is_empty());
        for v in frame.ground.iter().chain(&frame.objects) {
            let length = v.normal.iter().map(|c| c * c).sum::<f32>().sqrt();
            assert!((length - 1.0).abs() < 0.01, "{v:?}");
        }
        let all_on_screen = frame.ground.len() + frame.objects.len();
        game.units.retain(|u| !u.kind.flies());
        let without_planes = crate::render::build_frame(&game);
        assert!(without_planes.ground.len() + without_planes.objects.len() < all_on_screen);
        assert!(!without_planes.effects.is_empty(), "headlights");
        for v in &without_planes.effects {
            assert!(v.depth > 0.0 && v.depth < 1.0, "{v:?}");
        }
    }

    /// The lights of a frame but those of searchlights, which shine on whatever burns.
    fn burning_lights(frame: &crate::platform::Frame) -> Vec<crate::platform::PointLight> {
        frame
            .lights
            .iter()
            .filter(|light| light.color != crate::render::SEARCHLIGHT_COLOR)
            .copied()
            .collect()
    }

    #[test]
    fn a_blast_gives_the_backend_a_light_and_what_is_in_the_air_casts_a_shadow() {
        let mut game = in_battle();
        assert!(burning_lights(&crate::render::build_frame(&game)).is_empty());
        let (x, y, z) = game.units[0].center();
        game.combat.explode((x, y, z), 45.25);
        let frame = crate::render::build_frame(&game);
        assert_eq!(burning_lights(&frame).len(), 1);
        assert_eq!(burning_lights(&frame)[0].place[0], x);
        for _ in 0..crate::platform::MAX_LIGHTS * 2 {
            game.combat.explode((x, y, z), 45.25);
        }
        let many = burning_lights(&crate::render::build_frame(&game)).len();
        assert_eq!(many, crate::platform::MAX_LIGHTS);
    }

    #[test]
    fn an_eruption_gives_the_backend_a_light_while_it_burns() {
        let mut game = in_battle();
        game.fog = None;
        game.combat
            .eruptions
            .push(crate::weapons::Eruption::at_for_test((0.0, 0.0, 1810.0)));
        let frame = crate::render::build_frame(&game);
        assert_eq!(burning_lights(&frame).len(), 1);
        assert!(burning_lights(&frame)[0].color[0] > 0.0);
        assert!(
            burning_lights(&frame)[0].color[0] > burning_lights(&frame)[0].color[2],
            "fiery"
        );
    }

    #[test]
    fn every_house_has_the_discs_picture_for_all_it_builds_but_the_ordos_landing_pad() {
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let game = in_battle_as(house);
            for tab in [BuildTab::Infantry, BuildTab::Vehicles, BuildTab::Buildings] {
                for role in (0..hud::grid_cells(W, H).len()).filter_map(|i| game.build_role(tab, i))
                {
                    let missing = house == Faction::Ordos && role == Role::Helipad;
                    assert_eq!(
                        crate::render::names_icon(role, house),
                        !missing,
                        "{house:?} {role:?}"
                    );
                }
            }
            for ally in [
                Ally::Tleilaxu,
                Ally::Ix,
                Ally::Guild,
                Ally::Fremen,
                Ally::Sardaukar,
            ] {
                for role in [ally.main_building(), ally.unit()].into_iter().flatten() {
                    assert!(crate::render::names_icon(role, house), "{house:?} {role:?}");
                }
            }
        }
    }

    #[test]
    fn a_landing_pad_is_built_from_the_buildings_menu_and_every_side_starts_with_one() {
        let mut game = in_battle();
        assert!(offers(&game, BuildTab::Buildings, Role::Helipad));
        for owner in 0..game.teams.len() {
            assert!(
                game.units
                    .iter()
                    .any(|u| u.owner == owner && u.role == Role::Helipad),
                "owner {owner}"
            );
        }
        let pad = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Helipad)
            .unwrap();
        game.units[pad].selected = true;
        units::set_gathering(&mut game.units, (0.0, 1810.0));
        assert_eq!(game.units[pad].gathering, None, "it makes nothing to send");
    }

    #[test]
    fn damage_stains_what_is_worn_and_burning_gives_a_light_and_flames() {
        let mut game = in_battle();
        game.fog = None;
        let stained = |game: &Game| {
            crate::render::build_frame(game)
                .objects
                .iter()
                .filter(|v| v.stain[2] > 0.0)
                .count()
        };
        assert_eq!(stained(&game), 0);
        let before = crate::render::build_frame(&game);
        assert!(burning_lights(&before).is_empty());
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
            .unwrap();
        game.units[tank].health = game.units[tank].max_health() * 0.5;
        assert!(stained(&game) > 0, "black stains");
        assert!(
            burning_lights(&crate::render::build_frame(&game)).is_empty(),
            "not burning yet"
        );
        game.units[tank].health = game.units[tank].max_health() * 0.2;
        let burning = crate::render::build_frame(&game);
        assert_eq!(burning_lights(&burning).len(), 1);
        assert!(burning.effects.len() > before.effects.len(), "flames");
    }

    #[test]
    fn a_machine_short_of_rockets_shows_what_it_has_left_and_a_full_one_does_not() {
        let mut game = in_battle();
        game.units
            .retain(|u| u.owner != 0 || u.kind != UnitKind::Aircraft);
        let id = game.next_unit_id;
        let terrain = battlefield::terrain(game.battle_map);
        let side = (0, game.stats_rows()[0].color);
        let mut machine =
            units::new_building(terrain, battlefield::MAP_MIDDLE, Role::AtOrni, side, id);
        machine.selected = true;
        game.units.push(machine);
        let full = crate::render::build(&game).len();
        game.units.last_mut().unwrap().rockets_fired = 3;
        assert!(crate::render::build(&game).len() > full);
    }

    /// Adds an enemy tank where the player's second group stands, and returns where it is.
    fn enemy_tank_in(game: &mut Game) -> usize {
        let id = game.next_unit_id;
        game.next_unit_id += 1;
        let terrain = battlefield::terrain(game.battle_map);
        let at = terrain
            .nearest_walkable(
                battlefield::Going::Driving,
                (battlefield::MAP_MIDDLE.0 + 181.0, battlefield::MAP_MIDDLE.1),
            )
            .unwrap();
        let tank = units::new_building(terrain, at, Role::HkAssault, (1, TeamColor::Red), id);
        game.units.push(tank);
        game.units.len() - 1
    }

    #[test]
    fn a_click_on_an_enemy_vehicle_with_a_carryall_selected_sends_it_to_steal_it() {
        let mut game = in_battle();
        game.fog = None;
        let carryall = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::AdvCarryall)
            .unwrap();
        let enemy = enemy_tank_in(&mut game);
        game.camera.x = game.units[enemy].x;
        game.camera.z = game.units[enemy].z;
        let unit = game.units[enemy];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.units[carryall].selected = true;
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Carry);
        click_at(&mut game, x, y);
        assert_eq!(
            game.units[carryall].target,
            Some(crate::units::Target::Unit(unit.id))
        );
    }

    #[test]
    fn a_stolen_vehicle_draws_with_cables_while_held_and_while_hanging() {
        let mut game = in_battle();
        let carryall = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::AdvCarryall)
            .unwrap();
        let carrier = game.units[carryall].id;
        let enemy = enemy_tank_in(&mut game);
        // Cables are the only thing of their color.
        let cables = |game: &Game| {
            let frame = crate::render::build_frame(game);
            let is_cable = |v: &&crate::platform::WorldVertex| (v.r, v.g, v.b) == (0.1, 0.1, 0.1);
            frame.objects.iter().filter(is_cable).count()
        };
        assert_eq!(cables(&game), 0);
        game.units[enemy].theft = Some(crate::units::Theft {
            carrier,
            struggle: 3.0,
        });
        let held = cables(&game);
        assert!(held > 0, "two cables");
        game.units[enemy].theft = Some(crate::units::Theft {
            carrier,
            struggle: 0.0,
        });
        game.units[enemy].altitude = 45.25;
        assert!(cables(&game) > 0, "still hanging on them");
    }

    #[test]
    fn gunfire_draws_flames_going_the_way_of_the_shot_and_lights_what_is_around_it() {
        let mut game = in_battle();
        game.fog = None;
        let (x, y, z) = game.units[0].center();
        let flare = |direction| crate::weapons::Flare {
            at: (x, y, z),
            direction,
            size: 16.0,
            life: 1.0,
        };
        let before = crate::render::build_frame(&game);
        game.combat.flares.push(flare((1.0, 0.0, 0.0)));
        let right = crate::render::build_frame(&game);
        assert_eq!(right.lights.len(), before.lights.len() + 1);
        assert!(right.effects.len() > before.effects.len());
        game.combat.flares[0] = flare((-1.0, 0.0, 0.0));
        let left = crate::render::build_frame(&game);
        let (from_x, _) = game.view().project(x, y, z).unwrap();
        // The tips of the flames carry no light at all, and lie the way of the shot.
        let tips = |frame: &crate::platform::Frame| {
            let new = &frame.effects[before.effects.len()..];
            let mean = |it: Vec<f32>| it.iter().sum::<f32>() / it.len() as f32;
            mean(
                new.iter()
                    .filter(|v| v.a == 0.0 && v.r == 0.0)
                    .map(|v| v.x)
                    .collect(),
            )
        };
        assert!(tips(&right) > from_x, "to the right");
        assert!(tips(&left) < from_x, "to the left");
    }

    #[test]
    fn a_skirmish_starts_with_the_camera_on_the_players_base_away_from_the_middle() {
        let mut game = in_setup();
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        assert!(!crate::render::build(&game).is_empty());
        let player = *game.units.iter().find(|u| u.owner == 0).unwrap();
        assert_eq!(game.home(), (player.x, player.z));
        assert_eq!((game.camera.x, game.camera.z), (player.x, player.z));
        let (middle_x, middle_z) = battlefield::MAP_MIDDLE;
        assert!(
            (player.x - middle_x).hypot(player.z - middle_z) > 1810.0,
            "not in the middle: {player:?}"
        );
    }

    /// A battle with the cursor over open ground, and nothing selected.
    fn cursor_on_open_ground() -> Game {
        let mut game = in_battle();
        game.units.iter_mut().for_each(|u| u.selected = false);
        let (x, y) = clear_spot_on_screen();
        game.handle(Event::MouseMove(x, y));
        game
    }

    #[test]
    fn the_cursor_shows_where_the_selected_would_go_only_when_a_click_would_send_them() {
        let mut game = cursor_on_open_ground();
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "nothing selected");
        let ring = crate::render::MOVE_CURSOR;
        assert_eq!(line_vertices(&game, ring), 0);

        let soldier = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .unwrap();
        game.units[soldier].selected = true;
        assert_eq!(game.cursor_look(), CursorLook::Move);
        assert!(line_vertices(&game, ring) > 0, "it draws");
        assert!(!game.cursor_means_attack());

        // A click on open ground then does what the cursor said.
        let (x, y) = game.mouse;
        click_at(&mut game, x, y);
        assert!(game.units[soldier].is_moving());

        // Over the HUD it is an arrow again.
        let button = hud::all_buttons(W, H)[0];
        game.handle(Event::MouseMove(button.x + 1.0, button.y + 1.0));
        assert_eq!(game.cursor_look(), CursorLook::Arrow);
    }

    #[test]
    fn what_cannot_go_anywhere_shows_no_move_cursor_but_a_building_setting_its_point_does() {
        let selecting = |wanted: fn(&Unit) -> bool| {
            let mut game = cursor_on_open_ground();
            let chosen = game.units.iter().position(|u| u.owner == 0 && wanted(u));
            game.units[chosen.unwrap()].selected = true;
            game.cursor_look()
        };
        assert_eq!(selecting(|u| u.kind == UnitKind::Tower), CursorLook::Arrow);
        assert_eq!(selecting(|u| u.role == Role::Helipad), CursorLook::Arrow);
        assert_eq!(selecting(|u| u.role == Role::Factory), CursorLook::Move);
        assert_eq!(selecting(|u| u.kind == UnitKind::Vehicle), CursorLook::Move);
    }

    #[test]
    fn an_aircraft_barely_killed_falls_and_one_killed_hard_blows_apart_in_the_air() {
        let plane = |game: &Game| {
            game.units
                .iter()
                .position(|u| u.owner == 0 && u.kind == UnitKind::Aircraft && u.altitude > 0.0)
                .unwrap()
        };
        let mut game = in_battle();
        let barely = plane(&game);
        game.units[barely].health = -1.0;
        game.units[barely].alive = false;
        game.update(0.01);
        assert_eq!(game.combat.wrecks.len(), 1, "it falls");
        assert!(game.combat.blasts.is_empty(), "with no blast yet");
        assert_eq!(game.lost.aircraft, 1, "and is lost already");
        let drawn = crate::render::build_frame(&game).objects.len();
        game.combat.wrecks.clear();
        assert!(
            crate::render::build_frame(&game).objects.len() < drawn,
            "it draws"
        );

        let mut game = in_battle();
        let hard = plane(&game);
        game.units[hard].health = -game.units[hard].max_health();
        game.units[hard].alive = false;
        game.update(0.01);
        assert!(game.combat.wrecks.is_empty());
        assert!(game.combat.blasts.len() > 1, "it blows apart");
    }

    /// A Kindjal infantry of the player's on open ground, as yet unselected, and the cursor over it.
    fn at_kindjal_under_cursor() -> (Game, usize) {
        let mut game = in_battle();
        game.units.iter_mut().for_each(|u| u.selected = false);
        game.units
            .retain(|u| u.owner != 0 || u.kind != UnitKind::Infantry);
        let (x, y) = clear_spot_on_screen();
        let spot = game.view().ground_at(x, y).unwrap();
        let terrain = battlefield::terrain(game.battle_map);
        let side = (0, game.stats_rows()[0].color);
        let id = game.next_unit_id;
        game.next_unit_id += 1;
        let soldier = units::new_building(terrain, spot, Role::AtKindjal, side, id);
        game.units.push(soldier);
        let index = game.units.len() - 1;
        let (px, py) = game
            .view()
            .project(soldier.x, soldier.pick_height(), soldier.z)
            .unwrap();
        game.handle(Event::MouseMove(px, py));
        (game, index)
    }

    #[test]
    fn soldiers_board_an_apc_with_their_own_cursor_get_out_on_deploy_and_die_with_it() {
        let (mut game, apc) = at_kindjal_under_cursor();
        game.units[apc].role = Role::AtApc;
        game.units[apc].kind = UnitKind::Vehicle;
        let (x, y) = game.mouse;
        let terrain = battlefield::terrain(game.battle_map);
        let side = (0, game.stats_rows()[0].color);
        let near = beside((game.units[apc].x, game.units[apc].z), 90.51);
        let id = game.next_unit_id;
        game.next_unit_id += 1;
        let mut soldier = units::new_building(terrain, near, Role::AtInfantry, side, id);
        soldier.selected = true;
        game.units.push(soldier);
        assert_eq!(game.cursor_look(), CursorLook::Board);
        assert!(
            line_vertices(&game, crate::render::MOVE_CURSOR) > 0,
            "it draws"
        );
        click_at(&mut game, x, y);
        for _ in 0..100 {
            game.update(0.05);
        }
        assert!(
            game.units.iter().all(|u| u.id != id),
            "on board, out of sight"
        );
        let apc = game
            .units
            .iter()
            .position(|u| u.role == Role::AtApc)
            .unwrap();
        game.units[apc].selected = true;
        assert_eq!(game.cursor_look(), CursorLook::Deploy);
        click_at(&mut game, x, y);
        assert!(game.units.iter().any(|u| u.id == id), "out again");

        game.units.retain(|u| u.id != id);
        game.units[apc].passengers[0] = Some(units::Passenger::for_test(Role::AtInfantry));
        let lost = game.lost.infantry;
        game.units[apc].alive = false;
        game.update(0.01);
        assert_eq!(game.lost.infantry, lost + 1, "the passenger died with it");
    }

    #[test]
    fn hovering_a_selected_deployable_unit_shows_what_a_click_would_do_and_a_click_does_it() {
        let (mut game, soldier) = at_kindjal_under_cursor();
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "not selected yet");
        game.units[soldier].selected = true;
        assert_eq!(game.cursor_look(), CursorLook::Deploy);
        let deploy_look = line_vertices(&game, crate::render::MOVE_CURSOR);
        assert!(deploy_look > 0, "it draws");
        let (x, y) = game.mouse;
        click_at(&mut game, x, y);
        assert!(game.units[soldier].deployed);
        assert_eq!(
            game.units[soldier].weapon,
            crate::weapons::WeaponKind::Cannon
        );
        assert!(game.units[soldier].selected, "it stays selected");
        assert_eq!(game.cursor_look(), CursorLook::Undeploy);
        assert!(line_vertices(&game, crate::render::MOVE_CURSOR) > 0);
        click_at(&mut game, x, y);
        assert!(!game.units[soldier].deployed);
        assert_eq!(game.cursor_look(), CursorLook::Deploy);
    }

    #[test]
    fn two_clicks_on_a_deployable_unit_not_yet_selected_deploy_it_and_two_more_bring_it_back() {
        let (mut game, soldier) = at_kindjal_under_cursor();
        let (x, y) = game.mouse;
        click_at(&mut game, x, y);
        assert!(game.units[soldier].selected);
        assert!(!game.units[soldier].deployed, "the first only selects it");
        click_at(&mut game, x, y);
        assert!(game.units[soldier].deployed);

        game.units[soldier].selected = false;
        click_at(&mut game, x, y);
        assert!(game.units[soldier].deployed, "the first only selects it");
        click_at(&mut game, x, y);
        assert!(!game.units[soldier].deployed);
    }

    #[test]
    fn hovering_what_cannot_deploy_shows_no_deploy_cursor_and_a_click_on_it_still_picks_it() {
        let mut game = in_battle();
        game.units.iter_mut().for_each(|u| u.selected = false);
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
            .unwrap();
        game.units[tank].selected = true;
        let unit = game.units[tank];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Arrow);
        click_at(&mut game, x, y);
        assert!(game.units[tank].selected);
        assert!(!game.units[tank].deployed);
    }

    #[test]
    fn a_selected_building_shows_a_red_flag_on_its_gathering_point_as_well_as_the_line() {
        let red = |game: &Game| {
            crate::render::build_frame(game)
                .objects
                .iter()
                .filter(|v| v.r > 0.9 && v.g < 0.2 && v.b < 0.2)
                .map(|v| (v.x, v.z))
                .collect::<Vec<_>>()
        };
        let mut game = in_battle();
        game.units.iter_mut().for_each(|u| u.selected = false);
        let barracks = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Barracks)
            .unwrap();
        game.units.retain(|u| u.owner == 0 || !u.role.makes_units());
        let others = red(&game).len();
        game.units[barracks].gathering = Some((135.8, 3168.0));
        assert_eq!(red(&game).len(), others, "only while it is selected");

        game.units[barracks].selected = true;
        let flag = red(&game);
        assert_eq!(flag.len(), others + 3, "one triangle");
        assert!(
            flag.iter().any(|&(x, z)| (x, z) == (135.8, 3168.0)),
            "at the point"
        );
        assert!(line_vertices(&game, crate::render::ORDER_LINE) > 0);

        game.units[barracks].gathering = None;
        assert_eq!(red(&game).len(), others);
    }

    #[test]
    fn nothing_of_the_world_is_drawn_outside_a_battle() {
        let game = Game::new(W, H);
        let frame = crate::render::build_frame(&game);
        assert!(frame.ground.is_empty() && frame.objects.is_empty() && frame.lights.is_empty());
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
    /// The point this far to the right of another one on the screen, or to its left if it is
    /// less than nothing.
    fn beside((x, z): (f32, f32), right: f32) -> (f32, f32) {
        let (dx, dz) = battlefield::from_across_and_down(right, 0.0);
        (x + dx, z + dz)
    }

    fn enemy_beside_first_group(game: &mut Game) -> (usize, (f32, f32)) {
        let index = game.units.iter().position(|u| u.owner == 1).unwrap();
        let (x, z) = beside((game.camera.x, game.camera.z), 271.5);
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
            .project(own.x, own.y + own.altitude + 11.31, own.z)
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
        game.units[enemy].altitude = 135.8;
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
        // Nobody comes to finish them off in the minute this takes.
        game.units.retain(|u| u.owner == 0 || !u.role.makes_units());
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

    fn tower_of_role(game: &Game, owner: usize, role: Role) -> usize {
        game.units
            .iter()
            .position(|u| u.owner == owner && u.role == role)
            .unwrap()
    }

    fn tower_of(game: &Game, owner: usize) -> usize {
        game.units
            .iter()
            .position(|u| u.owner == owner && u.role == Role::AtRocketTurret)
            .unwrap()
    }

    #[test]
    fn the_players_tower_is_picked_only_by_a_click_and_on_its_own() {
        let mut game = in_battle();
        let tower = tower_of(&game, 0);
        let (from, to) = around_first_group();
        drag_select(&mut game, from, to);
        assert!(!game.units[tower].selected, "a box leaves it out");
        assert_eq!(selected_count(&game), FIRST_GROUP);

        let unit = game.units[tower];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        game.handle(Event::ShiftMouseUp);
        assert_eq!(selected_count(&game), 1, "even with Shift, on its own");
        assert!(game.units[tower].selected);

        let other = tower_of_role(&game, 0, Role::HkFlameTurret);
        let unit = game.units[other];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        game.handle(Event::ShiftMouseUp);
        assert_eq!(selected_count(&game), 1, "one building at a time");
        assert!(game.units[other].selected);

        game.handle(Event::RightDown);
        game.handle(Event::KeyDown(Key::Char('a')));
        assert!(!game.units[tower].selected, "but not by the select-all key");
    }

    #[test]
    fn a_selected_tower_can_be_forced_to_fire_on_a_place_or_a_friend() {
        let mut game = in_battle();
        let tower = tower_of(&game, 0);
        game.units[tower].selected = true;
        let (x, y) = clear_spot_on_screen();
        let place = game.view().ground_at(x, y).unwrap();
        game.handle(Event::Control(true));
        assert_eq!(game.cursor_look(), CursorLook::Attack);
        click_at(&mut game, x, y);
        assert_eq!(
            game.units[tower].target,
            Some(crate::units::Target::Place(place.0, place.1))
        );

        let friend = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap();
        let unit = game.units[friend];
        let (x, y) = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        click_at(&mut game, x, y);
        assert_eq!(
            game.units[tower].target,
            Some(crate::units::Target::Forced(unit.id))
        );
        assert_eq!(
            game.units[tower].destination,
            (game.units[tower].x, game.units[tower].z)
        );
    }

    #[test]
    fn a_building_that_makes_units_is_picked_only_by_a_click_and_sets_its_gathering_point() {
        let mut game = in_battle();
        let barracks = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Barracks)
            .unwrap();
        let unit = game.units[barracks];
        let at = game
            .view()
            .project(unit.x, unit.pick_height(), unit.z)
            .unwrap();
        // A box right around it finds nothing, and neither does the select-all key.
        drag_select(
            &mut game,
            (at.0 - 60.0, at.1 - 60.0),
            (at.0 + 60.0, at.1 + 60.0),
        );
        assert!(!game.units[barracks].selected);
        game.handle(Event::KeyDown(Key::Char('a')));
        assert!(!game.units[barracks].selected);
        assert!(selected_count(&game) > 0);

        // A click takes it alone, even with Shift.
        game.handle(Event::MouseMove(at.0, at.1));
        game.handle(Event::MouseDown);
        game.handle(Event::ShiftMouseUp);
        assert!(game.units[barracks].selected);
        assert_eq!(selected_count(&game), 1);

        assert_eq!(
            line_vertices(&game, crate::render::ORDER_LINE),
            0,
            "no point yet"
        );
        let (x, y) = clear_spot_on_screen();
        let point = game.view().ground_at(x, y).unwrap();
        click_at(&mut game, x, y);
        assert_eq!(game.units[barracks].gathering, Some(point));
        assert!(game.units[barracks].selected, "the selection stands");
        assert!(
            line_vertices(&game, crate::render::ORDER_LINE) > 0,
            "a dashed line"
        );

        let before = game.units.len();
        click_hud(&mut game, Action::BuildTab(BuildTab::Infantry));
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(game.units.len(), before + 1);
        assert_eq!(game.units[before].destination, point);

        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::RightDown);
        assert_eq!(
            line_vertices(&game, crate::render::ORDER_LINE),
            0,
            "only when selected"
        );
    }

    #[test]
    fn an_enemy_tower_can_be_clicked_and_destroyed_and_counts_as_a_building() {
        let mut game = in_battle();
        let tower = tower_of(&game, 1);
        let (x, z) = beside((game.camera.x, game.camera.z), 271.5);
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
            "Infantry",
            "Sniper",
            "Engineer",
            "Kindjal Infantry",
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
            assert!((new.x - tower.x).hypot(new.z - tower.z) < 543.1, "{new:?}");
            assert!(
                game.units[..before].iter().all(|u| u.id != new.id),
                "its id is its own"
            );
        }
    }

    #[test]
    fn the_vehicles_menu_lists_its_houses_own_in_the_originals_order_with_the_superweapon_after_the_hangars()
     {
        use Entry::{Make, Superweapon};
        for (house, ground, hangar, drone) in [
            (
                Faction::Atreides,
                &[
                    Role::AtTrike,
                    Role::AtMongoose,
                    Role::AtApc,
                    Role::AtRepairUnit,
                    Role::AtMinotaurus,
                ][..],
                &[Role::AtOrni][..],
                Role::AtAdp,
            ),
            (
                Faction::Harkonnen,
                &[
                    Role::HkBuzzsaw,
                    Role::HkAssault,
                    Role::HkFlame,
                    Role::HkInkVine,
                    Role::HkMissile,
                    Role::HkDevastator,
                ][..],
                &[Role::HkGunship][..],
                Role::HkAdp,
            ),
            (
                Faction::Ordos,
                &[
                    Role::OrDustScout,
                    Role::OrLaserTank,
                    Role::OrApc,
                    Role::OrKobra,
                ][..],
                &[Role::OrEits][..],
                Role::OrAdp,
            ),
        ] {
            let mut game = in_battle_as(house);
            give_palace(&mut game);
            let mut expected: Vec<Entry> = ground.iter().chain(hangar).map(|&r| Make(r)).collect();
            expected.extend([
                Make(Role::AdvCarryall),
                Superweapon,
                Make(drone),
                Make(Role::Harvester),
                Make(Role::Mcv),
                Make(Role::Carryall),
            ]);
            assert_eq!(game.menu(BuildTab::Vehicles), expected, "{house:?}");
            click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
            for (cell, &role) in ground.iter().enumerate() {
                let before = game.units.len();
                click_hud(&mut game, Action::BuildCell(cell));
                assert_eq!(game.units.len(), before + 1, "{house:?} {role:?}");
                assert_eq!(game.units[before].role, role);
                assert_eq!(game.units[before].kind, UnitKind::Vehicle);
                assert_eq!(game.units[before].house, house);
            }
            let before = game.units.len();
            let cell = game.superweapon_cell().unwrap();
            click_hud(&mut game, Action::BuildCell(cell));
            assert!(game.aiming_strike);
            assert_eq!(game.units.len(), before);
        }
    }

    #[test]
    fn a_menu_shows_only_what_can_be_made_now_packed_from_its_first_cell() {
        let mut game = in_battle();
        game.units.retain(|u| {
            u.owner != 0 || !matches!(u.role, Role::Factory | Role::Hanger | Role::Helipad)
        });
        assert_eq!(
            game.menu(BuildTab::Vehicles),
            [],
            "nothing makes vehicles or aircraft"
        );
        let buildings = game.menu(BuildTab::Buildings);
        assert_eq!(
            buildings,
            [
                Role::Windtrap,
                Role::Barracks,
                Role::Wall,
                Role::Refinery,
                Role::Factory,
                Role::Outpost,
                Role::AtPillbox,
                Role::AtRocketTurret,
                Role::Palace(Faction::Atreides),
            ]
            .map(Entry::Make),
            "towers need the barracks; the hangar, the pad and the starport wait: the palace \
             needs only the starport, which stands"
        );
        game.units
            .retain(|u| u.owner != 0 || u.role != Role::Barracks);
        assert!(!offers(&game, BuildTab::Buildings, Role::AtPillbox));
        assert_eq!(game.menu(BuildTab::Infantry), []);
    }

    #[test]
    fn the_buildings_menu_lists_in_the_originals_order_and_ordos_have_no_landing_pad() {
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let game = in_battle_as(house);
            let [lesser, greater] = Role::towers_of(house);
            let mut expected = vec![
                Role::Windtrap,
                Role::Barracks,
                Role::Wall,
                Role::Refinery,
                Role::Factory,
                Role::Outpost,
                lesser,
                greater,
                Role::Hanger,
                Role::Helipad,
                Role::Starport,
                Role::Palace(house),
            ];
            if house == Faction::Ordos {
                expected.retain(|&role| role != Role::Helipad);
            }
            assert_eq!(
                game.menu(BuildTab::Buildings),
                expected.into_iter().map(Entry::Make).collect::<Vec<_>>(),
                "{house:?}"
            );
            assert_eq!(Role::Helipad.made_by(house), house != Faction::Ordos);
        }
    }

    #[test]
    fn the_infantry_menu_lists_in_the_originals_order() {
        for (house, expected) in [
            (
                Faction::Atreides,
                [
                    Role::Scout,
                    Role::AtInfantry,
                    Role::AtSniper,
                    Role::Engineer,
                    Role::AtKindjal,
                ],
            ),
            (
                Faction::Harkonnen,
                [
                    Role::Scout,
                    Role::HkLightInf,
                    Role::HkTrooper,
                    Role::Engineer,
                    Role::HkFlamer,
                ],
            ),
            (
                Faction::Ordos,
                [
                    Role::Scout,
                    Role::OrChemical,
                    Role::Engineer,
                    Role::OrMortar,
                    Role::OrSaboteur,
                ],
            ),
        ] {
            let game = in_battle_as(house);
            assert_eq!(
                game.menu(BuildTab::Infantry),
                expected.map(Entry::Make),
                "{house:?}"
            );
        }
    }

    #[test]
    fn every_vehicle_and_tower_shines_white_light_ahead_from_each_of_its_lamps_and_nothing_else_does()
     {
        let lights = |game: &Game| {
            let frame = crate::render::build(game);
            let lit = || frame.iter().filter(|v| v.a == 0.0 && v.r > 0.0);
            // A beam stopped by a building ends part way to full brightness.
            let brightest = lit().map(|v| v.r).fold(0.0, f32::max);
            lit()
                .inspect(|v| assert_eq!((v.r, v.g), (v.b, v.b), "white"))
                .filter(|v| v.r == brightest)
                .count()
        };
        let mut game = in_battle();
        // Rocket engines are lit too, in orange, and landing pads and MCVs have searchlights,
        // and a deployed MCV red glares. The player's own stand where nothing is in the way of
        // their beams.
        game.units.retain(|u| {
            u.owner == 0 && !u.kind.flies() && !matches!(u.role, Role::Helipad | Role::Mcv)
        });
        let lamps = |game: &Game| {
            let has_lamps =
                |u: &&Unit| u.alive && matches!(u.kind, UnitKind::Vehicle | UnitKind::Tower);
            let vehicles = game.units.iter().filter(has_lamps);
            vehicles
                .map(|u| crate::render::lamps_on(u.role))
                .sum::<usize>()
        };
        assert!(lamps(&game) > 0);
        assert_eq!(
            lights(&game),
            lamps(&game) * 3,
            "three bright corners of two triangles from each lens"
        );
        // A flame turret has no searchlight.
        game.units
            .retain(|u| !matches!(u.kind, UnitKind::Vehicle) && u.role != Role::AtRocketTurret);
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
        let bare = |house: Faction| {
            let mut game = in_battle_as(house);
            game.units
                .retain(|u| matches!(u.kind, UnitKind::Infantry | UnitKind::Building));
            game.units
                .retain(|u| !matches!(u.role, Role::Helipad | Role::Mcv));
            assert_eq!(lit(&game), 0);
            click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
            game
        };
        let mut game = bare(Faction::Harkonnen);
        click_role(&mut game, BuildTab::Vehicles, Role::HkAdp);
        assert_eq!(game.units.last().unwrap().name(), "Air Defense Platform");
        assert_eq!(
            lit(&game),
            2 * 3,
            "two near corners, shared, in each of two"
        );
        let mut game = bare(Faction::Atreides);
        click_role(&mut game, BuildTab::Vehicles, Role::AtAdp);
        assert_eq!(game.units.last().unwrap().name(), "Air Drone");
        assert_eq!(lit(&game), 2 * 2, "a flame of two crossed triangles");
        click_role(&mut game, BuildTab::Vehicles, Role::AtOrni);
        assert_eq!(game.units.last().unwrap().name(), "Ornithopter");
        assert_eq!(game.units.last().unwrap().altitude, 0.0);
    }

    #[test]
    fn a_carryall_has_a_model_one_big_lamp_two_jet_flames_and_leaves_vapor_from_its_jets() {
        let lit = |game: &Game| {
            crate::render::build(game)
                .iter()
                .filter(|v| v.a == 0.0 && v.r > 0.0)
                .count()
        };
        let mut game = in_battle();
        game.skirmish.crates = false;
        game.units
            .retain(|u| matches!(u.kind, UnitKind::Infantry | UnitKind::Building));
        game.units
            .retain(|u| !matches!(u.role, Role::Helipad | Role::Mcv));
        let objects = |game: &Game| crate::render::build_frame(game).objects.len();
        let (dark, bare) = (lit(&game), objects(&game));
        assert_eq!(dark, 0);
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_role(&mut game, BuildTab::Vehicles, Role::Carryall);
        assert_eq!(game.units.last().unwrap().name(), "Carryall");
        assert!(objects(&game) > bare + 500, "a model of its own");
        assert_eq!(lit(&game), 3 + 2 * 2 * 2, "a lamp, and two flames");
        game.update(0.5);
        let jets = game.units.last().unwrap().wing_tips();
        assert!(
            game.combat.wisps.iter().any(|w| jets
                .iter()
                .any(|j| crate::units::distance(*j, w.position) < 45.25)),
            "vapor behind its jets"
        );
    }

    #[test]
    fn a_selected_machine_shows_the_land_cursor_over_its_own_pad_and_a_click_lands_it_there() {
        let mut game = in_battle();
        game.fog = None;
        let pad = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Helipad)
            .unwrap();
        let machine = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Aircraft)
            .unwrap();
        game.units[machine].role = Role::AtOrni;
        let at = game.units[pad];
        game.camera.look_at(at.x, at.z);
        let (x, y) = game.view().project(at.x, at.pick_height(), at.z).unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "nothing selected");
        game.units[machine].selected = true;
        assert_eq!(game.cursor_look(), CursorLook::Land);
        click_at(&mut game, x, y);
        assert!(
            game.units[machine].selected,
            "the pad is not picked instead"
        );
        assert!(!game.units[pad].selected);
        assert_eq!(game.units[machine].destination, (at.x, at.z));
        // An enemy's pad is no place to land.
        game.units[pad].owner = 1;
        assert_ne!(game.cursor_look(), CursorLook::Land);
    }

    #[test]
    fn a_selected_carryall_shows_the_carry_cursor_over_a_vehicle_and_where_it_can_put_one_down() {
        let mut game = in_battle();
        let carryall = game
            .units
            .iter()
            .position(|u| u.role == Role::AdvCarryall && u.owner == 0);
        let carryall = carryall.expect("the player starts with some");
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
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
        assert_eq!(game.cursor_look(), CursorLook::Move, "nothing in hand");
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
            .find(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap();
        let (tank_id, at) = (tank.id, (tank.x, tank.z));
        let carryall = game
            .units
            .iter()
            .position(|u| u.role == Role::AdvCarryall && u.owner == 0);
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
    fn without_its_building_a_button_is_not_offered_and_makes_nothing() {
        let mut game = in_battle();
        game.units.retain(|u| u.role != Role::Barracks);
        let before = game.units.len();
        assert!(game.menu(BuildTab::Infantry).is_empty(), "nothing to offer");
        click_hud(&mut game, Action::BuildTab(BuildTab::Infantry));
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(game.units.len(), before);
        let b = hud::grid_cells(W, H)[0];
        game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
        assert_eq!(game.tip(), None);
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
        assert!(!game.can_produce(Role::HkAssault));
        let theirs = game
            .units
            .iter_mut()
            .find(|u| u.role == Role::Factory)
            .unwrap();
        theirs.owner = 0;
        assert!(game.can_produce(Role::HkAssault));
    }

    #[test]
    fn machines_off_the_screen_are_left_out_or_only_cast_their_shadow_and_far_ones_are_simpler() {
        let mut game = in_battle();
        game.fog = None;
        let terrain = crate::battlefield::terrain(game.battle_map);
        let view = || crate::battlefield::View::new(game.camera, game.width, game.height);
        let (cx, cz) = view()
            .ground_at(game.width / 2.0, game.height / 2.0)
            .unwrap();
        let side = (0, game.stats_rows()[0].color);
        let tank_at = |x: f32, z: f32| {
            let mut tank = crate::units::new_building(terrain, (x, z), Role::HkAssault, side, 9999);
            tank.y = 0.0;
            tank
        };
        game.units.clear();
        let drawn = |game: &mut Game, tank: Unit| {
            game.units = vec![tank];
            let frame = crate::render::build_frame(game);
            (frame.objects.len(), frame.casters.len())
        };
        game.camera.zoom = 1.25;
        let (near, _) = drawn(&mut game, tank_at(cx, cz));
        game.camera.zoom = 0.5;
        // As far up a small screen as can be, it shows small enough to be simpler.
        let full = (game.width, game.height);
        (game.width, game.height) = (480.0, 300.0);
        let (fx, fz) = crate::battlefield::View::new(game.camera, game.width, game.height)
            .ground_at(game.width / 2.0, 1.0)
            .unwrap();
        let (far, _) = drawn(&mut game, tank_at(fx, fz));
        (game.width, game.height) = full;
        assert!(far > 0 && far < near, "{far} {near}");
        let (cx, cz) = crate::battlefield::View::new(game.camera, game.width, game.height)
            .ground_at(game.width / 2.0, game.height / 2.0)
            .unwrap();
        assert_eq!(drawn(&mut game, tank_at(cx + 22630.0, cz)), (0, 0));
        // Just past the screen's west edge, the sun throws its shadow east, back onto it.
        let (objects, casters) = (0..2000)
            .map(|step| {
                let (x, z) = beside((cx, cz), -(step as f32) * 2.263);
                drawn(&mut game, tank_at(x, z))
            })
            .find(|&(objects, _)| objects == 0)
            .unwrap();
        assert_eq!(objects, 0);
        assert!(casters > 0);
    }

    #[test]
    fn striders_draw_at_rest_on_the_move_and_with_their_cannons_recoiling() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_role(&mut game, BuildTab::Vehicles, Role::AtMongoose);
        click_role(&mut game, BuildTab::Vehicles, Role::AtMinotaurus);
        assert!(!crate::render::build(&game).is_empty());
        let strider = game.units.last_mut().unwrap();
        assert_eq!(strider.role, Role::AtMinotaurus);
        strider.recoil = [1.0, 0.5, 0.0, 0.0];
        strider.stride = 58.83;
        strider.destination.0 += 226.3;
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
    fn whatever_is_destroyed_goes_up_in_an_explosion_with_particles_that_draw() {
        let mut game = in_battle();
        let before = crate::render::build(&game).len();
        let on_ground = game
            .units
            .iter()
            .position(|u| u.altitude <= 0.0 && u.kind != UnitKind::Infantry)
            .unwrap();
        game.units[on_ground].alive = false;
        game.update(0.01);
        assert!(!game.combat.particles.is_empty());
        assert!(crate::render::build(&game).len() > before);
    }

    #[test]
    fn opponents_keep_every_building_busy_and_send_what_comes_out_to_their_rally_point() {
        let mut game = in_battle();
        let owners = 1..game.teams.len();
        let makers = |game: &Game, owner: usize| {
            game.units
                .iter()
                .filter(|u| u.owner == owner && u.role.makes_units())
                .collect::<Vec<_>>()
                .len()
        };
        let of = |game: &Game, owner: usize| game.units.iter().filter(|u| u.owner == owner).count();
        let before: Vec<usize> = owners.clone().map(|owner| of(&game, owner)).collect();
        game.update(0.01);
        for owner in owners.clone() {
            assert!(makers(&game, owner) > 0);
            assert!(
                game.units
                    .iter()
                    .filter(|u| u.owner == owner && u.role.makes_units())
                    .all(|u| u.making.is_some()),
                "owner {owner} starts on everything at once"
            );
        }
        let known = game.units.len();
        assert_eq!(
            known,
            before.iter().sum::<usize>() + of(&game, 0),
            "it takes time"
        );
        game.finish_all_making();
        for (owner, before) in owners.clone().zip(before) {
            assert_eq!(
                of(&game, owner),
                before + makers(&game, owner),
                "owner {owner}"
            );
        }
        // As soon as they are done, they are busy again.
        game.update(0.01);
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner != 0 && u.role.makes_units())
                .all(|u| u.making.is_some())
        );
        let new = &game.units[known..];
        assert!(
            new.iter()
                .all(|u| u.owner != 0 && u.is_moving() && !u.selected)
        );
        for unit in new {
            let rally = game.strategies[unit.owner - 1]
                .rally(&game.units, &game.teams)
                .unwrap();
            let off = (unit.destination.0 - rally.0).hypot(unit.destination.1 - rally.1);
            assert!(off < 271.5, "{unit:?} goes to {rally:?}");
        }
        assert!(
            !game.chat.iter().any(|line| line.text.ends_with("ready")),
            "only yours"
        );
    }

    #[test]
    fn opponents_gather_their_army_then_send_it_all_off_together_against_an_enemy_base() {
        let mut game = in_battle();
        assert_eq!(game.strategies.len(), game.teams.len() - 1);
        let rally = game.strategies[0].rally(&game.units, &game.teams).unwrap();
        let soldiers = |game: &Game| -> Vec<Unit> {
            game.units
                .iter()
                .filter(|u| u.alive && u.owner == 1 && !u.kind.is_building())
                .copied()
                .collect()
        };
        // A competent opponent waits a minute and a half, gathering what it makes.
        while game.battle_time < 85.0 {
            game.update(0.25);
        }
        assert!(game.strategies[0].attackers.is_empty(), "still waiting");
        let gathered = soldiers(&game)
            .iter()
            .filter(|u| (u.x - rally.0).hypot(u.z - rally.1) < 407.3)
            .count();
        assert!(
            gathered * 3 >= soldiers(&game).len() * 2,
            "most stand at the rally"
        );
        assert!(gathered >= 8, "enough for an attack");

        while game.battle_time < 100.0 {
            game.update(0.25);
        }
        let plan = &game.strategies[0];
        assert!(plan.attackers.len() >= 8, "they set off together");
        assert!(plan.wave_size > 8, "and the next attack waits for more");
        let enemy = |id: u32| {
            game.units
                .iter()
                .find(|u| u.id == id)
                .is_some_and(|u| units::are_enemies(&game.teams, 1, u.owner))
        };
        let attacking = soldiers(&game)
            .iter()
            .filter(|u| plan.attackers.contains(&u.id))
            .filter(|u| matches!(u.target, Some(units::Target::Unit(id)) if enemy(id)))
            .count();
        assert!(attacking * 2 >= plan.attackers.len(), "after the enemy");
    }

    #[test]
    fn opponents_stop_making_more_at_a_limit_and_start_again_when_they_lose_some() {
        let mut game = in_battle();
        let terrain = battlefield::terrain(game.battle_map);
        let side = (1, TeamColor::Red);
        let standing = game.units.iter().filter(|u| u.owner == 1).count();
        for i in standing..OPPONENT_UNITS_AT_MOST {
            let at = (9051.0 + i as f32, 1810.0);
            let extra = units::new_building(terrain, at, Role::HkAssault, side, 5000 + i as u32);
            game.units.push(extra);
        }
        game.units
            .iter_mut()
            .filter(|u| u.owner == 1)
            .for_each(|u| u.making = None);
        let idle = |game: &Game| {
            game.units
                .iter()
                .filter(|u| u.owner == 1 && u.role.makes_units())
                .all(|u| u.making.is_none())
        };
        game.run_opponents();
        assert!(idle(&game), "at the limit");
        let tank = game.units.iter().position(|u| u.id == 5079).unwrap();
        game.units[tank].alive = false;
        game.run_opponents();
        assert!(!idle(&game), "under it again");
    }

    #[test]
    fn opponents_without_their_buildings_build_nothing_but_with_one_left_only_its_kind() {
        let mut game = in_battle();
        let enemy = |u: &Unit| u.owner != 0;
        game.units
            .retain(|u| !(enemy(u) && u.kind == UnitKind::Building));
        let known = game.units.len();
        for _ in 0..4 {
            game.run_opponents();
            game.finish_all_making();
        }
        assert_eq!(game.units.len(), known);

        let mut game = in_battle();
        game.units
            .retain(|u| u.owner == 0 || u.role != Role::Factory && u.role != Role::Hanger);
        let known = game.units.len();
        for _ in 0..4 {
            game.run_opponents();
            game.finish_all_making();
        }
        assert!(game.units.len() > known);
        assert!(
            game.units[known..]
                .iter()
                .all(|u| u.kind == UnitKind::Infantry)
        );
    }

    #[test]
    fn a_click_starts_a_unit_that_appears_when_done_and_the_button_shows_how_far_it_is() {
        let mut game = in_battle();
        game.units.retain(|u| u.owner == 0 || !u.role.makes_units());
        let cell = hud::grid_cells(W, H)[1];
        let known = game.units.len();
        assert_eq!(game.making_progress(Role::AtInfantry), None);
        click_at(&mut game, cell.x + cell.w / 2.0, cell.y + cell.h / 2.0);
        assert_eq!(game.units.len(), known, "not at once");
        assert_eq!(game.making_progress(Role::AtInfantry), Some(0.0));
        let dim = crate::render::MAKING_DIM;
        assert!(line_vertices(&game, dim) > 0, "the button goes dim");

        let seconds = Role::AtInfantry.build_seconds();
        game.update(seconds / 2.0);
        let progress = game.making_progress(Role::AtInfantry).unwrap();
        assert!((progress - 0.5).abs() < 0.01, "{progress}");
        assert_eq!(game.units.len(), known);
        let half_dim = line_vertices(&game, dim);

        game.update(seconds / 2.0 + 0.01);
        assert_eq!(game.making_progress(Role::AtInfantry), None);
        assert_eq!(game.units.len(), known + 1);
        assert_eq!(game.units[known].name(), "Infantry");
        assert!(game.chat.iter().any(|l| l.text == "Infantry ready"));
        assert!(half_dim > 0);
        assert_eq!(line_vertices(&game, dim), 0, "the button is back");
    }

    #[test]
    fn repeat_clicks_count_on_the_button_and_units_come_out_in_the_order_clicked() {
        let mut game = in_battle();
        let cells = hud::grid_cells(W, H);
        let press = |game: &mut Game, cell: usize| {
            let b = cells[cell];
            click_at(game, b.x + b.w / 2.0, b.y + b.h / 2.0);
        };
        let (scout, light) = (Role::Scout, Role::AtInfantry);
        let known = game.units.len();

        press(&mut game, 1);
        assert_eq!(game.queue_badge(light), None, "the dimming shows this one");
        assert!(game.making_progress(light).is_some());
        press(&mut game, 1);
        assert_eq!(game.queue_badge(light), Some(2));
        press(&mut game, 1);
        assert_eq!(game.queue_badge(light), Some(3));

        press(&mut game, 0);
        assert_eq!(game.queue_badge(scout), Some(1), "waiting, so it shows a 1");
        assert_eq!(game.making_progress(scout), None, "and it is not dim");
        assert_eq!(game.queue_badge(light), Some(3));
        press(&mut game, 1);
        assert_eq!(game.queue_badge(light), Some(4));

        let with_badges = crate::render::build(&game).len();
        game.units.iter_mut().for_each(|u| u.making = None);
        assert!(with_badges > crate::render::build(&game).len(), "they draw");
        let mut game = in_battle();
        for cell in [1, 1, 0, 1] {
            press(&mut game, cell);
        }

        let mut made = Vec::new();
        for _ in 0..200 {
            game.update(0.5);
            made = game.units[known..]
                .iter()
                .filter(|u| u.owner == 0)
                .map(|u| u.role)
                .collect();
            if made.len() == 4 {
                break;
            }
        }
        assert_eq!(made, [light, light, scout, light]);
        assert_eq!(game.queue_badge(light), None);
        assert_eq!(game.queue_badge(scout), None);
    }

    #[test]
    fn a_right_click_on_a_button_takes_one_of_that_type_off_and_keeps_the_selection() {
        let mut game = in_battle();
        select_first_group(&mut game);
        let cell = hud::grid_cells(W, H)[1];
        let at = (cell.x + cell.w / 2.0, cell.y + cell.h / 2.0);
        for _ in 0..3 {
            click_at(&mut game, at.0, at.1);
        }
        let other = hud::grid_cells(W, H)[0];
        click_at(&mut game, other.x + other.w / 2.0, other.y + other.h / 2.0);
        let role = Role::AtInfantry;
        assert_eq!(game.queue_badge(role), Some(3));

        game.handle(Event::MouseMove(at.0, at.1));
        game.handle(Event::RightDown);
        assert_eq!(game.queue_badge(role), Some(2), "one less");
        assert_eq!(
            game.queue_badge(Role::Scout),
            Some(1),
            "the other type is untouched"
        );
        assert_eq!(selected_count(&game), FIRST_GROUP, "the selection stands");

        game.handle(Event::RightDown);
        assert_eq!(game.queue_badge(role), None);
        assert!(
            game.making_progress(role).is_some(),
            "the one being made goes last"
        );
        game.handle(Event::RightDown);
        assert_eq!(game.making_progress(role), None, "and then that goes");
        assert!(
            game.making_progress(Role::Scout).is_some(),
            "so the next starts"
        );
        game.handle(Event::RightDown);
        assert_eq!(
            game.queue_badge(Role::Scout),
            None,
            "nothing more of this type"
        );

        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game.handle(Event::RightDown);
        assert_eq!(selected_count(&game), 0, "elsewhere it still deselects");
    }

    #[test]
    fn the_factory_and_the_hangar_keep_queues_of_their_own() {
        let mut game = in_battle();
        let cells = hud::grid_cells(W, H);
        let press = |game: &mut Game, cell: usize| {
            let b = cells[cell];
            click_at(game, b.x + b.w / 2.0, b.y + b.h / 2.0);
        };
        press(&mut game, 0);
        press(&mut game, 0);
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        press(&mut game, 0);
        let cell = cell_of(&game, BuildTab::Vehicles, Role::AtAdp);
        press(&mut game, cell);
        assert!(game.making_progress(Role::Scout).is_some());
        assert!(
            game.making_progress(Role::AtTrike).is_some(),
            "not behind the scouts"
        );
        assert!(game.making_progress(Role::AtAdp).is_some());
        let cell = cell_of(&game, BuildTab::Vehicles, Role::AtRepairUnit);
        press(&mut game, cell);
        assert_eq!(game.queue_badge(Role::AtRepairUnit), Some(1));
        assert_eq!(game.queue_badge(Role::Scout), Some(2));
    }

    #[test]
    fn the_dim_part_shrinks_clockwise_from_twelve_oclock() {
        use crate::render::making_dim_wedges;
        let square = (0.0, 0.0, 100.0, 100.0);
        // At the start it covers everything.
        let full = making_dim_wedges(square, 0.0);
        assert!(
            full.iter()
                .any(|w| w.iter().any(|p| p.0 < 1.0 && p.1 > 50.0))
        );
        // A quarter through, the upper right is clear and the rest is not.
        let wedges = making_dim_wedges(square, 0.25);
        let centers = |wedge: &[(f32, f32); 3]| {
            (
                (wedge[0].0 + wedge[1].0 + wedge[2].0) / 3.0,
                (wedge[0].1 + wedge[1].1 + wedge[2].1) / 3.0,
            )
        };
        assert!(
            wedges
                .iter()
                .map(centers)
                .all(|c| !(c.0 > 55.0 && c.1 < 45.0))
        );
        assert!(wedges.iter().map(centers).any(|c| c.0 < 45.0 && c.1 < 45.0));
        assert!(wedges.iter().map(centers).any(|c| c.0 > 55.0 && c.1 > 55.0));
        assert!(making_dim_wedges(square, 1.0).is_empty());
    }

    #[test]
    fn the_buildings_menu_builds_with_the_clock_then_a_click_places_it_on_the_map() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        let cells = hud::grid_cells(W, H);
        let press = |game: &mut Game, cell: usize| {
            let b = cells[cell];
            click_at(game, b.x + b.w / 2.0, b.y + b.h / 2.0);
        };
        let ours = |game: &Game| game.units.iter().filter(|u| u.owner == 0).count();
        let known = ours(&game);
        let (barracks, factory) = (
            cell_of(&game, BuildTab::Buildings, Role::Barracks),
            cell_of(&game, BuildTab::Buildings, Role::Factory),
        );

        press(&mut game, factory);
        assert_eq!(game.making_progress(Role::Factory), Some(0.0));
        assert!(
            line_vertices(&game, crate::render::MAKING_DIM) > 0,
            "it dims"
        );
        press(&mut game, barracks);
        assert_eq!(game.making_progress(Role::Barracks), None, "one at a time");
        press(&mut game, factory);
        assert_eq!(
            game.queue_badge(Role::Factory),
            None,
            "buildings have no queue"
        );

        game.update(Role::Factory.build_seconds() + 0.1);
        assert_eq!(game.site, Site::Ready(Role::Factory));
        assert!(game.is_ready_to_place(Role::Factory));
        assert!(game.chat.iter().any(|l| l.text == "Factory ready to place"));
        assert_eq!(ours(&game), known, "not on the map yet");
        assert_eq!(game.making_progress(Role::Factory), None);

        press(&mut game, factory);
        assert_eq!(game.site, Site::Placing(Role::Factory));

        // On top of the first group it cannot go.
        let unit = game
            .units
            .iter()
            .find(|u| u.role == Role::HkAssault)
            .copied()
            .unwrap();
        let (x, y) = game.view().project(unit.x, unit.y, unit.z).unwrap();
        game.handle(Event::MouseMove(x, y));
        assert!(!game.placing_preview().unwrap().1);
        assert!(!crate::render::build(&game).is_empty());
        click_at(&mut game, x, y);
        assert_eq!(ours(&game), known);
        assert_eq!(
            game.site,
            Site::Placing(Role::Factory),
            "it waits for a good click"
        );

        // A right click puts it down again, ready.
        game.handle(Event::RightDown);
        assert_eq!(game.site, Site::Ready(Role::Factory));
        press(&mut game, factory);
        assert_eq!(
            game.cursor_look(),
            CursorLook::Arrow,
            "no orders while placing"
        );
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.site, Site::Ready(Role::Factory), "Escape as well");
        assert_eq!(game.screen, Screen::InGame);
        press(&mut game, factory);

        let (x, y) = buildable_spot_on_screen(&mut game);
        let known = ours(&game);
        let terrain = battlefield::terrain(game.battle_map);
        let spot = game.view().land_at(terrain, x, y).unwrap();
        game.handle(Event::MouseMove(x, y));
        assert!(game.placing_preview().unwrap().1, "{spot:?}");
        assert!(!crate::render::build(&game).is_empty());
        click_at(&mut game, x, y);
        assert_eq!(game.site, Site::Idle);
        assert_eq!(ours(&game), known + 1);
        let built = *game.units.last().unwrap();
        assert_eq!((built.role, built.owner), (Role::Factory, 0));
        assert_eq!(
            (built.x, built.z),
            Role::Factory.plot_at(game.faction_of(0), spot).center()
        );
        assert_eq!(built.color, game.units[0].color);
    }

    #[test]
    fn an_invisible_unit_shimmers_for_its_own_side_and_does_not_show_for_its_enemies() {
        let mut game = in_battle();
        let scout = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .unwrap();
        let cloaked = |game: &Game| {
            crate::render::build_frame(game)
                .objects
                .iter()
                .filter(|v| v.stain == crate::render::CLOAKED)
                .count()
        };
        assert_eq!(cloaked(&game), 0);
        game.units[scout].role = Role::Scout;
        assert!(cloaked(&game) > 0, "its own side sees it shimmer");
        game.units[scout].owner = 1;
        assert!(!game.shows(&game.units[scout]), "its enemies not at all");
        assert_eq!(cloaked(&game), 0);
    }

    #[test]
    fn a_battle_starts_with_one_packed_up_mcv_each_and_opponents_grow_a_base_from_theirs() {
        let mut game = in_setup();
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        let owners = game.teams.len();
        assert_eq!(game.units.len(), owners, "nothing but MCVs");
        for (owner, unit) in game.units.iter().enumerate() {
            assert_eq!((unit.owner, unit.role), (owner, Role::Mcv));
            assert!(!unit.deployed && unit.kind == UnitKind::Vehicle);
        }
        let mcv = game.units[0];
        assert!(
            (game.camera.x - mcv.x).hypot(game.camera.z - mcv.z) < 135.8,
            "in view"
        );
        for tab in [BuildTab::Infantry, BuildTab::Vehicles, BuildTab::Buildings] {
            assert!(!game.tab_enabled(tab), "nothing to make {tab:?} yet");
        }

        // A windtrap first, another for the greater tower, and the yard's upgrade it needs.
        let mut made_soldiers = vec![false; owners];
        while game.battle_time < 150.0 {
            game.update(0.25);
            for soldier in game.units.iter().filter(|u| u.kind == UnitKind::Infantry) {
                made_soldiers[soldier.owner] = true;
            }
        }
        assert!(!game.units[0].deployed, "the player deploys their own");
        let has = |game: &Game, owner: usize, role: Role| {
            game.units
                .iter()
                .filter(|u| u.alive && u.owner == owner && u.role == role)
                .count()
        };
        for owner in 1..owners {
            let mcv = game
                .units
                .iter()
                .find(|u| u.owner == owner && u.role == Role::Mcv);
            assert!(mcv.is_some_and(|u| u.deployed), "owner {owner} deploys");
            let [lesser, greater] = Role::towers_of(mcv.unwrap().house);
            assert!(
                has(&game, owner, Role::Windtrap) > 0,
                "owner {owner} powers it"
            );
            assert!(
                !units::is_short_of_power(&game.units, owner),
                "owner {owner} is not short of power"
            );
            for role in [Role::Barracks, Role::Factory, lesser, greater] {
                assert_eq!(
                    has(&game, owner, role),
                    1,
                    "owner {owner} builds a {role:?}"
                );
            }
            assert!(made_soldiers[owner], "owner {owner} makes an army");
        }

        // What it loses, it builds again, with the money for it.
        game.units
            .retain(|u| u.owner != 1 || u.role != Role::Barracks);
        game.money[1] += 4000;
        // After what it already has on its way.
        let back = game.battle_time + 80.0;
        while game.battle_time < back {
            game.update(0.25);
        }
        assert_eq!(has(&game, 1, Role::Barracks), 1);
    }

    /// A spot on the sand with rock close enough to drive to.
    fn sand_by_upland(terrain: &battlefield::Terrain) -> (f32, f32) {
        // Clear of the map's edge, which nobody gets right up to.
        (8..256)
            .flat_map(|i| (8..256).map(move |j| (i as f32 * 32.0 + 16.0, j as f32 * 32.0 + 16.0)))
            .find(|&(x, z)| {
                terrain.can_walk(x, z)
                    && !terrain.is_rock(x, z)
                    && units::deploy_spot_near(&[], terrain, Faction::Atreides, (x, z)).is_some()
            })
            .unwrap()
    }

    #[test]
    fn an_mcv_cannot_deploy_on_the_sand_and_an_opponents_drives_off_it_first() {
        let mut game = in_setup();
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        let terrain = battlefield::terrain(game.battle_map);
        let sand = sand_by_upland(terrain);
        for mcv in &mut game.units {
            (mcv.x, mcv.z) = sand;
            mcv.destination = sand;
        }
        game.units.truncate(2);
        // Theirs by the rock of the opposite corner, where nothing of the player's gets in its way.
        let (right, bottom) = terrain.extent();
        let theirs = (right - sand.0, bottom - sand.1);
        (game.units[1].x, game.units[1].z) = theirs;
        game.units[1].destination = theirs;
        game.units[0].selected = true;
        let hover = |game: &mut Game| {
            let mcv = game.units[0];
            game.camera.look_at(mcv.x, mcv.z);
            let (px, py) = game
                .view()
                .project(mcv.x, mcv.pick_height(), mcv.z)
                .unwrap();
            game.handle(Event::MouseMove(px, py));
            game.cursor_look()
        };
        assert_ne!(
            hover(&mut game),
            CursorLook::Deploy,
            "no deploy cursor on the sand"
        );
        game.handle(Event::KeyDown(Key::Char('d')));
        assert!(!game.units[0].deployed, "not on the sand");
        assert_eq!(chat_texts(&game), [CANNOT_DEPLOY]);
        let upland =
            units::deploy_spot_near(&game.units, terrain, game.units[0].house, sand).unwrap();
        (game.units[0].x, game.units[0].z) = upland;
        game.units[0].destination = upland;
        assert_eq!(
            hover(&mut game),
            CursorLook::Deploy,
            "up there it offers one"
        );
        game.handle(Event::KeyDown(Key::Char('d')));
        for _ in 0..60 {
            game.update(0.05);
        }
        assert!(game.units[0].deployed, "up there it can");

        let their_mcv = game.units[1].id;
        for _ in 0..3000 {
            game.update(0.05);
            if game.units.iter().any(|u| u.id == their_mcv && u.deployed) {
                break;
            }
        }
        let theirs = *game.units.iter().find(|u| u.id == their_mcv).unwrap();
        assert!(theirs.deployed, "{theirs:?}");
        assert!(terrain.is_rock(theirs.x, theirs.z));
    }

    #[test]
    fn a_deployed_mcv_packs_up_into_a_vehicle_and_deploys_again() {
        let mut game = in_battle();
        for owner in 0..game.teams.len() {
            let mcvs: Vec<&Unit> = game
                .units
                .iter()
                .filter(|u| u.owner == owner && u.role == Role::Mcv)
                .collect();
            assert_eq!(mcvs.len(), 1, "owner {owner}");
            assert!(mcvs[0].deployed && mcvs[0].kind == UnitKind::Building);
        }
        let mcv = mcv_of(&game);
        let at = (game.units[mcv].x, game.units[mcv].z);
        units::switch_deployed(&mut game.units[mcv]);
        assert_eq!(game.units[mcv].kind, UnitKind::Vehicle);
        assert!(
            game.units[mcv].radius() > UnitKind::Vehicle.radius(),
            "huge"
        );
        game.units[mcv].destination = (at.0, at.1 + 271.5);
        for _ in 0..100 {
            game.update(0.05);
        }
        let moved = (game.units[mcv].x - at.0).hypot(game.units[mcv].z - at.1);
        assert!(moved > 135.8, "it drives: {moved}");
        units::switch_deployed(&mut game.units[mcv]);
        assert_eq!(game.units[mcv].kind, UnitKind::Building);
        assert!(!game.units[mcv].is_moving(), "and stays put");
        assert!(!crate::render::build(&game).is_empty());
    }

    #[test]
    fn buildings_need_a_deployed_mcv_and_each_tab_something_that_makes_what_is_on_it() {
        let mut game = in_battle();
        for tab in [BuildTab::Infantry, BuildTab::Vehicles, BuildTab::Buildings] {
            assert!(game.tab_enabled(tab), "{tab:?}");
        }
        let mcv = mcv_of(&game);
        units::switch_deployed(&mut game.units[mcv]);
        assert!(!game.tab_enabled(BuildTab::Buildings), "packed up");
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        assert_eq!(
            game.build_tab,
            BuildTab::Infantry,
            "a greyed tab does not open"
        );
        game.build_tab = BuildTab::Buildings;
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(game.site, Site::Idle, "nothing to build it");

        units::switch_deployed(&mut game.units[mcv]);
        let barracks = cell_of(&game, BuildTab::Buildings, Role::Barracks);
        click_hud(&mut game, Action::BuildCell(barracks));
        assert!(matches!(game.site, Site::Building(Role::Barracks, _)));
        units::switch_deployed(&mut game.units[mcv]);
        let before = game.site;
        game.update(1.0);
        assert_eq!(game.site, before, "its clock stops meanwhile");

        let gone = |game: &mut Game, roles: &[Role]| {
            game.units
                .retain(|u| u.owner != 0 || !roles.contains(&u.role));
        };
        gone(&mut game, &[Role::Factory]);
        assert!(
            game.tab_enabled(BuildTab::Vehicles),
            "the hangar still makes aircraft"
        );
        gone(&mut game, &[Role::Hanger]);
        assert!(!game.tab_enabled(BuildTab::Vehicles));
        assert!(game.tab_enabled(BuildTab::Infantry));
        gone(&mut game, &[Role::Barracks]);
        assert!(!game.tab_enabled(BuildTab::Infantry));
    }

    #[test]
    fn a_factory_makes_a_spare_mcv() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        click_role(&mut game, BuildTab::Vehicles, Role::Mcv);
        game.finish_all_making();
        let made = *game.units.last().unwrap();
        assert_eq!((made.role, made.owner), (Role::Mcv, 0));
        assert_eq!(made.kind, UnitKind::Vehicle);
        assert!(!made.deployed, "it comes out on its tracks");
    }

    fn mcv_of(game: &Game) -> usize {
        game.units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Mcv)
            .unwrap()
    }

    #[test]
    fn each_house_builds_its_own_two_towers_of_two_by_two() {
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let game = in_battle_as(house);
            let towers = Role::towers_of(house);
            for role in towers {
                assert!(offers(&game, BuildTab::Buildings, role));
                assert!(role.made_by(house));
                assert_eq!(role.kind(), UnitKind::Tower);
                assert_eq!(role.plot_size(house), (2, 2));
                assert!(role.build_seconds() > 0.0, "{role:?}");
            }
            for other in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
                let foreign = Role::towers_of(other).iter().any(|r| r.made_by(house));
                assert_eq!(foreign, other == house);
            }
            assert!(offers(&game, BuildTab::Buildings, Role::Palace(house)));
        }
    }

    #[test]
    fn an_allied_mini_houses_main_building_is_offered_once_allied_needing_what_rules_say() {
        let mut game = in_battle();
        let own = game.menu(BuildTab::Buildings);
        game.skirmish.allies = [Ally::Fremen, Ally::Guild];
        let mut allied = own.clone();
        allied.extend([Role::FrCamp, Role::GuPalace].map(Entry::Make));
        assert_eq!(
            game.menu(BuildTab::Buildings),
            allied,
            "after the house's own"
        );
        for (ally, role, needs) in [
            (Ally::Tleilaxu, Role::TlFleshVat, Role::Factory),
            (Ally::Ix, Role::IxResCentre, Role::Factory),
            (Ally::Guild, Role::GuPalace, Role::Factory),
            (Ally::Sardaukar, Role::ImBarracks, Role::Barracks),
            (Ally::Fremen, Role::FrCamp, Role::Barracks),
        ] {
            assert_eq!(ally.main_building(), Some(role));
            assert_eq!(role.kind(), UnitKind::Building);
            assert_eq!(role.needs(), Some(needs));
            assert!(game.can_produce(role), "{role:?}");
            let kept = game.units.clone();
            game.units.retain(|u| u.owner != 0 || u.role != needs);
            assert!(!game.can_produce(role), "{role:?} without a {needs:?}");
            game.units = kept;
        }
        assert_eq!(
            [Role::FrCamp, Role::GuPalace].map(|r| r.rules_name(Faction::Atreides)),
            ["FRCamp", "GUPalace"]
        );
        assert_eq!(Ally::None.main_building(), None);
    }

    #[test]
    fn an_allied_mini_houses_unit_is_offered_once_allied_and_made_in_its_main_building() {
        for (ally, role, building) in [
            (Ally::Tleilaxu, Role::TlContaminator, Role::TlFleshVat),
            (Ally::Ix, Role::IxInfiltrator, Role::IxResCentre),
            (Ally::Guild, Role::GuMaker, Role::GuPalace),
            (Ally::Sardaukar, Role::ImSardaukar, Role::ImBarracks),
            (Ally::Fremen, Role::FrFremen, Role::FrCamp),
        ] {
            let mut game = in_battle();
            let own = game.menu(BuildTab::Infantry).len();
            game.skirmish.allies = [Ally::None, ally];
            assert!(
                !offers(&game, BuildTab::Infantry, role),
                "not without its building"
            );
            assert_eq!(ally.unit(), Some(role));
            assert_eq!(role.made_in(), Some(building));
            assert!(!game.can_produce(role), "{role:?} needs its {building:?}");
            // Its main building, up and standing beside the player's barracks.
            let mut made_in = *game
                .units
                .iter()
                .find(|u| u.owner == 0 && u.role == Role::Barracks)
                .unwrap();
            made_in.role = building;
            made_in.id = game.next_unit_id;
            game.next_unit_id += 1;
            game.units.push(made_in);
            assert!(game.can_produce(role), "{role:?}");
            let before = game.units.len();
            assert_eq!(
                cell_of(&game, BuildTab::Infantry, role),
                own,
                "after the house's own"
            );
            click_hud(&mut game, Action::BuildTab(BuildTab::Infantry));
            click_hud(&mut game, Action::BuildCell(own));
            game.update(role.build_seconds() + 0.5);
            let made = game.units[before..].iter().find(|u| u.role == role);
            let made = made.unwrap_or_else(|| panic!("{role:?} is made"));
            assert_eq!((made.owner, made.kind), (0, role.kind()));
        }
    }

    #[test]
    fn each_mini_houses_main_building_is_built_and_placed_like_other_buildings() {
        for ally in [
            Ally::Tleilaxu,
            Ally::Ix,
            Ally::Guild,
            Ally::Sardaukar,
            Ally::Fremen,
        ] {
            let mut game = in_battle();
            game.skirmish.allies = [ally, Ally::None];
            let role = ally.main_building().unwrap();
            let cell = cell_of(&game, BuildTab::Buildings, role);
            click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
            let money = game.player_money();
            click_hud(&mut game, Action::BuildCell(cell));
            game.update(role.build_seconds() + 0.1);
            assert_eq!(game.site, Site::Ready(role), "{ally:?}");
            assert_eq!(game.player_money(), money - role.cost(), "{ally:?}");
            click_hud(&mut game, Action::BuildCell(cell));
            let (x, y) = buildable_spot_on_screen(&mut game);
            game.handle(Event::MouseMove(x, y));
            click_at(&mut game, x, y);
            let built = *game.units.last().unwrap();
            assert_eq!((built.role, built.owner), (role, 0), "{ally:?}");
            assert_eq!(built.kind, UnitKind::Building);
        }
    }

    #[test]
    fn only_one_building_goes_up_at_a_time_and_the_others_are_greyed_out() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        let (first, second) = (Role::Barracks, Role::Factory);
        assert!(!game.is_held_up_by_site(second), "nothing on its way");
        let cell = cell_of(&game, BuildTab::Buildings, first);
        click_hud(&mut game, Action::BuildCell(cell));
        assert_eq!(game.site, Site::Building(first, first.build_seconds()));
        assert!(game.is_held_up_by_site(second));
        assert!(!game.is_held_up_by_site(first), "it shows its own progress");
        assert!(
            !game.is_held_up_by_site(Role::AtInfantry),
            "a unit waits for no site"
        );
        let other = cell_of(&game, BuildTab::Buildings, second);
        click_hud(&mut game, Action::BuildCell(other));
        assert_eq!(
            game.site,
            Site::Building(first, first.build_seconds()),
            "no queue"
        );
        game.update(first.build_seconds() + 0.1);
        assert_eq!(game.site, Site::Ready(first));
        assert!(game.is_held_up_by_site(second), "until it is placed");
        let other = cell_of(&game, BuildTab::Buildings, second);
        click_hud(&mut game, Action::BuildCell(other));
        assert_eq!(game.site, Site::Ready(first));
    }

    #[test]
    fn rocket_and_gun_turrets_are_built_and_placed_like_other_buildings() {
        for (cell, role, name) in [
            (6, Role::AtPillbox, "Machinegun Post"),
            (7, Role::AtRocketTurret, "Rocket Turret"),
        ] {
            let mut game = in_battle();
            click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
            assert_eq!(game.build_role(BuildTab::Buildings, cell), Some(role));
            click_hud(&mut game, Action::BuildCell(cell));
            game.update(role.build_seconds() + 0.1);
            assert_eq!(game.site, Site::Ready(role));
            click_hud(&mut game, Action::BuildCell(cell));
            let (x, y) = buildable_spot_on_screen(&mut game);
            game.handle(Event::MouseMove(x, y));
            click_at(&mut game, x, y);
            let built = *game.units.last().unwrap();
            assert_eq!((built.role, built.owner), (role, 0));
            assert_eq!(built.kind, UnitKind::Tower);
            assert_eq!(built.name(), name);
        }
    }

    #[test]
    fn a_lost_building_can_be_built_again_and_its_units_made_again() {
        let mut game = in_battle();
        game.units
            .retain(|u| u.role != Role::Barracks || u.owner != 0);
        assert!(!game.can_produce(Role::Scout));
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        let cell = cell_of(&game, BuildTab::Buildings, Role::Barracks);
        click_hud(&mut game, Action::BuildCell(cell));
        game.update(Role::Barracks.build_seconds() + 0.1);
        click_hud(&mut game, Action::BuildCell(cell));
        let (x, y) = buildable_spot_on_screen(&mut game);
        click_at(&mut game, x, y);
        assert!(!game.can_produce(Role::Scout), "still going up");
        game.update(units::CONSTRUCTION_SECONDS + 0.01);
        assert!(game.can_produce(Role::Scout));
    }

    #[test]
    fn a_right_click_on_a_buildings_button_calls_it_off_at_any_stage() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        let b = hud::grid_cells(W, H)[cell_of(&game, BuildTab::Buildings, Role::Hanger)];
        let at = (b.x + b.w / 2.0, b.y + b.h / 2.0);
        click_at(&mut game, at.0, at.1);
        assert!(matches!(game.site, Site::Building(Role::Hanger, _)));
        game.handle(Event::MouseMove(at.0, at.1));
        game.handle(Event::RightDown);
        assert_eq!(game.site, Site::Idle);

        game.site = Site::Ready(Role::Hanger);
        game.handle(Event::RightDown);
        assert_eq!(game.site, Site::Idle);
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
    fn the_deploy_button_and_the_d_key_switch_selected_kindjal_infantry() {
        let mut game = in_battle();
        let k = select_new(&mut game, 4);
        game.handle(Event::KeyDown(Key::Char('d')));
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
        with_bases(&mut game);
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
            .retain(|u| u.owner != 1 || u.role != Role::HkFlameTurret);
        let tower = tower_of(&game, 1);
        // Disarmed, tough enough to outlast the player's own fire, and moved into view.
        game.units[tower].cooldown = 1e9;
        game.units[tower].health = 1e9;
        let (x, z) = beside((game.camera.x, game.camera.z), 271.5);
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
        let (x, z) = beside((game.units[engineer].x, game.units[engineer].z), -226.3);
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
    fn each_house_makes_only_its_own_aircraft() {
        let all = [
            Role::AtOrni,
            Role::AtAdp,
            Role::HkGunship,
            Role::HkAdp,
            Role::OrEits,
            Role::OrAdp,
        ];
        for (house, own) in [
            (Faction::Atreides, [Role::AtOrni, Role::AtAdp]),
            (Faction::Harkonnen, [Role::HkGunship, Role::HkAdp]),
            (Faction::Ordos, [Role::OrEits, Role::OrAdp]),
        ] {
            let mut game = in_battle_as(house);
            click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
            for role in all {
                assert_eq!(
                    offers(&game, BuildTab::Vehicles, role),
                    own.contains(&role),
                    "{house:?} {role:?}"
                );
            }
            for role in own {
                let before = game.units.len();
                click_role(&mut game, BuildTab::Vehicles, role);
                assert_eq!(game.units[before].role, role, "{house:?}");
                assert_eq!(game.units[before].kind, UnitKind::Aircraft);
            }
            assert!(!crate::render::build(&game).is_empty());
        }
    }

    #[test]
    fn the_vehicles_menu_makes_both_carryalls_above_the_bottom_row() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        for (role, name) in [
            (Role::AdvCarryall, "Advanced Carryall"),
            (Role::Carryall, "Carryall"),
        ] {
            let before = game.units.len();
            click_role(&mut game, BuildTab::Vehicles, role);
            assert_eq!(game.units[before].name(), name);
            assert!(game.units[before].altitude > 0.0, "it flies");
        }
    }

    #[test]
    fn each_houses_barracks_makes_its_own_three_besides_scouts_and_engineers() {
        for (house, names) in [
            (
                Faction::Atreides,
                [
                    "Scout",
                    "Infantry",
                    "Sniper",
                    "Engineer",
                    "Kindjal Infantry",
                ],
            ),
            (
                Faction::Harkonnen,
                [
                    "Scout",
                    "Light Infantry",
                    "Trooper",
                    "Engineer",
                    "Flamethrower Infantry",
                ],
            ),
            (
                Faction::Ordos,
                [
                    "Scout",
                    "Chemical Trooper",
                    "Engineer",
                    "Mortar Infantry",
                    "Saboteur",
                ],
            ),
        ] {
            let mut game = in_battle_as(house);
            let all = names;
            for (cell, name) in all.into_iter().enumerate() {
                let before = game.units.len();
                click_hud(&mut game, Action::BuildCell(cell));
                assert_eq!(game.units[before].name(), name, "{house:?}");
                assert_eq!(game.units[before].house, house);
            }
            assert_eq!(game.build_role(BuildTab::Infantry, all.len()), None);
        }
    }

    #[test]
    fn fire_and_gas_draw_without_trouble() {
        let mut game = in_battle();
        let (x, z) = (game.camera.x, game.camera.z);
        for gas in [false, true] {
            game.combat.sprays.push(crate::weapons::Spray {
                from: (x, 45.25, z),
                heading: 0.5,
                reach: 203.6,
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
        give_palace(&mut game);
        let cell = |i: usize| {
            hud::all_buttons(W, H)
                .into_iter()
                .find(|b| b.action == Action::BuildCell(i))
                .unwrap()
        };
        for (i, name) in [
            "Scout",
            "Infantry",
            "Sniper",
            "Engineer",
            "Kindjal Infantry",
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
            Some("Sand Bike"),
            Some("Mongoose"),
            Some("APC"),
            Some("Repair Vehicle"),
            Some("Minotaurus"),
            Some("Ornithopter"),
            Some("Advanced Carryall"),
            Some("Hawk Strike"),
            Some("Air Drone"),
            Some("Harvester"),
            Some("MCV"),
            Some("Carryall"),
            None,
        ]
        .into_iter()
        .enumerate()
        {
            let b = cell(i);
            game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
            assert_eq!(game.tip(), name);
        }

        let tower = game.units[tower_of(&game, 0)];
        let (x, y) = game
            .view()
            .project(tower.x, tower.pick_height(), tower.z)
            .unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.tip(), Some("Rocket Turret"), "one's own");
        let (_, (x, y)) = enemy_beside_first_group(&mut game);
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.tip(), Some("Infantry"), "and enemies");
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
    fn the_a_key_selects_those_on_screen_then_the_whole_map() {
        let mut game = in_battle();
        let everyone = game
            .units
            .iter()
            .filter(|u| u.is_selectable() && !u.kind.is_building())
            .count();
        assert!(everyone > FIRST_GROUP, "including the group off screen");
        game.handle(Event::KeyDown(Key::Char('a')));
        let on_screen = selected_count(&game);
        assert!(on_screen > 0 && on_screen < everyone, "{on_screen}");

        // Part of them selected: still those on screen.
        let chosen = game.units.iter().position(|u| u.selected).unwrap();
        game.units[chosen].selected = false;
        game.handle(Event::KeyDown(Key::Char('a')));
        assert_eq!(selected_count(&game), on_screen);

        // All of them already: now the whole map.
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
            "only what can move"
        );

        game.handle(Event::RightDown);
        game.handle(Event::KeyDown(Key::Char('A')));
        assert_eq!(
            selected_count(&game),
            on_screen,
            "with Shift or Caps Lock too"
        );

        // A selected unit off screen is let go when only those on screen are taken.
        game.handle(Event::KeyDown(Key::Char('a')));
        assert_eq!(selected_count(&game), everyone);
        game.units[chosen].selected = false;
        game.handle(Event::KeyDown(Key::Char('a')));
        assert_eq!(selected_count(&game), on_screen);
    }

    #[test]
    fn with_none_of_the_players_units_on_screen_the_a_key_selects_the_whole_map() {
        let mut game = in_battle();
        let everyone = game
            .units
            .iter()
            .filter(|u| u.is_selectable() && !u.kind.is_building())
            .count();
        for unit in game.units.iter_mut().filter(|u| u.owner == 0) {
            unit.x += 22630.0;
        }
        game.handle(Event::KeyDown(Key::Char('a')));
        assert_eq!(selected_count(&game), everyone);
    }

    #[test]
    fn the_h_key_sends_the_selected_home_and_nobody_else() {
        let mut game = in_battle();
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap();
        let other = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::AtInfantry)
            .unwrap();
        let away = beside((game.units[tank].x, game.units[tank].z), 905.1);
        game.units[tank].x = away.0;
        game.units[tank].destination = away;
        let stays = game.units[other].destination;
        game.units[tank].selected = true;
        game.handle(Event::KeyDown(Key::Char('h')));
        let sent = game.units[tank].destination;
        assert!(
            (sent.0 - game.home().0).hypot(sent.1 - game.home().1) < 362.0,
            "{sent:?}"
        );
        assert_ne!(sent, away);
        assert_eq!(game.units[other].destination, stays, "not selected");

        game.units[tank].destination = away;
        game.handle(Event::KeyDown(Key::Char('H')));
        assert_ne!(
            game.units[tank].destination, away,
            "with Shift or Caps Lock too"
        );

        game.units[tank].destination = away;
        game.handle(Event::KeyDown(Key::Pause));
        game.handle(Event::KeyDown(Key::Char('h')));
        assert_eq!(game.units[tank].destination, away, "not while paused");
    }

    fn tank_of(game: &Game) -> usize {
        game.units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap()
    }

    #[test]
    fn following_lets_go_of_an_mcv_once_it_has_deployed_and_never_takes_a_building() {
        let mut game = in_battle();
        let mcv = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
            .unwrap();
        game.units[mcv].role = Role::Mcv;
        game.units[mcv].selected = true;
        game.handle(Event::KeyDown(Key::Char('f')));
        assert!(game.following.is_some(), "it can be followed on its tracks");
        units::switch_deployed(&mut game.units[mcv]);
        game.update(0.05);
        assert_eq!(game.following, None, "deployed, it is a building");
        game.handle(Event::KeyDown(Key::Char('f')));
        assert_eq!(game.following, None, "and stays that way");
    }

    #[test]
    fn the_f_key_keeps_the_camera_on_the_selected_unit_until_pressed_again() {
        let mut game = in_battle();
        game.handle(Event::KeyDown(Key::Char('f')));
        assert_eq!(game.following, None, "nobody selected");

        let tank = tank_of(&game);
        game.units[tank].selected = true;
        game.units[tank].destination = beside((game.units[tank].x, game.units[tank].z), 271.5);
        game.handle(Event::KeyDown(Key::Char('F')));
        let id = game.units[tank].id;
        assert_eq!(game.following, Some(id));
        let close = |game: &Game| {
            let unit = game.units.iter().find(|u| u.id == id).unwrap();
            (unit.x - game.camera.x).hypot(unit.z - game.camera.z) < 0.4525
        };
        assert!(close(&game), "at once");
        let start = game.camera.x;
        for _ in 0..20 {
            game.update(0.05);
            assert!(close(&game));
        }
        assert!(game.camera.x > start, "it went along");

        game.handle(Event::KeyDown(Key::Char('f')));
        assert_eq!(game.following, None, "released");
        let (x, z) = (game.camera.x, game.camera.z);
        game.update(0.5);
        assert_eq!((game.camera.x, game.camera.z), (x, z), "and it stays");
    }

    #[test]
    fn the_camera_lets_go_of_a_unit_that_is_gone_or_when_the_minimap_is_used() {
        let mut game = in_battle();
        let tank = tank_of(&game);
        game.units[tank].selected = true;
        game.handle(Event::KeyDown(Key::Char('f')));
        game.units[tank].alive = false;
        game.update(0.05);
        assert_eq!(game.following, None);

        let tank = tank_of(&game);
        game.units[tank].selected = true;
        game.handle(Event::KeyDown(Key::Char('f')));
        assert!(game.following.is_some());
        let (x, y, size) = hud::minimap_rect(W, H);
        click_at(&mut game, x + size / 3.0, y + size / 2.0);
        assert_eq!(game.following, None, "a click takes the camera away");
        game.handle(Event::KeyDown(Key::Char('f')));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseMove(x + size / 2.0, y + size / 2.0));
        assert_eq!(game.following, None, "a drag takes the camera away");
        game.handle(Event::MouseUp);
    }

    #[test]
    fn the_t_key_takes_the_types_selected_on_screen_then_on_the_whole_map() {
        let mut game = in_battle();
        let roles_of = |game: &Game| -> Vec<Role> {
            let mut roles: Vec<Role> = game.selected_units().iter().map(|u| u.role).collect();
            roles.dedup();
            roles
        };
        let tanks = |game: &Game| {
            game.units
                .iter()
                .filter(|u| u.owner == 0 && u.role == Role::HkAssault)
                .count()
        };
        game.handle(Event::KeyDown(Key::Char('t')));
        assert_eq!(selected_count(&game), 0, "nothing to go by");

        let tank = tank_of(&game);
        game.units[tank].selected = true;
        game.handle(Event::KeyDown(Key::Char('T')));
        let on_screen = selected_count(&game);
        assert!(on_screen > 1 && on_screen < tanks(&game), "{on_screen}");
        assert_eq!(roles_of(&game), [Role::HkAssault]);

        game.handle(Event::KeyDown(Key::Char('t')));
        assert_eq!(
            selected_count(&game),
            tanks(&game),
            "the second press: everywhere"
        );
        assert_eq!(roles_of(&game), [Role::HkAssault]);
        assert!(
            game.selected_units().iter().all(|u| u.owner == 0),
            "not the enemy's"
        );

        // Two types selected, one of them off screen: both, then both everywhere.
        game.handle(Event::RightDown);
        let away = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::AdvCarryall)
            .unwrap();
        game.units[tank].selected = true;
        game.units[away].selected = true;
        (game.units[away].x, game.units[away].z) = (0.0, 0.0);
        game.handle(Event::KeyDown(Key::Char('t')));
        assert!(game.units[away].selected, "kept off screen");
        game.handle(Event::KeyDown(Key::Char('t')));
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0 && [Role::HkAssault, Role::AdvCarryall].contains(&u.role))
                .all(|u| u.selected),
            "the second press: both types everywhere"
        );

        game.handle(Event::RightDown);
        let carryall = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::AdvCarryall)
            .unwrap();
        game.units[tank].selected = true;
        game.units[carryall].selected = true;
        game.handle(Event::KeyDown(Key::Char('t')));
        assert!(
            game.selected_units()
                .iter()
                .any(|u| u.role == Role::AdvCarryall)
        );
        assert!(
            game.selected_units()
                .iter()
                .any(|u| u.role == Role::HkAssault)
        );
        assert!(selected_count(&game) > 2);
    }

    #[test]
    fn the_p_key_takes_the_armed_on_screen_then_on_the_whole_map() {
        let mut game = in_battle();
        let armed = |u: &Unit| u.owner == 0 && !u.kind.is_building() && u.weapon.is_armed();
        let everyone = game.units.iter().filter(|u| armed(u)).count();
        game.handle(Event::KeyDown(Key::Char('p')));
        let on_screen = selected_count(&game);
        assert!(
            on_screen > 0 && on_screen < everyone,
            "{on_screen} {everyone}"
        );
        assert!(game.selected_units().iter().all(|u| armed(u)));
        assert!(
            !game
                .selected_units()
                .iter()
                .any(|u| u.role == Role::AdvCarryall),
            "carryalls carry no weapon"
        );
        game.handle(Event::KeyDown(Key::Char('P')));
        assert_eq!(selected_count(&game), everyone);
        use crate::weapons::WeaponKind;
        assert!(WeaponKind::Laser.is_armed());
        assert!(!WeaponKind::Repair.is_armed() && !WeaponKind::Unarmed.is_armed());

        game.handle(Event::RightDown);
        game.handle(Event::KeyDown(Key::Pause));
        game.handle(Event::KeyDown(Key::Char('p')));
        assert_eq!(selected_count(&game), 0, "not while paused");
    }

    #[test]
    fn the_a_key_leaves_a_selected_tower_out() {
        let mut game = in_battle();
        let tower = tower_of(&game, 0);
        game.units[tower].selected = true;
        game.handle(Event::KeyDown(Key::Char('a')));
        assert!(!game.units[tower].selected);
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

    /// A point on the screen over rock clear enough for a building, away from the HUD.
    fn buildable_spot_on_screen(game: &mut Game) -> (f32, f32) {
        let terrain = battlefield::terrain(game.battle_map);
        // The base standing by the map's middle is on the sand: put up a tower on rock.
        let at = terrain.starts()[2];
        let (x, z) = at;
        if !game.units.iter().any(|u| u.owner == 0 && (u.x, u.z) == at) {
            let tower = game
                .units
                .iter()
                .position(|u| u.kind == UnitKind::Tower && u.owner == 0);
            let mut outpost = game.units[tower.unwrap()];
            (outpost.x, outpost.z, outpost.destination) = (x, z, at);
            outpost.y = terrain.height_at(x, z);
            outpost.id = game.next_unit_id;
            game.next_unit_id += 1;
            game.units.push(outpost);
        }
        game.camera.look_at(x, z);
        game.update_fog();
        let role = match game.site {
            Site::Placing(role) | Site::Ready(role) => role,
            _ => Role::Factory,
        };
        (1..100)
            .flat_map(|i| (1..60).map(move |j| (i as f32 * W / 100.0, j as f32 * H / 60.0)))
            .filter(|&(x, y)| !hud::covers(x, y, W, H))
            .find(|&(x, y)| {
                game.view().land_at(terrain, x, y).is_some_and(|at| {
                    let plot = role.plot_facing(game.faction_of(0), at, game.placing_facing);
                    units::can_place_building(&game.units, terrain, plot)
                        && units::is_near_building_of(&game.units, plot, 0)
                })
            })
            .expect("somewhere to build")
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
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap();
        let repair = cell_of(&game, BuildTab::Vehicles, Role::AtRepairUnit);
        let fixer = select_new(&mut game, repair);
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
        let repair = cell_of(&game, BuildTab::Vehicles, Role::AtRepairUnit);
        let fixer = select_new(&mut game, repair);
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
        assert_eq!(
            game.cursor_look(),
            CursorLook::Move,
            "an enemy it cannot harm, which a click just walks up to"
        );
    }

    #[test]
    fn with_armed_units_selected_too_only_control_turns_a_friendly_vehicle_red() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap();
        let repair = cell_of(&game, BuildTab::Vehicles, Role::AtRepairUnit);
        let fixer = select_new(&mut game, repair);
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
        with_bases(&mut game);
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
        assert_eq!(plain.sight_at(1358.0, 1358.0), Sight::Visible);
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
        let (x, z) = beside((game.camera.x, game.camera.z), 271.5);
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
        assert_eq!(game.tip(), Some("Infantry"));
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
        (game.units[keep].x, game.units[keep].destination.0) = (x + 1810.0, x + 1810.0);
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
        let (far_x, far_z) = beside((game.camera.x, game.camera.z), -1131.0);
        assert_eq!(game.sight_at(far_x, far_z), Sight::Unexplored);
        let scout = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Aircraft)
            .unwrap();
        game.units[scout].destination = (far_x, far_z);
        // An enemy base stands out there, and its guards shoot back.
        game.units[scout].health = 1e9;
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
        for (x, z) in [(x, z), (0.0, 226.3)] {
            game.combat
                .poison
                .push(crate::weapons::PoisonZone { x, z, life: 30.0 });
            game.combat.blasts.push(crate::weapons::Blast {
                at: (x, 0.0, z),
                radius: 90.51,
                life: 0.5,
            });
            game.combat.flares.push(crate::weapons::Flare {
                at: (x, 45.25, z),
                direction: (1.0, 0.0, 0.0),
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
    fn units_answer_being_selected_sent_somewhere_and_told_to_attack() {
        let mut game = in_battle();
        game.fog = None;
        let _ = game.take_reply();
        select_first_group(&mut game);
        let (said, speakers) = game.take_reply().expect("the group answers");
        assert_eq!(said, Said::Selected);
        assert_eq!(speakers.len(), FIRST_GROUP);
        assert!(
            speakers
                .iter()
                .all(|&(_, house)| house == game.faction_of(0))
        );

        let (x, y) = clear_spot_on_screen();
        click_at(&mut game, x, y);
        assert_eq!(game.take_reply().map(|(said, _)| said), Some(Said::Moving));
        click_at(&mut game, x, y);
        assert_eq!(
            game.take_reply().map(|(said, _)| said),
            Some(Said::Moving),
            "the same again"
        );

        let (_, (x, y)) = enemy_beside_first_group(&mut game);
        click_at(&mut game, x, y);
        assert_eq!(
            game.take_reply().map(|(said, _)| said),
            Some(Said::Attacking)
        );

        // Nothing new selected, nothing said.
        game.handle(Event::MouseMove(x + 1.0, y));
        assert_eq!(game.take_reply(), None);
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
                .filter(|u| u.selected && !u.kind.is_building() && u.kind.flies() == flying)
                .map(|u| u.destination)
                .collect();
            assert!(!sent.is_empty());
            for (i, a) in sent.iter().enumerate() {
                assert!(
                    (a.0 - target.0).hypot(a.1 - target.1) < 226.3,
                    "near the click"
                );
                for b in &sent[i + 1..] {
                    assert!((a.0 - b.0).hypot(a.1 - b.1) > 45.25, "not one point");
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
    fn a_click_while_the_zoom_is_still_easing_sends_them_and_keeps_them_selected() {
        let mut game = in_battle();
        select_first_group(&mut game);
        game.camera.zoom_by(4.0);
        let clicked = (W / 2.0 - 800.0, H / 2.0 + 300.0);
        game.handle(Event::MouseMove(clicked.0, clicked.1));
        game.handle(Event::MouseDown);
        game.update(0.05);
        game.handle(Event::MouseUp);
        assert_eq!(selected_count(&game), FIRST_GROUP, "they stay selected");
        assert!(game.units.iter().any(|u| u.selected && u.is_moving()));
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
            west,
            battlefield::across_and_down(anchored.0, anchored.1).0,
            "the start stays where it was on the ground"
        );
        let under_cursor = game.view().ground_at(W - 1.0, H / 2.0).unwrap();
        let under_cursor = battlefield::across_and_down(under_cursor.0, under_cursor.1);
        assert!(
            (east - under_cursor.0).abs() < 0.04525,
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
        assert_eq!(game.camera.zoom, 1.25, "near in shows perspective best");
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
        let corner = |(x, y): (f32, f32)| {
            let (x, z) = view.ground_at(x, y).unwrap();
            game.camera.across_and_down(x, z)
        };
        let (corner_a, corner_b) = (corner(a), corner(b));
        // Just inside the far edge's right end on the ground, and just outside it: on the
        // screen both are left of where the near edge ends.
        let place = |across: f32| game.camera.from_across_and_down(across, corner_a.1 + 4.525);
        (game.units[0].x, game.units[0].z) = place(corner_b.0 - 4.525);
        (game.units[1].x, game.units[1].z) = place(corner_b.0 + 4.525);
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
        let across = |u: &&Unit| battlefield::across_and_down(u.x, u.z).0;
        let far = game
            .units
            .iter()
            .filter(|u| u.owner == 0)
            .max_by(|a, b| across(a).total_cmp(&across(b)))
            .map(|u| (u.x, u.z))
            .unwrap();
        for _ in 0..400 {
            game.update(0.05);
            let (sx, _) = game.view().project(far.0, 0.0, far.1).unwrap();
            if sx < W - 100.0 {
                break;
            }
        }
        game.handle(Event::MouseUp);
        let players = game
            .units
            .iter()
            .filter(|u| u.is_selected_in_groups())
            .count();
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
        let (a, b, c) = (
            find(Faction::Atreides),
            find(Faction::Harkonnen),
            find(Faction::Ordos),
        );
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
            .find(|b| b.action == Action::Faction(Faction::Atreides))
            .unwrap();
        game.handle(Event::MouseMove(a.x + 2.0, a.y + 2.0));
        assert_eq!(game.hovered(), None);
        game.handle(Event::MouseMove(a.x + a.w / 2.0, a.y + 2.0));
        assert_eq!(game.hovered(), Some(Action::Faction(Faction::Atreides)));
    }

    #[test]
    fn picking_plays_that_factions_movie_then_asks() {
        for faction in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
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
    fn a_factions_movie_from_the_disc_lasts_until_it_ends_then_asks() {
        let mut game = in_faction_select();
        click(&mut game, Action::Faction(Faction::Ordos));
        game.start_movie();
        game.update(MOVIE_SECONDS * 10.0);
        assert_eq!(game.screen, Screen::Movie(Faction::Ordos));
        assert!(game.is_playing_movie());
        game.end_movie();
        assert_eq!(game.screen, Screen::FactionSelect);
        assert_eq!(game.confirming, Some(Faction::Ordos));
        assert!(!game.is_playing_movie());
    }

    #[test]
    fn a_skipped_movie_leaves_the_next_movie_screen_on_its_own_time() {
        let mut game = Game::new(W, H);
        game.start_movie();
        game.handle(Event::KeyDown(Key::Other));
        assert_eq!(game.screen, Screen::MainMenu);
        assert!(!game.is_playing_movie());
        game.end_movie();
        assert_eq!(
            game.screen,
            Screen::MainMenu,
            "ending a skipped movie does nothing"
        );

        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::StartCampaign);
        pick(&mut game, Faction::Atreides);
        assert_eq!(game.confirming, Some(Faction::Atreides));
    }

    #[test]
    fn movie_can_be_skipped_by_click_or_key() {
        let mut game = in_faction_select();
        click(&mut game, Action::Faction(Faction::Harkonnen));
        game.handle(Event::KeyDown(Key::Other));
        assert_eq!(game.confirming, Some(Faction::Harkonnen));

        let mut game = in_faction_select();
        click(&mut game, Action::Faction(Faction::Harkonnen));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert_eq!(game.confirming, Some(Faction::Harkonnen));
    }

    #[test]
    fn cancel_lets_the_player_pick_again() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::Atreides);
        click(&mut game, Action::CancelFaction);
        assert_eq!(game.screen, Screen::FactionSelect);
        assert_eq!(game.confirming, None);

        pick(&mut game, Faction::Ordos);
        assert_eq!(game.confirming, Some(Faction::Ordos));

        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.confirming, None, "escape cancels the message box");
        assert_eq!(game.screen, Screen::FactionSelect);
    }

    #[test]
    fn ok_loads_the_fake_game_whose_only_button_is_back() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::Ordos);
        click(&mut game, Action::ConfirmFaction);
        game.update(OPENING_WAIT_SECONDS + 0.05);
        assert_eq!(game.screen, Screen::Loading);
        assert_eq!(game.loading_progress(), 0.0);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        assert_eq!(game.faction, Some(Faction::Ordos));
        assert!(game.buttons().is_empty());
    }

    #[test]
    fn message_box_blocks_the_screen_behind_it() {
        let mut game = in_faction_select();
        let behind = buttons(Screen::FactionSelect, W, H);
        pick(&mut game, Faction::Atreides);
        for b in behind {
            game.handle(Event::MouseMove(b.x + b.w / 2.0, b.y + b.h / 2.0));
            game.handle(Event::MouseDown);
            game.handle(Event::MouseUp);
            assert_eq!(game.screen, Screen::FactionSelect, "{} is blocked", b.label);
            assert_eq!(game.confirming, Some(Faction::Atreides));
        }
    }

    #[test]
    fn tutorial_after_a_campaign_has_no_faction() {
        let mut game = in_faction_select();
        pick(&mut game, Faction::Harkonnen);
        click(&mut game, Action::ConfirmFaction);
        game.update(OPENING_WAIT_SECONDS + 0.05);
        game.update(LOADING_SECONDS);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        assert_eq!(game.faction, None);
    }

    #[test]
    fn without_the_disc_the_setup_and_diplomacy_show_planets_and_shapes_not_logos() {
        let mut game = in_setup();
        game.skirmish.opponents[0].allies[0] = Ally::Fremen;
        let plain = |game: &Game| {
            crate::render::build(game)
                .iter()
                .all(|v| v.layer == crate::platform::NO_LAYER)
        };
        assert!(plain(&game));
        let mut battle = in_battle();
        battle.handle(Event::KeyDown(Key::Tab));
        assert!(battle.stats_open);
        assert!(plain(&battle));
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
            let items = map_items(MapFolder::Skirmish, 0, w, h);
            assert_eq!(items.len(), maps::list().len());
            let last = items.last().unwrap();
            assert!(last.y + last.h <= h, "at {w}x{h}");
            for item in &items {
                let Action::SelectMap(map) = item.action else {
                    continue;
                };
                // The name from the left, and how many players from the right, apart.
                let text = |text: &str| crate::font::text_width(text) * small_text_px(h);
                let note = text(&players_note(maps::list()[map].players));
                assert!(
                    text(item.label) + note + 48.0 * ui_scale(h) <= item.w,
                    "{} at {w}x{h}",
                    item.label
                );
            }
        }
        assert_eq!(players_note(4), "4 players");
        for map in 0..maps::list().len() {
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
    fn as_many_maps_as_the_disc_has_can_be_scrolled_to_on_the_screen_with_their_names() {
        const NAMES: [&str; 3] = ["Giedi Prime : Three Bridges", "Fishers Plain", "Sihaya"];
        for (w, h) in [
            (1280.0, 720.0),
            (1920.0, 1080.0),
            (2880.0, 1800.0),
            (1024.0, 768.0),
            (5120.0, 1440.0),
        ] {
            for (count, tabs) in [(34, false), (48, true)] {
                let entries = NAMES.iter().copied().cycle().take(count).enumerate();
                let most =
                    ListLayout::new(entries.clone().map(|(_, l)| l), tabs, w, h).most_scroll();
                let mut seen = Vec::new();
                for scroll in 0..=most + 1 {
                    let items = list_items(entries.clone(), tabs, scroll, w, h);
                    for item in &items {
                        assert!(item.x >= 0.0 && item.x + item.w <= w, "at {w}x{h}");
                        assert!(item.y + item.h <= h, "at {w}x{h}");
                        let width = crate::font::text_width(item.label) * small_text_px(h);
                        assert!(width + 16.0 * ui_scale(h) < item.w, "at {w}x{h}");
                        seen.push(item.action);
                    }
                }
                seen.sort_by_key(|action| match action {
                    Action::SelectMap(map) => *map,
                    _ => usize::MAX,
                });
                seen.dedup();
                assert_eq!(seen.len(), count, "each in some column at {w}x{h}");
            }
            let first = list_items([(0, "Sihaya")].into_iter(), false, 0, w, h)[0];
            assert_eq!(first.x, setup_column(false, w, h).0, "under the dropdown");
        }
    }

    #[test]
    fn the_wheel_scrolls_the_open_map_list_no_further_than_it_goes() {
        let mut game = in_setup();
        click_widget(&mut game, Action::MapDropdown);
        let zoom = game.camera.zoom;
        game.handle(Event::Wheel(-3.0));
        assert_eq!(game.map_scroll, 0, "the test maps all fit");
        game.handle(Event::Wheel(1.0));
        assert_eq!(game.map_scroll, 0);
        assert_eq!(game.camera.zoom, zoom);
        assert!(game.dropdown_open);
    }

    #[test]
    fn open_dropdown_blocks_what_is_under_it_and_closes_on_outside_click() {
        let mut game = in_setup();
        click_widget(&mut game, Action::MapDropdown);
        let item = map_items(MapFolder::Skirmish, 0, W, H)[0];
        game.handle(Event::MouseMove(
            item.x + item.w / 2.0,
            item.y + item.h / 2.0,
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
        assert_eq!(game.skirmish.faction, Faction::Atreides);
        for expected in [Faction::Harkonnen, Faction::Ordos, Faction::Atreides] {
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

        let mut allies = [Ally::Tleilaxu, Ally::None];
        cycle_ally(&mut allies, 1);
        assert_eq!(allies, [Ally::Tleilaxu, Ally::Ix], "T is skipped");

        let mut allies = [Ally::Sardaukar, Ally::Fremen];
        cycle_ally(&mut allies, 1);
        assert_eq!(
            allies,
            [Ally::Sardaukar, Ally::None],
            "S is skipped, none is still allowed"
        );

        let mut allies = [Ally::None, Ally::None];
        cycle_ally(&mut allies, 0);
        cycle_ally(&mut allies, 0);
        cycle_ally(&mut allies, 1);
        assert_eq!(
            allies,
            [Ally::Ix, Ally::Tleilaxu],
            "both slots can be filled"
        );
    }

    #[test]
    fn opponents_cannot_befriend_the_same_mini_faction_twice() {
        let mut game = in_setup();
        click_widget(&mut game, Action::CycleOpponentAlly(0, 0));
        click_widget(&mut game, Action::CycleOpponentAlly(0, 1));
        assert_eq!(
            game.skirmish.opponents[0].allies,
            [Ally::Tleilaxu, Ally::Ix]
        );
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
            [
                Ally::Tleilaxu,
                Ally::Ix,
                Ally::Guild,
                Ally::Fremen,
                Ally::Sardaukar,
                Ally::None
            ]
        );
        assert_eq!(game.skirmish.allies[0], Ally::None);
        click_widget(&mut game, Action::CycleAlly(0));
        assert_eq!(game.skirmish.allies, [Ally::Tleilaxu, Ally::None]);
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
        assert_eq!(game.skirmish.opponents[1].allies, [Ally::None, Ally::Ix]);
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

    /// The ids of the first two of the player's units that can be selected in groups.
    fn two_soldiers(game: &Game) -> (u32, u32) {
        let mut soldiers = game
            .units
            .iter()
            .filter(|u| u.is_selected_in_groups())
            .map(|u| u.id);
        (soldiers.next().unwrap(), soldiers.next().unwrap())
    }

    fn select_ids(game: &mut Game, ids: &[u32]) {
        for unit in &mut game.units {
            unit.selected = ids.contains(&unit.id);
        }
    }

    fn selected_ids(game: &Game) -> Vec<u32> {
        game.selected_units().iter().map(|u| u.id).collect()
    }

    #[test]
    fn a_number_key_selects_those_assigned_to_it_even_out_of_view() {
        let mut game = in_battle();
        let (first, second) = two_soldiers(&game);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(1, Some(Modifier::Control)));
        select_ids(&mut game, &[second]);

        let enemy = game.units.iter().find(|u| u.owner == 1).copied().unwrap();
        game.camera.look_at(enemy.x, enemy.z);
        game.handle(Event::NumberKey(1, None));
        assert_eq!(selected_ids(&game), [first]);
    }

    #[test]
    fn shift_and_a_number_key_adds_to_the_selection() {
        let mut game = in_battle();
        let (first, second) = two_soldiers(&game);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(9, Some(Modifier::Control)));
        select_ids(&mut game, &[second]);
        game.handle(Event::NumberKey(9, Some(Modifier::Shift)));
        assert_eq!(selected_ids(&game), [first, second]);
    }

    #[test]
    fn assigning_a_number_key_replaces_what_it_held() {
        let mut game = in_battle();
        let (first, second) = two_soldiers(&game);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(2, Some(Modifier::Control)));
        select_ids(&mut game, &[second]);
        game.handle(Event::NumberKey(2, Some(Modifier::Control)));
        select_ids(&mut game, &[]);
        game.handle(Event::NumberKey(2, None));
        assert_eq!(selected_ids(&game), [second]);
    }

    #[test]
    fn assigning_with_nothing_selected_keeps_what_the_key_held() {
        let mut game = in_battle();
        let (first, _) = two_soldiers(&game);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(3, Some(Modifier::Control)));
        select_ids(&mut game, &[]);
        game.handle(Event::NumberKey(3, Some(Modifier::Control)));
        game.handle(Event::NumberKey(3, None));
        assert_eq!(selected_ids(&game), [first]);
    }

    #[test]
    fn a_number_key_with_none_left_keeps_the_selection() {
        let mut game = in_battle();
        let (first, second) = two_soldiers(&game);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(4, Some(Modifier::Control)));
        game.units.retain(|u| u.id != first);
        select_ids(&mut game, &[second]);
        game.handle(Event::NumberKey(4, None));
        assert_eq!(
            selected_ids(&game),
            [second],
            "an empty key changes nothing"
        );
        game.handle(Event::NumberKey(5, None));
        assert_eq!(selected_ids(&game), [second], "as does one never assigned");
    }

    #[test]
    fn number_keys_do_nothing_while_paused() {
        let mut game = in_battle();
        let (first, second) = two_soldiers(&game);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(1, Some(Modifier::Control)));
        select_ids(&mut game, &[second]);
        game.handle(Event::KeyDown(Key::Pause));
        game.handle(Event::NumberKey(1, None));
        assert_eq!(selected_ids(&game), [second]);
    }

    #[test]
    fn a_unit_knows_the_lowest_number_key_it_is_under() {
        let mut game = in_battle();
        let (first, second) = two_soldiers(&game);
        assert_eq!(game.group_number(first), None);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(4, Some(Modifier::Control)));
        game.handle(Event::NumberKey(2, Some(Modifier::Control)));
        assert_eq!(game.group_number(first), Some(2));
        assert_eq!(game.group_number(second), None);
    }

    #[test]
    fn dragging_while_placing_turns_the_squares_and_the_next_click_builds_it_that_way() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        game.site = Site::Placing(Role::Factory);
        // Somewhere there is room for it the way it will end up.
        game.placing_facing = units::Facing::RightBottom;
        let (x, y) = buildable_spot_on_screen(&mut game);
        game.placing_facing = units::Facing::LeftBottom;
        let before = game.units.len();
        game.handle(Event::MouseMove(x, y));
        let exits = |game: &Game| {
            game.placing_squares()
                .iter()
                .filter(|(_, square)| *square == PlaceSquare::Exit)
                .count()
        };
        let plain = game.placing_plot().unwrap().1;

        game.handle(Event::MouseDown);
        assert_eq!(game.placing_arrow(), None, "no arrow before it turns");
        game.handle(Event::MouseMove(x + 60.0, y - 60.0));
        assert_eq!(
            game.placing_plot().unwrap().1.u,
            plain.u,
            "the squares stay put"
        );
        assert_eq!(game.placing_facing, units::Facing::RightTop);
        assert!(game.selection_ground().is_none(), "no selection box");
        assert_eq!(
            game.placing_arrow().map(|(_, way)| way),
            Some(units::Facing::RightTop.way_out()),
            "an arrow points the way it looks"
        );
        game.handle(Event::MouseMove(x + 60.0, y + 60.0));
        assert_eq!(game.placing_facing, units::Facing::RightBottom);
        assert_eq!(exits(&game), 2);
        let turned = game.placing_plot().unwrap().1;
        assert_eq!((turned.cols, turned.rows), (plain.rows, plain.cols));
        game.handle(Event::MouseUp);
        assert_eq!(game.placing_arrow(), None, "the arrow goes with the button");
        assert_eq!(game.units.len(), before, "a drag builds nothing");
        assert_eq!(game.site, Site::Placing(Role::Factory), "still placing");

        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
        assert_eq!(game.units.len(), before + 1, "the next click does");
        let built = *game.units.last().unwrap();
        assert_eq!(built.facing, units::Facing::RightBottom);
        assert_eq!((built.plot().unwrap().cols, built.plot().unwrap().rows), {
            let (cols, rows) = Role::Factory.plot_size(built.house);
            (rows, cols)
        });
    }

    #[test]
    fn the_cursors_for_orders_lie_flat_on_the_map() {
        let mut game = cursor_on_open_ground();
        let soldier = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Infantry)
            .unwrap();
        game.units[soldier].selected = true;
        // From low down, where a thing lying flat looks much wider than it is tall.
        for _ in 0..400 {
            game.camera.zoom_by(1.0);
            game.camera.ease_zoom(0.05);
        }
        // Close up, the machines nearby cover the spot.
        game.units.retain(|u| u.selected);
        assert_eq!(game.cursor_look(), CursorLook::Move);
        let drawn = crate::render::build(&game);
        // Its ring goes all the way round: as tall as it is wide only if it faces the screen.
        let cursor = |near: f32| {
            let (mx, my) = game.mouse;
            let around: Vec<_> = drawn
                .iter()
                .rev()
                .take(600)
                .filter(|v| (v.x - mx).hypot(v.y - my) < near)
                .collect();
            let reach = |value: fn(&&crate::platform::Vertex) -> f32| {
                let (low, high) = around
                    .iter()
                    .map(value)
                    .fold((f32::MAX, f32::MIN), |(a, b), c| (a.min(c), b.max(c)));
                high - low
            };
            (reach(|v| v.x), reach(|v| v.y))
        };
        let (wide, tall) = cursor(120.0);
        assert!(tall < wide * 0.85, "{wide} {tall}");
    }

    #[test]
    fn an_explosion_adds_balls_of_light_over_it() {
        let mut game = cursor_on_open_ground();
        let before = crate::render::build_frame(&game).effects.len();
        let at = (game.camera.x, 22.63, game.camera.z);
        game.combat.explode(at, 45.25);
        game.combat.particles.clear();
        let effects = crate::render::build_frame(&game).effects;
        let added: Vec<_> = effects[before..].iter().filter(|v| v.a == 0.0).collect();
        assert!(added.len() > 100, "{}", added.len());
    }

    #[test]
    fn a_tornado_shows_as_a_see_through_funnel() {
        let mut game = cursor_on_open_ground();
        let terrain = battlefield::terrain(game.battle_map);
        let before = crate::render::build_frame(&game).effects.len();
        let mut combat = Combat::default();
        for _ in 0..4000 {
            combat.stir_tornados(terrain, 0.05);
            if combat.tornados.iter().any(|t| t.life < 0.8) {
                break;
            }
        }
        let tornado = *combat.tornados.iter().find(|t| t.life < 0.8).unwrap();
        game.camera.look_at(tornado.at.0, tornado.at.2);
        game.update_fog();
        let before = before.min(crate::render::build_frame(&game).effects.len());
        game.combat.tornados = vec![tornado];
        let effects = crate::render::build_frame(&game).effects;
        assert!(effects.len() >= before + 9 * 14 * 6, "{}", effects.len());
        assert!(effects.iter().any(|v| v.a > 0.0 && v.a < 0.7));
    }

    #[test]
    fn a_new_battle_forgets_the_number_keys() {
        let mut game = in_battle();
        let (first, _) = two_soldiers(&game);
        select_ids(&mut game, &[first]);
        game.handle(Event::NumberKey(1, Some(Modifier::Control)));
        game.start_battle(0, None, BattleKind::Tutorial);
        assert!(game.control_groups[0].is_empty());
    }

    fn in_battle() -> Game {
        in_battle_as(Faction::Atreides)
    }

    /// A skirmish with the player of this house.
    fn in_battle_as(house: Faction) -> Game {
        let mut game = in_setup();
        game.skirmish.faction = house;
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        assert_eq!(game.screen, Screen::InGame);
        with_bases(&mut game);
        game.handle(Event::MouseMove(W / 2.0, H / 2.0));
        game
    }

    /// Swaps a battle's lone MCVs for bases already built and armies standing by them, the way a
    /// battle well under way looks.
    fn with_bases(game: &mut Game) {
        let colors: Vec<TeamColor> = game.stats_rows().iter().map(|row| row.color).collect();
        game.units = units::spawn_bases(battlefield::terrain(game.battle_map), &colors);
        let houses: Vec<Faction> = (0..colors.len()).map(|o| game.faction_of(o)).collect();
        for unit in &mut game.units {
            unit.house = houses[unit.owner];
        }
        game.next_unit_id = game.units.len() as u32;
        game.home = battlefield::MAP_MIDDLE;
        game.camera = Camera::centered();
        game.update_fog();
    }

    #[test]
    fn play_uses_the_selected_map_and_looks_at_the_players_mcv() {
        let mut game = in_setup();
        click_widget(&mut game, Action::MapDropdown);
        click(&mut game, Action::SelectMap(2));
        click_widget(&mut game, Action::StartBattle);
        assert_eq!(game.battle_map, 2);
        assert_eq!((game.camera.x, game.camera.z), game.home());
    }

    #[test]
    fn the_cursor_at_an_edge_scrolls_only_on_the_battlefield() {
        let mut game = in_battle();
        let start = game.camera;
        game.update(0.5);
        assert_eq!(game.camera, start, "cursor in the middle");

        let on_screen = |camera: Camera| battlefield::across_and_down(camera.x, camera.z);
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.update(0.5);
        let (across, down) = on_screen(game.camera);
        assert!(across > on_screen(start).0, "the right edge moves right");
        assert!((down - on_screen(start).1).abs() < 0.01);

        let before = game.camera;
        game.handle(Event::MouseMove(W / 2.0, 0.0));
        game.update(0.5);
        assert!(
            on_screen(game.camera).1 < on_screen(before).1,
            "the top edge moves up"
        );

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
        for _ in 0..300 {
            game.update(0.016);
        }
    }

    #[test]
    fn the_wheel_and_a_pinch_zoom_and_keys_and_fingers_turn_only_on_the_battlefield() {
        let mut game = in_battle();
        settle_zoom(&mut game);
        let usual = game.camera;
        game.handle(Event::Wheel(100.0));
        settle_zoom(&mut game);
        assert!(
            game.camera.zoom > usual.zoom,
            "wheel away from the player zooms in"
        );
        let near_in = game.camera.zoom;
        game.handle(Event::Wheel(-100.0));
        settle_zoom(&mut game);
        assert!(game.camera.zoom < usual.zoom);
        game.handle(Event::Magnify(0.2));
        assert!(game.camera.zoom > usual.zoom * 0.0 && game.camera.zoom < near_in);
        let pinched = game.camera.zoom;
        game.handle(Event::Magnify(0.2));
        assert!(game.camera.zoom > pinched, "fingers spread apart zoom in");
        game.handle(Event::Rotate(90.0));
        assert!((game.camera.yaw - usual.yaw - 90f32.to_radians()).abs() < 1e-4);
        game.handle(Event::KeyDown(Key::Char('/')));
        settle_zoom(&mut game);
        assert!(
            (game.camera.yaw - usual.yaw).abs() < 1e-3,
            "and back as it was"
        );
        game.handle(Event::KeyDown(Key::Char('.')));
        settle_zoom(&mut game);
        assert!(game.camera.yaw > usual.yaw);

        let mut game = in_setup();
        let before = game.camera;
        game.handle(Event::Wheel(100.0));
        game.handle(Event::Magnify(0.5));
        game.handle(Event::Rotate(45.0));
        for _ in 0..50 {
            game.update(0.016);
        }
        assert_eq!(
            (game.camera.zoom, game.camera.yaw),
            (before.zoom, before.yaw),
            "menus ignore it"
        );
    }

    #[test]
    fn a_new_battle_opens_with_the_originals_sweep_in_whatever_the_last_one_ended_at() {
        let mut game = in_battle();
        settle_zoom(&mut game);
        let usual = game.camera;
        game.handle(Event::Wheel(5.0));
        game.handle(Event::Rotate(30.0));
        settle_zoom(&mut game);
        assert!(game.camera.zoom > usual.zoom);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        game.update(LOADING_SECONDS);
        assert!(game.camera.zoom < usual.zoom, "it starts out far");
        settle_zoom(&mut game);
        assert_eq!((game.camera.zoom, game.camera.yaw), (usual.zoom, usual.yaw));
    }

    fn click_at(game: &mut Game, x: f32, y: f32) {
        game.handle(Event::MouseMove(x, y));
        game.handle(Event::MouseDown);
        game.handle(Event::MouseUp);
    }

    /// A point on the minimap well inside its diamond, and the map's place under it.
    fn minimap_point() -> ((f32, f32), (f32, f32)) {
        let rect = hud::minimap_rect(W, H);
        let at = (rect.0 + rect.2 * 0.6, rect.1 + rect.2 * 0.5);
        (
            at,
            battlefield::minimap_to_world(rect, battlefield::terrain(0).extent(), at.0, at.1)
                .unwrap(),
        )
    }

    #[test]
    fn a_click_on_the_minimap_moves_the_camera_there_and_gives_no_order() {
        let mut game = in_battle();
        let tank = tank_of(&game);
        game.units[tank].selected = true;
        let stays = game.units[tank].destination;
        let (at, world) = minimap_point();
        click_at(&mut game, at.0, at.1);
        let mut expected = game.camera;
        expected.look_at(world.0, world.1);
        assert_eq!((game.camera.x, game.camera.z), (expected.x, expected.z));
        assert_eq!(game.units[tank].destination, stays);
        assert!(game.units[tank].selected, "the selection stands");
    }

    #[test]
    fn a_click_or_the_cursor_anywhere_on_a_machine_picks_it_not_only_at_its_middle() {
        let mut game = in_battle();
        game.fog = None;
        let i = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.kind == UnitKind::Vehicle)
            .unwrap();
        game.units[i].role = Role::Harvester;
        game.units.iter_mut().for_each(|u| u.selected = false);
        let unit = game.units[i];
        game.units.retain(|u| u.id == unit.id);
        game.camera.look_at(unit.x, unit.z);
        for _ in 0..100 {
            game.camera.zoom_by(1.0);
            game.camera.ease_zoom(0.05);
        }
        let view = game.view();
        let (middle_x, middle_y) = view.project(unit.x, unit.pick_height(), unit.z).unwrap();
        let [corner, ..] = unit.selection_box();
        // By its nose, just inside its box.
        let near = |a: f32, b: f32| b + (a - b) * 0.9;
        let (nose_x, nose_y) = view
            .project(
                near(corner.0, unit.x),
                unit.pick_height(),
                near(corner.2, unit.z),
            )
            .unwrap();
        let reach = PICK_RADIUS * ui_scale(game.height);
        assert!(
            (nose_x - middle_x).hypot(nose_y - middle_y) > reach,
            "out of the old reach"
        );
        game.handle(Event::MouseMove(nose_x, nose_y));
        assert_eq!(game.tip(), Some(unit.name()), "its name shows");
        click_at(&mut game, nose_x, nose_y);
        assert!(game.units[0].selected);
    }

    #[test]
    fn a_click_anywhere_on_a_building_selects_it_not_only_at_its_middle() {
        let mut game = in_battle();
        let i = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Factory)
            .unwrap();
        let unit = game.units[i];
        game.camera.look_at(unit.x, unit.z);
        let view = game.view();
        let (middle_x, middle_y) = view.project(unit.x, unit.pick_height(), unit.z).unwrap();
        let plot = unit
            .role
            .plot_facing(unit.house, (unit.x, unit.z), unit.facing);
        let (x, z) = battlefield::grid_to_world(
            plot.u as f32 + plot.cols as f32 - 0.5,
            plot.v as f32 + plot.rows as f32 / 2.0,
        );
        let (edge_x, edge_y) = view.project(x, unit.y, z).unwrap();
        let reach = PICK_RADIUS * ui_scale(game.height);
        assert!(
            (edge_x - middle_x).hypot(edge_y - middle_y) > reach,
            "out of the old reach"
        );
        game.handle(Event::MouseMove(edge_x, edge_y));
        assert_eq!(
            game.unit_near_cursor(|_| true).map(|(found, _)| found),
            Some(i)
        );
    }

    #[test]
    fn the_menus_are_where_the_intro_and_the_battle_are_not() {
        let mut game = Game::new(1920.0, 1080.0);
        assert!(!game.is_in_menus(), "the intro");
        game.set_screen(Screen::MainMenu);
        assert!(game.is_in_menus());
        game.set_screen(Screen::Options);
        assert!(game.is_in_menus());
        assert!(!in_battle().is_in_menus());
    }

    #[test]
    fn every_house_makes_harvesters_in_its_factory_and_refineries_from_its_mcv() {
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let mut game = in_battle_as(house);
            assert!(offers(&game, BuildTab::Buildings, Role::Refinery));
            assert_eq!(Role::Harvester.made_in(), Some(Role::Factory));
            assert!(Role::Refinery.made_by(house));
            click_hud(&mut game, Action::BuildTab(BuildTab::Vehicles));
            let before = game.units.len();
            click_role(&mut game, BuildTab::Vehicles, Role::Harvester);
            assert_eq!(game.units.len(), before + 1, "{house:?}");
            assert_eq!(game.units[before].role, Role::Harvester);
        }
    }

    #[test]
    fn a_harvester_emptying_at_a_refinery_adds_to_its_owners_money() {
        let mut game = in_battle();
        let terrain = battlefield::terrain(game.battle_map);
        let at = (game.units[0].x, game.units[0].z);
        let refinery =
            units::new_building(terrain, at, Role::Refinery, (0, game.units[0].color), 9_999);
        game.units.push(refinery);
        let refinery = game.units.len() - 1;
        let mut harvester = game.units[refinery];
        harvester.role = Role::Harvester;
        harvester.kind = UnitKind::Vehicle;
        harvester.id = 10_000;
        (harvester.x, harvester.z) = game.units[refinery].refinery_pads()[0];
        harvester.load = units::HARVESTER_LOAD;
        harvester.harvest = units::Harvest::Dumping(units::Dock {
            refinery: 9_999,
            pad: 0,
        });
        game.units.push(harvester);
        let before = game.player_money();
        for _ in 0..(units::DUMP_SECONDS * 20.0) as usize + 10 {
            game.update(0.05);
        }
        assert_eq!(game.player_money(), before + 700);
    }

    #[test]
    fn selected_harvesters_show_the_land_cursor_over_a_refinery_and_red_over_sand() {
        let mut game = in_battle();
        game.fog = None;
        let terrain = battlefield::terrain(game.battle_map);
        let at = (game.units[0].x + 633.6, game.units[0].z + 633.6);
        let refinery =
            units::new_building(terrain, at, Role::Refinery, (0, game.units[0].color), 9_999);
        game.units.push(refinery);
        let refinery = game.units.len() - 1;
        let harvester = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap();
        game.units[harvester].role = Role::Harvester;
        game.units[harvester].load = 50.0;
        let at = game.units[refinery];
        game.camera.look_at(at.x, at.z);
        let (x, y) = game.view().project(at.x, at.pick_height(), at.z).unwrap();
        game.handle(Event::MouseMove(x, y));
        assert_eq!(game.cursor_look(), CursorLook::Arrow, "nothing selected");
        game.units[harvester].selected = true;
        assert_eq!(game.cursor_look(), CursorLook::Land);
        click_at(&mut game, x, y);
        assert!(game.units[harvester].selected, "the refinery is not picked");
        assert!(matches!(
            game.units[harvester].harvest,
            units::Harvest::ToPad(units::Dock {
                refinery: 9_999,
                ..
            })
        ));

        // Somewhere on the sand on screen, with nobody there.
        let (width, height) = (game.width, game.height);
        let sand = (0..40)
            .flat_map(|i| (0..30).map(move |j| (i, j)))
            .map(|(i, j)| (width * i as f32 / 40.0, height * j as f32 / 30.0))
            .find(|&(x, y)| {
                game.handle(Event::MouseMove(x, y));
                game.cursor_on_battlefield()
                    && game.unit_under_cursor().is_none()
                    && game
                        .land_under_cursor()
                        .is_some_and(|at| units::has_substance(terrain, at))
            })
            .unwrap();
        game.handle(Event::MouseMove(sand.0, sand.1));
        assert_eq!(game.cursor_look(), CursorLook::Harvest);
        click_at(&mut game, sand.0, sand.1);
        assert_eq!(game.units[harvester].harvest, units::Harvest::Seeking);

        let tank = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::HkAssault)
            .unwrap();
        game.units[tank].selected = true;
        game.handle(Event::MouseMove(sand.0, sand.1));
        assert_eq!(
            game.cursor_look(),
            CursorLook::Move,
            "not with a tank along"
        );
    }

    #[test]
    fn an_outpost_shows_the_whole_map_and_goes_on_the_buildings_menu_of_every_house() {
        for house in [Faction::Atreides, Faction::Harkonnen, Faction::Ordos] {
            let game = in_battle_as(house);
            assert!(offers(&game, BuildTab::Buildings, Role::Outpost));
            assert!(Role::Outpost.made_by(house));
        }
        let mut game = in_battle();
        game.fog = Some(Fog::new());
        game.update_fog();
        let (x, z) = (game.units[0].x + 2715.0, game.units[0].z + 2715.0);
        assert_ne!(game.sight_at(x, z), Sight::Visible);
        assert!(!game.has_outpost());
        let at = (game.units[0].x, game.units[0].z);
        let outpost = units::new_building(
            battlefield::terrain(game.battle_map),
            at,
            Role::Outpost,
            (0, game.units[0].color),
            9_999,
        );
        game.units.push(outpost);
        let last = game.units.len() - 1;
        game.units[last].constructing = 0.0;
        assert!(game.has_outpost());
        game.update_fog();
        assert_eq!(game.sight_at(x, z), Sight::Visible);
        game.units[last].alive = false;
        game.update_fog();
        assert_eq!(
            game.sight_at(x, z),
            Sight::Explored,
            "the radar went with it"
        );
    }

    #[test]
    fn a_palaces_launcher_opens_as_the_superweapon_comes_ready() {
        let mut game = in_battle();
        give_palace(&mut game);
        let palace = *game.units.last().unwrap();
        game.strike_recharge = 60.0;
        assert_eq!(game.launcher_open(&palace), 0.0);
        game.strike_recharge = LAUNCHER_OPENS_SECONDS / 2.0;
        assert!(game.launcher_open(&palace) > 0.0 && game.launcher_open(&palace) < 1.0);
        game.strike_recharge = 0.0;
        assert_eq!(game.launcher_open(&palace), 1.0);
    }

    #[test]
    fn a_drag_on_the_minimap_still_moves_the_camera_with_units_selected_and_gives_no_order() {
        let mut game = in_battle();
        let tank = tank_of(&game);
        game.units[tank].selected = true;
        let stays = game.units[tank].destination;
        let (at, _) = minimap_point();
        let far = (at.0 + 40.0, at.1 + 20.0);
        game.handle(Event::MouseMove(at.0, at.1));
        game.handle(Event::MouseDown);
        let camera = (game.camera.x, game.camera.z);
        game.handle(Event::MouseMove(far.0, far.1));
        assert_ne!((game.camera.x, game.camera.z), camera);
        game.handle(Event::MouseUp);
        assert_eq!(game.units[tank].destination, stays);
    }

    #[test]
    fn a_click_on_the_minimap_moves_the_camera_with_only_a_tower_selected() {
        let mut game = in_battle();
        let tower = tower_of(&game, 0);
        game.units[tower].selected = true;
        let camera = (game.camera.x, game.camera.z);
        let (at, _) = minimap_point();
        click_at(&mut game, at.0, at.1);
        assert_ne!((game.camera.x, game.camera.z), camera);
    }

    #[test]
    fn clicking_the_minimap_sends_the_camera_there() {
        let mut game = in_battle();
        let rect = hud::minimap_rect(W, H);
        let (x, y, size) = rect;
        assert!(x + size <= W && x > W / 2.0 && y < H / 2.0, "top right");

        // The east corner of the diamond.
        click_at(&mut game, x + size - 1.0, y + size / 2.0);
        let expected =
            battlefield::minimap_to_world(rect, game.map_extent(), x + size - 1.0, y + size / 2.0)
                .unwrap();
        assert!(
            (game.camera.x - expected.0).abs() < 45.25,
            "went east: {}",
            game.camera.x
        );
        assert!(game.camera.x > Camera::centered().x);

        click_at(&mut game, x + size / 2.0, y + size / 2.0);
        assert!((game.camera.x - Camera::centered().x).abs() < 0.4525);
        assert!(
            (game.camera.z - Camera::centered().z).abs() < 0.4525,
            "the middle is the map's middle"
        );
    }

    #[test]
    fn dragging_on_the_minimap_moves_the_camera_until_release() {
        let mut game = in_battle();
        let rect = hud::minimap_rect(W, H);
        let (x, y, size) = rect;
        let at = |fx: f32, fy: f32| (x + size * fx, y + size * fy);
        let extent = game.map_extent();
        let world = |p: (f32, f32)| battlefield::minimap_to_world(rect, extent, p.0, p.1).unwrap();

        let start = at(0.5, 0.3);
        game.handle(Event::MouseMove(start.0, start.1));
        game.handle(Event::MouseDown);
        assert!(
            (game.camera.z - world(start).1).abs() < 0.4525,
            "moves on press"
        );

        let next = at(0.6, 0.45);
        game.handle(Event::MouseMove(next.0, next.1));
        assert!(
            (game.camera.x - world(next).0).abs() < 0.4525,
            "follows the drag"
        );
        assert!(
            (game.camera.z - world(next).1).abs() < 0.4525,
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
            battlefield::minimap_to_world(rect, game.map_extent(), x + size - 0.01, y + size / 2.0)
                .unwrap();
        assert!(
            (game.camera.x - east_x).abs() < 0.4525,
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
            (landed.x - before.x).abs() < 0.4525 && (landed.z - before.z).abs() < 0.4525,
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
    fn a_new_battle_brings_the_camera_back_to_the_players_mcv() {
        let mut game = in_battle();
        game.handle(Event::MouseMove(W - 1.0, H / 2.0));
        game.update(0.5);
        leave_battle(&mut game);
        click(&mut game, Action::SinglePlayer);
        click(&mut game, Action::Tutorial);
        game.update(LOADING_SECONDS);
        assert_eq!((game.camera.x, game.camera.z), game.home());
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

    /// A spot on screen where a building of this role can go up for the player now.
    fn placeable_on_screen(game: &Game, role: Role) -> Option<(f32, f32)> {
        let terrain = battlefield::terrain(game.battle_map);
        (1..100)
            .flat_map(|i| (1..60).map(move |j| (i as f32 * W / 100.0, j as f32 * H / 60.0)))
            .filter(|&(x, y)| !hud::covers(x, y, W, H))
            .find(|&(x, y)| {
                game.view().land_at(terrain, x, y).is_some_and(|at| {
                    let plot = role.plot_facing(game.faction_of(0), at, game.placing_facing);
                    units::can_place_building(&game.units, terrain, plot)
                        && units::is_near_building_of(&game.units, plot, 0)
                })
            })
    }

    /// Builds one from the Buildings menu as the player does: clicks it, waits, clicks it
    /// again and puts it down on screen.
    fn build_and_place(game: &mut Game, role: Role) {
        let cell = cell_of(game, BuildTab::Buildings, role);
        click_hud(game, Action::BuildTab(BuildTab::Buildings));
        click_hud(game, Action::BuildCell(cell));
        let ticks = ((role.build_seconds() + 1.0) * 10.0) as usize;
        for _ in 0..ticks {
            game.update(0.1);
        }
        assert_eq!(game.site, Site::Ready(role));
        click_hud(game, Action::BuildCell(cell));
        let (x, y) = placeable_on_screen(game, role).unwrap_or_else(|| panic!("room for {role:?}"));
        game.handle(Event::MouseMove(x, y));
        click_at(game, x, y);
        let built = game.units.last().unwrap();
        assert_eq!((built.role, built.owner), (role, 0));
        assert_eq!(game.site, Site::Idle);
    }

    #[test]
    fn allies_picked_in_the_setup_have_their_buildings_built_once_a_factory_and_barracks_stand() {
        let mut game = in_setup();
        click_widget(&mut game, Action::CycleAlly(0));
        click_widget(&mut game, Action::CycleAlly(1));
        click_widget(&mut game, Action::StartBattle);
        game.update(LOADING_SECONDS);
        assert_eq!(game.player_allies(), [Ally::Tleilaxu, Ally::Ix]);
        game.units[0].selected = true;
        game.toggle_deploy();
        for _ in 0..200 {
            game.update(0.1);
        }
        let mcv = game.units[0];
        assert!(mcv.deployed);
        game.camera.look_at(mcv.x, mcv.z);
        game.update_fog();
        let menu = |game: &Game| game.menu(BuildTab::Buildings);
        assert!(
            !menu(&game).contains(&Entry::Make(Role::TlFleshVat)),
            "not before a factory"
        );
        // Power first: the barracks and the factory need a windtrap standing.
        assert!(!menu(&game).contains(&Entry::Make(Role::Barracks)));
        // Two: one is too little for all four buildings, and short of power all goes slower.
        build_and_place(&mut game, Role::Windtrap);
        build_and_place(&mut game, Role::Windtrap);
        game.update(units::CONSTRUCTION_SECONDS + 0.1);
        build_and_place(&mut game, Role::Barracks);
        build_and_place(&mut game, Role::Factory);
        assert!(
            !menu(&game).contains(&Entry::Make(Role::TlFleshVat)),
            "not while the factory is still going up"
        );
        while game
            .units
            .iter()
            .any(|u| u.owner == 0 && u.is_under_construction())
        {
            game.update(0.1);
        }
        for role in [Role::TlFleshVat, Role::IxResCentre] {
            assert!(menu(&game).contains(&Entry::Make(role)), "{role:?}");
            build_and_place(&mut game, role);
        }
    }

    /// Puts the cursor over the middle of a grid square.
    fn point_at_square(game: &mut Game, (u, v): (i32, i32)) {
        let terrain = battlefield::terrain(game.battle_map);
        let (x, z) = battlefield::grid_to_world(u as f32 + 0.5, v as f32 + 0.5);
        let (sx, sy) = game.view().project(x, terrain.height_at(x, z), z).unwrap();
        game.handle(Event::MouseMove(sx, sy));
        assert_eq!(game.square_under_cursor(), Some((u, v)));
    }

    /// Clicks where the cursor is.
    fn click_here(game: &mut Game) {
        let (x, y) = game.mouse;
        click_at(game, x, y);
    }

    /// A square a wall can start on, with free rock for five more squares along +x, by one
    /// of the player's buildings up on rock.
    fn wall_start_square(game: &mut Game) -> (i32, i32) {
        buildable_spot_on_screen(game);
        let squares: Vec<(i32, i32)> = game
            .units
            .iter()
            .filter(|u| u.owner == 0)
            .filter_map(Unit::plot)
            .flat_map(|plot| {
                (plot.u - 6..plot.u + plot.cols + 6)
                    .flat_map(move |u| (plot.v - 6..plot.v + plot.rows + 6).map(move |v| (u, v)))
            })
            .collect();
        let start = squares
            .into_iter()
            .find(|&(u, v)| {
                game.can_start_wall((u, v)) && (1..6).all(|i| game.can_wall((u + i, v)))
            })
            .expect("room for a wall");
        game.camera.look_at(
            (start.0 as f32 + 3.0) * battlefield::CELL_SIDE,
            (start.1 as f32 + 0.5) * battlefield::CELL_SIDE,
        );
        game.update_fog();
        start
    }

    #[test]
    fn a_wall_is_placed_from_one_click_to_the_next_and_goes_up_a_square_at_a_time() {
        let mut game = in_battle();
        let cell = cell_of(&game, BuildTab::Buildings, Role::Wall);
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        let money = game.player_money();
        click_hud(&mut game, Action::BuildCell(cell));
        assert_eq!(
            game.site,
            Site::Placing(Role::Wall),
            "placed before it is built"
        );
        assert_eq!(game.player_money(), money, "nothing paid yet");
        let start = wall_start_square(&mut game);
        point_at_square(&mut game, start);
        assert_eq!(
            game.placing_squares(),
            [(start, PlaceSquare::Free)],
            "the cursor is one square"
        );
        click_here(&mut game);
        assert_eq!(game.wall_start, Some(start));

        let end = (start.0 + 4, start.1);
        point_at_square(&mut game, end);
        let shown: Vec<_> = game.placing_squares().into_iter().map(|(c, _)| c).collect();
        assert_eq!(
            shown,
            (0..5).map(|i| (start.0 + i, start.1)).collect::<Vec<_>>()
        );
        // A right click lets go of the start, and another of the wall.
        game.handle(Event::RightDown);
        assert_eq!(
            (game.site, game.wall_start),
            (Site::Placing(Role::Wall), None)
        );
        game.handle(Event::RightDown);
        assert_eq!(game.site, Site::Idle);

        click_hud(&mut game, Action::BuildCell(cell));
        point_at_square(&mut game, start);
        click_here(&mut game);
        point_at_square(&mut game, end);
        click_here(&mut game);
        assert_eq!(
            game.site,
            Site::Building(Role::Wall, Role::Wall.build_seconds())
        );
        assert_eq!(game.queue_badge(Role::Wall), Some(5));
        let walls = |game: &Game| -> Vec<Unit> {
            game.units
                .iter()
                .filter(|u| u.role == Role::Wall)
                .copied()
                .collect()
        };
        game.update(Role::Wall.build_seconds() * 0.5);
        assert!(walls(&game).is_empty(), "the first still going up");
        assert!(
            game.making_progress(Role::Wall)
                .is_some_and(|p| p > 0.4 && p < 0.6)
        );
        game.update(Role::Wall.build_seconds() * 0.5 + 0.01);
        assert_eq!(walls(&game).len(), 1, "one square at a time");
        for _ in 0..40 {
            game.update(0.25);
        }
        let built = walls(&game);
        assert_eq!(built.len(), 5);
        assert_eq!(game.site, Site::Idle);
        assert_eq!(game.player_money(), money - 5 * Role::Wall.cost());
        let pieces: Vec<_> = built
            .iter()
            .map(|w| units::WallPiece::of(w.wall_joins))
            .collect();
        use units::WallPiece::{End, Middle};
        assert_eq!(
            pieces,
            [(End, 0), (Middle, 0), (Middle, 0), (Middle, 0), (End, 2)]
        );
        assert!(
            built.iter().all(|w| !w.is_selectable()),
            "walls are never picked"
        );
        // Nobody stands on a wall's square, and walls alone do not keep a side in the battle.
        assert!(!units::cell_is_free(
            &game.units,
            battlefield::terrain(game.battle_map),
            start
        ));
        game.units.retain(|u| u.owner != 0 || u.role == Role::Wall);
        game.update(0.1);
        assert_eq!(game.outcome, Some(Outcome::Defeat));
    }

    #[test]
    fn a_wall_goes_round_a_building_in_its_way() {
        let mut game = in_battle();
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        let cell = cell_of(&game, BuildTab::Buildings, Role::Wall);
        click_hud(&mut game, Action::BuildCell(cell));
        let start = wall_start_square(&mut game);
        let end = (start.0 + 4, start.1);
        // A tower stands on the way between.
        let mut post = *game
            .units
            .iter()
            .find(|u| u.owner == 0 && u.kind == UnitKind::Tower)
            .unwrap();
        (post.x, post.z) = battlefield::grid_to_world(start.0 as f32 + 2.5, start.1 as f32 + 0.5);
        post.id = game.next_unit_id;
        game.next_unit_id += 1;
        let plot = post.plot().unwrap();
        game.units.push(post);
        game.wall_start = Some(start);
        game.camera.look_at(post.x, post.z);
        point_at_square(&mut game, end);
        let route = game.wall_route().expect("a way round");
        assert_eq!((route[0], *route.last().unwrap()), (start, end));
        let on_post = |(u, v): (i32, i32)| {
            (plot.u..plot.u + plot.cols).contains(&u) && (plot.v..plot.v + plot.rows).contains(&v)
        };
        assert!(route.iter().all(|&cell| !on_post(cell)), "{route:?}");
        assert!(route.len() > 5);
        let shown: Vec<_> = game.placing_squares().into_iter().map(|(c, _)| c).collect();
        assert_eq!(shown, route);
    }

    #[test]
    fn windtraps_make_the_power_buildings_use_and_short_of_it_turrets_stop_and_work_slows() {
        let mut game = in_battle();
        let (made, used) = game.player_power();
        assert_eq!(made, 400, "four windtraps");
        // The hangar, the factory, the barracks, the pad, the starport and the rocket turret.
        assert_eq!(used, 50 + 40 + 20 + 30 + 100 + 75);
        assert!(game.power_to_spare() > 0.0 && game.power_to_spare() < 1.0);
        game.update(0.1);
        let turret = |game: &Game| {
            *game
                .units
                .iter()
                .find(|u| u.owner == 0 && u.role == Role::AtRocketTurret)
                .unwrap()
        };
        assert!(!turret(&game).unpowered);

        // Three windtraps gone: short of power.
        let mut left = 3;
        game.units.retain(|u| {
            let goes = u.owner == 0 && u.role == Role::Windtrap && left > 0;
            left -= usize::from(goes);
            !goes
        });
        assert!(units::is_short_of_power(&game.units, 0));
        assert_eq!(game.power_to_spare(), 0.0);
        game.update(0.1);
        assert!(turret(&game).unpowered, "the advanced turret stops");
        assert!(
            game.units
                .iter()
                .filter(|u| u.owner == 0 && u.role == Role::HkFlameTurret)
                .all(|u| !u.unpowered),
            "the lesser one needs none"
        );
        // And what is built goes up at half the pace.
        let cell = cell_of(&game, BuildTab::Buildings, Role::Windtrap);
        click_hud(&mut game, Action::BuildTab(BuildTab::Buildings));
        click_hud(&mut game, Action::BuildCell(cell));
        game.update(Role::Windtrap.build_seconds() + 0.1);
        assert!(matches!(game.site, Site::Building(Role::Windtrap, _)));
        game.update(Role::Windtrap.build_seconds());
        assert_eq!(game.site, Site::Ready(Role::Windtrap));

        // With none at all, what needs a windtrap is not on offer.
        game.units
            .retain(|u| u.owner != 0 || u.role != Role::Windtrap);
        for role in [Role::Barracks, Role::Refinery, Role::Factory, Role::Outpost] {
            assert_eq!(role.needs(), Some(Role::Windtrap));
            assert!(!game.can_produce(role), "{role:?}");
        }
    }

    #[test]
    fn buildings_are_upgraded_from_the_upgrades_tab_and_what_needs_it_waits_for_it() {
        let mut game = in_battle();
        // Nothing left to upgrade on a base upgraded all the way.
        assert!(!game.tab_enabled(BuildTab::Upgrades));
        for unit in game.units.iter_mut().filter(|u| u.owner == 0) {
            unit.upgrades = 0;
        }
        assert_eq!(
            game.menu(BuildTab::Upgrades),
            [Role::Barracks, Role::Factory, Role::Hanger, Role::Mcv].map(Entry::Upgrade),
            "no refinery to give a pad to"
        );
        for role in [
            Role::AtKindjal,
            Role::Engineer,
            Role::AtMinotaurus,
            Role::AtOrni,
        ] {
            assert!(role.needs_upgraded_maker());
            assert!(!game.can_produce(role), "{role:?} waits for the upgrade");
        }
        assert!(!game.can_produce(Role::AtRocketTurret), "the yard's");
        assert!(game.can_produce(Role::AtInfantry) && game.can_produce(Role::AtPillbox));

        click_hud(&mut game, Action::HudWide(hud::UPGRADE_BUTTON));
        let money = game.player_money();
        click_hud(&mut game, Action::BuildCell(0));
        game.update(Role::Barracks.upgrade_seconds() / 2.0);
        let progress = game.upgrade_progress(Role::Barracks).unwrap();
        assert!(progress > 0.4 && progress < 0.6, "{progress}");
        assert!(!game.can_produce(Role::AtKindjal), "not yet");
        // A right click calls it off and pays back.
        game.handle(Event::RightDown);
        assert_eq!(game.upgrade_progress(Role::Barracks), None);
        assert_eq!(game.player_money(), money);

        click_hud(&mut game, Action::BuildCell(0));
        game.update(Role::Barracks.upgrade_seconds() + 0.1);
        assert_eq!(
            game.player_money(),
            money - 800,
            "the original's UpgradeCost"
        );
        assert!(game.can_produce(Role::AtKindjal) && game.can_produce(Role::Engineer));
        assert!(!game.can_produce(Role::AtMinotaurus), "the factory is not");
        assert!(
            !game
                .menu(BuildTab::Upgrades)
                .contains(&Entry::Upgrade(Role::Barracks)),
            "once is all"
        );
        assert_eq!(Role::Mcv.upgrade_cost(), Some(600));
        assert_eq!(Role::Factory.upgrade_cost(), Some(1200));
        assert_eq!(Role::Wall.upgrade_cost(), None);
    }

    #[test]
    fn a_refinery_gains_a_pad_and_a_harvester_with_each_of_its_two_upgrades() {
        let mut game = in_battle();
        let mut refinery = *game
            .units
            .iter()
            .find(|u| u.owner == 0 && u.role == Role::Barracks)
            .unwrap();
        refinery.role = Role::Refinery;
        refinery.upgrades = 0;
        refinery.id = game.next_unit_id;
        game.next_unit_id += 1;
        game.refineries_served.push(refinery.id);
        game.units.push(refinery);
        let at = game.units.len() - 1;
        assert_eq!(game.units[at].working_pads().collect::<Vec<_>>(), [1]);
        let harvesters = |game: &Game| {
            game.units
                .iter()
                .filter(|u| u.owner == 0 && u.role == Role::Harvester)
                .count()
        };
        let before = harvesters(&game);
        let (_, used) = game.player_power();
        click_hud(&mut game, Action::HudWide(hud::UPGRADE_BUTTON));
        let money = game.player_money();
        for pads in [2, 3] {
            let cell = game
                .menu(BuildTab::Upgrades)
                .iter()
                .position(|&entry| entry == Entry::Upgrade(Role::Refinery))
                .expect("a pad on offer");
            click_hud(&mut game, Action::BuildCell(cell));
            game.update(Role::Refinery.upgrade_seconds() + 0.1);
            assert_eq!(game.units[at].working_pads().count(), pads);
        }
        assert_eq!(game.units[at].working_pads().collect::<Vec<_>>(), [1, 0, 2]);
        assert_eq!(game.player_money(), money - 2 * 1200);
        assert_eq!(
            harvesters(&game),
            before + 2,
            "each pad brings its harvester"
        );
        assert_eq!(game.player_power().1, used + 2 * 20, "and uses its power");
        assert!(!game.can_upgrade(Role::Refinery), "three pads are all");
    }

    #[test]
    fn the_five_small_buttons_repair_sell_lay_routes_retreat_and_guard_as_the_originals() {
        let labels: Vec<&str> = hud::command_buttons(W, H).iter().map(|b| b.label).collect();
        assert_eq!(
            labels,
            ["Fix", "$", "Way", "Out", "Grd", "Upgrade", "Starport"]
        );
        let mut game = in_battle();

        // Repair: a click on a hurt building mends it, paid for as it goes.
        let barracks = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Barracks)
            .unwrap();
        let whole = game.units[barracks].max_health();
        game.units[barracks].health = whole / 2.0;
        click_hud(&mut game, Action::HudSmall(hud::REPAIR_BUTTON));
        assert_eq!(game.click_mode, Some(ClickMode::Repair));
        assert!(game.hud_small_lit(hud::REPAIR_BUTTON));
        let (x, z, y) = (
            game.units[barracks].x,
            game.units[barracks].z,
            game.units[barracks].y,
        );
        game.camera.look_at(x, z);
        let (sx, sy) = game.view().project(x, y, z).unwrap();
        let money = game.player_money();
        click_at(&mut game, sx, sy);
        assert!(game.units[barracks].repairing);
        game.update(5.0);
        let mended = game.units[barracks].health;
        assert!(mended > whole / 2.0 && mended < whole, "{mended}");
        assert!(game.player_money() < money, "it costs");
        for _ in 0..40 {
            game.update(0.5);
        }
        assert_eq!(game.units[barracks].health, whole);
        assert!(!game.units[barracks].repairing, "done");
        let cost = money - game.player_money();
        assert!(
            (cost - Role::Barracks.cost() / 4).abs() <= 2,
            "half of half its price: {cost}"
        );
        click_hud(&mut game, Action::HudSmall(hud::REPAIR_BUTTON));
        assert_eq!(game.click_mode, None, "and off again");

        // The wide Upgrade button opens the Upgrades menu, when there is anything in it.
        for unit in game.units.iter_mut().filter(|u| u.owner == 0) {
            unit.upgrades = 0;
        }
        click_hud(&mut game, Action::HudWide(hud::UPGRADE_BUTTON));
        assert_eq!(game.build_tab, BuildTab::Upgrades);

        // Retreat asks first, and gives the battle up on the second click.
        click_hud(&mut game, Action::HudSmall(hud::RETREAT_BUTTON));
        assert_eq!(game.outcome, None);
        assert!(game.hud_small_lit(hud::RETREAT_BUTTON));
        game.update(RETREAT_CONFIRM_SECONDS + 0.5);
        assert!(!game.hud_small_lit(hud::RETREAT_BUTTON), "it stops waiting");
        click_hud(&mut game, Action::HudSmall(hud::RETREAT_BUTTON));
        click_hud(&mut game, Action::HudSmall(hud::RETREAT_BUTTON));
        assert_eq!(game.outcome, Some(Outcome::Defeat));
    }

    #[test]
    fn a_route_of_waypoints_is_followed_one_after_another_and_guards_come_back_to_their_post() {
        let mut game = in_battle();
        let rifleman = cell_of(&game, BuildTab::Infantry, Role::AtInfantry);
        let soldier = select_new(&mut game, rifleman);
        let id = game.units[soldier].id;
        let (x, z) = (game.units[soldier].x, game.units[soldier].z);
        let first = (x + 150.0, z);
        let second = (x + 150.0, z + 150.0);
        game.camera.look_at(x + 80.0, z + 80.0);
        game.update_fog();
        click_hud(&mut game, Action::HudSmall(hud::WAYPOINT_BUTTON));
        let terrain = battlefield::terrain(game.battle_map);
        for (wx, wz) in [first, second] {
            let (sx, sy) = game
                .view()
                .project(wx, terrain.height_at(wx, wz), wz)
                .unwrap();
            click_at(&mut game, sx, sy);
        }
        assert_eq!(game.click_mode, Some(ClickMode::Route), "it stays down");
        assert_eq!(game.waypoints().len(), 1, "one still ahead");
        let near = |game: &Game, (px, pz): (f32, f32)| {
            let unit = game.units.iter().find(|u| u.id == id).unwrap();
            (unit.x - px).hypot(unit.z - pz) < 60.0
        };
        let mut reached_first = false;
        for _ in 0..400 {
            game.update(0.1);
            reached_first |= near(&game, first);
        }
        assert!(reached_first, "by the first waypoint");
        assert!(near(&game, second), "and on to the second");
        assert!(game.waypoints().is_empty() && game.routes.is_empty());

        // Guard mode: on for the selected, lit, and off again.
        click_hud(&mut game, Action::HudSmall(hud::WAYPOINT_BUTTON));
        click_hud(&mut game, Action::HudSmall(hud::GUARD_BUTTON));
        let post = game.units.iter().find(|u| u.id == id).unwrap().guard;
        assert!(post.is_some() && game.hud_small_lit(hud::GUARD_BUTTON));
        click_hud(&mut game, Action::HudSmall(hud::GUARD_BUTTON));
        assert!(
            game.units
                .iter()
                .find(|u| u.id == id)
                .unwrap()
                .guard
                .is_none()
        );
    }

    #[test]
    fn a_starport_sells_at_moving_prices_what_it_has_and_a_frigate_brings_the_order() {
        let mut game = in_battle();
        assert_eq!(Role::Starport.needs(), Some(Role::Hanger));
        assert_eq!(
            Role::Palace(Faction::Atreides).needs(),
            Some(Role::Starport)
        );
        assert!(game.has_starport());
        click_hud(&mut game, Action::HudWide(hud::STARPORT_BUTTON));
        assert_eq!(game.build_tab, BuildTab::Starport);
        let menu = game.menu(BuildTab::Starport);
        assert_eq!(
            menu[0],
            Entry::Order(Role::AtTrike),
            "the house's own, as the original lists"
        );
        assert_eq!(
            menu[menu.len() - 2..],
            [Entry::Purchase, Entry::CancelOrder]
        );
        assert!(!menu.contains(&Entry::Order(Role::HkDevastator)));
        for offer in &game.market().offers {
            let cost = offer.role.cost() as f32;
            let (low, high) = (cost * 0.6 - 10.0, cost * 1.4 + 10.0);
            assert!((low..=high).contains(&(offer.price as f32)), "{offer:?}");
            assert!(offer.stock <= STARPORT_STARTING_STOCK);
        }
        // Prices move every minute; stock comes in.
        let before: Vec<i32> = game.market().offers.iter().map(|o| o.price).collect();
        game.market.offers[0].stock = 2;
        game.update(STARPORT_PRICE_SECONDS + 1.0);
        let after: Vec<i32> = game.market().offers.iter().map(|o| o.price).collect();
        assert_ne!(before, after);
        assert!(game.market().offers[0].stock >= 2);

        // Only what is in stock goes on the order, and a right click takes one off.
        game.market.offers[0].stock = 2;
        for _ in 0..3 {
            click_hud(&mut game, Action::BuildCell(0));
        }
        assert_eq!(game.market().ordered(Role::AtTrike), 2);
        let cell = hud::grid_cells(W, H)[0];
        game.handle(Event::MouseMove(cell.x + 1.0, cell.y + 1.0));
        game.handle(Event::RightDown);
        assert_eq!(game.market().ordered(Role::AtTrike), 1);
        let price = game.market().offers[0].price;
        assert_eq!(game.market().total(), price);

        // Paid at once; the frigate comes after its countdown and sets the trike down.
        let trikes = |game: &Game| {
            game.units
                .iter()
                .filter(|u| u.owner == 0 && u.role == Role::AtTrike)
                .count()
        };
        let (money, had) = (game.player_money(), trikes(&game));
        let purchase = menu.len() - 2;
        click_hud(&mut game, Action::BuildCell(purchase));
        assert_eq!(game.player_money(), money - price);
        assert!(game.market().order.is_empty() && game.market().frigate.is_some());
        assert_eq!(game.market().offers[0].stock, 1);
        click_hud(&mut game, Action::BuildCell(0));
        assert_eq!(
            game.market().ordered(Role::AtTrike),
            0,
            "not while a frigate is coming"
        );
        game.update(FRIGATE_SECONDS - 5.0);
        assert_eq!(trikes(&game), had, "not yet");
        game.update(6.0);
        assert!(game.market().frigate.is_none());
        assert_eq!(trikes(&game), had + 1, "set down by the starport");

        // No more than six in a delivery.
        for offer in &mut game.market.offers {
            offer.stock = STARPORT_MOST_STOCK;
        }
        for cell in [0, 1] {
            for _ in 0..5 {
                click_hud(&mut game, Action::BuildCell(cell));
            }
        }
        assert_eq!(game.market().order.len(), STARPORT_MOST_IN_A_DELIVERY);
        click_hud(&mut game, Action::BuildCell(purchase + 1));
        assert!(game.market().order.is_empty(), "cancelled");
    }

    #[test]
    fn a_campaign_opens_with_its_houses_movie_then_its_first_battle_with_ten_thousand() {
        let mut game = in_setup();
        game.set_screen(Screen::FactionSelect);
        game.confirming = Some(Faction::Atreides);
        game.activate(Action::ConfirmFaction);
        assert_eq!(game.screen, Screen::Opening(Faction::Atreides));
        // The movie plays as long as it lasts, and a key skips it.
        game.start_movie();
        game.update(5.0);
        assert_eq!(game.screen, Screen::Opening(Faction::Atreides));
        game.handle(Event::KeyDown(Key::Escape));
        assert_eq!(game.screen, Screen::Loading);
        assert_eq!(game.battle_kind, BattleKind::Campaign);
        assert_eq!(game.faction_of(0), Faction::Atreides);
        assert_eq!(
            game.faction_of(1),
            Faction::Harkonnen,
            "the Atreides' first enemy"
        );
        assert_eq!(game.player_money(), CAMPAIGN_START_MONEY);
        // Without the house's disc it goes straight on.
        let mut game = in_setup();
        game.confirming = Some(Faction::Ordos);
        game.activate(Action::ConfirmFaction);
        game.update(OPENING_WAIT_SECONDS + 0.05);
        assert_eq!(game.screen, Screen::Loading);
        assert_eq!(game.faction_of(0), Faction::Ordos);
    }

    #[test]
    fn the_battlefield_says_what_happens_and_does_not_repeat_itself() {
        let mut game = in_battle();
        game.update(0.1);
        game.take_speeches();
        let rifleman = cell_of(&game, BuildTab::Infantry, Role::AtInfantry);
        click_hud(&mut game, Action::BuildCell(rifleman));
        assert_eq!(game.take_speeches()[0], Speech::Training);
        game.update(Role::AtInfantry.build_seconds() + 0.5);
        game.take_speeches();

        // A building hit by the enemy: the base is under attack, once in a while.
        let barracks = game
            .units
            .iter()
            .position(|u| u.owner == 0 && u.role == Role::Barracks)
            .unwrap();
        for _ in 0..3 {
            game.units[barracks].health -= 1.0;
            game.update(0.1);
        }
        let said = game.take_speeches();
        let warnings = said
            .iter()
            .filter(|&&s| s == Speech::BaseUnderAttack)
            .count();
        assert_eq!(warnings, 1, "{said:?}");
        for _ in 0..(Speech::BaseUnderAttack.quiet_for() * 2.0) as usize {
            game.update(0.5);
        }
        game.units[barracks].health -= 1.0;
        game.update(0.1);
        assert!(
            game.take_speeches().contains(&Speech::BaseUnderAttack),
            "again later"
        );

        // Losing it, and running short of power.
        game.units
            .retain(|u| u.owner != 0 || u.role != Role::Windtrap);
        game.units[barracks].alive = false;
        game.update(0.1);
        let said = game.take_speeches();
        assert!(said.contains(&Speech::BuildingLost) && said.contains(&Speech::LowPower));
    }

    #[test]
    fn a_deployed_mcvs_sweeping_searchlight_lights_the_ground_where_it_points() {
        let mut game = in_battle();
        game.fog = None;
        let mcv = *game
            .units
            .iter()
            .find(|u| u.owner == 0 && u.role == Role::Mcv)
            .unwrap();
        game.camera.look_at(mcv.x, mcv.z);
        let frame = crate::render::build_frame(&game);
        let pale = frame.lights.iter().find(|light| {
            let [r, g, b] = light.color;
            (r - g).abs() < 0.01
                && b > 0.4
                && (light.place[0] - mcv.x).hypot(light.place[2] - mcv.z) > 30.0
        });
        assert!(pale.is_some(), "{:?}", frame.lights);
    }
}
