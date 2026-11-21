# Mission scripts: the `.tok` files

The original's missions are scripts, kept tokenised in `data\MISSIONS0001.RFD` of
`Game1.cab`: one `.tok` file a mission, such as `ATStart.tok` for the Atreides' first battle.
The archive's `missionlist.ini` lists the scripts by their source names (`ATStart.txt`), and
`header.tok` has a record of 228 bytes for each. The source text is not on the disc. What
follows was worked out from the files themselves, `ATStart.tok` above all; where it is a
guess it says so.

## The file

| Bytes | What |
| --- | --- |
| 0–3 | The size of the rest of the file after these eight bytes, little-endian. |
| 4–7 | How many lines the script has. |
| 8… | The lines, each ended by a zero byte. An empty line is just its zero. |

A line is one statement of the source, with its comments and spacing gone. Blank lines and
comment lines are kept as empty lines, so line numbers still match the source.

## Tokens

A line is made of plain characters and tokens. A token starts with a byte from `0x80` up;
every byte after it in the token also has its top bit set, and carries seven bits of a
number, lowest first.

| First byte | Bytes after | What it is |
| --- | --- | --- |
| `0x80` | 2 | A function or a keyword, by number. Below 128 are functions (`F0`…), from 128 keywords and operators (`K128`…). |
| `0x81` | 1 | A variable, by number, in the order the script declares them. |
| `0x82` | 2 | A kind of unit or building. |
| `0x83` | 2 | A line of speech. |

Everything else is plain text as written: digits, commas, brackets. Numbers are decimal
digits; a negative one is written with the keyword `K143`, as `K143(4)` for −4.

So the bytes `81 86 80 B4 81 80 85 80 28 31 29` read `v6 K180 F5(1)`: the variable 6 is set
to what function 5 gives for 1.

### Kinds of unit and building (`0x82`)

The number is 111 more than the kind's place among the entries of `Rules.txt`'s
`[BuildingTypes]` and `[UnitTypes]` lists, taken in the order they come in the file (the
lists come several times, each adding to its own). 127 is `ATWall`, 138 `ATConYard`, 246
`HKLightInf`, 270 `HKBuzzsaw`, 300 `MCV`. What the first 111 numbers are is not known; the
houses, bullets and warheads together make as many.

### Lines of speech (`0x83`)

The number is 502 more than the line's place among the entries of the `MissionMessages`
sections of the game's table of what is spoken, `E_Output_Pickup.txt` in
`data\STRINGS0001.RFD`: only those sections count, each from its `MissionMessages` line to its
`[END]`, and in them only lines with an id, not blank lines or comments. The briefings and
debriefings before each house's messages do not count, which is why counting every line of
the file only fitted the Atreides' first mission. 642 is `ATstart1`, the Mentat's welcome, 646
`ATtutorial1`, 901 `HKstart1` and 1098 `ORstart1`. The id at the start of that line names its
recording in `data\Sounds\sounds.txt` (`ATstart1` is `KK-M680E`), and the recording is in a
dialogue bag under that name without the language's letter: `KK-M680` in the Atreides disc's
`DIALOG.BAG`. Numbers below 502 name something else, not known yet.

## Keywords and functions

Their names are not on the disc. These are what their use in `ATStart.tok` shows:

| Token | Seems to be | Seen as |
| --- | --- | --- |
| `K162`, `K163`, `K164` | Declarations of variables, of three types | `K162(v0)` at the top |
| `K165` … `K167` | `if (…)` and its end | `K165(F0() K168 550)` … `K167` |
| `K168` | `==` | |
| `K169` | `!=` | `F17(v27) K169 F14()`: a building not the player's |
| `K170`, `K171`, `K172`, `K173` | `>=`, `<=`, `>`, `<` | `v32 K171 25`: the tutorial's step once it is 25; `F0() K173 15000`: before update 15000 |
| `K174`, `K175` | `&&`, `\|\|` | |
| `K176`, `K177` | `FALSE`, `TRUE` | |
| `K178` | `+` | `F0() K178 260` |
| `K180` | `=` | |
| `K143` | A negative number | `K143(10)` |
| `K139` | Puts a new object on the map and gives it: side, kind, place, tiles across, tiles down, facing | `K139(v0,<ATConYard>,v13,0,0,1)` |
| `K150` | Sets something of a side | `K150(v2,1)` |
| `K131`, `K132` | A scene begins and ends: the camera is the script's, not scrolled | around the first battles' opening |
| `K128`, `K129`, `K130`, `K133`, `K156` | Not known | |
| `F0()` | The battle's time, in updates | compared with 49, 50, 100, 150 … |
| `F14()` | The player's side | `v0 K180 F14()` |
| `F18()` | A new side | four of them, the enemy and others |
| `F3()` | The enemy's main base: the map's point `Primary` | |
| `F4(side)`, `F6(side)` | A side's base: the player's at the map's point `Default` | `v13 K180 F4(v0)` |
| `F5(n)` | The map's point `Script` and the number | `F5(1)` is `Script1` |
| `F2()` | Asked before the first reinforcements; taken as true | |
| `F21(side, kind, place)` | Brings a unit of that kind in for the side at a place | reinforcements |
| `F23(object)` | Whether an object is destroyed | all three of the enemy's main buildings ends the mission |
| `F35(side)` | How many units a side has left | `F35(v2) K168 0` |
| `F45(line)` | Plays a line of speech | |
| `F60(a, b)` | Makes side `a` an enemy of `b` | given both ways |
| `F62(side, n)` | Gives a side money | `F62(v0,10000)` |
| `F83(truth)` | Ends the battle, won if true and lost if false | `F83(TRUE)` once the enemy's palace and yards are gone |
| `F37(side, kind)` | How many things of a kind a side has; for `F16()`, the map's own too | `F37(F16(),<HKPalace>) K168 0` |
| `F8()` | Where reinforcements come in; taken as the player's base | `F21(F14(),<FRFremen>,F8())` |
| `F11(place)` | A place near one; taken as that place | `F11(F12(F16()))` |
| `F12(side)` | Where a side's units arrive; taken as its base | `F21(F14(),<ATMongoose>,F12(F14()))` |
| `F13(thing)` | Where a thing is | `F97(F13(v3),12)` |
| `F16()` | A side that is neither the player's nor one of the script's | `F59(v1,F16())` |
| `F17(thing)` | Whose a thing is, as a side | `F17(v3) K168 F14()`: the outpost captured |
| `F22(thing)` | Whether a thing still stands | |
| `F27(thing, side)` | Whether a side has hurt a thing; taken as: it is hurt, and the side is its enemy | `F27(v3,F14())` |
| `F32(a, b)` | Whether side `a`'s units are in sight of `b`'s | prisoners reached |
| `F44(thing)` | Whether a thing is destroyed | |
| `F54(thing, n)` | Sets a thing's health | a camp's windtraps at 90 to 320 |
| `F57(thing, kind, side)` | Turns a thing into a kind, for a side | prisoners freed as Fremen |
| `F59(a, b)` | Makes side `a` a friend of `b` | given both ways |
| `F24(thing, side)` | Whether a side has a unit at a thing; taken as within three squares of its edge | `F24(v61,F14())`: the tutorial's beacon reached |
| `F29(thing)`, `F30(thing)` | A thing's health and the most it has | `F29(v0) K168 F30(v0)`: the tutorial's windtrap mended |
| `F36(side)` | How many buildings a side has left | with `F35`, the tutorial's enemy base gone |
| `F38(side)` | Whether a side's units have all got where they were sent | after `F71` |
| `F42(a, b)` | Taken as: whether `b` has lost any of what the script gave it, to `a` | the tutorial's wave fought, not only met by the worm |
| `F64(side)` | A side's units go after the player's, and keep at it | the wave the worm left |
| `F71(side, place)` | Sends a side's units to a place | |
| `F82(place)` | A worm comes up at a place; not done yet, as there are no worms | `F82(F12(v43))` |
| `F84()` | The tutorial is over and done; taken as won | 27.5 s after its last line |
| `K135(thing)` | Takes a thing off the map | the tutorial's walls opened for the wave |
| `K144(n)` | A step of the tutorial, as the number; what else it does is not known | `v32 K180 K144(11)` |
| `F63`, `F85`, `F96`, `F97` | Not known; the tutorial runs without them | `F97(v35,20)` as a beacon goes down |
| `F107(thing)`, `F108()` | The camera follows a thing, or what carries it, and stops following | `F107(v1)`: the tutorial's harvester, at its start |
| `F100(place)`, `F114(place)` | The camera looks at a place; the second in the midst of the battle | `F100(v13)`: the player's base |
| `F101(place, n)`, `F115(place, n)` | The camera glides to a place over `n` updates | `F101(v16,50)`: to the enemy's base |
| `F105(degrees, way)` | The camera circles where it looks, so many degrees an update, `1` one way and `2` the other | `F105(2,1)`: once round in the 180 updates before `F113` |
| `F106()` | The camera stops circling | |
| `F112()` | The camera's view is kept | before it glides away |
| `F113(n)` | The camera stops circling and glides back to the view kept over `n` updates | `F113(60)` |

## The Atreides' first mission

`ATStart.tok` has 754 lines. It declares 53 variables, makes four sides besides the
player's, and gives the player 10 000 credits and a base already built: a construction yard
with walls round it, six windtraps, a barracks, an outpost, pillboxes and rocket turrets,
snipers, Kindjal infantry and Mongooses. The Harkonnen side has a palace, an outpost and a
hangar, three windtraps, four flame turrets, barrels, scouts and light infantry. At update 49
an MCV, two trikes and a Mongoose are brought in. Harkonnen light infantry, troopers and
buzzsaws come in waves by the entrances, and Atreides trikes and Mongooses in answer. The
Mentat speaks at the start and at updates 430, 490 and 550, and as each wave comes. The
mission is won when the three Harkonnen main buildings are destroyed.

## Points on the map

A map's `test.xbf` has named areas of named lists of points. Those the scripts use are the
first point of a list: `Primary` and `Default` in the area `Base`, `Script1` to `Script6` in
`Mission`, `Script7` on in `MoreScripts`. The Atreides' first map has its `Default` on the
left, where the script builds their base, and its `Primary` on the right, the Harkonnen's.

## What we do with them

`src/mission.rs` reads a `.tok` into its lines of tokens, and runs them: each line is a
statement (an `if`, its end, a variable set, or a call), all of them run once an update with
what is inside an `if` that does not hold left out, as the original's scripts are written to
be run. The functions and keywords in the table above do what it says; the others do
nothing yet. The update is taken to be a tenth of a second, which is a guess.

A battle for a territory runs the script of its house, phase, attack (`M`) or defence (`D`)
and territory, where the disc's `missionlist.ini` in `MISSIONS0001` lists one: the
Atreides' defence of Bilar Slopes, territory 7, in phase 1 is `ATP1D7FR`, the prison camp.
A script's sides are the enemy's if it makes them the player's enemy, the player's if it
makes them the enemy of one of those, and nobody's otherwise, as the camp's prisoners are;
what a side is follows each `F59` and `F60` again, as the scripts put things down before
saying whose enemy they are. A side the original keeps apart, such as Fremen freed to fight
beside the player, is the player's own here, as the battle has only two owners.

A house's first battle in the campaign runs its start script (`ATStart.tok` for the
Atreides): the battle starts with nothing but what the script puts down, the sides it makes
are all the player's enemy, and its lines of speech play off the disc that has them.
