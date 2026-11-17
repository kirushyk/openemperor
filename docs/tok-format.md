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

The number seems to be the line's place among the sections of the dialogue tables as
`data\Dialog\Dialog.txt` includes them, `AtreidesDialog.txt` first: 642 and 643, the two
the start mission plays first, are `KK-M066` and `KK-M068` there, lines of the Atreides
Mentat. This is not proven: nothing in the script says what the line says.

## Keywords and functions

Their names are not on the disc. These are what their use in `ATStart.tok` shows:

| Token | Seems to be | Seen as |
| --- | --- | --- |
| `K162`, `K163`, `K164` | Declarations of variables, of three types | `K162(v0)` at the top |
| `K165` … `K167` | `if (…)` and its end | `K165(F0() K168 550)` … `K167` |
| `K168` | `==` | |
| `K171` | A comparison, `<` it seems | `F35(F14()) K171 5` |
| `K174`, `K175` | `&&`, `\|\|` | |
| `K176`, `K177` | `FALSE`, `TRUE` | |
| `K178` | `+` | `F0() K178 260` |
| `K180` | `=` | |
| `K143` | A negative number | `K143(10)` |
| `K139` | Puts a new object on the map: side, kind, place, tiles across, tiles down, facing | `K139(v0,<ATConYard>,v13,0,0,1)` |
| `K150` | Sets something of a side | `K150(v2,1)` |
| `K128`, `K131`, `K132`, `K133`, `K156` | Not known | |
| `F0()` | The battle's time, in updates | compared with 49, 50, 100, 150 … |
| `F14()` | The player's side | `v0 K180 F14()` |
| `F18()` | A new side | four of them, the enemy and others |
| `F3()`, `F4(side)`, `F5(n)`, `F6(side)` | Places on the map: a side's base, the map's numbered entrances | |
| `F21(side, kind, place)` | Brings a unit of that kind in for the side at a place | reinforcements |
| `F23(object)` | Whether an object is destroyed | all three of the enemy's main buildings ends the mission |
| `F35(side)` | How many units a side has left | `F35(v2) K168 0` |
| `F45(line)` | Plays a line of speech | |
| `F60(a, b)` | Makes side `a` an enemy of `b` | given both ways |
| `F62(side, n)` | Gives a side money | `F62(v0,10000)` |
| `F83`, `F84` | End the mission, won it seems | after the three buildings are destroyed |

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

## What we do with them

`src/mission.rs` reads a `.tok` into its lines of tokens. Nothing runs the scripts yet.
