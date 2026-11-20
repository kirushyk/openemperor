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

/// The keywords and functions a script is run with, by their numbers. What each does is
/// worked out from how the original's scripts use them; `docs/tok-format.md` lists them.
const NEGATIVE: u16 = 143;
const PLACE: u16 = 139;
const IF: u16 = 165;
const ELSE: u16 = 166;
const END: u16 = 167;
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
const NOW: u16 = 0;
/// Asked before the first reinforcements: whether the battle goes on as usual, it seems.
const READY: u16 = 2;
const ENEMY_BASE: u16 = 3;
const BASE: u16 = 4;
const SCRIPT_POINT: u16 = 5;
const SIDE_PLACE: u16 = 6;
const PLAYER: u16 = 14;
const NEW_SIDE: u16 = 18;
const BRING: u16 = 21;
const DESTROYED: u16 = 23;
const LEFT: u16 = 35;
const SAY: u16 = 45;
const PAY: u16 = 62;
/// Ends the battle, won if given true and lost if false.
const WIN: u16 = 83;
/// How many of a side's things of a kind there are.
const COUNT: u16 = 37;
/// Where reinforcements come in: given to the player's mostly.
const ENTRANCE: u16 = 8;
/// A place near one given: taken as that place.
const NEAR: u16 = 11;
/// Where a side's units arrive, its base.
const ARRIVAL: u16 = 12;
/// Where a thing is.
const WHERE: u16 = 13;
/// A side that is neither the player's nor one the script makes: what the scripts make
/// friends and enemies of the others with, as if the world at large.
const SOMEONE: u16 = 16;
/// Whether a side's units are seen by another's.
const SEES: u16 = 32;
/// Sets a thing's health, in the original's.
const SET_HEALTH: u16 = 54;
/// Turns a thing into another kind, for a side: prisoners freed become Fremen.
const TURN_INTO: u16 = 57;
/// Makes one side a friend of another, or an enemy.
const FRIENDS: u16 = 59;
const ENEMIES: u16 = 60;
/// Whose a thing is: the player's, or the script's side it was put down for.
const OWNER: u16 = 17;
/// Whether a thing still stands.
const STANDS: u16 = 22;
/// Whether a side has hurt a thing: the outpost of a camp, fired on.
const HURT_BY: u16 = 27;
/// Whether a thing is destroyed, as `DESTROYED`.
const GONE: u16 = 44;
/// The camera's part in a scene: begun and ended by these keywords, kept, sent to look at a
/// place, glided to one over so many updates, circled about it at so many degrees an update
/// one way or the other, stopped circling, glided back to where it was kept.
const SCENE_BEGINS: u16 = 131;
const SCENE_ENDS: u16 = 132;
const LOOK_AT: u16 = 100;
const GLIDE_TO: u16 = 101;
const CIRCLE: u16 = 105;
const STOP_CIRCLING: u16 = 106;
const KEEP_VIEW: u16 = 112;
const GLIDE_BACK: u16 = 113;
/// The same in the midst of the battle.
const LOOK_AT_TOO: u16 = 114;
const GLIDE_TO_TOO: u16 = 115;
/// A thing's health, and the most it has.
const HEALTH: u16 = 29;
const FULL_HEALTH: u16 = 30;
/// Whether a side has a unit at a thing: the tutorial's beacons reached.
const REACHED: u16 = 24;
/// How many buildings a side has left.
const BUILDINGS_LEFT: u16 = 36;
/// Whether a side's units have got where they were sent.
const ARRIVED: u16 = 38;
/// Whether a side has lost any of what the script gave it; the tutorial asks it of the player
/// and its enemy's wave, to tell a wave fought from one only the worm met.
const FOUGHT: u16 = 42;
/// A side's units go after the player's.
const HUNT: u16 = 64;
/// A side's units are sent to a place.
const SEND: u16 = 71;
/// The tutorial is over, and done.
const DONE: u16 = 84;
/// Takes a thing off the map.
const REMOVE: u16 = 135;
/// A step of the tutorial, as the number it is; what else it does is not known.
const STEP: u16 = 144;
/// What `SOMEONE` is as a side.
pub const SOMEONE_SIDE: i32 = -1;

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
}

/// What a script asks of the battle it runs in. Its sides are numbers: the player's 0, and
/// each new one the next.
pub trait Battle {
    /// How many of the script's updates have gone by.
    fn now(&self) -> i32;
    fn new_side(&mut self) -> i32;
    /// Where a side's base is, and where the enemy's main base is.
    fn base(&self, side: i32) -> (f32, f32);
    fn enemy_base(&self) -> (f32, f32);
    /// The map's point for the script of this number.
    fn script_point(&self, number: i32) -> (f32, f32);
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
    fn pay(&mut self, side: i32, money: i32);
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
                    Piece::Keyword(NEGATIVE) => Value::Number(-number(0)),
                    Piece::Keyword(PLACE) => {
                        let Some(kind) = kind(1) else {
                            return Value::Thing(None);
                        };
                        let (x, z) = at(2).place();
                        let cell = crate::battlefield::CELL_SIDE;
                        let to = (x + number(3) as f32 * cell, z + number(4) as f32 * cell);
                        Value::Thing(battle.place(number(0), kind, to, number(5)))
                    }
                    Piece::Function(NOW) => Value::Number(battle.now()),
                    Piece::Function(READY) => Value::Number(1),
                    Piece::Function(ENEMY_BASE) => place(battle.enemy_base()),
                    Piece::Function(BASE | SIDE_PLACE) => place(battle.base(number(0))),
                    Piece::Function(SCRIPT_POINT) => place(battle.script_point(number(0))),
                    Piece::Function(PLAYER) => Value::Number(0),
                    Piece::Function(NEW_SIDE) => Value::Number(battle.new_side()),
                    Piece::Function(BRING) => match kind(1) {
                        Some(kind) => Value::Thing(battle.bring(number(0), kind, at(2).place())),
                        None => Value::Thing(None),
                    },
                    Piece::Function(OWNER) => {
                        Value::Number(battle.owner_side(at(0).thing()).unwrap_or(SOMEONE_SIDE))
                    }
                    Piece::Function(STANDS) => {
                        Value::Number(i32::from(!battle.destroyed(at(0).thing())))
                    }
                    Piece::Function(HURT_BY) => {
                        Value::Number(i32::from(battle.hurt_by(at(0).thing(), number(1))))
                    }
                    Piece::Function(DESTROYED | GONE) => {
                        Value::Number(i32::from(battle.destroyed(at(0).thing())))
                    }
                    Piece::Function(LEFT) => Value::Number(battle.left(number(0))),
                    Piece::Function(SAY) => {
                        if let Some(Expression::Speech(line)) = arguments.first() {
                            battle.say(*line);
                        }
                        Value::Number(0)
                    }
                    Piece::Function(PAY) => {
                        battle.pay(number(0), number(1));
                        Value::Number(0)
                    }
                    Piece::Function(WIN) => {
                        if arguments.is_empty() || number(0) != 0 {
                            battle.win();
                        } else {
                            battle.lose();
                        }
                        Value::Number(0)
                    }
                    Piece::Function(COUNT) => match kind(1) {
                        Some(kind) => Value::Number(battle.count(number(0), kind)),
                        None => Value::Number(0),
                    },
                    Piece::Function(ENTRANCE) => place(battle.base(0)),
                    Piece::Function(NEAR) => at(0),
                    Piece::Function(ARRIVAL) => place(battle.base(number(0))),
                    Piece::Function(WHERE) => battle
                        .place_of(at(0).thing())
                        .map_or(Value::Thing(None), place),
                    Piece::Function(SOMEONE) => Value::Number(SOMEONE_SIDE),
                    Piece::Function(SEES) => {
                        Value::Number(i32::from(battle.sees(number(0), number(1))))
                    }
                    Piece::Function(SET_HEALTH) => {
                        battle.set_health(at(0).thing(), number(1));
                        Value::Number(0)
                    }
                    Piece::Function(TURN_INTO) => match kind(1) {
                        Some(kind) => {
                            Value::Thing(battle.turn_into(at(0).thing(), kind, number(2)))
                        }
                        None => Value::Thing(None),
                    },
                    Piece::Keyword(SCENE_BEGINS | SCENE_ENDS) => {
                        battle.scene(*callee == Piece::Keyword(SCENE_BEGINS));
                        Value::Number(0)
                    }
                    Piece::Function(LOOK_AT | LOOK_AT_TOO) => {
                        film(battle, spot(0).map(Shot::LookAt))
                    }
                    Piece::Function(GLIDE_TO | GLIDE_TO_TOO) => {
                        film(battle, spot(0).map(|to| Shot::GlideTo(to, number(1))))
                    }
                    Piece::Function(CIRCLE) => {
                        film(battle, Some(Shot::Circle(number(0), number(1))))
                    }
                    Piece::Function(STOP_CIRCLING) => film(battle, Some(Shot::StopCircling)),
                    Piece::Function(KEEP_VIEW) => film(battle, Some(Shot::Keep)),
                    Piece::Function(GLIDE_BACK) => film(battle, Some(Shot::GlideBack(number(0)))),
                    Piece::Function(HEALTH) => Value::Number(
                        battle
                            .health(at(0).thing())
                            .map_or(0, |(health, _)| health as i32),
                    ),
                    Piece::Function(FULL_HEALTH) => Value::Number(
                        battle
                            .health(at(0).thing())
                            .map_or(0, |(_, most)| most as i32),
                    ),
                    Piece::Function(REACHED) => {
                        Value::Number(i32::from(battle.reached(at(0).thing(), number(1))))
                    }
                    Piece::Function(BUILDINGS_LEFT) => {
                        Value::Number(battle.buildings_left(number(0)))
                    }
                    Piece::Function(ARRIVED) => Value::Number(i32::from(battle.arrived(number(0)))),
                    Piece::Function(FOUGHT) => Value::Number(i32::from(battle.has_lost(number(1)))),
                    Piece::Function(HUNT) => {
                        battle.hunt(number(0));
                        Value::Number(0)
                    }
                    Piece::Function(SEND) => {
                        battle.send(number(0), at(1).place());
                        Value::Number(0)
                    }
                    Piece::Function(DONE) => {
                        battle.win();
                        Value::Number(0)
                    }
                    Piece::Keyword(REMOVE) => {
                        battle.remove(at(0).thing());
                        Value::Number(0)
                    }
                    Piece::Keyword(STEP) => Value::Number(number(0)),
                    Piece::Function(FRIENDS | ENEMIES) => {
                        battle.relate(number(0), number(1), *callee == Piece::Function(ENEMIES));
                        Value::Number(0)
                    }
                    _ => Value::Number(0),
                }
            }
        }
    }
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
        [Piece::Keyword(END), ..] => Statement::End,
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
        Script::new(&[vec![F(WIN), text("("), K(TRUE), text(")")]]).run(&mut won);
        assert!(won.won && !won.lost);
        let mut lost = Written::default();
        Script::new(&[vec![F(WIN), text("("), K(FALSE), text(")")]]).run(&mut lost);
        assert!(lost.lost && !lost.won);
        // v1 = F37(2, <kind 4>)
        let lines = vec![vec![
            V(1),
            K(SET),
            F(COUNT),
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
            let rest = [text(","), F(SCRIPT_POINT), text("(1),0,0,0)")];
            [
                vec![V(variable), K(SET), K(PLACE), open, Kind(1)],
                rest.to_vec(),
            ]
            .concat()
        };
        let set = |variable: u16, call: Vec<Token>| [vec![V(variable), K(SET)], call].concat();
        let lines = vec![
            put(0),
            put(1),
            set(2, vec![F(HEALTH), text("("), V(1), text(")")]),
            set(3, vec![F(FULL_HEALTH), text("("), V(1), text(")")]),
            set(
                4,
                vec![
                    F(REACHED),
                    text("("),
                    V(1),
                    text(","),
                    F(PLAYER),
                    text("())"),
                ],
            ),
            set(5, vec![F(BUILDINGS_LEFT), text("(2)")]),
            set(6, vec![F(ARRIVED), text("(2)")]),
            set(7, vec![F(FOUGHT), text("("), F(PLAYER), text("(),3)")]),
            vec![F(SEND), text("(2,"), F(SCRIPT_POINT), text("(4))")],
            vec![F(HUNT), text("(4)")],
            vec![K(REMOVE), text("("), V(0), text(")")],
            set(8, vec![K(STEP), text("(11)")]),
            vec![F(DONE), text("()")],
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
        let point = |n: &str| vec![F(SCRIPT_POINT), text(&format!("({n})"))];
        let lines = vec![
            [vec![V(13), K(SET)], point("13")].concat(),
            [vec![V(16), K(SET)], point("16")].concat(),
            vec![K(SCENE_BEGINS), text("()")],
            vec![F(LOOK_AT), text("("), V(13), text(")")],
            vec![F(KEEP_VIEW), text("()")],
            vec![F(GLIDE_TO), text("("), V(16), text(",50)")],
            vec![F(CIRCLE), text("(2,1)")],
            vec![F(STOP_CIRCLING), text("()")],
            vec![F(GLIDE_BACK), text("(60)")],
            vec![K(SCENE_ENDS), text("()")],
            vec![F(LOOK_AT_TOO), text("("), V(20), text(")")],
            vec![F(GLIDE_TO_TOO), text("("), V(13), text(",80)")],
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
            ]
        );
    }

    #[test]
    fn a_prison_camps_sides_are_related_and_its_prisoners_freed_once_seen() {
        use Token::{Function as F, Keyword as K, Kind, Variable as V};
        let lines = vec![
            // v1 = F18(); v2 = F18(); F59(v1, F14()); F60(F16(), v2)
            vec![V(1), K(SET), F(NEW_SIDE), text("()")],
            vec![V(2), K(SET), F(NEW_SIDE), text("()")],
            vec![
                F(FRIENDS),
                text("("),
                V(1),
                text(","),
                F(PLAYER),
                text("())"),
            ],
            vec![
                F(ENEMIES),
                text("("),
                F(SOMEONE),
                text("(),"),
                V(2),
                text(")"),
            ],
            // v3 = K139(v1, <kind 4>, F3(), 0, 0, 0); F54(v3, 250)
            vec![
                V(3),
                K(SET),
                K(PLACE),
                text("("),
                V(1),
                text(","),
                Kind(4),
                text(","),
                F(ENEMY_BASE),
                text("(),0,0,0)"),
            ],
            vec![F(SET_HEALTH), text("("), V(3), text(",250)")],
            // if (F32(v1, F14()) == TRUE) F57(v3, <kind 7>, F14())
            vec![
                K(IF),
                text("("),
                F(SEES),
                text("("),
                V(1),
                text(","),
                F(PLAYER),
                text("())"),
                K(EQUAL),
                K(TRUE),
                text(")"),
            ],
            vec![
                F(TURN_INTO),
                text("("),
                V(3),
                text(","),
                Kind(7),
                text(","),
                F(PLAYER),
                text("())"),
            ],
            vec![K(END)],
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
            vec![K(IF), text("("), F(NOW), text("()"), K(EQUAL), text("0)")],
            // v1 = F18()
            vec![V(1), K(SET), F(NEW_SIDE), text("()")],
            // v2 = K139(v1, <kind 5>, F3(), K143(2), 3, 1)
            vec![
                V(2),
                K(SET),
                K(PLACE),
                text("("),
                V(1),
                text(","),
                Kind(5),
                text(","),
                F(ENEMY_BASE),
                text("(),"),
                K(NEGATIVE),
                text("(2),3,1)"),
            ],
            vec![F(PAY), text("("), F(PLAYER), text("(),10000)")],
            vec![F(SAY), text("("), Speech(642), text(")")],
            vec![K(END)],
            // if ((F0() == 49) && (F35(v1) < 5)) bring
            vec![
                K(IF),
                text("(("),
                F(NOW),
                text("()"),
                K(EQUAL),
                text("49)"),
                K(AND),
                text("("),
                F(LEFT),
                text("("),
                V(1),
                text(")"),
                K(LESS),
                text("5))"),
            ],
            vec![
                F(BRING),
                text("("),
                V(1),
                text(","),
                Kind(9),
                text(","),
                F(SCRIPT_POINT),
                text("(2))"),
            ],
            vec![K(END)],
            vec![K(IF), text("("), F(DESTROYED), text("("), V(2), text("))")],
            vec![F(WIN), text("("), K(TRUE), text(")")],
            vec![K(END)],
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
