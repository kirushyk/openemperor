//! The original's mission scripts, its `.tok` files: tokenised source, a line a statement.
//! `docs/tok-format.md` describes them.

/// How long the head of a script is: the size of what follows, and how many lines.
const HEAD: usize = 8;
/// What a token starts with, by what it is.
const WORD: u8 = 0x80;
const VARIABLE: u8 = 0x81;
const KIND: u8 = 0x82;
const SPEECH: u8 = 0x83;
/// From this number up a word is a keyword or an operator; below it, a function.
const FIRST_KEYWORD: u16 = 128;
/// What the number of a kind of unit or building is more than its place in the rules' lists.
const FIRST_KIND: u16 = 111;

/// One piece of a statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// A function, by its number.
    Function(u16),
    /// A keyword or an operator, by its number.
    Keyword(u16),
    /// A variable, by its number.
    Variable(u16),
    /// A kind of unit or building, by its place among the entries of the rules'
    /// `[BuildingTypes]` and `[UnitTypes]` lists in the order of the file.
    Kind(u16),
    /// A line of speech, by its number.
    Speech(u16),
    /// What is written as it is: digits, commas, brackets.
    Text(String),
}

/// Reads a script into its lines, each the tokens of one statement: empty for a blank line
/// of the source. `None` if it is not a script.
pub fn read(script: &[u8]) -> Option<Vec<Vec<Token>>> {
    let field = |at: usize| {
        let bytes = script.get(at..at + 4)?;
        Some(u32::from_le_bytes(bytes.try_into().ok()?) as usize)
    };
    let (size, lines) = (field(0)?, field(4)?);
    let body = script.get(HEAD..HEAD + size)?;
    let read: Vec<Vec<Token>> = body.split(|&byte| byte == 0).map(tokens).collect();
    // The last line's zero leaves an empty piece after it.
    (read.len() == lines + 1).then(|| read[..lines].to_vec())
}

/// The tokens of one line.
fn tokens(line: &[u8]) -> Vec<Token> {
    // A number kept seven bits to a byte, lowest first, in bytes with their top bit set.
    let number = |bytes: &[u8]| {
        bytes
            .iter()
            .rev()
            .fold(0, |sum, &byte| sum << 7 | u16::from(byte & 0x7f))
    };
    let mut read = Vec::new();
    let mut at = 0;
    while at < line.len() {
        let long = match line[at] {
            WORD | KIND | SPEECH => 2,
            VARIABLE => 1,
            _ => 0,
        };
        let marked = line
            .get(at + 1..at + 1 + long)
            .filter(|bytes| long > 0 && bytes.iter().all(|&byte| byte >= 0x80));
        match marked {
            Some(bytes) => {
                let n = number(bytes);
                read.push(match line[at] {
                    WORD if n < FIRST_KEYWORD => Token::Function(n),
                    WORD => Token::Keyword(n),
                    VARIABLE => Token::Variable(n),
                    KIND => Token::Kind(n.saturating_sub(FIRST_KIND)),
                    _ => Token::Speech(n),
                });
                at += 1 + long;
            }
            None => {
                let end = line[at + 1..]
                    .iter()
                    .position(|&byte| byte >= 0x80)
                    .map_or(line.len(), |far| at + 1 + far);
                read.push(Token::Text(
                    String::from_utf8_lossy(&line[at..end]).into_owned(),
                ));
                at = end;
            }
        }
    }
    read
}

/// The lines of speech a script plays, in the order it names them, each once.
#[cfg(test)]
pub fn speeches(lines: &[Vec<Token>]) -> Vec<u16> {
    let mut named = Vec::new();
    for token in lines.iter().flatten() {
        if let Token::Speech(line) = token
            && !named.contains(line)
        {
            named.push(*line);
        }
    }
    named
}

/// The functions and keywords a script is run with, by their numbers, each named as the
/// original's table names it (`docs/tok-format.md` lists them all, with where GAME.EXE carries
/// each out). What we take one to do, where its name does not say it, is with it.
const MODEL_TICK: u16 = 0;
/// A whole number from 0 to one less than the number given.
const RANDOM: u16 = 1;
/// Whether the game is the co-op campaign, which it never is here: the start scripts give the
/// second player of one a gift of units only then.
const MULTIPLAYER: u16 = 2;
const GET_UNUSED_BASE_POINT: u16 = 3;
const GET_SIDE_BASE_POINT: u16 = 4;
const GET_SCRIPT_POINT: u16 = 5;
/// The map's entrance nearest a side's base, and the one farthest from that.
const GET_ENTRANCE_POINT: u16 = 6;
const GET_EXIT_POINT: u16 = 7;
/// Where reinforcements come in: given to the player's mostly.
const GET_NEUTRAL_ENTRANCE_POINT: u16 = 8;
/// The map's entrance of a number, the first for one it has not; the one nearest a place,
/// and the one farthest from it.
const GET_ENTRANCE_POINT_BY_INDEX: u16 = 9;
const GET_ENTRANCE_NEAR_TO_POS: u16 = 10;
const GET_ENTRANCE_FAR_FROM_POS: u16 = 11;
/// Where a side's units arrive, its base.
const GET_SIDE_POSITION: u16 = 12;
const GET_OBJECT_POSITION: u16 = 13;
const GET_PLAYER_SIDE: u16 = 14;
/// The co-op campaign's other human player's side; outside one, the player's.
const GET_SECOND_PLAYER_SIDE: u16 = 15;
/// The first computer side: taken as a side that is neither the player's nor one the script
/// makes, what the scripts make friends and enemies of the others with, as if the world at
/// large.
const GET_ENEMY_SIDE: u16 = 16;
/// Whose a thing is: the player's, or the script's side it was put down for.
const GET_OBJECT_SIDE: u16 = 17;
const CREATE_SIDE: u16 = 18;
/// A side's money; its spice is the same figure.
const GET_SIDE_CASH: u16 = 19;
const GET_SIDE_SPICE: u16 = 20;
const NEW_OBJECT: u16 = 21;
/// Whether a thing still stands.
const OBJECT_VALID: u16 = 22;
const OBJECT_DESTROYED: u16 = 23;
/// Whether a side has a unit at a thing: the tutorial's beacons reached.
const OBJECT_NEAR_TO_SIDE: u16 = 24;
/// Whether a side sees a thing: taken as whether it has hurt it, the outpost of a camp fired
/// on.
const OBJECT_VISIBLE_TO_SIDE: u16 = 27;
const OBJECT_GET_HEALTH: u16 = 29;
const OBJECT_MAX_HEALTH: u16 = 30;
/// Whether a side's things are near another's: taken as in sight of them.
const SIDE_NEAR_TO_SIDE: u16 = 32;
const SIDE_UNIT_COUNT: u16 = 35;
const SIDE_BUILDING_COUNT: u16 = 36;
/// How many of a side's things of a kind there are.
const SIDE_OBJECT_COUNT: u16 = 37;
/// Whether a side's units have got where they were sent.
const SIDE_AI_DONE: u16 = 38;
/// Whether a side attacked another this update: taken as whether it has lost any of what the
/// script gave it; the tutorial asks it of the player and its enemy's wave, to tell a wave
/// fought from one only the worm met.
const EVENT_SIDE_ATTACKS_SIDE: u16 = 42;
/// Whether a thing is destroyed, as `OBJECT_DESTROYED`.
const EVENT_OBJECT_DESTROYED: u16 = 44;
/// Plays a line of speech.
const MESSAGE: u16 = 45;
/// Sets a thing's health, in the original's.
const OBJECT_SET_HEALTH: u16 = 54;
/// Turns a thing into another kind, for a side: prisoners freed become Fremen.
const OBJECT_CHANGE: u16 = 57;
/// Makes one side a friend of another, or an enemy.
const SIDE_FRIEND_TO: u16 = 59;
const SIDE_ENEMY_TO: u16 = 60;
/// Gives a side money, on top of what it has.
const ADD_SIDE_CASH: u16 = 62;
/// A side's units go after the player's.
const SIDE_AI_AGGRESSIVE: u16 = 64;
/// A side's units are sent to a place.
const SIDE_AI_MOVE: u16 = 71;
/// Ends the battle, won if given true and lost if false.
const MISSION_OUTCOME: u16 = 83;
/// The tutorial is over, and done.
const END_GAME_WIN: u16 = 84;
/// The battle is lost.
const END_GAME_LOSE: u16 = 85;
/// Hands all a side's units over to another side, as the Atreides' Heighliner's Mongooses
/// join the player once found.
const SIDE_CHANGE_SIDE: u16 = 91;
/// Destroys everything a side has, and everything every side has.
const SIDE_NUKE: u16 = 93;
const SIDE_NUKE_ALL: u16 = 94;
/// Uncovers the fog so many squares round a place, as the tutorial does where it shows a
/// beacon and the enemy's base.
const REMOVE_SHROUD: u16 = 97;
/// Covers the map again so many squares round a place, as if never seen; and opens up the
/// whole map.
const REPLACE_SHROUD: u16 = 98;
const REMOVE_MAP_SHROUD: u16 = 99;
/// The camera's part in a scene: sent to look at a place, glided to one over so many updates,
/// circled about it at so many degrees an update one way or the other, stopped circling,
/// made to follow a thing and stop following it, kept, glided back to where it was kept.
const CAMERA_LOOK_AT_POINT: u16 = 100;
const CAMERA_PAN_TO_POINT: u16 = 101;
const CAMERA_START_ROTATE: u16 = 105;
const CAMERA_STOP_ROTATE: u16 = 106;
const CAMERA_TRACK_OBJECT: u16 = 107;
const CAMERA_STOP_TRACK: u16 = 108;
const CAMERA_STORE: u16 = 112;
const CAMERA_RESTORE: u16 = 113;
/// The picture-in-picture camera's: taken as the main camera's.
const PIP_CAMERA_LOOK_AT_POINT: u16 = 114;
const PIP_CAMERA_PAN_TO_POINT: u16 = 115;
/// A scene begins, the player's controls taken away, and ends.
const DISABLE_UI: u16 = 131;
const ENABLE_UI: u16 = 132;
/// Sells a building: taken as taking a thing off the map.
const OBJECT_SELL: u16 = 135;
const NEW_OBJECT_OFFSET_ORIENTATION: u16 = 139;
/// The last of the original's sixteen sides, kept for what is nobody's.
const GET_NEUTRAL_SIDE: u16 = 140;
const NEG: u16 = 143;
/// Sets a side's money, but only at the very start of the battle.
const SET_SIDE_CASH: u16 = 152;
/// Does nothing but write a line to the original's debugger.
const BREAK_POINT: u16 = 160;
/// The place of a square of the map, by its column and row: its corner nearest the map's
/// first, as the original's code gives it, though it works out the square's middle too.
const SET_TILE_POS: u16 = 158;
/// A step of the tutorial, as the number it is.
const SET_VALUE: u16 = 144;
const IF: u16 = 165;
const ELSE: u16 = 166;
const ENDIF: u16 = 167;
/// The comparisons, in the order of their numbers: `==`, `!=`, `>=`, `<=`, `>` and `<`. The
/// tutorial goes on to its next step at a count of exactly the one it is at (`<=`), a capture
/// is asked of whose a building is not (`!=`), and a time before one or after another (`<`,
/// `>`).
const EQUAL: u16 = 168;
const NOT_EQUAL: u16 = 169;
const AT_LEAST: u16 = 170;
const AT_MOST: u16 = 171;
const MORE: u16 = 172;
const LESS: u16 = 173;
const AND: u16 = 174;
const OR: u16 = 175;
const FALSE: u16 = 176;
const TRUE: u16 = 177;
const PLUS: u16 = 178;
const SET: u16 = 180;
/// What `GET_ENEMY_SIDE` is as a side.
pub const SOMEONE_SIDE: i32 = -1;
/// What `GET_NEUTRAL_SIDE` is: the last of sixteen.
const NEUTRAL_SIDE: i32 = 15;

/// A value of a script: a number, truth being 1 and falsehood 0; a place on the map; or a thing
/// on it, if one was made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Number(i32),
    Place(f32, f32),
    Thing(Option<u32>),
}

impl Value {
    fn number(self) -> i32 {
        match self {
            Value::Number(n) => n,
            Value::Place(..) => 0,
            Value::Thing(thing) => i32::from(thing.is_some()),
        }
    }

    fn place(self) -> (f32, f32) {
        match self {
            Value::Place(x, z) => (x, z),
            _ => (0.0, 0.0),
        }
    }

    fn thing(self) -> Option<u32> {
        match self {
            Value::Thing(thing) => thing,
            _ => None,
        }
    }
}

/// What a script has the camera do, its times in the script's updates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shot {
    Keep,
    LookAt((f32, f32)),
    GlideTo((f32, f32), i32),
    /// Degrees an update, and which way: 1 one, 2 the other.
    Circle(i32, i32),
    StopCircling,
    GlideBack(i32),
    /// Stays on a thing wherever it goes, or on what carries it.
    Follow(u32),
    StopFollowing,
}

/// What a script asks of the battle it runs in. Its sides are numbers: the player's 0, and
/// each new one the next.
pub trait Battle {
    /// How many of the script's updates have gone by.
    fn now(&self) -> i32;
    fn new_side(&mut self) -> i32;
    /// A number drawn at random, the next of a repeatable run.
    fn random(&mut self) -> u32 {
        0
    }
    /// Where a side's base is, and where the enemy's main base is.
    fn base(&self, side: i32) -> (f32, f32);
    fn enemy_base(&self) -> (f32, f32);
    /// The map's point for the script of this number.
    fn script_point(&self, number: i32) -> (f32, f32);
    /// Where things come onto the map and leave it, in the order of its file.
    fn entrances(&self) -> Vec<(f32, f32)> {
        Vec::new()
    }
    /// Puts a thing of a kind of the rules' lists on the map for a side, facing one of four
    /// ways, and says which it is.
    fn place(&mut self, side: i32, kind: u16, at: (f32, f32), facing: i32) -> Option<u32>;
    /// Brings a unit of a kind in for a side, at a place.
    fn bring(&mut self, side: i32, kind: u16, at: (f32, f32)) -> Option<u32>;
    /// How many units a side has left.
    fn left(&self, side: i32) -> i32;
    fn destroyed(&self, thing: Option<u32>) -> bool;
    /// Plays a line of speech, by its number.
    fn say(&mut self, line: u16);
    /// Gives a side money, on top of what it has.
    fn pay(&mut self, side: i32, money: i32);
    /// How much money a side has.
    fn cash(&self, _side: i32) -> i32 {
        0
    }
    /// Sets how much money a side has.
    fn set_cash(&mut self, _side: i32, _money: i32) {}
    /// The player has won.
    fn win(&mut self);
    /// The player has lost.
    fn lose(&mut self) {}
    /// How many things of a kind a side has standing.
    fn count(&self, _side: i32, _kind: u16) -> i32 {
        0
    }
    /// Where a thing is, if it is still there.
    fn place_of(&self, _thing: Option<u32>) -> Option<(f32, f32)> {
        None
    }
    /// Whether any of `seen`'s units are in sight of any of `by`'s.
    fn sees(&self, _seen: i32, _by: i32) -> bool {
        false
    }
    fn set_health(&mut self, _thing: Option<u32>, _health: i32) {}
    /// Turns a thing into one of another kind for a side, in its place, and says which it is.
    fn turn_into(&mut self, thing: Option<u32>, _kind: u16, _side: i32) -> Option<u32> {
        thing
    }
    /// Makes side `a` a friend of `b`, or an enemy.
    fn relate(&mut self, _a: i32, _b: i32, _enemies: bool) {}
    /// Whose a thing is, as a side, if it stands.
    fn owner_side(&self, _thing: Option<u32>) -> Option<i32> {
        None
    }
    /// Whether a thing has been hurt by a side.
    fn hurt_by(&self, _thing: Option<u32>, _side: i32) -> bool {
        false
    }
    /// A thing's health and the most it has, if it stands.
    fn health(&self, _thing: Option<u32>) -> Option<(f32, f32)> {
        None
    }
    /// Whether a side has a unit at a thing.
    fn reached(&self, _thing: Option<u32>, _side: i32) -> bool {
        false
    }
    /// How many buildings a side has left.
    fn buildings_left(&self, _side: i32) -> i32 {
        0
    }
    /// Whether a side has units, all of them where they were sent.
    fn arrived(&self, _side: i32) -> bool {
        false
    }
    /// Whether a side has lost any of what the script gave it.
    fn has_lost(&self, _side: i32) -> bool {
        false
    }
    /// Sends a side's units to a place.
    fn send(&mut self, _side: i32, _to: (f32, f32)) {}
    /// Has a side's units go after the player's.
    fn hunt(&mut self, _side: i32) {}
    /// Takes a thing off the map.
    fn remove(&mut self, _thing: Option<u32>) {}
    /// Destroys everything of a side, or of every side.
    fn nuke(&mut self, _side: Option<i32>) {}
    /// Uncovers the fog so many squares round a place for the player.
    fn uncover(&mut self, _at: (f32, f32), _squares: i32) {}
    /// Covers it again so many squares round a place.
    fn cover(&mut self, _at: (f32, f32), _squares: i32) {}
    /// Uncovers the whole map for the player.
    fn uncover_all(&mut self) {}
    /// Hands all of side `from`'s units over to side `to`.
    fn hand_over(&mut self, _from: i32, _to: i32) {}
    /// A scene begins, which the player watches, or ends.
    fn scene(&mut self, _playing: bool) {}
    fn camera(&mut self, _shot: Shot) {}
}

/// A piece of a line as the script reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Piece {
    Function(u16),
    Keyword(u16),
    Variable(u16),
    Kind(u16),
    Speech(u16),
    Number(i32),
    Open,
    Close,
    Comma,
}

#[derive(Clone, Debug, PartialEq)]
enum Expression {
    Number(i32),
    Variable(u16),
    Kind(u16),
    Speech(u16),
    Call(Piece, Vec<Expression>),
    Both(u16, Box<Expression>, Box<Expression>),
}

#[derive(Clone, Debug, PartialEq)]
enum Statement {
    Nothing,
    If(Expression),
    Else,
    End,
    Set(u16, Expression),
    Do(Expression),
}

/// A mission's script, ready to run: run once an update, all of it, the parts of its `if`s
/// that do not hold left out.
#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    statements: Vec<Statement>,
    variables: Vec<Value>,
}

impl Script {
    pub fn new(lines: &[Vec<Token>]) -> Self {
        Script {
            statements: lines.iter().map(|line| statement(&pieces(line))).collect(),
            variables: Vec::new(),
        }
    }

    /// Runs the script once, for the update the battle is at.
    pub fn run(&mut self, battle: &mut impl Battle) {
        let (mut depth, mut skipping) = (0usize, None);
        for at in 0..self.statements.len() {
            match &self.statements[at] {
                Statement::If(test) => {
                    depth += 1;
                    if skipping.is_none() && self.value(&test.clone(), battle).number() == 0 {
                        skipping = Some(depth);
                    }
                }
                Statement::Else => {
                    if skipping == Some(depth) {
                        skipping = None;
                    } else if skipping.is_none() {
                        skipping = Some(depth);
                    }
                }
                Statement::End => {
                    if skipping == Some(depth) {
                        skipping = None;
                    }
                    depth = depth.saturating_sub(1);
                }
                _ if skipping.is_some() => {}
                Statement::Set(variable, expression) => {
                    let (variable, expression) = (*variable as usize, expression.clone());
                    let value = self.value(&expression, battle);
                    if self.variables.len() <= variable {
                        self.variables.resize(variable + 1, Value::Number(0));
                    }
                    self.variables[variable] = value;
                }
                Statement::Do(expression) => {
                    self.value(&expression.clone(), battle);
                }
                Statement::Nothing => {}
            }
        }
    }

    fn value(&mut self, expression: &Expression, battle: &mut impl Battle) -> Value {
        match expression {
            Expression::Number(n) => Value::Number(*n),
            Expression::Variable(v) => self
                .variables
                .get(*v as usize)
                .copied()
                .unwrap_or(Value::Number(0)),
            Expression::Kind(kind) => Value::Number(i32::from(*kind)),
            Expression::Speech(line) => Value::Number(i32::from(*line)),
            Expression::Both(operator, left, right) => {
                let (left, right) = (self.value(left, battle), self.value(right, battle));
                let truth = |holds: bool| Value::Number(i32::from(holds));
                match *operator {
                    EQUAL => truth(left == right || left.number() == right.number()),
                    NOT_EQUAL => truth(left != right && left.number() != right.number()),
                    LESS => truth(left.number() < right.number()),
                    AT_LEAST => truth(left.number() >= right.number()),
                    MORE => truth(left.number() > right.number()),
                    AT_MOST => truth(left.number() <= right.number()),
                    AND => truth(left.number() != 0 && right.number() != 0),
                    OR => truth(left.number() != 0 || right.number() != 0),
                    _ => Value::Number(left.number() + right.number()),
                }
            }
            Expression::Call(callee, arguments) => {
                let values: Vec<Value> = arguments.iter().map(|a| self.value(a, battle)).collect();
                let at = |n: usize| values.get(n).copied().unwrap_or(Value::Number(0));
                let number = |n: usize| at(n).number();
                let kind = |n: usize| match arguments.get(n) {
                    Some(Expression::Kind(kind)) => Some(*kind),
                    _ => None,
                };
                let place = |(x, z): (f32, f32)| Value::Place(x, z);
                // A place the camera is sent to, if what it is of is still there.
                let spot = |n: usize| match at(n) {
                    Value::Place(x, z) => Some((x, z)),
                    _ => None,
                };
                match *callee {
                    Piece::Keyword(NEG) => Value::Number(-number(0)),
                    Piece::Keyword(GET_NEUTRAL_SIDE) => Value::Number(NEUTRAL_SIDE),
                    Piece::Keyword(BREAK_POINT) => Value::Number(0),
                    Piece::Keyword(SET_TILE_POS) => {
                        let cell = crate::battlefield::CELL_SIDE;
                        Value::Place(number(0) as f32 * cell, number(1) as f32 * cell)
                    }
                    Piece::Keyword(NEW_OBJECT_OFFSET_ORIENTATION) => {
                        let Some(kind) = kind(1) else {
                            return Value::Thing(None);
                        };
                        let (x, z) = at(2).place();
                        let cell = crate::battlefield::CELL_SIDE;
                        let to = (x + number(3) as f32 * cell, z + number(4) as f32 * cell);
                        Value::Thing(battle.place(number(0), kind, to, number(5)))
                    }
                    Piece::Function(MODEL_TICK) => Value::Number(battle.now()),
                    // The remainder of a signed division, as the original's; nothing to divide
                    // by gives 0.
                    Piece::Function(RANDOM) => {
                        let drawn = (battle.random() & i32::MAX as u32) as i32;
                        Value::Number(drawn.checked_rem(number(0)).unwrap_or(0))
                    }
                    Piece::Function(MULTIPLAYER) => Value::Number(0),
                    Piece::Function(GET_UNUSED_BASE_POINT) => place(battle.enemy_base()),
                    Piece::Function(GET_SIDE_BASE_POINT) => place(battle.base(number(0))),
                    Piece::Function(GET_ENTRANCE_POINT) => {
                        place(entrance(battle, battle.base(number(0)), Nearest))
                    }
                    Piece::Function(GET_EXIT_POINT) => {
                        let entrance_point = entrance(battle, battle.base(number(0)), Nearest);
                        place(entrance(battle, entrance_point, Farthest))
                    }
                    Piece::Function(GET_ENTRANCE_POINT_BY_INDEX) => {
                        let entrances = battle.entrances();
                        let index = usize::try_from(number(0)).ok();
                        let chosen = index.and_then(|i| entrances.get(i)).or(entrances.first());
                        chosen.map_or(Value::Place(0.0, 0.0), |&at| place(at))
                    }
                    Piece::Function(GET_ENTRANCE_NEAR_TO_POS) => {
                        place(entrance(battle, at(0).place(), Nearest))
                    }
                    Piece::Function(GET_ENTRANCE_FAR_FROM_POS) => {
                        place(entrance(battle, at(0).place(), Farthest))
                    }
                    Piece::Function(GET_SCRIPT_POINT) => place(battle.script_point(number(0))),
                    Piece::Function(GET_PLAYER_SIDE | GET_SECOND_PLAYER_SIDE) => Value::Number(0),
                    Piece::Function(CREATE_SIDE) => Value::Number(battle.new_side()),
                    Piece::Function(NEW_OBJECT) => match kind(1) {
                        Some(kind) => Value::Thing(battle.bring(number(0), kind, at(2).place())),
                        None => Value::Thing(None),
                    },
                    Piece::Function(GET_OBJECT_SIDE) => {
                        Value::Number(battle.owner_side(at(0).thing()).unwrap_or(SOMEONE_SIDE))
                    }
                    Piece::Function(OBJECT_VALID) => {
                        Value::Number(i32::from(!battle.destroyed(at(0).thing())))
                    }
                    Piece::Function(OBJECT_VISIBLE_TO_SIDE) => {
                        Value::Number(i32::from(battle.hurt_by(at(0).thing(), number(1))))
                    }
                    Piece::Function(OBJECT_DESTROYED | EVENT_OBJECT_DESTROYED) => {
                        Value::Number(i32::from(battle.destroyed(at(0).thing())))
                    }
                    Piece::Function(SIDE_UNIT_COUNT) => Value::Number(battle.left(number(0))),
                    Piece::Function(MESSAGE) => {
                        if let Some(Expression::Speech(line)) = arguments.first() {
                            battle.say(*line);
                        }
                        Value::Number(0)
                    }
                    Piece::Function(ADD_SIDE_CASH) => {
                        battle.pay(number(0), number(1));
                        Value::Number(0)
                    }
                    Piece::Function(GET_SIDE_CASH | GET_SIDE_SPICE) => {
                        Value::Number(battle.cash(number(0)))
                    }
                    Piece::Keyword(SET_SIDE_CASH) => {
                        if battle.now() == 0 {
                            battle.set_cash(number(0), number(1));
                        }
                        Value::Number(0)
                    }
                    Piece::Function(MISSION_OUTCOME) => {
                        if arguments.is_empty() || number(0) != 0 {
                            battle.win();
                        } else {
                            battle.lose();
                        }
                        Value::Number(0)
                    }
                    Piece::Function(SIDE_OBJECT_COUNT) => match kind(1) {
                        Some(kind) => Value::Number(battle.count(number(0), kind)),
                        None => Value::Number(0),
                    },
                    Piece::Function(GET_NEUTRAL_ENTRANCE_POINT) => place(battle.base(0)),
                    Piece::Function(GET_SIDE_POSITION) => place(battle.base(number(0))),
                    Piece::Function(GET_OBJECT_POSITION) => battle
                        .place_of(at(0).thing())
                        .map_or(Value::Thing(None), place),
                    Piece::Function(GET_ENEMY_SIDE) => Value::Number(SOMEONE_SIDE),
                    Piece::Function(SIDE_NEAR_TO_SIDE) => {
                        Value::Number(i32::from(battle.sees(number(0), number(1))))
                    }
                    Piece::Function(OBJECT_SET_HEALTH) => {
                        battle.set_health(at(0).thing(), number(1));
                        Value::Number(0)
                    }
                    Piece::Function(OBJECT_CHANGE) => match kind(1) {
                        Some(kind) => {
                            Value::Thing(battle.turn_into(at(0).thing(), kind, number(2)))
                        }
                        None => Value::Thing(None),
                    },
                    Piece::Keyword(DISABLE_UI | ENABLE_UI) => {
                        battle.scene(*callee == Piece::Keyword(DISABLE_UI));
                        Value::Number(0)
                    }
                    Piece::Function(CAMERA_LOOK_AT_POINT | PIP_CAMERA_LOOK_AT_POINT) => {
                        film(battle, spot(0).map(Shot::LookAt))
                    }
                    Piece::Function(CAMERA_PAN_TO_POINT | PIP_CAMERA_PAN_TO_POINT) => {
                        film(battle, spot(0).map(|to| Shot::GlideTo(to, number(1))))
                    }
                    Piece::Function(CAMERA_START_ROTATE) => {
                        film(battle, Some(Shot::Circle(number(0), number(1))))
                    }
                    Piece::Function(CAMERA_STOP_ROTATE) => film(battle, Some(Shot::StopCircling)),
                    Piece::Function(CAMERA_STORE) => film(battle, Some(Shot::Keep)),
                    Piece::Function(CAMERA_RESTORE) => {
                        film(battle, Some(Shot::GlideBack(number(0))))
                    }
                    Piece::Function(SIDE_CHANGE_SIDE) => {
                        battle.hand_over(number(0), number(1));
                        Value::Number(0)
                    }
                    Piece::Function(REMOVE_SHROUD) => {
                        battle.uncover(at(0).place(), number(1));
                        Value::Number(0)
                    }
                    Piece::Function(SIDE_NUKE) => {
                        battle.nuke(Some(number(0)));
                        Value::Number(0)
                    }
                    Piece::Function(SIDE_NUKE_ALL) => {
                        battle.nuke(None);
                        Value::Number(0)
                    }
                    Piece::Function(REPLACE_SHROUD) => {
                        battle.cover(at(0).place(), number(1));
                        Value::Number(0)
                    }
                    Piece::Function(REMOVE_MAP_SHROUD) => {
                        battle.uncover_all();
                        Value::Number(0)
                    }
                    Piece::Function(CAMERA_TRACK_OBJECT) => {
                        film(battle, at(0).thing().map(Shot::Follow))
                    }
                    Piece::Function(CAMERA_STOP_TRACK) => film(battle, Some(Shot::StopFollowing)),
                    Piece::Function(OBJECT_GET_HEALTH) => Value::Number(
                        battle
                            .health(at(0).thing())
                            .map_or(0, |(health, _)| health as i32),
                    ),
                    Piece::Function(OBJECT_MAX_HEALTH) => Value::Number(
                        battle
                            .health(at(0).thing())
                            .map_or(0, |(_, most)| most as i32),
                    ),
                    Piece::Function(OBJECT_NEAR_TO_SIDE) => {
                        Value::Number(i32::from(battle.reached(at(0).thing(), number(1))))
                    }
                    Piece::Function(SIDE_BUILDING_COUNT) => {
                        Value::Number(battle.buildings_left(number(0)))
                    }
                    Piece::Function(SIDE_AI_DONE) => {
                        Value::Number(i32::from(battle.arrived(number(0))))
                    }
                    Piece::Function(EVENT_SIDE_ATTACKS_SIDE) => {
                        Value::Number(i32::from(battle.has_lost(number(1))))
                    }
                    Piece::Function(SIDE_AI_AGGRESSIVE) => {
                        battle.hunt(number(0));
                        Value::Number(0)
                    }
                    Piece::Function(SIDE_AI_MOVE) => {
                        battle.send(number(0), at(1).place());
                        Value::Number(0)
                    }
                    Piece::Function(END_GAME_WIN) => {
                        battle.win();
                        Value::Number(0)
                    }
                    Piece::Function(END_GAME_LOSE) => {
                        battle.lose();
                        Value::Number(0)
                    }
                    Piece::Keyword(OBJECT_SELL) => {
                        battle.remove(at(0).thing());
                        Value::Number(0)
                    }
                    Piece::Keyword(SET_VALUE) => Value::Number(number(0)),
                    Piece::Function(SIDE_FRIEND_TO | SIDE_ENEMY_TO) => {
                        battle.relate(
                            number(0),
                            number(1),
                            *callee == Piece::Function(SIDE_ENEMY_TO),
                        );
                        Value::Number(0)
                    }
                    _ => {
                        if let Some(warning) = first_unknown(*callee) {
                            eprintln!("{warning}");
                        }
                        Value::Number(0)
                    }
                }
            }
        }
    }
}

/// A warning that a script calls a function or keyword we don't carry out, by its number, the
/// first time one is called in this run; `None` after that, or for anything else.
fn first_unknown(callee: Piece) -> Option<String> {
    static WARNED: std::sync::Mutex<Vec<Piece>> = std::sync::Mutex::new(Vec::new());
    let name = match callee {
        Piece::Function(n) => format!("function F{n}"),
        Piece::Keyword(n) => format!("keyword K{n}"),
        _ => return None,
    };
    let mut warned = WARNED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if warned.contains(&callee) {
        return None;
    }
    warned.push(callee);
    Some(format!(
        "Warning: the mission script calls {name}, which is not carried out"
    ))
}

/// Which of the map's entrances a script asks for, by how far it is from a place.
#[derive(Clone, Copy)]
enum Reach {
    Nearest,
    Farthest,
}
use Reach::{Farthest, Nearest};

/// The map's entrance nearest a place or farthest from it, the first of those as near or as
/// far, as the original goes through them; the place itself on a map with none.
fn entrance(battle: &impl Battle, (x, z): (f32, f32), reach: Reach) -> (f32, f32) {
    let distance = |&(ex, ez): &(f32, f32)| (ex - x).hypot(ez - z);
    let mut chosen: Option<((f32, f32), f32)> = None;
    for at in battle.entrances() {
        let far = distance(&at);
        let better = chosen.is_none_or(|(_, best)| match reach {
            Nearest => far < best,
            Farthest => far > best,
        });
        if better {
            chosen = Some((at, far));
        }
    }
    chosen.map_or((x, z), |(at, _)| at)
}

/// Has the camera do this, if there is anything to do.
fn film(battle: &mut impl Battle, shot: Option<Shot>) -> Value {
    if let Some(shot) = shot {
        battle.camera(shot);
    }
    Value::Number(0)
}

/// A line's tokens as the pieces the script reads: its text cut into numbers, brackets and
/// commas.
fn pieces(line: &[Token]) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for token in line {
        match token {
            Token::Function(n) => pieces.push(Piece::Function(*n)),
            Token::Keyword(n) => pieces.push(Piece::Keyword(*n)),
            Token::Variable(n) => pieces.push(Piece::Variable(*n)),
            Token::Kind(n) => pieces.push(Piece::Kind(*n)),
            Token::Speech(n) => pieces.push(Piece::Speech(*n)),
            Token::Text(text) => {
                let mut number: Option<i32> = None;
                for ch in text.chars() {
                    if let Some(digit) = ch.to_digit(10) {
                        number = Some(number.unwrap_or(0) * 10 + digit as i32);
                        continue;
                    }
                    if let Some(n) = number.take() {
                        pieces.push(Piece::Number(n));
                    }
                    match ch {
                        '(' => pieces.push(Piece::Open),
                        ')' => pieces.push(Piece::Close),
                        ',' => pieces.push(Piece::Comma),
                        _ => {}
                    }
                }
                if let Some(n) = number {
                    pieces.push(Piece::Number(n));
                }
            }
        }
    }
    pieces
}

/// What a line does.
fn statement(pieces: &[Piece]) -> Statement {
    match pieces {
        [] => Statement::Nothing,
        [Piece::Keyword(IF), rest @ ..] => Statement::If(
            Reader {
                pieces: rest,
                at: 0,
            }
            .expression(0),
        ),
        [Piece::Keyword(ELSE), ..] => Statement::Else,
        [Piece::Keyword(ENDIF), ..] => Statement::End,
        [Piece::Variable(v), Piece::Keyword(SET), rest @ ..] => Statement::Set(
            *v,
            Reader {
                pieces: rest,
                at: 0,
            }
            .expression(0),
        ),
        _ => Statement::Do(Reader { pieces, at: 0 }.expression(0)),
    }
}

/// Reads expressions out of a line's pieces, by how tightly each operator binds.
struct Reader<'a> {
    pieces: &'a [Piece],
    at: usize,
}

impl Reader<'_> {
    fn next(&mut self) -> Option<Piece> {
        let piece = self.pieces.get(self.at).copied();
        self.at += 1;
        piece
    }

    fn peek(&self) -> Option<Piece> {
        self.pieces.get(self.at).copied()
    }

    /// An expression whose operators bind at least as tightly as `least`.
    fn expression(&mut self, least: u8) -> Expression {
        let mut left = self.single();
        while let Some(Piece::Keyword(operator)) = self.peek() {
            let Some(binds) = binding(operator).filter(|&binds| binds >= least) else {
                break;
            };
            self.at += 1;
            let right = self.expression(binds + 1);
            left = Expression::Both(operator, Box::new(left), Box::new(right));
        }
        left
    }

    fn single(&mut self) -> Expression {
        match self.next() {
            Some(Piece::Number(n)) => Expression::Number(n),
            Some(Piece::Variable(v)) => Expression::Variable(v),
            Some(Piece::Kind(kind)) => Expression::Kind(kind),
            Some(Piece::Speech(line)) => Expression::Speech(line),
            Some(Piece::Keyword(FALSE)) => Expression::Number(0),
            Some(Piece::Keyword(TRUE)) => Expression::Number(1),
            Some(Piece::Open) => {
                let inside = self.expression(0);
                if self.peek() == Some(Piece::Close) {
                    self.at += 1;
                }
                inside
            }
            Some(callee @ (Piece::Function(_) | Piece::Keyword(_))) => {
                let mut arguments = Vec::new();
                if self.peek() == Some(Piece::Open) {
                    self.at += 1;
                    while !matches!(self.peek(), None | Some(Piece::Close)) {
                        arguments.push(self.expression(0));
                        if self.peek() == Some(Piece::Comma) {
                            self.at += 1;
                        } else if !matches!(self.peek(), None | Some(Piece::Close)) {
                            self.at += 1;
                        }
                    }
                    self.at += 1;
                }
                Expression::Call(callee, arguments)
            }
            _ => Expression::Number(0),
        }
    }
}

/// How tightly an operator binds, if it is one: `||` least, then `&&`, the comparisons, `+`.
fn binding(keyword: u16) -> Option<u8> {
    match keyword {
        OR => Some(1),
        AND => Some(2),
        EQUAL | NOT_EQUAL | LESS | AT_LEAST | MORE | AT_MOST => Some(3),
        PLUS => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_we_do_not_carry_out_is_warned_of_once_by_its_number() {
        let warning = first_unknown(Piece::Function(999)).unwrap();
        assert!(warning.contains("F999"), "{warning}");
        assert_eq!(first_unknown(Piece::Function(999)), None, "once");
        assert!(first_unknown(Piece::Keyword(999)).unwrap().contains("K999"));
        assert_eq!(first_unknown(Piece::Number(3)), None);
    }

    /// A script of these lines, with its head.
    fn script(lines: &[&[u8]]) -> Vec<u8> {
        let body: Vec<u8> = lines
            .iter()
            .flat_map(|line| line.iter().copied().chain([0]))
            .collect();
        let mut script = (body.len() as u32).to_le_bytes().to_vec();
        script.extend((lines.len() as u32).to_le_bytes());
        script.extend(body);
        script
    }

    #[test]
    fn a_script_reads_into_lines_of_functions_keywords_variables_kinds_and_text() {
        let lines: [&[u8]; 4] = [
            // v6 = F5(1)
            &[
                0x81, 0x86, 0x80, 0xb4, 0x81, 0x80, 0x85, 0x80, b'(', b'1', b')',
            ],
            &[],
            // K139(v0,<kind 138>,v13,0,0,1)
            b"\x80\x8b\x81(\x81\x80,\x82\x8a\x81,\x81\x8d,0,0,1)",
            // F45(<speech 642>)
            b"\x80\xad\x80(\x83\x82\x85)",
        ];
        let read = read(&script(&lines)).unwrap();
        assert_eq!(read.len(), 4);
        assert_eq!(
            read[0],
            [
                Token::Variable(6),
                Token::Keyword(180),
                Token::Function(5),
                Token::Text("(1)".to_string()),
            ]
        );
        assert!(read[1].is_empty(), "a blank line of the source");
        assert_eq!(read[2][0], Token::Keyword(139));
        assert_eq!(read[2][2], Token::Variable(0));
        // 138 in the file: the 27th of the rules' lists, the Atreides construction yard.
        assert_eq!(read[2][4], Token::Kind(27));
        assert_eq!(read[2].last(), Some(&Token::Text(",0,0,1)".to_string())));
        assert_eq!(read[3][0], Token::Function(45));
        assert_eq!(speeches(&read), [642]);
    }

    #[test]
    fn what_is_not_a_script_is_not_read() {
        assert_eq!(read(&[1, 2, 3]), None);
        let mut cut = script(&[b"1", b"2"]);
        cut.truncate(cut.len() - 1);
        assert_eq!(read(&cut), None, "shorter than it says");
        let mut wrong = script(&[b"1", b"2"]);
        wrong[4] = 5;
        assert_eq!(read(&wrong), None, "not as many lines as it says");
    }

    /// A battle that writes down what a script does in it.
    #[derive(Default)]
    struct Written {
        now: i32,
        sides: i32,
        placed: Vec<(i32, u16, (f32, f32), i32)>,
        brought: Vec<(i32, u16)>,
        said: Vec<u16>,
        paid: Vec<(i32, i32)>,
        won: bool,
        related: Vec<(i32, i32, bool)>,
        turned: Vec<(Option<u32>, u16, i32)>,
        health: Vec<(Option<u32>, i32)>,
        scenes: Vec<bool>,
        shots: Vec<Shot>,
        lost: bool,
        sent: Vec<(i32, (f32, f32))>,
        hunting: Vec<i32>,
        removed: Vec<Option<u32>>,
        uncovered: Vec<((f32, f32), i32)>,
        covered: Vec<((f32, f32), i32)>,
        all_uncovered: bool,
        nuked: Vec<Option<i32>>,
        handed: Vec<(i32, i32)>,
        entrances: Vec<(f32, f32)>,
        drawn: u32,
    }

    impl Battle for Written {
        fn now(&self) -> i32 {
            self.now
        }
        fn new_side(&mut self) -> i32 {
            self.sides += 1;
            self.sides
        }
        fn base(&self, side: i32) -> (f32, f32) {
            (100.0 * side as f32, 0.0)
        }
        fn enemy_base(&self) -> (f32, f32) {
            (1000.0, 1000.0)
        }
        fn script_point(&self, number: i32) -> (f32, f32) {
            (number as f32, 0.0)
        }
        fn entrances(&self) -> Vec<(f32, f32)> {
            self.entrances.clone()
        }
        fn random(&mut self) -> u32 {
            self.drawn = self.drawn.wrapping_add(7);
            self.drawn
        }
        fn place(&mut self, side: i32, kind: u16, at: (f32, f32), facing: i32) -> Option<u32> {
            self.placed.push((side, kind, at, facing));
            Some(self.placed.len() as u32)
        }
        fn bring(&mut self, side: i32, kind: u16, _: (f32, f32)) -> Option<u32> {
            self.brought.push((side, kind));
            None
        }
        fn left(&self, _: i32) -> i32 {
            3
        }
        fn destroyed(&self, thing: Option<u32>) -> bool {
            thing == Some(1)
        }
        fn say(&mut self, line: u16) {
            self.said.push(line);
        }
        fn pay(&mut self, side: i32, money: i32) {
            self.paid.push((side, money));
        }
        fn cash(&self, side: i32) -> i32 {
            let paid = self.paid.iter().filter(|&&(to, _)| to == side);
            paid.map(|&(_, money)| money).sum()
        }
        fn set_cash(&mut self, side: i32, money: i32) {
            self.paid.retain(|&(to, _)| to != side);
            self.paid.push((side, money));
        }
        fn win(&mut self) {
            self.won = true;
        }
        fn relate(&mut self, a: i32, b: i32, enemies: bool) {
            self.related.push((a, b, enemies));
        }
        fn turn_into(&mut self, thing: Option<u32>, kind: u16, side: i32) -> Option<u32> {
            self.turned.push((thing, kind, side));
            thing
        }
        fn set_health(&mut self, thing: Option<u32>, health: i32) {
            self.health.push((thing, health));
        }
        fn sees(&self, seen: i32, by: i32) -> bool {
            (seen, by) == (1, 0)
        }
        fn scene(&mut self, playing: bool) {
            self.scenes.push(playing);
        }
        fn uncover(&mut self, at: (f32, f32), squares: i32) {
            self.uncovered.push((at, squares));
        }
        fn cover(&mut self, at: (f32, f32), squares: i32) {
            self.covered.push((at, squares));
        }
        fn uncover_all(&mut self) {
            self.all_uncovered = true;
        }
        fn nuke(&mut self, side: Option<i32>) {
            self.nuked.push(side);
        }
        fn hand_over(&mut self, from: i32, to: i32) {
            self.handed.push((from, to));
        }
        fn lose(&mut self) {
            self.lost = true;
        }
        fn count(&self, side: i32, kind: u16) -> i32 {
            side * 10 + i32::from(kind)
        }
        fn health(&self, thing: Option<u32>) -> Option<(f32, f32)> {
            (thing == Some(2)).then_some((300.0, 600.0))
        }
        fn reached(&self, thing: Option<u32>, side: i32) -> bool {
            thing == Some(2) && side == 0
        }
        fn buildings_left(&self, side: i32) -> i32 {
            side + 3
        }
        fn arrived(&self, side: i32) -> bool {
            side == 2
        }
        fn has_lost(&self, side: i32) -> bool {
            side == 3
        }
        fn send(&mut self, side: i32, to: (f32, f32)) {
            self.sent.push((side, to));
        }
        fn hunt(&mut self, side: i32) {
            self.hunting.push(side);
        }
        fn remove(&mut self, thing: Option<u32>) {
            self.removed.push(thing);
        }
        fn camera(&mut self, shot: Shot) {
            self.shots.push(shot);
        }
    }

    fn text(text: &str) -> Token {
        Token::Text(text.to_string())
    }

    #[test]
    fn a_battle_ends_won_or_lost_as_told_and_counts_a_sides_things_of_a_kind() {
        use Token::{Function as F, Keyword as K, Kind, Variable as V};
        let mut won = Written::default();
        Script::new(&[vec![F(MISSION_OUTCOME), text("("), K(TRUE), text(")")]]).run(&mut won);
        assert!(won.won && !won.lost);
        let mut lost = Written::default();
        Script::new(&[vec![F(MISSION_OUTCOME), text("("), K(FALSE), text(")")]]).run(&mut lost);
        assert!(lost.lost && !lost.won);
        let mut lost = Written::default();
        Script::new(&[vec![F(END_GAME_LOSE), text("()")]]).run(&mut lost);
        assert!(lost.lost && !lost.won);
        // v1 = F37(2, <kind 4>)
        let lines = vec![vec![
            V(1),
            K(SET),
            F(SIDE_OBJECT_COUNT),
            text("(2,"),
            Kind(4),
            text(")"),
        ]];
        let mut script = Script::new(&lines);
        script.run(&mut Written::default());
        assert_eq!(script.variables[1], Value::Number(24));
    }

    #[test]
    fn the_tutorials_calls_ask_the_battle_of_health_beacons_sides_and_its_end() {
        use Token::{Function as F, Keyword as K, Kind, Variable as V};
        let put = |variable: u16| {
            let open = text("(0,");
            let rest = [text(","), F(GET_SCRIPT_POINT), text("(1),0,0,0)")];
            [
                vec![
                    V(variable),
                    K(SET),
                    K(NEW_OBJECT_OFFSET_ORIENTATION),
                    open,
                    Kind(1),
                ],
                rest.to_vec(),
            ]
            .concat()
        };
        let set = |variable: u16, call: Vec<Token>| [vec![V(variable), K(SET)], call].concat();
        let lines = vec![
            put(0),
            put(1),
            set(2, vec![F(OBJECT_GET_HEALTH), text("("), V(1), text(")")]),
            set(3, vec![F(OBJECT_MAX_HEALTH), text("("), V(1), text(")")]),
            set(
                4,
                vec![
                    F(OBJECT_NEAR_TO_SIDE),
                    text("("),
                    V(1),
                    text(","),
                    F(GET_PLAYER_SIDE),
                    text("())"),
                ],
            ),
            set(5, vec![F(SIDE_BUILDING_COUNT), text("(2)")]),
            set(6, vec![F(SIDE_AI_DONE), text("(2)")]),
            set(
                7,
                vec![
                    F(EVENT_SIDE_ATTACKS_SIDE),
                    text("("),
                    F(GET_PLAYER_SIDE),
                    text("(),3)"),
                ],
            ),
            vec![
                F(SIDE_AI_MOVE),
                text("(2,"),
                F(GET_SCRIPT_POINT),
                text("(4))"),
            ],
            vec![F(SIDE_AI_AGGRESSIVE), text("(4)")],
            vec![K(OBJECT_SELL), text("("), V(0), text(")")],
            set(8, vec![K(SET_VALUE), text("(11)")]),
            vec![F(END_GAME_WIN), text("()")],
        ];
        let mut battle = Written::default();
        let mut script = Script::new(&lines);
        script.run(&mut battle);
        let numbers: Vec<Value> = (2..=8).map(|v| script.variables[v]).collect();
        let n = Value::Number;
        assert_eq!(numbers, [n(300), n(600), n(1), n(5), n(1), n(1), n(11)]);
        assert_eq!(battle.sent, [(2, battle.script_point(4))]);
        assert_eq!(battle.hunting, [4]);
        assert_eq!(battle.removed, [Some(1)]);
        assert!(battle.won, "the tutorial done");
    }

    #[test]
    fn a_script_asks_for_the_maps_entrances_by_side_number_and_how_far_from_a_place() {
        use Token::{Function as F, Keyword as K, Variable as V};
        let set = |variable: u16, call: Vec<Token>| [vec![V(variable), K(SET)], call].concat();
        let point = vec![F(GET_SCRIPT_POINT), text("(990)")];
        let lines = vec![
            set(1, vec![F(GET_ENTRANCE_POINT), text("(1)")]),
            set(2, vec![F(GET_EXIT_POINT), text("(1)")]),
            set(3, vec![F(GET_ENTRANCE_POINT_BY_INDEX), text("(2)")]),
            set(4, vec![F(GET_ENTRANCE_POINT_BY_INDEX), text("(5)")]),
            set(
                5,
                [vec![F(GET_ENTRANCE_NEAR_TO_POS), text("(")], point.clone()].concat(),
            ),
            set(
                6,
                [vec![F(GET_ENTRANCE_FAR_FROM_POS), text("(")], point].concat(),
            ),
        ];
        let mut battle = Written {
            entrances: vec![(0.0, 500.0), (150.0, 0.0), (1000.0, 0.0)],
            ..Written::default()
        };
        let mut script = Script::new(&lines);
        script.run(&mut battle);
        let p = |x, z| Value::Place(x, z);
        assert_eq!(
            script.variables[1..=6],
            [
                p(150.0, 0.0),
                p(1000.0, 0.0),
                p(1000.0, 0.0),
                p(0.0, 500.0),
                p(1000.0, 0.0),
                p(0.0, 500.0)
            ],
            "side 1's base is at (100, 0)"
        );
        // A map without entrances gives the place asked about.
        let mut script = Script::new(&lines);
        script.run(&mut Written::default());
        assert_eq!(script.variables[1], p(100.0, 0.0));
    }

    #[test]
    fn a_random_number_is_below_the_one_given_and_nothing_to_divide_by_gives_naught() {
        use Token::{Function as F, Keyword as K, Variable as V};
        // v1 = Random(5); v2 = Random(5); v3 = Random(0); v4 = Random(3000) + 100
        let lines = vec![
            vec![V(1), K(SET), F(RANDOM), text("(5)")],
            vec![V(2), K(SET), F(RANDOM), text("(5)")],
            vec![V(3), K(SET), F(RANDOM), text("(0)")],
            vec![
                V(4),
                K(SET),
                F(RANDOM),
                text("(3000)"),
                K(PLUS),
                text("100"),
            ],
        ];
        let mut script = Script::new(&lines);
        script.run(&mut Written::default());
        let n = Value::Number;
        assert_eq!(script.variables[1..=4], [n(2), n(4), n(0), n(128)]);
    }

    #[test]
    fn a_sides_money_is_added_to_asked_of_as_cash_or_spice_and_set_only_at_the_start() {
        use Token::{Function as F, Keyword as K, Variable as V};
        let lines = vec![
            vec![F(ADD_SIDE_CASH), text("(1,500)")],
            vec![F(ADD_SIDE_CASH), text("(1,250)")],
            vec![V(1), K(SET), F(GET_SIDE_CASH), text("(1)")],
            vec![K(SET_SIDE_CASH), text("(1,100)")],
            vec![V(2), K(SET), F(GET_SIDE_SPICE), text("(1)")],
        ];
        let mut script = Script::new(&lines);
        let mut battle = Written::default();
        script.run(&mut battle);
        let n = Value::Number;
        assert_eq!(script.variables[1..=2], [n(750), n(100)]);
        battle.now = 1;
        script.run(&mut battle);
        assert_eq!(
            script.variables[1..=2],
            [n(850), n(850)],
            "set only at the start"
        );
    }

    #[test]
    fn a_squares_place_is_its_corner_in_the_worlds_units() {
        use Token::{Keyword as K, Variable as V};
        let lines = vec![vec![V(1), K(SET), K(SET_TILE_POS), text("(141,132)")]];
        let mut script = Script::new(&lines);
        script.run(&mut Written::default());
        assert_eq!(
            script.variables[1],
            Value::Place(141.0 * 32.0, 132.0 * 32.0)
        );
    }

    #[test]
    fn a_break_point_does_nothing_and_is_not_warned_of() {
        use Token::Keyword as K;
        let mut battle = Written::default();
        Script::new(&[vec![K(BREAK_POINT), text("()")]]).run(&mut battle);
        assert!(battle.said.is_empty() && battle.shots.is_empty() && !battle.won);
        assert!(
            first_unknown(Piece::Keyword(BREAK_POINT)).is_some(),
            "not warned of yet"
        );
    }

    #[test]
    fn a_game_is_never_the_co_op_campaign() {
        use Token::{Function as F, Keyword as K, Variable as V};
        let lines = vec![
            vec![V(1), K(SET), F(MULTIPLAYER), text("()")],
            vec![V(2), K(SET), F(GET_SECOND_PLAYER_SIDE), text("()")],
            vec![V(3), K(SET), K(GET_NEUTRAL_SIDE), text("()")],
        ];
        let mut script = Script::new(&lines);
        script.run(&mut Written::default());
        assert_eq!(script.variables[1], Value::Number(0));
        assert_eq!(script.variables[2], Value::Number(0), "the player's side");
        assert_eq!(
            script.variables[3],
            Value::Number(15),
            "the last of sixteen"
        );
    }

    #[test]
    fn the_comparisons_go_by_their_numbers_from_equal_to_less() {
        use Token::{Keyword as K, Variable as V};
        let compare = |left: &str, operator: u16, right: &str| {
            let lines = vec![vec![V(1), K(SET), text(left), K(operator), text(right)]];
            let mut script = Script::new(&lines);
            script.run(&mut Written::default());
            script.variables[1]
        };
        let truth = |holds: bool| Value::Number(i32::from(holds));
        for (left, right) in [("1", "1"), ("2", "1"), ("1", "2")] {
            let (a, b): (i32, i32) = (left.parse().unwrap(), right.parse().unwrap());
            assert_eq!(compare(left, AT_LEAST, right), truth(a >= b));
            assert_eq!(compare(left, MORE, right), truth(a > b));
            assert_eq!(compare(left, AT_MOST, right), truth(a <= b));
            assert_eq!(compare(left, LESS, right), truth(a < b));
            assert_eq!(compare(left, NOT_EQUAL, right), truth(a != b));
        }
        assert_eq!((NOT_EQUAL, AT_MOST, LESS), (169, 171, 173));
    }

    #[test]
    fn a_first_battles_opening_scene_has_the_camera_look_glide_circle_and_come_back() {
        use Token::{Function as F, Keyword as K, Variable as V};
        // As the houses' first battles have it: K131(); F100(v13); F112(); F101(v16,50);
        // F105(2,1); F113(60); K132(); and the camera sent to a thing gone does nothing.
        let point = |n: &str| vec![F(GET_SCRIPT_POINT), text(&format!("({n})"))];
        let lines = vec![
            [vec![V(13), K(SET)], point("13")].concat(),
            [vec![V(16), K(SET)], point("16")].concat(),
            vec![K(DISABLE_UI), text("()")],
            vec![F(CAMERA_LOOK_AT_POINT), text("("), V(13), text(")")],
            vec![F(CAMERA_STORE), text("()")],
            vec![F(CAMERA_PAN_TO_POINT), text("("), V(16), text(",50)")],
            vec![F(CAMERA_START_ROTATE), text("(2,1)")],
            vec![F(CAMERA_STOP_ROTATE), text("()")],
            vec![F(CAMERA_RESTORE), text("(60)")],
            vec![K(ENABLE_UI), text("()")],
            vec![F(PIP_CAMERA_LOOK_AT_POINT), text("("), V(20), text(")")],
            vec![F(PIP_CAMERA_PAN_TO_POINT), text("("), V(13), text(",80)")],
            // As the tutorial's opening: v1 = K139(...); F107(v1); ... F108(); and the camera
            // sent after a thing there is not does nothing.
            vec![
                V(1),
                K(SET),
                K(NEW_OBJECT_OFFSET_ORIENTATION),
                text("(0,"),
                Token::Kind(1),
                text(","),
                V(13),
                text(",0,0,0)"),
            ],
            vec![F(CAMERA_TRACK_OBJECT), text("("), V(1), text(")")],
            vec![F(CAMERA_STOP_TRACK), text("()")],
            vec![F(CAMERA_TRACK_OBJECT), text("("), V(30), text(")")],
            // F97(v13,20), as the tutorial shows its beacon; F98(v16,8); F99().
            vec![F(REMOVE_SHROUD), text("("), V(13), text(",20)")],
            vec![F(REPLACE_SHROUD), text("("), V(16), text(",8)")],
            vec![F(REMOVE_MAP_SHROUD), text("()")],
            vec![F(SIDE_NUKE), text("(3)")],
            vec![F(SIDE_NUKE_ALL), text("()")],
            // F91(v18, F14()), as the Atreides' Heighliner hands its Mongooses over.
            vec![F(SIDE_CHANGE_SIDE), text("(2,0)")],
        ];
        let mut battle = Written::default();
        Script::new(&lines).run(&mut battle);
        assert_eq!(battle.scenes, [true, false]);
        assert_eq!(
            battle.shots,
            [
                Shot::LookAt((13.0, 0.0)),
                Shot::Keep,
                Shot::GlideTo((16.0, 0.0), 50),
                Shot::Circle(2, 1),
                Shot::StopCircling,
                Shot::GlideBack(60),
                Shot::GlideTo((13.0, 0.0), 80),
                Shot::Follow(1),
                Shot::StopFollowing,
            ]
        );
        assert_eq!(battle.uncovered, [((13.0, 0.0), 20)]);
        assert_eq!(battle.covered, [((16.0, 0.0), 8)]);
        assert!(battle.all_uncovered);
        assert_eq!(battle.nuked, [Some(3), None]);
        assert_eq!(battle.handed, [(2, 0)]);
    }

    #[test]
    fn a_prison_camps_sides_are_related_and_its_prisoners_freed_once_seen() {
        use Token::{Function as F, Keyword as K, Kind, Variable as V};
        let lines = vec![
            // v1 = F18(); v2 = F18(); F59(v1, F14()); F60(F16(), v2)
            vec![V(1), K(SET), F(CREATE_SIDE), text("()")],
            vec![V(2), K(SET), F(CREATE_SIDE), text("()")],
            vec![
                F(SIDE_FRIEND_TO),
                text("("),
                V(1),
                text(","),
                F(GET_PLAYER_SIDE),
                text("())"),
            ],
            vec![
                F(SIDE_ENEMY_TO),
                text("("),
                F(GET_ENEMY_SIDE),
                text("(),"),
                V(2),
                text(")"),
            ],
            // v3 = K139(v1, <kind 4>, F3(), 0, 0, 0); F54(v3, 250)
            vec![
                V(3),
                K(SET),
                K(NEW_OBJECT_OFFSET_ORIENTATION),
                text("("),
                V(1),
                text(","),
                Kind(4),
                text(","),
                F(GET_UNUSED_BASE_POINT),
                text("(),0,0,0)"),
            ],
            vec![F(OBJECT_SET_HEALTH), text("("), V(3), text(",250)")],
            // if (F32(v1, F14()) == TRUE) F57(v3, <kind 7>, F14())
            vec![
                K(IF),
                text("("),
                F(SIDE_NEAR_TO_SIDE),
                text("("),
                V(1),
                text(","),
                F(GET_PLAYER_SIDE),
                text("())"),
                K(EQUAL),
                K(TRUE),
                text(")"),
            ],
            vec![
                F(OBJECT_CHANGE),
                text("("),
                V(3),
                text(","),
                Kind(7),
                text(","),
                F(GET_PLAYER_SIDE),
                text("())"),
            ],
            vec![K(ENDIF)],
        ];
        let mut battle = Written::default();
        Script::new(&lines).run(&mut battle);
        assert_eq!(
            battle.related,
            [(1, 0, false), (SOMEONE_SIDE, 2, true)],
            "friends, then enemies"
        );
        assert_eq!(battle.health, [(Some(1), 250)]);
        assert_eq!(battle.turned, [(Some(1), 7, 0)], "seen, it is the player's");
    }

    #[test]
    fn a_script_runs_its_ifs_when_they_hold_and_keeps_its_variables() {
        use Token::{Function as F, Keyword as K, Kind, Speech, Variable as V};
        let lines = vec![
            // if (F0() == 0)
            vec![
                K(IF),
                text("("),
                F(MODEL_TICK),
                text("()"),
                K(EQUAL),
                text("0)"),
            ],
            // v1 = F18()
            vec![V(1), K(SET), F(CREATE_SIDE), text("()")],
            // v2 = K139(v1, <kind 5>, F3(), K143(2), 3, 1)
            vec![
                V(2),
                K(SET),
                K(NEW_OBJECT_OFFSET_ORIENTATION),
                text("("),
                V(1),
                text(","),
                Kind(5),
                text(","),
                F(GET_UNUSED_BASE_POINT),
                text("(),"),
                K(NEG),
                text("(2),3,1)"),
            ],
            vec![
                F(ADD_SIDE_CASH),
                text("("),
                F(GET_PLAYER_SIDE),
                text("(),10000)"),
            ],
            vec![F(MESSAGE), text("("), Speech(642), text(")")],
            vec![K(ENDIF)],
            // if ((F0() == 49) && (F35(v1) < 5)) bring
            vec![
                K(IF),
                text("(("),
                F(MODEL_TICK),
                text("()"),
                K(EQUAL),
                text("49)"),
                K(AND),
                text("("),
                F(SIDE_UNIT_COUNT),
                text("("),
                V(1),
                text(")"),
                K(LESS),
                text("5))"),
            ],
            vec![
                F(NEW_OBJECT),
                text("("),
                V(1),
                text(","),
                Kind(9),
                text(","),
                F(GET_SCRIPT_POINT),
                text("(2))"),
            ],
            vec![K(ENDIF)],
            vec![
                K(IF),
                text("("),
                F(OBJECT_DESTROYED),
                text("("),
                V(2),
                text("))"),
            ],
            vec![F(MISSION_OUTCOME), text("("), K(TRUE), text(")")],
            vec![K(ENDIF)],
        ];
        let mut script = Script::new(&lines);
        let mut battle = Written::default();
        script.run(&mut battle);
        let cell = crate::battlefield::CELL_SIDE;
        assert_eq!(
            battle.placed,
            [(1, 5, (1000.0 - 2.0 * cell, 1000.0 + 3.0 * cell), 1)]
        );
        assert_eq!(battle.paid, [(0, 10000)]);
        assert_eq!(battle.said, [642]);
        assert!(battle.won, "the thing it placed is gone");
        battle.now = 1;
        script.run(&mut battle);
        assert_eq!(battle.placed.len(), 1, "only at the start");
        assert!(battle.brought.is_empty());
        battle.now = 49;
        script.run(&mut battle);
        assert_eq!(battle.brought, [(1, 9)], "with the side it kept");
    }
}
